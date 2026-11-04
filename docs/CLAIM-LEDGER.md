# Claim ledger

Audit date: 2026-11-04

Status: `VERIFIED_CI`

## Evidence-backed claims

| Claim | Status | Evidence | Limit |
| --- | --- | --- | --- |
| The pinned crate builds locally in release mode | `VERIFIED_LOCAL` | `cargo build --release --locked` completed with `Finished release profile [optimized] target(s) in 3m 25s`. | This is a local build result. |
| The CLI verifies valid restore and extend envelopes and returns deterministic reports | `VERIFIED_LOCAL` | `cargo test --all-targets --locked`: 17 CLI integration tests and 4 Phase C tests passed. Release binary positive examples exit 0. | The checks apply to supplied XDR and local policy. |
| Invalid decoded envelopes return structured violations and exit 2 | `VERIFIED_LOCAL` | Integration tests cover operation isolation, footprint ordering and uniqueness, durability, resource fields, TTL limits, archive indexes, and snapshot policy. Release fixture exits 2. | This is not a ledger execution result. |
| Malformed XDR, unsafe paths, over-limit inputs, invalid snapshots, and invalid arguments fail closed with exit 3 | `VERIFIED_LOCAL` | Integration tests cover truncated XDR, oversized and symlink input, unsafe output paths, snapshot schema and key errors, and argument errors. Release truncated fixture exits 3. | The input limit is 1 MiB. |
| Formatting and Clippy checks pass | `VERIFIED_LOCAL` | `cargo fmt --all -- --check` exited 0. `cargo clippy --all-targets --locked -- -D warnings` finished without warnings. | Run on the local Rust 1.97.1 toolchain. |
| The GitHub Actions workflow passed | `VERIFIED_CI` | Run [41921470108](https://github.com/ArchivalGate/soroban-restore-verify/actions/runs/41921470108), conclusion `success`, head `a66dc4260b86b4d008656f88985e23fa9bd3567d`. | The run tested code and workflow commits 1 through 6. Commit 7 adds documentation only. |
| The transaction state was verified against a live network | `NOT APPLICABLE` | The program has no RPC client, wallet, or transaction submission path. | Caller-supplied snapshots are not live chain state. |
| External PR review and contributor merges are ready | `DEFERRED` | The repository is maintained solo before intake. | External review is deferred until after MVP. |
| Drips Stellar Wave / GrantFox eligibility or application status was confirmed | `UNVERIFIED` | No eligibility or application work was performed. | Owner only. No onboarding or application portal was visited. |

## Toolchain and dependency pins

| Item | Observed or resolved version |
| --- | --- |
| `rustc` | `1.97.1 (8bab26f4f 2026-07-14)` |
| `cargo` | `1.97.1 (c980f4866 2026-06-30)` |
| `stellar-xdr` | `30.0.0` |
| `clap` | `4.6.7` |
| `serde` | `1.0.229` |
| `serde_json` | `1.0.154` |
| `sha2` | `0.11.1` |
| `hex` | `0.4.3` |
| `thiserror` | `2.0.21` |
| `tempfile` | `3.27.0` |

Exact pins and the resolved lockfile are committed. `stellar-xdr 30.0.0` exposes `SorobanResources.disk_read_bytes` and `SorobanTransactionDataExt::{V0, V1(SorobanResourcesExtV0)}` at the crate root.

## Test evidence

Command: `cargo test --all-targets --locked`

Observed summaries:

```text
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.36s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
```

The two suites are `tests/cli_integration.rs` and `tests/experiment_c1.rs`. The library and binary test harnesses each contained zero unit tests.

All five `--dump-fixture` outputs were compared with their committed files and were byte-identical: the four base64 XDR fixtures and `sample-snapshot.json`.

The CI run above completed all configured checks on commit `a66dc4260b86b4d008656f88985e23fa9bd3567d`. Its smoke step checked a valid fixture at exit 0, an invariant fixture at exit 2, and a truncated XDR fixture at exit 3.

## Git evidence

The first six commits match the planned logical units:

1. `87e6cb2` `docs: add Phase A project profile and Phase B toolchain matrix`
2. `b423d3b` `test: add Phase C experiment C-1 XDR and file safety verification`
3. `ff23455` `docs: add Phase D architecture and authority contract`
4. `1f1b9d3` `feat: implement core CAP-0046-12 verifier, snapshot engine, and file safety`
5. `167ddbe` `feat: implement CLI surface, deterministic fixtures, and integration tests`
6. `a66dc42` `ci: add non-mutating least-privilege GitHub Actions workflow`

The code and workflow head tested by CI was `a66dc4260b86b4d008656f88985e23fa9bd3567d`. This Phase K record is the documentation commit 7 and does not change code or CI. A Git commit cannot include its own commit ID in its content because that ID depends on the content; the final audit-record commit ID is available from `git rev-parse HEAD` and the completion summary.

`git push -u origin main` succeeded. The remote is `https://github.com/ArchivalGate/soroban-restore-verify.git`.
