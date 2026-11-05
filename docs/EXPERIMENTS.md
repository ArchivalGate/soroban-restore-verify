# XDR and file safety experiments

Status: verified locally

## Objective

Confirm that the pinned XDR crate decodes archival operation envelopes within explicit depth and byte limits, and that canonical key ordering and file-safety primitives behave as required by the verifier.

## Environment

The experiment used `rustc 1.97.1 (8bab26f4f 2026-07-14)`, `cargo 1.97.1 (c980f4866 2026-06-30)`, `stellar-xdr 30.0.0`, and Linux x86_64. XDR decoding used `Limits { depth: 64, len: 1_048_576 }`.

## Coverage

- Encode and decode valid `RestoreFootprintOp` and `ExtendFootprintTtlOp` `TransactionEnvelope` values.
- Reject truncated XDR and invalid base64.
- Compare Rust `LedgerKey` ordering with canonical XDR byte ordering for an Account key and a ContractCode key.
- Distinguish persistent and temporary ContractData, ContractCode, and classic Account keys.
- Reject symlink and over-limit file input with symlink metadata and a bounded `Read::take` reader.
- Refuse to replace an existing output file with `create_new(true)` and confirm its contents remain intact.

## Verification command

```bash
rtk proxy cargo test --locked --test experiment_c1
```

## Result

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

## Conclusion

The observed XDR decoding, key ordering, and file-safety behavior matched the requirements for these cases.
