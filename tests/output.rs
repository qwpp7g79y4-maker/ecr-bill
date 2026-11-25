//! `--output FILE`: the report goes to a file, atomically, never through a
//! symbolic link, and a failure is exit 2 with a plain message.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const EXAMPLE: &str = "examples/describe-images.json";

/// A fresh directory under the target directory, named by the test.
fn scratch(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("output")
        .join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// The price date of the built-in prices: a fixed day, so no test reads the clock.
const AS_OF: &str = "2026-11-09";

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

fn names(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    v.sort();
    v
}

#[test]
fn help_lists_output() {
    let (code, stdout, _) = run(&["--help"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("--output"), "{stdout}");
}

#[test]
fn the_file_holds_exactly_what_standard_output_would_and_standard_output_is_empty() {
    let dir = scratch("same-bytes");
    for (i, format) in [None, Some("--json"), Some("--md")].into_iter().enumerate() {
        let mut args = vec!["--as-of", "2026-11-20", EXAMPLE];
        if let Some(f) = format {
            args.insert(0, f);
        }
        let (code, expected, _) = run(&args);
        assert_eq!(code, 0);
        let path = dir.join(format!("report-{i}.out"));
        let path_text = path.to_str().unwrap();
        let mut with_output = vec!["--output", path_text];
        with_output.extend(&args);
        let (code, stdout, stderr) = run(&with_output);
        assert_eq!(code, 0, "{stderr}");
        assert!(stdout.is_empty(), "{stdout}");
        assert!(stderr.is_empty(), "{stderr}");
        assert_eq!(fs::read_to_string(&path).unwrap(), expected);
    }
    // Only the three reports, no temporary file.
    assert_eq!(names(&dir).len(), 3, "{:?}", names(&dir));
}

#[test]
fn an_existing_file_is_replaced_whole_and_no_temporary_file_is_left() {
    let dir = scratch("replace");
    let path = dir.join("report.txt");
    fs::write(&path, "x".repeat(100_000)).unwrap();
    let (code, expected, _) = run(&[EXAMPLE]);
    assert_eq!(code, 0);
    let (code, _, stderr) = run(&["--output", path.to_str().unwrap(), EXAMPLE]);
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(fs::read_to_string(&path).unwrap(), expected);
    assert_eq!(names(&dir), vec!["report.txt".to_string()]);
}

#[cfg(unix)]
#[test]
fn a_symbolic_link_is_refused_and_not_followed() {
    let dir = scratch("symlink");
    let target = dir.join("target.txt");
    fs::write(&target, "keep me").unwrap();
    let link = dir.join("link.txt");
    std::os::unix::fs::symlink(&target, &link).unwrap();
    let (code, stdout, stderr) = run(&[
        "--as-of",
        AS_OF,
        "--output",
        link.to_str().unwrap(),
        EXAMPLE,
    ]);
    assert_eq!(code, 2);
    assert!(stdout.is_empty());
    assert!(stderr.starts_with("ecr-bill: could not write "), "{stderr}");
    assert!(stderr.contains("symbolic link"), "{stderr}");
    assert_eq!(fs::read_to_string(&target).unwrap(), "keep me");
    assert!(fs::symlink_metadata(&link)
        .unwrap()
        .file_type()
        .is_symlink());
    assert_eq!(names(&dir).len(), 2);
    // A link to a missing file is refused too, and nothing is created behind it.
    let dangling = dir.join("dangling.txt");
    std::os::unix::fs::symlink(dir.join("not-there.txt"), &dangling).unwrap();
    let (code, _, stderr) = run(&[
        "--as-of",
        AS_OF,
        "--output",
        dangling.to_str().unwrap(),
        EXAMPLE,
    ]);
    assert_eq!(code, 2, "{stderr}");
    assert!(!dir.join("not-there.txt").exists());
}

#[test]
fn a_directory_and_a_missing_parent_are_errors_and_nothing_is_created() {
    let dir = scratch("bad-target");
    let (code, stdout, stderr) =
        run(&["--as-of", AS_OF, "--output", dir.to_str().unwrap(), EXAMPLE]);
    assert_eq!(code, 2);
    assert!(stdout.is_empty());
    assert!(stderr.contains("not a regular file"), "{stderr}");
    let missing = dir.join("no-such-dir").join("report.txt");
    let (code, stdout, stderr) = run(&[
        "--as-of",
        AS_OF,
        "--output",
        missing.to_str().unwrap(),
        EXAMPLE,
    ]);
    assert_eq!(code, 2, "{stderr}");
    assert!(stdout.is_empty());
    assert!(stderr.starts_with("ecr-bill: could not write "), "{stderr}");
    assert!(!stderr.contains("panicked"));
    assert!(!dir.join("no-such-dir").exists());
    assert!(names(&dir).is_empty());
}

#[cfg(unix)]
#[test]
fn a_directory_that_cannot_be_written_is_exit_2_with_a_message() {
    use std::os::unix::fs::PermissionsExt;
    let dir = scratch("read-only");
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o555)).unwrap();
    let writable = fs::File::create(dir.join("probe")).is_ok();
    if !writable {
        let path = dir.join("report.txt");
        let (code, stdout, stderr) = run(&[
            "--as-of",
            AS_OF,
            "--output",
            path.to_str().unwrap(),
            EXAMPLE,
        ]);
        assert_eq!(code, 2, "{stderr}");
        assert!(stdout.is_empty());
        assert!(stderr.starts_with("ecr-bill: could not write "), "{stderr}");
        assert!(!stderr.contains("panicked"));
    }
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o755)).unwrap();
}

#[test]
fn a_failed_run_leaves_an_existing_output_file_as_it_was() {
    let dir = scratch("failed-run");
    let path = dir.join("report.txt");
    fs::write(&path, "previous report\n").unwrap();
    let (code, stdout, stderr) = run(&[
        "--output",
        path.to_str().unwrap(),
        "tests/fixtures/does-not-exist.json",
    ]);
    assert_eq!(code, 2);
    assert!(stdout.is_empty());
    assert!(stderr.starts_with("ecr-bill: "), "{stderr}");
    assert_eq!(fs::read_to_string(&path).unwrap(), "previous report\n");
    assert_eq!(names(&dir), vec!["report.txt".to_string()]);
    let fresh = dir.join("fresh.txt");
    let (code, _, _) = run(&[
        "--output",
        fresh.to_str().unwrap(),
        "tests/fixtures/does-not-exist.json",
    ]);
    assert_eq!(code, 2);
    assert!(!fresh.exists());
}

#[test]
fn the_message_escapes_the_file_name_and_verbose_names_the_file() {
    let dir = scratch("escapes");
    let path = dir.join("no-dir\u{1b}[31m").join("r.txt");
    let (code, _, stderr) = run(&["--output", path.to_str().unwrap(), EXAMPLE]);
    assert_eq!(code, 2);
    assert!(!stderr.contains('\u{1b}'), "{stderr:?}");
    assert!(stderr.contains("\\u{1b}[31m"), "{stderr:?}");
    let ok = dir.join("ok.txt");
    let (code, stdout, stderr) = run(&["--verbose", "--output", ok.to_str().unwrap(), EXAMPLE]);
    assert_eq!(code, 0);
    assert!(stdout.is_empty());
    assert!(stderr.contains("ecr-bill: writing "), "{stderr}");
    assert!(stderr.ends_with("ecr-bill: done\n"), "{stderr}");
}

#[cfg(unix)]
#[test]
fn a_sigint_before_the_report_is_finished_keeps_the_existing_output_file() {
    use std::io::Write;
    use std::process::Stdio;
    use std::time::Duration;
    let dir = scratch("sigint");
    let path = dir.join("report.txt");
    fs::write(&path, "previous report\n").unwrap();
    let input = dir.join("slow.json");
    let mut file = fs::File::create(&input).unwrap();
    file.write_all(b"{\"imageDetails\":[],\"x\":[").unwrap();
    let chunk = "[0],".repeat(1 << 16);
    for _ in 0..(60_000_000 / chunk.len() + 1) {
        file.write_all(chunk.as_bytes()).unwrap();
    }
    file.write_all(b"[0]]}").unwrap();
    drop(file);
    let child = Command::new(env!("CARGO_BIN_EXE_ecr-bill"))
        .args(["--output", path.to_str().unwrap(), input.to_str().unwrap()])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    std::thread::sleep(Duration::from_millis(600));
    let status = Command::new("kill")
        .args(["-INT", &child.id().to_string()])
        .status()
        .unwrap();
    assert!(status.success());
    let out = child.wait_with_output().unwrap();
    assert_eq!(out.status.code(), Some(130));
    assert!(out.stdout.is_empty());
    assert_eq!(fs::read_to_string(&path).unwrap(), "previous report\n");
    assert_eq!(
        names(&dir),
        vec!["report.txt".to_string(), "slow.json".to_string()]
    );
}

#[cfg(unix)]
#[test]
fn a_file_name_of_245_characters_can_be_written() {
    let dir = scratch("long-name");
    let path = dir.join("a".repeat(245));
    let (code, expected, _) = run(&["--as-of", AS_OF, EXAMPLE]);
    assert_eq!(code, 0);
    let (code, _, stderr) = run(&[
        "--as-of",
        AS_OF,
        "--output",
        path.to_str().unwrap(),
        EXAMPLE,
    ]);
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(fs::read_to_string(&path).unwrap(), expected);
    assert_eq!(names(&dir), vec!["a".repeat(245)]);
}

#[cfg(unix)]
#[test]
fn an_existing_file_keeps_its_permissions() {
    use std::os::unix::fs::PermissionsExt;
    for mode in [0o600, 0o640, 0o644, 0o664] {
        let dir = scratch(&format!("mode-{mode:o}"));
        let path = dir.join("report.txt");
        fs::write(&path, "old").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(mode)).unwrap();
        let (code, _, stderr) = run(&[
            "--as-of",
            AS_OF,
            "--output",
            path.to_str().unwrap(),
            EXAMPLE,
        ]);
        assert_eq!(code, 0, "{stderr}");
        let got = fs::metadata(&path).unwrap().permissions().mode() & 0o7777;
        assert_eq!(got, mode, "mode {mode:o} became {got:o}");
        assert_ne!(fs::read_to_string(&path).unwrap(), "old");
        assert_eq!(names(&dir), vec!["report.txt".to_string()]);
    }
}
