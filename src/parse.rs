use rust_decimal::Decimal;
use serde_json::Value;

use crate::error::Error;
use crate::json_scan::{parse_json, reject_duplicate_keys};
use crate::money::{days_from_civil, valid_date};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Class {
    Standard,
    Archive,
    Unpriced,
}

#[derive(Debug, Clone)]
pub(crate) struct ImageRec {
    pub repository: String,
    pub digest: String,
    pub bytes: u64,
    pub tags: u32,
    pub untagged: bool,
    pub pushed_epoch: Option<i64>,
    pub class: Class,
}

pub(crate) fn parse_document(text: &str, file_no: u32) -> Result<Vec<ImageRec>, Error> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    reject_duplicate_keys(text)?;
    let value: Value = parse_json(text)?;
    let details = value
        .as_object()
        .and_then(|obj| obj.get("imageDetails"))
        .ok_or_else(|| Error::new("missing imageDetails"))?;
    let list = details
        .as_array()
        .ok_or_else(|| Error::new("missing imageDetails"))?;
    let mut out = Vec::with_capacity(list.len());
    for (index, item) in list.iter().enumerate() {
        let image_no = u64::try_from(index + 1).unwrap_or(u64::MAX);
        out.push(parse_image(item, file_no, image_no)?);
    }
    Ok(out)
}

fn parse_image(value: &Value, file_no: u32, image_no: u64) -> Result<ImageRec, Error> {
    let obj = value
        .as_object()
        .ok_or_else(|| Error::new(format!("file {file_no}: image {image_no}: not an object")))?;
    if let Some(registry) = obj.get("registryId") {
        let text = registry.as_str().unwrap_or("");
        if text.len() != 12 || !text.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(Error::new(format!(
                "file {file_no}: image {image_no}: invalid registryId"
            )));
        }
    }
    let repository = obj
        .get("repositoryName")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            Error::new(format!(
                "file {file_no}: image {image_no}: missing repositoryName"
            ))
        })?;
    if !valid_repository(repository) {
        return Err(Error::new(format!(
            "file {file_no}: image {image_no}: invalid repositoryName"
        )));
    }
    let digest = match obj.get("imageDigest") {
        None | Some(Value::Null) => "(none)".to_string(),
        // A missing digest is shown as `(none)`. A real digest that would look
        // like it (`(none)` itself, or anything that starts with a quote and so
        // could be taken for a quoted one) is wrapped in quotes, which keeps the
        // display one-to-one.
        Some(Value::String(text)) if !text.is_empty() && !text.contains([' ', '\t']) => {
            if text == "(none)" || text.starts_with('"') {
                format!("\"{text}\"")
            } else {
                text.clone()
            }
        }
        Some(_) => {
            return Err(Error::new(format!(
                "file {file_no}: image {image_no}: invalid imageDigest"
            )));
        }
    };
    let bytes = match obj.get("imageSizeInBytes") {
        Some(value) => json_u64(value).ok_or_else(|| {
            Error::new(format!(
                "file {file_no}: image {image_no}: invalid imageSizeInBytes"
            ))
        })?,
        None => {
            return Err(Error::new(format!(
                "file {file_no}: image {image_no}: missing imageSizeInBytes"
            )));
        }
    };
    let (tags, untagged) = tags_of(obj.get("imageTags"), file_no, image_no)?;
    let pushed_epoch = match obj.get("imagePushedAt") {
        None | Some(Value::Null) => None,
        Some(value) => Some(json_epoch(value).map_err(|()| {
            Error::new(format!(
                "file {file_no}: image {image_no}: invalid imagePushedAt"
            ))
        })?),
    };
    let class = class_of(obj.get("imageStatus"), file_no, image_no)?;
    Ok(ImageRec {
        repository: repository.to_string(),
        digest,
        bytes,
        tags,
        untagged,
        pushed_epoch,
        class,
    })
}

fn tags_of(value: Option<&Value>, file_no: u32, image_no: u64) -> Result<(u32, bool), Error> {
    let Some(value) = value else {
        return Ok((0, true));
    };
    if value.is_null() {
        return Ok((0, true));
    }
    let list = value.as_array().ok_or_else(|| {
        Error::new(format!(
            "file {file_no}: image {image_no}: invalid imageTags"
        ))
    })?;
    if list.is_empty() {
        return Ok((0, true));
    }
    for tag in list {
        let text = tag.as_str().ok_or_else(|| {
            Error::new(format!(
                "file {file_no}: image {image_no}: invalid image tag"
            ))
        })?;
        if text.is_empty() || text.len() > 300 {
            return Err(Error::new(format!(
                "file {file_no}: image {image_no}: invalid image tag"
            )));
        }
    }
    let count = u32::try_from(list.len()).map_err(|_| Error::new("image total does not fit"))?;
    Ok((count, false))
}

fn class_of(value: Option<&Value>, file_no: u32, image_no: u64) -> Result<Class, Error> {
    let Some(value) = value else {
        return Ok(Class::Standard);
    };
    if value.is_null() {
        return Ok(Class::Standard);
    }
    let text = value.as_str().ok_or_else(|| {
        Error::new(format!(
            "file {file_no}: image {image_no}: invalid imageStatus"
        ))
    })?;
    if text.chars().any(|c| c.is_whitespace() && !c.is_ascii()) {
        return Err(Error::new(format!(
            "file {file_no}: image {image_no}: invalid imageStatus"
        )));
    }
    let mut upper = text.trim_matches([' ', '\t', '\r', '\n']).to_string();
    upper.make_ascii_uppercase();
    Ok(match upper.as_str() {
        "" | "ACTIVE" => Class::Standard,
        "ARCHIVED" => Class::Archive,
        _ => Class::Unpriced,
    })
}

fn json_u64(value: &Value) -> Option<u64> {
    let text = value.as_number()?.to_string();
    if text.is_empty() || text.bytes().any(|byte| !byte.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}

fn json_epoch(value: &Value) -> Result<i64, ()> {
    match value {
        Value::String(text) => iso8601_epoch(text),
        Value::Number(number) => number_epoch(&number.to_string()),
        _ => Err(()),
    }
}

fn number_epoch(text: &str) -> Result<i64, ()> {
    if text.is_empty() || text.starts_with('+') || text.starts_with('-') {
        return Err(());
    }
    if text.contains(['e', 'E']) {
        return exponent_epoch(text);
    }
    if !plain_number(text) {
        return Err(());
    }
    let whole = text.split('.').next().unwrap_or("");
    whole.parse().map_err(|_| ())
}

fn plain_number(text: &str) -> bool {
    let whole = text.split('.').next().unwrap_or("");
    if whole.is_empty() || !whole.bytes().all(|byte| byte.is_ascii_digit()) {
        return false;
    }
    match text.split_once('.') {
        Some((_, frac)) => !frac.is_empty() && frac.bytes().all(|byte| byte.is_ascii_digit()),
        None => true,
    }
}

fn exponent_epoch(text: &str) -> Result<i64, ()> {
    let split = text.find(['e', 'E']).ok_or(())?;
    let mantissa = &text[..split];
    let exp = text[split + 1..]
        .strip_prefix('+')
        .unwrap_or(&text[split + 1..]);
    if !plain_number(mantissa)
        || exp.is_empty()
        || exp.len() > 9
        || !exp.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(());
    }
    let decimal = Decimal::from_scientific(&format!("{mantissa}E{exp}")).map_err(|_| ())?;
    i64::try_from(decimal.trunc()).map_err(|_| ())
}

fn iso8601_epoch(text: &str) -> Result<i64, ()> {
    // Byte slices below are only safe on ASCII. A multibyte character panics.
    if !text.is_ascii() || text.len() < 20 || !date_ok(&text[..10]) {
        return Err(());
    }
    let bytes = text.as_bytes();
    if !bytes[10].eq_ignore_ascii_case(&b'T') || bytes[13] != b':' || bytes[16] != b':' {
        return Err(());
    }
    let hour = two_digits(&text[11..13])?;
    let minute = two_digits(&text[14..16])?;
    let second = two_digits(&text[17..19])?;
    if hour > 23 || minute > 59 || second > 59 {
        return Err(());
    }
    let offset = zone_offset(&text[19..])?;
    let year: i32 = text[..4].parse().map_err(|_| ())?;
    let month: u32 = text[5..7].parse().map_err(|_| ())?;
    let day: u32 = text[8..10].parse().map_err(|_| ())?;
    let local = days_from_civil(year, month, day)
        .checked_mul(86_400)
        .and_then(|secs| secs.checked_add(i64::from(hour) * 3_600))
        .and_then(|secs| secs.checked_add(i64::from(minute) * 60))
        .and_then(|secs| secs.checked_add(i64::from(second)))
        .ok_or(())?;
    local.checked_sub(offset).ok_or(())
}

fn date_ok(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes[..4].iter().all(u8::is_ascii_digit)
        && bytes[5..7].iter().all(u8::is_ascii_digit)
        && bytes[8..10].iter().all(u8::is_ascii_digit)
        && valid_date(text)
}

fn two_digits(text: &str) -> Result<u32, ()> {
    let bytes = text.as_bytes();
    if bytes.len() != 2 || !bytes[0].is_ascii_digit() || !bytes[1].is_ascii_digit() {
        return Err(());
    }
    Ok(u32::from(bytes[0] - b'0') * 10 + u32::from(bytes[1] - b'0'))
}

fn zone_offset(rest: &str) -> Result<i64, ()> {
    let rest = match rest.strip_prefix('.') {
        Some(after) => {
            let digits = after.bytes().take_while(u8::is_ascii_digit).count();
            if digits == 0 {
                return Err(());
            }
            &after[digits..]
        }
        None => rest,
    };
    if rest.eq_ignore_ascii_case("Z") {
        return Ok(0);
    }
    let bytes = rest.as_bytes();
    if bytes.len() != 6 || (bytes[0] != b'+' && bytes[0] != b'-') || bytes[3] != b':' {
        return Err(());
    }
    let hour = two_digits(&rest[1..3])?;
    let minute = two_digits(&rest[4..6])?;
    if hour > 23 || minute > 59 {
        return Err(());
    }
    let secs = i64::from(hour) * 3_600 + i64::from(minute) * 60;
    if bytes[0] == b'+' {
        Ok(secs)
    } else {
        Ok(-secs)
    }
}

fn valid_repository(name: &str) -> bool {
    if name.len() < 2 || name.len() > 256 {
        return false;
    }
    name.split('/').all(valid_component)
}

fn valid_component(part: &str) -> bool {
    let bytes = part.as_bytes();
    let mut i = 0;
    if !take_alnum(bytes, &mut i) {
        return false;
    }
    while i < bytes.len() {
        let start = i;
        if bytes[i] == b'.' {
            i += 1;
        } else if bytes[i] == b'_' {
            i += 1;
            if i < bytes.len() && bytes[i] == b'_' {
                i += 1;
            }
        } else if bytes[i] == b'-' {
            while i < bytes.len() && bytes[i] == b'-' {
                i += 1;
            }
        } else {
            return false;
        }
        if i == start || !take_alnum(bytes, &mut i) {
            return false;
        }
    }
    true
}

fn take_alnum(bytes: &[u8], i: &mut usize) -> bool {
    if *i >= bytes.len() || !is_name(bytes[*i]) {
        return false;
    }
    while *i < bytes.len() && is_name(bytes[*i]) {
        *i += 1;
    }
    true
}

fn is_name(byte: u8) -> bool {
    byte.is_ascii_lowercase() || byte.is_ascii_digit()
}

#[cfg(test)]
mod tests {
    use super::parse_document;

    #[test]
    fn claim_c7_empty_tags_are_untagged() {
        let doc = r#"{"imageDetails":[
            {"repositoryName":"app","imageSizeInBytes":1,"imageTags":[]},
            {"repositoryName":"app","imageSizeInBytes":1},
            {"repositoryName":"app","imageSizeInBytes":1,"imageTags":null},
            {"repositoryName":"app","imageSizeInBytes":1,"imageTags":["v1"]}
        ]}"#;
        let images = parse_document(doc, 1).unwrap();
        assert!(images[0].untagged);
        assert!(images[1].untagged);
        assert!(images[2].untagged);
        assert!(!images[3].untagged);
        assert_eq!(images[3].tags, 1);
    }

    #[test]
    fn claim_c18_bad_size_hides_the_value() {
        let doc = r#"{"imageDetails":[{"repositoryName":"app","imageDigest":"sha256:secret","imageSizeInBytes":"12abc"}]}"#;
        let err = parse_document(doc, 1).unwrap_err();
        assert!(err.message().contains("invalid imageSizeInBytes"));
        assert!(!err.message().contains("12abc"));
        assert!(!err.message().contains("secret"));
    }

    #[test]
    fn missing_details_and_empty_list() {
        assert_eq!(
            parse_document("{}", 1).unwrap_err().message(),
            "missing imageDetails"
        );
        assert!(parse_document(r#"{"imageDetails":[]}"#, 1)
            .unwrap()
            .is_empty());
        assert_eq!(
            parse_document("not-json", 1).unwrap_err().message(),
            "not JSON"
        );
    }

    #[test]
    fn repository_names_follow_the_api_pattern() {
        assert!(parse_document(
            r#"{"imageDetails":[{"repositoryName":"library/app","imageSizeInBytes":1}]}"#,
            1
        )
        .is_ok());
        let err = parse_document(
            r#"{"imageDetails":[{"repositoryName":"Bad","imageSizeInBytes":1}]}"#,
            1,
        )
        .unwrap_err();
        assert!(err.message().contains("invalid repositoryName"));
        assert!(!err.message().contains("Bad"));
    }

    #[test]
    fn fractional_push_time_floors_and_registry_must_be_twelve_digits() {
        let doc = r#"{"imageDetails":[{"repositoryName":"app","imageSizeInBytes":1,"imagePushedAt":1705276800.9,"registryId":"123456789012"}]}"#;
        let images = parse_document(doc, 1).unwrap();
        assert_eq!(images[0].pushed_epoch, Some(1_705_276_800));
        let err = parse_document(
            r#"{"imageDetails":[{"repositoryName":"app","imageSizeInBytes":1,"registryId":"123"}]}"#,
            1,
        )
        .unwrap_err();
        assert!(err.message().contains("invalid registryId"));
        assert!(!err.message().contains("123"));
    }

    #[test]
    fn push_time_accepts_iso8601_and_an_exponent() {
        let epoch = Some(1_705_276_800);
        for pushed in [
            r#""2024-01-15T00:00:00+00:00""#,
            r#""2024-01-15T00:00:00Z""#,
            r#""2024-01-15T00:00:00.9+00:00""#,
            "1.7052768E9",
            "1.7052768E+9",
            "1.7052768009E9",
        ] {
            let doc = format!(
                r#"{{"imageDetails":[{{"repositoryName":"app","imageSizeInBytes":1,"imagePushedAt":{pushed}}}]}}"#
            );
            let images = parse_document(&doc, 1).unwrap();
            assert_eq!(images[0].pushed_epoch, epoch, "{pushed}");
        }
        let ahead = r#"{"imageDetails":[{"repositoryName":"app","imageSizeInBytes":1,"imagePushedAt":"2024-01-15T00:00:00+01:00"}]}"#;
        assert_eq!(
            parse_document(ahead, 1).unwrap()[0].pushed_epoch,
            Some(1_705_273_200)
        );
        let err = parse_document(
            r#"{"imageDetails":[{"repositoryName":"app","imageSizeInBytes":1,"imagePushedAt":"not-a-time"}]}"#,
            1,
        )
        .unwrap_err();
        assert!(err.message().contains("invalid imagePushedAt"));
        assert!(!err.message().contains("not-a-time"));
        let err = parse_document(
            r#"{"imageDetails":[{"repositoryName":"app","imageSizeInBytes":1,"imagePushedAt":1E19}]}"#,
            1,
        )
        .unwrap_err();
        assert!(err.message().contains("invalid imagePushedAt"));
    }

    #[test]
    fn push_time_rejects_non_ascii_without_panic() {
        for pushed in [r#""2024-01-1é00:00:00Z""#, r#""2024-01-15T00:00:0é+00:00""#] {
            let doc = format!(
                r#"{{"imageDetails":[{{"repositoryName":"app","imageSizeInBytes":1,"imagePushedAt":{pushed}}}]}}"#
            );
            let err = parse_document(&doc, 1).unwrap_err();
            assert!(err.message().contains("invalid imagePushedAt"), "{pushed}");
            assert!(!err.message().contains('é'), "{pushed}");
        }
    }

    #[test]
    fn image_shape_errors_name_file_and_image() {
        let one = |image: &str| {
            parse_document(&format!(r#"{{"imageDetails":[{image}]}}"#), 3)
                .unwrap_err()
                .message()
                .to_string()
        };
        assert_eq!(one("5"), "file 3: image 1: not an object");
        assert_eq!(
            one(r#"{"imageSizeInBytes":1}"#),
            "file 3: image 1: missing repositoryName"
        );
        assert_eq!(
            one(r#"{"repositoryName":"app"}"#),
            "file 3: image 1: missing imageSizeInBytes"
        );
        assert_eq!(
            one(r#"{"repositoryName":"app","imageSizeInBytes":1,"imageDigest":7}"#),
            "file 3: image 1: invalid imageDigest"
        );
        assert_eq!(
            one(r#"{"repositoryName":"app","imageSizeInBytes":1,"imageDigest":""}"#),
            "file 3: image 1: invalid imageDigest"
        );
        assert_eq!(
            one(
                r#"{"repositoryName":"app","imageSizeInBytes":1,"imageDigest":"sha256:x bytes=0"}"#
            ),
            "file 3: image 1: invalid imageDigest"
        );
        assert_eq!(
            one(
                r#"{"repositoryName":"app","imageSizeInBytes":1,"imageDigest":"sha256:x\tbytes=0"}"#
            ),
            "file 3: image 1: invalid imageDigest"
        );
        assert_eq!(
            one(r#"{"repositoryName":"app","imageSizeInBytes":1,"imageTags":"v1"}"#),
            "file 3: image 1: invalid imageTags"
        );
        assert_eq!(
            one(r#"{"repositoryName":"app","imageSizeInBytes":1,"imageTags":[1]}"#),
            "file 3: image 1: invalid image tag"
        );
        assert_eq!(
            one(r#"{"repositoryName":"app","imageSizeInBytes":1,"imageTags":[""]}"#),
            "file 3: image 1: invalid image tag"
        );
        assert_eq!(
            one(r#"{"repositoryName":"app","imageSizeInBytes":1,"imageStatus":5}"#),
            "file 3: image 1: invalid imageStatus"
        );
        for bad in ["\\u00a0ACTIVE", "ACTIVE\\u3000", "\\u2028ARCHIVED"] {
            let doc =
                format!(r#"{{"repositoryName":"app","imageSizeInBytes":1,"imageStatus":"{bad}"}}"#);
            assert_eq!(one(&doc), "file 3: image 1: invalid imageStatus");
        }
        let ok =
            r#"{"repositoryName":"app","imageSizeInBytes":1,"imageStatus":" \t\r\nACTIVE \n"}"#;
        assert!(parse_document(&format!(r#"{{"imageDetails":[{ok}]}}"#), 3).is_ok());
    }

    #[test]
    fn the_image_number_counts_from_one_per_file() {
        let doc = r#"{"imageDetails":[{"repositoryName":"app","imageSizeInBytes":1},{"imageSizeInBytes":1}]}"#;
        assert_eq!(
            parse_document(doc, 2).unwrap_err().message(),
            "file 2: image 2: missing repositoryName"
        );
    }

    #[test]
    fn digest_display_is_one_to_one_for_none_and_quotes() {
        fn digest_of(body: &str) -> String {
            let doc = format!(
                r#"{{"imageDetails":[{{"repositoryName":"app","imageSizeInBytes":1{body}}}]}}"#
            );
            super::parse_document(&doc, 1).unwrap().remove(0).digest
        }
        assert_eq!(digest_of(""), "(none)");
        assert_eq!(digest_of(r#","imageDigest":null"#), "(none)");
        assert_eq!(digest_of(r#","imageDigest":"(none)""#), "\"(none)\"");
        assert_eq!(
            digest_of(r#","imageDigest":"\"(none)\"""#),
            "\"\"(none)\"\""
        );
        assert_eq!(digest_of(r#","imageDigest":"\"x""#), "\"\"x\"");
        assert_eq!(digest_of(r#","imageDigest":"sha256:ab""#), "sha256:ab");
    }
}
