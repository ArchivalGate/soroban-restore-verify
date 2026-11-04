# Architecture and authority contract

Status: `IMPLEMENTED`

## Data flow

```text
untrusted CLI input
  --input PATH or stdin, --xdr BASE64, --raw-xdr
  --snapshot PATH
  --output PATH
        |
        v
io_safety
  reject symlinks and non-regular files
  check metadata size, then read at most 1 MiB + 1 byte
  reject existing output before report generation
        |
        v
xdr_decoder
  decode TransactionEnvelope with depth 64 and byte limit 1 MiB
  classify malformed XDR as an input error
        |
        v
verifier <------ snapshot engine
  check envelope, operation, footprint, resources, TTL policy
  compare optional caller JSON state with the supplied ledger number
        |
        v
report
  stable JSON or human text to stdout
  or create the requested output path with create_new(true)
```

`io_safety`, `xdr_decoder`, `verifier`, `snapshot`, and `report` are library modules. `main` parses the CLI and maps errors to exit codes. The CLI makes no RPC calls.

## Authority

| Data | Authority | Meaning |
| --- | --- | --- |
| Decoded `TransactionEnvelope` XDR | `AUTHORITATIVE_PROTOCOL_XDR` | Authoritative for the operation, declared footprint, Soroban resource fields, and encoded keys present in that envelope. A clean result is `VERIFIED_LOCAL`; it does not prove transaction acceptance, execution, or live ledger state. |
| Optional snapshot JSON | `CALLER_SNAPSHOT` | Caller-supplied ledger entry live-until-ledger states. It is not on-chain truth. Reports label its use and the supplied current ledger. |
| Missing key in optional snapshot | `CALLER_SNAPSHOT` | Report `INCONCLUSIVE_MISSING_FROM_SNAPSHOT`. With `--require-complete-snapshot`, emit `SNAPSHOT_MISSING_KEY_STATE` and return `VIOLATION`. |
| Output file | Local filesystem | Created only when absent. Existing files, symlinks, and non-regular output paths are errors. |

## CLI specification

Program name: `soroban-restore-verify`

| Option | Type | Behavior |
| --- | --- | --- |
| `--input <PATH>`, `-i <PATH>` | Path or `-` | Read base64 text from a regular file or stdin. Stdin is used when the path is `-`. |
| `--xdr <BASE64>` | String | Decode the base64 envelope argument directly. It cannot be combined with `--input` or `--raw-xdr`. |
| `--raw-xdr` | Flag | Treat `--input` or stdin bytes as raw binary XDR. It cannot be combined with `--xdr`. |
| `--snapshot <PATH>` | Path | Read optional caller-supplied JSON snapshot. |
| `--current-ledger <u32>` | Integer | Required with `--snapshot`; must be greater than zero. |
| `--max-entry-ttl <u32>` | Integer | Protocol maximum entry TTL. Default `3110400`; maximum `extend_to` is `max_entry_ttl.saturating_sub(1)`, which is `3110399` at the default. |
| `--require-complete-snapshot` | Flag | Turn any footprint key absent from the supplied snapshot into `SNAPSHOT_MISSING_KEY_STATE`. Requires `--snapshot`. |
| `--format <json\|human>` | Enum | Select report format. Default is `human`. |
| `--output <PATH>`, `-o <PATH>` | Path | Write the report to a new path. Refuse existing paths, symlinks, and non-regular existing paths. |
| `--dump-fixture <NAME>` | Enum | Write a deterministic fixture to stdout and exit. Names: `valid-restore`, `valid-extend`, `invalid-restore-temp`, `invalid-extend-unsorted`, `sample-snapshot`. Transaction fixtures are base64 XDR with a trailing newline; `sample-snapshot` is JSON. This option cannot be combined with verification inputs or `--output`. |

Input and snapshot files must be regular files and must not be symlinks. The input byte limit is 1,048,576 bytes for files and stdin. The reader is capped at 1,048,577 bytes after checking metadata, so a file that grows during reading is still rejected. XDR decoding uses `Limits { depth: 64, len: 1_048_576 }`.

### Exit codes and streams

| Code | Meaning | Streams |
| --- | --- | --- |
| `0` | Decoded envelope and selected policy checks have no violations. Report status is `VERIFIED_LOCAL`. | Report to stdout or `--output`; diagnostics are empty. |
| `2` | Decoded envelope or caller-snapshot policy violation. Report status is `VIOLATION`. | Structured or human report to stdout or `--output`; concise error summary to stderr. |
| `3` | Malformed/truncated XDR, invalid snapshot JSON, unsafe or over-limit path/input, output creation failure, unsupported encoding, or invalid CLI flags. | Diagnostic to stderr. No verification report is emitted for undecodable or unsafe input. |

Stdout is reserved for report or fixture data. Stderr is reserved for diagnostics and error summaries.

## Snapshot JSON

The top-level object has `schema` set to `soroban-restore-verify/snapshot/1.0` and an `entries` array. Each entry has an integer `live_until_ledger_seq` and at least one of `key_xdr_base64` or `key_sha256`.

```json
{
  "schema": "soroban-restore-verify/snapshot/1.0",
  "entries": [
    {
      "key_xdr_base64": "AAAA...",
      "key_sha256": "lowercase-sha256-of-canonical-key-xdr",
      "live_until_ledger_seq": 500000
    }
  ]
}
```

`key_sha256` is SHA-256 over canonical binary XDR bytes for one `LedgerKey`. If both key forms are present, they must decode to the same canonical bytes and digest. Duplicate canonical keys are invalid. Unknown JSON fields are rejected. `--current-ledger` is mandatory with a snapshot and must be positive.

Snapshot comparisons follow the project policy:

- A restore key with `live_until_ledger_seq >= current_ledger` emits `SNAPSHOT_RESTORE_ALREADY_LIVE_KEY`.
- An extend target that is persistent or ContractCode with `live_until_ledger_seq < current_ledger` emits `SNAPSHOT_EXTEND_ARCHIVED_PERSISTENT_KEY`.
- An extend target with `live_until_ledger_seq >= current_ledger.saturating_add(extend_to)` emits `SNAPSHOT_REDUNDANT_TTL_EXTENSION`.
- A missing entry is inconclusive unless complete snapshot coverage is required.

## Protocol and local policy checks

The verifier accepts only `TransactionEnvelope::Tx` or fee-bump envelopes containing a supported inner transaction. It rejects V0 transaction envelopes, empty operation lists, non-isolated transactions, unsupported operation kinds, and missing `TransactionExt::V1(SorobanTransactionData)`.

For each footprint section, keys are encoded as canonical XDR and checked for strict ascending order and duplicates. Cross-section duplicates are also violations. Restore targets must be non-empty persistent ContractData or ContractCode keys, with an empty read-only section. Extend targets must be non-empty ContractData or ContractCode read-only keys, with an empty read-write section. Classic keys and temporary restore targets are rejected. The declared resource fields and archive extension indexes are checked according to the policy codes listed in the implementation and claim ledger.

The protocol source for archival operation requirements is [CAP-0046-12](https://github.com/stellar/stellar-protocol/blob/master/core/cap-0046-12.md). [CAP-0066](https://github.com/stellar/stellar-protocol/blob/master/core/cap-0066.md) changes the resource field from `readBytes` to `diskReadBytes` and defines archived Soroban footprint indexes. The resolved XDR crate confirms the corresponding Rust field and extension variants. Snapshot comparisons and additional fail-closed checks in this tool are local policy; they cannot establish ledger state.

## JSON report schema

Schema identifier: `soroban-restore-verify/1.0`

The machine-readable schema is in [`REPORT-SCHEMA.json`](REPORT-SCHEMA.json).

The JSON report is an object with these fields:

| Field | Type | Meaning |
| --- | --- | --- |
| `schema` | string | Always `soroban-restore-verify/1.0`. |
| `status` | string | `VERIFIED_LOCAL` or `VIOLATION`. |
| `authority` | string | `AUTHORITATIVE_PROTOCOL_XDR`. |
| `operation` | string or null | Detected supported operation name, when available. |
| `is_fee_bump` | boolean | Whether the decoded envelope used the fee-bump envelope form. |
| `operation_count` | integer | Number of operations in the transaction. |
| `max_entry_ttl` | integer | TTL limit used for this run. |
| `snapshot` | object or null | Caller snapshot authority and `current_ledger`, when supplied. |
| `keys` | array | Each key's footprint section, type, durability when applicable, canonical XDR base64, lowercase SHA-256 digest, and snapshot state. |
| `violations` | array | Stable violation codes with message and optional operation index and key digest. |
| `warnings` | array | Non-failing notes, including keys missing from an optional incomplete snapshot. |

Object fields are serialized in a stable order. Footprint keys retain their original section and vector order. Violation and warning order follows transaction operation order, then footprint section and key order.
