# Toolchain matrix

Status: `IMPLEMENTED`

Recorded 2026-11-04 from local commands before implementation. Direct dependencies are pinned exactly in `Cargo.toml` and locked in `Cargo.lock`.

## Verified versions

| Tool or crate | Verified version | Evidence |
| --- | --- | --- |
| `rustc` | `1.97.1 (8bab26f4f 2026-07-14)` | `rustc --version` |
| `cargo` | `1.97.1 (c980f4866 2026-06-30)` | `cargo --version` |
| `stellar-xdr` | `30.0.0` | `cargo search stellar-xdr --limit 1`; `cargo info stellar-xdr@30.0.0` |
| `clap` | `4.6.7` | `cargo info clap` |
| `serde` | `1.0.229` | `cargo info serde` |
| `serde_json` | `1.0.154` | `cargo info serde_json` |
| `sha2` | `0.11.1` | `cargo info sha2` |
| `hex` | `0.4.3` | `cargo info hex` |
| `thiserror` | `2.0.21` | `cargo info thiserror` |
| `tempfile` | `3.27.0` | `cargo info tempfile` |

The selected `stellar-xdr` release declares Rust 1.87 as its minimum. The selected `sha2` release declares Rust 1.88, so this crate declares Rust 1.88 as its minimum. The observed local compiler is newer.

## Resolved XDR fields

Inspection of the downloaded `stellar-xdr 30.0.0` generated source verified:

```text
SorobanResources {
    footprint: LedgerFootprint,
    instructions: u32,
    disk_read_bytes: u32,
    write_bytes: u32,
}

SorobanTransactionData {
    ext: SorobanTransactionDataExt,
    resources: SorobanResources,
    resource_fee: i64,
}

SorobanTransactionDataExt = V0 | V1(SorobanResourcesExtV0)
SorobanResourcesExtV0 {
    archived_soroban_entries: VecM<u32>,
}
```

The read-bytes field is named `disk_read_bytes`. The transaction-data extension is `SorobanTransactionDataExt::V0` or `V1(SorobanResourcesExtV0)`. The generated definitions are in the [`stellar-xdr` 30.0.0 source](https://github.com/stellar/rs-stellar-xdr/tree/v30.0.0).

## Capability and authority matrix

| Capability or input | Authority | Project handling |
| --- | --- | --- |
| Base64 or raw `TransactionEnvelope` XDR | `AUTHORITATIVE_PROTOCOL_XDR` for the fields it contains | Decode within bounded input and XDR limits; report transaction invariants as `VERIFIED_LOCAL` or `VIOLATION`. This does not prove ledger acceptance or execution. |
| Optional snapshot JSON | `CALLER_SNAPSHOT` | Validate the JSON schema and key digests. Use it only for local TTL comparisons; never describe it as live chain state. |
| Snapshot key absent | `CALLER_SNAPSHOT` | Mark the key `INCONCLUSIVE_MISSING_FROM_SNAPSHOT`, or emit `SNAPSHOT_MISSING_KEY_STATE` when complete coverage is required. |
| RPC, transaction submission, signing, or wallet state | None | Not implemented. No network access is required by the CLI. |
| `--output` path | Local filesystem | Refuse symlinks and existing paths; create with `create_new(true)`. |
