# Phase C experiment record

Status: `VERIFIED_LOCAL`

## C-1: XDR and file-safety verification

**Question:** Can the pinned XDR crate decode the two archival operation envelopes with explicit depth and byte limits, and do the canonical ordering and file-safety primitives needed by the verifier behave as expected?

**Environment:** `rustc 1.97.1 (8bab26f4f 2026-07-14)`, `cargo 1.97.1 (c980f4866 2026-06-30)`, `stellar-xdr 30.0.0`, Linux x86_64. The experiment uses `Limits { depth: 64, len: 1_048_576 }`.

**Coverage:**

- Encode and decode structurally valid `RestoreFootprintOp` and `ExtendFootprintTtlOp` `TransactionEnvelope` values.
- Reject a truncated XDR payload and invalid base64.
- Compare Rust `LedgerKey` ordering with canonical XDR byte ordering for an Account key and ContractCode key.
- Distinguish persistent and temporary ContractData, ContractCode, and classic Account keys.
- Reject symlink and over-limit file input using symlink metadata and a bounded `Read::take` reader.
- Reject creation over an existing output file with `create_new(true)` and confirm its contents remain intact.

**Command:** `rtk proxy cargo test --locked --test experiment_c1`

**Observed output:**

```text
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.25s
     Running tests/experiment_c1.rs (target/debug/deps/experiment_c1-c754b90644d660af)

running 4 tests
test ledger_key_order_matches_canonical_xdr_and_durability_is_visible ... ok
test decodes_restore_and_extend_envelopes_with_explicit_limits ... ok
test rejects_truncated_and_corrupted_base64_xdr ... ok
test refuses_symlinks_oversized_files_and_existing_output_paths ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
```

**Decision:** `KEEP`

**Gate G-C:** `PASS`
