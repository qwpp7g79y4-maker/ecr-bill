//! SIGINT (Ctrl-C): exit code 130, a one-line message on standard error, and
//! nothing on standard output. Unix only: Windows has no such signal here.
#![cfg(unix)]

use std::fs;
use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// A valid file that takes several seconds to read: one long array that the
/// parser must walk. The path does not depend on the process id.
fn slow_input() -> std::path::PathBuf {
    let dir = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
    let path = dir.join("interrupt-slow-input.json");
    if fs::metadata(&path).map(|m| m.len()).unwrap_or(0) < 60_000_000 {
        let mut file = fs::File::create(&path).unwrap();
        file.write_all(b"{\"imageDetails\":[],\"x\":[").unwrap();
        let chunk = "[0],".repeat(1 << 16);
        for _ in 0..(60_000_000 / chunk.len() + 1) {
            file.write_all(chunk.as_bytes()).unwrap();
        }
        file.write_all(b"[0]]}").unwrap();
    }
    path
}

fn spawn(args: &[&std::ffi::OsStr]) -> std::process::Child {
    Command::new(env!("CARGO_BIN_EXE_ecr-bill"))
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .env("NO_COLOR", "1")
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap()
}

fn interrupt(child: &std::process::Child) {
    let status = Command::new("kill")
        .args(["-INT", &child.id().to_string()])
        .status()
        .unwrap();
    assert!(status.success());
}

#[test]
fn a_sigint_while_reading_exits_130_with_a_message_and_no_output() {
    let input = slow_input();
    let mut child = spawn(&[input.as_os_str()]);
    std::thread::sleep(Duration::from_millis(600));
    interrupt(&child);
    let started = Instant::now();
    let status = child.wait().unwrap();
    let mut stdout = String::new();
    let mut stderr = String::new();
    child
        .stdout
        .take()
        .unwrap()
        .read_to_string(&mut stdout)
        .unwrap();
    child
        .stderr
        .take()
        .unwrap()
        .read_to_string(&mut stderr)
        .unwrap();
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "it did not stop at once"
    );
    assert_eq!(status.code(), Some(130), "stderr: {stderr}");
    assert!(stdout.is_empty(), "stdout: {stdout}");
    assert_eq!(stderr, "ecr-bill: interrupted by SIGINT\n");
    assert!(!stderr.contains("panicked"));
}

#[test]
fn a_run_that_is_not_interrupted_is_unchanged() {
    let out = Command::new(env!("CARGO_BIN_EXE_ecr-bill"))
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .arg("examples/describe-images.json")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0));
    assert!(!out.stdout.is_empty());
}
