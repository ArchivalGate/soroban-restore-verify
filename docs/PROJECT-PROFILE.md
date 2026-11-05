# Project profile

## Purpose

`soroban-restore-verify` is an offline Rust CLI and library for preflight checks of Stellar Soroban archival transaction envelopes. It helps developers and keepers review a transaction's operation, footprint, resources, TTL target, and optional caller-supplied ledger snapshot before signing or submission.

## Supported inputs and reports

- Read base64 or raw `TransactionEnvelope` XDR from a file, stdin, or the `--xdr` argument.
- Optionally compare footprint keys with a JSON snapshot and a caller-supplied current ledger number.
- Emit human-readable or JSON reports with operation details, canonical key XDR, SHA-256 digests, and snapshot status.

## Protocol checks

The verifier checks `RestoreFootprintOp`, `ExtendFootprintTtlOp`, and archival `InvokeHostFunctionOp` envelopes against the invariants in [CAP-0046-12](https://github.com/stellar/stellar-protocol/blob/master/core/cap-0046-12.md) and [CAP-0066](https://github.com/stellar/stellar-protocol/blob/master/core/cap-0066.md). It validates operation isolation, canonical footprint ordering, supported key types and durability, resource declarations, TTL bounds, and archived-entry indexes.

## Trust boundaries

- The CLI does not use RPC, sign transactions, or submit transactions.
- Snapshot checks reflect caller-provided JSON and do not prove current network state.
- XDR, snapshot, and stdin inputs are limited to 1 MiB. Symlink and non-regular paths are rejected.
- `--output` creates a new file and refuses to replace an existing path.

## Implementation

The project is a Rust 2024 edition crate with a binary CLI and a reusable library. Dependency versions are pinned in `Cargo.toml` and `Cargo.lock`. See the [toolchain matrix](TOOLCHAIN-MATRIX.md) and [architecture](ARCHITECTURE.md) for implementation details.
