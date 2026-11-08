# Contributing

Keep each PR focused on one behavior, explain the change, and include the verification results.

## Module Seams & File Ownership

Start with the [src/ module table](README.md#modular-architecture-map). Use these seams to keep parallel PRs in isolated files:

| Change | Primary file |
| --- | --- |
| New protocol rules | `src/verifier.rs` |
| Output formats | `src/report.rs` |
| Input adapters and file safety | `src/io_safety.rs` |
| End-to-end CLI tests | `tests/cli_integration.rs` |

List the files you plan to change in the issue or PR. Coordinate shared edits to `src/main.rs` and `src/model.rs`, and avoid unrelated refactors. Update the README when public behavior changes.

## Engineering & Safety Rules

- Do not use `unwrap()` or `expect()` on untrusted inputs. Return typed errors and useful diagnostics.
- Preserve the 1 MiB input bounds, bounded reads, and refusal of symlink and non-regular file paths. Output must use `create_new(true)` and refuse existing paths.
- Preserve exit codes: `0` for verified local input, `2` for decoded policy violations, and `3` for input, configuration, or I/O errors.
- Preserve existing CLI flags, report schemas, deterministic output, and stream separation. Reports and fixtures go to stdout or the requested output file; diagnostics go to stderr.
- Keep verification offline. Caller snapshots do not establish live ledger state.
- Use plain engineering language. Use zero em dashes in code, comments, Markdown, commit messages, and issues. Do not add AI co-author trailers.
- Keep the existing regression tests passing and cover changed behavior at the appropriate module seam.

## Local Pre-PR Verification Commands

Run these commands from the repository root:

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --all-targets --locked
cargo run --example verify_envelope --locked
```

All four commands must succeed. The library example checks both valid and invalid restore envelopes in memory and returns an error if either result is unexpected.
