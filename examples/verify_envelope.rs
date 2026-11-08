use std::error::Error;
use std::io::{self, Write};

use soroban_restore_verify::model::ReportStatus as Status;
use soroban_restore_verify::verifier::{VerifyOptions, verify_envelope};
use soroban_restore_verify::xdr_decoder::{InputEncoding, decode_envelope};

const VALID_RESTORE_XDR: &[u8] = include_bytes!("../fixtures/valid-restore.xdr");
const INVALID_RESTORE_XDR: &[u8] = include_bytes!("../fixtures/invalid-restore-temp.xdr");

fn main() -> Result<(), Box<dyn Error>> {
    let cases = [
        ("valid restore", VALID_RESTORE_XDR, Status::VerifiedLocal),
        ("invalid restore", INVALID_RESTORE_XDR, Status::Violation),
    ];
    let mut stdout = io::stdout().lock();

    for (name, xdr, expected_status) in cases {
        let envelope = decode_envelope(xdr, InputEncoding::Base64)?;
        let report = verify_envelope(&envelope, VerifyOptions::default(), None);

        // Return an error on a failed status assertion instead of panicking.
        if report.status != expected_status {
            return Err(format!(
                "{name}: expected {expected_status:?}, got {:?}",
                report.status
            )
            .into());
        }

        writeln!(stdout, "{name}: {:?}", report.status)?;
    }

    Ok(())
}
