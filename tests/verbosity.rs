//! `--quiet` and `--verbose`. Quiet keeps the result and the errors and drops
//! the warnings on standard error. Verbose adds progress lines on standard
//! error. Neither changes standard output or an exit code, and the two
//! together are a usage error.

use std::process::Command;

const EXAMPLE: &str = "examples/describe-images.json";
const NO_PUSH: &str = "tests/fixtures/no-push-time.json";

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
fn help_lists_quiet_and_verbose() {
    let (code, stdout, _) = run(&["--help"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("--quiet"), "{stdout}");
    assert!(stdout.contains("--verbose"), "{stdout}");
}

#[test]
fn quiet_with_verbose_is_a_usage_error_with_exit_2_and_no_output() {
    for args in [
        ["--quiet", "--verbose", EXAMPLE],
        ["--verbose", "--quiet", EXAMPLE],
    ] {
        let (code, stdout, stderr) = run(&args);
        assert_eq!(code, 2, "{stderr}");
        assert!(stdout.is_empty(), "{stdout}");
        assert!(
            stderr.contains("--quiet") && stderr.contains("--verbose"),
            "{stderr}"
        );
        assert_eq!(
            stderr.lines().next().map(|l| l.starts_with("error:")),
            Some(true)
        );
    }
}

#[test]
fn quiet_drops_the_price_age_warning_and_keeps_standard_output() {
    let args = ["--as-of", "2027-06-01", EXAMPLE];
    let (code, stdout, stderr) = run(&args);
    assert_eq!(code, 0);
    assert!(stderr.starts_with("warning: prices are "), "{stderr}");
    let (qcode, qstdout, qstderr) = run(&["--quiet", "--as-of", "2027-06-01", EXAMPLE]);
    assert_eq!(qcode, 0);
    assert_eq!(qstdout, stdout);
    assert!(qstderr.is_empty(), "{qstderr}");
}

#[test]
fn quiet_drops_the_json_warnings_on_standard_error_and_keeps_them_in_the_json() {
    let args = ["--json", "--as-of", "2026-11-20", NO_PUSH];
    let (code, stdout, stderr) = run(&args);
    assert_eq!(code, 0);
    assert!(stderr.contains("warning:"), "{stderr}");
    let (qcode, qstdout, qstderr) = run(&["--quiet", "--json", "--as-of", "2026-11-20", NO_PUSH]);
    assert_eq!(qcode, 0);
    assert_eq!(qstdout, stdout);
    assert!(qstdout.contains("\"warnings\": ["), "{qstdout}");
    assert!(qstderr.is_empty(), "{qstderr}");
}

#[test]
fn quiet_still_reports_errors_and_exits_2() {
    let (code, stdout, stderr) = run(&["--quiet", "tests/fixtures/does-not-exist.json"]);
    assert_eq!(code, 2);
    assert!(stdout.is_empty());
    assert!(stderr.starts_with("ecr-bill: "), "{stderr}");
}

#[test]
fn verbose_adds_progress_on_standard_error_and_nothing_to_standard_output() {
    let (_, plain, _) = run(&["--as-of", "2026-11-20", EXAMPLE, NO_PUSH]);
    let (code, stdout, stderr) = run(&["--verbose", "--as-of", "2026-11-20", EXAMPLE, NO_PUSH]);
    assert_eq!(code, 0);
    assert_eq!(stdout, plain);
    assert!(stderr.contains("ecr-bill: reading 2 files\n"), "{stderr}");
    assert!(
        stderr.contains(&format!("ecr-bill: reading {EXAMPLE}\n")),
        "{stderr}"
    );
    assert!(
        stderr.contains(&format!("ecr-bill: reading {NO_PUSH}\n")),
        "{stderr}"
    );
    assert!(stderr.ends_with("ecr-bill: done\n"), "{stderr}");
    // Without it nothing is added.
    let (_, _, plain_err) = run(&["--as-of", "2026-11-20", EXAMPLE, NO_PUSH]);
    assert!(!plain_err.contains("reading"), "{plain_err}");
}

#[test]
fn verbose_keeps_the_exit_code_of_a_failed_run_and_escapes_file_names() {
    let (code, stdout, stderr) = run(&["--verbose", "bad\u{1b}[31m\nname.json"]);
    assert_eq!(code, 2);
    assert!(stdout.is_empty());
    assert!(!stderr.contains('\u{1b}'), "{stderr:?}");
    assert!(
        stderr.contains("reading bad\\u{1b}[31m\\nname.json\n"),
        "{stderr:?}"
    );
    assert!(!stderr.contains("done"), "{stderr:?}");
}

#[test]
fn verbose_with_json_keeps_standard_output_a_json_object() {
    let (code, stdout, stderr) = run(&["--verbose", "--json", EXAMPLE]);
    assert_eq!(code, 0);
    assert!(stdout.starts_with('{'));
    assert!(stderr.contains("reading"), "{stderr}");
}
