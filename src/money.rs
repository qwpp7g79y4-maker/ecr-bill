use rust_decimal::{Decimal, RoundingStrategy};

use crate::error::Error;

/// 1 GB. The billing unit this tool uses for the page's per-GB rates.
pub const GIB: u64 = 1_073_741_824;

/// `bytes / 1024^3`.
#[must_use]
pub fn gb_from_bytes(bytes: Decimal) -> Decimal {
    bytes / Decimal::from(GIB)
}

/// Days since 1970-01-01 in the civil calendar. Howard Hinnant's algorithm.
/// `year`, `month`, and `day` must be a real Gregorian date.
#[must_use]
pub fn days_from_civil(year: i32, month: u32, day: u32) -> i64 {
    let mut y = i64::from(year);
    let m = i64::from(month);
    let d = i64::from(day);
    if m <= 2 {
        y -= 1;
    }
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = if m > 2 { m - 3 } else { m + 9 };
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// The civil date of a day count since 1970-01-01 (inverse of [`days_from_civil`]).
pub(crate) fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (yoe + era * 400 + i64::from(m <= 2), m, d)
}

/// Decimal text with no exponent. Trailing zeros that `normalize` drops stay dropped.
#[must_use]
pub fn format_dec(value: Decimal) -> String {
    let text = value.normalize().to_string();
    if text.contains(['e', 'E']) {
        return value.round_dp(12).normalize().to_string();
    }
    text
}

/// Half away from zero, to the cent (two decimal places).
#[must_use]
pub(crate) fn round_cents(value: Decimal) -> Decimal {
    value.round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero)
}

/// Dollar text rounded half away from zero to the cent.
#[must_use]
pub fn format_usd(value: Decimal) -> String {
    let value = round_cents(value);
    let negative = value.is_sign_negative() && !value.is_zero();
    let text = format_dec(value.abs());
    let body = match text.split_once('.') {
        Some((whole, frac)) if frac.len() >= 2 => format!("{whole}.{frac}"),
        Some((whole, frac)) => format!("{whole}.{frac}{}", "0".repeat(2 - frac.len())),
        None => format!("{text}.00"),
    };
    if negative {
        format!("-{body}")
    } else {
        body
    }
}

/// Why a rate text is not a number.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum MoneyError {
    /// Not digits with at most one inner decimal point, or too long.
    NotDecimal,
    /// A decimal, but one the decimal type can only hold rounded.
    TooPrecise,
}

pub(crate) fn parse_money(raw: &str) -> Result<Decimal, MoneyError> {
    let text = raw.trim_matches([' ', '\t', '\r', '\n']);
    if text.is_empty() || text.len() > 40 {
        return Err(MoneyError::NotDecimal);
    }
    let bytes = text.as_bytes();
    let mut dot = false;
    for (i, byte) in bytes.iter().enumerate() {
        match byte {
            b'0'..=b'9' => {}
            b'.' if !dot && i > 0 && i + 1 < bytes.len() => dot = true,
            _ => return Err(MoneyError::NotDecimal),
        }
    }
    let value: Decimal = text.parse().map_err(|_| MoneyError::NotDecimal)?;
    // The parser rounds a text of more than 28 digits instead of failing.
    // A rate that is not the number written would price every image with
    // another number, so it is refused. Zeros at either end change nothing.
    let (whole, fraction) = text.split_once('.').unwrap_or((text, ""));
    let whole = whole.trim_start_matches('0');
    let fraction = fraction.trim_end_matches('0');
    let shown = value.to_string();
    let (shown_whole, shown_fraction) = shown.split_once('.').unwrap_or((&shown, ""));
    let shown_whole = shown_whole.trim_start_matches('0');
    let shown_fraction = shown_fraction.trim_end_matches('0');
    if whole != shown_whole || fraction != shown_fraction {
        return Err(MoneyError::TooPrecise);
    }
    Ok(value)
}

pub(crate) fn valid_date(text: &str) -> bool {
    let bytes = text.as_bytes();
    if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return false;
    }
    let digits_only = |range: std::ops::Range<usize>| bytes[range].iter().all(u8::is_ascii_digit);
    if !digits_only(0..4) || !digits_only(5..7) || !digits_only(8..10) {
        return false;
    }
    let Ok(year) = text[0..4].parse::<i32>() else {
        return false;
    };
    let Ok(month) = text[5..7].parse::<u32>() else {
        return false;
    };
    let Ok(day) = text[8..10].parse::<u32>() else {
        return false;
    };
    if !(1..=12).contains(&month) {
        return false;
    }
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let max = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return false,
    };
    (1..=max).contains(&day)
}

pub(crate) fn ymd_days(text: &str) -> Option<i64> {
    if !valid_date(text) {
        return None;
    }
    Some(days_from_civil(
        text[0..4].parse().ok()?,
        text[5..7].parse().ok()?,
        text[8..10].parse().ok()?,
    ))
}

/// Unsigned 512-bit integer, least significant limb first. Every operation
/// returns `None` instead of wrapping, so a number is never silently cut.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Big([u64; 8]);

impl Big {
    const ZERO: Big = Big([0; 8]);

    fn from_u128(value: u128) -> Big {
        let mut limbs = [0; 8];
        limbs[0] = value as u64;
        limbs[1] = (value >> 64) as u64;
        Big(limbs)
    }

    fn is_zero(&self) -> bool {
        self.0.iter().all(|limb| *limb == 0)
    }

    fn mul_u64(self, factor: u64) -> Option<Big> {
        let mut out = [0u64; 8];
        let mut carry = 0u128;
        for (slot, limb) in out.iter_mut().zip(self.0) {
            let step = u128::from(limb) * u128::from(factor) + carry;
            *slot = step as u64;
            carry = step >> 64;
        }
        (carry == 0).then_some(Big(out))
    }

    fn mul_pow10(self, mut power: u32) -> Option<Big> {
        let mut value = self;
        while power > 0 {
            let step = power.min(19);
            value = value.mul_u64(10u64.pow(step))?;
            power -= step;
        }
        Some(value)
    }

    fn checked_add(self, other: Big) -> Option<Big> {
        let mut out = [0u64; 8];
        let mut carry = false;
        for ((slot, a), b) in out.iter_mut().zip(self.0).zip(other.0) {
            let (sum, c1) = a.overflowing_add(b);
            let (sum, c2) = sum.overflowing_add(u64::from(carry));
            *slot = sum;
            carry = c1 || c2;
        }
        (!carry).then_some(Big(out))
    }

    /// Floor quotient and remainder. `divisor` must not be zero.
    fn divmod_u64(self, divisor: u64) -> (Big, u64) {
        let mut out = [0u64; 8];
        let mut rest = 0u128;
        for i in (0..8).rev() {
            let step = (rest << 64) | u128::from(self.0[i]);
            out[i] = (step / u128::from(divisor)) as u64;
            rest = step % u128::from(divisor);
        }
        (Big(out), rest as u64)
    }

    fn decimal_digits(self) -> String {
        if self.is_zero() {
            return "0".to_string();
        }
        let mut chunks = Vec::new();
        let mut value = self;
        while !value.is_zero() {
            let (quotient, rest) = value.divmod_u64(10_000_000_000_000_000_000);
            chunks.push(rest);
            value = quotient;
        }
        let mut text = chunks.pop().map_or_else(String::new, |top| top.to_string());
        for chunk in chunks.iter().rev() {
            text.push_str(&format!("{chunk:019}"));
        }
        text
    }
}

/// Decimals every amount is held with: an amount is a whole number over
/// `10^28 * 2^30`, 28 being the most decimals a rate can have and 2^30 the
/// bytes in one gibibyte. A byte count times a rate is then a whole number
/// with nothing dropped.
const AMOUNT_DECIMALS: u32 = 28;

/// An exact amount of dollars: `Big / (10^28 * 2^30)`. Nothing is rounded
/// until [`Amount::cents_text`] writes the cent, once, half away from zero.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct Amount(Big);

fn no_fit() -> Error {
    Error::new("cost does not fit")
}

impl Amount {
    pub(crate) const ZERO: Amount = Amount(Big::ZERO);

    pub(crate) fn add(self, other: Amount) -> Result<Amount, Error> {
        self.0.checked_add(other.0).map(Amount).ok_or_else(no_fit)
    }

    /// `self * factor`, exact.
    pub(crate) fn times(self, factor: u64) -> Result<Amount, Error> {
        self.0.mul_u64(factor).map(Amount).ok_or_else(no_fit)
    }

    /// The dollars of `self / divisor` with two decimals, rounded half away
    /// from zero. This is the only rounding of an amount.
    pub(crate) fn cents_text(self, divisor: u64) -> Result<String, Error> {
        // floor(200 x) is odd exactly when x has a half cent or more past the
        // cent below it, so (floor(200 x) + 1) / 2 is x rounded half up.
        let doubled = self.0.mul_u64(200).ok_or_else(no_fit)?;
        let (value, _) = doubled.divmod_u64(divisor.max(1));
        let (value, _) = value.divmod_u64(1 << 30);
        let (value, _) = value.divmod_u64(10_u64.pow(AMOUNT_DECIMALS / 2));
        let (value, _) = value.divmod_u64(10_u64.pow(AMOUNT_DECIMALS / 2));
        let up = value.checked_add(Big::from_u128(1)).ok_or_else(no_fit)?;
        let (cents, _) = up.divmod_u64(2);
        let digits = cents.decimal_digits();
        let digits = format!("{digits:0>3}");
        let (whole, frac) = digits.split_at(digits.len() - 2);
        Ok(format!("{whole}.{frac}"))
    }
}

/// `bytes * rate / 1024^3` as an exact amount.
pub(crate) fn charge(bytes: u64, rate: Decimal) -> Result<Amount, Error> {
    let mantissa = u128::try_from(rate.mantissa()).map_err(|_| no_fit())?;
    let product = Big::from_u128(mantissa)
        .mul_u64(bytes)
        .and_then(|value| value.mul_pow10(AMOUNT_DECIMALS - rate.scale()))
        .ok_or_else(no_fit)?;
    Ok(Amount(product))
}

pub(crate) fn checked_add(slot: &mut u64, add: u64) -> Result<(), Error> {
    *slot = slot
        .checked_add(add)
        .ok_or_else(|| Error::new("byte total does not fit"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        charge, days_from_civil, format_usd, gb_from_bytes, parse_money, valid_date, Amount, Big,
        GIB,
    };
    use rust_decimal::Decimal;

    #[test]
    fn parse_money_trims_only_ascii_whitespace() {
        assert!(parse_money(" 1").is_ok());
        assert!(parse_money("\t1\r\n").is_ok());
        assert!(parse_money("\u{a0}1").is_err());
        assert!(parse_money("1\u{a0}").is_err());
        assert!(parse_money("\u{2028}1").is_err());
        assert!(parse_money("\u{3000}1").is_err());
    }

    #[test]
    fn gibibyte_matches_the_page_unit() {
        assert_eq!(GIB, 1_073_741_824);
        assert_eq!(gb_from_bytes(Decimal::from(GIB)), Decimal::ONE);
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(days_from_civil(2024, 1, 15) * 86_400, 1_705_276_800);
        assert_eq!(days_from_civil(2020, 1, 1) * 86_400, 1_577_836_800);
    }

    #[test]
    fn usd_rounds_half_away_from_zero_to_the_cent() {
        assert_eq!(format_usd(Decimal::new(1, 1)), "0.10");
        assert_eq!(format_usd(Decimal::from(4)), "4.00");
        assert_eq!(format_usd(Decimal::ZERO), "0.00");
        assert_eq!(format_usd(Decimal::new(5, 3)), "0.01");
        assert_eq!(format_usd(Decimal::new(4, 3)), "0.00");
        assert_eq!(format_usd(Decimal::new(-5, 3)), "-0.01");
        assert_eq!(format_usd(Decimal::new(1, 10)), "0.00");
        let text = format_usd(Decimal::new(1, 10));
        let frac = text.split_once('.').unwrap().1;
        assert_eq!(frac.len(), 2);
    }

    fn dollars(bytes: u64, rate: &str) -> String {
        charge(bytes, rate.parse().unwrap())
            .unwrap()
            .cents_text(1)
            .unwrap()
    }

    #[test]
    fn a_charge_is_exact_for_any_rate_the_decimal_type_holds() {
        // 2 GiB at the largest rate: 2 x 79228162514264337593543950335.
        assert_eq!(
            dollars(2 * GIB, "79228162514264337593543950335"),
            "158456325028528675187087900670.00"
        );
        assert_eq!(dollars(u64::MAX, "0"), "0.00");
        assert_eq!(dollars(0, "79228162514264337593543950335"), "0.00");
        // Python Fraction: 99231259014473123 x 0.894698637813 / 2^30 is
        // 82684748.12499999999999999999906..., just under a half cent.
        assert_eq!(
            dollars(99_231_259_014_473_123, "0.894698637813"),
            "82684748.12"
        );
        assert_eq!(
            dollars(126_819_283, "9683085068513257837"),
            "1143665896371805327.83"
        );
        assert_eq!(
            dollars(86_918_756_001_175_831, "0.2929751363929"),
            "23716161.39"
        );
        // 18446744073709551615 x 10000000000 / 2^30 = 171798691839999999990.68677...
        assert_eq!(dollars(u64::MAX, "10000000000"), "171798691839999999990.69");
        // Most decimals and most bytes together do not overflow.
        assert_eq!(dollars(u64::MAX, "0.0000000000000000000000000001"), "0.00");
    }

    #[test]
    fn a_sum_of_amounts_is_exact() {
        let big = charge(GIB, "99999999999999999999999999".parse().unwrap()).unwrap();
        let tenth = charge(GIB, "0.10".parse().unwrap()).unwrap();
        assert_eq!(
            big.add(tenth).unwrap().cents_text(1).unwrap(),
            "99999999999999999999999999.10"
        );
        let huge = charge(GIB, "79228162514264337593543950335".parse().unwrap()).unwrap();
        assert_eq!(
            huge.add(tenth).unwrap().cents_text(1).unwrap(),
            "79228162514264337593543950335.10"
        );
        assert_eq!(Amount::ZERO.add(Amount::ZERO).unwrap(), Amount::ZERO);
        assert_eq!(
            huge.times(3).unwrap().cents_text(3).unwrap(),
            huge.cents_text(1).unwrap()
        );
    }

    #[test]
    fn an_amount_that_overflows_the_integer_is_an_error() {
        let huge = charge(u64::MAX, Decimal::MAX).unwrap();
        let mut value = huge;
        let mut failed = false;
        for _ in 0..400 {
            match value.times(u64::MAX) {
                Ok(next) => value = next,
                Err(err) => {
                    assert_eq!(err.message(), "cost does not fit");
                    failed = true;
                    break;
                }
            }
        }
        assert!(failed);
    }

    #[test]
    fn cents_round_once_half_away_from_zero_and_pad() {
        // An exact half cent rounds up, one part in 2^30 under it does not.
        assert_eq!(dollars(1, "5368709.12"), "0.01");
        assert_eq!(dollars(5, "1073741.824"), "0.01");
        assert_eq!(dollars(1, "5368709.119999999"), "0.00");
        assert_eq!(dollars(GIB, "0.005"), "0.01");
        assert_eq!(dollars(GIB, "0.0049999999999999999999999999"), "0.00");
        assert_eq!(dollars(GIB, "0.1"), "0.10");
        assert_eq!(dollars(GIB, "7"), "7.00");
        assert_eq!(
            dollars(GIB, "7900827040092728403.555"),
            "7900827040092728403.56"
        );
    }

    #[test]
    fn big_numbers_have_no_silent_wraparound() {
        let max = Big([u64::MAX; 8]);
        assert!(max.mul_u64(2).is_none());
        assert!(max.checked_add(Big::from_u128(1)).is_none());
        assert_eq!(Big::from_u128(0).decimal_digits(), "0");
        assert_eq!(
            Big::from_u128(u128::MAX).decimal_digits(),
            "340282366920938463463374607431768211455"
        );
        assert_eq!(Big::from_u128(10).divmod_u64(3), (Big::from_u128(3), 1));
    }

    #[test]
    fn money_rejects_exponent_sign_and_empty() {
        assert!(parse_money("0.10").is_ok());
        assert!(parse_money("1e2").is_err());
        assert_eq!(
            parse_money("0.1234567890123456789012345678901"),
            Err(super::MoneyError::TooPrecise)
        );
        assert!(parse_money("0.1000000000000000000000000000000").is_ok());
        assert!(parse_money("0000.10").is_ok());
        assert!(parse_money("-1").is_err());
        assert!(parse_money("").is_err());
        assert!(parse_money("+1").is_err());
    }

    #[test]
    fn dates_follow_the_calendar() {
        assert!(valid_date("2024-02-29"));
        assert!(!valid_date("2025-02-29"));
        assert!(!valid_date("2026-02-31"));
        assert!(valid_date("2026-11-09"));
        assert!(!valid_date("2026-+1-09"));
        assert!(!valid_date("2026-01-+9"));
        assert!(!valid_date("+026-01-09"));
    }
}

#[cfg(test)]
mod civil_tests {
    use super::{civil_from_days, days_from_civil};

    #[test]
    fn civil_from_days_inverts_days_from_civil() {
        for (y, m, d) in [
            (1970, 1, 1),
            (2000, 2, 29),
            (2026, 11, 20),
            (2027, 5, 8),
            (1969, 12, 31),
            (9999, 12, 31),
        ] {
            assert_eq!(
                civil_from_days(days_from_civil(y, m, d)),
                (i64::from(y), i64::from(m), i64::from(d))
            );
        }
    }
}
