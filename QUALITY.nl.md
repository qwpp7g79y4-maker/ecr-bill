# Kwaliteit

Meting van 2026-11-25 op codecommit 157f671 (schone worktree, tak fix/output-long-name). Linux x86_64, rustc 1.99.0 en cargo 1.99.0 (stable).

Deze pagina legt controles vast. Het is geen garantie en geen fuzzrapport.

## Commando's

| Commando | Exit | Uitslag |
| --- | ---: | --- |
| `cargo fmt --check` | - | not run |
| `cargo clippy --all-targets --locked -- -D warnings` | - | not run |
| `cargo test --locked` (debug) | 0 | 206 geslaagd, 0 mislukt, 0 genegeerd |
| `cargo test --release --locked` | 0 | 206 geslaagd, 0 mislukt, 0 genegeerd |

Per doel: lib-unittests 96, `tests/cli.rs` 53, `tests/docs_clean.rs` 4, `tests/documented_shape.rs` 4, `tests/handbook_help.rs` 2, `tests/interrupt.rs` 2, `tests/markdown.rs` 6, `tests/output.rs` 11, `tests/readme.rs` 16, `tests/schema_version.rs` 4, `tests/verbosity.rs` 8, doc-tests 0.

## Not run

- Rust 1.85 (MSRV): gemeten op 2026-11-25 op de code van b70469c met rustc 1.85.1 en cargo 1.85.1: `cargo +1.85 fmt --check` exit 0, `cargo +1.85 clippy --all-targets --locked -- -D warnings` exit 0, `cargo +1.85 test --locked` (debug) exit 0 met 206 geslaagd, 0 mislukt, 0 genegeerd; release met 1.85: `cargo +1.85 test --release --locked` exit 0 met 241 geslaagd, 0 mislukt, 0 genegeerd, gemeten op 2026-11-25 op a883081
- Windows-doel: not run
- macOS-doel: not run
- Uitgepakt pakket (`cargo package`): not run
- `cargo deny` en `cargo audit`: not run
- Fuzzen en mutatietests: not run
- Eerdere metingen (203 geslaagd op 7cdee6b, 204 op 00e03bd en op 8003f90) zijn door deze meting vervangen en zijn buiten de tabel hierboven niet herhaald.

Versie 1.0.0 is niet gepubliceerd en heeft geen git-tag.
