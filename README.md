# soroban-restore-verify

Offline verification for Stellar Soroban restore and footprint TTL extension transaction envelopes.

## Project records

[Project profile](docs/PROJECT-PROFILE.md) · [Architecture](docs/ARCHITECTURE.md) · [Toolchain matrix](docs/TOOLCHAIN-MATRIX.md) · [Experiments](docs/EXPERIMENTS.md) · [Report schema](docs/REPORT-SCHEMA.json) · [Claim ledger](docs/CLAIM-LEDGER.md) · [Final audit](docs/FINAL-AUDIT.md) · [Security](SECURITY.md)

Repository status: `VERIFIED_LOCAL`. GitHub Actions status: `VERIFIED_CI` for commit `a66dc4260b86b4d008656f88985e23fa9bd3567d` in [run 41921470108](https://github.com/ArchivalGate/soroban-restore-verify/actions/runs/41921470108).

## What it checks

The CLI reads base64 or raw `TransactionEnvelope` XDR from a file or stdin, or base64 directly from `--xdr`. It checks operation isolation, footprint ordering and uniqueness, key types and durability, the selected Soroban resource fields, TTL bounds, archive indexes, and optional local snapshot comparisons. It reports canonical key XDR and SHA-256 digests.

The tool does not call an RPC service, sign a transaction, or submit it. `VERIFIED_LOCAL` means the supplied envelope passed this verifier's local checks. It does not prove that a transaction was accepted or applied by Stellar.

Snapshot checks use only the JSON file supplied by the caller. They do not prove current on-chain state.

## Build

Rust 1.88 or newer is required by the pinned dependencies. The tested toolchain is recorded in the [toolchain matrix](docs/TOOLCHAIN-MATRIX.md).

```sh
cargo build --release --locked
```

## Verify an envelope

Read a base64 envelope from a file:

```sh
./target/release/soroban-restore-verify --input transaction.xdr --format json
```

Read raw binary XDR from a file:

```sh
./target/release/soroban-restore-verify --input transaction.bin --raw-xdr
```

Read base64 XDR from stdin:

```sh
cat transaction.xdr | ./target/release/soroban-restore-verify --input -
```

Check against a caller-supplied snapshot:

```sh
./target/release/soroban-restore-verify \
  --input fixtures/valid-extend.xdr \
  --snapshot fixtures/sample-snapshot.json \
  --current-ledger 100 \
  --format json
```

With `--require-complete-snapshot`, every footprint key must be present. Without it, a missing key is marked `INCONCLUSIVE_MISSING_FROM_SNAPSHOT`.

To write a report, use `--output PATH`. The command refuses an existing path or symlink and creates a new file with `create_new(true)`.

## Reproducible result examples

The following commands were run with the release binary built from the locked manifest. The observed output is shown with stdout and stderr identified separately.

### Valid restore envelope

Command:

```sh
./target/release/soroban-restore-verify --input fixtures/valid-restore.xdr --format human
```

Observed stdout:

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

Exit code: `0`.

### Decoded invariant violation

Command:

```sh
./target/release/soroban-restore-verify --input fixtures/invalid-restore-temp.xdr --format human
```

Observed stdout:

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

Observed stderr:

```json
{"code":"VERIFICATION_VIOLATION","message":"one or more policy invariants failed"}
```

Exit code: `2`.

### Truncated XDR refusal

Command:

```sh
./target/release/soroban-restore-verify --input fixtures/truncated.xdr --format json
```

Observed stdout: empty.

Observed stderr:

```json
{"code":"XDR_INVALID","message":"invalid TransactionEnvelope XDR: failed to fill whole buffer"}
```

Exit code: `3`.

## Built-in fixtures

`--dump-fixture` writes deterministic test data to stdout:

```text
valid-restore
valid-extend
invalid-restore-temp
invalid-extend-unsorted
sample-snapshot
```

The committed examples are under [fixtures](fixtures/). The CLI rejects files and stdin payloads larger than 1 MiB, symlink and non-regular input paths, malformed XDR, invalid snapshot JSON, and output paths that already exist.

## Scope

This is an offline CLI for local envelope checks. There is no dashboard, contract deployment, database, indexer, daemon, wallet signing, RPC connection, or testnet/mainnet write transaction. Program eligibility and any submission are owner-managed. No application work is performed by this repository.
