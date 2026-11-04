# Security

## Scope and trust boundaries

The CLI accepts untrusted XDR bytes, command-line values, file paths, and optional caller-supplied snapshot JSON. It does not connect to Stellar RPC, sign transactions, or submit transactions.

The verifier checks that the decoded envelope meets its local protocol and policy checks. A `VERIFIED_LOCAL` report does not establish that a transaction was accepted, applied, or valid against the current ledger. Snapshot state comes from the caller and is not live chain data.

## Input and output handling

- Files and stdin are limited to 1,048,576 bytes. File metadata is checked and reads are capped at 1,048,577 bytes to catch growth during reading.
- XDR decoding uses depth 64 and a 1 MiB byte limit.
- Symlink and non-regular input, snapshot, and output paths are refused.
- `--output` uses exclusive file creation and does not replace an existing path.
- Snapshot JSON rejects unknown fields, duplicate ledger keys, invalid key digests, and inconsistent key identifiers.
- Diagnostics go to stderr. Reports and fixture data go to stdout or to the selected new output file.

## Reporting a vulnerability

Use GitHub Security Advisories for this repository if private vulnerability reporting is enabled. If it is not available, contact the repository owner through GitHub before sharing sensitive details publicly.

## Maintenance posture

Dependency versions are pinned in `Cargo.toml` and `Cargo.lock`. The CI workflow has read-only repository permissions and runs formatting, lint, test, and fixture smoke checks. External contributor merges remain deferred until after MVP.
