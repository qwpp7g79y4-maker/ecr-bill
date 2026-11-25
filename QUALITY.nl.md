# Kwaliteit

Het openen van een invoerbestand blokkeert op Unix niet: een pad dat tussen de controle en het openen door een fifo wordt vervangen, wordt geweigerd. Op codecommit 00e03bd, dat dat openen en één test in `tests/cli.rs` toevoegt, liepen `cargo fmt --check`, `cargo clippy --locked --offline --all-targets -- -D warnings` en debug `cargo test --locked --offline -j 2` met `--test-threads 2` op stable schoon: 204 geslaagd (lib-unittests 96, `tests/cli.rs` 53, de andere targets ongewijzigd). Release, Rust 1.85, Windows, macOS en het uitgepakte pakket zijn op 00e03bd niet opnieuw gedraaid. De meting van 2026-11-20 hieronder met 203 geslaagd geldt voor de eerdere codecommit 7cdee6b, vóór dat openen.

Nederlandse samenvatting van [QUALITY.md](QUALITY.md). De Engelse pagina is leidend; de claim→test-tabel met alle testnamen staat daar.

Suite: 203 geslaagd, 0 genegeerd, 0 mislukt (debug en release, Rust stable en 1.85).

Meting van 2026-11-20, rustc 1.99.0 en cargo 1.99.0 (stable), rustc 1.85.1, Linux x86_64 met glibc. De controles draaiden op codecommit 7cdee6b (schone werkmap, HEAD 3d986f0): fmt, clippy op stable en 1.85, de suite in debug en release op stable en 1.85, en het uitgepakte pakket, allemaal met 203 geslaagd. De commits na 7cdee6b wijzigen alleen documentatie, tot het openen zonder blokkeren. 7ac683f verschilt van de eerder gecontroleerde commit 38d13c3 in `README.md`, `README.nl.md` en de lijsten met koppen in `tests/readme.rs` (de kop Onderhoud), niet in de broncode. Versie 1.0.0 is niet gepubliceerd en heeft geen git-tag.

Deze pagina legt controles vast. Het is geen garantie en geen fuzz-rapport.

## Commando's

Alle commando's draaiden op 2026-11-20 met `-j 2` voor cargo en `--test-threads 2` voor de tests, na de laatste wijziging van de broncode.

| Commando | Afsluitcode |
| --- | ---: |
| `cargo fmt --check` | 0 |
| `cargo clippy --locked --offline --all-targets -- -D warnings` (stable) | 0 |
| `cargo +1.85 clippy --locked --offline --all-targets -- -D warnings` | 0 |
| `cargo test --locked --offline` | 0 |
| `cargo test --locked --offline --release` | 0 |
| `cargo +1.85 test --locked --offline` | 0 |
| `cargo +1.85 test --locked --offline --release` | 0 |
| `cargo check` en `cargo clippy --all-targets -D warnings` voor `x86_64-pc-windows-gnu` en `x86_64-apple-darwin` (vanaf Linux; ze draaien de code niet) | 0 |
| `cargo deny --locked check licenses` en `cargo audit --no-fetch` | 0 |
| `cargo package --locked --offline --allow-dirty --no-verify`, daarna `cargo test --offline` in het uitgepakte pakket (203 geslaagd) | 0 |

## Testresultaat per doel

| Doel | Geslaagd | Mislukt | Genegeerd |
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
| totaal | 203 | 0 | 0 |

## Niet uitgevoerd

- GitHub Actions: de workflows zijn niet gestart; eerdere runs zijn rood door een betaalprobleem, los van de getallen hierboven.
- De releaseworkflow en zijn archieven.
- Een fuzz-campagne en een mutatierun.
- Windows en macOS: er draaide geen test op die systemen.
- In deze meting niet herhaald: de musl-build en `scripts/check-glibc.sh`.
- `--output` en SIGINT op Windows en macOS: de SIGINT-code is `cfg(unix)` en de bijbehorende tests alleen op Unix; de hernoeming van `--output` is daar niet gedraaid.
- Of privé-kwetsbaarheidsmelding aanstaat voor de repository waar `SECURITY.md` naar verwijst.

## Bekende grenzen

- De controle op een gesloten of alleen-lezen standaarduitvoer werkt alleen op Linux.
- Geheugengebruik hangt af van de vorm van het bestand en de grens van 268435456 bytes geldt per bestand; de gemeten factoren staan in QUALITY.md en README.md.
- Een pad dat tussen de eerste controle en `open` door een fifo (zonder schrijver) wordt vervangen, blokkeerde `open` tot het proces werd gestopt. Gemeten op 2026-11-20, vóór de reparatie, met de debug-binary: een thread die het pad `target` wisselde tussen een symbolische link naar een gewoon bestand en een naar een fifo, en 60 runs van `ecr-bill target` met een grens van 2 seconden, gaven 10 hangs, 24 keer exit 2 (`not a file`) en 26 keer exit 0; een keurder zag 21 hangs in 150 runs. De aanvaller heeft schrijfrechten in de map van het bestand nodig. Het openen wacht nu niet (Unix), en het type wordt op het geopende bestand nogmaals gecontroleerd, dus zo'n pad wordt geweigerd (`not a file`) en hangt niet. Test: `a_fifo_replaced_between_the_check_and_the_open_is_refused`. Een fifo die bij de eerste controle al een fifo is, wordt nog steeds geweigerd zonder te worden geopend.
- Deze tool is niet vergeleken met een echte AWS-factuur: de uitvoer is een schatting uit lijstprijzen.
