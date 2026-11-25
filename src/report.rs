use std::fmt::Write as _;

use serde_json::{json, Value};

use crate::bill::Bill;
use crate::error::{is_unsafe, show_text};

/// Text table, or pretty JSON when `json` is set.
#[must_use]
pub fn render(bill: &Bill, json: bool) -> String {
    if json {
        render_json(bill)
    } else {
        render_text(bill)
    }
}

/// The report as Markdown tables. The amounts are the strings of the text and
/// JSON reports; nothing is computed or rounded again here.
#[must_use]
pub fn render_markdown(bill: &Bill) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "# ecr-bill\n");
    let _ = writeln!(
        out,
        "Estimate from list prices, not your AWS invoice (free tier, credits, discounts not included)\n"
    );
    let _ = writeln!(
        out,
        "Source: {}, price date {}. Region: {} ({}). Free tier not applied. Not applied: retrieval {} USD per GB; archive minimum {} days.\n",
        md_cell(&bill.source_url),
        md_cell(&bill.source_date),
        md_cell(&bill.region),
        md_cell(&bill.region_name),
        bill.retrieval_usd_per_gb,
        bill.archive_min_days
    );
    let _ = writeln!(out, "## Repositories\n");
    let _ = writeln!(
        out,
        "| Repository | Images | Bytes | Untagged | Old | Estimated USD per month |"
    );
    let _ = writeln!(out, "| --- | ---: | ---: | ---: | ---: | ---: |");
    for repo in &bill.repos {
        let old = repo
            .old
            .map_or_else(|| "omitted".to_string(), |count| count.to_string());
        let _ = writeln!(
            out,
            "| {} | {} | {} | {} | {old} | {} |",
            md_cell(&repo.name),
            repo.images,
            repo.bytes,
            repo.untagged,
            repo.usd_month
        );
    }
    let _ = writeln!(
        out,
        "| **Total** | {} | {} | {} | | {} |\n",
        bill.total_images, bill.total_bytes, bill.total_untagged, bill.total_usd
    );
    let _ = writeln!(out, "## Untagged\n");
    let _ = writeln!(out, "| Repository | Digest | Bytes | Tags |");
    let _ = writeln!(out, "| --- | --- | ---: | ---: |");
    for image in &bill.untagged {
        let _ = writeln!(
            out,
            "| {} | {} | {} | {} |",
            md_cell(&image.repository),
            md_cell(&show_digest(&image.digest)),
            image.bytes,
            image.tags
        );
    }
    let _ = writeln!(out, "\n## Old\n");
    match &bill.old {
        None => {
            let _ = writeln!(out, "Omitted: pass `--as-of YYYY-MM-DD`.");
        }
        Some(images) => {
            let _ = writeln!(
                out,
                "As of {}, older than {} days.\n",
                md_cell(bill.as_of.as_deref().unwrap_or("")),
                bill.older_than_days
            );
            let _ = writeln!(out, "| Repository | Digest | Age in days | Tags | Bytes |");
            let _ = writeln!(out, "| --- | --- | ---: | ---: | ---: |");
            for image in images {
                let _ = writeln!(
                    out,
                    "| {} | {} | {} | {} | {} |",
                    md_cell(&image.repository),
                    md_cell(&show_digest(&image.digest)),
                    image.age_days.unwrap_or(0),
                    image.tags,
                    image.bytes
                );
            }
        }
    }
    let _ = writeln!(out, "\n## Lifecycle\n\nNot executed.\n\n```json");
    out.push_str(&bill.policy_json);
    if !bill.policy_json.ends_with('\n') {
        out.push('\n');
    }
    let _ = writeln!(out, "```");
    if !bill.warnings.is_empty() {
        let _ = writeln!(out, "\n## Warnings\n");
        for warning in &bill.warnings {
            let _ = writeln!(out, "- {}", md_cell(warning));
        }
    }
    out
}

/// Text for a table cell or a list item: control and line-break characters are
/// written as escapes first, then every character that Markdown or a table row
/// would read as syntax gets a backslash. A pipe cannot end the cell.
fn md_cell(text: &str) -> String {
    let mut out = String::new();
    for ch in show_text(text).chars() {
        if matches!(ch, '\\' | '|' | '`' | '*' | '_' | '<' | '>' | '[' | ']') {
            out.push('\\');
        }
        out.push(ch);
    }
    out
}

fn render_text(bill: &Bill) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "ecr-bill");
    let _ = writeln!(
        out,
        "Estimate from list prices, not your AWS invoice (free tier, credits, discounts not included)"
    );
    let _ = writeln!(out, "source: {}", show_text(&bill.source_url));
    let _ = writeln!(out, "as of: {}", show_text(&bill.source_date));
    let _ = writeln!(
        out,
        "region: {} ({})",
        show_text(&bill.region),
        show_text(&bill.region_name)
    );
    let _ = writeln!(out, "free tier not applied");
    let _ = writeln!(
        out,
        "not applied: retrieval {} USD per GB; archive minimum {} days",
        bill.retrieval_usd_per_gb, bill.archive_min_days
    );
    let _ = writeln!(out);
    let _ = writeln!(out, "repositories");
    for repo in &bill.repos {
        let old = match repo.old {
            Some(count) => count.to_string(),
            None => "omitted".to_string(),
        };
        let _ = writeln!(
            out,
            "{} images={} bytes={} untagged={} old={old} estimated_usd_month={}",
            show_text(&repo.name),
            repo.images,
            repo.bytes,
            repo.untagged,
            repo.usd_month
        );
    }
    let _ = writeln!(
        out,
        "total images={} bytes={} untagged={} estimated_usd_month={}",
        bill.total_images, bill.total_bytes, bill.total_untagged, bill.total_usd
    );
    let _ = writeln!(out);
    let _ = writeln!(out, "untagged");
    for image in &bill.untagged {
        let _ = writeln!(
            out,
            "{} {} bytes={} tags={}",
            show_text(&image.repository),
            show_digest(&image.digest),
            image.bytes,
            image.tags
        );
    }
    let _ = writeln!(out);
    let _ = writeln!(out, "old");
    match &bill.old {
        None => {
            let _ = writeln!(out, "omitted: pass --as-of YYYY-MM-DD");
        }
        Some(images) => {
            let _ = writeln!(
                out,
                "as-of: {}",
                show_text(bill.as_of.as_deref().unwrap_or(""))
            );
            let _ = writeln!(out, "older_than_days: {}", bill.older_than_days);
            for image in images {
                let _ = writeln!(
                    out,
                    "{} {} age_days={} tags={} bytes={}",
                    show_text(&image.repository),
                    show_digest(&image.digest),
                    image.age_days.unwrap_or(0),
                    image.tags,
                    image.bytes
                );
            }
        }
    }
    let _ = writeln!(out);
    let _ = writeln!(out, "lifecycle");
    let _ = writeln!(out, "Not executed");
    out.push_str(&bill.policy_json);
    if !bill.policy_json.ends_with('\n') {
        let _ = writeln!(out);
    }
    for warning in &bill.warnings {
        let _ = writeln!(out, "warning: {}", show_text(warning));
    }
    out
}

fn render_json(bill: &Bill) -> String {
    let policy: Value = serde_json::from_str(&bill.policy_json).unwrap_or(Value::Null);
    let repos: Vec<Value> = bill
        .repos
        .iter()
        .map(|repo| {
            json!({
                "repository": repo.name,
                "images": repo.images,
                "bytes": repo.bytes.to_string(),
                "untagged": repo.untagged,
                "old": repo.old,
                "estimated_usd_month": repo.usd_month,
            })
        })
        .collect();
    let untagged: Vec<Value> = bill
        .untagged
        .iter()
        .map(|image| {
            json!({
                "repository": image.repository,
                "digest": image.digest,
                "bytes": image.bytes.to_string(),
                "tags": image.tags,
            })
        })
        .collect();
    let old = match &bill.old {
        None => Value::Null,
        Some(images) => Value::Array(
            images
                .iter()
                .map(|image| {
                    json!({
                        "repository": image.repository,
                        "digest": image.digest,
                        "bytes": image.bytes.to_string(),
                        "tags": image.tags,
                        "age_days": image.age_days,
                    })
                })
                .collect(),
        ),
    };
    let value = json!({
        "schema_version": 1,
        "estimate": true,
        "source_url": show_text(&bill.source_url),
        "source_date": show_text(&bill.source_date),
        "region": show_text(&bill.region),
        "region_name": show_text(&bill.region_name),
        "free_tier_applied": false,
        "retrieval_usd_per_gb": bill.retrieval_usd_per_gb,
        "archive_min_days": bill.archive_min_days,
        "retrieval_applied": false,
        "as_of": bill.as_of,
        "older_than_days": bill.older_than_days,
        "untagged_days": bill.untagged_days,
        "keep": bill.keep,
        "images": bill.total_images,
        "bytes": bill.total_bytes.to_string(),
        "untagged_images": bill.total_untagged,
        "estimated_usd_month": bill.total_usd,
        "repositories": repos,
        "untagged": untagged,
        "old": old,
        "lifecycle": {
            "not_executed": true,
            "policy": policy
        },
        "warnings": bill.warnings,
    });
    let text = serde_json::to_string_pretty(&value).expect("json");
    let mut text = escape_unsafe_in_json(&text);
    text.push('\n');
    text
}

/// `serde_json` escapes only the characters below U+0020. Every other character
/// a terminal could act on (DEL, C1 controls, line and paragraph separators,
/// bidi and invisible format characters) is written as a `\uXXXX` escape, which
/// is the same JSON string. The only line breaks left in the pretty text are
/// the ones between tokens, so `\n` stays.
fn escape_unsafe_in_json(text: &str) -> String {
    if !text.chars().any(|ch| ch != '\n' && is_unsafe(ch)) {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        if ch != '\n' && is_unsafe(ch) {
            let mut units = [0u16; 2];
            for unit in ch.encode_utf16(&mut units) {
                let _ = write!(out, "\\u{unit:04x}");
            }
        } else {
            out.push(ch);
        }
    }
    out
}

/// A digest on a text line: whitespace becomes `\u{hex}` and the `=` of the
/// field names used on these lines becomes `\=`, so it cannot forge a field.
fn show_digest(digest: &str) -> String {
    let mut out = String::new();
    for ch in show_text(digest).chars() {
        if ch.is_whitespace() {
            let _ = write!(out, "\\u{{{:x}}}", u32::from(ch));
        } else {
            out.push(ch);
        }
    }
    for word in [
        "bytes=",
        "tags=",
        "images=",
        "estimated_usd_month=",
        "usd_month=",
        "untagged=",
        "age_days=",
    ] {
        out = out.replace(word, &word.replace('=', "\\="));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::render;
    use crate::bill::{bill_documents, BillOptions};
    use crate::policy::validate_policy;
    use crate::price::prices_builtin;

    fn sample(as_of: Option<&str>) -> crate::bill::Bill {
        let doc = r#"{"imageDetails":[
            {"registryId":"123456789012","repositoryName":"app","imageDigest":"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","imageTags":["v1"],"imageSizeInBytes":1073741824,"imagePushedAt":1705276800,"imageStatus":"ACTIVE"},
            {"registryId":"123456789012","repositoryName":"app","imageDigest":"sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb","imageSizeInBytes":1073741824,"imagePushedAt":1577836800}
        ]}"#;
        bill_documents(
            &[doc],
            &prices_builtin(),
            &BillOptions {
                as_of: as_of.map(str::to_string),
                ..BillOptions::default()
            },
        )
        .unwrap()
    }

    #[test]
    fn claim_c15_total_is_the_sum_and_json_uses_strings() {
        let bill = sample(Some("2026-11-09"));
        assert_eq!(bill.total_usd, "0.20");
        let json = render(&bill, true);
        assert!(json.contains("\"estimated_usd_month\": \"0.20\""));
        assert!(json.contains("\"bytes\": \"2147483648\""));
        assert!(json.contains("\"not_executed\": true"));
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed["estimated_usd_month"].is_string());
        let policy = serde_json::to_string(&parsed["lifecycle"]["policy"]).unwrap();
        validate_policy(&policy).unwrap();
    }

    #[test]
    fn claim_c16_registry_id_is_not_printed() {
        let text = render(&sample(None), false);
        assert!(!text.contains("123456789012"));
        assert!(text.contains("Not executed"));
        assert!(text.contains("omitted: pass --as-of YYYY-MM-DD"));
        assert!(text
            .contains("sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"));
        let json = render(&sample(None), true);
        assert!(!json.contains("123456789012"));
    }

    #[test]
    fn text_lists_source_and_region() {
        let text = render(&sample(Some("2026-11-09")), false);
        assert!(text.contains("source: https://aws.amazon.com/ecr/pricing/"));
        assert!(text.contains("as of: 2026-11-09"));
        assert!(text.contains("region: us-east-1 (US East (N. Virginia))"));
        assert!(text.contains("free tier not applied"));
        assert!(text.contains("old=2"));
    }

    #[test]
    fn digest_escapes_invisible_characters() {
        let shown = super::show_digest("a\u{2800}b\u{3164}c\u{034F}d\u{FE0F}e\u{E0041}");
        assert_eq!(shown, "a\\u{2800}b\\u{3164}c\\u{34f}d\\u{fe0f}e\\u{e0041}");
    }

    #[test]
    fn digest_cannot_forge_a_field_name() {
        let shown = super::show_digest("sha256:bytes=0 tags=9\t");
        assert_eq!(shown, "sha256:bytes\\=0\\u{20}tags\\=9\\t");
    }

    #[test]
    fn json_text_escapes_unsafe_characters_and_keeps_pretty_line_breaks() {
        let text = "{\n  \"a\": \"x\u{9b}y\u{7f}z\u{2028}\u{202e}\u{e0041}\"\n}";
        let shown = super::escape_unsafe_in_json(text);
        assert_eq!(
            shown,
            "{\n  \"a\": \"x\\u009by\\u007fz\\u2028\\u202e\\udb40\\udc41\"\n}"
        );
        let back: serde_json::Value = serde_json::from_str(&shown).unwrap();
        assert_eq!(back["a"], "x\u{9b}y\u{7f}z\u{2028}\u{202e}\u{e0041}");
        assert_eq!(super::escape_unsafe_in_json("plain é\n"), "plain é\n");
    }
}
