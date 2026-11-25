use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::bill::{bill_documents_labeled, BillOptions};
use crate::error::{show_text, Error};
use crate::price::{prices_builtin, Prices};
use crate::report::{render, render_markdown};
use crate::MAX_FILE_BYTES;

/// What to price, and how to print it.
#[derive(Debug, Clone)]
pub struct Options {
    pub prices: Prices,
    pub as_of: Option<String>,
    pub older_than_days: u32,
    pub untagged_days: u32,
    pub keep: u32,
    pub json: bool,
    pub markdown: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            prices: prices_builtin(),
            as_of: None,
            older_than_days: 30,
            untagged_days: 14,
            keep: 10,
            json: false,
            markdown: false,
        }
    }
}

/// Rendered report. `warnings` is also included in `stdout` for text output.
#[derive(Debug)]
pub struct Output {
    pub stdout: String,
    pub warnings: Vec<String>,
}

pub fn execute_paths(paths: &[PathBuf], options: &Options) -> Result<Output, Error> {
    if paths.is_empty() {
        return Err(Error::new("no inventory files"));
    }
    let mut docs = Vec::with_capacity(paths.len());
    for path in paths {
        docs.push(read_text_file(path)?);
    }
    let refs: Vec<&str> = docs.iter().map(String::as_str).collect();
    let bill_options = BillOptions {
        as_of: options.as_of.clone(),
        older_than_days: options.older_than_days,
        untagged_days: options.untagged_days,
        keep: options.keep,
    };
    let labels: Vec<String> = paths.iter().map(|p| display_path(p)).collect();
    let bill = bill_documents_labeled(&refs, &labels, &options.prices, &bill_options)?;
    let warnings = bill.warnings.clone();
    let stdout = if options.markdown {
        render_markdown(&bill)
    } else {
        render(&bill, options.json)
    };
    Ok(Output { stdout, warnings })
}

/// Path text for messages and labels. Valid UTF-8 equals `path.display()`;
/// anything else is the `escape_ascii` form of the raw bytes, never U+FFFD.
pub fn display_path(path: &Path) -> String {
    match path.to_str() {
        Some(text) => show_text(text),
        None => raw_bytes(path).escape_ascii().to_string(),
    }
}

#[cfg(unix)]
fn raw_bytes(path: &Path) -> Vec<u8> {
    use std::os::unix::ffi::OsStrExt;
    path.as_os_str().as_bytes().to_vec()
}

#[cfg(not(unix))]
fn raw_bytes(path: &Path) -> Vec<u8> {
    path.to_string_lossy().into_owned().into_bytes()
}

/// Read a UTF-8 file up to [`MAX_FILE_BYTES`]. A symlink to a file is followed.
/// Only regular files are read, and never past the length the metadata gave.
/// On Unix the open does not wait, so a fifo swapped in after the check is
/// refused instead of blocking.
pub fn read_text_file(path: &Path) -> Result<String, Error> {
    let shown = display_path(path);
    let meta = fs::metadata(path).map_err(|err| Error::new(format!("{shown}: {err}")))?;
    check_regular(&meta, &shown)?;
    let file = open_for_read(path).map_err(|err| Error::new(format!("{shown}: {err}")))?;
    read_opened(file, &shown)
}

/// Open after the check above. On Unix `O_NONBLOCK` is set so a fifo with no
/// writer, swapped in after that check, returns at once. A regular file ignores
/// the flag. The opened inode is checked again before any byte is read.
fn open_for_read(path: &Path) -> std::io::Result<File> {
    if let Some(hook) = BEFORE_OPEN.with(|cell| cell.get()) {
        hook(path);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(O_NONBLOCK)
            .open(path)
    }
    #[cfg(not(unix))]
    File::open(path)
}

// Linux and Android use octal 04000. The BSDs and macOS use 0x4.
#[cfg(any(target_os = "linux", target_os = "android"))]
const O_NONBLOCK: i32 = 0o4000;
#[cfg(any(
    target_os = "macos",
    target_os = "ios",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly",
))]
const O_NONBLOCK: i32 = 0x4;

thread_local! {
    static BEFORE_OPEN: std::cell::Cell<Option<fn(&Path)>> = const { std::cell::Cell::new(None) };
}

/// Runs after the metadata check and before the open. Unset on every normal
/// call. Hidden: it exists so a test can put a fifo in that window.
#[doc(hidden)]
pub fn set_before_open_for_test(hook: Option<fn(&Path)>) {
    BEFORE_OPEN.with(|cell| cell.set(hook));
}

/// Read what was opened. The path may have changed since the check before the
/// `open`, so the file is looked at again, and read no further than its length.
fn read_opened(mut file: File, shown: &str) -> Result<String, Error> {
    let meta = file
        .metadata()
        .map_err(|err| Error::new(format!("{shown}: {err}")))?;
    check_regular(&meta, shown)?;
    let bytes = read_capped((&mut file).take(meta.len()), MAX_FILE_BYTES)
        .map_err(|err| Error::new(format!("{shown}: {err}")))?;
    let text = String::from_utf8(bytes).map_err(|_| Error::new(format!("{shown}: not UTF-8")))?;
    Ok(text)
}

/// A regular file that is not above the size limit.
fn check_regular(meta: &fs::Metadata, shown: &str) -> Result<(), Error> {
    if meta.is_dir() {
        return Err(Error::new(format!("{shown}: is a directory")));
    }
    if !meta.is_file() {
        return Err(Error::new(format!("{shown}: not a file")));
    }
    if meta.len() > MAX_FILE_BYTES {
        return Err(Error::new(format!(
            "{shown}: file is larger than {MAX_FILE_BYTES} bytes"
        )));
    }
    Ok(())
}

pub(crate) fn read_capped<R: Read>(mut reader: R, cap: u64) -> Result<Vec<u8>, Error> {
    let mut out = Vec::new();
    let mut buf = [0u8; 64 * 1024];
    let mut total = 0u64;
    loop {
        let read = reader
            .read(&mut buf)
            .map_err(|_| Error::new("read failed"))?;
        if read == 0 {
            break;
        }
        total = total.saturating_add(read as u64);
        if total > cap {
            return Err(Error::new(format!("file is larger than {cap} bytes")));
        }
        out.extend_from_slice(&buf[..read]);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    #[cfg(unix)]
    use super::read_opened;
    use super::{execute_paths, read_capped, read_text_file};
    use crate::Options;
    use crate::MAX_FILE_BYTES;

    // A path that is a regular file at the first check and something else at
    // the `open` is read from what was opened: a directory is refused there.
    #[cfg(unix)]
    #[test]
    fn what_was_opened_is_checked_again() {
        let dir = std::fs::File::open(std::env::temp_dir()).unwrap();
        let err = read_opened(dir, "swapped").unwrap_err();
        assert_eq!(err.message(), "swapped: is a directory");
    }

    #[test]
    fn file_larger_than_268435456_bytes() {
        assert_eq!(MAX_FILE_BYTES, 268_435_456);
        let err = read_capped(&mut &b"abcdef"[..], 4).unwrap_err();
        assert!(err.message().contains("file is larger than 4 bytes"));
        assert!(!err.message().contains("abcdef"));
    }

    #[test]
    fn directory_is_rejected_and_symlink_is_followed() {
        let root = std::env::temp_dir().join(format!("ecr-bill-{}-sym", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let options = Options::default();
        let err = execute_paths(std::slice::from_ref(&root), &options).unwrap_err();
        assert!(err.message().contains("is a directory"));
        #[cfg(unix)]
        {
            let file = root.join("images.json");
            std::fs::write(
                &file,
                r#"{"imageDetails":[{"repositoryName":"app","imageSizeInBytes":1073741824}]}"#,
            )
            .unwrap();
            let link = root.join("link.json");
            std::os::unix::fs::symlink(&file, &link).unwrap();
            let out = execute_paths(std::slice::from_ref(&link), &options).unwrap();
            assert!(out.stdout.contains("estimated_usd_month=0.10"));
        }
        let _ = std::fs::remove_dir_all(&root);
    }

    struct Failing;

    impl std::io::Read for Failing {
        fn read(&mut self, _buf: &mut [u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("secret detail"))
        }
    }

    #[test]
    fn a_failing_reader_says_read_failed_and_hides_the_cause() {
        let err = read_capped(Failing, 10).unwrap_err();
        assert_eq!(err.message(), "read failed");
    }

    #[test]
    fn file_read_errors_are_prefixed_with_the_path() {
        let root = std::env::temp_dir().join(format!("ecr-bill-{}-read", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();

        let missing = root.join("missing.json");
        let err = read_text_file(&missing).unwrap_err();
        assert!(
            err.message()
                .starts_with(&format!("{}: ", missing.display())),
            "{}",
            err.message()
        );

        let bad = root.join("bad.json");
        std::fs::write(&bad, [0xff, 0xfe, 0x41]).unwrap();
        assert_eq!(
            read_text_file(&bad).unwrap_err().message(),
            format!("{}: not UTF-8", bad.display())
        );

        let big = root.join("big.json");
        let file = std::fs::File::create(&big).unwrap();
        file.set_len(MAX_FILE_BYTES + 1).unwrap();
        drop(file);
        assert_eq!(
            read_text_file(&big).unwrap_err().message(),
            format!(
                "{}: file is larger than {MAX_FILE_BYTES} bytes",
                big.display()
            )
        );
        let _ = std::fs::remove_dir_all(&root);
    }
}
