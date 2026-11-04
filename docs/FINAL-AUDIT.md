# Final audit

Audit date: 2026-11-04

Status: `VERIFIED_CI`

## Repository verdict

**READY FOR SUBMISSION - REPOSITORY**

Local formatting, Clippy, all-target tests, release build, fixture reports, and file-safety cases passed. GitHub Actions passed on code and workflow head `a66dc4260b86b4d008656f88985e23fa9bd3567d`: [run 41921470108](https://github.com/ArchivalGate/soroban-restore-verify/actions/runs/41921470108).

The audit record is the seventh logical commit and adds documentation only. The exact tested code and workflow SHA is recorded above. The final audit-record commit SHA is reported in the completion summary because a commit cannot contain its own content-dependent ID.

## Other verdicts

| Area | Verdict | Evidence |
| --- | --- | --- |
| External contribution | `DEFERRED` / `NOT READY FOR EXTERNAL CONTRIBUTOR MERGES` | Solo pre-intake posture. External PR review is deferred until after MVP. |
| Testnet or deployed code | `NOT APPLICABLE` | This is an offline CLI. There is no contract deployment or network write path. |
| Program eligibility and application | `OWNER ONLY; NO APPLICATION WORK PERFORMED` | No eligibility assessment, onboarding portal, application form, or submission was performed. |
| Live chain proof | `NOT APPLICABLE` | The CLI reads local XDR and optional caller-supplied snapshot JSON only. |

## Observed local verification

| Command | Observed result |
| --- | --- |
| `rustc --version` | `rustc 1.97.1 (8bab26f4f 2026-07-14)` |
| `cargo --version` | `cargo 1.97.1 (c980f4866 2026-06-30)` |
| `cargo fmt --all -- --check` | Exit 0. |
| `cargo clippy --all-targets --locked -- -D warnings` | Exit 0 with no warnings. |
| `cargo test --all-targets --locked` | 21 passed across 17 CLI integration tests and 4 experiment tests; 0 failed. |
| `cargo build --release --locked` | Exit 0. `Finished release profile [optimized] target(s) in 3m 25s`. |
| Release binary with `fixtures/valid-restore.xdr` | Exit 0, status `VERIFIED_LOCAL`. |
| Release binary with `fixtures/invalid-restore-temp.xdr` | Exit 2, status `VIOLATION`. |
| Release binary with `fixtures/truncated.xdr` | Exit 3, stderr code `XDR_INVALID`. |
| `git push -u origin main` | Exit 0. New `main` branch pushed to `origin`. |
| GitHub Actions | `VERIFIED_CI`, run [41921470108](https://github.com/ArchivalGate/soroban-restore-verify/actions/runs/41921470108), conclusion `success`. |

Test counts and command results are recorded from observed runs. No mainnet or testnet proof is claimed.

## Audit boundary

The report's `AUTHORITATIVE_PROTOCOL_XDR` authority is limited to fields present in the supplied `TransactionEnvelope`. `CALLER_SNAPSHOT` state is caller-provided and is not evidence of live ledger state. The CLI does not establish transaction acceptance, execution, or current network state.

The status of Drips Stellar Wave / GrantFox eligibility remains `UNVERIFIED`; eligibility and any application work are owner-managed. No application work was performed.
