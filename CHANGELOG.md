# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed

- A path that becomes a fifo with no writer between the file check and the open no longer blocks the process. On Unix the open does not wait, and the opened file is refused when it is not a regular file (`not a file`). A fifo that is already a fifo at the check is still refused without being opened. Test: `a_fifo_replaced_between_the_check_and_the_open_is_refused`.

### Changed

- Documentation: the entry under 1.0.0 about a path that changes between the first check and the open is limited to what it proves: the file is never read as another kind of file. The package file count in `QUALITY.md` is corrected (52 from a `git archive`, 53 in the repository) and the README sections in its claim table now include Maintenance.
- The tests no longer read the system clock: every test that runs the program and expects an empty standard error passes a fixed `--as-of` (the price date), so the price age warning cannot turn them red later. Checked with the clock moved 400 days ahead: 11 tests failed before, none after (203 tests in all, unchanged). `SECURITY.md` and the README now say the same about time and memory: the size limit bounds the input, the measured factors are in the README. No behaviour of the program changed.
- The output says it is an estimate from list prices, not your AWS invoice. The text report has the line `Estimate from list prices, not your AWS invoice (free tier, credits, discounts not included)` under the title, and `--json` has `"estimate": true` at the top level. The money field is renamed `usd_month` to `estimated_usd_month` in every text row, the text total, the JSON total and each JSON repository, and a digest that contains `estimated_usd_month=` is escaped like the other field names. The old name is gone: a script that reads `usd_month` must read `estimated_usd_month`.

### Added

- `THIRD-PARTY-LICENSES.html`: the licenses of the third-party crates in the binary, generated with `cargo about generate --locked about.hbs`.
- `"schema_version": 1` is the first key of the `--json` report (the version of the layout of the JSON). A failed run prints no JSON, only one error line on standard error.

- `--output FILE` writes the report to a file instead of standard output: a new file in the same directory is written and renamed over `FILE`, so `FILE` holds the old content or the whole new report. An existing regular file is replaced; a symbolic link, a directory or any other file that is not a regular file is refused, a missing directory is not created, and a failed or interrupted run leaves an existing `FILE` as it was. A write failure is exit code 2 with `ecr-bill: could not write FILE: reason`.

- SIGINT (Ctrl-C) on Unix ends the run with exit code 130 and the line `ecr-bill: interrupted by SIGINT` on standard error; nothing is written to standard output, and the signal is ignored once the finished report is being written, so a report is never cut in half. The `--help` text and the man page list code 130.

- `--md`: the report as Markdown tables (repositories with a total row, untagged, old, the lifecycle JSON, warnings), with the same amount strings as the text and JSON reports and the estimate line. Pipes, backslashes and other Markdown characters in a value get a backslash, and control and line-break characters are written as escapes first. `--md` with `--json` is a usage error with exit code 2.

- `--quiet` and `--verbose`. `--quiet` prints only the report and errors: the warning lines on standard error are not written (they stay in the report). `--verbose` adds progress lines on standard error. Both leave standard output and the exit code as they are; together they are a usage error with exit code 2 (the usage-error code of this tool).

- A test file with the documented shape of `aws ecr describe-images` output (`tests/documented_shape.rs`, fixtures in `tests/fixtures/`): epoch and ISO 8601 push times, missing and empty tag lists, several tags, names with `/`, `_`, `.` and `-`, extra documented fields, a `nextToken`, an empty list and several files in one call. No change to the program.

- Documentation standard: the Dutch README (`README.nl.md`) and `QUALITY.nl.md`, `CONTRIBUTING.md`, README sections Installation, Aan de slag in 5 minuten, Options, Example, Exit codes, Support and Contact (the Maintenance section is now Support, and the quick start no longer writes a file), `examples/README.md` with `examples/expected-output.txt`, `build.rs` (a debug build shows the git commit in `--version`; a release build does not), `about.hbs` and `about.toml` for the third-party license listing, and a release workflow that builds archives with that listing. No change to the numbers or to the exit codes.

- A warning on standard error, with exit code 0 and the same standard output, when the `source_date` of the price file in use (built-in or `--prices`) is more than 180 days before `--as-of`, or before today without `--as-of`: `warning: prices are N days old (price date YYYY-MM-DD, as of YYYY-MM-DD); check current prices`. A `source_date` that is not `YYYY-MM-DD` gives no warning. The clock is read only for this check.

- `README.md` and `README.nl.md` have Prices and responsibility, and Maintenance (its own section, apart from Support): the built-in prices are a snapshot the reader checks, and a new version follows a reported bug, a request, or a security issue.

## [1.0.0] - 2026-11-19

### Changed

- Version 1.0.0. CI also runs `cargo audit` and `cargo deny --locked check licenses`.
- A file is read no further than the length its metadata reported, and a fifo or other non-regular file is refused.
- Text lines and error messages never carry a raw control character, line or paragraph separator, or invisible format character (bidi overrides, zero-width characters, tag characters, blank and filler characters, variation selectors). Each is written as `\u{hex}`. Usage errors from the command line are escaped the same way, and the arguments they repeat are escaped before the message is built, so an argument with a line break, a carriage return or an escape sequence stays on the one line clap prints and cannot start a line of its own. In `--json`, `source_url`, `source_date`, `region` and `region_name` are escaped by the same rule, and every string in the document has its control, line-break and invisible characters (including DEL and the C1 controls, which `serde_json` leaves raw) written as `\uXXXX` JSON escapes.
- An image digest on a text line has its whitespace written as `\u{hex}` and the `=` after `bytes`, `tags`, `images`, `usd_month`, `untagged` and `age_days` written as `\=`, so a digest cannot forge a field. An `imageDigest` that contains a space or a tab is an error (`invalid imageDigest`). A digest that is literally `(none)`, or that starts with a quote, is shown inside quotes (`"(none)"`, `""(none)""`), so no digest looks like a missing one or like another digest.
- Only ASCII whitespace (space, tab, carriage return, line feed) is trimmed around an `imageStatus` and around a price. An `imageStatus` that contains any other Unicode whitespace is an error (`invalid imageStatus`).
- A path that is not valid UTF-8 is shown as its escaped bytes in messages, not with U+FFFD. An error that does not already say `file N` now starts with the name of the file it came from.
- JSON nesting up to 128 levels is accepted and deeper input is the error `JSON nesting is deeper than 128 levels` (it was 64 levels and `not JSON`). The duplicate-key scan reads one leading value, and anything after that value is `not JSON`, so a deep second value never reaches the parser. `serde_json` has the `unbounded_depth` feature so that 128 levels parse.
- Amounts are exact. A repository cost (`bytes` times the rate divided by 1,073,741,824), a repository's share of a tiered archive amount and every sum are whole numbers over `10^28 * 2^30` in a 512-bit integer, and the cent is taken once at the end, half away from zero, for each row and for the total. Before, the quotient by 2^30 and a share were rounded to 28 digits first, so `99231259014473123` bytes at `0.894698637813` (exactly 82684748.12499999999999999999906...) printed `82684748.13` and `126819283` bytes at `9683085068513257837` printed `...327.84` where the exact cent is `...327.83`. A cost no longer fails when a sum has too many digits for the decimal type: the old limit and the old error for it are gone, and `cost does not fit` remains only for a 512-bit overflow that no valid input reaches. An archive tier boundary that does not fit in 64 bits is not an error: no byte total can pass it, so everything is priced at the first-tier rate.
- A repository's archive amount is priced from its own archived bytes (`bytes` times the rate divided by 1,073,741,824) whenever every archived byte is at one rate: the tier boundary is not crossed, or the two tier rates are equal. Before, the last archive row was the total minus the earlier shares, so an exact half cent could print one cent low: at an archive rate of `0.005`, 1,298,153,865,216 bytes (1209 GiB, exactly 6.045) printed `6.04` next to a second repository. When the boundary is crossed and the rates differ, a repository gets its proportional share of the tiered total, as an exact fraction.
- A usage error for a command-line argument that is not valid UTF-8 is one line that ends with a newline; before, it had no newline at the end.
- A price-file field that is present but of the wrong kind or out of range (`source_url` not starting with `https://`, a text field that is not a string, `archive_tier_tb` or `archive_min_days` that is not a whole number up to 4294967295) is `price file has an invalid NAME`; before, it said `price file is missing NAME`, which is kept for an absent field.
- A closed standard output (`>&-`) is now detected, and the run, `--help` and `--version` exit 2 with `ecr-bill: could not write output` instead of exit 0. A standard output that is open for reading only (`1<file`, or a directory) gives the same message and exit 2; before, it exited 0 and wrote nothing. A full device or a closed pipe gives the same message and exit 2. `>/dev/null` and `1<>/dev/null` still exit 0. The check reads whether file descriptor 1 was already closed, or open for reading only, at process start, which is possible only on Linux; `src/stdout_probe.rs` holds the one `unsafe` call, and the library keeps `#![forbid(unsafe_code)]`.
- A price rate of more than 28 digits is the error `price file has a rate with more than 28 digits`. The decimal type rounded it when the file was read, so one byte at `5368709.1199999999999999999999999999999` (just under half a cent) was printed as `0.01` where the exact amount rounds to `0.00`. Zeros at either end of a rate do not count.
- A price file whose rate is present but is not a decimal string (`"1e2"`, `"-0.10"`, a number, `null`) is now `price file has a rate that is not a decimal` for every rate; `price file is missing standard` is kept for a rate that is absent. Before, an invalid `standard` or archive rate said it was missing.
- A file is checked again after it is opened (a regular file, not above the size limit), so a path that changes between the first check and the open cannot be read as another kind of file. The second check has its own function and a test.
- `Cargo.toml`: `homepage` is `https://www.pepai.nl`, and the package is an allow list (`include`) of sources, tests, examples, the man page, prices, the licenses and the documents, so no working file can enter it and no file name is listed for exclusion.
- `.gitattributes` forces LF for every text file (`* text=auto eol=lf`, plus `*.1` and `*.json`), so the man page and the example files are byte-exact in a checkout that would otherwise convert line endings.
- `.github/workflows/ci.yml` sets `timeout-minutes` on the test jobs, `persist-credentials: false` on every checkout, and `--no-fail-fast` on the test commands, cancels an older run of the same ref (`concurrency`), and has a `glibc` job that fails when the release binary needs a glibc symbol newer than 2.34.
- `deny.toml` allows an unused license entry, so `cargo deny` no longer warns about one.
- Comments in `src/stdout_probe.rs` are in English.
- `README.md` states that warnings are `warning:` lines in the text report, and are in the `warnings` array and on standard error with `--json`. It states the exit codes, and it says a file that is not a regular file is an error. It also states the accepted forms of `imagePushedAt`, the shape and rules of a price file, the error for an amount that does not fit, the input rules, the ranges of `--keep`, `--older-than-days`, `--untagged-days` and `--as-of`, the nesting limit of 128, the measured memory use, and the platforms that were and were not run. It no longer states the number of tests. `man/ecr-bill.1` has the same rules and ranges.
- `README.md` and `man/ecr-bill.1` describe which rates are refused (those the decimal type cannot hold exactly, with the two messages), the invalid-field message, products and quotients, blank and `null` statuses, the `+` sign in a number option, the text form of the four price-file texts in `--json`, and how an archive repository is priced.
- `README.md`, `QUALITY.md` and `man/ecr-bill.1` give memory use per input shape, measured with the release binary at 8388608, 16777216 and 33554432 bytes. One-element lists were 103.32, 103.06 and 102.99 times the file; tiny values 53.07, 53.08 and 53.01 times; lists nested 120 levels 152.30, 152.26 and 152.18 times; 100000, 200000 and 400000 ordinary images 17.72, 17.43 and 17.32 times. The highest, the 120-level lists, would need 40883978240 bytes at the per-file limit of 268435456 bytes (calculated from the 8388608-byte run, not run). Several files add up.
- `QUALITY.md` is a measured record: commands with exit codes, the count per test target, what was not run, known limits, and a table from each README claim to its test.

### Added

- `ecr_bill::show_lines` and `ecr_bill::display_path` are public.
- `scripts/check-glibc.sh`, which the `glibc` job runs, and is part of the package.
- `SECURITY.md`, and `.github/dependabot.yml`.
- `man/ecr-bill.1`, and `tests/handbook_help.rs`, which checks that the man page holds the `--help` text verbatim, that its OPTIONS section on its own has exactly the long options of `--help`, and that `.gitattributes` forces LF. `tests/docs_clean.rs` fails, in any letter case and with the line number, if a document, the man page, a package file (including `Cargo.lock` and both licenses), a script, a workflow, a source or a test holds a local path, a build-wrapper option, the name of a working file or an internal tool, the name of an AI system or a person, or a co-author line.
- Tests for the escaping, digest, depth, rounding, archive-share, file-label, policy-schema and stdout cases above. There are now 158 tests: 95 unit tests, 46 command-line tests, 11 README tests, 2 handbook tests and 4 document tests.

This version is not published and has no git tag.

## [0.1.0] - 2026-11-09

### Added

- `ecr-bill` reads local `aws ecr describe-images` JSON and prices storage per repository.
- Untagged images and images older than a chosen number of days are listed.
- A lifecycle policy is printed as JSON text. The policy is not applied.
- Built-in US East (N. Virginia) rates from the Amazon ECR pricing page, read on 2026-11-09, and a `--prices` file for another quote.

This version is not published and has no git tag.
