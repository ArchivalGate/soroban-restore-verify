use std::fs::{self, File, OpenOptions};
use std::io::Read;
use std::path::Path;

use stellar_xdr::{
    ContractDataDurability, ExtendFootprintTtlOp, ExtensionPoint, LedgerKey, LedgerKeyAccount,
    LedgerKeyContractCode, LedgerKeyContractData, Limits, Operation, OperationBody,
    RestoreFootprintOp, SorobanTransactionData, Transaction, TransactionEnvelope,
    TransactionExt, TransactionV1Envelope, ReadXdr, WriteXdr,
};

const MAX_INPUT_BYTES: usize = 1_048_576;

fn limits() -> Limits {
    Limits {
        depth: 64,
        len: MAX_INPUT_BYTES,
    }
}

fn envelope(body: OperationBody) -> TransactionEnvelope {
    let operation = Operation {
        body,
        ..Operation::default()
    };
    let mut tx = Transaction {
        operations: vec![operation].try_into().expect("one operation fits"),
        ..Transaction::default()
    };
    tx.ext = TransactionExt::V1(SorobanTransactionData::default());
    TransactionEnvelope::Tx(TransactionV1Envelope {
        tx,
        ..TransactionV1Envelope::default()
    })
}

#[test]
fn decodes_restore_and_extend_envelopes_with_explicit_limits() {
    let restore = envelope(OperationBody::RestoreFootprint(RestoreFootprintOp {
        ext: ExtensionPoint::default(),
    }));
    let restore_b64 = restore.to_xdr_base64(limits()).expect("encode restore");
    let decoded_restore = TransactionEnvelope::from_xdr_base64(&restore_b64, limits())
        .expect("decode restore");
    assert_eq!(decoded_restore, restore);

    let extend = envelope(OperationBody::ExtendFootprintTtl(ExtendFootprintTtlOp {
        ext: ExtensionPoint::default(),
        extend_to: 100,
    }));
    let extend_b64 = extend.to_xdr_base64(limits()).expect("encode extend");
    let decoded_extend = TransactionEnvelope::from_xdr_base64(&extend_b64, limits())
        .expect("decode extend");
    assert_eq!(decoded_extend, extend);
}

#[test]
fn rejects_truncated_and_corrupted_base64_xdr() {
    let valid = envelope(OperationBody::RestoreFootprint(RestoreFootprintOp {
        ext: ExtensionPoint::default(),
    }))
    .to_xdr_base64(limits())
    .expect("encode envelope");

    let truncated = &valid[..valid.len() - 4];
    assert!(TransactionEnvelope::from_xdr_base64(truncated, limits()).is_err());
    assert!(TransactionEnvelope::from_xdr_base64("%%%", limits()).is_err());
}

#[test]
fn ledger_key_order_matches_canonical_xdr_and_durability_is_visible() {
    let account = LedgerKey::Account(LedgerKeyAccount::default());
    let code = LedgerKey::ContractCode(LedgerKeyContractCode::default());
    assert!(account < code);
    let account_bytes = account.to_xdr(limits()).expect("encode account key");
    let code_bytes = code.to_xdr(limits()).expect("encode code key");
    assert!(account_bytes < code_bytes);

    let persistent = LedgerKey::ContractData(LedgerKeyContractData {
        durability: ContractDataDurability::Persistent,
        ..LedgerKeyContractData::default()
    });
    let temporary = LedgerKey::ContractData(LedgerKeyContractData {
        durability: ContractDataDurability::Temporary,
        ..LedgerKeyContractData::default()
    });
    assert!(matches!(
        persistent,
        LedgerKey::ContractData(LedgerKeyContractData {
            durability: ContractDataDurability::Persistent,
            ..
        })
    ));
    assert!(matches!(
        temporary,
        LedgerKey::ContractData(LedgerKeyContractData {
            durability: ContractDataDurability::Temporary,
            ..
        })
    ));
    assert!(matches!(code, LedgerKey::ContractCode(_)));
    assert!(matches!(account, LedgerKey::Account(_)));
}

fn bounded_regular_file(path: &Path) -> std::io::Result<Vec<u8>> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(std::io::Error::other("unsafe file type"));
    }
    if metadata.len() > MAX_INPUT_BYTES as u64 {
        return Err(std::io::Error::other("file exceeds limit"));
    }

    let file = File::open(path)?;
    let mut bytes = Vec::new();
    file.take((MAX_INPUT_BYTES + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_INPUT_BYTES {
        return Err(std::io::Error::other("file exceeds limit"));
    }
    Ok(bytes)
}

#[cfg(unix)]
#[test]
fn refuses_symlinks_oversized_files_and_existing_output_paths() {
    use std::os::unix::fs::symlink;

    let dir = tempfile::tempdir().expect("temporary directory");
    let regular = dir.path().join("regular");
    fs::write(&regular, b"small").expect("write regular file");
    let link = dir.path().join("link");
    symlink(&regular, &link).expect("create symlink");
    assert!(bounded_regular_file(&link).is_err());

    let oversized = dir.path().join("oversized");
    fs::write(&oversized, vec![0_u8; MAX_INPUT_BYTES + 1]).expect("write oversized file");
    assert!(bounded_regular_file(&oversized).is_err());

    let output = dir.path().join("existing-output");
    fs::write(&output, b"keep").expect("write existing output");
    let result = OpenOptions::new().write(true).create_new(true).open(&output);
    assert!(result.is_err());
    assert_eq!(fs::read(output).expect("read existing output"), b"keep");
}
