use std::collections::BTreeSet;

use sha2::{Digest, Sha256};
use stellar_xdr::{
    ContractDataDurability, FeeBumpTransactionInnerTx, LedgerFootprint, LedgerKey, Limits,
    OperationBody, SorobanTransactionData, SorobanTransactionDataExt, TransactionEnvelope,
    TransactionExt, WriteXdr,
};

use crate::io_safety::MAX_INPUT_BYTES;
use crate::model::{KeyReport, Report, ReportWarning, ViolationCode, violation};
use crate::snapshot::SnapshotIndex;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VerifyOptions {
    pub max_entry_ttl: u32,
    pub require_complete_snapshot: bool,
}

impl Default for VerifyOptions {
    fn default() -> Self {
        Self {
            max_entry_ttl: 3_110_400,
            require_complete_snapshot: false,
        }
    }
}

pub fn verify_envelope(
    envelope: &TransactionEnvelope,
    options: VerifyOptions,
    snapshot: Option<&SnapshotIndex>,
) -> Report {
    let mut report = Report::new(
        options.max_entry_ttl,
        snapshot.map(SnapshotIndex::report_summary),
    );

    let (tx, is_fee_bump) = match envelope {
        TransactionEnvelope::TxV0(legacy) => {
            report.operation_count = legacy.tx.operations.len();
            report.operation = legacy
                .tx
                .operations
                .first()
                .map(|operation| operation_name(&operation.body).to_owned());
            report.violations.push(violation(
                ViolationCode::EnvelopeV0UnsupportedForSoroban,
                "legacy transaction envelope V0 is unsupported for Soroban verification",
                None,
                None,
            ));
            report.finish();
            return report;
        }
        TransactionEnvelope::Tx(v1) => (&v1.tx, false),
        TransactionEnvelope::TxFeeBump(fee_bump) => {
            let FeeBumpTransactionInnerTx::Tx(inner) = &fee_bump.tx.inner_tx;
            (&inner.tx, true)
        }
    };

    report.is_fee_bump = is_fee_bump;
    report.operation_count = tx.operations.len();
    report.operation = if tx.operations.len() > 1 {
        Some("MULTIPLE".to_owned())
    } else {
        tx.operations
            .first()
            .map(|operation| operation_name(&operation.body).to_owned())
    };

    if tx.operations.is_empty() {
        report.violations.push(violation(
            ViolationCode::EmptyOperations,
            "transaction has no operations",
            None,
            None,
        ));
    } else if tx.operations.len() > 1 {
        report.violations.push(violation(
            ViolationCode::OpNotIsolated,
            "Soroban archival and host operations must be isolated in a single-operation transaction",
            None,
            None,
        ));
    }

    let soroban_data = match &tx.ext {
        TransactionExt::V1(data) => Some(data),
        TransactionExt::V0 => {
            report.violations.push(violation(
                ViolationCode::MissingSorobanTransactionData,
                "transaction does not contain Soroban transaction data",
                None,
                None,
            ));
            None
        }
    };

    if let Some(data) = soroban_data {
        let footprint = &data.resources.footprint;
        validate_footprint(footprint, &mut report);
        append_key_reports(
            footprint,
            snapshot,
            options.require_complete_snapshot,
            &mut report,
        );

        for (operation_index, operation) in tx.operations.iter().enumerate() {
            match &operation.body {
                OperationBody::RestoreFootprint(_) => {
                    verify_restore(data, options, snapshot, operation_index, &mut report)
                }
                OperationBody::ExtendFootprintTtl(extend) => verify_extend(
                    data,
                    extend.extend_to,
                    options,
                    snapshot,
                    operation_index,
                    &mut report,
                ),
                OperationBody::InvokeHostFunction(_) => {
                    verify_invoke(data, operation_index, &mut report)
                }
                _ => report.violations.push(violation(
                    ViolationCode::UnsupportedOperationType,
                    format!(
                        "unsupported operation type: {}",
                        operation_name(&operation.body)
                    ),
                    Some(operation_index),
                    None,
                )),
            }
        }
    } else {
        for (operation_index, operation) in tx.operations.iter().enumerate() {
            if !matches!(
                operation.body,
                OperationBody::RestoreFootprint(_)
                    | OperationBody::ExtendFootprintTtl(_)
                    | OperationBody::InvokeHostFunction(_)
            ) {
                report.violations.push(violation(
                    ViolationCode::UnsupportedOperationType,
                    format!(
                        "unsupported operation type: {}",
                        operation_name(&operation.body)
                    ),
                    Some(operation_index),
                    None,
                ));
            }
        }
    }

    report.finish();
    report
}

fn verify_restore(
    data: &SorobanTransactionData,
    _options: VerifyOptions,
    snapshot: Option<&SnapshotIndex>,
    operation_index: usize,
    report: &mut Report,
) {
    let footprint = &data.resources.footprint;
    if !footprint.read_only.is_empty() {
        report.violations.push(violation(
            ViolationCode::RestoreNonEmptyReadOnly,
            "RestoreFootprintOp requires an empty read_only footprint",
            Some(operation_index),
            None,
        ));
    }
    if footprint.read_write.is_empty() {
        report.violations.push(violation(
            ViolationCode::RestoreEmptyReadWrite,
            "RestoreFootprintOp requires at least one read_write key",
            Some(operation_index),
            None,
        ));
    }
    for key in &footprint.read_write {
        match key {
            LedgerKey::ContractCode(_) => {}
            LedgerKey::ContractData(data) => {
                if data.durability == ContractDataDurability::Temporary {
                    report.violations.push(violation(
                        ViolationCode::RestoreForbiddenTemporaryKey,
                        "RestoreFootprintOp cannot target temporary ContractData",
                        Some(operation_index),
                        digest_for_violation(key),
                    ));
                }
            }
            _ => report.violations.push(violation(
                ViolationCode::RestoreForbiddenClassicKey,
                format!(
                    "RestoreFootprintOp cannot target classic {} keys",
                    ledger_key_type(key)
                ),
                Some(operation_index),
                digest_for_violation(key),
            )),
        }
    }

    if data.resources.instructions != 0 {
        report.violations.push(violation(
            ViolationCode::RestoreNonZeroInstructions,
            "RestoreFootprintOp requires instructions to be zero",
            Some(operation_index),
            None,
        ));
    }
    if data.resources.write_bytes == 0 {
        report.violations.push(violation(
            ViolationCode::RestoreZeroWriteBytes,
            "RestoreFootprintOp requires write_bytes greater than zero",
            Some(operation_index),
            None,
        ));
    }
    if data.resources.disk_read_bytes == 0 {
        report.violations.push(violation(
            ViolationCode::RestoreZeroDiskReadBytes,
            "RestoreFootprintOp requires disk_read_bytes greater than zero",
            Some(operation_index),
            None,
        ));
    }
    if data.resource_fee <= 0 {
        report.violations.push(violation(
            ViolationCode::RestoreNonPositiveResourceFee,
            "RestoreFootprintOp requires a positive resource_fee",
            Some(operation_index),
            None,
        ));
    }

    if archived_indices(data).is_some_and(|indices| !indices.is_empty()) {
        report.violations.push(violation(
            ViolationCode::RestoreForbiddenArchivedEntriesExt,
            "RestoreFootprintOp requires an empty archived_soroban_entries extension",
            Some(operation_index),
            None,
        ));
    }

    check_snapshot_restore(footprint, snapshot, operation_index, report);
}

fn verify_extend(
    data: &SorobanTransactionData,
    extend_to: u32,
    options: VerifyOptions,
    snapshot: Option<&SnapshotIndex>,
    operation_index: usize,
    report: &mut Report,
) {
    let footprint = &data.resources.footprint;
    if !footprint.read_write.is_empty() {
        report.violations.push(violation(
            ViolationCode::ExtendNonEmptyReadWrite,
            "ExtendFootprintTtlOp requires an empty read_write footprint",
            Some(operation_index),
            None,
        ));
    }
    if footprint.read_only.is_empty() {
        report.violations.push(violation(
            ViolationCode::ExtendEmptyReadOnly,
            "ExtendFootprintTtlOp requires at least one read_only key",
            Some(operation_index),
            None,
        ));
    }
    for key in &footprint.read_only {
        if !matches!(key, LedgerKey::ContractData(_) | LedgerKey::ContractCode(_)) {
            report.violations.push(violation(
                ViolationCode::ExtendForbiddenClassicKey,
                format!(
                    "ExtendFootprintTtlOp cannot target classic {} keys",
                    ledger_key_type(key)
                ),
                Some(operation_index),
                digest_for_violation(key),
            ));
        }
    }

    if extend_to == 0 {
        report.violations.push(violation(
            ViolationCode::ExtendTtlZero,
            "extend_to must be greater than zero",
            Some(operation_index),
            None,
        ));
    }
    if extend_to > options.max_entry_ttl.saturating_sub(1) {
        report.violations.push(violation(
            ViolationCode::ExtendTtlExceedsMax,
            "extend_to exceeds max_entry_ttl minus one",
            Some(operation_index),
            None,
        ));
    }
    if data.resources.instructions != 0 {
        report.violations.push(violation(
            ViolationCode::ExtendNonZeroInstructions,
            "ExtendFootprintTtlOp requires instructions to be zero",
            Some(operation_index),
            None,
        ));
    }
    if data.resources.write_bytes != 0 {
        report.violations.push(violation(
            ViolationCode::ExtendNonZeroWriteBytes,
            "ExtendFootprintTtlOp requires write_bytes to be zero",
            Some(operation_index),
            None,
        ));
    }
    if data.resource_fee <= 0 {
        report.violations.push(violation(
            ViolationCode::ExtendNonPositiveResourceFee,
            "ExtendFootprintTtlOp requires a positive resource_fee",
            Some(operation_index),
            None,
        ));
    }

    if archived_indices(data).is_some_and(|indices| !indices.is_empty()) {
        report.violations.push(violation(
            ViolationCode::ExtendForbiddenArchivedEntriesExt,
            "ExtendFootprintTtlOp requires an empty archived_soroban_entries extension",
            Some(operation_index),
            None,
        ));
    }

    check_snapshot_extend(footprint, extend_to, snapshot, operation_index, report);
}

fn verify_invoke(data: &SorobanTransactionData, operation_index: usize, report: &mut Report) {
    let footprint = &data.resources.footprint;
    for key in footprint
        .read_only
        .iter()
        .chain(footprint.read_write.iter())
    {
        if is_classic_key(key) {
            report.violations.push(violation(
                ViolationCode::InvokeForbiddenClassicKey,
                format!(
                    "InvokeHostFunctionOp Soroban footprint checks reject classic {} keys",
                    ledger_key_type(key)
                ),
                Some(operation_index),
                digest_for_violation(key),
            ));
        }
    }

    let Some(indices) = archived_indices(data) else {
        return;
    };
    let mut seen = BTreeSet::new();
    let mut previous = None;
    for index in indices {
        if !seen.insert(*index) {
            report.violations.push(violation(
                ViolationCode::InvokeArchivedIndexDuplicate,
                "archived_soroban_entries contains a duplicate index",
                Some(operation_index),
                None,
            ));
        }
        if let Some(prev) = previous
            && prev > *index
        {
            report.violations.push(violation(
                ViolationCode::InvokeArchivedIndexUnsorted,
                "archived_soroban_entries must be strictly sorted",
                Some(operation_index),
                None,
            ));
        }
        previous = Some(*index);
        let Some(key) = footprint.read_write.get(*index as usize) else {
            report.violations.push(violation(
                ViolationCode::InvokeArchivedIndexOutOfBounds,
                "archived_soroban_entries index is outside the read_write footprint",
                Some(operation_index),
                None,
            ));
            continue;
        };
        let persistent = match key {
            LedgerKey::ContractCode(_) => true,
            LedgerKey::ContractData(data) => data.durability == ContractDataDurability::Persistent,
            _ => false,
        };
        if !persistent {
            report.violations.push(violation(
                ViolationCode::InvokeArchivedKeyNotPersistent,
                "archived_soroban_entries must point to persistent ContractData or ContractCode",
                Some(operation_index),
                digest_for_violation(key),
            ));
        }
    }
}

fn validate_footprint(footprint: &LedgerFootprint, report: &mut Report) {
    let read_only = validate_section("read_only", &footprint.read_only, report);
    let read_write = validate_section("read_write", &footprint.read_write, report);
    for bytes in read_only.intersection(&read_write) {
        let digest = Sha256::digest(bytes);
        report.violations.push(violation(
            ViolationCode::FootprintDuplicateKey,
            "the same LedgerKey appears in both footprint sections",
            None,
            Some(hex::encode(digest)),
        ));
    }
}

fn validate_section(section: &str, keys: &[LedgerKey], report: &mut Report) -> BTreeSet<Vec<u8>> {
    let mut seen = BTreeSet::new();
    let mut previous: Option<Vec<u8>> = None;
    for key in keys {
        let Ok(bytes) = key.to_xdr(limits()) else {
            report.violations.push(violation(
                ViolationCode::KeyXdrEncodingFailed,
                format!("could not encode a {section} LedgerKey as canonical XDR"),
                None,
                None,
            ));
            continue;
        };
        if !seen.insert(bytes.clone()) {
            report.violations.push(violation(
                ViolationCode::FootprintDuplicateKey,
                format!("{section} footprint contains a duplicate LedgerKey"),
                None,
                Some(hex::encode(Sha256::digest(&bytes))),
            ));
        }
        if let Some(prev) = &previous
            && prev > &bytes
        {
            report.violations.push(violation(
                ViolationCode::FootprintUnsorted,
                format!("{section} footprint is not in canonical XDR byte order"),
                None,
                Some(hex::encode(Sha256::digest(&bytes))),
            ));
        }
        previous = Some(bytes);
    }
    seen
}

fn append_key_reports(
    footprint: &LedgerFootprint,
    snapshot: Option<&SnapshotIndex>,
    require_complete_snapshot: bool,
    report: &mut Report,
) {
    for (section, keys) in [
        ("read_only", &footprint.read_only),
        ("read_write", &footprint.read_write),
    ] {
        for (index, key) in keys.iter().enumerate() {
            let (encoded, digest) = match encoded_key(key) {
                Ok(value) => value,
                Err(()) => {
                    report.violations.push(violation(
                        ViolationCode::KeyXdrEncodingFailed,
                        "could not encode a LedgerKey as canonical XDR",
                        None,
                        None,
                    ));
                    continue;
                }
            };
            let snapshot_state = snapshot.and_then(|state| state.state_for(&digest));
            let snapshot_status = match (snapshot, snapshot_state) {
                (None, _) => None,
                (Some(_), Some(_)) => Some("PRESENT".to_owned()),
                (Some(_), None) => {
                    if require_complete_snapshot {
                        report.violations.push(violation(
                            ViolationCode::SnapshotMissingKeyState,
                            "required snapshot does not contain this footprint key",
                            None,
                            Some(digest.clone()),
                        ));
                    } else {
                        report.warnings.push(ReportWarning {
                            code: "SNAPSHOT_MISSING_KEY_STATE".to_owned(),
                            message: "snapshot does not contain this footprint key; state is inconclusive"
                                .to_owned(),
                            key_sha256: Some(digest.clone()),
                        });
                    }
                    Some("INCONCLUSIVE_MISSING_FROM_SNAPSHOT".to_owned())
                }
            };
            let (key_type, durability) = key_description(key);
            report.keys.push(KeyReport {
                section: section.to_owned(),
                index,
                key_type,
                durability,
                key_xdr_base64: encoded,
                key_sha256: digest,
                snapshot_status,
                live_until_ledger_seq: snapshot_state.map(|state| state.live_until_ledger_seq),
            });
        }
    }
}

fn check_snapshot_restore(
    footprint: &LedgerFootprint,
    snapshot: Option<&SnapshotIndex>,
    operation_index: usize,
    report: &mut Report,
) {
    let Some(snapshot) = snapshot else {
        return;
    };
    for key in &footprint.read_write {
        let Some((_, digest)) = encoded_key(key).ok() else {
            continue;
        };
        if snapshot
            .state_for(&digest)
            .is_some_and(|state| state.live_until_ledger_seq >= snapshot.current_ledger())
        {
            report.violations.push(violation(
                ViolationCode::SnapshotRestoreAlreadyLiveKey,
                "snapshot says this restore target is already live",
                Some(operation_index),
                Some(digest),
            ));
        }
    }
}

fn check_snapshot_extend(
    footprint: &LedgerFootprint,
    extend_to: u32,
    snapshot: Option<&SnapshotIndex>,
    operation_index: usize,
    report: &mut Report,
) {
    let Some(snapshot) = snapshot else {
        return;
    };
    let required_live_until = snapshot.current_ledger().saturating_add(extend_to);
    for key in &footprint.read_only {
        let Some((_, digest)) = encoded_key(key).ok() else {
            continue;
        };
        let Some(state) = snapshot.state_for(&digest) else {
            continue;
        };
        if is_persistent_key(key) && state.live_until_ledger_seq < snapshot.current_ledger() {
            report.violations.push(violation(
                ViolationCode::SnapshotExtendArchivedPersistentKey,
                "snapshot says this persistent extension target is archived; restore it first",
                Some(operation_index),
                Some(digest.clone()),
            ));
        }
        if state.live_until_ledger_seq >= required_live_until {
            report.violations.push(violation(
                ViolationCode::SnapshotRedundantTtlExtension,
                "snapshot says this target already meets the requested TTL extension",
                Some(operation_index),
                Some(digest),
            ));
        }
    }
}

fn archived_indices(data: &SorobanTransactionData) -> Option<&[u32]> {
    match &data.ext {
        SorobanTransactionDataExt::V0 => None,
        SorobanTransactionDataExt::V1(extension) => Some(&extension.archived_soroban_entries),
    }
}

fn is_classic_key(key: &LedgerKey) -> bool {
    !matches!(key, LedgerKey::ContractData(_) | LedgerKey::ContractCode(_))
}

fn is_persistent_key(key: &LedgerKey) -> bool {
    match key {
        LedgerKey::ContractCode(_) => true,
        LedgerKey::ContractData(data) => data.durability == ContractDataDurability::Persistent,
        _ => false,
    }
}

fn key_description(key: &LedgerKey) -> (String, Option<String>) {
    let durability = match key {
        LedgerKey::ContractData(data) => Some(
            match data.durability {
                ContractDataDurability::Persistent => "Persistent",
                ContractDataDurability::Temporary => "Temporary",
            }
            .to_owned(),
        ),
        _ => None,
    };
    (ledger_key_type(key).to_owned(), durability)
}

fn ledger_key_type(key: &LedgerKey) -> &'static str {
    match key {
        LedgerKey::Account(_) => "Account",
        LedgerKey::Trustline(_) => "Trustline",
        LedgerKey::Offer(_) => "Offer",
        LedgerKey::Data(_) => "Data",
        LedgerKey::ClaimableBalance(_) => "ClaimableBalance",
        LedgerKey::LiquidityPool(_) => "LiquidityPool",
        LedgerKey::ContractData(_) => "ContractData",
        LedgerKey::ContractCode(_) => "ContractCode",
        LedgerKey::ConfigSetting(_) => "ConfigSetting",
        LedgerKey::Ttl(_) => "Ttl",
    }
}

fn operation_name(body: &OperationBody) -> &'static str {
    match body {
        OperationBody::InvokeHostFunction(_) => "InvokeHostFunction",
        OperationBody::ExtendFootprintTtl(_) => "ExtendFootprintTtl",
        OperationBody::RestoreFootprint(_) => "RestoreFootprint",
        _ => "Unsupported",
    }
}

fn encoded_key(key: &LedgerKey) -> Result<(String, String), ()> {
    let bytes = key.to_xdr(limits()).map_err(|_| ())?;
    let encoded = key.to_xdr_base64(limits()).map_err(|_| ())?;
    Ok((encoded, hex::encode(Sha256::digest(bytes))))
}

fn digest_for_violation(key: &LedgerKey) -> Option<String> {
    encoded_key(key).ok().map(|(_, digest)| digest)
}

fn limits() -> Limits {
    Limits {
        depth: 64,
        len: MAX_INPUT_BYTES,
    }
}
