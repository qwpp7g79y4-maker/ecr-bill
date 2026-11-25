//! Offline estimate of Amazon ECR storage cost from `describe-images` JSON.
//!
//! The library reads bytes you already have. It does not open a socket, it does
//! not link an AWS SDK, and it does not apply a lifecycle policy.

#![forbid(unsafe_code)]

mod bill;
mod error;
mod json_scan;
mod money;
mod parse;
mod policy;
mod price;
mod report;
mod run;

pub use bill::{bill_documents, Bill, BillOptions, ImageLine, RepoLine};
pub use error::{show_lines, Error};
pub use money::{days_from_civil, format_usd, gb_from_bytes};
pub use policy::{propose_policy, validate_policy};
pub use price::{
    load_prices_str, price_age_warning, prices_builtin, Prices, PRICE_PAGE_DATE, PRICE_PAGE_URL,
    PRICE_STALE_DAYS,
};
pub use report::{render, render_markdown};
pub use run::{
    display_path, execute_paths, read_text_file, set_before_open_for_test, Options, Output,
};

/// Largest input accepted per file, in bytes (256 MiB).
pub const MAX_FILE_BYTES: u64 = 268_435_456;

/// Page the built-in rates were read from.
pub const PRICE_SOURCE_LINE: &str = "https://aws.amazon.com/ecr/pricing/ as of 2026-11-09";
