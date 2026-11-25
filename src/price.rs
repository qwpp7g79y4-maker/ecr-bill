use rust_decimal::Decimal;
use serde_json::Value;

use crate::error::{show_text, Error};
use crate::json_scan::{parse_json, reject_duplicate_keys};
use crate::money::{civil_from_days, parse_money, ymd_days, MoneyError};

/// Pricing page the built-in numbers were read from.
pub const PRICE_PAGE_URL: &str = "https://aws.amazon.com/ecr/pricing/";

/// Day that page was read.
pub const PRICE_PAGE_DATE: &str = "2026-11-09";

/// Rates for one region. Retrieval is stored and is not added to storage.
#[derive(Debug, Clone)]
pub struct Prices {
    pub source_url: String,
    pub source_date: String,
    pub region: String,
    pub region_name: String,
    pub(crate) standard: Decimal,
    pub(crate) archive_first: Decimal,
    pub(crate) archive_over: Decimal,
    pub(crate) retrieval: Decimal,
    pub(crate) archive_tier_tb: u32,
    pub(crate) archive_min_days: u32,
}

/// Built-in US East (N. Virginia) quote.
#[must_use]
pub fn prices_builtin() -> Prices {
    load_prices_str(include_str!("../prices/ecr-storage.json")).expect("built-in prices")
}

/// Replace the built-in quote. Unknown fields are ignored.
pub fn load_prices_str(text: &str) -> Result<Prices, Error> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    reject_duplicate_keys(text)?;
    let value: Value = parse_json(text)?;
    let obj = value.as_object().ok_or_else(|| Error::new("not JSON"))?;
    let rates = required_object(obj, "usd_per_gb_month")?;
    Ok(Prices {
        source_url: https_url(required_str(obj, "source_url")?)?,
        source_date: required_str(obj, "source_date")?.to_string(),
        region: required_str(obj, "region")?.to_string(),
        region_name: required_str(obj, "region_name")?.to_string(),
        standard: money_field(rates, "standard")?,
        archive_first: money_field(rates, "archive_first_150_tb")?,
        archive_over: money_field(rates, "archive_over_150_tb")?,
        retrieval: money_field(obj, "usd_per_gb_retrieval")?,
        archive_tier_tb: required_u32(obj, "archive_tier_tb")?,
        archive_min_days: required_u32(obj, "archive_min_days")?,
    })
}

/// Prices older than this many days before the as-of day get a warning.
pub const PRICE_STALE_DAYS: i64 = 180;

/// A one-line warning when the price date is more than [`PRICE_STALE_DAYS`]
/// before `as_of` (`YYYY-MM-DD`) or, without it, before `today_days` (days
/// since 1970-01-01). A price date that is not `YYYY-MM-DD` gives no warning.
#[must_use]
pub fn price_age_warning(prices: &Prices, as_of: Option<&str>, today_days: i64) -> Option<String> {
    let price_day = ymd_days(&prices.source_date)?;
    let (as_of_day, as_of_text) = match as_of {
        Some(text) => (ymd_days(text)?, show_text(text)),
        None => {
            let (y, m, d) = civil_from_days(today_days);
            (today_days, format!("{y:04}-{m:02}-{d:02}"))
        }
    };
    let age = as_of_day - price_day;
    (age > PRICE_STALE_DAYS).then(|| {
        format!(
            "prices are {age} days old (price date {}, as of {as_of_text}); check current prices",
            show_text(&prices.source_date)
        )
    })
}

fn missing(key: &str) -> Error {
    Error::new(format!("price file is missing {key}"))
}

fn invalid(key: &str) -> Error {
    Error::new(format!("price file has an invalid {key}"))
}

/// A field is `missing` only when its key is absent; any other wrong value is `invalid`.
fn present<'a>(obj: &'a serde_json::Map<String, Value>, key: &str) -> Result<&'a Value, Error> {
    obj.get(key).ok_or_else(|| missing(key))
}

fn required_object<'a>(
    obj: &'a serde_json::Map<String, Value>,
    key: &str,
) -> Result<&'a serde_json::Map<String, Value>, Error> {
    present(obj, key)?.as_object().ok_or_else(|| invalid(key))
}

fn required_str<'a>(obj: &'a serde_json::Map<String, Value>, key: &str) -> Result<&'a str, Error> {
    present(obj, key)?.as_str().ok_or_else(|| invalid(key))
}

/// A rate: absent is `missing`; present but not a decimal string is its own error.
fn money_field(obj: &serde_json::Map<String, Value>, key: &str) -> Result<Decimal, Error> {
    let value = present(obj, key)?;
    let not_decimal = || Error::new("price file has a rate that is not a decimal");
    match parse_money(value.as_str().ok_or_else(not_decimal)?) {
        Ok(rate) => Ok(rate),
        Err(MoneyError::NotDecimal) => Err(not_decimal()),
        Err(MoneyError::TooPrecise) => {
            Err(Error::new("price file has a rate with more than 28 digits"))
        }
    }
}

fn required_u32(obj: &serde_json::Map<String, Value>, key: &str) -> Result<u32, Error> {
    present(obj, key)?
        .as_u64()
        .and_then(|number| u32::try_from(number).ok())
        .ok_or_else(|| invalid(key))
}

fn https_url(raw: &str) -> Result<String, Error> {
    if raw.starts_with("https://") && !raw.chars().any(char::is_whitespace) {
        Ok(raw.to_string())
    } else {
        Err(invalid("source_url"))
    }
}

#[cfg(test)]
mod tests {
    use super::{load_prices_str, prices_builtin, PRICE_PAGE_DATE, PRICE_PAGE_URL};

    #[test]
    fn claim_c3_standard_rate_source_date_region() {
        let prices = prices_builtin();
        assert_eq!(prices.source_url, PRICE_PAGE_URL);
        assert_eq!(prices.source_date, PRICE_PAGE_DATE);
        assert_eq!(prices.region, "us-east-1");
        assert_eq!(prices.region_name, "US East (N. Virginia)");
        assert_eq!(prices.standard.to_string(), "0.10");
        assert_eq!(prices.archive_first.to_string(), "0.10");
        assert_eq!(prices.archive_over.to_string(), "0.07");
        assert_eq!(prices.retrieval.to_string(), "0.03");
        assert_eq!(prices.archive_tier_tb, 150);
        assert_eq!(prices.archive_min_days, 90);
        let file = include_str!("../prices/ecr-storage.json");
        assert!(file.contains("https://aws.amazon.com/ecr/pricing/"));
        assert!(file.contains("2026-11-09"));
        assert!(file.contains("2026-09-11T12:44:25Z"));
    }

    #[test]
    fn claim_c17_unknown_field_ignored_and_duplicate_hidden() {
        let mut text = include_str!("../prices/ecr-storage.json").to_string();
        text = text.replacen(
            "\"source_url\": \"https://aws.amazon.com/ecr/pricing/\"",
            "\"source_url\": \"https://aws.amazon.com/ecr/pricing/\", \"note2\": \"kept\"",
            1,
        );
        assert!(load_prices_str(&text).is_ok());
        let err = load_prices_str(
            r#"{"source_url":"https://example.test/","source_url":"https://example.test/"}"#,
        )
        .unwrap_err();
        assert_eq!(err.message(), "duplicate key");
        let err = load_prices_str("Bucket,Key\n").unwrap_err();
        assert_eq!(err.message(), "not JSON");
    }

    #[test]
    fn missing_field_names_only_the_field() {
        let err = load_prices_str(r#"{"source_url":"https://example.test/a"}"#).unwrap_err();
        assert!(err.message().contains("price file is missing"));
        assert!(!err.message().contains("example.test"));
    }

    #[test]
    fn bad_money_does_not_echo_the_value() {
        let mut text = include_str!("../prices/ecr-storage.json").to_string();
        text = text.replacen("\"standard\": \"0.10\"", "\"standard\": \"1e2\"", 1);
        let err = load_prices_str(&text).unwrap_err();
        assert!(!err.message().contains("1e2"));
    }

    #[test]
    fn bom_is_accepted() {
        let text = format!("\u{feff}{}", include_str!("../prices/ecr-storage.json"));
        assert!(load_prices_str(&text).is_ok());
    }

    #[test]
    fn a_rate_that_is_not_a_decimal_is_named() {
        let text = include_str!("../prices/ecr-storage.json").replacen(
            "\"usd_per_gb_retrieval\": \"0.03\"",
            "\"usd_per_gb_retrieval\": \"1e2\"",
            1,
        );
        let err = load_prices_str(&text).unwrap_err();
        assert_eq!(err.message(), "price file has a rate that is not a decimal");
        assert!(!err.message().contains("1e2"));
    }

    #[test]
    fn source_url_must_be_https_without_whitespace() {
        for url in ["http://example.test/", "https://exa mple.test/", "ftp://x/"] {
            let text = include_str!("../prices/ecr-storage.json").replacen(
                "https://aws.amazon.com/ecr/pricing/\",",
                &format!("{url}\","),
                1,
            );
            let err = load_prices_str(&text).unwrap_err();
            assert_eq!(
                err.message(),
                "price file has an invalid source_url",
                "{url}"
            );
            assert!(!err.message().contains("example"));
        }
    }

    // Red before the fix: a present field of the wrong kind was reported as missing.
    #[test]
    fn a_field_is_missing_only_when_the_key_is_absent() {
        let base = include_str!("../prices/ecr-storage.json");
        let with = |from: &str, to: &str| load_prices_str(&base.replacen(from, to, 1));
        for (from, to, name) in [
            (
                "\"source_url\": \"https://aws.amazon.com/ecr/pricing/\"",
                "\"source_url\": \"http://x/\"",
                "source_url",
            ),
            (
                "\"source_url\": \"https://aws.amazon.com/ecr/pricing/\"",
                "\"source_url\": 5",
                "source_url",
            ),
            ("\"region\": \"us-east-1\"", "\"region\": 5", "region"),
            (
                "\"archive_tier_tb\": 150",
                "\"archive_tier_tb\": -1",
                "archive_tier_tb",
            ),
            (
                "\"archive_tier_tb\": 150",
                "\"archive_tier_tb\": 4294967296",
                "archive_tier_tb",
            ),
            (
                "\"archive_min_days\": 90",
                "\"archive_min_days\": \"90\"",
                "archive_min_days",
            ),
        ] {
            let err = with(from, to).unwrap_err();
            assert_eq!(
                err.message(),
                format!("price file has an invalid {name}"),
                "{to}"
            );
        }
        let err = with("\"region\": \"us-east-1\"", "\"other\": 1").unwrap_err();
        assert_eq!(err.message(), "price file is missing region");
    }

    #[test]
    fn a_rate_is_missing_only_when_the_key_is_absent() {
        let base = include_str!("../prices/ecr-storage.json");
        let with = |from: &str, to: &str| load_prices_str(&base.replacen(from, to, 1));
        for to in [
            "\"standard\": \"1e2\"",
            "\"standard\": \"-0.10\"",
            "\"standard\": 0.1",
            "\"standard\": null",
        ] {
            let err = with("\"standard\": \"0.10\"", to).unwrap_err();
            assert_eq!(
                err.message(),
                "price file has a rate that is not a decimal",
                "{to}"
            );
        }
        let err = with("\"standard\": \"0.10\"", "\"other\": \"0.10\"").unwrap_err();
        assert_eq!(err.message(), "price file is missing standard");
        let err = with(
            "\"usd_per_gb_retrieval\": \"0.03\"",
            "\"usd_per_gb_retrieval\": 3",
        )
        .unwrap_err();
        assert_eq!(err.message(), "price file has a rate that is not a decimal");
        let err = with("\"usd_per_gb_retrieval\": \"0.03\"", "\"x\": 3").unwrap_err();
        assert_eq!(err.message(), "price file is missing usd_per_gb_retrieval");
    }
}
