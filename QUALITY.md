# Quality

Record of 2026-11-25 on code commit 8003f90 (clean worktree, detached at origin/main). Linux x86_64, rustc 1.99.0 and cargo 1.99.0 (stable).

This page records checks. It is not a guarantee and it is not a fuzz report.

## Commands

| Command | Exit | Result |
| --- | ---: | --- |
| `cargo fmt --check` | 0 | clean |
| `cargo clippy --all-targets --locked -- -D warnings` | 0 | no warnings |
| `cargo test --locked` (debug) | 0 | 204 passed, 0 failed, 0 ignored |
| `cargo test --release --locked` | 0 | 204 passed, 0 failed, 0 ignored |

Per target: lib unittests 96, `tests/cli.rs` 53, `tests/docs_clean.rs` 4, `tests/documented_shape.rs` 4, `tests/handbook_help.rs` 2, `tests/interrupt.rs` 2, `tests/markdown.rs` 6, `tests/output.rs` 9, `tests/readme.rs` 16, `tests/schema_version.rs` 4, `tests/verbosity.rs` 8, doc-tests 0. In the debug run the result line of `tests/documented_shape.rs` was garbled by interleaved output; its 4 tests are all listed `ok`, and the release run reports 4 passed.

## Not run

- Rust 1.85 (MSRV): not run
- Windows target: not run
- macOS target: not run
- Unpacked package (`cargo package`): not run
- `cargo deny` and `cargo audit`: not run
- Fuzzing and mutation testing: not run
- Earlier measurements (203 passed on 7cdee6b, 204 on 00e03bd) are replaced by this record and were not repeated beyond the table above.

Version 1.0.0 is not published and has no git tag.
