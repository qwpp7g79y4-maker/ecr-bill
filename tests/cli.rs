//! Command-line behavior: exit codes, files, and the report the binary prints.

use std::fs;
#[cfg(unix)]
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

struct Scratch {
    path: PathBuf,
}

impl Scratch {
    fn new() -> Self {
        static N: AtomicU64 = AtomicU64::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("ecr-bill-cli-{}-{n}", std::process::id()));
        fs::create_dir_all(&path).unwrap();
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

/// The price date of the built-in prices: a fixed day, so no test reads the clock.
const AS_OF: &str = "2026-11-09";

fn command(args: &[&str]) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_ecr-bill"));
    cmd.current_dir(env!("CARGO_MANIFEST_DIR"));
    cmd.env("NO_COLOR", "1");
    cmd.args(args);
    cmd
}

fn run(args: &[&str]) -> (i32, String, String) {
    let out = command(args).output().unwrap();
    (
        out.status.code().unwrap(),
        String::from_utf8(out.stdout).unwrap(),
        String::from_utf8(out.stderr).unwrap(),
    )
}

#[test]
fn help_and_version_exit_0() {
    let (code, stdout, stderr) = run(&["--help"]);
    assert_eq!(code, 0);
    assert!(stderr.is_empty(), "{stderr}");
    assert!(stdout.contains("no network"));
    assert!(stdout.contains("Exit codes: 0 ok, 2 input or usage error, 130 interrupted (SIGINT)."));
    let (code, stdout, stderr) = run(&["--version"]);
    assert_eq!(code, 0);
    assert!(stderr.is_empty(), "{stderr}");
    assert!(stdout.contains("ecr-bill 1.0.0"));
}

#[test]
fn usage_errors_escape_control_and_bidi_characters() {
    let (code, _, stderr) = run(&["--keep=a\u{202E}b\u{200B}c", "x"]);
    assert_eq!(code, 2);
    assert!(stderr.contains("a\\u{202e}b\\u{200b}c"), "{stderr:?}");
    assert!(!stderr.contains('\u{202E}') && !stderr.contains('\u{200B}'));
}

#[test]
fn usage_errors_exit_2() {
    let (code, stdout, stderr) = run(&[]);
    assert_eq!(code, 2);
    assert!(stdout.is_empty());
    assert!(stderr.contains("ecr-bill"));
    let (code, _, stderr) = run(&["--no-such-flag", "x"]);
    assert_eq!(code, 2);
    assert!(!stderr.is_empty());
    let (code, _, stderr) = run(&["--keep=0", "examples/describe-images.json"]);
    assert_eq!(code, 2);
    assert!(!stderr.is_empty());
    let (code, _, stderr) = run(&["--untagged-days=-1", "examples/describe-images.json"]);
    assert_eq!(code, 2);
    assert!(!stderr.is_empty());
    let (code, stdout, stderr) = run(&["--as-of", "2025-02-29", "examples/describe-images.json"]);
    assert_eq!(code, 2);
    assert!(stdout.is_empty());
    assert!(stderr.starts_with("ecr-bill: "));
    assert!(stderr.contains("invalid --as-of"));
    assert!(!stderr.contains("123456789012"));
}

#[test]
fn missing_file_exits_2() {
    let (code, stdout, stderr) = run(&["missing-images.json"]);
    assert_eq!(code, 2);
    assert!(stdout.is_empty());
    assert!(stderr.starts_with("ecr-bill: "));
    assert!(stderr.contains("missing-images.json"));
}

const ESTIMATE_LINE: &str =
    "Estimate from list prices, not your AWS invoice (free tier, credits, discounts not included)";

#[test]
fn text_and_json_say_the_amount_is_an_estimate() {
    let (code, stdout, _) = run(&["--as-of", "2026-11-09", "examples/describe-images.json"]);
    assert_eq!(code, 0);
    assert_eq!(stdout.matches(ESTIMATE_LINE).count(), 1, "{stdout}");
    assert!(stdout.lines().any(|line| line == ESTIMATE_LINE));
    assert!(stdout.contains("total images=2 bytes=2147483648 untagged=1 estimated_usd_month=0.20"));
    assert!(!stdout.contains(" usd_month="), "{stdout}");
    let (code, json, _) = run(&[
        "--json",
        "--as-of",
        "2026-11-09",
        "examples/describe-images.json",
    ]);
    assert_eq!(code, 0);
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["estimate"], serde_json::Value::Bool(true));
    assert_eq!(parsed["estimated_usd_month"], "0.20");
    assert_eq!(parsed["repositories"][0]["estimated_usd_month"], "0.20");
    assert!(parsed.get("usd_month").is_none());
    assert!(parsed["repositories"][0].get("usd_month").is_none());
}

#[test]
fn example_hides_the_account_and_json_is_quiet() {
    let (code, stdout, stderr) = run(&["--as-of", "2026-11-09", "examples/describe-images.json"]);
    assert_eq!(code, 0, "{stderr}");
    assert!(stderr.is_empty(), "{stderr}");
    assert!(stdout.contains("estimated_usd_month=0.20"));
    assert!(stdout.contains("Not executed"));
    assert!(!stdout.contains("123456789012"));
    let (code, stdout, stderr) = run(&[
        "--json",
        "--as-of",
        "2026-11-09",
        "examples/describe-images.json",
    ]);
    assert_eq!(code, 0, "{stderr}");
    assert!(stderr.is_empty(), "{stderr}");
    assert!(stdout.starts_with('{'));
    assert!(stdout.contains("\"not_executed\": true"));
    assert!(stdout.contains("\"estimated_usd_month\": \"0.20\""));
    assert!(!stdout.contains("123456789012"));
}

#[test]
fn keep_changes_the_proposed_count() {
    let (code, stdout, stderr) = run(&["--quiet", "--keep", "3", "examples/describe-images.json"]);
    assert_eq!(code, 0, "{stderr}");
    assert!(stderr.is_empty());
    assert!(stdout.contains("\"countNumber\": 3"));
    assert!(stdout.contains("Keep the newest 3 tagged images"));
    assert!(stdout.contains("omitted: pass --as-of YYYY-MM-DD"));
}

#[test]
fn symlink_is_followed_and_a_directory_is_rejected() {
    let scratch = Scratch::new();
    let (code, stdout, stderr) = run(&["--as-of", AS_OF, scratch.path().to_str().unwrap()]);
    assert_eq!(code, 2);
    assert!(stdout.is_empty());
    assert!(stderr.contains("is a directory"));
    #[cfg(unix)]
    {
        let file = scratch.path().join("real.json");
        fs::write(
            &file,
            r#"{"imageDetails":[{"repositoryName":"app","imageSizeInBytes":1073741824}]}"#,
        )
        .unwrap();
        let link = scratch.path().join("link.json");
        symlink(&file, &link).unwrap();
        let (code, stdout, stderr) = run(&["--as-of", AS_OF, link.to_str().unwrap()]);
        assert_eq!(code, 0, "{stderr}");
        assert!(stderr.is_empty());
        assert!(stdout.contains("estimated_usd_month=0.10"));
    }
}

#[test]
fn empty_details_are_zero_and_bad_size_hides_the_row() {
    let scratch = Scratch::new();
    let empty = scratch.path().join("empty.json");
    fs::write(&empty, r#"{"imageDetails":[]}"#).unwrap();
    let (code, stdout, stderr) = run(&["--as-of", AS_OF, empty.to_str().unwrap()]);
    assert_eq!(code, 0, "{stderr}");
    assert!(stderr.is_empty());
    assert!(stdout.contains("total images=0 bytes=0 untagged=0 estimated_usd_month=0.00"));
    let bad = scratch.path().join("bad.json");
    fs::write(
        &bad,
        r#"{"imageDetails":[{"repositoryName":"app","imageDigest":"sha256:secret","imageSizeInBytes":"12abc"}]}"#,
    )
    .unwrap();
    let (code, stdout, stderr) = run(&["--as-of", AS_OF, bad.to_str().unwrap()]);
    assert_eq!(code, 2);
    assert!(stdout.is_empty());
    assert!(stderr.contains("invalid imageSizeInBytes"));
    assert!(!stderr.contains("secret"));
    assert!(!stderr.contains("12abc"));
}

#[test]
fn push_time_files_accept_iso8601_and_an_exponent() {
    let scratch = Scratch::new();
    let iso = scratch.path().join("iso.json");
    fs::write(
        &iso,
        r#"{"imageDetails":[{"repositoryName":"app","imageSizeInBytes":1073741824,"imagePushedAt":"2024-01-15T00:00:00+00:00"}]}"#,
    )
    .unwrap();
    let exp = scratch.path().join("exp.json");
    fs::write(
        &exp,
        r#"{"imageDetails":[{"repositoryName":"app","imageSizeInBytes":1073741824,"imagePushedAt":1.7052768E9}]}"#,
    )
    .unwrap();
    let bad = scratch.path().join("bad-pushed.json");
    fs::write(
        &bad,
        r#"{"imageDetails":[{"repositoryName":"app","imageSizeInBytes":1,"imagePushedAt":"not-a-time"}]}"#,
    )
    .unwrap();
    for path in [&iso, &exp] {
        let (code, stdout, stderr) = run(&["--as-of", "2026-11-09", path.to_str().unwrap()]);
        assert_eq!(code, 0, "{path:?} {stderr}");
        assert!(stderr.is_empty(), "{stderr}");
        assert!(stdout.contains("estimated_usd_month=0.10"), "{stdout}");
        assert!(stdout.contains("age_days=1029"), "{stdout}");
    }
    let (code, stdout, stderr) = run(&[bad.to_str().unwrap()]);
    assert_eq!(code, 2);
    assert!(stdout.is_empty());
    assert!(stderr.contains("invalid imagePushedAt"));
    assert!(!stderr.contains("not-a-time"));
}

#[test]
fn non_ascii_push_time_exits_2_without_panic() {
    let scratch = Scratch::new();
    let path = scratch.path().join("pushed.json");
    fs::write(
        &path,
        "{\"imageDetails\":[{\"repositoryName\":\"app\",\"imageSizeInBytes\":1,\"imagePushedAt\":\"2024-01-1\u{e9}00:00:00Z\"}]}",
    )
    .unwrap();
    let (code, stdout, stderr) = run(&[path.to_str().unwrap()]);
    assert_eq!(code, 2, "{stderr}");
    assert!(stdout.is_empty());
    assert!(stderr.contains("invalid imagePushedAt"));
    assert!(!stderr.to_ascii_lowercase().contains("panic"));
    assert!(!stderr.contains("char boundary"));
}

#[test]
fn digest_controls_are_not_raw_in_text() {
    let scratch = Scratch::new();
    let path = scratch.path().join("digest.json");
    fs::write(
        &path,
        r#"{"imageDetails":[{"repositoryName":"app","imageSizeInBytes":1,"imageDigest":"a\nb\u001b"}]}"#,
    )
    .unwrap();
    let (code, stdout, stderr) = run(&["--as-of", AS_OF, path.to_str().unwrap()]);
    assert_eq!(code, 0, "{stderr}");
    assert!(stderr.is_empty(), "{stderr}");
    let line = stdout
        .lines()
        .find(|line| line.contains("tags=0"))
        .expect("digest line");
    assert!(line.contains(r"a\nb\u{1b}"), "{line}");
    assert!(!line.chars().any(char::is_control), "{line:?}");
    assert!(!stdout.chars().any(|ch| ch == '\u{1b}' || ch == '\u{7}'));
}

#[test]
fn digest_with_hangul_filler_is_escaped_in_text() {
    let scratch = Scratch::new();
    let path = scratch.path().join("digest.json");
    fs::write(
        &path,
        r#"{"imageDetails":[{"repositoryName":"app","imageSizeInBytes":1,"imageDigest":"a\u3164b"}]}"#,
    )
    .unwrap();
    let (code, stdout, stderr) = run(&[path.to_str().unwrap()]);
    assert_eq!(code, 0, "{stderr}");
    assert!(!stdout.contains('\u{3164}'), "{stdout:?}");
    assert!(stdout.contains(r"a\u{3164}b"), "{stdout:?}");
}

#[test]
fn digest_with_unassigned_format_chars_is_escaped_in_text() {
    for (json_esc, raw, shown) in [
        (r"\u2065", '\u{2065}', r"a\u{2065}b"),
        (r"\udb40\udc00", '\u{E0000}', r"a\u{e0000}b"),
    ] {
        let scratch = Scratch::new();
        let path = scratch.path().join("digest.json");
        fs::write(
            &path,
            format!(
                r#"{{"imageDetails":[{{"repositoryName":"app","imageSizeInBytes":1,"imageDigest":"a{json_esc}b"}}]}}"#
            ),
        )
        .unwrap();
        let (code, stdout, stderr) = run(&[path.to_str().unwrap()]);
        assert_eq!(code, 0, "{stderr}");
        assert!(!stdout.contains(raw), "{stdout:?}");
        assert!(stdout.contains(shown), "{stdout:?}");
        assert!(
            stdout.contains("total images=1 bytes=1 untagged=1 estimated_usd_month=0.00"),
            "{stdout:?}"
        );
    }
}

#[test]
fn digest_with_hangul_choseong_filler_is_escaped_and_counted() {
    let scratch = Scratch::new();
    let path = scratch.path().join("digest.json");
    fs::write(
        &path,
        r#"{"imageDetails":[{"repositoryName":"app","imageSizeInBytes":1,"imageDigest":"a\u115Fb"}]}"#,
    )
    .unwrap();
    let (code, stdout, stderr) = run(&[path.to_str().unwrap()]);
    assert_eq!(code, 0, "{stderr}");
    assert!(!stdout.contains('\u{115F}'), "{stdout:?}");
    assert!(stdout.contains(r"a\u{115f}b"), "{stdout:?}");
    assert!(
        stdout.contains("total images=1 bytes=1 untagged=1 estimated_usd_month=0.00"),
        "{stdout:?}"
    );
}

#[test]
fn digest_with_spaces_is_rejected() {
    let scratch = Scratch::new();
    let path = scratch.path().join("forge.json");
    fs::write(
        &path,
        r#"{"imageDetails":[{"repositoryName":"app","imageSizeInBytes":5,"imageDigest":"sha256:x bytes=0 tags=9"}]}"#,
    )
    .unwrap();
    let (code, stdout, stderr) = run(&[path.to_str().unwrap()]);
    assert_eq!(code, 2, "{stderr}");
    assert!(stdout.is_empty(), "{stdout:?}");
    assert!(stderr.contains("invalid imageDigest"), "{stderr}");
}

#[test]
fn literal_none_digest_differs_from_missing_digest() {
    let scratch = Scratch::new();
    let path = scratch.path().join("none.json");
    fs::write(
        &path,
        r#"{"imageDetails":[{"repositoryName":"app","imageSizeInBytes":1,"imageDigest":"(none)"},{"repositoryName":"app","imageSizeInBytes":1}]}"#,
    )
    .unwrap();
    let (code, stdout, stderr) = run(&[path.to_str().unwrap()]);
    assert_eq!(code, 0, "{stderr}");
    let lines: Vec<&str> = stdout.lines().filter(|l| l.contains("tags=0")).collect();
    assert_eq!(lines.len(), 2, "{stdout}");
    assert_ne!(lines[0], lines[1], "{stdout}");
}

#[test]
fn prices_file_is_a_small_regular_file() {
    let scratch = Scratch::new();
    let prices = scratch.path().join("prices.json");
    let mut text = include_str!("../prices/ecr-storage.json").to_string();
    text = text.replacen("\"standard\": \"0.10\"", "\"standard\": \"0.50\"", 1);
    fs::write(&prices, text).unwrap();
    let image = scratch.path().join("image.json");
    fs::write(
        &image,
        r#"{"imageDetails":[{"repositoryName":"app","imageSizeInBytes":1073741824}]}"#,
    )
    .unwrap();
    let (code, stdout, stderr) = run(&[
        "--as-of",
        AS_OF,
        "--prices",
        prices.to_str().unwrap(),
        image.to_str().unwrap(),
    ]);
    assert_eq!(code, 0, "{stderr}");
    assert!(stderr.is_empty(), "{stderr}");
    assert!(stdout.contains("estimated_usd_month=0.50"), "{stdout}");
    let (code, stdout, stderr) = run(&[
        "--as-of",
        AS_OF,
        "--prices",
        scratch.path().to_str().unwrap(),
        image.to_str().unwrap(),
    ]);
    assert_eq!(code, 2);
    assert!(stdout.is_empty());
    assert!(stderr.contains("is a directory"));
}

#[test]
fn invalid_price_file_content_exits_2_and_names_the_file() {
    let scratch = Scratch::new();
    let prices = scratch.path().join("broken-prices.json");
    fs::write(&prices, "{}").unwrap();
    let (code, stdout, stderr) = run(&[
        "--prices",
        prices.to_str().unwrap(),
        "examples/describe-images.json",
    ]);
    assert_eq!(code, 2, "{stderr}");
    assert!(stdout.is_empty());
    assert!(stderr.starts_with("ecr-bill: price file "), "{stderr}");
    assert!(stderr.contains("broken-prices.json"), "{stderr}");
}

#[test]
fn an_extreme_price_is_priced_exactly_without_panic() {
    let scratch = Scratch::new();
    let prices = scratch.path().join("prices.json");
    let mut text = include_str!("../prices/ecr-storage.json").to_string();
    text = text.replacen(
        "\"standard\": \"0.10\"",
        "\"standard\": \"79228162514264337593543950335\"",
        1,
    );
    fs::write(&prices, text).unwrap();
    let image = scratch.path().join("image.json");
    fs::write(
        &image,
        r#"{"imageDetails":[{"repositoryName":"app","imageSizeInBytes":2147483648}]}"#,
    )
    .unwrap();
    let (code, stdout, stderr) = run(&[
        "--prices",
        prices.to_str().unwrap(),
        image.to_str().unwrap(),
    ]);
    assert_eq!(code, 0, "{stderr}");
    assert!(
        stdout.contains("estimated_usd_month=158456325028528675187087900670.00"),
        "{stdout}"
    );
    assert!(!stderr.to_ascii_lowercase().contains("panic"));
}

#[test]
fn utf8_and_escaped_keys_and_a_surrogate_pair_in_a_tag() {
    let scratch = Scratch::new();
    let same = scratch.path().join("same.json");
    fs::write(
        &same,
        "{\n  \"imageDetails\": [],\n  \"caf\u{e9}\": 1,\n  \"caf\\u00e9\": 2\n}\n",
    )
    .unwrap();
    let (code, stdout, stderr) = run(&["--as-of", AS_OF, same.to_str().unwrap()]);
    assert_eq!(code, 2, "{stderr}");
    assert!(stdout.is_empty());
    assert!(stderr.contains("duplicate key"), "{stderr}");

    let different = scratch.path().join("different.json");
    fs::write(
        &different,
        "{\n  \"imageDetails\": [],\n  \"caf\u{e9}\": 1,\n  \"caf\\u00e8\": 2\n}\n",
    )
    .unwrap();
    let (code, stdout, stderr) = run(&["--as-of", AS_OF, different.to_str().unwrap()]);
    assert_eq!(code, 0, "{stderr}");
    assert!(stderr.is_empty(), "{stderr}");
    assert!(stdout.contains("total images=0"), "{stdout}");

    let emoji = scratch.path().join("emoji.json");
    fs::write(
        &emoji,
        "{\n  \"imageDetails\": [\n    {\n      \"repositoryName\": \"app\",\n      \"imageSizeInBytes\": 1,\n      \"imageTags\": [\"\\uD83D\\uDE00\"]\n    }\n  ]\n}\n",
    )
    .unwrap();
    let (code, stdout, stderr) = run(&["--as-of", AS_OF, emoji.to_str().unwrap()]);
    assert_eq!(code, 0, "{stderr}");
    assert!(stderr.is_empty(), "{stderr}");
    assert!(
        stdout.contains("total images=1 bytes=1 untagged=0"),
        "{stdout}"
    );
}

#[cfg(unix)]
#[test]
fn prices_fifo_is_refused_without_opening_it() {
    let scratch = Scratch::new();
    let fifo = scratch.path().join("prices.fifo");
    let made = Command::new("mkfifo").arg(&fifo).status().unwrap();
    assert!(made.success());
    let image = scratch.path().join("image.json");
    fs::write(&image, r#"{"imageDetails":[]}"#).unwrap();
    let (code, stdout, stderr) =
        run(&["--prices", fifo.to_str().unwrap(), image.to_str().unwrap()]);
    assert_eq!(code, 2);
    assert!(stdout.is_empty());
    assert!(stderr.contains("not a file"), "{stderr}");
}

#[cfg(target_os = "linux")]
#[test]
fn write_failure_to_dev_full_exits_2_without_panic() {
    for args in [
        &["--help"][..],
        &["--version"][..],
        &["--as-of", "2026-11-09", "examples/describe-images.json"][..],
    ] {
        let full = fs::OpenOptions::new()
            .write(true)
            .open("/dev/full")
            .unwrap();
        let out = command(args).stdout(full).output().unwrap();
        let stderr = String::from_utf8(out.stderr).unwrap();
        assert_eq!(out.status.code(), Some(2), "{args:?}: {stderr}");
        assert!(stderr.contains("could not write output"), "{stderr}");
        assert!(!stderr.contains("panicked"), "{stderr}");
    }
}

#[test]
fn a_deep_second_value_exits_2_without_a_crash() {
    let scratch = Scratch::new();
    let mut text = r#"{"imageDetails":[]}"#.to_string();
    text.push_str(&"[".repeat(100_000));
    let image = scratch.path().join("tail.json");
    fs::write(&image, &text).unwrap();
    let (code, stdout, stderr) = run(&[image.to_str().unwrap()]);
    assert_eq!(code, 2, "{stderr}");
    assert!(stdout.is_empty(), "{stdout}");
    assert!(stderr.contains("not JSON"), "{stderr}");
    assert!(!stderr.contains("panicked"), "{stderr}");

    let prices = scratch.path().join("prices.json");
    fs::write(&prices, &text).unwrap();
    let (code, stdout, stderr) = run(&[
        "--prices",
        prices.to_str().unwrap(),
        "examples/describe-images.json",
    ]);
    assert_eq!(code, 2, "{stderr}");
    assert!(stdout.is_empty(), "{stdout}");
    assert!(stderr.contains("not JSON"), "{stderr}");
    assert!(!stderr.contains("panicked"), "{stderr}");

    let mut number = String::from("1 ");
    number.push_str(&"[".repeat(100_000));
    let scalar = scratch.path().join("scalar.json");
    fs::write(&scalar, number).unwrap();
    let (code, stdout, stderr) = run(&[scalar.to_str().unwrap()]);
    assert_eq!(code, 2, "{stderr}");
    assert!(stdout.is_empty(), "{stdout}");
    assert!(stderr.contains("not JSON"), "{stderr}");
    assert!(!stderr.contains("panicked"), "{stderr}");
}

#[test]
fn single_file_errors_name_the_file() {
    let dir = std::env::temp_dir().join(format!("ecr-bill-name-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let good = r#"{"imageDetails":[{"repositoryName":"aa","imageSizeInBytes":1}]}"#;
    let cases = [
        ("not json", "not JSON"),
        (r#"{"imageDetails":[],"imageDetails":[]}"#, "duplicate"),
        ("{}", "missing imageDetails"),
    ];
    for (n, (body, want)) in cases.iter().enumerate() {
        std::fs::write(dir.join("one.json"), good).unwrap();
        std::fs::write(dir.join("two.json"), good).unwrap();
        let bad = dir.join(format!("bad{n}.json"));
        std::fs::write(&bad, body).unwrap();
        let out = std::process::Command::new(env!("CARGO_BIN_EXE_ecr-bill"))
            .args([dir.join("one.json"), dir.join("two.json"), bad.clone()])
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(2));
        let err = String::from_utf8_lossy(&out.stderr);
        assert!(err.contains(&format!("bad{n}.json")), "{err}");
        assert!(err.contains(want), "{err}");
        assert!(!err.contains("one.json"), "{err}");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[cfg(target_os = "linux")]
#[test]
fn closed_full_and_null_stdout() {
    let scratch = Scratch::new();
    let file = scratch.path().join("empty.json");
    fs::write(&file, r#"{"imageDetails":[]}"#).unwrap();
    let bin = env!("CARGO_BIN_EXE_ecr-bill");
    let path = file.to_str().unwrap();
    let run = |redirect: &str| {
        let script = format!("exec \"$0\" --as-of {AS_OF} \"$1\" {redirect}");
        let out = Command::new("sh")
            .args(["-c", &script, bin, path])
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .output()
            .unwrap();
        (
            out.status.code().unwrap(),
            String::from_utf8(out.stderr).unwrap(),
        )
    };

    let (code, stderr) = run(">&-");
    assert_eq!(code, 2, "{stderr}");
    assert!(stderr.contains("could not write output"), "{stderr}");
    assert!(!stderr.contains("panic"), "{stderr}");

    let (code, stderr) = run(">/dev/full");
    assert_eq!(code, 2, "{stderr}");
    assert!(stderr.contains("could not write output"), "{stderr}");
    assert!(!stderr.contains("panic"), "{stderr}");

    let (code, stderr) = run(">/dev/null");
    assert_eq!(code, 0, "{stderr}");
    assert!(stderr.is_empty(), "{stderr}");

    let (code, stderr) = run("1<>/dev/null");
    assert_eq!(code, 0, "{stderr}");
    assert!(stderr.is_empty(), "{stderr}");

    for flag in ["--help", "--version"] {
        let script = format!("exec \"$0\" {flag} >&-");
        let out = Command::new("sh")
            .args(["-c", &script, bin])
            .output()
            .unwrap();
        let stderr = String::from_utf8(out.stderr).unwrap();
        assert_eq!(out.status.code(), Some(2), "{flag}: {stderr}");
        assert!(stderr.contains("could not write output"), "{stderr}");
        assert!(!stderr.contains("panic"), "{stderr}");
    }
}

#[cfg(target_os = "linux")]
#[test]
fn help_and_version_to_a_full_device_exit_2() {
    for flag in ["--help", "--version"] {
        let full = std::fs::OpenOptions::new()
            .write(true)
            .open("/dev/full")
            .unwrap();
        let status = std::process::Command::new(env!("CARGO_BIN_EXE_ecr-bill"))
            .arg(flag)
            .stdout(full)
            .stderr(std::process::Stdio::null())
            .status()
            .unwrap();
        assert_eq!(status.code(), Some(2), "{flag}");
    }
}

#[cfg(target_os = "linux")]
#[test]
fn non_utf8_file_name_is_shown_as_byte_escape() {
    use std::os::unix::ffi::OsStrExt;
    let scratch = Scratch::new();
    let name = std::ffi::OsStr::from_bytes(b"bad\xff.json");
    let good = scratch.path().join(name);
    fs::write(
        &good,
        r#"{"imageDetails":[{"repositoryName":"app","imageSizeInBytes":1073741824}]}"#,
    )
    .unwrap();
    let out = command(&[]).arg(&good).output().unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert_eq!(out.status.code(), Some(0), "{stderr}");
    assert!(stdout.contains("estimated_usd_month=0.10"), "{stdout}");
    assert!(!stdout.contains('\u{fffd}'), "{stdout}");
    assert!(!stderr.contains('\u{fffd}'), "{stderr}");

    let broken = scratch
        .path()
        .join(std::ffi::OsStr::from_bytes(b"worse\xff.json"));
    fs::write(&broken, "not a document").unwrap();
    let out = command(&[]).arg(&broken).output().unwrap();
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert_eq!(out.status.code(), Some(2), "{stderr}");
    assert!(out.stdout.is_empty());
    assert!(stderr.contains("worse\\xff.json"), "{stderr}");
    assert!(!stderr.contains('\u{fffd}'), "{stderr}");

    let bytes = scratch
        .path()
        .join(std::ffi::OsStr::from_bytes(b"content\xff.json"));
    fs::write(&bytes, [0xff, 0xfe]).unwrap();
    let out = command(&[]).arg(&bytes).output().unwrap();
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert_eq!(out.status.code(), Some(2), "{stderr}");
    assert!(stderr.contains("content\\xff.json: not UTF-8"), "{stderr}");
    assert!(!stderr.contains('\u{fffd}'), "{stderr}");
}

#[test]
fn region_name_with_bidi_override_is_escaped_in_text_and_json() {
    let scratch = Scratch::new();
    let prices = scratch.path().join("prices.json");
    let text = include_str!("../prices/ecr-storage.json").replacen(
        "\"region_name\": \"US East (N. Virginia)\"",
        "\"region_name\": \"US \u{202E}East\"",
        1,
    );
    fs::write(&prices, text).unwrap();
    let image = scratch.path().join("image.json");
    fs::write(
        &image,
        r#"{"imageDetails":[{"repositoryName":"app","imageSizeInBytes":1073741824}]}"#,
    )
    .unwrap();
    let p = prices.to_str().unwrap();
    let i = image.to_str().unwrap();
    let (code, plain, stderr) = run(&["--prices", p, i]);
    assert_eq!(code, 0, "{stderr}");
    assert!(
        !plain.contains('\u{202E}') && plain.contains("US \\u{202e}East"),
        "{plain}"
    );
    let (code, json, stderr) = run(&["--json", "--prices", p, i]);
    assert_eq!(code, 0, "{stderr}");
    assert!(!json.contains('\u{202E}'), "{json}");
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(value["region_name"], "US \\u{202e}East");
    let (_, base, _) = run(&["--json", i]);
    let base: serde_json::Value = serde_json::from_str(&base).unwrap();
    assert_eq!(value["repositories"], base["repositories"]);
}

#[test]
fn file_name_with_bidi_override_still_counts_and_is_escaped() {
    let scratch = Scratch::new();
    let good = scratch.path().join("a\u{202E}b.json");
    fs::write(
        &good,
        r#"{"imageDetails":[{"repositoryName":"app","imageSizeInBytes":1073741824}]}"#,
    )
    .unwrap();
    let bad = scratch.path().join("c\u{202E}d.json");
    fs::write(&bad, "{}").unwrap();
    let (code, stdout, stderr) = run(&[good.to_str().unwrap()]);
    assert_eq!(code, 0, "{stderr}");
    assert!(stdout.contains("estimated_usd_month=0.10"), "{stdout}");
    assert!(!stdout.contains('\u{202E}') && !stderr.contains('\u{202E}'));
    let (code, stdout, stderr) = run(&[bad.to_str().unwrap()]);
    assert_eq!(code, 2);
    assert!(stdout.is_empty());
    assert!(!stderr.contains('\u{202E}'), "{stderr}");
    assert!(stderr.contains("c\\u{202e}d.json"), "{stderr}");
}

/// No stderr line may start with attacker-chosen text: every line of a usage
/// error is clap's own, and the argument is shown on one line with escapes.
fn assert_one_line_per_message(stderr: &str, plain: &str) {
    assert!(!stderr.contains('\u{1b}'), "{stderr:?}");
    assert!(!stderr.contains('\r'), "{stderr:?}");
    for line in stderr.lines() {
        assert!(!line.starts_with("FAKE"), "fake line in {stderr:?}");
    }
    assert_eq!(stderr.lines().count(), plain.lines().count(), "{stderr:?}");
}

#[test]
fn usage_errors_keep_a_newline_in_an_argument_on_one_line() {
    let (_, _, plain_option) = run(&["--keep", "x", "examples/describe-images.json"]);
    for option in ["--keep", "--older-than-days", "--untagged-days"] {
        let value = "1\u{1b}[31m\nFAKE: line";
        let (code, stdout, stderr) = run(&[option, value, "examples/describe-images.json"]);
        assert_eq!(code, 2, "{option}");
        assert!(stdout.is_empty(), "{stdout}");
        assert_one_line_per_message(&stderr, &plain_option);
        assert!(stderr.contains("\\u{1b}[31m\\nFAKE: line"), "{stderr:?}");
    }
    let (_, _, plain_unknown) = run(&["--x", "examples/describe-images.json"]);
    let (code, _, stderr) = run(&["--x\u{1b}[31m\nFAKE: line", "examples/describe-images.json"]);
    assert_eq!(code, 2);
    assert_one_line_per_message(&stderr, &plain_unknown);
    assert!(stderr.contains("\\u{1b}[31m\\nFAKE: line"), "{stderr:?}");
    let (code, _, stderr) = run(&["--json=a\r\nFAKE: line", "examples/describe-images.json"]);
    assert_eq!(code, 2);
    for line in stderr.lines() {
        assert!(!line.starts_with("FAKE"), "{stderr:?}");
    }
    assert!(!stderr.contains('\r'), "{stderr:?}");
    let (code, _, stderr) = run(&["-\u{1b}\nFAKE: line", "examples/describe-images.json"]);
    assert_eq!(code, 2);
    for line in stderr.lines() {
        assert!(!line.starts_with("FAKE"), "{stderr:?}");
    }
    assert!(!stderr.contains('\u{1b}'), "{stderr:?}");
}

// Red before the fix: the message was one line without its closing newline.
#[cfg(target_os = "linux")]
#[test]
fn a_usage_error_for_an_argument_that_is_not_utf8_is_one_line_with_a_newline() {
    use std::os::unix::ffi::OsStrExt;
    let value = std::ffi::OsStr::from_bytes(b"\xff\nerror: FAKE\x1b[31m");
    let out = command(&["--as-of"])
        .arg(value)
        .arg("examples/describe-images.json")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty());
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.ends_with('\n'), "{stderr:?}");
    assert!(!stderr.contains('\u{1b}'), "{stderr:?}");
    for line in stderr.lines() {
        assert!(
            line.starts_with("error: ")
                || line.starts_with("Usage:")
                || line.starts_with("For more"),
            "{stderr:?}"
        );
        assert!(!line.starts_with("error: FAKE"), "{stderr:?}");
    }
}

#[test]
fn json_digest_never_carries_c1_del_or_bidi_characters_raw() {
    let scratch = Scratch::new();
    let image = scratch.path().join("digest.json");
    fs::write(
        &image,
        "{\"imageDetails\":[{\"repositoryName\":\"app\",\"imageSizeInBytes\":1,\
         \"imageDigest\":\"sha256:\\u009b2J\\u009d0;pwn\\u0007\\u0085FAKE\\u007f\\u2028\\u202e\\ufeff\"}]}",
    )
    .unwrap();
    let (code, json, stderr) = run(&["--json", image.to_str().unwrap()]);
    assert_eq!(code, 0, "{stderr}");
    for raw in [
        '\u{9b}', '\u{9d}', '\u{85}', '\u{7f}', '\u{7}', '\u{2028}', '\u{202e}', '\u{feff}',
    ] {
        assert!(!json.contains(raw), "raw U+{:04X} in {json}", raw as u32);
    }
    let value: serde_json::Value = serde_json::from_str(&json).expect("still valid JSON");
    assert_eq!(
        value["untagged"][0]["digest"],
        "sha256:\u{9b}2J\u{9d}0;pwn\u{7}\u{85}FAKE\u{7f}\u{2028}\u{202e}\u{feff}"
    );
}

#[test]
fn quoted_none_and_bare_none_and_a_missing_digest_are_three_different_digests() {
    let scratch = Scratch::new();
    let mut shown = Vec::new();
    for (name, digest) in [
        ("missing", None),
        ("bare", Some("(none)")),
        ("quoted", Some("\\\"(none)\\\"")),
    ] {
        let body = match digest {
            Some(text) => format!(
                "{{\"imageDetails\":[{{\"repositoryName\":\"app\",\"imageSizeInBytes\":1,\"imageDigest\":\"{text}\"}}]}}"
            ),
            None => {
                "{\"imageDetails\":[{\"repositoryName\":\"app\",\"imageSizeInBytes\":1}]}".to_string()
            }
        };
        let image = scratch.path().join(format!("{name}.json"));
        fs::write(&image, body).unwrap();
        let (code, text, stderr) = run(&[image.to_str().unwrap()]);
        assert_eq!(code, 0, "{stderr}");
        let (code, json, stderr) = run(&["--json", image.to_str().unwrap()]);
        assert_eq!(code, 0, "{stderr}");
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        let line = text
            .lines()
            .find(|line| line.starts_with("app ") && line.contains(" tags="))
            .unwrap()
            .to_string();
        shown.push((line, value["untagged"][0]["digest"].to_string()));
    }
    assert_eq!(shown[0].0, "app (none) bytes=1 tags=0");
    for left in 0..3 {
        for right in left + 1..3 {
            assert_ne!(shown[left].0, shown[right].0, "text {shown:?}");
            assert_ne!(shown[left].1, shown[right].1, "json {shown:?}");
        }
    }
}

#[test]
fn price_file_rates_say_missing_only_when_absent() {
    let scratch = Scratch::new();
    let image = scratch.path().join("image.json");
    fs::write(&image, r#"{"imageDetails":[]}"#).unwrap();
    let base = include_str!("../prices/ecr-storage.json");
    let cases = [
        (
            "\"standard\": \"0.10\"",
            "\"standard\": \"1e2\"",
            "not a decimal",
        ),
        (
            "\"standard\": \"0.10\"",
            "\"standard\": \"-0.10\"",
            "not a decimal",
        ),
        (
            "\"standard\": \"0.10\"",
            "\"standard\": 0.1",
            "not a decimal",
        ),
        ("\"standard\": \"0.10\"", "\"x\": 1", "missing standard"),
    ];
    for (n, (from, to, want)) in cases.iter().enumerate() {
        assert!(base.contains(from), "{from}");
        let prices = scratch.path().join(format!("p{n}.json"));
        fs::write(&prices, base.replacen(from, to, 1)).unwrap();
        let (code, _, stderr) = run(&[
            "--prices",
            prices.to_str().unwrap(),
            image.to_str().unwrap(),
        ]);
        assert_eq!(code, 2, "{to}");
        assert!(stderr.contains(want), "{to}: {stderr}");
    }
}

#[test]
fn warnings_are_lines_in_text_and_an_array_plus_stderr_in_json() {
    let scratch = Scratch::new();
    let image = scratch.path().join("status.json");
    fs::write(
        &image,
        r#"{"imageDetails":[{"repositoryName":"app","imageSizeInBytes":1,"imageStatus":"ACTIVATING","imagePushedAt":"2026-11-01T00:00:00+00:00"}]}"#,
    )
    .unwrap();
    let path = image.to_str().unwrap();
    let (code, text, stderr) = run(&["--as-of", AS_OF, path]);
    assert_eq!(code, 0, "{stderr}");
    assert!(stderr.is_empty(), "{stderr}");
    let warning = text
        .lines()
        .find(|line| line.starts_with("warning: "))
        .unwrap_or_else(|| panic!("no warning line in {text}"));
    let (code, json, stderr) = run(&["--as-of", AS_OF, "--json", path]);
    assert_eq!(code, 0, "{stderr}");
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(value["warnings"].as_array().unwrap().len(), 1);
    assert_eq!(stderr.trim_end(), warning);
    assert_eq!(
        format!("warning: {}", value["warnings"][0].as_str().unwrap()),
        warning
    );
}

#[cfg(unix)]
thread_local! {
    static FIFO_SWAP: std::cell::RefCell<Option<PathBuf>> =
        const { std::cell::RefCell::new(None) };
}

#[cfg(unix)]
fn retarget_to_fifo(path: &Path) {
    let fifo = FIFO_SWAP
        .with(|cell| cell.borrow().clone())
        .expect("fifo path");
    fs::remove_file(path).unwrap();
    symlink(&fifo, path).unwrap();
}

// The path is a regular file at the check and a fifo at the open. The open
// must return, and the fifo must not be read as a file.
#[cfg(unix)]
#[test]
fn a_fifo_replaced_between_the_check_and_the_open_is_refused() {
    let scratch = Scratch::new();
    let real = scratch.path().join("real.json");
    fs::write(&real, "{}\n").unwrap();
    let pipe = scratch.path().join("pipe");
    let made = Command::new("mkfifo").arg(&pipe).status().unwrap();
    assert!(made.success());
    let target = scratch.path().join("target");
    symlink(&real, &target).unwrap();

    let target_for_call = target.clone();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        FIFO_SWAP.with(|cell| *cell.borrow_mut() = Some(pipe));
        ecr_bill::set_before_open_for_test(Some(retarget_to_fifo));
        let result = ecr_bill::read_text_file(&target_for_call);
        ecr_bill::set_before_open_for_test(None);
        FIFO_SWAP.with(|cell| *cell.borrow_mut() = None);
        let _ = tx.send(result.map(|_| ()).map_err(|err| err.message().to_string()));
    });
    let got = rx.recv_timeout(std::time::Duration::from_secs(2));
    match got {
        Ok(Ok(())) => panic!("a fifo was read as a file"),
        Ok(Err(message)) => assert!(message.contains("not a file"), "{message}"),
        Err(_) => panic!("read blocked on a fifo"),
    }
}

#[cfg(unix)]
#[test]
fn an_input_fifo_is_refused_without_opening_it() {
    let scratch = Scratch::new();
    let fifo = scratch.path().join("input.fifo");
    let made = Command::new("mkfifo").arg(&fifo).status().unwrap();
    assert!(made.success());
    let (code, stdout, stderr) = run(&[fifo.to_str().unwrap()]);
    assert_eq!(code, 2);
    assert!(stdout.is_empty());
    assert!(stderr.contains("not a file"), "{stderr}");
}

#[test]
fn documented_ranges_are_accepted_at_the_edge_and_refused_beyond_it() {
    let file = "examples/describe-images.json";
    for (args, ok) in [
        (vec!["--keep", "1"], true),
        (vec!["--keep", "10000"], true),
        (vec!["--keep", "10001"], false),
        (vec!["--older-than-days", "36500"], true),
        (vec!["--older-than-days", "36501"], false),
        (vec!["--untagged-days", "1"], true),
        (vec!["--untagged-days", "36501"], false),
        (vec!["--as-of", "0000-01-01"], true),
        (vec!["--as-of", "9999-12-31"], true),
        (vec!["--as-of", "10000-01-01"], false),
    ] {
        let mut all = args.clone();
        all.push(file);
        let (code, _, stderr) = run(&all);
        assert_eq!(code == 0, ok, "{args:?}: {stderr}");
        assert!(code == 0 || code == 2, "{args:?}");
    }
}

#[cfg(target_os = "linux")]
#[test]
fn a_closed_pipe_exits_2_without_a_panic_and_a_null_stdio_exits_0() {
    let bin = env!("CARGO_BIN_EXE_ecr-bill");
    // The reader (`true`) is gone long before the subshell starts the tool.
    let script = r#"(sleep 0.3; "$0" "$1"; echo "exit=$?" >&2) | true"#;
    let out = Command::new("sh")
        .args(["-c", script, bin, "examples/describe-images.json"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.contains("could not write output"), "{stderr}");
    assert!(stderr.contains("exit=2"), "{stderr}");
    assert!(!stderr.contains("panic"), "{stderr}");
    let status = command(&["examples/describe-images.json"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(0));
}

#[test]
fn readme_input_rules_hold_for_bom_utf16_status_case_and_the_byte_range() {
    let scratch = Scratch::new();
    let write = |name: &str, body: &[u8]| {
        let path = scratch.path().join(name);
        fs::write(&path, body).unwrap();
        path.to_str().unwrap().to_string()
    };
    let one = r#"{"imageDetails":[{"repositoryName":"aa","imageSizeInBytes":1073741824}]}"#;
    let mut bom = vec![0xEF, 0xBB, 0xBF];
    bom.extend_from_slice(one.as_bytes());
    let (code, stdout, stderr) = run(&[&write("bom.json", &bom)]);
    assert_eq!(code, 0, "{stderr}");
    assert!(stdout.contains("estimated_usd_month=0.10"), "{stdout}");

    let mut utf16: Vec<u8> = one.encode_utf16().flat_map(u16::to_le_bytes).collect();
    let (code, stdout, stderr) = run(&[&write("utf16-plain.json", &utf16)]);
    assert_eq!(code, 2);
    assert!(stdout.is_empty());
    assert!(stderr.contains("not JSON"), "{stderr}");
    utf16.splice(0..0, [0xFF, 0xFE]);
    let (code, stdout, stderr) = run(&[&write("utf16-bom.json", &utf16)]);
    assert_eq!(code, 2);
    assert!(stdout.is_empty());
    assert!(stderr.contains("not UTF-8"), "{stderr}");

    // An archive rate that differs from the Standard rate shows which class a status gave.
    let prices = include_str!("../prices/ecr-storage.json").replacen(
        "\"archive_first_150_tb\": \"0.10\"",
        "\"archive_first_150_tb\": \"0.20\"",
        1,
    );
    let prices = write("prices.json", prices.as_bytes());
    for (status, want) in [
        (" archived ", "0.20"),
        ("ARCHIVED", "0.20"),
        ("\\tActive\\n", "0.10"),
    ] {
        let body = format!(
            r#"{{"imageDetails":[{{"repositoryName":"aa","imageSizeInBytes":1073741824,"imageStatus":"{status}"}}]}}"#
        );
        let (code, stdout, stderr) = run(&["--prices", &prices, &write("s.json", body.as_bytes())]);
        assert_eq!(code, 0, "{status}: {stderr}");
        assert!(
            stdout.contains(&format!("estimated_usd_month={want}")),
            "{status}: {stdout}"
        );
    }

    let max =
        r#"{"imageDetails":[{"repositoryName":"aa","imageSizeInBytes":18446744073709551615}]}"#;
    let (code, _, stderr) = run(&[&write("max.json", max.as_bytes())]);
    assert_eq!(code, 0, "{stderr}");
    let over = max.replace("551615", "551616");
    let (code, _, stderr) = run(&[&write("over.json", over.as_bytes())]);
    assert_eq!(code, 2);
    assert!(stderr.contains("invalid imageSizeInBytes"), "{stderr}");
    let (code, _, stderr) = run(&[
        &write("max.json", max.as_bytes()),
        &write("max.json", max.as_bytes()),
    ]);
    assert_eq!(code, 2);
    assert!(stderr.contains("byte total does not fit"), "{stderr}");
}

/// A price file whose Standard rate is the largest the 40-character limit allows.
fn write_extreme_standard_rate(scratch: &Scratch, rate: &str) -> String {
    let prices = scratch.path().join("extreme-prices.json");
    let text = include_str!("../prices/ecr-storage.json").replacen(
        "\"standard\": \"0.10\"",
        &format!("\"standard\": \"{rate}\""),
        1,
    );
    fs::write(&prices, text).unwrap();
    prices.to_str().unwrap().to_string()
}

// Expected values from Python `fractions.Fraction`. Red before the fix: the
// quotient by 2^30 was rounded to 28 digits before the cent was taken.
#[test]
fn a_quotient_just_under_a_half_cent_prints_the_exact_cent_in_text_and_json() {
    let scratch = Scratch::new();
    for (rate, bytes, shown) in [
        ("0.894698637813", "99231259014473123", "82684748.12"),
        ("9683085068513257837", "126819283", "1143665896371805327.83"),
        ("0.2929751363929", "86918756001175831", "23716161.39"),
    ] {
        let image = scratch.path().join("quotient.json");
        fs::write(
            &image,
            format!(
                r#"{{"imageDetails":[{{"repositoryName":"app","imageSizeInBytes":{bytes}}}]}}"#
            ),
        )
        .unwrap();
        let prices = write_extreme_standard_rate(&scratch, rate);
        let (code, stdout, stderr) = run(&["--prices", &prices, image.to_str().unwrap()]);
        assert_eq!(code, 0, "{rate}: {stderr}");
        assert!(
            stdout.contains(&format!("estimated_usd_month={shown}")),
            "{rate}: {stdout}"
        );
        let (code, json, _) = run(&["--json", "--prices", &prices, image.to_str().unwrap()]);
        assert_eq!(code, 0, "{rate}");
        assert!(
            json.contains(&format!("\"estimated_usd_month\": \"{shown}\"")),
            "{rate}: {json}"
        );
    }
}

#[test]
fn a_blank_status_a_plus_sign_and_the_price_texts_are_as_the_readme_says() {
    let scratch = Scratch::new();
    let image = scratch.path().join("blank-status.json");
    fs::write(
        &image,
        r#"{"imageDetails":[
        {"repositoryName":"aa","imageSizeInBytes":1073741824,"imageStatus":""},
        {"repositoryName":"aa","imageSizeInBytes":1073741824,"imageStatus":"   "},
        {"repositoryName":"aa","imageSizeInBytes":1073741824,"imageStatus":null}]}"#,
    )
    .unwrap();
    let (code, stdout, stderr) = run(&[image.to_str().unwrap()]);
    assert_eq!(code, 0, "{stderr}");
    assert!(stdout.contains("estimated_usd_month=0.30"), "{stdout}");
    assert!(!stdout.contains("warning"), "{stdout}");
    let (code, _, stderr) = run(&["--keep", "+5", image.to_str().unwrap()]);
    assert_eq!(code, 0, "{stderr}");
    // A price-file text is written in the text form first, then JSON-escaped.
    let prices = scratch.path().join("texts.json");
    let text = include_str!("../prices/ecr-storage.json").replacen(
        "\"region\": \"us-east-1\"",
        "\"region\": \"r\\u202e\\nx\"",
        1,
    );
    fs::write(&prices, text).unwrap();
    let (code, json, stderr) = run(&[
        "--json",
        "--prices",
        prices.to_str().unwrap(),
        image.to_str().unwrap(),
    ]);
    assert_eq!(code, 0, "{stderr}");
    assert!(
        json.contains("\"region\": \"r\\\\u{202e}\\\\nx\","),
        "{json}"
    );
}

// Red before the fix: the product was rounded to `...403.55` without a message.
#[test]
fn a_product_that_would_lose_digits_is_exact_in_text_and_json() {
    let scratch = Scratch::new();
    let image = scratch.path().join("one-gib.json");
    fs::write(
        &image,
        r#"{"imageDetails":[{"repositoryName":"aa","imageSizeInBytes":1073741824}]}"#,
    )
    .unwrap();
    for (rate, shown) in [
        ("7900827040092728403.555", "7900827040092728403.56"),
        ("751746635355857900.585", "751746635355857900.59"),
        ("7882933473361814224.005", "7882933473361814224.01"),
    ] {
        let prices = write_extreme_standard_rate(&scratch, rate);
        let (code, stdout, stderr) = run(&["--prices", &prices, image.to_str().unwrap()]);
        assert_eq!(code, 0, "{rate}: {stderr}");
        assert!(
            stdout.contains(&format!("estimated_usd_month={shown}")),
            "{rate}: {stdout}"
        );
        let (code, json, _) = run(&["--json", "--prices", &prices, image.to_str().unwrap()]);
        assert_eq!(code, 0, "{rate}");
        assert!(
            json.contains(&format!("\"estimated_usd_month\": \"{shown}\"")),
            "{rate}: {json}"
        );
    }
}

#[test]
fn a_sum_of_large_amounts_is_exact_in_text_and_json() {
    let scratch = Scratch::new();
    let image = scratch.path().join("two-classes.json");
    // One GiB Standard and one GiB Archive in one repository, and in two.
    fs::write(
        &image,
        r#"{"imageDetails":[
        {"repositoryName":"aa","imageSizeInBytes":1073741824},
        {"repositoryName":"aa","imageSizeInBytes":1073741824,"imageStatus":"ARCHIVED"}]}"#,
    )
    .unwrap();
    let two_repos = scratch.path().join("two-repos.json");
    fs::write(
        &two_repos,
        r#"{"imageDetails":[
        {"repositoryName":"aa","imageSizeInBytes":1073741824},
        {"repositoryName":"bb","imageSizeInBytes":1073741824,"imageStatus":"ARCHIVED"}]}"#,
    )
    .unwrap();
    // Expected: Python integers. The total is the rate plus the archive 0.10.
    for (rate, total) in [
        (
            "79228162514264337593543950335",
            "79228162514264337593543950335.10",
        ),
        (
            "7922816251426433759354395033",
            "7922816251426433759354395033.10",
        ),
        (
            "999999999999999999999999999",
            "999999999999999999999999999.10",
        ),
        (
            "99999999999999999999999999",
            "99999999999999999999999999.10",
        ),
    ] {
        let prices = write_extreme_standard_rate(&scratch, rate);
        for file in [&image, &two_repos] {
            let (code, stdout, stderr) = run(&["--prices", &prices, file.to_str().unwrap()]);
            assert_eq!(code, 0, "{rate}: {stderr}");
            assert!(
                stdout.contains(&format!("estimated_usd_month={total}")),
                "{stdout}"
            );
            let (code, json, stderr) =
                run(&["--json", "--prices", &prices, file.to_str().unwrap()]);
            assert_eq!(code, 0, "{rate}: {stderr}");
            assert!(
                json.contains(&format!("\"estimated_usd_month\": \"{total}\"")),
                "{json}"
            );
        }
    }
}

#[cfg(target_os = "linux")]
#[test]
fn a_read_only_standard_output_exits_2_with_a_message() {
    let scratch = Scratch::new();
    let file = scratch.path().join("read-only.txt");
    fs::write(&file, "").unwrap();
    for args in [
        &["examples/describe-images.json"][..],
        &["--json", "examples/describe-images.json"][..],
        &["--help"][..],
        &["--version"][..],
    ] {
        // Descriptor 1 opened for reading only: a write to it fails with EBADF.
        let read_only = fs::File::open(&file).unwrap();
        let out = command(args).stdout(read_only).output().unwrap();
        let stderr = String::from_utf8(out.stderr).unwrap();
        assert_eq!(out.status.code(), Some(2), "{args:?}: {stderr}");
        assert!(stderr.contains("could not write output"), "{stderr}");
        assert!(!stderr.contains("panic"), "{stderr}");
    }
    // A directory opened for reading is the same case.
    let dir = fs::File::open(scratch.path()).unwrap();
    let out = command(&["examples/describe-images.json"])
        .stdout(dir)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    // A file opened for writing stays fine, and the file gets the report.
    let writable = fs::OpenOptions::new().write(true).open(&file).unwrap();
    let out = command(&["examples/describe-images.json"])
        .stdout(writable)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0));
    assert!(fs::read_to_string(&file)
        .unwrap()
        .contains("estimated_usd_month="));
}

#[test]
fn the_documented_forms_of_a_push_time_are_accepted_and_every_other_is_refused() {
    let scratch = Scratch::new();
    let path = scratch.path().join("pushed.json");
    let path = path.to_str().unwrap();
    let write = |value: &str| {
        fs::write(
            path,
            format!(
                r#"{{"imageDetails":[{{"repositoryName":"aa","imageSizeInBytes":1,"imagePushedAt":{value}}}]}}"#
            ),
        )
        .unwrap();
    };
    for accepted in [
        "1577836800",
        "0",
        "9223372036854775807",
        "1577836800.5",
        "1.5778368e9",
        "1.5778368E+9",
        r#""2020-01-01T00:00:00Z""#,
        r#""2020-01-01t00:00:00z""#,
        r#""2020-01-01T00:00:00+00:00""#,
        r#""2020-01-01T00:00:00.123456+00:00""#,
        r#""2020-01-01T00:00:00+23:59""#,
        r#""2020-01-01T00:00:00-12:00""#,
        r#""1969-12-31T23:59:59Z""#,
        "null",
    ] {
        write(accepted);
        let (code, _, stderr) = run(&["--as-of", "2020-01-31", path]);
        assert_eq!(code, 0, "{accepted}: {stderr}");
    }
    for refused in [
        r#""1577836800""#,
        "9223372036854775808",
        "1e30",
        "-5",
        "-0",
        "true",
        r#""2020-01-01""#,
        r#""2020-01-01 00:00:00Z""#,
        r#""2020-01-01T00:00:00""#,
        r#""2020-01-01T00:00:60Z""#,
        r#""2020-01-01T25:00:00Z""#,
        r#""2020-01-01T00:00:00+0100""#,
        r#""2020-01-01T00:00:00+24:00""#,
        r#""2020-01-01T00:00:00+00:60""#,
    ] {
        write(refused);
        let (code, stdout, stderr) = run(&["--as-of", "2020-01-31", path]);
        assert_eq!(code, 2, "{refused}: {stdout}");
        assert!(stderr.contains("invalid imagePushedAt"), "{stderr}");
    }
    // The zone counts: midnight in +14:00 is 10:00 of the day before in UTC.
    write(r#""2020-01-01T00:00:00+14:00""#);
    let (code, stdout, _) = run(&["--as-of", "2020-01-31", path]);
    assert_eq!(code, 0);
    assert!(stdout.contains("age_days=31"), "{stdout}");
}

#[test]
fn a_rate_with_more_digits_than_the_decimal_type_holds_is_refused_not_rounded() {
    let scratch = Scratch::new();
    let image = scratch.path().join("one-byte.json");
    fs::write(
        &image,
        r#"{"imageDetails":[{"repositoryName":"aa","imageSizeInBytes":1}]}"#,
    )
    .unwrap();
    let image = image.to_str().unwrap();
    // One byte at 5368709.12 is exactly half a cent: 0.01.
    let prices = write_extreme_standard_rate(&scratch, "5368709.12");
    let (code, stdout, stderr) = run(&["--prices", &prices, image]);
    assert_eq!(code, 0, "{stderr}");
    assert!(stdout.contains("estimated_usd_month=0.01"), "{stdout}");
    // Trailing zeros past the 28th digit change nothing and are fine.
    let prices = write_extreme_standard_rate(&scratch, "5368709.1200000000000000000000000000000");
    let (code, stdout, stderr) = run(&["--prices", &prices, image]);
    assert_eq!(code, 0, "{stderr}");
    assert!(stdout.contains("estimated_usd_month=0.01"), "{stdout}");
    // Just under that half cent the exact amount is 0.00; rounding the rate to
    // 28 digits first would print 0.01.
    for rate in [
        "5368709.1199999999999999999999999999999",
        "0.0000000000000000000000000000000001",
    ] {
        let prices = write_extreme_standard_rate(&scratch, rate);
        let (code, stdout, stderr) = run(&["--prices", &prices, image]);
        assert_eq!(code, 2, "{rate}: {stdout}");
        assert!(stdout.is_empty(), "{stdout}");
        assert!(stderr.contains("more than 28 digits"), "{stderr}");
        assert!(!stderr.contains(rate), "{stderr}");
    }
}

const STALE_EXAMPLE: &str = "examples/describe-images.json";

fn stale_warning(days: u32, date: &str, as_of: &str) -> String {
    format!(
        "warning: prices are {days} days old (price date {date}, as of {as_of}); check current prices\n"
    )
}

#[test]
fn builtin_prices_older_than_180_days_warn_once_on_stderr_and_stdout_is_unchanged() {
    // Built-in price date is 2026-11-09: 180 days later is 2027-05-08.
    let (code, base_out, stderr) = run(&["--as-of", "2027-05-08", STALE_EXAMPLE]);
    assert_eq!(code, 0);
    assert!(stderr.is_empty(), "exactly 180 days is not old: {stderr}");
    let (code, stdout, stderr) = run(&["--as-of", "2027-05-09", STALE_EXAMPLE]);
    assert_eq!(code, 0);
    assert_eq!(stderr, stale_warning(181, "2026-11-09", "2027-05-09"));
    assert!(!stdout.contains("days old"), "{stdout}");
    assert_eq!(stdout.lines().count(), base_out.lines().count());
    let (code, json_out, stderr) = run(&["--json", "--as-of", "2027-05-09", STALE_EXAMPLE]);
    assert_eq!(code, 0);
    assert_eq!(stderr.matches("days old").count(), 1, "{stderr}");
    assert!(stderr.contains(&stale_warning(181, "2026-11-09", "2027-05-09")));
    assert!(!json_out.contains("days old"), "{json_out}");
}

#[test]
fn own_price_file_is_judged_by_its_own_source_date() {
    let scratch = Scratch::new();
    let text =
        fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("prices/ecr-storage.json"))
            .unwrap();
    let old = scratch.path().join("old.json");
    fs::write(
        &old,
        text.replace(
            "\"source_date\": \"2026-11-09\"",
            "\"source_date\": \"2026-01-01\"",
        ),
    )
    .unwrap();
    let old = old.to_str().unwrap();
    let (code, _, stderr) = run(&["--prices", old, "--as-of", "2026-06-30", STALE_EXAMPLE]);
    assert_eq!(code, 0);
    assert!(stderr.is_empty(), "180 days exactly: {stderr}");
    let (code, stdout, stderr) = run(&["--prices", old, "--as-of", "2026-07-01", STALE_EXAMPLE]);
    assert_eq!(code, 0);
    assert_eq!(stderr, stale_warning(181, "2026-01-01", "2026-07-01"));
    assert!(!stdout.contains("days old"));
    // A date that is not YYYY-MM-DD gives no warning and no error.
    let free = scratch.path().join("free.json");
    fs::write(
        &free,
        text.replace(
            "\"source_date\": \"2026-11-09\"",
            "\"source_date\": \"last spring\"",
        ),
    )
    .unwrap();
    let (code, _, stderr) = run(&[
        "--prices",
        free.to_str().unwrap(),
        "--as-of",
        "2030-01-01",
        STALE_EXAMPLE,
    ]);
    assert_eq!(code, 0);
    assert!(stderr.is_empty(), "{stderr}");
}

#[test]
fn a_price_date_in_the_future_or_a_failed_run_gives_no_age_warning() {
    let (code, _, stderr) = run(&["--as-of", "2026-01-01", STALE_EXAMPLE]);
    assert_eq!(code, 0);
    assert!(stderr.is_empty(), "{stderr}");
    let (code, _, stderr) = run(&["--as-of", "2030-01-01", "no-such-file.json"]);
    assert_eq!(code, 2);
    assert!(!stderr.contains("days old"), "{stderr}");
}

#[test]
fn without_as_of_the_price_age_is_taken_from_today() {
    // A price file dated 2000-01-01 is over 180 days old on any real clock.
    let scratch = Scratch::new();
    let text =
        fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("prices/ecr-storage.json"))
            .unwrap();
    let file = scratch.path().join("p.json");
    fs::write(
        &file,
        text.replace(
            "\"source_date\": \"2026-11-09\"",
            "\"source_date\": \"2000-01-01\"",
        ),
    )
    .unwrap();
    let (code, stdout, stderr) = run(&["--prices", file.to_str().unwrap(), STALE_EXAMPLE]);
    assert_eq!(code, 0);
    assert!(stderr.starts_with("warning: prices are "), "{stderr}");
    assert!(
        stderr.contains("days old (price date 2000-01-01, as of "),
        "{stderr}"
    );
    assert_eq!(stderr.lines().count(), 1);
    assert!(!stdout.contains("days old"));
}

/// A debug build shows the commit that git reported when it was built; a
/// release build, or a build without git (an unpacked crate), shows only the
/// crate version.
#[test]
fn version_shows_the_commit_in_a_debug_build_only_when_git_names_one() {
    let (code, stdout, stderr) = run(&["--version"]);
    assert_eq!(code, 0);
    assert!(stderr.is_empty(), "{stderr}");
    let line = stdout.trim_end();
    let git_head = std::process::Command::new("git")
        .args(["rev-parse", "--verify", "HEAD"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .ok()
        .filter(|out| out.status.success())
        .and_then(|out| String::from_utf8(out.stdout).ok())
        .map(|text| text.trim().to_string());
    match (cfg!(debug_assertions), git_head) {
        (true, Some(sha)) if sha.len() == 40 => {
            // The binary was built at some commit; it is a full sha, not HEAD at test time.
            let rest = line
                .strip_prefix("ecr-bill 1.0.0 (")
                .and_then(|r| r.strip_suffix(')'))
                .unwrap_or_else(|| panic!("no commit in {line:?}"));
            assert_eq!(rest.len(), 40, "{line:?}");
            assert!(rest.bytes().all(|b| b.is_ascii_hexdigit()), "{line:?}");
        }
        (false, _) => assert_eq!(line, "ecr-bill 1.0.0"),
        _ => assert!(line.starts_with("ecr-bill 1.0.0"), "{line:?}"),
    }
}
