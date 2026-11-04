use std::fmt::Write as _;

use crate::model::{Report, ReportStatus};

pub fn render_json(report: &Report) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(report).map(|mut output| {
        output.push('\n');
        output
    })
}

#[must_use]
pub fn render_human(report: &Report) -> String {
    let mut output = String::new();
    let status = match report.status {
        ReportStatus::VerifiedLocal => "VERIFIED_LOCAL",
        ReportStatus::Violation => "VIOLATION",
    };
    let _ = writeln!(output, "Status: {status}");
    let _ = writeln!(output, "Authority: {}", report.authority);
    let operation = report.operation.as_deref().unwrap_or("UNKNOWN");
    let _ = writeln!(output, "Operation: {operation}");
    let _ = writeln!(
        output,
        "Fee bump: {}",
        if report.is_fee_bump { "yes" } else { "no" }
    );
    let _ = writeln!(output, "Operation count: {}", report.operation_count);
    let _ = writeln!(output, "Maximum entry TTL: {}", report.max_entry_ttl);

    if let Some(snapshot) = &report.snapshot {
        let _ = writeln!(
            output,
            "Snapshot: {} at ledger {}",
            snapshot.authority, snapshot.current_ledger
        );
    } else {
        let _ = writeln!(output, "Snapshot: not supplied");
    }

    let _ = writeln!(output, "Keys: {}", report.keys.len());
    for key in &report.keys {
        let durability = key
            .durability
            .as_deref()
            .map(|value| format!("/{value}"))
            .unwrap_or_default();
        let _ = write!(
            output,
            "  {}[{}] {}{} sha256 {}",
            key.section, key.index, key.key_type, durability, key.key_sha256
        );
        if let Some(snapshot_status) = &key.snapshot_status {
            let _ = write!(output, " snapshot {snapshot_status}");
        }
        if let Some(live_until) = key.live_until_ledger_seq {
            let _ = write!(output, " live_until_ledger_seq {live_until}");
        }
        output.push('\n');
    }

    let _ = writeln!(output, "Violations: {}", report.violations.len());
    for item in &report.violations {
        let _ = write!(output, "  {}: {}", item.code.as_str(), item.message);
        if let Some(index) = item.operation_index {
            let _ = write!(output, " operation_index {index}");
        }
        if let Some(digest) = &item.key_sha256 {
            let _ = write!(output, " key_sha256 {digest}");
        }
        output.push('\n');
    }

    let _ = writeln!(output, "Warnings: {}", report.warnings.len());
    for warning in &report.warnings {
        let _ = write!(output, "  {}: {}", warning.code, warning.message);
        if let Some(digest) = &warning.key_sha256 {
            let _ = write!(output, " key_sha256 {digest}");
        }
        output.push('\n');
    }
    output
}
