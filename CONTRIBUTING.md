# Contributing

Thanks for your interest in ecr-bill.

## Building and testing

```sh
cargo build
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

These checks must pass before a change is merged. Test fixtures must be synthetic: an invented account id, invented digests and invented repository names. Never add a real account listing, a real registry id or a real image name.

A change that alters behavior also updates `README.md`, `README.nl.md`, `QUALITY.md` and `CHANGELOG.md`; a test runs every console block of the README against the binary.

## Commit messages

Commit messages are short English sentences that say what changed, for example `Escape controls in values shown on the report`.

## Contact

PEPAI, pep_ai@icloud.com
