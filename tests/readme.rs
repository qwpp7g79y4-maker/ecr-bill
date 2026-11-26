//! README.md is executable: console blocks match the binary, and every
//! numbered claim has a test.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn normalize_newlines(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

fn read(rel: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel);
    let text = fs::read_to_string(&path).unwrap_or_else(|err| panic!("read {rel}: {err}"));
    normalize_newlines(&text)
}

fn section<'a>(text: &'a str, heading: &str) -> &'a str {
    let marker = format!("{heading}\n");
    let start = text
        .find(&marker)
        .unwrap_or_else(|| panic!("missing heading {heading}"))
        + marker.len();
    let rest = &text[start..];
    match rest.find("\n## ") {
        Some(end) => &rest[..end],
        None => rest,
    }
}

fn rust_sources() -> String {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut out = String::new();
    for dir in ["src", "tests"] {
        for entry in fs::read_dir(root.join(dir)).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
                out.push_str(&fs::read_to_string(&path).unwrap());
                out.push('\n');
            }
        }
    }
    out
}

fn console_blocks(doc: &str) -> Vec<(String, Vec<String>, String)> {
    let mut out = Vec::new();
    let mut lines = doc.lines();
    while let Some(line) = lines.next() {
        if line != "```console" {
            continue;
        }
        let call = lines.next().expect("empty console block");
        let args: Vec<String> = call
            .strip_prefix("$ ecr-bill ")
            .unwrap_or_else(|| panic!("console block does not start with `$ ecr-bill `: {call}"))
            .split_whitespace()
            .map(str::to_string)
            .collect();
        let mut body = String::new();
        let mut closed = false;
        for line in lines.by_ref() {
            if line == "```" {
                closed = true;
                break;
            }
            body.push_str(line);
            body.push('\n');
        }
        assert!(closed, "console block not closed: {call}");
        out.push((call.to_string(), args, body));
    }
    assert!(!out.is_empty(), "README has no console block");
    out
}

/// The `## ` headings of a document, outside fenced code blocks.
fn headings(doc: &str) -> Vec<String> {
    let mut fenced = false;
    let mut out = Vec::new();
    for line in doc.lines() {
        if line.starts_with("```") {
            fenced = !fenced;
        } else if !fenced && line.starts_with("## ") {
            out.push(line.trim_start_matches("## ").to_string());
        }
    }
    out
}

#[test]
fn readme_sections_follow_the_documentation_standard() {
    assert_eq!(
        headings(&read("README.md")),
        vec![
            "Why",
            "Installation",
            "Quick start",
            "Quickstart in 5 minutes",
            "Options",
            "Example",
            "How it counts",
            "Exit codes",
            "Limitations",
            "Prices and responsibility",
            "Maintenance",
            "Support",
            "License",
            "Contact",
        ]
    );
}

#[test]
fn the_dutch_readme_has_the_same_sections_and_the_same_console_blocks() {
    assert_eq!(
        headings(&read("README.nl.md")),
        vec![
            "Waarom",
            "Installatie",
            "Snelstart",
            "Aan de slag in 5 minuten",
            "Opties",
            "Voorbeeld",
            "Hoe het telt",
            "Afsluitcodes",
            "Beperkingen",
            "Prijzen en verantwoordelijkheid",
            "Onderhoud",
            "Ondersteuning",
            "Licentie",
            "Contact",
        ]
    );
    let en = console_blocks(&read("README.md"));
    let nl = console_blocks(&read("README.nl.md"));
    assert_eq!(en, nl);
    let nl_text = read("README.nl.md");
    let how = section(&nl_text, "## Hoe het telt");
    let ids = how
        .lines()
        .filter(|l| l.starts_with('C') && l[1..].chars().next().is_some_and(|c| c.is_ascii_digit()))
        .count();
    assert_eq!(ids, 18);
}

#[test]
fn contact_is_last_and_names_the_maintainer_and_both_readmes_agree() {
    for name in ["README.md", "README.nl.md"] {
        let text = read(name);
        let contact = section(&text, "## Contact");
        for needle in [
            "pep_ai@icloud.com",
            "https://www.pepai.nl",
            "https://x.com/pepai_nl",
            "https://ko-fi.com/pepai",
        ] {
            assert!(contact.contains(needle), "{name}: contact lacks {needle}");
        }
        assert!(text.trim_end().ends_with(contact.trim_end()));
        assert!(!text.contains("## PEPAI"));
    }
}

#[test]
fn installation_uses_a_local_path() {
    let readme = read("README.md");
    let install = section(&readme, "## Installation");
    assert!(install.contains("cargo install --locked --path ."));
    assert!(install.contains("1.85"));
    assert!(install.contains("cargo install ecr-bill"));
    assert!(!install.contains("not published"));
}

#[test]
fn the_quick_start_writes_nothing_and_the_example_file_is_the_one_described() {
    for name in ["README.md", "README.nl.md"] {
        let readme = read(name);
        assert!(!readme.contains("<<'JSON'"), "{name} writes a file");
        for line in readme.lines() {
            assert!(
                !(line.contains("> examples/") || line.contains("cat >")),
                "{name} writes into the repository: {line}"
            );
        }
    }
    let example = read("examples/describe-images.json");
    assert!(example.contains("\"repositoryName\": \"app\""));
    assert!(example.contains("1073741824"));
}

#[test]
fn the_five_minute_commands_run_literally_and_exit_0() {
    for name in ["README.md", "README.nl.md"] {
        let readme = read(name);
        let heading = if name == "README.md" {
            "## Quickstart in 5 minutes"
        } else {
            "## Aan de slag in 5 minuten"
        };
        let block = section(&readme, heading);
        let start = block.find("```sh\n").expect("sh block") + 6;
        let end = start + block[start..].find("```").expect("end of block");
        let calls: Vec<&str> = block[start..end].lines().collect();
        assert_eq!(calls.len(), 5, "{name}");
        for call in calls {
            let args: Vec<&str> = call
                .strip_prefix("ecr-bill ")
                .unwrap_or_else(|| panic!("not an ecr-bill call: {call}"))
                .split_whitespace()
                .collect();
            let out = Command::new(env!("CARGO_BIN_EXE_ecr-bill"))
                .current_dir(env!("CARGO_MANIFEST_DIR"))
                .env("NO_COLOR", "1")
                .args(&args)
                .output()
                .unwrap();
            assert!(out.status.success(), "{call}: {:?}", out.stderr);
            assert!(!out.stdout.is_empty(), "{call}");
        }
    }
}

#[test]
fn console_examples_match_the_binary() {
    let readme = read("README.md");
    for (call, args, expected) in console_blocks(&readme) {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_ecr-bill"));
        cmd.current_dir(env!("CARGO_MANIFEST_DIR"));
        cmd.env("NO_COLOR", "1");
        cmd.args(&args);
        let out = cmd.output().unwrap();
        let stdout = String::from_utf8(out.stdout).unwrap();
        let stderr = String::from_utf8(out.stderr).unwrap();
        assert!(out.status.success(), "{call} failed: {stderr}");
        assert!(stderr.is_empty(), "{call} wrote to stderr: {stderr}");
        assert_eq!(stdout, expected, "console mismatch for {call}");
        assert!(!stdout.contains("123456789012"));
    }
}

#[test]
fn every_how_it_counts_claim_has_a_test() {
    let readme = read("README.md");
    let how = section(&readme, "## How it counts");
    let mut ids = Vec::new();
    for line in how.lines() {
        let Some(rest) = line.trim().strip_prefix('C') else {
            continue;
        };
        let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        if digits.is_empty() || !rest[digits.len()..].starts_with('.') {
            continue;
        }
        ids.push(digits.parse::<u32>().unwrap());
    }
    assert_eq!(ids, (1..=18).collect::<Vec<_>>());
    assert!(how.contains("half away from zero"));
    assert!(how.contains("nearest cent"));
    let sources = rust_sources();
    for id in 1..=18 {
        let needle = format!("fn claim_c{id}_");
        assert!(sources.contains(&needle), "missing {needle}");
    }
}

#[test]
fn limitations_name_behaviors_the_tests_cover() {
    let readme = read("README.md");
    let limits = section(&readme, "## Limitations");
    for phrase in [
        "not executed",
        "PutLifecyclePolicy",
        "no network",
        "AWS SDK",
        "free tier is not applied",
        "retrieval",
        "us-east-1",
        "symlink",
        "directory",
        "268435456",
        "same digest counts twice",
        "not a forecast",
        "unrounded",
        "90",
        "other registries",
    ] {
        assert!(limits.contains(phrase), "limitations missing {phrase}");
    }
    let sources = rust_sources();
    for needle in [
        "fn file_larger_than_268435456_bytes",
        "fn directory_is_rejected_and_symlink_is_followed",
        "fn claim_c5_",
        "fn claim_c11_",
        "fn claim_c16_",
    ] {
        assert!(sources.contains(needle), "missing {needle}");
    }
}

#[test]
fn section_finds_a_heading_when_the_text_uses_crlf() {
    let crlf = "## Installation\r\ncargo install --locked --path .\r\n## Quick start\r\n";
    let text = normalize_newlines(crlf);
    let install = section(&text, "## Installation");
    assert!(install.contains("cargo install --locked --path ."));
    assert!(!install.contains("Quick start"));
    let readme = read("README.md");
    assert!(!readme.contains('\r'));
    assert_eq!(normalize_newlines(&readme.replace('\n', "\r\n")), readme);
}

#[test]
fn licenses_and_changelog_match_the_package() {
    let mit = read("LICENSE-MIT");
    assert!(mit.contains("Copyright (c) 2026 PEPAI"));
    let apache = read("LICENSE-APACHE");
    assert!(apache.contains("Copyright 2026 PEPAI"));
    let changelog = read("CHANGELOG.md");
    assert!(changelog.contains("## [1.0.0] - 2026-11-19"));
    assert!(changelog.contains("## [0.1.0] - 2026-11-09"));
    assert!(changelog.contains("not published"));
    let cargo = read("Cargo.toml");
    assert!(cargo.contains("MIT OR Apache-2.0"));
    assert!(cargo.contains("rust-version = \"1.85\""));
    assert!(!cargo.contains("[dev-dependencies]"));
    let readme = read("README.md");
    let license = section(&readme, "## License");
    assert!(license.contains("MIT OR Apache-2.0"));
    assert!(license.contains("LICENSE-MIT"));
    assert!(license.contains("LICENSE-APACHE"));
    assert!(readme.contains("https://aws.amazon.com/ecr/pricing/"));
    assert!(readme.contains("2026-11-09"));
}

/// Source text and lock file name no network client, no AWS SDK and no code
/// that starts another process. This reads text; it does not watch system calls.
#[test]
fn claim_r1_sources_and_lock_file_have_no_network_or_process_code() {
    let join = |parts: &[&str]| parts.concat();
    let source_needles = [
        join(&["aws", "-sdk"]),
        join(&["req", "west"]),
        join(&["std::", "net"]),
        join(&["Tcp", "Stream"]),
        join(&["Udp", "Socket"]),
        join(&["Put", "LifecyclePolicy"]),
    ];
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for entry in fs::read_dir(root.join("src")).unwrap() {
        let path = entry.unwrap().path();
        let text = fs::read_to_string(&path).unwrap();
        for needle in &source_needles {
            assert!(
                !text.contains(needle.as_str()),
                "{} has {needle}",
                path.display()
            );
        }
        let process = join(&["Command", "::new"]);
        assert!(
            !text.contains(&process),
            "{} starts a process",
            path.display()
        );
    }
    let lock = read("Cargo.lock");
    let names: Vec<&str> = lock
        .lines()
        .filter_map(|line| line.strip_prefix("name = \""))
        .map(|rest| rest.trim_end_matches('"'))
        .collect();
    assert!(names.contains(&"clap"));
    for bad in [
        "reqwest",
        "hyper",
        "ureq",
        "curl",
        "tokio",
        "openssl",
        "rustls",
        "native-tls",
        "isahc",
        "attohttpc",
    ] {
        assert!(!names.contains(&bad), "Cargo.lock has {bad}");
    }
    assert!(
        !names.iter().any(|name| name.starts_with("aws-")),
        "Cargo.lock has an aws crate"
    );
}

#[test]
fn the_package_list_is_an_allow_list_without_working_file_names() {
    let cargo = read("Cargo.toml");
    assert!(
        cargo.contains("\ninclude = ["),
        "Cargo.toml needs an include list"
    );
    assert!(
        !cargo.contains("exclude"),
        "an exclude list would name files"
    );
}

/// The rows of the claim table in QUALITY.md: id, tests cell, status cell.
fn claim_rows(quality: &str) -> Vec<(String, String, String)> {
    let mut rows = Vec::new();
    for line in quality.lines() {
        let cells: Vec<&str> = line.split(" | ").collect();
        if cells.len() != 4 || !line.starts_with("| ") {
            continue;
        }
        let id = cells[0].trim_start_matches("| ").trim();
        let mut chars = id.chars();
        let first = chars.next();
        if !matches!(first, Some('C' | 'R' | 'M')) || id.len() < 2 {
            continue;
        }
        if !chars.all(|c| c.is_ascii_digit()) {
            continue;
        }
        rows.push((
            id.to_string(),
            cells[2].to_string(),
            cells[3].trim_end_matches(" |").trim().to_string(),
        ));
    }
    rows
}

#[test]
fn the_quality_claim_table_counts_match_its_summary_and_name_real_tests() {
    let quality = read("QUALITY.md");
    let rows = claim_rows(&quality);
    assert!(rows.len() >= 50, "the claim table has {} rows", rows.len());
    let retired = rows.iter().filter(|r| r.2 == "not a claim").count();
    let tested = rows.iter().filter(|r| r.2 == "tested").count();
    let gaps = rows.iter().filter(|r| r.2.starts_with("gap")).count();
    assert_eq!(retired + tested + gaps, rows.len(), "an unknown status");
    let claims = rows.len() - retired;
    let summary = format!("{claims} claims ({retired} retired): {tested} tested, {gaps} gaps");
    assert!(
        quality.contains(&summary),
        "QUALITY.md must say `{summary}`"
    );
    // Every test the table names exists as a function in src/ or tests/.
    let sources = rust_sources();
    for (id, tests, status) in &rows {
        if status == "tested" {
            assert!(tests.contains('`'), "{id} is tested by no test");
        }
        let mut parts = tests.split('`');
        parts.next();
        for token in parts.step_by(2) {
            let plain = !token.is_empty()
                && token
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == ':');
            if !plain || !token.contains('_') {
                continue;
            }
            let name = token.rsplit("::").next().unwrap();
            assert!(
                sources.contains(&format!("fn {name}")),
                "{id} names {token}, which is not a test"
            );
        }
    }
}

#[test]
fn the_example_directory_has_the_expected_output_of_its_command() {
    let readme = read("examples/README.md");
    assert!(readme.contains("ecr-bill --as-of 2026-11-09 examples/describe-images.json"));
    let out = Command::new(env!("CARGO_BIN_EXE_ecr-bill"))
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .env("NO_COLOR", "1")
        .args(["--as-of", "2026-11-09", "examples/describe-images.json"])
        .output()
        .unwrap();
    assert!(out.status.success());
    assert_eq!(
        normalize_newlines(&String::from_utf8(out.stdout).unwrap()),
        read("examples/expected-output.txt")
    );
}

#[test]
fn the_markdown_example_in_both_readmes_is_what_the_binary_prints() {
    let out = Command::new(env!("CARGO_BIN_EXE_ecr-bill"))
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .env("NO_COLOR", "1")
        .args([
            "--md",
            "--as-of",
            "2026-11-09",
            "examples/describe-images.json",
        ])
        .output()
        .unwrap();
    assert!(out.status.success());
    let expected = String::from_utf8(out.stdout).unwrap();
    for name in ["README.md", "README.nl.md"] {
        let text = read(name);
        let start = text
            .find("````markdown\n")
            .unwrap_or_else(|| panic!("{name}: no block"))
            + 13;
        let end = start + text[start..].find("````\n").unwrap();
        assert_eq!(&text[start..end], expected, "{name}");
        assert!(text.contains("ecr-bill --md --as-of 2026-11-09 examples/describe-images.json"));
    }
}
