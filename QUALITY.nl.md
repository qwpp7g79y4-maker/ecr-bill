# Kwaliteit

Meting van 2026-11-25 op codecommit 8003f90 (schone worktree, losgekoppeld op origin/main). Linux x86_64, rustc 1.99.0 en cargo 1.99.0 (stable).

Deze pagina legt controles vast. Het is geen garantie en geen fuzzrapport.

## Commando's

| Commando | Exit | Uitslag |
| --- | ---: | --- |
| `cargo fmt --check` | 0 | schoon |
| `cargo clippy --all-targets --locked -- -D warnings` | 0 | geen waarschuwingen |
| `cargo test --locked` (debug) | 0 | 204 geslaagd, 0 mislukt, 0 genegeerd |
| `cargo test --release --locked` | 0 | 204 geslaagd, 0 mislukt, 0 genegeerd |

Per doel: lib-unittests 96, `tests/cli.rs` 53, `tests/docs_clean.rs` 4, `tests/documented_shape.rs` 4, `tests/handbook_help.rs` 2, `tests/interrupt.rs` 2, `tests/markdown.rs` 6, `tests/output.rs` 9, `tests/readme.rs` 16, `tests/schema_version.rs` 4, `tests/verbosity.rs` 8, doc-tests 0. In de debugrun was de resultaatregel van `tests/documented_shape.rs` door door elkaar lopende uitvoer onleesbaar; de 4 tests staan alle vier als `ok`, en de releaserun meldt 4 geslaagd.

## Not run

- Rust 1.85 (MSRV): not run
- Windows-doel: not run
- macOS-doel: not run
- Uitgepakt pakket (`cargo package`): not run
- `cargo deny` en `cargo audit`: not run
- Fuzzen en mutatietests: not run
- Eerdere metingen (203 geslaagd op 7cdee6b, 204 op 00e03bd) zijn door deze meting vervangen en zijn buiten de tabel hierboven niet herhaald.

Versie 1.0.0 is niet gepubliceerd en heeft geen git-tag.
