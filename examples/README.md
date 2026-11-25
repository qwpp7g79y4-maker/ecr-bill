# Examples

`describe-images.json` is a synthetic `aws ecr describe-images` listing: two Standard images of 1 GiB in repository `app`, one tagged `v1` (pushed 2024-01-15) and one untagged (pushed 2020-01-01). The account id is invented.

`expected-output.txt` is what this prints, byte for byte:

```sh
ecr-bill --as-of 2026-11-09 examples/describe-images.json
```

The command reads the file and writes the report to standard output. It writes nothing else and uses no network. A test compares the output with `expected-output.txt`.
