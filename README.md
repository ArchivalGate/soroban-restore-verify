# soroban-restore-verify

[![CI](https://github.com/ArchivalGate/soroban-restore-verify/actions/workflows/ci.yml/badge.svg)](https://github.com/ArchivalGate/soroban-restore-verify/actions/workflows/ci.yml)
![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)

## Summary

`soroban-restore-verify` is an offline preflight verifier for Stellar Soroban `RestoreFootprintOp`, `ExtendFootprintTtlOp`, and archival `InvokeHostFunctionOp` transaction envelopes. It checks their XDR against [CAP-0046-12](https://github.com/stellar/stellar-protocol/blob/master/core/cap-0046-12.md) and [CAP-0066](https://github.com/stellar/stellar-protocol/blob/master/core/cap-0066.md) protocol invariants and reports local results without contacting Stellar RPC.

## Why This Tool Exists

In production, `RestoreFootprintMalformed` and `ExtendFootprintTTLMalformed` can result from envelope details that are easy to miss in a manual `stellar tx decode` review: unsorted `LedgerKey` vectors, `Temporary` keys in a restore footprint, classic keys in a Soroban footprint, `extend_to` greater than `max_entry_ttl - 1`, or an attempt to extend an archived persistent key before restoring it. This CLI checks those conditions before a transaction is handed off for signing or submission.

## Quick Start

Build the release binary, verify a valid restore, check a protocol violation, then compare an extension against the included caller snapshot. The commands print each process exit code, including the expected `2` for the violation.

```bash
cargo build --release --locked

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

## Example Output

The following reports are from the committed fixtures and release CLI.

### `valid-restore.xdr`, human format, exit 0

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

### `valid-restore.xdr`, JSON format, exit 0

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

### `invalid-restore-temp.xdr`, human format, exit 2

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

The CLI also writes this diagnostic to stderr:

```json
{"code":"VERIFICATION_VIOLATION","message":"one or more policy invariants failed"}
```

### `invalid-restore-temp.xdr`, JSON format, exit 2

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

## Exit Codes & Input Safety

| Result | Behavior |
| --- | --- |
| Exit `0` | The decoded envelope passes the selected checks. Report status is `VERIFIED_LOCAL`. |
| Exit `2` | A decoded envelope or snapshot check violates a verifier invariant. Report status is `VIOLATION`. |
| Exit `3` | Input, XDR, snapshot, path, output, or CLI argument error. A verification report is not produced for malformed or unsafe input. |
| Input limit | Files, stdin, base64 XDR arguments, and snapshots are limited to 1 MiB. |
| Symlinks | Symlink and non-regular input, snapshot, and output paths are rejected. |
| `--output` | Atomically creates the report path with exclusive file creation, refuses existing paths, and removes a partial file if writing fails. |

## Scope & Limitations

- This is an offline XDR verifier and makes no RPC calls.
- It does not hold keys, sign transactions, or submit transactions.
- Snapshot checks use caller-provided JSON and do not represent live ledger state.
- `VERIFIED_LOCAL` means the supplied envelope passed these checks; it does not prove network acceptance or execution.

## Documentation & Repository Layout

| Document | Contents |
| --- | --- |
| [Architecture](docs/ARCHITECTURE.md) | Data flow, authority boundaries, CLI behavior, snapshot and report schemas. |
| [Toolchain matrix](docs/TOOLCHAIN-MATRIX.md) | Verified Rust and dependency versions, XDR fields, input authority. |
| [Project profile](docs/PROJECT-PROFILE.md) | Project purpose, users, inputs, outputs, and scope. |
| [Experiments](docs/EXPERIMENTS.md) | XDR decoding, canonical key ordering, and file-safety evidence. |
| [Claim ledger](docs/CLAIM-LEDGER.md) | Evidence and limits for repository claims. |
| [Final audit](docs/FINAL-AUDIT.md) | Audit findings and local and CI verification results. |
| [Security](SECURITY.md) | Trust boundaries, input and output handling, vulnerability reporting. |
