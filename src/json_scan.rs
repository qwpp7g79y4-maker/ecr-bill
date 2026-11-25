use std::collections::HashSet;

use crate::error::Error;

/// Deepest nesting accepted; matches serde_json's own recursion limit.
pub(crate) const MAX_DEPTH: u32 = 128;

fn depth_error() -> Error {
    Error::new("JSON nesting is deeper than 128 levels")
}

/// Parse one JSON value.
///
/// The scanner runs first. It refuses a second value and anything deeper
/// than 128, so serde never sees the input that overflowed the stack.
/// Serde's own limit is lifted only after that check: its default stops one
/// level earlier than 128, and 128 itself does not overflow.
pub(crate) fn parse_json(text: &str) -> Result<serde_json::Value, Error> {
    reject_duplicate_keys(text)?;
    let mut de = serde_json::Deserializer::from_str(text);
    de.disable_recursion_limit();
    let mut items = de.into_iter::<serde_json::Value>();
    let value = match items.next() {
        Some(Ok(value)) => value,
        _ => return Err(Error::new("not JSON")),
    };
    if items.next().is_some() {
        return Err(Error::new("not JSON"));
    }
    Ok(value)
}

/// Reject objects that repeat a key.
///
/// One leading JSON value is scanned. Bytes after that value are `not JSON`,
/// so a deep second value is never passed to serde. A token that only looks
/// like a literal (`not-json`) is left to the parser, which reports it.
pub(crate) fn reject_duplicate_keys(text: &str) -> Result<(), Error> {
    let bytes = text.as_bytes();
    let mut i = 0;
    skip_ws(bytes, &mut i);
    if i >= bytes.len() {
        return Ok(());
    }
    let scalar = matches!(bytes[i], b'"' | b't' | b'f' | b'n' | b'-' | b'0'..=b'9');
    if !scalar && !matches!(bytes[i], b'{' | b'[') {
        return Ok(());
    }
    let mut depth = 0u32;
    if let Err(err) = scan_value(bytes, &mut i, &mut depth) {
        if scalar {
            return Ok(());
        }
        return Err(err);
    }
    skip_ws(bytes, &mut i);
    if i < bytes.len() {
        return Err(Error::new("not JSON"));
    }
    Ok(())
}

fn scan_value(bytes: &[u8], i: &mut usize, depth: &mut u32) -> Result<(), Error> {
    skip_ws(bytes, i);
    if *i >= bytes.len() {
        return Err(Error::new("not JSON"));
    }
    match bytes[*i] {
        b'{' => scan_object(bytes, i, depth),
        b'[' => scan_array(bytes, i, depth),
        b'"' => {
            let _ = scan_string(bytes, i)?;
            Ok(())
        }
        b't' => scan_lit(bytes, i, b"true"),
        b'f' => scan_lit(bytes, i, b"false"),
        b'n' => scan_lit(bytes, i, b"null"),
        b'-' | b'0'..=b'9' => scan_number(bytes, i),
        _ => Err(Error::new("not JSON")),
    }
}

fn scan_object(bytes: &[u8], i: &mut usize, depth: &mut u32) -> Result<(), Error> {
    *depth = depth.checked_add(1).ok_or_else(|| Error::new("not JSON"))?;
    if *depth > MAX_DEPTH {
        return Err(depth_error());
    }
    *i += 1;
    let mut keys = HashSet::<String>::new();
    loop {
        skip_ws(bytes, i);
        if *i >= bytes.len() {
            return Err(Error::new("not JSON"));
        }
        if bytes[*i] == b'}' {
            *i += 1;
            *depth -= 1;
            return Ok(());
        }
        if !keys.is_empty() {
            if bytes[*i] != b',' {
                return Err(Error::new("not JSON"));
            }
            *i += 1;
            skip_ws(bytes, i);
        }
        let key = scan_string(bytes, i)?;
        if !keys.insert(key) {
            return Err(Error::new("duplicate key"));
        }
        skip_ws(bytes, i);
        if *i >= bytes.len() || bytes[*i] != b':' {
            return Err(Error::new("not JSON"));
        }
        *i += 1;
        scan_value(bytes, i, depth)?;
    }
}

fn scan_array(bytes: &[u8], i: &mut usize, depth: &mut u32) -> Result<(), Error> {
    *depth = depth.checked_add(1).ok_or_else(|| Error::new("not JSON"))?;
    if *depth > MAX_DEPTH {
        return Err(depth_error());
    }
    *i += 1;
    let mut any = false;
    loop {
        skip_ws(bytes, i);
        if *i >= bytes.len() {
            return Err(Error::new("not JSON"));
        }
        if bytes[*i] == b']' {
            *i += 1;
            *depth -= 1;
            return Ok(());
        }
        if any {
            if bytes[*i] != b',' {
                return Err(Error::new("not JSON"));
            }
            *i += 1;
        }
        any = true;
        scan_value(bytes, i, depth)?;
    }
}

fn scan_string(bytes: &[u8], i: &mut usize) -> Result<String, Error> {
    if *i >= bytes.len() || bytes[*i] != b'"' {
        return Err(Error::new("not JSON"));
    }
    *i += 1;
    let mut out = String::new();
    while *i < bytes.len() {
        match bytes[*i] {
            b'"' => {
                *i += 1;
                return Ok(out);
            }
            b'\\' => {
                *i += 1;
                push_escape(&mut out, bytes, i)?;
            }
            0x20..=0x7e => {
                out.push(bytes[*i] as char);
                *i += 1;
            }
            _ => out.push(decode_utf8(bytes, i)?),
        }
    }
    Err(Error::new("not JSON"))
}

fn push_escape(out: &mut String, bytes: &[u8], i: &mut usize) -> Result<(), Error> {
    if *i >= bytes.len() {
        return Err(Error::new("not JSON"));
    }
    let esc = bytes[*i];
    *i += 1;
    match esc {
        b'"' | b'\\' | b'/' => out.push(esc as char),
        b'b' => out.push('\u{0008}'),
        b'f' => out.push('\u{000c}'),
        b'n' => out.push('\n'),
        b'r' => out.push('\r'),
        b't' => out.push('\t'),
        b'u' => push_u_escape(out, bytes, i)?,
        _ => return Err(Error::new("not JSON")),
    }
    Ok(())
}

/// A `\u` escape is one UTF-16 code unit. A surrogate pair is one scalar value.
fn push_u_escape(out: &mut String, bytes: &[u8], i: &mut usize) -> Result<(), Error> {
    let unit = read_hex4(bytes, i)?;
    if matches!(unit, 0xDC00..=0xDFFF) {
        return Err(Error::new("not JSON"));
    }
    if matches!(unit, 0xD800..=0xDBFF) {
        if *i + 1 >= bytes.len() || bytes[*i] != b'\\' || bytes[*i + 1] != b'u' {
            return Err(Error::new("not JSON"));
        }
        *i += 2;
        let low = read_hex4(bytes, i)?;
        if !matches!(low, 0xDC00..=0xDFFF) {
            return Err(Error::new("not JSON"));
        }
        let scalar = 0x1_0000 + ((unit - 0xD800) << 10) + (low - 0xDC00);
        out.push(char::from_u32(scalar).ok_or_else(|| Error::new("not JSON"))?);
        return Ok(());
    }
    out.push(char::from_u32(unit).ok_or_else(|| Error::new("not JSON"))?);
    Ok(())
}

fn read_hex4(bytes: &[u8], i: &mut usize) -> Result<u32, Error> {
    if *i + 4 > bytes.len() {
        return Err(Error::new("not JSON"));
    }
    let hex = std::str::from_utf8(&bytes[*i..*i + 4]).map_err(|_| Error::new("not JSON"))?;
    let code = u32::from_str_radix(hex, 16).map_err(|_| Error::new("not JSON"))?;
    *i += 4;
    Ok(code)
}

fn decode_utf8(bytes: &[u8], i: &mut usize) -> Result<char, Error> {
    let first = *bytes.get(*i).ok_or_else(|| Error::new("not JSON"))?;
    let width = match first {
        0x00..=0x7F => 1,
        0xC2..=0xDF => 2,
        0xE0..=0xEF => 3,
        0xF0..=0xF4 => 4,
        _ => return Err(Error::new("not JSON")),
    };
    let end = i.checked_add(width).ok_or_else(|| Error::new("not JSON"))?;
    if end > bytes.len() {
        return Err(Error::new("not JSON"));
    }
    let text = std::str::from_utf8(&bytes[*i..end]).map_err(|_| Error::new("not JSON"))?;
    let ch = text.chars().next().ok_or_else(|| Error::new("not JSON"))?;
    *i = end;
    Ok(ch)
}

fn scan_lit(bytes: &[u8], i: &mut usize, lit: &[u8]) -> Result<(), Error> {
    if bytes[*i..].starts_with(lit) {
        *i += lit.len();
        Ok(())
    } else {
        Err(Error::new("not JSON"))
    }
}

fn scan_number(bytes: &[u8], i: &mut usize) -> Result<(), Error> {
    let start = *i;
    if bytes[*i] == b'-' {
        *i += 1;
    }
    while *i < bytes.len() && matches!(bytes[*i], b'0'..=b'9' | b'.' | b'e' | b'E' | b'+' | b'-') {
        *i += 1;
    }
    if *i == start {
        return Err(Error::new("not JSON"));
    }
    Ok(())
}

fn skip_ws(bytes: &[u8], i: &mut usize) {
    while *i < bytes.len() && matches!(bytes[*i], b' ' | b'\n' | b'\r' | b'\t') {
        *i += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::reject_duplicate_keys;

    #[test]
    fn duplicate_key_is_exact() {
        let err = reject_duplicate_keys(r#"{"a":1,"a":2}"#).unwrap_err();
        assert_eq!(err.message(), "duplicate key");
        let err = reject_duplicate_keys(r#"{"a":{"b":1,"b":2}}"#).unwrap_err();
        assert_eq!(err.message(), "duplicate key");
    }

    #[test]
    fn distinct_keys_and_non_objects_pass_the_scanner() {
        assert!(reject_duplicate_keys(r#"{"a":1,"b":2}"#).is_ok());
        assert!(reject_duplicate_keys("[1,2]").is_ok());
        assert!(reject_duplicate_keys("not-json").is_ok());
    }

    #[test]
    fn raw_utf8_and_u_escape_are_one_key() {
        let err = reject_duplicate_keys("{\"café\":1,\"caf\\u00e9\":2}").unwrap_err();
        assert_eq!(err.message(), "duplicate key");
        let err = reject_duplicate_keys("{\"caf\\u00E9\":1,\"café\":2}").unwrap_err();
        assert_eq!(err.message(), "duplicate key");
        let err = reject_duplicate_keys("{\"😀\":1,\"\\uD83D\\uDE00\":2}").unwrap_err();
        assert_eq!(err.message(), "duplicate key");
    }

    #[test]
    fn different_unicode_keys_are_not_duplicates() {
        assert!(reject_duplicate_keys("{\"café\":1,\"caf\\u00e8\":2}").is_ok());
        assert!(reject_duplicate_keys("{\"é\":1,\"\\u00c3\\u00a9\":2}").is_ok());
        assert!(reject_duplicate_keys("{\"\\uD83D\\uDE00\":1,\"\\uD83D\\uDE01\":2}").is_ok());
    }

    #[test]
    fn a_surrogate_pair_is_json_and_a_lone_surrogate_is_not() {
        assert!(reject_duplicate_keys(r#"{"imageTags":["\uD83D\uDE00"]}"#).is_ok());
        assert_eq!(
            reject_duplicate_keys(r#"{"a":"\uD800"}"#)
                .unwrap_err()
                .message(),
            "not JSON"
        );
        assert_eq!(
            reject_duplicate_keys(r#"{"\uDE00":1}"#)
                .unwrap_err()
                .message(),
            "not JSON"
        );
        assert_eq!(
            reject_duplicate_keys(r#"{"\uD800\u0041":1}"#)
                .unwrap_err()
                .message(),
            "not JSON"
        );
    }

    #[test]
    fn a_few_hundred_keys_are_checked_per_object() {
        let mut outer = String::from(r#"{"k0":{"k0":1}"#);
        for n in 1..400 {
            outer.push_str(&format!(r#","k{n}":1"#));
        }
        outer.push('}');
        assert!(outer.len() < 8_000);
        assert!(reject_duplicate_keys(&outer).is_ok());

        let mut repeated = String::from("{");
        for n in 0..400 {
            if n > 0 {
                repeated.push(',');
            }
            repeated.push_str(&format!(r#""k{n}":1"#));
        }
        repeated.push_str(r#","k7":2}"#);
        assert!(repeated.len() < 8_000);
        let err = reject_duplicate_keys(&repeated).unwrap_err();
        assert_eq!(err.message(), "duplicate key");
    }
}

#[cfg(test)]
mod depth_tests {
    use super::*;

    fn nested(n: usize) -> String {
        format!(r#"{{"x":{}{}}}"#, "[".repeat(n - 1), "]".repeat(n - 1))
    }

    #[test]
    fn depth_128_is_valid_and_129_names_the_limit() {
        assert!(reject_duplicate_keys(&nested(128)).is_ok());
        assert!(parse_json(&nested(128)).is_ok());
        let deep = nested(129);
        assert!(reject_duplicate_keys(&deep)
            .unwrap_err()
            .message()
            .contains("128"));
        let arr = format!("{}{}", "[".repeat(200), "]".repeat(200));
        assert!(reject_duplicate_keys(&arr)
            .unwrap_err()
            .message()
            .contains("128"));
        let ok = format!("{}{}", "[".repeat(128), "]".repeat(128));
        assert!(reject_duplicate_keys(&ok).is_ok() && parse_json(&ok).is_ok());
        let huge = nested(100_000);
        assert!(reject_duplicate_keys(&huge)
            .unwrap_err()
            .message()
            .contains("128"));
    }

    #[test]
    fn a_deep_tail_after_the_first_value_is_not_json() {
        let mut text = r#"{"imageDetails":[]}"#.to_string();
        text.push_str(&"[".repeat(100_000));
        assert_eq!(
            reject_duplicate_keys(&text).unwrap_err().message(),
            "not JSON"
        );
        let mut number = String::from("1 ");
        number.push_str(&"[".repeat(100_000));
        assert_eq!(
            reject_duplicate_keys(&number).unwrap_err().message(),
            "not JSON"
        );
    }

    #[test]
    fn the_depth_error_text_is_exact() {
        let deep = format!("{}{}", "[".repeat(129), "]".repeat(129));
        assert_eq!(
            reject_duplicate_keys(&deep).unwrap_err().message(),
            "JSON nesting is deeper than 128 levels"
        );
    }
}
