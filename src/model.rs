use serde::Serialize;

pub const EXIT_OK: i32 = 0;
pub const EXIT_VIOLATION: i32 = 2;
pub const EXIT_ERROR: i32 = 3;
pub const REPORT_SCHEMA: &str = "soroban-restore-verify/1.0";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReportStatus {
    VerifiedLocal,
    Violation,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ViolationCode {
    EnvelopeV0UnsupportedForSoroban,
    EmptyOperations,
    OpNotIsolated,
    UnsupportedOperationType,
    MissingSorobanTransactionData,
    FootprintDuplicateKey,
    FootprintUnsorted,
    KeyXdrEncodingFailed,
    RestoreNonEmptyReadOnly,
    RestoreEmptyReadWrite,
    RestoreForbiddenTemporaryKey,
    RestoreForbiddenClassicKey,
    RestoreNonZeroInstructions,
    RestoreZeroWriteBytes,
    RestoreZeroDiskReadBytes,
    RestoreNonPositiveResourceFee,
    RestoreForbiddenArchivedEntriesExt,
    ExtendNonEmptyReadWrite,
    ExtendEmptyReadOnly,
    ExtendForbiddenClassicKey,
    ExtendTtlZero,
    ExtendTtlExceedsMax,
    ExtendNonZeroInstructions,
    ExtendNonZeroWriteBytes,
    ExtendNonPositiveResourceFee,
    ExtendForbiddenArchivedEntriesExt,
    InvokeForbiddenClassicKey,
    InvokeArchivedIndexOutOfBounds,
    InvokeArchivedIndexUnsorted,
    InvokeArchivedIndexDuplicate,
    InvokeArchivedKeyNotPersistent,
    SnapshotRestoreAlreadyLiveKey,
    SnapshotExtendArchivedPersistentKey,
    SnapshotRedundantTtlExtension,
    SnapshotMissingKeyState,
}

impl ViolationCode {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::EnvelopeV0UnsupportedForSoroban => "ENVELOPE_V0_UNSUPPORTED_FOR_SOROBAN",
            Self::EmptyOperations => "EMPTY_OPERATIONS",
            Self::OpNotIsolated => "OP_NOT_ISOLATED",
            Self::UnsupportedOperationType => "UNSUPPORTED_OPERATION_TYPE",
            Self::MissingSorobanTransactionData => "MISSING_SOROBAN_TRANSACTION_DATA",
            Self::FootprintDuplicateKey => "FOOTPRINT_DUPLICATE_KEY",
            Self::FootprintUnsorted => "FOOTPRINT_UNSORTED",
            Self::KeyXdrEncodingFailed => "KEY_XDR_ENCODING_FAILED",
            Self::RestoreNonEmptyReadOnly => "RESTORE_NON_EMPTY_READ_ONLY",
            Self::RestoreEmptyReadWrite => "RESTORE_EMPTY_READ_WRITE",
            Self::RestoreForbiddenTemporaryKey => "RESTORE_FORBIDDEN_TEMPORARY_KEY",
            Self::RestoreForbiddenClassicKey => "RESTORE_FORBIDDEN_CLASSIC_KEY",
            Self::RestoreNonZeroInstructions => "RESTORE_NON_ZERO_INSTRUCTIONS",
            Self::RestoreZeroWriteBytes => "RESTORE_ZERO_WRITE_BYTES",
            Self::RestoreZeroDiskReadBytes => "RESTORE_ZERO_DISK_READ_BYTES",
            Self::RestoreNonPositiveResourceFee => "RESTORE_NON_POSITIVE_RESOURCE_FEE",
            Self::RestoreForbiddenArchivedEntriesExt => "RESTORE_FORBIDDEN_ARCHIVED_ENTRIES_EXT",
            Self::ExtendNonEmptyReadWrite => "EXTEND_NON_EMPTY_READ_WRITE",
            Self::ExtendEmptyReadOnly => "EXTEND_EMPTY_READ_ONLY",
            Self::ExtendForbiddenClassicKey => "EXTEND_FORBIDDEN_CLASSIC_KEY",
            Self::ExtendTtlZero => "EXTEND_TTL_ZERO",
            Self::ExtendTtlExceedsMax => "EXTEND_TTL_EXCEEDS_MAX",
            Self::ExtendNonZeroInstructions => "EXTEND_NON_ZERO_INSTRUCTIONS",
            Self::ExtendNonZeroWriteBytes => "EXTEND_NON_ZERO_WRITE_BYTES",
            Self::ExtendNonPositiveResourceFee => "EXTEND_NON_POSITIVE_RESOURCE_FEE",
            Self::ExtendForbiddenArchivedEntriesExt => "EXTEND_FORBIDDEN_ARCHIVED_ENTRIES_EXT",
            Self::InvokeForbiddenClassicKey => "INVOKE_FORBIDDEN_CLASSIC_KEY",
            Self::InvokeArchivedIndexOutOfBounds => "INVOKE_ARCHIVED_INDEX_OUT_OF_BOUNDS",
            Self::InvokeArchivedIndexUnsorted => "INVOKE_ARCHIVED_INDEX_UNSORTED",
            Self::InvokeArchivedIndexDuplicate => "INVOKE_ARCHIVED_INDEX_DUPLICATE",
            Self::InvokeArchivedKeyNotPersistent => "INVOKE_ARCHIVED_KEY_NOT_PERSISTENT",
            Self::SnapshotRestoreAlreadyLiveKey => "SNAPSHOT_RESTORE_ALREADY_LIVE_KEY",
            Self::SnapshotExtendArchivedPersistentKey => "SNAPSHOT_EXTEND_ARCHIVED_PERSISTENT_KEY",
            Self::SnapshotRedundantTtlExtension => "SNAPSHOT_REDUNDANT_TTL_EXTENSION",
            Self::SnapshotMissingKeyState => "SNAPSHOT_MISSING_KEY_STATE",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Violation {
    pub code: ViolationCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operation_index: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_sha256: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ReportWarning {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_sha256: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SnapshotReport {
    pub authority: &'static str,
    pub current_ledger: u32,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct KeyReport {
    pub section: String,
    pub index: usize,
    pub key_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub durability: Option<String>,
    pub key_xdr_base64: String,
    pub key_sha256: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub live_until_ledger_seq: Option<u32>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Report {
    pub schema: &'static str,
    pub status: ReportStatus,
    pub authority: &'static str,
    pub operation: Option<String>,
    pub is_fee_bump: bool,
    pub operation_count: usize,
    pub max_entry_ttl: u32,
    pub snapshot: Option<SnapshotReport>,
    pub keys: Vec<KeyReport>,
    pub violations: Vec<Violation>,
    pub warnings: Vec<ReportWarning>,
}

impl Report {
    #[must_use]
    pub fn new(max_entry_ttl: u32, snapshot: Option<SnapshotReport>) -> Self {
        Self {
            schema: REPORT_SCHEMA,
            status: ReportStatus::VerifiedLocal,
            authority: "AUTHORITATIVE_PROTOCOL_XDR",
            operation: None,
            is_fee_bump: false,
            operation_count: 0,
            max_entry_ttl,
            snapshot,
            keys: Vec::new(),
            violations: Vec::new(),
            warnings: Vec::new(),
        }
    }

    pub fn finish(&mut self) {
        if !self.violations.is_empty() {
            self.status = ReportStatus::Violation;
        }
    }
}

pub fn violation(
    code: ViolationCode,
    message: impl Into<String>,
    operation_index: Option<usize>,
    key_sha256: Option<String>,
) -> Violation {
    Violation {
        code,
        message: message.into(),
        operation_index,
        key_sha256,
    }
}
