//! Records the commit of this tree for debug builds, so `--version` can show
//! it. It never invents one: no git, a failed command or a value that is not
//! a full 40-digit hex sha leaves the version as the crate version. Release
//! builds do not receive the commit.

use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo:rustc-check-cfg=cfg(ecr_bill_git_sha)");
    println!("cargo:rerun-if-changed=build.rs");
    if let Some(head) = git_path("HEAD") {
        println!("cargo:rerun-if-changed={}", head.display());
        if let Ok(text) = std::fs::read_to_string(&head) {
            if let Some(name) = text.trim().strip_prefix("ref:") {
                if let Some(path) = git_path(name.trim()) {
                    println!("cargo:rerun-if-changed={}", path.display());
                }
            }
        }
    }
    if std::env::var("PROFILE").unwrap_or_default() != "debug" {
        return;
    }
    if let Some(sha) = git_output(&["rev-parse", "--verify", "HEAD"]) {
        if sha.len() == 40 && sha.bytes().all(|b| b.is_ascii_hexdigit()) {
            println!("cargo:rustc-cfg=ecr_bill_git_sha");
            println!("cargo:rustc-env=ECR_BILL_GIT_SHA={sha}");
        }
    }
}

fn git_output(args: &[&str]) -> Option<String> {
    let dir = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR")?);
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8(out.stdout).ok()?.trim().to_string())
}

fn git_path(name: &str) -> Option<PathBuf> {
    let text = git_output(&["rev-parse", "--git-path", name])?;
    let path = PathBuf::from(text);
    let path = if path.is_absolute() {
        path
    } else {
        PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR")?).join(path)
    };
    path.is_file().then_some(path)
}
