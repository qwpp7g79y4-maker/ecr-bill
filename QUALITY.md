# Quality

The open of an input file does not block on Unix: a path swapped to a fifo between the check and the open is refused. On code commit 00e03bd, which adds that open and one test in `tests/cli.rs`, `cargo fmt --check`, `cargo clippy --locked --offline --all-targets -- -D warnings` and debug `cargo test --locked --offline -j 2` with `--test-threads 2` on stable ran clean: 204 passed (lib unittests 96, `tests/cli.rs` 53, the other targets unchanged). Release, Rust 1.85, Windows, macOS and the unpacked package were not re-run on 00e03bd. The measurements of 2026-11-20 below with 203 passed apply to the earlier code commit 7cdee6b, before that open.

Suite: 203 passed, 0 ignored, 0 failed (debug and release, Rust stable and 1.85).

Record of 2026-11-20, rustc 1.99.0 and cargo 1.99.0 (stable), rustc 1.85.1, Linux x86_64 with glibc. The checks ran on code commit 7cdee6b (clean working tree, HEAD 3d986f0): fmt, clippy on stable and 1.85, the suite in debug and release on stable and 1.85, and the unpacked package, all with 203 passed. The commits after 7cdee6b change only documentation, until the non-blocking open. 7ac683f differs from the earlier checked commit 38d13c3 in `README.md`, `README.nl.md` and the heading lists in `tests/readme.rs` (the Maintenance heading), not in the sources. Version 1.0.0 is not published and has no git tag.

This page records checks. It is not a guarantee and it is not a fuzz report.

## Commands

All commands ran on 2026-11-20 with `-j 2` for cargo and `--test-threads 2` for the tests, after the last change of the sources.

| Command | Exit |
| --- | ---: |
| `cargo fmt --check` | 0 |
| `cargo clippy --locked --offline --all-targets -- -D warnings` (stable) | 0 |
| `cargo +1.85 clippy --locked --offline --all-targets -- -D warnings` | 0 |
| `cargo check --locked --offline --tests --target x86_64-pc-windows-gnu` | 0 |
| `cargo check --locked --offline --tests --target x86_64-apple-darwin` | 0 |
| `cargo clippy --locked --offline --all-targets --target x86_64-pc-windows-gnu -- -D warnings` | 0 |
| `cargo clippy --locked --offline --all-targets --target x86_64-apple-darwin -- -D warnings` | 0 |
| `cargo test --locked --offline` | 0 |
| `cargo test --locked --offline --release` | 0 |
| `cargo +1.85 test --locked --offline` | 0 |
| `cargo +1.85 test --locked --offline --release` | 0 |
| `cargo deny --locked check licenses` | 0 |
| `cargo audit --no-fetch` (1527 advisories, 71 crates) | 0 |
| `cargo package --locked --offline --allow-dirty --no-verify`, then `cargo test --offline` in the unpacked crate (203 passed) | 0 |
| the same tests with the program's clock moved 400 days ahead (copy of the tree, not committed) | 0 (203 passed) |

The two Windows and two macOS rows compile and lint the code for those targets from Linux. They do not run it.

`cargo package --locked --offline --allow-dirty --no-verify` packed 52 files when run on a `git archive` of the commit, and 53 when run in the repository (cargo then adds `.cargo_vcs_info.json`): the 40 of the documentation standard (see CHANGELOG), `src/interrupt.rs`, the new test files `tests/documented_shape.rs`, `tests/verbosity.rs`, `tests/markdown.rs`, `tests/interrupt.rs`, `tests/output.rs` and `tests/schema_version.rs`, and the files in `tests/fixtures/`. `Cargo.toml` has an `include` list, so no working note and no CI file can enter the package. The package was unpacked in an empty directory and `cargo test --offline` ran there: 203 passed, 0 failed. The tests need no path outside the package; the two that make large files write them under the target directory of the test run.

## Test result per target

Debug (`cargo test --locked --offline`), release (the same with `--release`) and Rust 1.85 (`cargo +1.85 test --locked --offline`, debug) give the same counts.

| Target | Passed | Failed | Ignored |
| --- | ---: | ---: | ---: |
| unittests `src/lib.rs` | 96 | 0 | 0 |
| unittests `src/main.rs` | 0 | 0 | 0 |
| `tests/cli.rs` | 52 | 0 | 0 |
| `tests/docs_clean.rs` | 4 | 0 | 0 |
| `tests/documented_shape.rs` | 4 | 0 | 0 |
| `tests/handbook_help.rs` | 2 | 0 | 0 |
| `tests/interrupt.rs` | 2 | 0 | 0 |
| `tests/markdown.rs` | 6 | 0 | 0 |
| `tests/output.rs` | 9 | 0 | 0 |
| `tests/readme.rs` | 16 | 0 | 0 |
| `tests/schema_version.rs` | 4 | 0 | 0 |
| `tests/verbosity.rs` | 8 | 0 | 0 |
| doc-tests | 0 | 0 | 0 |
| total, debug | 203 | 0 | 0 |
| total, release | 203 | 0 | 0 |
| total, Rust 1.85 debug | 203 | 0 | 0 |
| total, Rust 1.85 release | 203 | 0 | 0 |

Before the documentation standard the suite had 164 tests (96, 0, 51, 4, 2, 11, 0); before `--quiet`, `--verbose`, `--md`, `--output`, SIGINT and `schema_version` it had 173 (96, 0, 52, 4, 4, 2, 15, 0); before the price-age warning it had 159 tests (95, 0, 47, 4, 2, 11, 0); before the estimate wording it had 158 tests (95, 0, 46, 4, 2, 11, 0); before this record it had 146 tests (87, 0, 42, 4, 2, 11, 0); before the exact-amount change it had 154 (92, 0, 45, 4, 2, 11, 0).

Red before green. The price-age warning (R43) was red on commit ea9edfc: 3 of the 4 new tests in `tests/cli.rs` failed there (the fourth is a guard that no warning appears for a future price date or a failed run, and passes on both). The estimate wording (R42) was red on commit 0521477: 20 tests failed (the new one and 19 that read the renamed field). Each test that goes with a behavior change was run first against the sources of commit 284275e and failed there (unit tests in `src/`: the new test with the old function; `tests/cli.rs`: the new file copied into an archive of 284275e):

- `bill::a_shared_archive_row_that_is_an_exact_half_cent_rounds_up`: `6.04` where the exact amount is 6.045 and rounds to `6.05`;
- `bill::a_crossed_tier_with_one_rate_on_both_sides_is_priced_per_repository`: `6.04` for the same repository when the tier boundary is crossed and both rates are equal;
- `bill::a_product_that_would_lose_digits_is_an_error_or_exact_never_rounded` and `a_product_that_would_lose_digits_is_exact_in_text_and_json` (`tests/cli.rs`): `7900827040092728403.55` where the exact amount rounds to `.56`;
- `price::a_field_is_missing_only_when_the_key_is_absent`: `price file is missing source_url` for `http://x/`;
- `a_usage_error_for_an_argument_that_is_not_utf8_is_one_line_with_a_newline` (`tests/cli.rs`): one line, but without the closing newline;
- `bill::a_quotient_just_under_a_half_cent_is_not_rounded_up`, `bill::rows_and_total_of_a_crossed_tier_are_exact_fractions_rounded_once` and `a_quotient_just_under_a_half_cent_prints_the_exact_cent_in_text_and_json` (`tests/cli.rs`): run on commit f9d5501 and red: `82684748.13` for `82684748.12`, a row `9544005200673628659.06` for `.07`.

`run::what_was_opened_is_checked_again`, `a_blank_status_a_plus_sign_and_the_price_texts_are_as_the_readme_says` describe behavior that 284275e already had, so they have no red run on its sources; the first of them goes with a function that did not exist. All of the tests above pass now.

The man page is checked by `help_long_options_match_the_man_page_and_unknown_option_is_not_a_panic` (`tests/handbook_help.rs`): it holds the `--help` text verbatim, the long options of the whole page are exactly those of `--help`, and the OPTIONS section alone has exactly those long options. On a copy of the tree, with the `--keep` entry removed from OPTIONS only, it failed: `the OPTIONS section of man/ecr-bill.1 must describe exactly the long options of --help`.

`tests/docs_clean.rs` reads the four documents, every file under `man/`, `Cargo.toml`, `Cargo.lock`, `deny.toml`, `.gitattributes`, both licenses, the workflows, `scripts/`, `src/`, `tests/`, `examples/` and `prices/`. It matches 27 words in any letter case: local paths for Linux, macOS, Windows and the shell, options of a build wrapper, names of working files and helpers, names of AI systems, a co-author line and a person. On a copy of the tree whose README held ten such lines (a path in each of three forms, two wrapper options, two working-file names, a model name, a co-author line and a model abbreviation) and whose `src/main.rs` held one, both file tests failed and listed each line with its number. The test source holds none of its words whole.

The claim table below is read by `the_quality_claim_table_counts_match_its_summary_and_name_real_tests`: its summary line must equal the count of the rows, and every test it names must exist.

## Closed and full standard output

`src/stdout_probe.rs` holds the one `unsafe` call of the project. Rust replaces a closed file descriptor 1 with `/dev/null` before `main` runs, so afterwards a closed stdout cannot be told from `1<>/dev/null`, and a write to a descriptor opened for reading only (`1<file`) fails with EBADF, which Rust counts as written. On Linux a constructor in `.init_array` runs earlier and records whether descriptor 1 was already closed or open for reading only (`fcntl` with `F_GETFD` and `F_GETFL`). `src/main.rs` reads that flag and exits 2 with `ecr-bill: could not write output`. `src/lib.rs` does not use the module and keeps `#![forbid(unsafe_code)]`. On other systems the flag is always false.

Measured on 2026-11-19 with the release binary and the debug binary built from the sources of this record, on `examples/describe-images.json` (FILE), `--help` and `--version`. Both binaries gave the same result in every row:

| Call | Exit | Message |
| --- | ---: | --- |
| FILE, `--help`, `--version` with `>&-` | 2 | `ecr-bill: could not write output` |
| FILE, `--help`, `--version` with `>/dev/full` | 2 | `ecr-bill: could not write output` |
| FILE, `--help`, `--version` with `1<` an empty file (descriptor open for reading only) | 2 | `ecr-bill: could not write output` |
| FILE, `--help`, `--version` with `1<.` (a directory) | 2 | `ecr-bill: could not write output` |
| FILE, `--help`, `--version` with `>/dev/null` | 0 | none |
| FILE, `--help`, `--version` with `1<>/dev/null` | 0 | none |
| Python `subprocess.run(..., stdout=DEVNULL)`, for FILE, `--help` and `--version` | 0 | none |
| `ecr-bill FILE \| true` with `pipefail`, 200 runs per binary; also once each for `--help` and `--version` | debug: 2 in 200 runs; release: 2 in 200 runs; exit 2 for `--help` and `--version` | `ecr-bill: could not write output`; no panic in any run |
| `/proc` replaced by an empty tmpfs (`unshare -Urm`), FILE, `--help` and `--version` to a file | 0 | none; 1174, 985 and 15 bytes written |
| the same without `/proc`, FILE with `>&-` and with `1<` an empty file | 2 | `ecr-bill: could not write output` |

A pipe run that exits 0 is possible, because it is a race: if the tool writes its report into the pipe before the reader `true` has exited, a write that the pipe accepted cannot be told from a write that succeeds. An earlier series of 200 release runs of this tool had one such run; the series of this record had none. The test `a_closed_pipe_exits_2_without_a_panic_and_a_null_stdio_exits_0` waits until the reader is gone, so it does not depend on the race.

The tests `closed_full_and_null_stdout`, `help_and_version_to_a_full_device_exit_2`, `write_failure_to_dev_full_exits_2_without_panic`, `a_read_only_standard_output_exits_2_with_a_message` and `a_closed_pipe_exits_2_without_a_panic_and_a_null_stdio_exits_0` in `tests/cli.rs` cover every row. The Python call is covered through `Stdio::null()`, which opens `/dev/null` the same way; the missing `/proc` is not covered by a test. These tests run on Linux only (`#[cfg(target_os = "linux")]`).

## Usage errors

With the release and the debug binary, `--keep`, `--older-than-days` and `--untagged-days` given `$'1\x1b[31m\nFAKE: line'` exit 2 with a three-line message and no line that starts with `FAKE`; the argument shows as `'1\u{1b}[31m\nFAKE: line'` on the line of clap's `error:`. The unknown option `$'--x\x1b[31m\nFAKE: line'` exits 2 with a seven-line message, no line starting with `FAKE`, and no escape byte. `--as-of` and `--prices` given the same argument exit 2 with one `ecr-bill:` line and no escape byte. A carriage return, a bidi override and a zero-width space in an argument also appear as `\u{..}` on one line. An argument that is not valid UTF-8 (`--as-of $'\xff\nerror: FAKE\x1b[31m'`) exits 2 with one line that ends in a newline and holds no escape byte; this record added the closing newline and a test.

## Not run

- In this record: the musl build and `scripts/check-glibc.sh`. They ran for the record of 2026-11-19 and not again for this one.
- `--output` and SIGINT on Windows and macOS: the SIGINT code is `cfg(unix)` and its tests are Unix only; the `--output` rename was not run there.
- A fuzz campaign.
- A mutation run.
- GitHub Actions. The workflows were not started; every earlier run is red from billing, which is separate from the numbers above.
- Windows and macOS. Only `cargo check` and `cargo clippy` for those targets ran, from Linux. No test ran on them, the closed-stdout and read-only-stdout checks work only on Linux, and no minimum version of either system was determined.
- musl: the binary was built and run once on the example; the test suite was not run on that target.
- The release workflow and its binaries. The workflow was not started on GitHub, and no archive was built.
- Whether private vulnerability reporting is switched on for the repository that `SECURITY.md` points to.
- `git push`, `git tag`, and `cargo publish`.

## Known limits

- The closed-stdout and read-only-stdout checks work only on Linux. On other systems the program cannot tell a closed stdout from `/dev/null`; those cases were not measured there.
- `ecr-bill FILE | true` exits 2 with `could not write output` when the reader has already closed the pipe, and does not panic. When the reader has not yet exited, the pipe accepts the report and the exit code is 0: one of 200 release runs did so in an earlier series, none of 200 in the series of this record.
- Memory depends on the shape of the file, and the size limit is per file, not per run. Several files add up, because all files are read before any is parsed. Measured with the release binary (maximum resident set, in KiB of 1024 bytes, divided by the file size) at 8388608, 16777216 and 33554432 bytes. One-element lists `{"imageDetails":[],"x":[[0],[0],...]}` took 0.900 s and 846392 KiB (103.32 times), 1.700 s and 1688608 KiB (103.06 times), and 3.500 s and 3374884 KiB (102.99 times); the same lists in `{"imageDetails":[[0],[0],...]}` took 0.880 s, 1.700 s and 3.490 s at 103.22, 103.05 and 103.02 times and exit 2 (`not an object`). Tiny values `[1,1,...]` in that `x` array took 0.560 s and 434780 KiB (53.07 times), 1.110 s and 869688 KiB (53.08 times), and 2.470 s and 1737104 KiB (53.01 times). Lists nested 120 levels, repeated in that array, took 1.130 s and 1247680 KiB (152.30 times), 2.240 s and 2494620 KiB (152.26 times), and 4.460 s and 4986792 KiB (152.18 times). A normal list of 100000, 200000 and 400000 images `{"repositoryName":"aa","imageSizeInBytes":1}` (4500018, 9000018 and 18000018 bytes) took 0.230 s and 77876 KiB (17.72 times), 0.430 s and 153160 KiB (17.43 times), and 0.880 s and 304452 KiB (17.32 times). Twice and four times the smallest file used 1.995 and 3.987 times the resident set for one-element lists, 1.999 and 3.997 for the 120-level lists, 2.000 and 3.995 for tiny values, and 1.967 and 3.909 for the image list. The highest ratio is the 120-level lists: 268435456 bytes of that shape would need 40883978240 bytes, from the 8388608-byte run, and that file was not run. A file of 268435456 NUL bytes took 0.190 s and 264616 KiB and is `not JSON`; one byte more is refused at once. A machine with less memory ends the process with the operating system's kill, not with exit code 2. Making this smaller needs a different parser and is not part of this version.
- All files are read before any is parsed, so the memory of several large files adds up.
- A path replaced by a fifo (no writer) between the first check and the open used to block `open` until the process was killed. Measured on 2026-11-20, before the fix, with the debug binary: a thread that swapped the path `target` between a symbolic link to a regular file and a symbolic link to a fifo, and 60 runs of `ecr-bill target` with a 2-second limit, gave 10 hangs, 24 exit 2 (`not a file`) and 26 exit 0; a reviewer saw 21 hangs in 150 runs. The attacker needs write access to the directory of the file. The open is now non-blocking on Unix, and the file type is checked again on the opened file, so that path is refused (`not a file`) and does not hang. Test: `a_fifo_replaced_between_the_check_and_the_open_is_refused`. A fifo that is already a fifo at the first check is still refused without being opened.
- An error about one image says `file N: image M`, and an error about a whole file says the file name. This is how the messages are built; a file name that starts with `file ` followed by a number is not mistaken for the first kind.
- Sums, products and rates. A rate that the decimal type cannot hold exactly is refused when the price file is read: `price file has a rate with more than 28 digits` when it would be rounded, `price file has a rate that is not a decimal` when it is above 79228162514264337593543950335. Measured: `1234567890123456789012345678.9` and `79228162514264337593543950335` (29 digits) are accepted, `9999999999999999999999999.1234` and `0.00000000000000000000000000001` are `more than 28 digits`, `99999999999999999999999999999` and `79228162514264337593543950336` are `not a decimal`; zeros right after the point count as digits, zeros at the ends of the text do not. Before this limit a rate of 31 digits was rounded, and one byte at `5368709.1199999999999999999999999999999` printed `0.01` instead of `0.00`. The cost is exact: bytes times rate, every sum and every archive share are whole numbers over `10^28 * 2^30` in a 512-bit integer, and the cent is taken once at the end, half away from zero. Measured: one Standard and one Archive gibibyte at a Standard rate of 26 nines print `99999999999999999999999999.10`, and at 27 nines, 28 digits and 29 digits (`79228162514264337593543950335`) print the exact amount ending in `.10` (by test); two gibibytes at 79228162514264337593543950335 print `158456325028528675187087900670.00`; one gibibyte at `7900827040092728403.555` prints `7900827040092728403.56`; `99231259014473123` bytes at `0.894698637813` print `82684748.12` (the version before this change printed `.13`), and `126819283` bytes at `9683085068513257837` print `1143665896371805327.83` (before: `.84`). The old guarantee "above about 1e24 USD the cent is not guaranteed" and the old error for a sum of 27 nines are gone: there is no amount limit other than 512 bits, which a rate of at most 28 digits and fewer than 2^64 bytes cannot reach (such a number has at most about 320 bits). The error `cost does not fit` is kept for that overflow and is tested on the integer type.
- Differential check against exact fractions. 22,112 random inputs (rates of 1 to 22 digits and 0 to 12 decimals for Standard, first and over tier; tiers of 1, 2 and 150 TB; one to four repositories; Standard bytes up to 2^59 and archive bytes up to 2^55; text sizes as JSON) were run with the release binary and compared, for every row and the total, with `fractions.Fraction` rounded half away from zero: 0 differences. The generator with rates of 14 to 22 digits, run on the sources before this change, found 7 differences in 6,365 inputs (rows and totals). The comparison script is not part of the package. Only some of the 22,112 runs have an exact half cent, so that case is covered by the unit tests, not by this count.
- Archive amounts. When every archived byte is at one rate (the tier boundary is not crossed, or the two rates are equal), each repository is priced from its own archived bytes. When the boundary is crossed and the rates differ, each row is `(standard * all + archive total * its bytes) / all` as one exact fraction and the total is the exact sum, each rounded once; before this change a share was a quotient of 28 significant digits and the total a sum of such quotients. Test: `bill::rows_and_total_of_a_crossed_tier_are_exact_fractions_rounded_once` (two cases, expected values from Python `fractions.Fraction`, red on the old sources: `9544005200673628659.06` for `.07`, and a total `…325.78` for `…325.77`). Time and memory were measured on the release binary with 100,000, 200,000 and 400,000 images over half as many repositories and a 1 TB tier: 0.35, 0.63 and 1.30 seconds and 88, 175 and 346 MB, the same as the old sources (0.34, 0.64, 1.29 seconds): linear.
- A rate with leading zeros after the point counts them as digits: `0.00000000000000000000000000001` is refused, although it has one significant digit.
- An empty or blank `imageStatus`, and `null`, count as Standard without a warning; a status in lower case is read as in upper case. A leading `+` is accepted in a number option (`--keep +5`).
- "No network call" is checked by the dependency list (`cargo tree -e normal` has `clap`, `rust_decimal` and `serde_json` only), by `Cargo.lock`, and by `claim_r1_sources_and_lock_file_have_no_network_or_process_code`, which looks for names (including the bare name of the policy call) in every source file. No test watches the system calls.
- The built-in rates were read from the Amazon pricing page on 2026-11-09. They were not read again for this record.
- This page does not compare the tool with a real AWS bill.

## Open points

These are choices for the owner, not defects found by a test.

- No fuzz targets and no funding file. They are not needed to build or use the tool.
- Pricing a repository's share exactly when the archive tier is crossed with two different rates (see Known limits).
- No minimum version of macOS or Windows.
- No total limit on memory; see above.
- The text `(none)` for a missing digest is kept. Real digests that look like it are shown inside quotes.
- No upper limit on a rate other than what the decimal type holds (79228162514264337593543950335); an amount is exact at any size the 512-bit integer holds, which no valid input exceeds.

## Claim → test

Every claim of `README.md`, the man page and the line-ending rule, with the test that proves it. Test names are from the 2026-11-20 run of 203 tests. "Gap" means no test proves it, or only part of it. Tests are in `src/*.rs` unless the file is named.

| # | Claim | Test | Status |
| --- | --- | --- | --- |
| C1 | One `imageDetails` element is one image; the same digest in two files counts twice | `bill::claim_c1_one_element_is_one_image_and_duplicates_count` | tested |
| C2 | 1 GB is 1,073,741,824 bytes; 40 GB is 4.00 USD and 20 GB is 2.00 USD | `bill::claim_c2_forty_gibibytes_match_the_page_example`, `money::gibibyte_matches_the_page_unit` | tested |
| C3 | Standard is 0.10 USD per GB-month, source, date and region | `price::claim_c3_standard_rate_source_date_region` | tested |
| C4 | A missing `imageStatus` is Standard | `bill::claim_c4_missing_status_is_standard` | tested |
| C5 | `ARCHIVED` is tiered at 150 TB; retrieval is not added | `bill::claim_c5_archive_tiers_do_not_add_retrieval`, `bill::two_archive_repos_split_the_tier` | tested |
| C6 | `ACTIVATING` and other statuses are unpriced, counted, and not echoed | `bill::claim_c6_activating_and_unknown_are_unpriced` | tested |
| C7 | No tags, null, or an empty array is untagged | `parse::claim_c7_empty_tags_are_untagged` | tested |
| C8 | Without `--as-of` the old list is omitted | `bill::claim_c8_savings_age_omitted_without_as_of` | tested |
| C9 | Old means more than `--older-than-days` before `--as-of`; a missing push time is skipped for age and the digest is not echoed | `bill::claim_c9_age_uses_the_threshold_and_hides_a_missing_push` | tested |
| C10 | The lifecycle block is two expire rules; the words Not executed are outside the JSON | `policy::claim_c10_proposal_is_two_expire_rules`, `readme::console_examples_match_the_binary` (`tests/readme.rs`) | tested |
| C11 | The proposal and the documented lifecycle examples pass the schema check | `policy::claim_c11_documented_examples_are_valid` | tested |
| C12 | `tagStatus` `tagged` needs a pattern or a prefix list, not both; at most four `*` | `policy::claim_c12_tag_lists_and_wildcards` | tested |
| C13 | `sinceImagePulled` and `sinceImageTransitioned` rules | `policy::claim_c13_pull_and_archive_actions` | tested |
| C14 | `any` priority order, positive `countNumber`, `countUnit` | `policy::claim_c14_priority_count_and_units` | tested |
| C15 | Rows round half away from zero; the total is the unrounded sum rounded once; JSON money is a string | `report::claim_c15_total_is_the_sum_and_json_uses_strings`, `bill::claim_c15_small_amount_rounds_to_the_cent`, `bill::exact_half_cent_with_a_custom_rate_rounds_up`, `bill::thousand_small_repositories_total_is_the_unrounded_sum`, `money::usd_rounds_half_away_from_zero_to_the_cent` | tested |
| C16 | The registry id is not printed | `report::claim_c16_registry_id_is_not_printed`, `example_hides_the_account_and_json_is_quiet` (`tests/cli.rs`) | tested |
| C17 | `--prices` replaces the quote; an unknown field is ignored; a duplicate key is an error | `bill::claim_c17_prices_override_changes_the_bill`, `price::claim_c17_unknown_field_ignored_and_duplicate_hidden`, `json_scan::duplicate_key_is_exact` | tested |
| C18 | A bad `imageSizeInBytes` is reported without the value or the digest | `parse::claim_c18_bad_size_hides_the_value`, `empty_details_are_zero_and_bad_size_hides_the_row` (`tests/cli.rs`) | tested |
| R1 | Reads files only, no network, no AWS SDK, no PutLifecyclePolicy | `claim_r1_sources_and_lock_file_have_no_network_or_process_code` (`tests/readme.rs`) | gap: source text and lock file only, no system-call test |
| R2 | Install with `cargo install --locked --path .`, Rust 1.85, not published | `installation_uses_a_local_path`, `licenses_and_changelog_match_the_package` (`tests/readme.rs`) | tested |
| R3 | Version 1.0.0 | `help_and_version_exit_0` (`tests/cli.rs`, the `--version` text), `licenses_and_changelog_match_the_package` (`tests/readme.rs`) | tested |
| R4 | (retired: the README no longer states a test count) | none | not a claim |
| R5 | The quick start names `examples/describe-images.json` and writes no file | `the_quick_start_writes_nothing_and_the_example_file_is_the_one_described` (`tests/readme.rs`) | tested |
| R6 | The text and JSON examples are what the binary prints | `console_examples_match_the_binary` (`tests/readme.rs`) | tested |
| R7 | `--as-of` is midnight UTC | `money::dates_follow_the_calendar`, `console_examples_match_the_binary` (age 1029 and 2504 days) | tested |
| R8 | Warnings are `warning:` lines in the text report, and in the `warnings` array and on stderr with `--json` | `warnings_are_lines_in_text_and_an_array_plus_stderr_in_json` (`tests/cli.rs`), `bill::claim_c6_activating_and_unknown_are_unpriced` | tested |
| R9 | Exit 0 is success, 2 is an input or usage error, including output that cannot be written | `usage_errors_exit_2`, `missing_file_exits_2`, `help_and_version_exit_0`, `write_failure_to_dev_full_exits_2_without_panic`, `closed_full_and_null_stdout`, `help_and_version_to_a_full_device_exit_2`, `a_read_only_standard_output_exits_2_with_a_message` (`tests/cli.rs`) | tested |
| R10 | `not_executed` sits beside the policy object in JSON | `console_examples_match_the_binary` (`tests/readme.rs`) | tested |
| R11 | Archive dollars are tiered across the archived bytes of the input; every archived byte at one rate is priced per repository, otherwise each repository gets its proportional share | `bill::two_archive_repos_split_the_tier`, `bill::large_archive_at_5368709_12_is_pinned`, `bill::three_archive_repos_at_5368709_12_are_pinned`, `bill::a_shared_archive_row_that_is_an_exact_half_cent_rounds_up`, `bill::a_crossed_tier_with_one_rate_on_both_sides_is_priced_per_repository` | tested |
| R12 | The 90-day archive minimum and the free tier are not applied | `report::text_lists_source_and_region`, `console_examples_match_the_binary` (the `not applied` and `free tier not applied` lines) | tested |
| R13 | A file above 268435456 bytes is refused | `run::file_larger_than_268435456_bytes` | tested |
| R14 | A symlink to a file is followed; a directory is an error | `run::directory_is_rejected_and_symlink_is_followed`, `symlink_is_followed_and_a_directory_is_rejected` (`tests/cli.rs`) | tested |
| R15 | Any other file that is not a regular file is an error, including a path replaced by a fifo between the check and the open, which does not block | `an_input_fifo_is_refused_without_opening_it`, `prices_fifo_is_refused_without_opening_it`, `a_fifo_replaced_between_the_check_and_the_open_is_refused` (`tests/cli.rs`) | tested |
| R16 | The built-in quote is us-east-1 | `price::claim_c3_standard_rate_source_date_region` | tested |
| R17 | The rates, the 150 TB tier and the page examples are those on the Amazon pricing page of 2026-11-09 | none | gap: external page, not read again |
| R18 | Other registries, data transfer, managed signing and encryption are out of scope; the age list and the policy are not a forecast | none | gap: statements of scope, nothing to test |
| R19 | MIT OR Apache-2.0, with both license files | `licenses_and_changelog_match_the_package` (`tests/readme.rs`) | tested |
| R20 | The README sections are Why, Installation, Quick start, Aan de slag in 5 minuten, Options, Example, How it counts, Exit codes, Limitations, Prices and responsibility, Maintenance, Support, License, Contact | `readme_sections_follow_the_documentation_standard` (`tests/readme.rs`) | tested |
| M1 | The man page holds the `--help` text and the long options of `--help`, in SYNOPSIS and in OPTIONS | `help_long_options_match_the_man_page_and_unknown_option_is_not_a_panic` (`tests/handbook_help.rs`) | tested |
| M2 | Text files are checked out with LF, so the byte-exact files do not change | `line_endings_are_forced_to_lf_so_byte_exact_files_survive_a_crlf_checkout` (`tests/handbook_help.rs`) | tested |
| R21 | A file is UTF-8 JSON with one value; a byte order mark is accepted, UTF-16 is refused, a second value is `not JSON`, a duplicate key is an error | `readme_input_rules_hold_for_bom_utf16_status_case_and_the_byte_range`, `a_deep_second_value_exits_2_without_a_crash`, `non_utf8_file_name_is_shown_as_byte_escape` (`tests/cli.rs`), `json_scan::duplicate_key_is_exact` | tested |
| R22 | Nesting is at most 128 levels and deeper input has a named error | `json_scan::depth_tests::depth_128_is_valid_and_129_names_the_limit`, `json_scan::depth_tests::the_depth_error_text_is_exact` | tested |
| R23 | `imageSizeInBytes` is 0 to 18446744073709551615 and a total that does not fit is an error | `readme_input_rules_hold_for_bom_utf16_status_case_and_the_byte_range` (`tests/cli.rs`), `bill::byte_totals_that_overflow_are_an_error` | tested |
| R24 | `imageStatus` is read without regard to case and ASCII spaces are trimmed | `readme_input_rules_hold_for_bom_utf16_status_case_and_the_byte_range` (`tests/cli.rs`) | tested |
| R25 | A digest with a space or a tab is an error; a missing digest is `(none)`; a digest that is `(none)` or starts with a quote is shown in quotes | `digest_with_spaces_is_rejected`, `quoted_none_and_bare_none_and_a_missing_digest_are_three_different_digests`, `literal_none_digest_differs_from_missing_digest` (`tests/cli.rs`), `parse::digest_display_is_one_to_one_for_none_and_quotes` | tested |
| R26 | Control, line-break and invisible characters are written as `\u{hex}` in text and messages, and as `\uXXXX` in `--json` strings | `digest_controls_are_not_raw_in_text`, `digest_with_hangul_filler_is_escaped_in_text`, `digest_with_unassigned_format_chars_is_escaped_in_text`, `json_digest_never_carries_c1_del_or_bidi_characters_raw`, `region_name_with_bidi_override_is_escaped_in_text_and_json`, `file_name_with_bidi_override_still_counts_and_is_escaped` (`tests/cli.rs`), `error::tests::*`, `report::json_text_escapes_unsafe_characters_and_keeps_pretty_line_breaks` | tested |
| R27 | A usage error repeats an argument on one line and no argument starts a line of its own | `usage_errors_keep_a_newline_in_an_argument_on_one_line`, `usage_errors_escape_control_and_bidi_characters`, `a_usage_error_for_an_argument_that_is_not_utf8_is_one_line_with_a_newline` (`tests/cli.rs`) | tested |
| R28 | `--keep` is 1 to 10000, `--older-than-days` and `--untagged-days` are 1 to 36500, `--as-of` is 0000-01-01 to 9999-12-31 | `documented_ranges_are_accepted_at_the_edge_and_refused_beyond_it`, `usage_errors_exit_2` (`tests/cli.rs`) | tested |
| R29 | Errors about a whole file name the file; errors about one image say `file N: image M` | `single_file_errors_name_the_file` (`tests/cli.rs`), `parse::image_shape_errors_name_file_and_image`, `bill::a_file_label_prefixes_errors_that_lack_a_file_number` | tested |
| R30 | Memory use depends on the shape: one-element lists 103.32, 103.06 and 102.99 times the file at 8388608, 16777216 and 33554432 bytes; tiny values 53.07, 53.08 and 53.01; 120-level lists 152.30, 152.26 and 152.18 (highest; 40883978240 bytes calculated at the 268435456-byte limit, not run); 100000, 200000 and 400000 images 17.72, 17.43 and 17.32 times | none | gap: measured by hand with the release binary, not by a test |
| R31 | Built on Linux x86_64 with glibc 2.34 or newer; musl built and run once; Windows and macOS not run; closed-stdout check on Linux only | `closed_full_and_null_stdout` (`tests/cli.rs`, Linux only) | gap: the platform statements are measurements, and the glibc job is not started on GitHub |
| R32 | (CHANGELOG) A price rate that is present but not a decimal is not reported as missing | `price_file_rates_say_missing_only_when_absent` (`tests/cli.rs`), `price::a_rate_is_missing_only_when_the_key_is_absent` | tested |
| R33 | A price file has the shape of `prices/ecr-storage.json`; a rate is a string of at most 40 characters that the decimal type holds exactly (more than 28 digits and a number above the largest value are different errors); an absent field is missing, a field of the wrong kind is invalid; a file that is not a regular file is an error | `prices_file_is_a_small_regular_file`, `invalid_price_file_content_exits_2_and_names_the_file`, `price_file_rates_say_missing_only_when_absent`, `a_rate_with_more_digits_than_the_decimal_type_holds_is_refused_not_rounded`, `prices_fifo_is_refused_without_opening_it` (`tests/cli.rs`), `price::a_rate_is_missing_only_when_the_key_is_absent`, `price::a_field_is_missing_only_when_the_key_is_absent` | tested |
| R34 | `imagePushedAt` is a number of seconds from 0 to 9223372036854775807 or an ISO 8601 string with a zone; a missing or null value skips the image for age; anything else is `invalid imagePushedAt` | `the_documented_forms_of_a_push_time_are_accepted_and_every_other_is_refused`, `push_time_files_accept_iso8601_and_an_exponent`, `non_ascii_push_time_exits_2_without_panic` (`tests/cli.rs`), `parse::push_time_accepts_iso8601_and_an_exponent` | tested |
| R35 | An amount is exact: bytes times rate, every share and every sum are exact fractions and the cent is taken once, half away from zero; a rate of 79228162514264337593543950335 and any sum of such amounts print exactly | `a_sum_of_large_amounts_is_exact_in_text_and_json`, `a_product_that_would_lose_digits_is_exact_in_text_and_json`, `an_extreme_price_is_priced_exactly_without_panic`, `a_quotient_just_under_a_half_cent_prints_the_exact_cent_in_text_and_json` (`tests/cli.rs`), `money::a_charge_is_exact_for_any_rate_the_decimal_type_holds`, `money::a_sum_of_amounts_is_exact`, `money::cents_round_once_half_away_from_zero_and_pad`, `money::an_amount_that_overflows_the_integer_is_an_error`, `bill::extreme_rate_is_priced_exactly`, `bill::a_quotient_just_under_a_half_cent_is_not_rounded_up`, `bill::rows_and_total_of_a_crossed_tier_are_exact_fractions_rounded_once`, `bill::a_product_that_would_lose_digits_is_an_error_or_exact_never_rounded` | tested |
| R36 | A standard output that is open for reading only exits 2 with `could not write output` (Linux) | `a_read_only_standard_output_exits_2_with_a_message` (`tests/cli.rs`) | tested |
| R37 | An empty or blank `imageStatus` and `null` count as Standard without a warning; `--keep +5` is accepted; the four price-file texts in `--json` are in the text form first | `a_blank_status_a_plus_sign_and_the_price_texts_are_as_the_readme_says` (`tests/cli.rs`) | tested |
| R38 | (CHANGELOG) A file is checked again after it is opened | `run::what_was_opened_is_checked_again` | tested |
| R39 | (CHANGELOG) A repository's archive row is not a remainder of the total | `bill::a_shared_archive_row_that_is_an_exact_half_cent_rounds_up` | tested |
| R40 | (CHANGELOG) The `glibc` job fails when the release binary needs a symbol newer than 2.34 | none | gap: the job was not started on GitHub; the script ran locally |
| R41 | A usage error echoes no raw control character and no argument starts a line, also for an unknown option | `usage_errors_keep_a_newline_in_an_argument_on_one_line` (`tests/cli.rs`) | tested |
| R42 | (CHANGELOG) The output says it is an estimate from list prices, not the AWS invoice: one text line, the field `estimated_usd_month` in rows, total and JSON, and `"estimate": true` in JSON | `text_and_json_say_the_amount_is_an_estimate`, `example_hides_the_account_and_json_is_quiet` (`tests/cli.rs`), `report::claim_c15_total_is_the_sum_and_json_uses_strings`, `console_examples_match_the_binary` (`tests/readme.rs`) | tested |
| R43 | (CHANGELOG) A price date (`source_date`, built-in or `--prices`) more than 180 days before `--as-of`, or before today without it, gives one warning on standard error with exit 0 and unchanged standard output; exactly 180 days, a date that is not `YYYY-MM-DD` and a later price date give none | `builtin_prices_older_than_180_days_warn_once_on_stderr_and_stdout_is_unchanged`, `own_price_file_is_judged_by_its_own_source_date`, `a_price_date_in_the_future_or_a_failed_run_gives_no_age_warning`, `without_as_of_the_price_age_is_taken_from_today` (`tests/cli.rs`), `money::civil_tests::civil_from_days_inverts_days_from_civil` | tested |
| R44 | `README.nl.md` has the same sections, in the same order, and the same console blocks and claims C1 to C18 as the English README | `the_dutch_readme_has_the_same_sections_and_the_same_console_blocks` (`tests/readme.rs`) | tested |
| R45 | The five commands of the five-minute section run as written, offline, and exit 0 | `the_five_minute_commands_run_literally_and_exit_0` (`tests/readme.rs`) | tested |
| R46 | Both READMEs end with a Contact section with the e-mail address, website, X and Ko-fi links | `contact_is_last_and_names_the_maintainer_and_both_readmes_agree` (`tests/readme.rs`) | tested |
| R47 | `--version` shows the commit in a debug build when git names one, and only the crate version in a release build or an unpacked crate | `version_shows_the_commit_in_a_debug_build_only_when_git_names_one` (`tests/cli.rs`) | tested |
| R48 | `examples/` holds a README and the expected output of its command | `the_example_directory_has_the_expected_output_of_its_command` (`tests/readme.rs`) | tested |
| R49 | The documented shape of `aws ecr describe-images` output (source: <https://docs.aws.amazon.com/cli/latest/reference/ecr/describe-images.html>, also named in `tests/documented_shape.rs`), in its CLI v1 and v2 forms and the variants seen in practice, is read without error and priced exactly | `the_documented_cli_v1_shape_is_priced_exactly`, `the_documented_cli_v2_shape_with_iso_times_and_zones_is_priced_exactly`, `an_empty_list_a_next_token_and_several_files_add_up_exactly`, `an_empty_list_alone_is_a_zero_bill_not_an_error` (`tests/documented_shape.rs`) | tested |
| R50 | `--quiet` drops the warning lines on standard error and keeps errors, `--verbose` adds escaped progress lines on standard error, neither changes standard output or the exit code, and the two together are exit 2 | `quiet_drops_the_price_age_warning_and_keeps_standard_output`, `quiet_drops_the_json_warnings_on_standard_error_and_keeps_them_in_the_json`, `quiet_still_reports_errors_and_exits_2`, `verbose_adds_progress_on_standard_error_and_nothing_to_standard_output`, `verbose_keeps_the_exit_code_of_a_failed_run_and_escapes_file_names`, `verbose_with_json_keeps_standard_output_a_json_object`, `quiet_with_verbose_is_a_usage_error_with_exit_2_and_no_output`, `help_lists_quiet_and_verbose` (`tests/verbosity.rs`) | tested |
| R51 | `--md` prints Markdown tables with the amounts of the text and JSON reports, the estimate line, escaped cells, and is refused together with `--json` | `markdown_amounts_equal_text_and_json_for_every_input`, `markdown_says_it_is_an_estimate_and_hides_the_account`, `markdown_escapes_pipes_and_line_breaks_in_cells`, `markdown_and_json_together_are_a_usage_error`, `markdown_keeps_the_exit_code_and_errors_of_the_other_formats`, `markdown_to_a_full_device_exits_2_without_a_panic` (`tests/markdown.rs`), `the_markdown_example_in_both_readmes_is_what_the_binary_prints` (`tests/readme.rs`) | tested |
| R52 | SIGINT exits 130 with `ecr-bill: interrupted by SIGINT` on standard error and nothing on standard output; an uninterrupted run is unchanged | `a_sigint_while_reading_exits_130_with_a_message_and_no_output`, `a_run_that_is_not_interrupted_is_unchanged` (`tests/interrupt.rs`), `help_and_version_exit_0` (`tests/cli.rs`) | tested |
| R53 | `--output FILE` writes the same bytes as standard output through a temporary file and a rename, refuses a symbolic link and a non-regular file, creates no directory, keeps an old file on failure or SIGINT, and a write failure is exit 2 with a message | `the_file_holds_exactly_what_standard_output_would_and_standard_output_is_empty`, `an_existing_file_is_replaced_whole_and_no_temporary_file_is_left`, `a_symbolic_link_is_refused_and_not_followed`, `a_directory_and_a_missing_parent_are_errors_and_nothing_is_created`, `a_directory_that_cannot_be_written_is_exit_2_with_a_message`, `a_failed_run_leaves_an_existing_output_file_as_it_was`, `the_message_escapes_the_file_name_and_verbose_names_the_file`, `a_sigint_before_the_report_is_finished_keeps_the_existing_output_file`, `help_lists_output` (`tests/output.rs`) | tested |
| R54 | Every `--json` report (also with `--quiet`, `--verbose`, `--output` and `--prices`, and for an empty list) starts with `"schema_version": 1`; a failed run prints no JSON | `every_json_report_starts_with_the_schema_version`, `the_json_in_an_output_file_has_the_schema_version`, `a_failed_json_run_prints_no_json_only_one_error_line`, `the_readme_json_example_and_the_docs_name_the_schema_version` (`tests/schema_version.rs`), `console_examples_match_the_binary` (`tests/readme.rs`) | tested |

73 claims (1 retired): 67 tested, 6 gaps (R1 in part, R17, R18, R30, R31, R40).
