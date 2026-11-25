use std::collections::BTreeMap;

use crate::error::Error;
use crate::money::{charge, checked_add, format_usd, ymd_days, Amount, GIB};
use crate::parse::{parse_document, Class, ImageRec};
use crate::policy::propose_policy;
use crate::price::Prices;

/// One repository in the report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoLine {
    pub name: String,
    pub images: u64,
    pub bytes: u64,
    pub untagged: u64,
    pub old: Option<u64>,
    pub usd_month: String,
}

/// One image in the untagged or old list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageLine {
    pub repository: String,
    pub digest: String,
    pub bytes: u64,
    pub tags: u32,
    pub age_days: Option<i64>,
}

/// Priced describe-images documents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bill {
    pub source_url: String,
    pub source_date: String,
    pub region: String,
    pub region_name: String,
    pub as_of: Option<String>,
    pub older_than_days: u32,
    pub untagged_days: u32,
    pub keep: u32,
    pub repos: Vec<RepoLine>,
    pub total_images: u64,
    pub total_bytes: u64,
    pub total_untagged: u64,
    pub total_usd: String,
    pub retrieval_usd_per_gb: String,
    pub archive_min_days: u32,
    pub untagged: Vec<ImageLine>,
    pub old: Option<Vec<ImageLine>>,
    pub policy_json: String,
    pub warnings: Vec<String>,
}

/// Knobs that do not include the price list.
#[derive(Debug, Clone)]
pub struct BillOptions {
    pub as_of: Option<String>,
    pub older_than_days: u32,
    pub untagged_days: u32,
    pub keep: u32,
}

impl Default for BillOptions {
    fn default() -> Self {
        Self {
            as_of: None,
            older_than_days: 30,
            untagged_days: 14,
            keep: 10,
        }
    }
}

struct RepoAccum {
    images: u64,
    bytes: u64,
    untagged: u64,
    old: u64,
    standard_bytes: u64,
    archive_bytes: u64,
}

/// Price one or more `describe-images` documents. Each object is one image.
pub fn bill_documents(
    docs: &[&str],
    prices: &Prices,
    options: &BillOptions,
) -> Result<Bill, Error> {
    bill_documents_labeled(docs, &[], prices, options)
}

/// Like [`bill_documents`]. A label, when given for a document, names that file
/// in errors that do not already say `file N`.
pub(crate) fn bill_documents_labeled(
    docs: &[&str],
    labels: &[String],
    prices: &Prices,
    options: &BillOptions,
) -> Result<Bill, Error> {
    if docs.is_empty() {
        return Err(Error::new("no inventory files"));
    }
    if options.older_than_days == 0 || options.untagged_days == 0 || options.keep == 0 {
        return Err(Error::new("policy counts must be positive"));
    }
    let as_of_days = match options.as_of.as_deref() {
        Some(text) => Some(ymd_days(text).ok_or_else(|| Error::new("invalid --as-of"))?),
        None => None,
    };
    let mut images = Vec::new();
    for (index, doc) in docs.iter().enumerate() {
        let file_no = u32::try_from(index + 1).unwrap_or(u32::MAX);
        let parsed = parse_document(doc, file_no).map_err(|err| match labels.get(index) {
            Some(label) if !err.message().starts_with("file ") => {
                Error::new(format!("{label}: {err}"))
            }
            _ => err,
        })?;
        images.extend(parsed);
    }
    finish(images, prices, options, as_of_days)
}

fn finish(
    images: Vec<ImageRec>,
    prices: &Prices,
    options: &BillOptions,
    as_of_days: Option<i64>,
) -> Result<Bill, Error> {
    let mut repos: BTreeMap<String, RepoAccum> = BTreeMap::new();
    let mut untagged = Vec::new();
    let mut old = Vec::new();
    let mut total_images = 0u64;
    let mut total_bytes = 0u64;
    let mut total_untagged = 0u64;
    let mut archive_bytes = 0u64;
    let mut unpriced = 0u64;
    let mut skipped_age = 0u64;

    for image in &images {
        total_images = total_images
            .checked_add(1)
            .ok_or_else(|| Error::new("image total does not fit"))?;
        checked_add(&mut total_bytes, image.bytes)?;
        let repo = repos.entry(image.repository.clone()).or_insert(RepoAccum {
            images: 0,
            bytes: 0,
            untagged: 0,
            old: 0,
            standard_bytes: 0,
            archive_bytes: 0,
        });
        repo.images += 1;
        checked_add(&mut repo.bytes, image.bytes)?;
        if image.untagged {
            total_untagged += 1;
            repo.untagged += 1;
            untagged.push(ImageLine {
                repository: image.repository.clone(),
                digest: image.digest.clone(),
                bytes: image.bytes,
                tags: image.tags,
                age_days: None,
            });
        }
        match image.class {
            Class::Standard => {
                checked_add(&mut repo.standard_bytes, image.bytes)?;
            }
            Class::Archive => {
                checked_add(&mut archive_bytes, image.bytes)?;
                checked_add(&mut repo.archive_bytes, image.bytes)?;
            }
            Class::Unpriced => {
                unpriced += 1;
            }
        }
        if let Some(as_of) = as_of_days {
            match image.pushed_epoch {
                Some(epoch) => {
                    let pushed_day = epoch.div_euclid(86_400);
                    let age = as_of - pushed_day;
                    if age > i64::from(options.older_than_days) {
                        repo.old += 1;
                        old.push(ImageLine {
                            repository: image.repository.clone(),
                            digest: image.digest.clone(),
                            bytes: image.bytes,
                            tags: image.tags,
                            age_days: Some(age),
                        });
                    }
                }
                None => skipped_age += 1,
            }
        }
    }

    let archive_usd = archive_charge(archive_bytes, prices)?;
    let (repo_lines, total_usd) = repo_lines(
        &repos,
        prices,
        archive_usd,
        archive_bytes,
        as_of_days.is_some(),
    )?;
    let mut warnings = Vec::new();
    if unpriced > 0 {
        warnings.push(format!("images without a built-in rate: {unpriced}"));
    }
    if skipped_age > 0 {
        warnings.push(format!("images skipped for age: {skipped_age}"));
    }
    Ok(Bill {
        source_url: prices.source_url.clone(),
        source_date: prices.source_date.clone(),
        region: prices.region.clone(),
        region_name: prices.region_name.clone(),
        as_of: options.as_of.clone(),
        older_than_days: options.older_than_days,
        untagged_days: options.untagged_days,
        keep: options.keep,
        repos: repo_lines,
        total_images,
        total_bytes,
        total_untagged,
        total_usd,
        retrieval_usd_per_gb: format_usd(prices.retrieval),
        archive_min_days: prices.archive_min_days,
        untagged,
        old: as_of_days.map(|_| old),
        policy_json: propose_policy(options.untagged_days, options.keep)?,
        warnings,
    })
}

/// The archive tier boundary in bytes, or `None` when no u64 byte total can pass it.
fn archive_boundary(prices: &Prices) -> Option<u64> {
    let tb = u64::from(prices.archive_tier_tb);
    tb.checked_mul(1024).and_then(|gb| gb.checked_mul(GIB))
}

/// True when every archived byte is priced at one rate: the boundary is not
/// crossed, or the rate over it is the rate under it.
fn archive_is_one_rate(bytes: u64, prices: &Prices) -> bool {
    prices.archive_first == prices.archive_over
        || archive_boundary(prices).is_none_or(|boundary| bytes <= boundary)
}

fn archive_charge(bytes: u64, prices: &Prices) -> Result<Amount, Error> {
    if archive_is_one_rate(bytes, prices) {
        return charge(bytes, prices.archive_first);
    }
    let boundary = archive_boundary(prices).unwrap_or(u64::MAX);
    let first = charge(boundary, prices.archive_first)?;
    let over = charge(bytes - boundary, prices.archive_over)?;
    first.add(over)
}

/// The rows and the total, each an exact amount rounded once at the end.
///
/// A repository's archive part is either its own bytes at the one rate, or,
/// when a tier boundary is crossed between two rates, its share
/// `archive_usd * its bytes / all archived bytes`. A row is
/// `(standard * all + archive_usd * its bytes) / all` as one exact fraction;
/// the shares add up to `archive_usd`, so the total is the exact sum.
fn repo_lines(
    repos: &BTreeMap<String, RepoAccum>,
    prices: &Prices,
    archive_usd: Amount,
    archive_bytes: u64,
    with_old: bool,
) -> Result<(Vec<RepoLine>, String), Error> {
    let one_rate = archive_is_one_rate(archive_bytes, prices);
    let mut total = if one_rate { Amount::ZERO } else { archive_usd };
    let mut lines = Vec::with_capacity(repos.len());
    for (name, repo) in repos {
        let standard = charge(repo.standard_bytes, prices.standard)?;
        total = total.add(standard)?;
        let (raw, divisor) = if repo.archive_bytes == 0 {
            (standard, 1)
        } else if one_rate {
            // One rate for every archived byte: price the repository's own
            // bytes, so an exact half cent stays exact and no row is a rest.
            let own = charge(repo.archive_bytes, prices.archive_first)?;
            total = total.add(own)?;
            (standard.add(own)?, 1)
        } else {
            let part = archive_usd.times(repo.archive_bytes)?;
            (standard.times(archive_bytes)?.add(part)?, archive_bytes)
        };
        lines.push(RepoLine {
            name: name.clone(),
            images: repo.images,
            bytes: repo.bytes,
            untagged: repo.untagged,
            old: with_old.then_some(repo.old),
            usd_month: raw.cents_text(divisor)?,
        });
    }
    Ok((lines, total.cents_text(1)?))
}

#[cfg(test)]
mod tests {
    use super::{bill_documents, bill_documents_labeled, BillOptions};
    use crate::money::days_from_civil;
    use crate::price::{load_prices_str, prices_builtin};
    use rust_decimal::Decimal;

    fn doc(body: &str) -> String {
        format!(r#"{{"imageDetails":[{body}]}}"#)
    }

    fn image(repo: &str, bytes: u64, extra: &str) -> String {
        format!(r#"{{"repositoryName":"{repo}","imageSizeInBytes":{bytes}{extra}}}"#)
    }

    fn opts(as_of: Option<&str>) -> BillOptions {
        BillOptions {
            as_of: as_of.map(str::to_string),
            ..BillOptions::default()
        }
    }

    #[test]
    fn claim_c1_one_element_is_one_image_and_duplicates_count() {
        let row = image("app", 10, r#","imageDigest":"sha256:abc""#);
        let text = doc(&row);
        let bill = bill_documents(&[&text, &text], &prices_builtin(), &opts(None)).unwrap();
        assert_eq!(bill.total_images, 2);
        assert_eq!(bill.total_bytes, 20);
    }

    #[test]
    fn claim_c2_forty_gibibytes_match_the_page_example() {
        let text = doc(&image("app", 40 * 1_073_741_824, ""));
        let bill = bill_documents(&[&text], &prices_builtin(), &opts(None)).unwrap();
        assert_eq!(bill.total_usd, "4.00");
        let twenty = doc(&image("app", 20 * 1_073_741_824, ""));
        let bill = bill_documents(&[&twenty], &prices_builtin(), &opts(None)).unwrap();
        assert_eq!(bill.total_usd, "2.00");
    }

    #[test]
    fn claim_c4_missing_status_is_standard() {
        let text = doc(&image("app", 1_073_741_824, ""));
        let bill = bill_documents(&[&text], &prices_builtin(), &opts(None)).unwrap();
        assert_eq!(bill.total_usd, "0.10");
        assert!(bill.warnings.is_empty());
    }

    #[test]
    fn claim_c5_archive_tiers_do_not_add_retrieval() {
        let one = doc(&image("app", 1_073_741_824, r#","imageStatus":"ARCHIVED""#));
        let bill = bill_documents(&[&one], &prices_builtin(), &opts(None)).unwrap();
        assert_eq!(bill.total_usd, "0.10");
        let boundary = 150 * 1024 * 1_073_741_824;
        let over = doc(&image(
            "cold",
            boundary + 1_073_741_824,
            r#","imageStatus":"ARCHIVED""#,
        ));
        let bill = bill_documents(&[&over], &prices_builtin(), &opts(None)).unwrap();
        assert_eq!(bill.total_usd, "15360.07");
    }

    #[test]
    fn claim_c6_activating_and_unknown_are_unpriced() {
        let text = doc(&format!(
            "{},{}",
            image("app", 1_073_741_824, r#","imageStatus":"ACTIVATING""#),
            image("app", 1_073_741_824, r#","imageStatus":"SECRET-CLASS""#)
        ));
        let bill = bill_documents(&[&text], &prices_builtin(), &opts(None)).unwrap();
        assert_eq!(bill.total_usd, "0.00");
        assert_eq!(bill.total_images, 2);
        assert_eq!(bill.total_bytes, 2 * 1_073_741_824);
        assert_eq!(bill.warnings.len(), 1);
        assert!(bill.warnings[0].contains("images without a built-in rate: 2"));
        assert!(!bill.warnings[0].contains("SECRET"));
        assert!(!bill.warnings[0].contains("ACTIVATING"));
    }

    #[test]
    fn claim_c8_savings_age_omitted_without_as_of() {
        let text = doc(&image("app", 1_073_741_824, ""));
        let bill = bill_documents(&[&text], &prices_builtin(), &opts(None)).unwrap();
        assert!(bill.old.is_none());
        assert_eq!(bill.total_usd, "0.10");
        assert!(bill.repos[0].old.is_none());
    }

    #[test]
    fn claim_c9_age_uses_the_threshold_and_hides_a_missing_push() {
        let as_of = days_from_civil(2026, 11, 9);
        let exactly = (as_of - 30) * 86_400;
        let older = (as_of - 31) * 86_400;
        let text = doc(&format!(
            "{},{},{}",
            image(
                "app",
                1,
                &format!(r#","imagePushedAt":{exactly},"imageDigest":"sha256:keep""#)
            ),
            image(
                "app",
                1,
                &format!(r#","imagePushedAt":{older},"imageDigest":"sha256:old""#)
            ),
            image("app", 1, r#","imageDigest":"sha256:secret-missing""#)
        ));
        let bill = bill_documents(&[&text], &prices_builtin(), &opts(Some("2026-11-09"))).unwrap();
        let old = bill.old.as_ref().unwrap();
        assert_eq!(old.len(), 1);
        assert_eq!(old[0].digest, "sha256:old");
        assert_eq!(old[0].age_days, Some(31));
        assert!(bill.warnings[0].contains("images skipped for age: 1"));
        assert!(!bill.warnings[0].contains("secret"));
    }

    #[test]
    fn claim_c17_prices_override_changes_the_bill() {
        let mut text = include_str!("../prices/ecr-storage.json").to_string();
        text = text.replacen("\"standard\": \"0.10\"", "\"standard\": \"0.50\"", 1);
        text = text.replacen(
            "\"source_url\": \"https://aws.amazon.com/ecr/pricing/\"",
            "\"source_url\": \"https://example.test/quote\"",
            1,
        );
        let prices = load_prices_str(&text).unwrap();
        let doc = doc(&image("app", 1_073_741_824, ""));
        let bill = bill_documents(&[&doc], &prices, &opts(None)).unwrap();
        assert_eq!(bill.total_usd, "0.50");
        assert_eq!(bill.source_url, "https://example.test/quote");
    }

    #[test]
    fn two_archive_repos_split_the_tier() {
        let left = doc(&image("aa", 1_073_741_824, r#","imageStatus":"ARCHIVED""#));
        let right = doc(&image("bb", 1_073_741_824, r#","imageStatus":"ARCHIVED""#));
        let bill = bill_documents(&[&left, &right], &prices_builtin(), &opts(None)).unwrap();
        assert_eq!(bill.total_usd, "0.20");
        assert_eq!(bill.repos[0].usd_month, "0.10");
        assert_eq!(bill.repos[1].usd_month, "0.10");
    }

    #[test]
    fn empty_details_are_a_zero_bill_with_a_policy() {
        let bill =
            bill_documents(&[r#"{"imageDetails":[]}"#], &prices_builtin(), &opts(None)).unwrap();
        assert_eq!(bill.total_images, 0);
        assert_eq!(bill.total_usd, "0.00");
        assert!(bill.policy_json.contains("Not executed") || bill.policy_json.contains("untagged"));
        assert!(bill.policy_json.contains("\"type\": \"expire\""));
    }

    #[test]
    fn no_documents_is_rejected() {
        let err = bill_documents(&[], &prices_builtin(), &opts(None)).unwrap_err();
        assert_eq!(err.message(), "no inventory files");
    }

    #[test]
    fn bad_as_of_is_rejected() {
        let err = bill_documents(
            &[r#"{"imageDetails":[]}"#],
            &prices_builtin(),
            &opts(Some("2025-02-29")),
        )
        .unwrap_err();
        assert_eq!(err.message(), "invalid --as-of");
    }

    #[test]
    fn claim_c15_small_amount_rounds_to_the_cent() {
        let tiny = doc(&image("aa", 1, ""));
        let bill = bill_documents(&[&tiny], &prices_builtin(), &opts(None)).unwrap();
        assert_eq!(bill.repos[0].usd_month, "0.00");
        assert_eq!(bill.total_usd, "0.00");
        assert_eq!(bill.total_usd.split_once('.').unwrap().1.len(), 2);

        // Just over half a cent at 0.10 USD per GiB. Each row rounds to 0.01.
        // The rows add to 0.03. The unrounded sum rounds to 0.02.
        let rows = format!(
            "{},{},{}",
            image("aa", 53_687_092, ""),
            image("bb", 53_687_092, ""),
            image("cc", 53_687_092, "")
        );
        let bill = bill_documents(&[&doc(&rows)], &prices_builtin(), &opts(None)).unwrap();
        let mut shown = Decimal::ZERO;
        for repo in &bill.repos {
            assert_eq!(repo.usd_month, "0.01");
            assert_eq!(repo.usd_month.split_once('.').unwrap().1.len(), 2);
            shown += repo.usd_month.parse::<Decimal>().unwrap();
        }
        assert_eq!(shown, Decimal::new(3, 2));
        assert_eq!(bill.total_usd, "0.02");
    }

    #[test]
    fn exact_half_cent_with_a_custom_rate_rounds_up() {
        // 1 byte at 5368709.12 and 5 bytes at 1073741.824 are both exactly 0.005 USD.
        for (rate, bytes) in [("5368709.12", 1u64), ("1073741.824", 5)] {
            let mut prices = prices_builtin();
            prices.standard = rate.parse().unwrap();
            let bill =
                bill_documents(&[&doc(&image("aa", bytes, ""))], &prices, &opts(None)).unwrap();
            assert_eq!(bill.repos[0].usd_month, "0.01", "{rate} x {bytes} B");
            assert_eq!(bill.total_usd, "0.01", "{rate} x {bytes} B");
        }
    }

    #[test]
    fn thousand_small_repositories_total_is_the_unrounded_sum() {
        // 53_687_091 bytes is just under half a cent. 53_687_092 is just over.
        // A thousand of the first round to 0.00 per row and about 5 USD together.
        // A thousand of the second round to 0.01 per row (10.00 if added) and 5 USD together.
        let under = inventory(1_000, 53_687_091);
        let bill = bill_documents(&[&under], &prices_builtin(), &opts(None)).unwrap();
        assert_eq!(bill.repos.len(), 1_000);
        assert!(bill.repos.iter().all(|repo| repo.usd_month == "0.00"));
        assert_eq!(bill.total_usd, "5.00");

        let over = inventory(1_000, 53_687_092);
        let bill = bill_documents(&[&over], &prices_builtin(), &opts(None)).unwrap();
        assert_eq!(bill.repos.len(), 1_000);
        assert!(bill.repos.iter().all(|repo| repo.usd_month == "0.01"));
        assert_eq!(bill.total_usd, "5.00");
    }

    fn inventory(repos: usize, bytes: u64) -> String {
        let mut body = String::with_capacity(repos * 64);
        for n in 0..repos {
            if n > 0 {
                body.push(',');
            }
            body.push_str(&image(&format!("r{n:04}"), bytes, ""));
        }
        let text = doc(&body);
        assert!(text.len() <= 300 * 1024, "json is {} bytes", text.len());
        text
    }

    #[test]
    fn huge_archive_tier_without_archive_bytes_is_fine() {
        let base = include_str!("../prices/ecr-storage.json");
        for tier in ["16777215", "16777216", "4294967295"] {
            let text = base.replacen(
                "\"archive_tier_tb\": 150",
                &format!("\"archive_tier_tb\": {tier}"),
                1,
            );
            let prices = load_prices_str(&text).unwrap();
            let d = doc(&image("app", 1_073_741_824, ""));
            let bill = bill_documents(&[&d], &prices, &opts(None)).unwrap();
            assert_eq!(bill.total_usd, "0.10", "tier {tier}");
        }
    }

    #[test]
    fn large_archive_rate_splits_exactly_when_the_total_fits() {
        let base = include_str!("../prices/ecr-storage.json");
        let text = base.replacen(
            "\"archive_first_150_tb\": \"0.10\"",
            "\"archive_first_150_tb\": \"100000000000000000000\"",
            1,
        );
        let prices = load_prices_str(&text).unwrap();
        let archived = r#","imageStatus":"ARCHIVED""#;
        let d = doc(&format!(
            "{},{}",
            image("aa", 1_073_741_824, archived),
            image("bb", 1_073_741_824, archived)
        ));
        let bill = bill_documents(&[&d], &prices, &opts(None)).unwrap();
        assert_eq!(bill.repos[0].usd_month, "100000000000000000000.00");
        assert_eq!(bill.total_usd, "200000000000000000000.00");
    }

    #[test]
    fn large_archive_at_5368709_12_is_pinned() {
        let base = include_str!("../prices/ecr-storage.json");
        let text = base.replacen(
            "\"archive_first_150_tb\": \"0.10\"",
            "\"archive_first_150_tb\": \"5368709.12\"",
            1,
        );
        let prices = load_prices_str(&text).unwrap();
        let archived = r#","imageStatus":"ARCHIVED""#;
        let d = doc(&format!(
            "{},{}",
            image("aa", 100_000_000_000_000_001, archived),
            image("bb", 50_000_000_000_000_003, archived)
        ));
        let bill = bill_documents(&[&d], &prices, &opts(None)).unwrap();
        assert_eq!(bill.repos[0].usd_month, "549762325978.02");
        assert_eq!(bill.repos[1].usd_month, "274881162989.01");
        assert_eq!(bill.total_usd, "824643488967.03");
    }

    #[test]
    fn three_archive_repos_at_5368709_12_are_pinned() {
        let base = include_str!("../prices/ecr-storage.json");
        let text = base.replacen(
            "\"archive_first_150_tb\": \"0.10\"",
            "\"archive_first_150_tb\": \"5368709.12\"",
            1,
        );
        let prices = load_prices_str(&text).unwrap();
        let archived = r#","imageStatus":"ARCHIVED""#;
        let d = doc(&format!(
            "{},{},{}",
            image("aa", 100_000_000_000_000_001, archived),
            image("bb", 50_000_000_000_000_003, archived),
            image("cc", 30_000_000_000_000_007, archived)
        ));
        let bill = bill_documents(&[&d], &prices, &opts(None)).unwrap();
        assert_eq!(bill.repos[0].usd_month, "458136358191.36");
        assert_eq!(bill.repos[1].usd_month, "229068179095.68");
        assert_eq!(bill.repos[2].usd_month, "137440907457.41");
        assert_eq!(bill.total_usd, "824645444744.44");
    }

    /// A prices quote with another rate in one field of the built-in file.
    fn prices_with(field: &str, rate: &str) -> crate::price::Prices {
        let base = include_str!("../prices/ecr-storage.json");
        let from = base
            .lines()
            .find(|line| line.contains(&format!("\"{field}\"")))
            .unwrap();
        let text = base.replacen(from, &format!("    \"{field}\": \"{rate}\","), 1);
        load_prices_str(&text).unwrap()
    }

    // Red before the fix: the last archive row was `archive_usd - used`, and the
    // 28-digit quotient of the first row left it just under the half cent.
    #[test]
    fn a_shared_archive_row_that_is_an_exact_half_cent_rounds_up() {
        // 1298153865216 bytes are 1209 GiB: 1209 x 0.005 = 6.045 exactly.
        let prices = prices_with("archive_first_150_tb", "0.005");
        let archived = r#","imageStatus":"ARCHIVED""#;
        let d = doc(&format!(
            "{},{}",
            image("aa", 58_355_734_422_394, archived),
            image("bb", 1_298_153_865_216, archived)
        ));
        let bill = bill_documents(&[&d], &prices, &opts(None)).unwrap();
        assert_eq!(bill.repos[0].usd_month, "271.74");
        assert_eq!(bill.repos[1].usd_month, "6.05");
        assert_eq!(bill.total_usd, "277.79");
    }

    // Red before the fix: with a tier boundary crossed, the rows were shares of
    // the total even when both tier rates are the same, and `bb` printed 6.04.
    #[test]
    fn a_crossed_tier_with_one_rate_on_both_sides_is_priced_per_repository() {
        let mut prices = prices_with("archive_first_150_tb", "0.005");
        prices.archive_over = prices.archive_first;
        prices.archive_tier_tb = 1;
        let archived = r#","imageStatus":"ARCHIVED""#;
        let d = doc(&format!(
            "{},{}",
            image("aa", 58_355_734_422_394, archived),
            image("bb", 1_298_153_865_216, archived)
        ));
        let bill = bill_documents(&[&d], &prices, &opts(None)).unwrap();
        assert_eq!(bill.repos[1].usd_month, "6.05");
        assert_eq!(bill.total_usd, "277.79");
    }

    // Red before the fix: `checked_mul` rounded the product when the 96-bit
    // mantissa overflowed, so 1 GiB at ...403.555 printed ...403.55.
    #[test]
    fn a_product_that_would_lose_digits_is_an_error_or_exact_never_rounded() {
        for (rate, shown) in [
            ("7900827040092728403.555", "7900827040092728403.56"),
            ("751746635355857900.585", "751746635355857900.59"),
            ("7882933473361814224.005", "7882933473361814224.01"),
        ] {
            let prices = prices_with("standard", rate);
            let d = doc(&image("aa", 1_073_741_824, ""));
            let bill = bill_documents(&[&d], &prices, &opts(None)).unwrap();
            assert_eq!(bill.repos[0].usd_month, shown, "{rate}");
            assert_eq!(bill.total_usd, shown, "{rate}");
        }
    }

    // Red before the fix: the quotient by 2^30 was rounded to 28 digits before
    // the cent, so a value just under a half cent printed one cent too many.
    #[test]
    fn a_quotient_just_under_a_half_cent_is_not_rounded_up() {
        for (rate, bytes, shown) in [
            ("0.894698637813", 99_231_259_014_473_123_u64, "82684748.12"),
            ("9683085068513257837", 126_819_283, "1143665896371805327.83"),
            ("0.2929751363929", 86_918_756_001_175_831, "23716161.39"),
        ] {
            let prices = prices_with("standard", rate);
            let d = doc(&image("app", bytes, ""));
            let bill = bill_documents(&[&d], &prices, &opts(None)).unwrap();
            assert_eq!(bill.repos[0].usd_month, shown, "{rate}");
            assert_eq!(bill.total_usd, shown, "{rate}");
        }
    }

    // Expected values from Python `fractions.Fraction`: a row is its Standard
    // cost plus the archive total times its share of the archived bytes, rounded
    // once; the total is the exact sum of the unrounded rows. Red before the
    // fix: the share and the sum were quotients of 28 digits.
    #[test]
    fn rows_and_total_of_a_crossed_tier_are_exact_fractions_rounded_once() {
        let archived = r#","imageStatus":"ARCHIVED""#;
        // (standard, first, over, tier, [(repo, standard bytes, archive bytes)], rows, total)
        type Case<'a> = (
            &'a str,
            &'a str,
            &'a str,
            u32,
            &'a [(&'a str, u64, u64)],
            &'a [&'a str],
            &'a str,
        );
        let cases: [Case; 2] = [
            (
                "76231213083644066",
                "3350100371752275100277",
                "82691401530662279",
                2,
                &[
                    ("r0", 2_466_353, 3_977_681_073_400_848),
                    ("r1", 0, 5_296_801_097),
                ],
                &["7167157715925504992623892.71", "9544005200673628659.07"],
                "7167167259930705666252551.78",
            ),
            (
                "6716067728621937881",
                "1518447288360233834591",
                "5320268721.883129683388",
                1,
                &[
                    ("r0", 12_745, 16_974),
                    ("r1", 2_033_848_837_505, 0),
                    ("r2", 0, 567_326_934_754),
                ],
                &[
                    "24083741526890448.95",
                    "12721369548200233071029.35",
                    "802293462390950512443847.47",
                ],
                "815014856022892272405325.77",
            ),
        ];
        for (standard, first, over, tier, repos, rows, total) in cases {
            let mut prices = prices_with("standard", standard);
            prices.archive_first = first.parse().unwrap();
            prices.archive_over = over.parse().unwrap();
            prices.archive_tier_tb = tier;
            let mut images = Vec::new();
            for (name, standard_bytes, archive_bytes) in repos {
                if *standard_bytes > 0 {
                    images.push(image(name, *standard_bytes, ""));
                }
                if *archive_bytes > 0 {
                    images.push(image(name, *archive_bytes, archived));
                }
            }
            let d = doc(&images.join(","));
            let bill = bill_documents(&[&d], &prices, &opts(None)).unwrap();
            let shown: Vec<&str> = bill.repos.iter().map(|r| r.usd_month.as_str()).collect();
            assert_eq!(shown, rows, "{standard}");
            assert_eq!(bill.total_usd, total, "{standard}");
        }
    }

    #[test]
    fn extreme_rate_is_priced_exactly() {
        let mut text = include_str!("../prices/ecr-storage.json").to_string();
        text = text.replacen(
            "\"standard\": \"0.10\"",
            "\"standard\": \"79228162514264337593543950335\"",
            1,
        );
        let prices = load_prices_str(&text).unwrap();
        let doc = doc(&image("app", 2 * 1_073_741_824, ""));
        let bill = bill_documents(&[&doc], &prices, &opts(None)).unwrap();
        assert_eq!(bill.total_usd, "158456325028528675187087900670.00");
    }

    #[test]
    fn zero_policy_counts_are_rejected() {
        let empty = r#"{"imageDetails":[]}"#;
        for (older, untagged, keep) in [(0, 14, 10), (30, 0, 10), (30, 14, 0)] {
            let options = BillOptions {
                older_than_days: older,
                untagged_days: untagged,
                keep,
                ..BillOptions::default()
            };
            let err = bill_documents(&[empty], &prices_builtin(), &options).unwrap_err();
            assert_eq!(err.message(), "policy counts must be positive");
        }
    }

    #[test]
    fn a_file_label_prefixes_errors_that_lack_a_file_number() {
        let labels = ["a.json".to_string(), "b.json".to_string()];
        let good = r#"{"imageDetails":[]}"#;
        let err = bill_documents_labeled(
            &[good, "nope"],
            &labels,
            &prices_builtin(),
            &BillOptions::default(),
        )
        .unwrap_err();
        assert_eq!(err.message(), "b.json: not JSON");
        let err = bill_documents_labeled(
            &[good, r#"{"imageDetails":[{"imageSizeInBytes":1}]}"#],
            &labels,
            &prices_builtin(),
            &BillOptions::default(),
        )
        .unwrap_err();
        assert_eq!(err.message(), "file 2: image 1: missing repositoryName");
    }

    #[test]
    fn byte_totals_that_overflow_are_an_error() {
        let text = doc(&format!(
            "{},{}",
            image("app", u64::MAX, ""),
            image("app", 1, "")
        ));
        let err = bill_documents(&[&text], &prices_builtin(), &opts(None)).unwrap_err();
        assert_eq!(err.message(), "byte total does not fit");
    }
}
