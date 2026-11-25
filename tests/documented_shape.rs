//! The real input shape of `aws ecr describe-images`, as the AWS CLI reference
//! documents it: <https://docs.aws.amazon.com/cli/latest/reference/ecr/describe-images.html>
//! (output: `imageDetails` with `registryId`, `repositoryName`, `imageDigest`,
//! `imageTags`, `imageSizeInBytes`, `imagePushedAt`, `imageManifestMediaType`,
//! `artifactMediaType`, `imageScanStatus`, `imageScanFindingsSummary`,
//! `lastRecordedPullTime`, `imageStatus`, and a `nextToken` beside the list).
//! The files are in `tests/fixtures/` and are synthetic.
//!
//! Version 1 of the CLI writes times as epoch seconds with a fraction; version 2
//! writes ISO 8601 strings with a zone. Both are covered, with the variants that
//! occur in practice: tags missing or an empty list, several tags, names with
//! `/`, `_`, `.` and `-`, extra fields, a `nextToken`, an empty list, and
//! several files in one call.

use std::process::Command;

const V1: &str = "tests/fixtures/documented-cli-v1.json";
const V2: &str = "tests/fixtures/documented-cli-v2.json";
const EMPTY: &str = "tests/fixtures/documented-cli-empty.json";

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

#[test]
fn the_documented_cli_v1_shape_is_priced_exactly() {
    let (code, stdout, stderr) = run(&["--as-of", "2026-11-20", V1]);
    assert_eq!(code, 0, "{stderr}");
    assert!(stderr.is_empty(), "{stderr}");
    // 2 + 1 + 1 GiB Standard in one repository, 10 GiB archived in another.
    assert!(stdout.contains(
        "team/app-api_v2.0 images=3 bytes=4294967296 untagged=2 old=3 estimated_usd_month=0.40\n"
    ));
    assert!(stdout.contains(
        "legacy.tools images=1 bytes=10737418240 untagged=0 old=1 estimated_usd_month=1.00\n"
    ));
    assert!(
        stdout.contains("total images=4 bytes=15032385536 untagged=2 estimated_usd_month=1.40\n")
    );
    assert!(!stdout.contains("012345678910"));
    // An empty tag list and a missing one are both untagged.
    assert_eq!(stdout.matches("tags=0 bytes=").count(), 2);
}

#[test]
fn the_documented_cli_v2_shape_with_iso_times_and_zones_is_priced_exactly() {
    let (code, stdout, stderr) = run(&["--as-of", "2026-11-20", V2]);
    assert_eq!(code, 0, "{stderr}");
    assert!(stderr.is_empty(), "{stderr}");
    // 3 + 1 GiB Standard is 0.40 USD; only the image of 2026-01-01 is older than 30 days.
    assert!(stdout.contains(
        "team/app-api_v2.0 images=2 bytes=4294967296 untagged=1 old=1 estimated_usd_month=0.40\n"
    ));
    assert!(stdout.contains("age_days=323 tags=0 bytes=1073741824\n"));
}

#[test]
fn an_empty_list_a_next_token_and_several_files_add_up_exactly() {
    let (code, stdout, stderr) = run(&["--json", "--as-of", "2026-11-20", V1, V2, EMPTY]);
    assert_eq!(code, 0, "{stderr}");
    assert!(stderr.is_empty(), "{stderr}");
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["images"], 6);
    assert_eq!(json["bytes"], "19327352832");
    assert_eq!(json["untagged_images"], 3);
    // 8 GiB Standard (0.80) and 10 GiB archived (1.00).
    assert_eq!(json["estimated_usd_month"], "1.80");
    let repos = json["repositories"].as_array().unwrap();
    assert_eq!(repos.len(), 2);
    assert_eq!(repos[0]["repository"], "legacy.tools");
    assert_eq!(repos[0]["estimated_usd_month"], "1.00");
    assert_eq!(repos[1]["repository"], "team/app-api_v2.0");
    assert_eq!(repos[1]["estimated_usd_month"], "0.80");
}

#[test]
fn an_empty_list_alone_is_a_zero_bill_not_an_error() {
    let (code, stdout, stderr) = run(&[EMPTY]);
    assert_eq!(code, 0, "{stderr}");
    assert!(stdout.contains("total images=0 bytes=0 untagged=0 estimated_usd_month=0.00\n"));
}
