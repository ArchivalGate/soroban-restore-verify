use std::collections::BTreeMap;

use serde::Deserialize;
use sha2::{Digest, Sha256};
use stellar_xdr::{LedgerKey, Limits, ReadXdr, WriteXdr};

use crate::io_safety::MAX_INPUT_BYTES;
use crate::model::SnapshotReport;

const SNAPSHOT_SCHEMA: &str = "soroban-restore-verify/snapshot/1.0";

#[derive(Debug, thiserror::Error)]
pub enum SnapshotError {
    #[error("snapshot JSON exceeds 1 MiB")]
    TooLarge,
    #[error("current ledger must be greater than zero")]
    InvalidCurrentLedger,
    #[error("invalid snapshot JSON: {0}")]
    InvalidJson(String),
    #[error("unsupported snapshot schema: {0}")]
    UnsupportedSchema(String),
    #[error("snapshot entry must include key_xdr_base64 or key_sha256")]
    MissingKey,
    #[error("invalid key_xdr_base64 in snapshot: {0}")]
    InvalidKeyXdr(String),
    #[error("invalid key_sha256 in snapshot")]
    InvalidKeyDigest,
    #[error("key_xdr_base64 and key_sha256 do not identify the same key")]
    MismatchedKeyForms,
    #[error("snapshot contains a duplicate ledger key: {0}")]
    DuplicateKey(String),
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SnapshotFile {
    schema: String,
    entries: Vec<SnapshotEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SnapshotEntry {
    #[serde(default)]
    key_xdr_base64: Option<String>,
    #[serde(default)]
    key_sha256: Option<String>,
    live_until_ledger_seq: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SnapshotState {
    pub live_until_ledger_seq: u32,
}

#[derive(Clone, Debug)]
pub struct SnapshotIndex {
    current_ledger: u32,
    entries: BTreeMap<String, SnapshotState>,
}

impl SnapshotIndex {
    pub fn parse(json: &[u8], current_ledger: u32) -> Result<Self, SnapshotError> {
        if json.len() > MAX_INPUT_BYTES {
            return Err(SnapshotError::TooLarge);
        }
        if current_ledger == 0 {
            return Err(SnapshotError::InvalidCurrentLedger);
        }

        let parsed: SnapshotFile = serde_json::from_slice(json)
            .map_err(|error| SnapshotError::InvalidJson(error.to_string()))?;
        if parsed.schema != SNAPSHOT_SCHEMA {
            return Err(SnapshotError::UnsupportedSchema(parsed.schema));
        }

        let mut entries = BTreeMap::new();
        for entry in parsed.entries {
            let digest = entry_digest(&entry)?;
            let state = SnapshotState {
                live_until_ledger_seq: entry.live_until_ledger_seq,
            };
            if entries.insert(digest.clone(), state).is_some() {
                return Err(SnapshotError::DuplicateKey(digest));
            }
        }
        Ok(Self {
            current_ledger,
            entries,
        })
    }

    #[must_use]
    pub fn current_ledger(&self) -> u32 {
        self.current_ledger
    }

    #[must_use]
    pub fn state_for(&self, key_sha256: &str) -> Option<SnapshotState> {
        self.entries.get(key_sha256).copied()
    }

    #[must_use]
    pub fn report_summary(&self) -> SnapshotReport {
        SnapshotReport {
            authority: "CALLER_SNAPSHOT",
            current_ledger: self.current_ledger,
        }
    }
}

fn entry_digest(entry: &SnapshotEntry) -> Result<String, SnapshotError> {
    let digest_from_xdr = if let Some(encoded) = &entry.key_xdr_base64 {
        let key = LedgerKey::from_xdr_base64(encoded, limits())
            .map_err(|error| SnapshotError::InvalidKeyXdr(error.to_string()))?;
        let canonical_bytes = key
            .to_xdr(limits())
            .map_err(|error| SnapshotError::InvalidKeyXdr(error.to_string()))?;
        Some(hex::encode(Sha256::digest(canonical_bytes)))
    } else {
        None
    };

    let digest_from_hex = entry
        .key_sha256
        .as_deref()
        .map(normalize_digest)
        .transpose()?;

    match (digest_from_xdr, digest_from_hex) {
        (None, None) => Err(SnapshotError::MissingKey),
        (Some(digest), None) | (None, Some(digest)) => Ok(digest),
        (Some(from_xdr), Some(from_hex)) if from_xdr == from_hex => Ok(from_xdr),
        (Some(_), Some(_)) => Err(SnapshotError::MismatchedKeyForms),
    }
}

fn normalize_digest(value: &str) -> Result<String, SnapshotError> {
    let bytes = hex::decode(value).map_err(|_| SnapshotError::InvalidKeyDigest)?;
    if bytes.len() != 32 {
        return Err(SnapshotError::InvalidKeyDigest);
    }
    Ok(hex::encode(bytes))
}

fn limits() -> Limits {
    Limits {
        depth: 64,
        len: MAX_INPUT_BYTES,
    }
}
