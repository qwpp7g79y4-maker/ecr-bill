use std::io::{stdout, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::Parser;

#[allow(unsafe_code)]
mod interrupt;
#[allow(unsafe_code)]
mod stdout_probe;

use ecr_bill::{
    display_path, execute_paths, load_prices_str, price_age_warning, read_text_file, show_lines,
    Error, Options,
};

// `-V` and `--version` print this. A debug build appends the commit that
// git reported at compile time; a release build, or a build without git,
// prints the crate version only.
#[cfg(all(debug_assertions, ecr_bill_git_sha))]
const VERSION: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    " (",
    env!("ECR_BILL_GIT_SHA"),
    ")"
);

#[cfg(not(all(debug_assertions, ecr_bill_git_sha)))]
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Estimate ECR storage cost from describe-images JSON. Reads files only, no network.
#[derive(Parser)]
#[command(
    name = "ecr-bill",
    bin_name = "ecr-bill",
    term_width = 0,
    version = VERSION,
    about = "Estimate ECR storage cost from describe-images JSON. Reads files only, no network.",
    after_help = "Exit codes: 0 ok, 2 input or usage error, 130 interrupted (SIGINT)."
)]
struct Cli {
    /// `aws ecr describe-images` JSON files.
    #[arg(required = true, value_name = "FILE")]
    files: Vec<PathBuf>,
    /// Price file. The default is the built-in US East (N. Virginia) list.
    #[arg(long, value_name = "FILE")]
    prices: Option<PathBuf>,
    /// Date ages are measured to, `YYYY-MM-DD`, midnight UTC. Without it, old images are omitted.
    #[arg(long, value_name = "YYYY-MM-DD")]
    as_of: Option<String>,
    /// An image is old when its push date is more than this many days before `--as-of`.
    #[arg(long, default_value_t = 30, value_parser = clap::value_parser!(u32).range(1..=36500))]
    older_than_days: u32,
    /// Untagged images older than this many days are in the proposed expire rule.
    #[arg(long, default_value_t = 14, value_parser = clap::value_parser!(u32).range(1..=36500))]
    untagged_days: u32,
    /// Tagged images beyond this count are in the proposed expire rule.
    #[arg(long, default_value_t = 10, value_parser = clap::value_parser!(u32).range(1..=10000))]
    keep: u32,
    /// Write the report to this file instead of standard output. An existing regular file is replaced; a symbolic link is refused.
    #[arg(long, value_name = "FILE")]
    output: Option<PathBuf>,
    /// Print JSON instead of the text table.
    #[arg(long)]
    json: bool,
    /// Print Markdown tables instead of the text table. Not with `--json`.
    #[arg(long, conflicts_with = "json")]
    md: bool,
    /// Print only the report and errors: no warning lines on standard error. Not with `--verbose`.
    #[arg(long, conflicts_with = "verbose")]
    quiet: bool,
    /// Add progress lines on standard error. Not with `--quiet`.
    #[arg(long)]
    verbose: bool,
}

fn main() -> ExitCode {
    interrupt::install();
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(err) => {
            if err.use_stderr() {
                let text = usage_error_text(&err);
                let _ = write!(std::io::stderr(), "{text}");
                return ExitCode::from(2);
            }
            let text = err.render().to_string();
            let mut out = stdout().lock();
            // Rust treats a closed stdout (`>&-`) as a successful write.
            if stdout_was_closed()
                || out.write_all(text.as_bytes()).is_err()
                || out.flush().is_err()
            {
                let _ = writeln!(std::io::stderr(), "ecr-bill: could not write output");
                return ExitCode::from(2);
            }
            return ExitCode::SUCCESS;
        }
    };
    if cli.verbose {
        progress(&format!(
            "reading {} file{}",
            cli.files.len(),
            if cli.files.len() == 1 { "" } else { "s" }
        ));
        for file in &cli.files {
            progress(&format!("reading {}", display_path(file)));
        }
    }
    match run(&cli) {
        Ok((output, price_warning)) => {
            if let (Some(warning), false) = (price_warning, cli.quiet) {
                let _ = writeln!(std::io::stderr(), "warning: {warning}");
            }
            if cli.json && !cli.quiet {
                for warning in &output.warnings {
                    let _ = writeln!(std::io::stderr(), "warning: {warning}");
                }
            }
            // The report is complete: from here a SIGINT cannot cut it in half.
            interrupt::ignore();
            if let Some(path) = &cli.output {
                if cli.verbose {
                    progress(&format!("writing {}", display_path(path)));
                }
                if let Err(reason) = write_report_file(path, output.stdout.as_bytes()) {
                    let _ = writeln!(
                        std::io::stderr(),
                        "ecr-bill: could not write {}: {reason}",
                        display_path(path)
                    );
                    return ExitCode::from(2);
                }
                if cli.verbose {
                    progress("done");
                }
                return ExitCode::SUCCESS;
            }
            let mut out = stdout().lock();
            // Rust treats a closed stdout (`>&-`) as a successful write.
            if stdout_was_closed()
                || out.write_all(output.stdout.as_bytes()).is_err()
                || out.flush().is_err()
            {
                let _ = writeln!(std::io::stderr(), "ecr-bill: could not write output");
                return ExitCode::from(2);
            }
            if cli.verbose {
                progress("done");
            }
            ExitCode::SUCCESS
        }
        Err(err) => {
            let _ = writeln!(std::io::stderr(), "ecr-bill: {err}");
            ExitCode::from(2)
        }
    }
}

/// Write the report to `path` in one step: the bytes go to a new file in the
/// same directory, which is then renamed over `path`, so `path` holds either
/// the old content or the whole new report. A symbolic link and anything that
/// is not a regular file are refused; the temporary file is created with
/// `create_new`, which does not follow a link either.
fn write_report_file(path: &Path, bytes: &[u8]) -> Result<(), String> {
    match std::fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_symlink() => {
            return Err("the path is a symbolic link".to_string())
        }
        Ok(meta) if !meta.is_file() => return Err("the path is not a regular file".to_string()),
        Ok(_) => {}
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(err) => return Err(err.to_string()),
    }
    let name = path
        .file_name()
        .ok_or_else(|| "the path has no file name".to_string())?;
    let parent = match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    };
    let mut temp_name = std::ffi::OsString::from(".");
    temp_name.push(name);
    temp_name.push(format!(".ecr-bill-{}.tmp", std::process::id()));
    let temp = parent.join(temp_name);
    let result = (|| -> std::io::Result<()> {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        std::fs::rename(&temp, path)
    })();
    if let Err(err) = result {
        let _ = std::fs::remove_file(&temp);
        return Err(err.to_string());
    }
    Ok(())
}

/// One progress line for `--verbose`. The text is built from escaped parts.
fn progress(text: &str) {
    let _ = writeln!(std::io::stderr(), "ecr-bill: {text}");
}

/// The text of a usage error. Clap repeats the user's arguments in it, so the
/// message is rendered again from arguments whose control, line-break and
/// invisible characters are already written as escapes: the argument stays on
/// the one line clap gives it, and no argument can start a line of its own.
fn usage_error_text(err: &clap::Error) -> String {
    let shown: Vec<String> = std::env::args_os()
        .map(|arg| display_path(Path::new(&arg)))
        .collect();
    let rendered = match Cli::try_parse_from(&shown) {
        Err(again) if again.use_stderr() => again.render().to_string(),
        // The escaped arguments parsed after all (an argument that is not
        // UTF-8): do not trust either text, and keep it on one line that ends
        // with a newline.
        _ => format!(
            "{}\n",
            err.render().to_string().trim_end().replace('\n', " ")
        ),
    };
    show_lines(&rendered)
}

/// True only when fd 1 was closed (`>&-`) or open for reading only (`1<file`) at process start.
/// `1<>/dev/null` and `>/dev/null` stay open. Missing `/proc` is not closed.
fn stdout_was_closed() -> bool {
    stdout_probe::stdout_was_closed_at_start()
}

fn run(cli: &Cli) -> Result<(ecr_bill::Output, Option<String>), Error> {
    let prices = if let Some(path) = &cli.prices {
        let text = read_text_file(path)?;
        load_prices_str(&text)
            .map_err(|err| Error::new(format!("price file {}: {err}", display_path(path))))?
    } else {
        ecr_bill::prices_builtin()
    };
    // The clock is read only to judge the price date, never to date the bill.
    let today = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| {
            i64::try_from(since.as_secs() / 86_400).unwrap_or(0)
        });
    let price_warning = price_age_warning(&prices, cli.as_of.as_deref(), today);
    let options = Options {
        prices,
        as_of: cli.as_of.clone(),
        older_than_days: cli.older_than_days,
        untagged_days: cli.untagged_days,
        keep: cli.keep,
        json: cli.json,
        markdown: cli.md,
    };
    Ok((execute_paths(&cli.files, &options)?, price_warning))
}
