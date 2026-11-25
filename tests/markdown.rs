//! `--md`: the report as Markdown tables. The amounts are those of the text and
//! the JSON report, character for character.

use std::process::Command;

const FILES: [&str; 3] = [
    "examples/describe-images.json",
    "tests/fixtures/documented-cli-v1.json",
    "tests/fixtures/documented-cli-v2.json",
];

fn run(args: &[&str]) -> (i32, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_ecr-bill"))
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .env("NO_COLOR", "1")
        .args(args)
        .output()
        .unwrap();
    (
        out.status.code().unwrap(),
        String::from_utf8(out.stdout).unwrap(),
        String::from_utf8(out.stderr).unwrap(),
    )
}

/// The cells of a Markdown table row, split on pipes that are not escaped.
fn cells(row: &str) -> Vec<String> {
    let inner = row
        .trim()
        .strip_prefix('|')
        .unwrap()
        .strip_suffix('|')
        .unwrap();
    let mut out = vec![String::new()];
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                let last = out.last_mut().unwrap();
                last.push(c);
                if let Some(next) = chars.next() {
                    last.push(next);
                }
            }
            '|' => out.push(String::new()),
            _ => out.last_mut().unwrap().push(c),
        }
    }
    out.iter().map(|c| c.trim().to_string()).collect()
}

#[test]
fn markdown_amounts_equal_text_and_json_for_every_input() {
    for file in FILES {
        let (code, md, stderr) = run(&["--md", "--as-of", "2026-11-20", file]);
        assert_eq!(code, 0, "{stderr}");
        let (_, text, _) = run(&["--as-of", "2026-11-20", file]);
        let (_, json, _) = run(&["--json", "--as-of", "2026-11-20", file]);
        let json: serde_json::Value = serde_json::from_str(&json).unwrap();
        // Per repository: the text row `NAME images=.. estimated_usd_month=X`.
        for repo in json["repositories"].as_array().unwrap() {
            let name = repo["repository"].as_str().unwrap().replace('_', "\\_");
            let usd = repo["estimated_usd_month"].as_str().unwrap();
            let row = md
                .lines()
                .filter(|l| l.starts_with('|'))
                .map(cells)
                .find(|c| c[0] == name)
                .unwrap_or_else(|| panic!("no row for {name}: {md}"));
            assert_eq!(row.last().unwrap(), usd, "{file} {name}");
            assert!(
                text.contains(&format!("images={} ", repo["images"]))
                    && text.contains(&format!("estimated_usd_month={usd}\n")),
                "{file}"
            );
        }
        let total = json["estimated_usd_month"].as_str().unwrap();
        let total_row = md
            .lines()
            .filter(|l| l.starts_with('|'))
            .map(cells)
            .find(|c| c[0] == "**Total**")
            .unwrap();
        assert_eq!(total_row.last().unwrap(), total);
        assert!(text.contains(&format!("estimated_usd_month={total}\n")));
        assert_eq!(total_row[2], json["bytes"].as_str().unwrap());
    }
}

#[test]
fn markdown_says_it_is_an_estimate_and_hides_the_account() {
    let (code, md, _) = run(&["--md", "examples/describe-images.json"]);
    assert_eq!(code, 0);
    assert!(md.starts_with("# ecr-bill\n"), "{md}");
    assert!(md.contains(
        "Estimate from list prices, not your AWS invoice (free tier, credits, discounts not included)"
    ));
    assert!(md.contains("Not executed"));
    assert!(md.contains("\"rulePriority\": 1"));
    assert!(!md.contains("123456789012"));
}

#[test]
fn markdown_escapes_pipes_and_line_breaks_in_cells() {
    let (code, md, stderr) = run(&[
        "--md",
        "--prices",
        "tests/fixtures/prices-pipes-and-lines.json",
        "tests/fixtures/pipes-and-lines.json",
    ]);
    assert_eq!(code, 0, "{stderr}");
    // The region name of the price file has a pipe and a line break.
    assert!(
        md.contains("Region: us-east-1 (US\\|East\\\\nline)."),
        "{md}"
    );
    // The digest has a pipe, a backtick and an asterisk.
    let untagged: Vec<Vec<String>> = md
        .lines()
        .filter(|l| l.starts_with("| app.two"))
        .map(cells)
        .filter(|c| c.len() == 4)
        .collect();
    assert_eq!(untagged.len(), 1, "{md}");
    assert_eq!(untagged[0].len(), 4, "{untagged:?}");
    assert_eq!(untagged[0][1], "sha256:d2\\|x\\`y\\*z");
    // Every row of a table has as many cells as its header: no cell was split.
    let rows: Vec<Vec<String>> = md
        .lines()
        .filter(|l| l.starts_with('|'))
        .map(cells)
        .collect();
    assert!(rows.iter().filter(|c| c.len() == 6).count() >= 4, "{md}");
    assert!(rows
        .iter()
        .all(|c| c.len() == 6 || c.len() == 4 || c.len() == 5));
    assert!(md.contains("| app\\_one |"), "{md}");
    // No raw line break from a value: every line starts with a known marker.
    for line in md.lines() {
        assert!(
            line.is_empty()
                || [
                    "# ",
                    "## ",
                    "| ",
                    "Estimate",
                    "Source:",
                    "Not executed",
                    "```",
                    "As of",
                    "Omitted",
                    "{",
                    "}",
                    " ",
                    "- "
                ]
                .iter()
                .any(|m| line.starts_with(m)),
            "stray line {line:?}"
        );
    }
    assert!(!md.contains('\r'));
}

#[test]
fn markdown_and_json_together_are_a_usage_error() {
    let (code, stdout, stderr) = run(&["--md", "--json", "examples/describe-images.json"]);
    assert_eq!(code, 2);
    assert!(stdout.is_empty());
    assert!(stderr.starts_with("error:"), "{stderr}");
}

#[test]
fn markdown_keeps_the_exit_code_and_errors_of_the_other_formats() {
    let (code, stdout, stderr) = run(&["--md", "tests/fixtures/does-not-exist.json"]);
    assert_eq!(code, 2);
    assert!(stdout.is_empty());
    assert!(stderr.starts_with("ecr-bill: "), "{stderr}");
    let (code, md, _) = run(&[
        "--md",
        "tests/fixtures/no-push-time.json",
        "--as-of",
        "2026-11-20",
    ]);
    assert_eq!(code, 0);
    assert!(md.contains("## Warnings"), "{md}");
}

#[cfg(target_os = "linux")]
#[test]
fn markdown_to_a_full_device_exits_2_without_a_panic() {
    let full = std::fs::OpenOptions::new()
        .write(true)
        .open("/dev/full")
        .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_ecr-bill"))
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .args(["--md", "examples/describe-images.json"])
        .stdout(full)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.contains("could not write output"), "{stderr}");
    assert!(!stderr.contains("panicked"));
}
