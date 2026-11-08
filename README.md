<div align="center">
  <img src="assets/logo.svg" alt="soroban-restore-verify" width="96" height="96" />

  # soroban-restore-verify

  **Verify first. Sign and broadcast only valid archival XDRs.**<br />
  Deterministic offline CAP-0046-12 and CAP-0066 transaction envelope verification for Stellar Soroban.

  [Quick Start](#quick-start) · [Architecture](#verification-pipeline) · [Example Output](#example-output) · [CLI Options](#cli-flags--options) · [Violation Codes](#verified-protocol-invariants--violation-codes) · [Security](SECURITY.md) · [Final Audit](docs/FINAL-AUDIT.md) · [Contributing](CONTRIBUTING.md)

  [![CI](https://github.com/ArchivalGate/soroban-restore-verify/actions/workflows/ci.yml/badge.svg)](https://github.com/ArchivalGate/soroban-restore-verify/actions/workflows/ci.yml)
  [![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
</div>

---

## Overview & Why This Tool Exists

`soroban-restore-verify` is an offline preflight verifier, available as a CLI and Rust library, for Stellar Soroban `RestoreFootprintOp`, `ExtendFootprintTtlOp`, and archival `InvokeHostFunctionOp` transaction envelopes. It checks the supplied XDR against [CAP-0046-12](https://github.com/stellar/stellar-protocol/blob/master/core/cap-0046-12.md) and [CAP-0066](https://github.com/stellar/stellar-protocol/blob/master/core/cap-0066.md) invariants without connecting to Stellar RPC.

In production, `RestoreFootprintMalformed` and `ExtendFootprintTTLMalformed` can result when operators rely on manual `stellar tx decode` inspection. Decoding can leave mistakes unnoticed, including unsorted `LedgerKey` vectors, `Temporary` keys in a restore footprint, classic keys in a Soroban footprint, `extend_to` greater than `max_entry_ttl - 1`, non-zero instructions on archival operations, or an attempt to extend an archived persistent key before restoring it. A supplied caller snapshot lets the verifier check whether a persistent extension target is already archived.

## Verification Pipeline

The CLI processes local input through four stages:

```text
[Input Guard: <= 1 MiB, no symlinks]
  ->
[Bounded XDR Decoder: depth <= 64, TxV1 or FeeBump]
  ->
[CAP-0046-12 & CAP-0066 Invariant Engine + Optional Snapshot Policy]
  ->
[Deterministic Report: Exit 0 / 2 / 3]
```

`AUTHORITATIVE_PROTOCOL_XDR` applies to transaction structure, footprint key types and durability, canonical key ordering, and declared resource bounds in the envelope. `CALLER_SNAPSHOT` applies only to optional local JSON entry-state checks and is not live ledger state.

## Quick Start

```bash
cargo build --release --locked

# Optional: install the CLI into Cargo's binary directory.
cargo install --path . --locked

./target/release/soroban-restore-verify --input fixtures/valid-restore.xdr
printf 'exit code: %s\n' "$?"

./target/release/soroban-restore-verify --input fixtures/invalid-restore-temp.xdr
printf 'exit code: %s\n' "$?"

./target/release/soroban-restore-verify \
  --input fixtures/valid-extend.xdr \
  --snapshot fixtures/sample-snapshot.json \
  --current-ledger 100
printf 'exit code: %s\n' "$?"
```

The valid restore exits `0`, the temporary-key restore exits `2`, and the snapshot cross-check exits `0`.

## Example Output

These outputs are from the committed fixtures and release CLI.

#### 1. Valid Restore Envelope (Human Format, Exit 0)

Command: `./target/release/soroban-restore-verify --input fixtures/valid-restore.xdr --format human`

```text
Status: VERIFIED_LOCAL
Authority: AUTHORITATIVE_PROTOCOL_XDR
Operation: RestoreFootprint
Fee bump: no
Operation count: 1
Maximum entry TTL: 3110400
Snapshot: not supplied
Keys: 1
  read_write[0] ContractData/Persistent sha256 6e0233b0a9b1ccf457b070e104fc7417141c087723bd5d849f08c6ffb8c036e3
Violations: 0
Warnings: 0
```

#### 2. Valid Restore Envelope (JSON Format, Exit 0)

Command: `./target/release/soroban-restore-verify --input fixtures/valid-restore.xdr --format json`

```json
{
  "schema": "soroban-restore-verify/1.0",
  "status": "VERIFIED_LOCAL",
  "authority": "AUTHORITATIVE_PROTOCOL_XDR",
  "operation": "RestoreFootprint",
  "is_fee_bump": false,
  "operation_count": 1,
  "max_entry_ttl": 3110400,
  "snapshot": null,
  "keys": [
    {
      "section": "read_write",
      "index": 0,
      "key_type": "ContractData",
      "durability": "Persistent",
      "key_xdr_base64": "AAAABgAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAE=",
      "key_sha256": "6e0233b0a9b1ccf457b070e104fc7417141c087723bd5d849f08c6ffb8c036e3"
    }
  ],
  "violations": [],
  "warnings": []
}
```

#### 3. Invalid Restore Envelope (Human Format and stderr Diagnostic, Exit 2)

Command: `./target/release/soroban-restore-verify --input fixtures/invalid-restore-temp.xdr --format human`

```text
Status: VIOLATION
Authority: AUTHORITATIVE_PROTOCOL_XDR
Operation: RestoreFootprint
Fee bump: no
Operation count: 1
Maximum entry TTL: 3110400
Snapshot: not supplied
Keys: 1
  read_write[0] ContractData/Temporary sha256 9ccf3a0b9f7fa00938e5c5cb1772271ea50fff6dab95f780c079d518bf8e6cbb
Violations: 1
  RESTORE_FORBIDDEN_TEMPORARY_KEY: RestoreFootprintOp cannot target temporary ContractData operation_index 0 key_sha256 9ccf3a0b9f7fa00938e5c5cb1772271ea50fff6dab95f780c079d518bf8e6cbb
Warnings: 0
```

stderr:

```json
{"code":"VERIFICATION_VIOLATION","message":"one or more policy invariants failed"}
```

#### 4. Invalid Restore Envelope (JSON Format, Exit 2)

Command: `./target/release/soroban-restore-verify --input fixtures/invalid-restore-temp.xdr --format json`

```json
{
  "schema": "soroban-restore-verify/1.0",
  "status": "VIOLATION",
  "authority": "AUTHORITATIVE_PROTOCOL_XDR",
  "operation": "RestoreFootprint",
  "is_fee_bump": false,
  "operation_count": 1,
  "max_entry_ttl": 3110400,
  "snapshot": null,
  "keys": [
    {
      "section": "read_write",
      "index": 0,
      "key_type": "ContractData",
      "durability": "Temporary",
      "key_xdr_base64": "AAAABgAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=",
      "key_sha256": "9ccf3a0b9f7fa00938e5c5cb1772271ea50fff6dab95f780c079d518bf8e6cbb"
    }
  ],
  "violations": [
    {
      "code": "RESTORE_FORBIDDEN_TEMPORARY_KEY",
      "message": "RestoreFootprintOp cannot target temporary ContractData",
      "operation_index": 0,
      "key_sha256": "9ccf3a0b9f7fa00938e5c5cb1772271ea50fff6dab95f780c079d518bf8e6cbb"
    }
  ],
  "warnings": []
}
```

## CLI Flags & Options

| Flag | Default | Description |
| --- | --- | --- |
| `-i, --input <PATH>` | None | Read base64 XDR from a regular file, or from stdin when `PATH` is `-`. Provide exactly one of `--input` or `--xdr`. |
| `--raw-xdr` | `false` | Read `--input` or stdin as raw binary XDR. Requires `--input` and cannot be combined with `--xdr`. |
| `--xdr <BASE64>` | None | Read base64 XDR from an argument. It cannot be combined with `--input` or `--raw-xdr`. |
| `--snapshot <PATH>` | None | Read a caller-supplied snapshot JSON file. |
| `--current-ledger <U32>` | None | Current ledger number for snapshot comparisons. Required with `--snapshot` and must be greater than zero. |
| `--max-entry-ttl <U32>` | `3110400` | Maximum entry TTL used to check `extend_to`. |
| `--require-complete-snapshot` | `false` | Treat a footprint key missing from the snapshot as a violation. Requires `--snapshot`. |
| `--format <FORMAT>` | `human` | Select `json` or `human` output. Possible values: `json`, `human`. |
| `-o, --output <PATH>` | None | Write the report to a new file. Existing paths and symlinks are refused. |
| `--dump-fixture <NAME>` | None | Write one fixture to stdout and exit. Possible values: `valid-restore`, `valid-extend`, `invalid-restore-temp`, `invalid-extend-unsorted`, `sample-snapshot`. |
| `-h, --help` | None | Print help. |
| `-V, --version` | None | Print version. |

## Verified Protocol Invariants & Violation Codes

The verifier returns these stable codes for the checks implemented in the envelope verifier and caller-snapshot policy. Snapshot results describe the supplied JSON only.

| Code | Category | Condition Enforced |
| --- | --- | --- |
| `ENVELOPE_V0_UNSUPPORTED_FOR_SOROBAN` | Envelope | Reject a legacy V0 transaction envelope. |
| `EMPTY_OPERATIONS` | Envelope | Require at least one transaction operation. |
| `OP_NOT_ISOLATED` | Envelope | Require one operation per archival or Soroban host transaction. |
| `UNSUPPORTED_OPERATION_TYPE` | Envelope | Accept only supported restore, TTL extension, and host-function operations. |
| `MISSING_SOROBAN_TRANSACTION_DATA` | Envelope | Require Soroban transaction data on the transaction. |
| `FOOTPRINT_DUPLICATE_KEY` | Footprint | Reject duplicate keys within a footprint section or across read-only and read-write sections. |
| `FOOTPRINT_UNSORTED` | Footprint | Require each footprint vector to be strictly sorted by canonical XDR bytes. |
| `KEY_XDR_ENCODING_FAILED` | Footprint | Require each ledger key to encode as canonical XDR for comparison and reporting. |
| `RESTORE_NON_EMPTY_READ_ONLY` | RestoreFootprintOp | Require an empty read-only footprint. |
| `RESTORE_EMPTY_READ_WRITE` | RestoreFootprintOp | Require at least one read-write key. |
| `RESTORE_FORBIDDEN_TEMPORARY_KEY` | RestoreFootprintOp | Reject temporary ContractData keys. |
| `RESTORE_FORBIDDEN_CLASSIC_KEY` | RestoreFootprintOp | Restrict targets to Soroban ContractData or ContractCode keys. |
| `RESTORE_NON_ZERO_INSTRUCTIONS` | RestoreFootprintOp | Require `instructions` to be zero. |
| `RESTORE_ZERO_WRITE_BYTES` | RestoreFootprintOp | Require `write_bytes` greater than zero. |
| `RESTORE_ZERO_DISK_READ_BYTES` | RestoreFootprintOp | Require `disk_read_bytes` greater than zero. |
| `RESTORE_NON_POSITIVE_RESOURCE_FEE` | RestoreFootprintOp | Require a positive `resource_fee`. |
| `RESTORE_FORBIDDEN_ARCHIVED_ENTRIES_EXT` | RestoreFootprintOp | Require an empty `archived_soroban_entries` extension. |
| `EXTEND_NON_EMPTY_READ_WRITE` | ExtendFootprintTtlOp | Require an empty read-write footprint. |
| `EXTEND_EMPTY_READ_ONLY` | ExtendFootprintTtlOp | Require at least one read-only key. |
| `EXTEND_FORBIDDEN_CLASSIC_KEY` | ExtendFootprintTtlOp | Restrict targets to Soroban ContractData or ContractCode keys. |
| `EXTEND_TTL_ZERO` | ExtendFootprintTtlOp | Require `extend_to` greater than zero. |
| `EXTEND_TTL_EXCEEDS_MAX` | ExtendFootprintTtlOp | Require `extend_to` to be at most `max_entry_ttl - 1`. |
| `EXTEND_NON_ZERO_INSTRUCTIONS` | ExtendFootprintTtlOp | Require `instructions` to be zero. |
| `EXTEND_NON_ZERO_WRITE_BYTES` | ExtendFootprintTtlOp | Require `write_bytes` to be zero. |
| `EXTEND_NON_POSITIVE_RESOURCE_FEE` | ExtendFootprintTtlOp | Require a positive `resource_fee`. |
| `EXTEND_FORBIDDEN_ARCHIVED_ENTRIES_EXT` | ExtendFootprintTtlOp | Require an empty `archived_soroban_entries` extension. |
| `INVOKE_FORBIDDEN_CLASSIC_KEY` | InvokeHostFunctionOp | Reject classic keys in the Soroban footprint. |
| `INVOKE_ARCHIVED_INDEX_OUT_OF_BOUNDS` | InvokeHostFunctionOp | Require each archived-entry index to address a read-write footprint key. |
| `INVOKE_ARCHIVED_INDEX_UNSORTED` | InvokeHostFunctionOp | Require archived-entry indexes to be strictly sorted. |
| `INVOKE_ARCHIVED_INDEX_DUPLICATE` | InvokeHostFunctionOp | Require archived-entry indexes to be unique. |
| `INVOKE_ARCHIVED_KEY_NOT_PERSISTENT` | InvokeHostFunctionOp | Require archived indexes to point to persistent ContractData or ContractCode keys. |
| `SNAPSHOT_RESTORE_ALREADY_LIVE_KEY` | Snapshot | Reject a restore target that the snapshot marks live at the supplied current ledger. |
| `SNAPSHOT_EXTEND_ARCHIVED_PERSISTENT_KEY` | Snapshot | Require an archived persistent extension target to be restored first. |
| `SNAPSHOT_REDUNDANT_TTL_EXTENSION` | Snapshot | Reject an extension already covered by the snapshot's live-until ledger. |
| `SNAPSHOT_MISSING_KEY_STATE` | Snapshot | Require every footprint key to have a snapshot entry when complete coverage is requested. |

## Caller Snapshot Format (`--snapshot`)

The snapshot file is JSON with schema `soroban-restore-verify/snapshot/1.0`. Each entry identifies a canonical `LedgerKey` by base64 XDR, its SHA-256 digest, or both, and supplies `live_until_ledger_seq`.

```json
{
  "schema": "soroban-restore-verify/snapshot/1.0",
  "entries": [
    {
      "key_xdr_base64": "AAAABgAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAE=",
      "key_sha256": "6e0233b0a9b1ccf457b070e104fc7417141c087723bd5d849f08c6ffb8c036e3",
      "live_until_ledger_seq": 150
    }
  ]
}
```

Pass `--current-ledger` with `--snapshot` to compare each footprint key's live-until ledger with the requested operation. Add `--require-complete-snapshot` to report missing key state as `SNAPSHOT_MISSING_KEY_STATE`; otherwise, a missing key is reported as inconclusive.

## Library Usage (Rust Crate)

Add this crate as a path dependency in the consuming Rust tool or keeper service. Its Rust library name is `soroban_restore_verify`. Decode and verify an envelope in memory with the same public functions used by the CLI:

```rust
use soroban_restore_verify::{
    model::Report,
    verifier::{VerifyOptions, verify_envelope},
    xdr_decoder::{DecodeError, InputEncoding, decode_envelope},
};

pub fn preflight(base64_xdr: &[u8]) -> Result<Report, DecodeError> {
    let envelope = decode_envelope(base64_xdr, InputEncoding::Base64)?;
    Ok(verify_envelope(&envelope, VerifyOptions::default(), None))
}
```

The returned `Report` contains `VERIFIED_LOCAL` or `VIOLATION`, the affected keys, and any violation codes. Use `InputEncoding::RawXdr` for raw XDR bytes. A caller can pass a parsed `SnapshotIndex` instead of `None` when local snapshot checks are required.

Run the [library consumer example](examples/verify_envelope.rs) from the repository root:

```sh
cargo run --example verify_envelope --locked
```

The example embeds the valid and invalid restore XDR fixtures at compile time and verifies both in memory. It imports `ReportStatus` as `Status`, checks for `Status::VerifiedLocal` and `Status::Violation`, and returns errors for decoding failures or unexpected results without panicking. The invalid fixture targets temporary ContractData, so its expected result is a policy violation.

## Exit Codes & Input Safety

| Result | Behavior |
| --- | --- |
| Exit `0` | The decoded envelope passes the selected checks. Report status is `VERIFIED_LOCAL`. |
| Exit `2` | A decoded envelope or snapshot check violates a verifier invariant. Report status is `VIOLATION`. |
| Exit `3` | Input, XDR, snapshot, path, output, or CLI argument error. A verification report is not produced for malformed or unsafe input. |
| Input limit | Files, stdin, base64 XDR arguments, and snapshots are limited to 1 MiB. |
| Symlinks | Symlink and non-regular input, snapshot, and output paths are rejected. |
| `--output` | Atomically creates the report path with exclusive file creation, refuses existing paths, and removes a partial file if writing fails. |

## Ecosystem Comparison

| Tool | Focus | Difference |
| --- | --- | --- |
| [stellar-cli](https://github.com/stellar/stellar-cli) | `stellar tx decode` inspects XDR; `stellar contract restore` prepares a restore workflow. | General Stellar CLI commands. This tool checks archival transaction invariants in a supplied envelope without RPC. |
| [stellar-api-inspector](https://github.com/Dantama022/stellar-api-inspector) | Node CLI for Horizon and Soroban RPC health, accounts, contracts, transactions, and operations. | Endpoint and network inspection; this verifier reads local XDR and does not connect to endpoints. |
| [soroban-cost-linter](https://github.com/Tollcraft/soroban-cost-linter) | Rust and Dylint static analysis for input-independent Soroban contract resource-cost patterns. | Analyzes contract source; this verifier checks serialized transaction footprints and TTL fields. |
| [Soroban State Lens](https://github.com/Vynix-Labs/Soroban-state-lens) | Browser-based Soroban state explorer and visual debugger. | Interactive state inspection; this verifier produces a local report for a transaction envelope. |

## Test Suite & Reproducible Verification

Run the formatting, lint, and test checks from the repository root:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --all-targets --locked
```

The test command runs 21 tests: 17 CLI integration tests and 4 Phase C boundary tests.

After building the release CLI, reproduce and compare all five generated fixtures byte for byte:

```bash
cargo build --release --locked
tmpdir=$(mktemp -d)
trap 'rm -rf "$tmpdir"' EXIT

for name in valid-restore valid-extend invalid-restore-temp invalid-extend-unsorted; do
  ./target/release/soroban-restore-verify --dump-fixture "$name" > "$tmpdir/$name.xdr"
  cmp "fixtures/$name.xdr" "$tmpdir/$name.xdr"
done

./target/release/soroban-restore-verify --dump-fixture sample-snapshot > "$tmpdir/sample-snapshot.json"
cmp fixtures/sample-snapshot.json "$tmpdir/sample-snapshot.json"
```

## Scope & Limitations

- Offline XDR verification only. The CLI makes no RPC calls.
- No key custody, transaction signing, or transaction submission.
- Snapshot checks evaluate caller-supplied JSON and do not read live RPC state.
- `VERIFIED_LOCAL` means the supplied envelope passed these local checks. It does not prove network acceptance or execution.

## Modular Architecture Map

Use these module seams to keep contributor changes focused and parallel PRs in separate files where possible.

| Source module | Responsibility | Contributor seam |
| --- | --- | --- |
| [`src/main.rs`](src/main.rs) | CLI argument parsing with `clap`, exit-code routing (`0`, `2`, `3`), and `stdout`/`stderr` stream separation. | CLI flags and orchestration. |
| [`src/lib.rs`](src/lib.rs) | Public library entry point exporting verification, snapshot, I/O safety, decoding, fixture, and report interfaces. | Public module exports. |
| [`src/model.rs`](src/model.rs) | Domain data structures, report and snapshot-summary schemas, exit-code constants, and all 35 `ViolationCode` definitions. | Shared types and stable serialized contracts. |
| [`src/io_safety.rs`](src/io_safety.rs) | Bounded 1 MiB file/stdin readers, `symlink_metadata` path guards, and file output with atomic `create_new(true)` creation. | Input adapters and filesystem safety. |
| [`src/verifier.rs`](src/verifier.rs) | Pure CAP-0046-12 and CAP-0066 invariant rules for `RestoreFootprintOp`, `ExtendFootprintTtlOp`, and `InvokeHostFunctionOp`. | Protocol rules, verification options, and TTL bounds. |
| [`src/snapshot.rs`](src/snapshot.rs) | Caller-supplied JSON snapshot schema and parser with `deny_unknown_fields`, plus SHA-256 / base64 `LedgerKey` indexing. | Snapshot validation and key lookup. |
| [`src/report.rs`](src/report.rs) | Deterministic human-readable and JSON report formatters. | Output formats. |
| [`src/fixtures.rs`](src/fixtures.rs) | Deterministic XDR envelope and snapshot builders backing `--dump-fixture` and regression tests. | Reusable test fixtures. |
| [`src/xdr_decoder.rs`](src/xdr_decoder.rs) | Bounded base64 and raw `TransactionEnvelope` XDR decoding with typed errors. | Envelope decoding and input encodings. |

Configuration bounds live with their enforcement: the 1 MiB input limit is in `io_safety.rs`, XDR depth limits are in the decoding modules, and `VerifyOptions` and TTL bounds are in `verifier.rs`.

## Documentation & Repository Layout

| Document | Contents |
| --- | --- |
| [Architecture](docs/ARCHITECTURE.md) | Data flow, authority boundaries, CLI behavior, and report schemas. |
| [Toolchain matrix](docs/TOOLCHAIN-MATRIX.md) | Verified Rust and dependency versions, XDR fields, and input authority. |
| [Project profile](docs/PROJECT-PROFILE.md) | Project purpose, users, supported inputs, and scope. |
| [Experiments](docs/EXPERIMENTS.md) | XDR decoding, key ordering, and file-safety evidence. |
| [Claim ledger](docs/CLAIM-LEDGER.md) | Evidence and limits for repository claims. |
| [Final audit](docs/FINAL-AUDIT.md) | Audit findings and local and CI verification results. |
| [Security](SECURITY.md) | Trust boundaries, input and output handling, vulnerability reporting. |
| [MIT License](LICENSE) | License text and terms. |
| [Contributing](CONTRIBUTING.md) | Module ownership, engineering rules, and local pre-PR checks. |
| [Report schema](docs/REPORT-SCHEMA.json) | Machine-readable JSON report schema. |
| [src/](src/) | Source modules listed in the architecture map above. |
| [Library example](examples/verify_envelope.rs) | Runnable Rust consumer verifying both valid and invalid restore XDR in memory. |
| [tests/](tests/) | CLI integration and experiment regression tests. |
| [fixtures/](fixtures/) | Committed XDR envelopes and caller snapshot data. |
