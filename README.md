# ecr-bill

[![crates.io](https://img.shields.io/crates/v/ecr-bill.svg)](https://crates.io/crates/ecr-bill) [![downloads](https://img.shields.io/crates/d/ecr-bill.svg)](https://crates.io/crates/ecr-bill) [![CI](https://github.com/qwpp7g79y4-maker/ecr-bill/actions/workflows/ci.yml/badge.svg)](https://github.com/qwpp7g79y4-maker/ecr-bill/actions/workflows/ci.yml) [![PEPAI checked](https://img.shields.io/endpoint?url=https%3A%2F%2Fraw.githubusercontent.com%2Fqwpp7g79y4-maker%2Fecr-bill%2Fmain%2F.github%2Fbadge.json)](QUALITY.md) [![MSRV 1.85](https://img.shields.io/badge/MSRV-1.85-blue.svg)](https://github.com/qwpp7g79y4-maker/ecr-bill/blob/main/Cargo.toml) [![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/qwpp7g79y4-maker/ecr-bill#license)

Estimate Amazon ECR storage cost from `describe-images` JSON. Reads files only, no network.

## Why

An `aws ecr describe-images` export already lists each image's size, tags, and status. ecr-bill prices that JSON on your machine, per repository, lists untagged and old images, and prints a lifecycle policy as JSON. It does not call AWS and it does not need an account.

## Installation

Version 1.0.0. From crates.io:

```sh
cargo install ecr-bill
```

Or build from source.

With Rust 1.85 or newer, from a clone of this repository:

```sh
cargo install --locked --path .
ecr-bill --help
```

The tests that `cargo test --locked` runs are counted per test file in [QUALITY.md](QUALITY.md).

## Quick start

`examples/describe-images.json` is in the repository and is two Standard images in repository `app`, 1 GiB each. One is tagged `v1` and was pushed on 2024-01-15. The other has no tags and was pushed on 2020-01-01. The account id in the file is not printed. Nothing below writes a file: run the commands from the root of a clone, with `ecr-bill` on your `PATH`.

A test runs every console block below and compares it to this binary. `--as-of` is midnight UTC. Rates are US East (N. Virginia) from <https://aws.amazon.com/ecr/pricing/> as read on 2026-11-09.

```console
$ ecr-bill --as-of 2026-11-09 examples/describe-images.json
ecr-bill
Estimate from list prices, not your AWS invoice (free tier, credits, discounts not included)
source: https://aws.amazon.com/ecr/pricing/
as of: 2026-11-09
region: us-east-1 (US East (N. Virginia))
free tier not applied
not applied: retrieval 0.03 USD per GB; archive minimum 90 days

repositories
app images=2 bytes=2147483648 untagged=1 old=2 estimated_usd_month=0.20
total images=2 bytes=2147483648 untagged=1 estimated_usd_month=0.20

untagged
app sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb bytes=1073741824 tags=0

old
as-of: 2026-11-09
older_than_days: 30
app sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa age_days=1029 tags=1 bytes=1073741824
app sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb age_days=2504 tags=0 bytes=1073741824

lifecycle
Not executed
{
  "rules": [
    {
      "rulePriority": 1,
      "description": "Expire untagged images older than 14 days",
      "selection": {
        "tagStatus": "untagged",
        "countType": "sinceImagePushed",
        "countUnit": "days",
        "countNumber": 14
      },
      "action": {
        "type": "expire"
      }
    },
    {
      "rulePriority": 2,
      "description": "Keep the newest 10 tagged images",
      "selection": {
        "tagStatus": "tagged",
        "tagPatternList": [
          "*"
        ],
        "countType": "imageCountMoreThan",
        "countNumber": 10
      },
      "action": {
        "type": "expire"
      }
    }
  ]
}
```

Two gibibytes of ECR Standard are 0.20 USD per month. Both images are more than 30 days old on 2026-11-09, so `old=2`. The lifecycle JSON is a proposal: expire untagged images older than 14 days, and keep the newest 10 tagged images. The line `Not executed` means the tool did not call AWS.

Warnings are printed as `warning:` lines in the text report. With `--json` they are in the `warnings` array and also go to stderr. Exit code 0 is success, and 2 is an input or usage error, including output that cannot be written.

`--json` prints the same totals. Money and byte totals are strings. The policy object is the lifecycle text; `not_executed` sits beside it, not inside it.


## Quickstart in 5 minutes

Five commands, offline, on the file in `examples/`. They need no account and no network, and they write nothing. Run them from the root of a clone, with `ecr-bill` on your `PATH` (after `cargo install --locked --path .`, or use `target/release/ecr-bill`). A test runs the commands in this block literally.

```sh
ecr-bill --as-of 2026-11-09 examples/describe-images.json
ecr-bill --json --as-of 2026-11-09 examples/describe-images.json
ecr-bill --as-of 2026-11-09 --older-than-days 2000 examples/describe-images.json
ecr-bill --as-of 2026-11-09 --keep 1 --untagged-days 7 examples/describe-images.json
ecr-bill --prices prices/ecr-storage.json examples/describe-images.json
```

1. The text report, with the old list measured to 2026-11-09.
2. The same as one JSON object.
3. Only images pushed more than 2000 days before that date count as old.
4. Another lifecycle policy: keep 1 tagged image, expire untagged images after 7 days.
5. The built-in price file named explicitly; no `--as-of`, so the old list is omitted.

All five exit 0. For your own account, save `aws ecr describe-images --repository-name NAME` to a file and pass it as `FILE`.

## Options

Every option in `--help`.

- `FILE...`: `aws ecr describe-images` JSON files (at least one).
- `--prices <FILE>`: price file; the default is the built-in US East (N. Virginia) list.
- `--as-of <YYYY-MM-DD>`: date ages are measured to, midnight UTC. Without it, old images are omitted.
- `--older-than-days <N>`: an image is old when pushed more than N days before `--as-of` (default 30, 1 to 36500).
- `--untagged-days <N>`: untagged images older than N days are in the proposed expire rule (default 14, 1 to 36500).
- `--keep <N>`: tagged images beyond N are in the proposed expire rule (default 10, 1 to 10000).
- `--output <FILE>`: write the report to this file instead of standard output (standard output stays empty). The report goes to a new file in the same directory and is renamed over `FILE`, so `FILE` holds the old content or the whole new report. An existing regular file is replaced and keeps its permissions (`chmod 600` stays 600); a new file gets the default mode. The temporary file has a short name of its own (`.ecr-bill-<pid>.tmp`), so a file name close to the 255-byte limit works. A symbolic link and anything that is not a regular file are refused, and a missing directory is not created: exit 2 with `ecr-bill: could not write FILE: reason`. A failed or interrupted run leaves an existing `FILE` as it was.
- `--json`: print JSON instead of the text table. The object starts with `"schema_version": 1`, the version of the layout of this JSON; it changes only when a field is removed or its meaning changes. A run that fails prints no JSON, only one error line on standard error.
- `--md`: print Markdown tables instead of the text table. The amounts are the strings of the text and JSON reports. A pipe, a backslash and the other Markdown characters in a value get a backslash, and control and line-break characters are written as escapes first. Not with `--json` (usage error, exit 2).
- `--quiet`: print only the report and errors. The warning lines on standard error (the price-age warning and, with `--json`, the image warnings) are not written; they stay in the report. Standard output and the exit code do not change. Together with `--verbose` it is a usage error (exit 2).
- `--verbose`: add progress lines on standard error (`ecr-bill: reading N files`, `ecr-bill: reading FILE` with control characters escaped, `ecr-bill: done`). Standard output and the exit code do not change.
- `-h`, `--help`: print help. `-V`, `--version`: print the version.

## Example

The same file as JSON. A test compares this block to the binary.

```console
$ ecr-bill --json --as-of 2026-11-09 examples/describe-images.json
{
  "schema_version": 1,
  "estimate": true,
  "source_url": "https://aws.amazon.com/ecr/pricing/",
  "source_date": "2026-11-09",
  "region": "us-east-1",
  "region_name": "US East (N. Virginia)",
  "free_tier_applied": false,
  "retrieval_usd_per_gb": "0.03",
  "archive_min_days": 90,
  "retrieval_applied": false,
  "as_of": "2026-11-09",
  "older_than_days": 30,
  "untagged_days": 14,
  "keep": 10,
  "images": 2,
  "bytes": "2147483648",
  "untagged_images": 1,
  "estimated_usd_month": "0.20",
  "repositories": [
    {
      "repository": "app",
      "images": 2,
      "bytes": "2147483648",
      "untagged": 1,
      "old": 2,
      "estimated_usd_month": "0.20"
    }
  ],
  "untagged": [
    {
      "repository": "app",
      "digest": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
      "bytes": "1073741824",
      "tags": 0
    }
  ],
  "old": [
    {
      "repository": "app",
      "digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "bytes": "1073741824",
      "tags": 1,
      "age_days": 1029
    },
    {
      "repository": "app",
      "digest": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
      "bytes": "1073741824",
      "tags": 0,
      "age_days": 2504
    }
  ],
  "lifecycle": {
    "not_executed": true,
    "policy": {
      "rules": [
        {
          "rulePriority": 1,
          "description": "Expire untagged images older than 14 days",
          "selection": {
            "tagStatus": "untagged",
            "countType": "sinceImagePushed",
            "countUnit": "days",
            "countNumber": 14
          },
          "action": {
            "type": "expire"
          }
        },
        {
          "rulePriority": 2,
          "description": "Keep the newest 10 tagged images",
          "selection": {
            "tagStatus": "tagged",
            "tagPatternList": [
              "*"
            ],
            "countType": "imageCountMoreThan",
            "countNumber": 10
          },
          "action": {
            "type": "expire"
          }
        }
      ]
    }
  },
  "warnings": []
}
```

The same report as Markdown. A test compares this block to the binary.

```sh
ecr-bill --md --as-of 2026-11-09 examples/describe-images.json
```

````markdown
# ecr-bill

Estimate from list prices, not your AWS invoice (free tier, credits, discounts not included)

Source: https://aws.amazon.com/ecr/pricing/, price date 2026-11-09. Region: us-east-1 (US East (N. Virginia)). Free tier not applied. Not applied: retrieval 0.03 USD per GB; archive minimum 90 days.

## Repositories

| Repository | Images | Bytes | Untagged | Old | Estimated USD per month |
| --- | ---: | ---: | ---: | ---: | ---: |
| app | 2 | 2147483648 | 1 | 2 | 0.20 |
| **Total** | 2 | 2147483648 | 1 | | 0.20 |

## Untagged

| Repository | Digest | Bytes | Tags |
| --- | --- | ---: | ---: |
| app | sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb | 1073741824 | 0 |

## Old

As of 2026-11-09, older than 30 days.

| Repository | Digest | Age in days | Tags | Bytes |
| --- | --- | ---: | ---: | ---: |
| app | sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa | 1029 | 1 | 1073741824 |
| app | sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb | 2504 | 0 | 1073741824 |

## Lifecycle

Not executed.

```json
{
  "rules": [
    {
      "rulePriority": 1,
      "description": "Expire untagged images older than 14 days",
      "selection": {
        "tagStatus": "untagged",
        "countType": "sinceImagePushed",
        "countUnit": "days",
        "countNumber": 14
      },
      "action": {
        "type": "expire"
      }
    },
    {
      "rulePriority": 2,
      "description": "Keep the newest 10 tagged images",
      "selection": {
        "tagStatus": "tagged",
        "tagPatternList": [
          "*"
        ],
        "countType": "imageCountMoreThan",
        "countNumber": 10
      },
      "action": {
        "type": "expire"
      }
    }
  ]
}
```
````

## How it counts

Built-in rates are the US East (N. Virginia) figures on <https://aws.amazon.com/ecr/pricing/> as of 2026-11-09. The page fills its storage table from the public pricing map published 2026-09-11T12:44:25Z. ECR Standard, all storage, is 0.10 USD per GB-month. ECR Archive is 0.10 USD per GB-month for the first 150 TB and 0.07 USD per GB-month over 150 TB. The page's worked examples are 40 GB at 0.10 USD, which is 4 USD, and 20 GB in us-east-1, which is 2 USD. This tool treats 1 GB as 1,073,741,824 bytes and 1 TB as 1,024 GB, so the archive boundary is 150 times 1,024 GB. `--prices` replaces the whole quote.

C1. One `imageDetails` element is one image. The same digest in two files counts twice.
C2. 1 GB is 1,073,741,824 bytes. 40 GB of ECR Standard is 4.00 USD, and 20 GB is 2.00 USD, matching the worked examples on the pricing page.
C3. Standard storage is 0.10 USD per GB-month from https://aws.amazon.com/ecr/pricing/ as of 2026-11-09, region us-east-1.
C4. A missing `imageStatus` is ECR Standard, the default class on the pricing page.
C5. `ARCHIVED` bytes cost 0.10 USD per GB-month for the first 150 TB and 0.07 USD per GB-month after that. Retrieval at 0.03 USD per GB is not added.
C6. `ACTIVATING` and any other status are unpriced. They still count as images and bytes. The warning does not echo the status.
C7. An image with no `imageTags`, null, or an empty array is untagged.
C8. Without `--as-of`, storage is priced and the old list is omitted. The clock is not a default date for the bill; it is read only for the price-age warning (see Prices and responsibility).
C9. An image is old when its push date is more than `--older-than-days` before `--as-of`. A missing push time stays in the storage bill and is skipped for age. The warning does not echo the digest.
C10. The lifecycle block is JSON with an untagged `sinceImagePushed` expire rule and a tagged `imageCountMoreThan` expire rule. The words Not executed are outside that JSON.
C11. That JSON, and the lifecycle examples in the Amazon ECR User Guide, pass the schema check in this crate.
C12. `tagStatus` `tagged` needs `tagPatternList` or `tagPrefixList`, not both. Untagged and any omit both. A pattern may contain at most four `*` characters.
C13. `sinceImagePulled` is only valid with action `transition` to archive. `sinceImageTransitioned` requires `storageClass` `archive`, action `expire`, and a `countNumber` of at least 90.
C14. A rule with `tagStatus` `any` must have a higher `rulePriority` than every rule that is not `any`. `countNumber` must be a positive integer. `countUnit` `days` belongs only to the `sinceImage` count types.
C15. Each repository amount is rounded half away from zero to the nearest cent. The dollar total is the sum of the unrounded amounts, then rounded once to the cent. JSON money and byte totals are strings.
C16. The registry id is not printed.
C17. `--prices` replaces the built-in quote. An unknown JSON field is ignored. A duplicate key is an error, and the message is `duplicate key`.
C18. A bad `imageSizeInBytes` is reported without the value or the digest.

Archive dollars are tiered across the archived bytes in the input. When every archived byte is at one rate (the boundary is not crossed, or both rates are equal), each repository is priced from its own archived bytes. When the boundary is crossed and the two rates differ, each repository gets its proportional share of the tiered total. The 90-day archive minimum on the pricing page is not charged here, because `describe-images` does not say how a deletion would fall inside that window.

## Exit codes

| Code | Meaning |
| ---- | ------- |
| 0 | ok |
| 2 | an input or usage error: unreadable or invalid file, a file above 268435456 bytes, a value out of range, an unknown option, or standard output or an `--output` file that could not be written |
| 130 | interrupted by SIGINT (Ctrl-C), on Unix. Standard error is `ecr-bill: interrupted by SIGINT`, standard output is empty. A signal that arrives once the report is complete and being written is ignored, so a report is never cut in half |

## Limitations

Version 1.0.0 prices Amazon ECR only, and other registries are out of scope. The lifecycle line is not executed: this tool does not call PutLifecyclePolicy. There is no AWS SDK and no network call. A file above 268435456 bytes is refused. A symlink to a file is followed. A directory or any other file that is not a regular file is an error. A path that is replaced by a fifo between that check and the open is refused the same way, and the open does not wait for a writer. The built-in quote is us-east-1. Object retrieval is not in this storage bill, and the archive minimum of 90 days is not charged. The account free tier is not applied: the private 500 MB and the public 50 GB are not subtracted. The same digest counts twice when two files list it. Data transfer, managed signing, and encryption are not priced. The age list and the policy are not a forecast. A repository row stays on the cent. The dollar total is the sum of the unrounded amounts, then rounded once to the cent.

Input rules. A file is UTF-8 JSON with one value; a leading byte order mark is accepted, UTF-16 is refused (`not UTF-8` with a byte order mark, `not JSON` without one), and a second value after the first is `not JSON`. Nesting is at most 128 levels, and deeper input is the error `JSON nesting is deeper than 128 levels`. A duplicate key in an object is an error. `imageSizeInBytes` is a whole number from 0 to 18446744073709551615, and a byte total that does not fit in 64 bits is an error. `imageStatus` is read without regard to case and with ASCII spaces around it trimmed; an empty or blank status, and `null`, count as Standard like a missing one, without a warning. An `imageDigest` with a space or a tab is an error. A missing digest is shown as `(none)`; a digest that is `(none)` itself, or that starts with a quote, is shown inside quotes, so the two cannot be mistaken for each other. Control, line-break, and invisible characters in a value, a file name, or a command-line argument that a message repeats are written as `\u{hex}` in text and messages, and as a `\uXXXX` escape inside `--json` strings; the four price-file texts in `--json` (`source_url`, `source_date`, `region`, `region_name`) are written in the text form (`\u{202e}`, `\n`) first, so in the JSON they show with a doubled backslash (`\\u{202e}`). A usage error therefore stays on the lines clap prints. Errors about a whole file name the file; errors about one image say `file N: image M`.

`imagePushedAt` is either a JSON number of seconds since 1970-01-01 UTC, from 0 to 9223372036854775807 (a fraction is cut off and an exponent is allowed), or an ISO 8601 string `YYYY-MM-DDTHH:MM:SS` with an optional fraction and a zone, `Z` or `+HH:MM` or `-HH:MM` up to 23:59; `T` and `Z` may be lowercase. A missing or `null` value skips the image for age and gives a warning. Anything else is `invalid imagePushedAt`: a string of digits, a date without a time, a time without a zone, a space instead of `T`, a seconds field of 60, an hour of 25, an offset such as `+0100`, a negative number, or a number above that limit.

Price file. `--prices` reads a JSON file shaped like `prices/ecr-storage.json`: `source_url` (starting with `https://`), `source_date`, `region`, `region_name`, `usd_per_gb_month` with `standard`, `archive_first_150_tb` and `archive_over_150_tb`, `usd_per_gb_retrieval`, `archive_tier_tb` and `archive_min_days`. Each rate is a string of digits with at most one decimal point between digits, at most 40 characters, with ASCII spaces around it trimmed: `"1e2"`, `"-0.10"`, `".5"`, `"5."` and a number are `price file has a rate that is not a decimal`. A rate that the decimal type cannot hold exactly is refused, because it would round the rate and every price would then use another number. The type holds at most 28 digits (zeros before the first digit and after the last do not count, zeros right after the point do) and nothing above 79228162514264337593543950335: a rate that would be rounded is `price file has a rate with more than 28 digits`, and a rate above the largest value is `price file has a rate that is not a decimal`. A field that is absent is `price file is missing NAME`; a field that is present but of the wrong kind or out of range (`source_url` not starting with `https://`, a text field that is not a string, `archive_tier_tb` or `archive_min_days` that is not a whole number up to 4294967295) is `price file has an invalid NAME`. A file that is not a regular file is an error, as for an input file.

Exact amounts. Money is decimal, never floating point, and an amount is never rounded before the cent. A rate is held with at most 28 digits; the cost of a repository is its bytes times the rate divided by 1,073,741,824, a repository's share of a tiered archive amount is that amount times its archived bytes divided by all archived bytes, and every one of these is kept as an exact fraction in integer arithmetic (512 bits) and added exactly. The cent is taken once, at the end, half away from zero, for each row and for the total. So `99231259014473123` bytes at `0.894698637813` (exactly 82684748.12499999999999999999906...) print `82684748.12`, and `126819283` bytes at `9683085068513257837` print `1143665896371805327.83`. A rate of 79228162514264337593543950335 with two gibibytes prints `158456325028528675187087900670.00`. The error `cost does not fit` remains for a number that would not fit in 512 bits, which no input of a rate up to 28 digits and a byte total below 2^64 can reach. Checked against Python `fractions.Fraction` on 22,112 random inputs with rates of 1 to 22 digits, 0 to 12 decimals, one to four repositories and tiers of 1, 2 and 150 TB (rows and total): no difference; the version before this change gave 7 differences in 6,365 runs of the second generator (rates of 14 to 22 digits). Time and memory grow in line with the input: 100,000, 200,000 and 400,000 images took 0.35, 0.63 and 1.30 seconds and 88, 175 and 346 MB.

Ranges. `--keep` is 1 to 10000. `--older-than-days` and `--untagged-days` are 1 to 36500. `--as-of` is 0000-01-01 to 9999-12-31, as a real calendar date. A value outside a range is a usage error with exit code 2. A leading `+` is accepted in a number option (`--keep +5`), as Rust reads it.

Memory and size. The size limit bounds the input, not the time or the memory; the factors measured below are what a file under the limit can cost, and `SECURITY.md` counts only a panic, a hang beyond them, or growth beyond them as a vulnerability. A file above 268435456 bytes is refused, but the limit is per file, not per run, and the parsed JSON is larger than the file. Every file is read before any is parsed, so the memory of several files adds up. There is no single multiple of the file size: it depends on the shape. Measured with the release binary (maximum resident set, in KiB of 1024 bytes, divided by the file size) at 8388608, 16777216 and 33554432 bytes. One-element lists `{"imageDetails":[],"x":[[0],[0],...]}` took 0.900 s and 846392 KiB (103.32 times), 1.700 s and 1688608 KiB (103.06 times), and 3.500 s and 3374884 KiB (102.99 times). The same lists as `{"imageDetails":[[0],[0],...]}` took 0.880 s, 1.700 s and 3.490 s at 103.22, 103.05 and 103.02 times, and exit 2 (`not an object`). Tiny values `[1,1,...]` in that `x` array took 0.560 s and 434780 KiB (53.07 times), 1.110 s and 869688 KiB (53.08 times), and 2.470 s and 1737104 KiB (53.01 times). Lists nested 120 levels and repeated in that array took 1.130 s and 1247680 KiB (152.30 times), 2.240 s and 2494620 KiB (152.26 times), and 4.460 s and 4986792 KiB (152.18 times). A list of 100000, 200000 and 400000 images `{"repositoryName":"aa","imageSizeInBytes":1}` (4500018, 9000018 and 18000018 bytes) took 0.230 s and 77876 KiB (17.72 times), 0.430 s and 153160 KiB (17.43 times), and 0.880 s and 304452 KiB (17.32 times). Twice and four times the smallest file used 1.995 and 3.987 times the resident set for one-element lists, 1.999 and 3.997 for the 120-level lists, 2.000 and 3.995 for tiny values, and 1.967 and 3.909 for the image list. The highest ratio is the 120-level lists: 268435456 bytes of that shape would need 40883978240 bytes, from the 8388608-byte run, and that file was not run. A file of 268435456 NUL bytes took 0.190 s and 264616 KiB and is `not JSON`. One byte more is refused at once.

Platforms. Built and tested on Linux x86_64 with glibc (the binary needs glibc 2.34 or newer) and also built and run with the musl target. Windows and macOS were not run for this version, and no minimum version of either was determined. The check that turns a closed standard output (`>&-`) into exit code 2 exists on Linux only; on other systems it is not made, and that case was not measured. On Linux a full device (`>/dev/full`), a closed pipe, and a standard output that is open for reading only (`1<file`) also exit 2, with the message `ecr-bill: could not write output`. `>/dev/null` and `1<>/dev/null` exit 0.

## Prices and responsibility

The built-in prices are a snapshot of <https://aws.amazon.com/ecr/pricing/> as read on 2026-11-09. Check current prices yourself. Decisions based on this output are yours.

`--prices` supplies your own price file.

Old prices are shown, not hidden. When the `source_date` of the price file in use (the built-in one or the one from `--prices`) is more than 180 days before `--as-of`, or before today when `--as-of` is missing, one line goes to standard error and the exit code stays 0: `warning: prices are N days old (price date YYYY-MM-DD, as of YYYY-MM-DD); check current prices`. Exactly 180 days gives no warning. Standard output is the same with or without the warning (it is not in the `warnings` array). The clock is read only for this check, never to date the bill. A `source_date` that is not written `YYYY-MM-DD` (an older price file with free text there) gives no warning and no error, and neither does a price date after the as-of day.

## Maintenance

Report a bug through GitHub Issues on this repository. After a release this tool is dormant. A new version appears only for a reported bug, a request, or a security issue. Security always comes first.

## Support

Report a bug through GitHub Issues on this repository.

## License

MIT OR Apache-2.0. See [LICENSE-MIT](LICENSE-MIT) and [LICENSE-APACHE](LICENSE-APACHE).

The licenses of the third-party crates built into the binary are listed in [THIRD-PARTY-LICENSES.html](THIRD-PARTY-LICENSES.html).

## Contact

PEPAI, pep_ai@icloud.com, <https://www.pepai.nl>. X: <https://x.com/pepai_nl>. Ko-fi: <https://ko-fi.com/pepai>.
