# Project profile

Status: `IMPLEMENTED`

Profile date: 2026-11-04

## Intake

| Question | Answer |
| --- | --- |
| Problem | A locally constructed Soroban archival transaction can have an invalid footprint, operation mix, resource declaration, TTL target, or caller-snapshot relationship. XDR decoding alone does not check this project policy. |
| User | Stellar developers and keepers preparing `RestoreFootprintOp` or `ExtendFootprintTtlOp` envelopes. |
| Existing options | Stellar CLI decodes XDR and can build or submit a restore flow. RPC inspectors, source-code linters, and state explorers answer adjacent questions. The dated comparison below records their scope. |
| Positive path | Pass base64 or raw `TransactionEnvelope` XDR. Receive a deterministic report with the operation, affected ledger keys, SHA-256 digests, snapshot state if supplied, and `VERIFIED_LOCAL`; exit 0. |
| Negative path | Malformed or unsafe input exits 3. A decoded envelope or complete caller-snapshot policy violation exits 2 with structured diagnostics. |
| Product shape | Offline Rust CLI and library in one crate. JSON and human reports go to stdout or to a newly created output file. |
| Why Web3 XDR is essential | The operation, Soroban transaction data, footprint, and ledger-key discriminants are encoded in Stellar XDR. The verifier must decode and inspect those protocol types to check the requested invariants. |
| Authoritative truth | Decoded transaction XDR is authoritative for the transaction structure and declared resources only. It is not proof that the transaction was accepted, applied, or valid against a particular ledger. |
| Caller snapshot | Optional JSON supplied by the caller. It is not a live RPC view or proof of network state. Missing entries are reported as inconclusive unless complete coverage is required. |
| Trust boundaries | CLI arguments, stdin, input and snapshot paths, XDR bytes, and JSON are untrusted. Inputs are capped at 1 MiB. Symlinks and non-regular files are rejected. Output creation is atomic and no-clobber. |
| Scope exclusions | No dashboard, contract deployment, database, background service, signing, RPC access, network write, or testnet/mainnet transaction. No multi-repository split. |
| Delivery route | Build and audit the local repository first. Drips Stellar Wave / GrantFox eligibility and any submission are owner-managed only. No onboarding or application portal work is in scope. |
| Stop condition | Stop at a locally verified offline CLI with an honest repository audit. External contribution review remains deferred until after MVP. Program eligibility or acceptance is not claimed. |

## User to task to improvement

| User | Task | Current workaround | Failure or cost | Observable improvement |
| --- | --- | --- | --- | --- |
| Developer or keeper preparing an archival transaction | Check that an envelope has an isolated archival operation and a valid footprint before handing it to another tool | Decode XDR manually, inspect CLI output, or write a one-off script | Ordering, durability, resource, and operation-mixing mistakes can be missed or handled inconsistently | One local command returns stable violations, key digests, and an explicit local-only status |
| Operator with a local ledger-state export | Compare target keys with known live-until ledger values | Inspect the export separately or query a live service | The result depends on caller interpretation and may be confused with current chain state | A supplied snapshot is labeled as caller data and checked against the stated current ledger |

## Comparator matrix

Checked 2026-11-04. These tools have different inputs and authority; this comparison describes their documented project scope.

| Project | Documented scope | Overlap | Gap for this project |
| --- | --- | --- | --- |
| [Stellar CLI](https://github.com/stellar/stellar-cli) | `stellar xdr decode` handles XDR decoding. `stellar contract restore` prepares a restore request and uses network configuration and a source identity for its operation. See the [restore guide](https://developers.stellar.org/docs/tools/cli/cookbook/restore-contract-storage). | XDR decoding and restore workflow. | The requested tool is a local policy checker for a supplied transaction envelope and does not submit transactions or query RPC. |
| [Dantama022/stellar-api-inspector](https://github.com/Dantama022/stellar-api-inspector) | A Node CLI for inspecting Horizon and Soroban RPC health, transactions, contracts, accounts, and operations. Its README describes endpoint connections and transaction lookup. | Stellar transaction and Soroban inspection. | It is endpoint-oriented and does not document this offline envelope policy check. |
| [Tollcraft/soroban-cost-linter](https://github.com/Tollcraft/soroban-cost-linter) | A Rust/Dylint static analyzer for input-independent Soroban contract resource-cost patterns before compilation. | Resource-cost review before a transaction is used. | It analyzes contract source patterns, not a serialized archival transaction footprint or caller TTL snapshot. |
| [Vynix-Labs/Soroban-state-lens](https://github.com/Vynix-Labs/Soroban-state-lens) | A browser-based Soroban state explorer with RPC-backed live state views and XDR value decoding. | Ledger state inspection and XDR decoding. | It is an interactive state viewer, not a deterministic local validator for restore and TTL-extension envelopes. |

## Lifecycle applicability

| Playbook phase | Status | Reason |
| --- | --- | --- |
| Phase F: network deployment | `NOT APPLICABLE` | The target is an offline CLI. It has no contract to deploy. |
| Phase H: live-chain proof | `NOT APPLICABLE` | The CLI reads local XDR and optional caller JSON. It does not need or claim testnet/mainnet state. |
| Phase L: external contribution readiness | `CONDITIONAL` / `DEFERRED` | This is a solo pre-intake repository. External PR review and contributor merges are deferred until after MVP. |
