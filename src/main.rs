use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, ValueEnum, error::ErrorKind};
use soroban_restore_verify::fixtures::{FixtureName, fixture_payload};
use soroban_restore_verify::io_safety::{
    MAX_INPUT_BYTES, SafetyError, check_output_available, read_bounded_file, read_bounded_stdin,
    write_output_atomic_no_clobber,
};
use soroban_restore_verify::model::{EXIT_ERROR, EXIT_OK, EXIT_VIOLATION, ReportStatus};
use soroban_restore_verify::report::{render_human, render_json};
use soroban_restore_verify::snapshot::{SnapshotError, SnapshotIndex};
use soroban_restore_verify::verifier::{VerifyOptions, verify_envelope};
use soroban_restore_verify::xdr_decoder::{DecodeError, InputEncoding, decode_envelope};

#[derive(Debug, Parser)]
#[command(
    name = "soroban-restore-verify",
    version,
    about = "Check Soroban restore and TTL extension envelopes from local XDR"
)]
struct Cli {
    #[arg(
        short,
        long,
        value_name = "PATH",
        conflicts_with = "xdr",
        conflicts_with = "dump_fixture"
    )]
    input: Option<String>,

    #[arg(long, requires = "input", conflicts_with = "xdr")]
    raw_xdr: bool,

    #[arg(
        long,
        value_name = "BASE64",
        conflicts_with = "input",
        conflicts_with = "dump_fixture"
    )]
    xdr: Option<String>,

    #[arg(long, value_name = "PATH", conflicts_with = "dump_fixture")]
    snapshot: Option<PathBuf>,

    #[arg(long, value_name = "U32", requires = "snapshot")]
    current_ledger: Option<u32>,

    #[arg(long, value_name = "U32", default_value_t = 3_110_400)]
    max_entry_ttl: u32,

    #[arg(long, requires = "snapshot")]
    require_complete_snapshot: bool,

    #[arg(long, value_enum, default_value = "human")]
    format: OutputFormat,

    #[arg(short, long, value_name = "PATH", conflicts_with = "dump_fixture")]
    output: Option<PathBuf>,

    #[arg(
        long,
        value_enum,
        value_name = "NAME",
        conflicts_with_all = ["input", "raw_xdr", "xdr", "snapshot", "current_ledger", "require_complete_snapshot", "output"]
    )]
    dump_fixture: Option<FixtureName>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
#[value(rename_all = "lower")]
enum OutputFormat {
    Json,
    Human,
}

fn main() -> ExitCode {
    ExitCode::from(run() as u8)
}

fn run() -> i32 {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(error) => match error.kind() {
            ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => {
                print!("{error}");
                return EXIT_OK;
            }
            _ => {
                diagnostic("CLI_ARGUMENT_ERROR", &error.to_string());
                return EXIT_ERROR;
            }
        },
    };

    if let Some(name) = cli.dump_fixture {
        let payload = match fixture_payload(name) {
            Ok(payload) => payload,
            Err(error) => {
                diagnostic("FIXTURE_ERROR", &error.to_string());
                return EXIT_ERROR;
            }
        };
        return match io::stdout().write_all(&payload) {
            Ok(()) => EXIT_OK,
            Err(error) => {
                diagnostic("STDOUT_ERROR", &error.to_string());
                EXIT_ERROR
            }
        };
    }

    if cli.input.is_none() == cli.xdr.is_none() {
        diagnostic(
            "CLI_ARGUMENT_ERROR",
            "provide exactly one of --input or --xdr",
        );
        return EXIT_ERROR;
    }
    if cli.require_complete_snapshot && cli.snapshot.is_none() {
        diagnostic(
            "CLI_ARGUMENT_ERROR",
            "--require-complete-snapshot requires --snapshot",
        );
        return EXIT_ERROR;
    }
    if cli.snapshot.is_some() && cli.current_ledger.is_none() {
        diagnostic("CLI_ARGUMENT_ERROR", "--snapshot requires --current-ledger");
        return EXIT_ERROR;
    }

    if let Some(output_path) = &cli.output
        && let Err(error) = check_output_available(output_path)
    {
        safety_diagnostic("output", &error);
        return EXIT_ERROR;
    }

    let input = match read_input(&cli) {
        Ok(input) => input,
        Err(error) => {
            safety_diagnostic("input", &error);
            return EXIT_ERROR;
        }
    };
    let encoding = if cli.raw_xdr {
        InputEncoding::RawXdr
    } else {
        InputEncoding::Base64
    };
    let envelope = match decode_envelope(&input, encoding) {
        Ok(envelope) => envelope,
        Err(error) => {
            let code = match error {
                DecodeError::TooLarge => "INPUT_TOO_LARGE",
                DecodeError::Invalid(_) => "XDR_INVALID",
            };
            diagnostic(code, &error.to_string());
            return EXIT_ERROR;
        }
    };

    let snapshot = match (&cli.snapshot, cli.current_ledger) {
        (Some(path), Some(current_ledger)) => {
            let bytes = match read_bounded_file(path) {
                Ok(bytes) => bytes,
                Err(error) => {
                    safety_diagnostic("snapshot", &error);
                    return EXIT_ERROR;
                }
            };
            match SnapshotIndex::parse(&bytes, current_ledger) {
                Ok(snapshot) => Some(snapshot),
                Err(error) => {
                    snapshot_diagnostic(&error);
                    return EXIT_ERROR;
                }
            }
        }
        (None, None) => None,
        _ => {
            diagnostic(
                "CLI_ARGUMENT_ERROR",
                "--snapshot and --current-ledger must be supplied together",
            );
            return EXIT_ERROR;
        }
    };

    let report = verify_envelope(
        &envelope,
        VerifyOptions {
            max_entry_ttl: cli.max_entry_ttl,
            require_complete_snapshot: cli.require_complete_snapshot,
        },
        snapshot.as_ref(),
    );
    let output = match cli.format {
        OutputFormat::Json => match render_json(&report) {
            Ok(output) => output,
            Err(error) => {
                diagnostic("REPORT_ERROR", &error.to_string());
                return EXIT_ERROR;
            }
        },
        OutputFormat::Human => render_human(&report),
    };

    let write_result = if let Some(path) = &cli.output {
        write_output_atomic_no_clobber(path, output.as_bytes())
    } else {
        return match io::stdout().write_all(output.as_bytes()) {
            Ok(()) => report_exit_code(report.status),
            Err(error) => {
                diagnostic("STDOUT_ERROR", &error.to_string());
                EXIT_ERROR
            }
        };
    };
    if let Err(error) = write_result {
        safety_diagnostic("output", &error);
        return EXIT_ERROR;
    }
    report_exit_code(report.status)
}

fn read_input(cli: &Cli) -> Result<Vec<u8>, SafetyError> {
    if let Some(encoded) = &cli.xdr {
        if encoded.len() > MAX_INPUT_BYTES {
            return Err(SafetyError::TooLarge(PathBuf::from("--xdr")));
        }
        return Ok(encoded.as_bytes().to_vec());
    }
    let input = cli.input.as_deref().expect("input source was checked");
    if input == "-" {
        read_bounded_stdin(io::stdin().lock())
    } else {
        read_bounded_file(&PathBuf::from(input))
    }
}

fn report_exit_code(status: ReportStatus) -> i32 {
    match status {
        ReportStatus::VerifiedLocal => EXIT_OK,
        ReportStatus::Violation => {
            diagnostic(
                "VERIFICATION_VIOLATION",
                "one or more policy invariants failed",
            );
            EXIT_VIOLATION
        }
    }
}

fn safety_diagnostic(context: &str, error: &SafetyError) {
    let code = match error {
        SafetyError::Symlink(_) => "SYMLINK_REFUSED",
        SafetyError::NotRegular(_) => "NON_REGULAR_FILE",
        SafetyError::TooLarge(_) => "INPUT_TOO_LARGE",
        SafetyError::OutputExists(_) => "OUTPUT_EXISTS",
        SafetyError::Io { .. } => "FILESYSTEM_ERROR",
    };
    diagnostic(code, &format!("{context}: {error}"));
}

fn snapshot_diagnostic(error: &SnapshotError) {
    let code = match error {
        SnapshotError::TooLarge => "SNAPSHOT_TOO_LARGE",
        SnapshotError::InvalidCurrentLedger => "SNAPSHOT_INVALID_CURRENT_LEDGER",
        SnapshotError::InvalidJson(_) => "SNAPSHOT_INVALID_JSON",
        SnapshotError::UnsupportedSchema(_) => "SNAPSHOT_UNSUPPORTED_SCHEMA",
        SnapshotError::MissingKey => "SNAPSHOT_KEY_MISSING",
        SnapshotError::InvalidKeyXdr(_) => "SNAPSHOT_KEY_XDR_INVALID",
        SnapshotError::InvalidKeyDigest => "SNAPSHOT_KEY_SHA256_INVALID",
        SnapshotError::MismatchedKeyForms => "SNAPSHOT_KEY_FORMS_MISMATCH",
        SnapshotError::DuplicateKey(_) => "SNAPSHOT_DUPLICATE_KEY",
    };
    diagnostic(code, &error.to_string());
}

fn diagnostic(code: &str, message: &str) {
    let encoded_message =
        serde_json::to_string(message).unwrap_or_else(|_| "\"diagnostic error\"".to_owned());
    eprintln!("{{\"code\":\"{code}\",\"message\":{encoded_message}}}");
}
