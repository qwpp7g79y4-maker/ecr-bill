# Security policy

## Supported versions

| Version | Supported |
| --- | --- |
| 1.0.x | Yes |

Security fixes go into the latest release only.

## Prices

The built-in prices are a snapshot dated in `source_date`. When that date (or the one in a `--prices` file) is more than 180 days before `--as-of`, or before today without it, ecr-bill writes one warning line to standard error and still exits 0. A price file whose `source_date` is not `YYYY-MM-DD` gets no warning. Check current prices yourself.

## Reporting a vulnerability

Please report security problems privately, not in a public issue: open the **Security** tab of this repository and choose **Report a vulnerability**. If that is not possible, write to pep_ai@icloud.com. Examples are an image digest, repository name, region name, or file name that can add a fake line or a terminal escape to the report or to an error, an error that echoes a value it should hide, a way to make ecr-bill write, run, or connect to something, a file that makes it read without the stated size limit, crash, or hang beyond the measured factor, and input that changes an amount without an error.

Please include:

- the ecr-bill version (the `version` field in `Cargo.toml`, or the commit you built) and your operating system;
- the exact command;
- a minimal synthetic `describe-images` JSON file that reproduces the problem, never a real account listing;
- what ecr-bill printed and what you expected.

The size limit bounds the input, not the time or the memory. What a file below the limit can cost in time and memory is measured and stated as factors in the README (see Memory and size under Limitations); growth within those factors is not a vulnerability. What does count as a vulnerability: a panic, a hang beyond the measured factor, growth without bound beyond the documented factor, and a report that does not follow the limits stated in the README.

We aim to respond within 7 days (best effort). Once a fix is released, the report can be credited in the changelog if you wish.
