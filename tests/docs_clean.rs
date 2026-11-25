//! The published files carry no local path, no name of a working file, no
//! prefix of a build wrapper and no name of an internal tool or a person.
//!
//! Every word is written in pieces, so this file does not contain them either.
//! A word matches in any letter case, and a hit names its file and line.

use std::fs;
use std::path::{Path, PathBuf};

const FORBIDDEN: [&str; 27] = [
    // Local paths on Linux, macOS, Windows and in a shell.
    concat!("/me", "dia/"),
    concat!("/ho", "me/"),
    concat!("/Us", "ers/"),
    concat!("/t", "mp/"),
    concat!("~", "/"),
    concat!("C:", "\\"),
    // Options of a build wrapper.
    concat!("--max", "-gb"),
    concat!("--na", "am"),
    // Working files and the helpers around them.
    concat!("task", ".md"),
    concat!("blueprint", ".md"),
    concat!("geheugen", "wachter"),
    concat!("bouw", "-tmp"),
    concat!("brug", "-prompts"),
    concat!("review", "-repo"),
    concat!("open", "bot"),
    // Names of AI systems and of people.
    concat!("gr", "ok"),
    concat!("cla", "ude"),
    concat!("anthr", "opic"),
    concat!("son", "net"),
    concat!("op", "us"),
    concat!("hai", "ku"),
    concat!("fa", "ble"),
    concat!("gem", "ini"),
    concat!("co", "pilot"),
    concat!("g", "pt"),
    concat!("co-", "authored"),
    concat!("pepi", "jn"),
];

fn collect(path: &Path, out: &mut Vec<PathBuf>) {
    if path.is_dir() {
        let mut entries: Vec<PathBuf> = fs::read_dir(path)
            .unwrap_or_else(|err| panic!("read dir {}: {err}", path.display()))
            .map(|entry| entry.expect("entry").path())
            .collect();
        entries.sort();
        for entry in entries {
            collect(&entry, out);
        }
    } else {
        out.push(path.to_path_buf());
    }
}

/// The words found in `text`, each with its 1-based line number.
fn hits(text: &str) -> Vec<(usize, &'static str)> {
    let mut found = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let lower = line.to_lowercase();
        for word in FORBIDDEN {
            if lower.contains(&word.to_lowercase()) {
                found.push((index + 1, word));
            }
        }
    }
    found
}

fn problems_in(files: &[PathBuf]) -> Vec<String> {
    let mut problems = Vec::new();
    for file in files {
        let text =
            fs::read_to_string(file).unwrap_or_else(|err| panic!("read {}: {err}", file.display()));
        for (line, word) in hits(&text) {
            problems.push(format!("{}:{line}: {word}", file.display()));
        }
    }
    problems
}

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn the_documents_and_the_man_page_carry_no_internal_text() {
    let mut files = Vec::new();
    for name in [
        "README.md",
        "QUALITY.md",
        "CHANGELOG.md",
        "SECURITY.md",
        "THIRD-PARTY-LICENSES.html",
    ] {
        files.push(root().join(name));
    }
    collect(&root().join("man"), &mut files);
    assert!(files.len() >= 5, "man/ holds no page: {files:?}");
    let problems = problems_in(&files);
    assert!(
        problems.is_empty(),
        "internal text in the documents:\n{}",
        problems.join("\n")
    );
}

#[test]
fn the_package_files_workflows_sources_and_tests_carry_no_internal_text_either() {
    let mut files = Vec::new();
    // `.gitignore` is not in the package and has to name working files.
    for name in [
        "Cargo.toml",
        "Cargo.lock",
        "deny.toml",
        ".gitattributes",
        "LICENSE-MIT",
        "LICENSE-APACHE",
    ] {
        files.push(root().join(name));
    }
    // `.github` is in the repository, not in the package: read it when it is there.
    for dir in [".github", "src", "tests", "examples", "prices", "scripts"] {
        let path = root().join(dir);
        if dir != ".github" || path.is_dir() {
            collect(&path, &mut files);
        }
    }
    assert!(files.len() >= 20, "too few files found: {files:?}");
    let problems = problems_in(&files);
    assert!(
        problems.is_empty(),
        "internal text in the package files:\n{}",
        problems.join("\n")
    );
}

#[test]
fn the_check_finds_each_forbidden_word_in_any_letter_case() {
    for word in FORBIDDEN {
        let text = format!("fine line\nbefore {word} after\n");
        assert_eq!(hits(&text), vec![(2, word)], "{word}");
        let upper = format!("fine line\nbefore {} after\n", word.to_uppercase());
        assert_eq!(hits(&upper), vec![(2, word)], "{word} in capitals");
    }
    assert!(hits("plain text, a `cargo test` command, ./target/debug").is_empty());
}

#[test]
fn this_file_spells_out_none_of_its_words() {
    let source = fs::read_to_string(root().join("tests/docs_clean.rs")).unwrap();
    assert!(hits(&source).is_empty(), "{:?}", hits(&source));
}
