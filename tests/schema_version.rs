//! Every JSON report has `"schema_version": 1` as its first key. A run that
//! fails prints no JSON at all: the error is one text line on standard error.

use std::process::Command;

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

fn assert_versioned(json: &str, what: &str) {
    let value: serde_json::Value =
        serde_json::from_str(json).unwrap_or_else(|e| panic!("{what}: {e}"));
    let object = value
        .as_object()
        .unwrap_or_else(|| panic!("{what}: not an object"));
    let first = object.iter().next().unwrap();
    assert_eq!(first.0, "schema_version", "{what}: first key");
    assert_eq!(first.1, 1, "{what}");
    assert_eq!(
        object.keys().filter(|k| *k == "schema_version").count(),
        1,
        "{what}"
    );
}

#[test]
fn every_json_report_starts_with_the_schema_version() {
    let inputs: [&[&str]; 7] = [
        &["examples/describe-images.json"],
        &[
            "tests/fixtures/documented-cli-v1.json",
            "tests/fixtures/documented-cli-v2.json",
        ],
        &["tests/fixtures/documented-cli-empty.json"],
        &["tests/fixtures/no-push-time.json"],
        &["--as-of", "2026-11-20", "examples/describe-images.json"],
        &[
            "--prices",
            "tests/fixtures/prices-pipes-and-lines.json",
            "tests/fixtures/pipes-and-lines.json",
        ],
        &[
            "--keep",
            "1",
            "--untagged-days",
            "1",
            "--older-than-days",
            "1",
            "examples/describe-images.json",
        ],
    ];
    for args in inputs {
        let mut all = vec!["--json"];
        all.extend(args);
        let (code, stdout, stderr) = run(&all);
        assert_eq!(code, 0, "{args:?}: {stderr}");
        assert_versioned(&stdout, &format!("{args:?}"));
        // The flags that change where or how much is printed do not change the JSON.
        for flag in ["--quiet", "--verbose"] {
            let mut flagged = vec!["--json", flag];
            flagged.extend(args);
            let (code, flagged_out, _) = run(&flagged);
            assert_eq!(code, 0);
            assert_eq!(flagged_out, stdout, "{flag} {args:?}");
        }
    }
}

#[test]
fn the_json_in_an_output_file_has_the_schema_version() {
    let dir = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("schema-version");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("report.json");
    let (code, stdout, stderr) = run(&[
        "--json",
        "--output",
        path.to_str().unwrap(),
        "examples/describe-images.json",
    ]);
    assert_eq!(code, 0, "{stderr}");
    assert!(stdout.is_empty());
    assert_versioned(&std::fs::read_to_string(&path).unwrap(), "output file");
}

#[test]
fn a_failed_json_run_prints_no_json_only_one_error_line() {
    for args in [
        &["--json", "tests/fixtures/does-not-exist.json"][..],
        &[
            "--json",
            "--prices",
            "tests/fixtures/does-not-exist.json",
            "examples/describe-images.json",
        ][..],
        &[
            "--json",
            "--as-of",
            "2026-13-40",
            "examples/describe-images.json",
        ][..],
        &["--json", "Cargo.toml"][..],
    ] {
        let (code, stdout, stderr) = run(args);
        assert_eq!(code, 2, "{args:?}: {stderr}");
        assert!(stdout.is_empty(), "{args:?}: {stdout}");
        assert!(!stderr.contains("schema_version"), "{stderr}");
        assert_eq!(stderr.lines().count(), 1, "{args:?}: {stderr}");
    }
}

#[test]
fn the_readme_json_example_and_the_docs_name_the_schema_version() {
    for name in ["README.md", "README.nl.md", "QUALITY.md", "CHANGELOG.md"] {
        let text =
            std::fs::read_to_string(format!("{}/{name}", env!("CARGO_MANIFEST_DIR"))).unwrap();
        assert!(text.contains("schema_version"), "{name}");
    }
}
