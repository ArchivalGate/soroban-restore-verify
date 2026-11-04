use clap::ValueEnum;
use serde::Serialize;
use sha2::Digest;
use stellar_xdr::{
    ContractDataDurability, ExtendFootprintTtlOp, ExtensionPoint, FeeBumpTransaction,
    FeeBumpTransactionEnvelope, FeeBumpTransactionInnerTx, Hash, InvokeHostFunctionOp,
    LedgerFootprint, LedgerKey, LedgerKeyContractCode, LedgerKeyContractData, Limits, Operation,
    OperationBody, RestoreFootprintOp, SorobanResources, SorobanTransactionData,
    SorobanTransactionDataExt, Transaction, TransactionEnvelope, TransactionExt,
    TransactionV1Envelope, WriteXdr,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
#[value(rename_all = "kebab-case")]
pub enum FixtureName {
    ValidRestore,
    ValidExtend,
    InvalidRestoreTemp,
    InvalidExtendUnsorted,
    SampleSnapshot,
}

#[derive(Debug, thiserror::Error)]
pub enum FixtureError {
    #[error("could not encode fixture XDR: {0}")]
    Xdr(String),
    #[error("could not encode fixture JSON: {0}")]
    Json(String),
}

pub fn fixture_payload(name: FixtureName) -> Result<Vec<u8>, FixtureError> {
    match name {
        FixtureName::SampleSnapshot => {
            let key = contract_data_key(ContractDataDurability::Persistent);
            sample_snapshot_for_key(&key, 150)
        }
        _ => {
            let envelope = fixture_envelope(name);
            let mut encoded = envelope
                .to_xdr_base64(limits())
                .map_err(|error| FixtureError::Xdr(error.to_string()))?;
            encoded.push('\n');
            Ok(encoded.into_bytes())
        }
    }
}

pub fn fixture_envelope(name: FixtureName) -> TransactionEnvelope {
    match name {
        FixtureName::ValidRestore => build_envelope(EnvelopeSpec {
            operation_bodies: vec![restore_operation()],
            footprint: LedgerFootprint {
                read_write: vec![contract_data_key(ContractDataDurability::Persistent)]
                    .try_into()
                    .expect("one fixture key fits XDR"),
                ..LedgerFootprint::default()
            },
            instructions: 0,
            disk_read_bytes: 8,
            write_bytes: 8,
            resource_fee: 100,
            data_ext: SorobanTransactionDataExt::V0,
        }),
        FixtureName::ValidExtend | FixtureName::SampleSnapshot => build_envelope(EnvelopeSpec {
            operation_bodies: vec![extend_operation(100)],
            footprint: LedgerFootprint {
                read_only: vec![contract_data_key(ContractDataDurability::Persistent)]
                    .try_into()
                    .expect("one fixture key fits XDR"),
                ..LedgerFootprint::default()
            },
            instructions: 0,
            disk_read_bytes: 8,
            write_bytes: 0,
            resource_fee: 100,
            data_ext: SorobanTransactionDataExt::V0,
        }),
        FixtureName::InvalidRestoreTemp => build_envelope(EnvelopeSpec {
            operation_bodies: vec![restore_operation()],
            footprint: LedgerFootprint {
                read_write: vec![contract_data_key(ContractDataDurability::Temporary)]
                    .try_into()
                    .expect("one fixture key fits XDR"),
                ..LedgerFootprint::default()
            },
            instructions: 0,
            disk_read_bytes: 8,
            write_bytes: 8,
            resource_fee: 100,
            data_ext: SorobanTransactionDataExt::V0,
        }),
        FixtureName::InvalidExtendUnsorted => build_envelope(EnvelopeSpec {
            operation_bodies: vec![extend_operation(100)],
            footprint: LedgerFootprint {
                read_only: vec![contract_code_key(1), contract_code_key(0)]
                    .try_into()
                    .expect("two fixture keys fit XDR"),
                ..LedgerFootprint::default()
            },
            instructions: 0,
            disk_read_bytes: 8,
            write_bytes: 0,
            resource_fee: 100,
            data_ext: SorobanTransactionDataExt::V0,
        }),
    }
}

pub struct EnvelopeSpec {
    pub operation_bodies: Vec<OperationBody>,
    pub footprint: LedgerFootprint,
    pub instructions: u32,
    pub disk_read_bytes: u32,
    pub write_bytes: u32,
    pub resource_fee: i64,
    pub data_ext: SorobanTransactionDataExt,
}

pub fn build_envelope(spec: EnvelopeSpec) -> TransactionEnvelope {
    let operations = spec
        .operation_bodies
        .into_iter()
        .map(|body| Operation {
            body,
            ..Operation::default()
        })
        .collect::<Vec<_>>();
    let mut tx = Transaction {
        operations: operations
            .try_into()
            .expect("fixture operation count fits XDR"),
        ..Transaction::default()
    };
    tx.ext = TransactionExt::V1(SorobanTransactionData {
        ext: spec.data_ext,
        resources: SorobanResources {
            footprint: spec.footprint,
            instructions: spec.instructions,
            disk_read_bytes: spec.disk_read_bytes,
            write_bytes: spec.write_bytes,
        },
        resource_fee: spec.resource_fee,
    });
    TransactionEnvelope::Tx(TransactionV1Envelope {
        tx,
        ..TransactionV1Envelope::default()
    })
}

pub fn fee_bump(envelope: TransactionEnvelope) -> TransactionEnvelope {
    match envelope {
        TransactionEnvelope::Tx(inner) => {
            TransactionEnvelope::TxFeeBump(FeeBumpTransactionEnvelope {
                tx: FeeBumpTransaction {
                    inner_tx: FeeBumpTransactionInnerTx::Tx(inner),
                    ..FeeBumpTransaction::default()
                },
                ..FeeBumpTransactionEnvelope::default()
            })
        }
        other => other,
    }
}

pub fn restore_operation() -> OperationBody {
    OperationBody::RestoreFootprint(RestoreFootprintOp {
        ext: ExtensionPoint::default(),
    })
}

pub fn extend_operation(extend_to: u32) -> OperationBody {
    OperationBody::ExtendFootprintTtl(ExtendFootprintTtlOp {
        ext: ExtensionPoint::default(),
        extend_to,
    })
}

pub fn invoke_operation() -> OperationBody {
    OperationBody::InvokeHostFunction(InvokeHostFunctionOp::default())
}

pub fn contract_data_key(durability: ContractDataDurability) -> LedgerKey {
    LedgerKey::ContractData(LedgerKeyContractData {
        durability,
        ..LedgerKeyContractData::default()
    })
}

pub fn contract_code_key(byte: u8) -> LedgerKey {
    let mut hash = [0_u8; 32];
    hash[31] = byte;
    LedgerKey::ContractCode(LedgerKeyContractCode { hash: Hash(hash) })
}

pub fn sample_snapshot_for_key(
    key: &LedgerKey,
    live_until_ledger_seq: u32,
) -> Result<Vec<u8>, FixtureError> {
    let key_xdr_base64 = key
        .to_xdr_base64(limits())
        .map_err(|error| FixtureError::Xdr(error.to_string()))?;
    let bytes = key
        .to_xdr(limits())
        .map_err(|error| FixtureError::Xdr(error.to_string()))?;
    let key_sha256 = hex::encode(sha2::Sha256::digest(bytes));
    let snapshot = SnapshotFixture {
        schema: "soroban-restore-verify/snapshot/1.0",
        entries: vec![SnapshotEntryFixture {
            key_xdr_base64,
            key_sha256,
            live_until_ledger_seq,
        }],
    };
    let mut output = serde_json::to_vec_pretty(&snapshot)
        .map_err(|error| FixtureError::Json(error.to_string()))?;
    output.push(b'\n');
    Ok(output)
}

#[derive(Serialize)]
struct SnapshotFixture {
    schema: &'static str,
    entries: Vec<SnapshotEntryFixture>,
}

#[derive(Serialize)]
struct SnapshotEntryFixture {
    key_xdr_base64: String,
    key_sha256: String,
    live_until_ledger_seq: u32,
}

fn limits() -> Limits {
    Limits {
        depth: 64,
        len: 1_048_576,
    }
}
