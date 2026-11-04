use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use soroban_restore_verify::fixtures::{
    EnvelopeSpec, FixtureName, build_envelope, contract_code_key, contract_data_key,
    extend_operation, fee_bump, fixture_envelope, invoke_operation, restore_operation,
    sample_snapshot_for_key,
};
use stellar_xdr::{
    ContractDataDurability, LedgerFootprint, LedgerKey, LedgerKeyAccount, Limits, OperationBody,
    PaymentOp, SorobanResourcesExtV0, SorobanTransactionDataExt, TransactionEnvelope,
    TransactionV0Envelope, WriteXdr,
};

fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_soroban-restore-verify"))
}

fn fixture_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(name)
}

fn run(args: &[String]) -> Output {
    cli().args(args).output().expect("run CLI")
}

fn run_envelope(envelope: &TransactionEnvelope, extra: &[String]) -> Output {
    let encoded = envelope
        .to_xdr_base64(limits())
        .expect("encode test envelope");
    let mut args = vec!["--format".to_owned(), "json".to_owned()];
    args.extend(["--xdr".to_owned(), encoded]);
    args.extend_from_slice(extra);
    run(&args)
}

fn run_fixture(name: &str, extra: &[String]) -> Output {
    let mut args = vec!["--format".to_owned(), "json".to_owned()];
    args.extend([
        "--input".to_owned(),
        fixture_path(name).display().to_string(),
    ]);
    args.extend_from_slice(extra);
    run(&args)
}

fn report(output: &Output) -> serde_json::Value {
    serde_json::from_slice(&output.stdout).expect("JSON report on stdout")
}

fn codes(output: &Output) -> Vec<String> {
    report(output)["violations"]
        .as_array()
        .expect("violations array")
        .iter()
        .map(|violation| {
            violation["code"]
                .as_str()
                .expect("violation code")
                .to_owned()
        })
        .collect()
}

fn footprint(read_only: Vec<LedgerKey>, read_write: Vec<LedgerKey>) -> LedgerFootprint {
    LedgerFootprint {
        read_only: read_only.try_into().expect("read_only fixture fits"),
        read_write: read_write.try_into().expect("read_write fixture fits"),
    }
}

fn make_envelope(
    operations: Vec<OperationBody>,
    footprint: LedgerFootprint,
    instructions: u32,
    disk_read_bytes: u32,
    write_bytes: u32,
    resource_fee: i64,
    extension: SorobanTransactionDataExt,
) -> TransactionEnvelope {
    build_envelope(EnvelopeSpec {
        operation_bodies: operations,
        footprint,
        instructions,
        disk_read_bytes,
        write_bytes,
        resource_fee,
        data_ext: extension,
    })
}

fn restore(read_only: Vec<LedgerKey>, read_write: Vec<LedgerKey>) -> TransactionEnvelope {
    make_envelope(
        vec![restore_operation()],
        footprint(read_only, read_write),
        0,
        8,
        8,
        100,
        SorobanTransactionDataExt::V0,
    )
}

fn extend(
    extend_to: u32,
    read_only: Vec<LedgerKey>,
    read_write: Vec<LedgerKey>,
) -> TransactionEnvelope {
    make_envelope(
        vec![extend_operation(extend_to)],
        footprint(read_only, read_write),
        0,
        8,
        0,
        100,
        SorobanTransactionDataExt::V0,
    )
}

fn snapshot_file(dir: &Path, key: &LedgerKey, live_until: u32) -> PathBuf {
    let path = dir.join("snapshot.json");
    fs::write(
        &path,
        sample_snapshot_for_key(key, live_until).expect("snapshot fixture"),
    )
    .expect("write snapshot");
    path
}

fn with_snapshot(path: &Path, current_ledger: u32) -> Vec<String> {
    vec![
        "--snapshot".to_owned(),
        path.display().to_string(),
        "--current-ledger".to_owned(),
        current_ledger.to_string(),
    ]
}

fn has_code(output: &Output, expected: &str) -> bool {
    codes(output).iter().any(|code| code == expected)
}

#[test]
fn valid_restore_file_reports_verified_local_deterministically() {
    let output = run_fixture("valid-restore.xdr", &[]);
    let repeated = run_fixture("valid-restore.xdr", &[]);
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    assert_eq!(report(&output)["status"], "VERIFIED_LOCAL");
    assert_eq!(report(&output)["authority"], "AUTHORITATIVE_PROTOCOL_XDR");
    assert_eq!(output.stdout, repeated.stdout);
}

#[test]
fn valid_extend_works_with_and_without_caller_snapshot() {
    let without_snapshot = run_fixture("valid-extend.xdr", &[]);
    assert_eq!(without_snapshot.status.code(), Some(0));
    assert_eq!(report(&without_snapshot)["status"], "VERIFIED_LOCAL");

    let snapshot = fixture_path("sample-snapshot.json");
    let with_snapshot = run_fixture("valid-extend.xdr", &with_snapshot(&snapshot, 100));
    assert_eq!(with_snapshot.status.code(), Some(0));
    assert_eq!(
        report(&with_snapshot)["snapshot"]["authority"],
        "CALLER_SNAPSHOT"
    );
    assert_eq!(
        report(&with_snapshot)["keys"][0]["live_until_ledger_seq"],
        150
    );
}

#[test]
fn valid_raw_xdr_file_and_stdin_are_supported() {
    let envelope = fixture_envelope(FixtureName::ValidRestore);
    let raw = envelope.to_xdr(limits()).expect("raw envelope XDR");
    let dir = tempfile::tempdir().expect("temporary directory");
    let raw_path = dir.path().join("envelope.xdr");
    fs::write(&raw_path, raw).expect("write raw XDR");
    let raw_output = run(&[
        "--format".to_owned(),
        "json".to_owned(),
        "--raw-xdr".to_owned(),
        "--input".to_owned(),
        raw_path.display().to_string(),
    ]);
    assert_eq!(raw_output.status.code(), Some(0));

    let encoded = fs::read(fixture_path("valid-restore.xdr")).expect("read base64 fixture");
    let mut child = cli()
        .args(["--input", "-", "--format", "json"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn CLI for stdin");
    child
        .stdin
        .take()
        .expect("stdin pipe")
        .write_all(&encoded)
        .expect("write stdin input");
    let stdin_output = child.wait_with_output().expect("wait for CLI");
    assert_eq!(stdin_output.status.code(), Some(0));
}

#[test]
fn fee_bump_envelope_is_unwrapped_and_reported() {
    let envelope = fee_bump(fixture_envelope(FixtureName::ValidRestore));
    let output = run_envelope(&envelope, &[]);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(report(&output)["is_fee_bump"], true);
}

#[test]
fn restore_rejects_non_empty_read_only_temporary_and_classic_keys() {
    let persistent = contract_data_key(ContractDataDurability::Persistent);
    let non_empty_read_only =
        run_envelope(&restore(vec![persistent.clone()], vec![persistent]), &[]);
    assert_eq!(non_empty_read_only.status.code(), Some(2));
    assert!(has_code(
        &non_empty_read_only,
        "RESTORE_NON_EMPTY_READ_ONLY"
    ));

    let temporary = run_fixture("invalid-restore-temp.xdr", &[]);
    assert_eq!(temporary.status.code(), Some(2));
    assert!(has_code(&temporary, "RESTORE_FORBIDDEN_TEMPORARY_KEY"));

    let classic = run_envelope(
        &restore(
            Vec::new(),
            vec![LedgerKey::Account(LedgerKeyAccount::default())],
        ),
        &[],
    );
    assert_eq!(classic.status.code(), Some(2));
    assert!(has_code(&classic, "RESTORE_FORBIDDEN_CLASSIC_KEY"));
}

#[test]
fn extend_rejects_unsorted_duplicate_and_over_limit_footprints() {
    let unsorted = run_fixture("invalid-extend-unsorted.xdr", &[]);
    assert_eq!(unsorted.status.code(), Some(2));
    assert!(has_code(&unsorted, "FOOTPRINT_UNSORTED"));

    let duplicate_key = contract_code_key(0);
    let duplicate = run_envelope(
        &extend(100, vec![duplicate_key.clone(), duplicate_key], Vec::new()),
        &[],
    );
    assert_eq!(duplicate.status.code(), Some(2));
    assert!(has_code(&duplicate, "FOOTPRINT_DUPLICATE_KEY"));

    let over_limit = run_envelope(
        &extend(
            3_110_400,
            vec![contract_data_key(ContractDataDurability::Persistent)],
            Vec::new(),
        ),
        &[],
    );
    assert_eq!(over_limit.status.code(), Some(2));
    assert!(has_code(&over_limit, "EXTEND_TTL_EXCEEDS_MAX"));
}

#[test]
fn mixed_operations_fail_operation_isolation() {
    let envelope = make_envelope(
        vec![invoke_operation(), restore_operation()],
        footprint(
            Vec::new(),
            vec![contract_data_key(ContractDataDurability::Persistent)],
        ),
        0,
        8,
        8,
        100,
        SorobanTransactionDataExt::V0,
    );
    let output = run_envelope(&envelope, &[]);
    assert_eq!(output.status.code(), Some(2));
    assert!(has_code(&output, "OP_NOT_ISOLATED"));
}

#[test]
fn snapshot_checks_archived_redundant_and_already_live_entries() {
    let dir = tempfile::tempdir().expect("temporary directory");
    let key = contract_data_key(ContractDataDurability::Persistent);

    let archived = snapshot_file(dir.path(), &key, 99);
    let archived_output = run_envelope(
        &extend(100, vec![key.clone()], Vec::new()),
        &with_snapshot(&archived, 100),
    );
    assert_eq!(archived_output.status.code(), Some(2));
    assert!(has_code(
        &archived_output,
        "SNAPSHOT_EXTEND_ARCHIVED_PERSISTENT_KEY"
    ));

    let redundant = snapshot_file(dir.path(), &key, 200);
    let redundant_output = run_envelope(
        &extend(100, vec![key.clone()], Vec::new()),
        &with_snapshot(&redundant, 100),
    );
    assert_eq!(redundant_output.status.code(), Some(2));
    assert!(has_code(
        &redundant_output,
        "SNAPSHOT_REDUNDANT_TTL_EXTENSION"
    ));

    let already_live = snapshot_file(dir.path(), &key, 100);
    let live_output = run_envelope(
        &restore(Vec::new(), vec![key.clone()]),
        &with_snapshot(&already_live, 100),
    );
    assert_eq!(live_output.status.code(), Some(2));
    assert!(has_code(&live_output, "SNAPSHOT_RESTORE_ALREADY_LIVE_KEY"));
}

#[cfg(unix)]
#[test]
fn malformed_oversized_and_symlink_inputs_fail_closed() {
    let truncated = run_fixture("truncated.xdr", &[]);
    assert_eq!(truncated.status.code(), Some(3));
    assert!(String::from_utf8_lossy(&truncated.stderr).contains("XDR_INVALID"));

    let dir = tempfile::tempdir().expect("temporary directory");
    let oversized = dir.path().join("oversized.xdr");
    fs::write(&oversized, vec![b'A'; 1_048_577]).expect("write oversized input");
    let oversized_output = run(&[
        "--input".to_owned(),
        oversized.display().to_string(),
        "--format".to_owned(),
        "json".to_owned(),
    ]);
    assert_eq!(oversized_output.status.code(), Some(3));
    assert!(String::from_utf8_lossy(&oversized_output.stderr).contains("INPUT_TOO_LARGE"));

    let link = dir.path().join("input-link");
    std::os::unix::fs::symlink(fixture_path("valid-restore.xdr"), &link)
        .expect("create input symlink");
    let link_output = run(&[
        "--input".to_owned(),
        link.display().to_string(),
        "--format".to_owned(),
        "json".to_owned(),
    ]);
    assert_eq!(link_output.status.code(), Some(3));
    assert!(String::from_utf8_lossy(&link_output.stderr).contains("SYMLINK_REFUSED"));
}

#[cfg(unix)]
#[test]
fn output_paths_are_no_clobber_and_symlinks_are_refused() {
    let dir = tempfile::tempdir().expect("temporary directory");
    let existing = dir.path().join("existing.json");
    fs::write(&existing, b"keep").expect("create output file");
    let existing_output = run(&[
        "--input".to_owned(),
        fixture_path("valid-restore.xdr").display().to_string(),
        "--output".to_owned(),
        existing.display().to_string(),
    ]);
    assert_eq!(existing_output.status.code(), Some(3));
    assert!(String::from_utf8_lossy(&existing_output.stderr).contains("OUTPUT_EXISTS"));
    assert_eq!(fs::read(&existing).expect("read existing output"), b"keep");

    let link = dir.path().join("output-link");
    std::os::unix::fs::symlink(&existing, &link).expect("create output symlink");
    let symlink_output = run(&[
        "--input".to_owned(),
        fixture_path("valid-restore.xdr").display().to_string(),
        "--output".to_owned(),
        link.display().to_string(),
    ]);
    assert_eq!(symlink_output.status.code(), Some(3));
    assert!(String::from_utf8_lossy(&symlink_output.stderr).contains("SYMLINK_REFUSED"));

    let fresh = dir.path().join("new.json");
    let success = run(&[
        "--input".to_owned(),
        fixture_path("valid-restore.xdr").display().to_string(),
        "--format".to_owned(),
        "json".to_owned(),
        "--output".to_owned(),
        fresh.display().to_string(),
    ]);
    assert_eq!(success.status.code(), Some(0));
    assert!(success.stdout.is_empty());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&fs::read(fresh).expect("read report"))
            .expect("valid JSON report")["status"],
        "VERIFIED_LOCAL"
    );
}

#[test]
fn unknown_snapshot_fields_and_invalid_cli_arguments_fail_with_three() {
    let dir = tempfile::tempdir().expect("temporary directory");
    let snapshot = dir.path().join("unknown-field.json");
    fs::write(
        &snapshot,
        br#"{"schema":"soroban-restore-verify/snapshot/1.0","entries":[],"unexpected":true}"#,
    )
    .expect("write snapshot with unknown field");
    let unknown = run(&[
        "--input".to_owned(),
        fixture_path("valid-restore.xdr").display().to_string(),
        "--snapshot".to_owned(),
        snapshot.display().to_string(),
        "--current-ledger".to_owned(),
        "100".to_owned(),
    ]);
    assert_eq!(unknown.status.code(), Some(3));
    assert!(String::from_utf8_lossy(&unknown.stderr).contains("SNAPSHOT_INVALID_JSON"));

    let missing_current = run(&[
        "--input".to_owned(),
        fixture_path("valid-restore.xdr").display().to_string(),
        "--snapshot".to_owned(),
        fixture_path("sample-snapshot.json").display().to_string(),
    ]);
    assert_eq!(missing_current.status.code(), Some(3));
    assert!(String::from_utf8_lossy(&missing_current.stderr).contains("CLI_ARGUMENT_ERROR"));
}

#[test]
fn snapshot_missing_key_is_inconclusive_or_a_required_violation() {
    let key = contract_data_key(ContractDataDurability::Persistent);
    let other = contract_data_key(ContractDataDurability::Temporary);
    let dir = tempfile::tempdir().expect("temporary directory");
    let snapshot = snapshot_file(dir.path(), &other, 200);
    let envelope = extend(100, vec![key], Vec::new());

    let optional = run_envelope(&envelope, &with_snapshot(&snapshot, 100));
    assert_eq!(optional.status.code(), Some(0));
    assert_eq!(
        report(&optional)["keys"][0]["snapshot_status"],
        "INCONCLUSIVE_MISSING_FROM_SNAPSHOT"
    );

    let mut required_args = with_snapshot(&snapshot, 100);
    required_args.push("--require-complete-snapshot".to_owned());
    let required = run_envelope(&envelope, &required_args);
    assert_eq!(required.status.code(), Some(2));
    assert!(has_code(&required, "SNAPSHOT_MISSING_KEY_STATE"));
}

#[test]
fn invoke_archived_indexes_check_bounds_sorting_uniqueness_and_durability() {
    let persistent = contract_data_key(ContractDataDurability::Persistent);
    let valid = make_envelope(
        vec![invoke_operation()],
        footprint(Vec::new(), vec![persistent.clone()]),
        10,
        8,
        0,
        100,
        SorobanTransactionDataExt::V1(SorobanResourcesExtV0 {
            archived_soroban_entries: vec![0].try_into().expect("one archived index"),
        }),
    );
    let valid_output = run_envelope(&valid, &[]);
    assert_eq!(valid_output.status.code(), Some(0));

    let out_of_bounds = make_envelope(
        vec![invoke_operation()],
        footprint(Vec::new(), vec![persistent.clone()]),
        10,
        8,
        0,
        100,
        SorobanTransactionDataExt::V1(SorobanResourcesExtV0 {
            archived_soroban_entries: vec![1].try_into().expect("one index"),
        }),
    );
    let out_of_bounds_output = run_envelope(&out_of_bounds, &[]);
    assert_eq!(out_of_bounds_output.status.code(), Some(2));
    assert!(has_code(
        &out_of_bounds_output,
        "INVOKE_ARCHIVED_INDEX_OUT_OF_BOUNDS"
    ));

    let temporary = make_envelope(
        vec![invoke_operation()],
        footprint(
            Vec::new(),
            vec![contract_data_key(ContractDataDurability::Temporary)],
        ),
        10,
        8,
        0,
        100,
        SorobanTransactionDataExt::V1(SorobanResourcesExtV0 {
            archived_soroban_entries: vec![0].try_into().expect("one index"),
        }),
    );
    let temporary_output = run_envelope(&temporary, &[]);
    assert_eq!(temporary_output.status.code(), Some(2));
    assert!(has_code(
        &temporary_output,
        "INVOKE_ARCHIVED_KEY_NOT_PERSISTENT"
    ));

    let classic = make_envelope(
        vec![invoke_operation()],
        footprint(
            vec![LedgerKey::Account(LedgerKeyAccount::default())],
            Vec::new(),
        ),
        10,
        8,
        0,
        100,
        SorobanTransactionDataExt::V0,
    );
    let classic_output = run_envelope(&classic, &[]);
    assert_eq!(classic_output.status.code(), Some(2));
    assert!(has_code(&classic_output, "INVOKE_FORBIDDEN_CLASSIC_KEY"));

    let unsorted_indexes = make_envelope(
        vec![invoke_operation()],
        footprint(Vec::new(), vec![contract_code_key(0), contract_code_key(1)]),
        10,
        8,
        0,
        100,
        SorobanTransactionDataExt::V1(SorobanResourcesExtV0 {
            archived_soroban_entries: vec![1, 0].try_into().expect("two indexes"),
        }),
    );
    let unsorted_output = run_envelope(&unsorted_indexes, &[]);
    assert_eq!(unsorted_output.status.code(), Some(2));
    assert!(has_code(&unsorted_output, "INVOKE_ARCHIVED_INDEX_UNSORTED"));

    let duplicate_indexes = make_envelope(
        vec![invoke_operation()],
        footprint(Vec::new(), vec![persistent]),
        10,
        8,
        0,
        100,
        SorobanTransactionDataExt::V1(SorobanResourcesExtV0 {
            archived_soroban_entries: vec![0, 0].try_into().expect("two indexes"),
        }),
    );
    let duplicate_output = run_envelope(&duplicate_indexes, &[]);
    assert_eq!(duplicate_output.status.code(), Some(2));
    assert!(has_code(
        &duplicate_output,
        "INVOKE_ARCHIVED_INDEX_DUPLICATE"
    ));
}

#[test]
fn verifier_checks_empty_operations_missing_soroban_data_and_resource_fields() {
    let mut empty = build_envelope(EnvelopeSpec {
        operation_bodies: Vec::new(),
        footprint: LedgerFootprint::default(),
        instructions: 0,
        disk_read_bytes: 0,
        write_bytes: 0,
        resource_fee: 0,
        data_ext: SorobanTransactionDataExt::V0,
    });
    let empty_output = run_envelope(&empty, &[]);
    assert_eq!(empty_output.status.code(), Some(2));
    assert!(has_code(&empty_output, "EMPTY_OPERATIONS"));

    if let TransactionEnvelope::Tx(ref mut tx) = empty {
        tx.tx.ext = stellar_xdr::TransactionExt::V0;
        tx.tx.operations = vec![stellar_xdr::Operation {
            body: restore_operation(),
            ..stellar_xdr::Operation::default()
        }]
        .try_into()
        .expect("one operation");
    }
    let missing_data = run_envelope(&empty, &[]);
    assert_eq!(missing_data.status.code(), Some(2));
    assert!(has_code(&missing_data, "MISSING_SOROBAN_TRANSACTION_DATA"));

    let bad_resources = make_envelope(
        vec![restore_operation()],
        footprint(
            Vec::new(),
            vec![contract_data_key(ContractDataDurability::Persistent)],
        ),
        1,
        0,
        0,
        0,
        SorobanTransactionDataExt::V1(SorobanResourcesExtV0 {
            archived_soroban_entries: vec![0].try_into().expect("one archived index"),
        }),
    );
    let bad_resource_output = run_envelope(&bad_resources, &[]);
    assert_eq!(bad_resource_output.status.code(), Some(2));
    assert!(has_code(
        &bad_resource_output,
        "RESTORE_NON_ZERO_INSTRUCTIONS"
    ));
    assert!(has_code(&bad_resource_output, "RESTORE_ZERO_WRITE_BYTES"));
    assert!(has_code(
        &bad_resource_output,
        "RESTORE_ZERO_DISK_READ_BYTES"
    ));
    assert!(has_code(
        &bad_resource_output,
        "RESTORE_NON_POSITIVE_RESOURCE_FEE"
    ));
    assert!(has_code(
        &bad_resource_output,
        "RESTORE_FORBIDDEN_ARCHIVED_ENTRIES_EXT"
    ));

    let legacy = TransactionEnvelope::TxV0(TransactionV0Envelope::default());
    let legacy_output = run_envelope(&legacy, &[]);
    assert_eq!(legacy_output.status.code(), Some(2));
    assert!(has_code(
        &legacy_output,
        "ENVELOPE_V0_UNSUPPORTED_FOR_SOROBAN"
    ));

    let unsupported = make_envelope(
        vec![OperationBody::Payment(PaymentOp::default())],
        LedgerFootprint::default(),
        0,
        0,
        0,
        0,
        SorobanTransactionDataExt::V0,
    );
    let unsupported_output = run_envelope(&unsupported, &[]);
    assert_eq!(unsupported_output.status.code(), Some(2));
    assert!(has_code(&unsupported_output, "UNSUPPORTED_OPERATION_TYPE"));
}

#[cfg(unix)]
#[test]
fn snapshot_file_symlinks_are_refused() {
    let dir = tempfile::tempdir().expect("temporary directory");
    let snapshot = fixture_path("sample-snapshot.json");
    let link = dir.path().join("snapshot-link.json");
    std::os::unix::fs::symlink(snapshot, &link).expect("create snapshot symlink");
    let output = run(&[
        "--input".to_owned(),
        fixture_path("valid-extend.xdr").display().to_string(),
        "--snapshot".to_owned(),
        link.display().to_string(),
        "--current-ledger".to_owned(),
        "100".to_owned(),
    ]);
    assert_eq!(output.status.code(), Some(3));
    assert!(String::from_utf8_lossy(&output.stderr).contains("SYMLINK_REFUSED"));
}

#[test]
fn extend_rejects_empty_or_mixed_sections_classic_keys_and_bad_resources() {
    let empty = run_envelope(&extend(100, Vec::new(), Vec::new()), &[]);
    assert_eq!(empty.status.code(), Some(2));
    assert!(has_code(&empty, "EXTEND_EMPTY_READ_ONLY"));

    let read_write = run_envelope(
        &extend(
            100,
            vec![contract_data_key(ContractDataDurability::Persistent)],
            vec![contract_data_key(ContractDataDurability::Persistent)],
        ),
        &[],
    );
    assert_eq!(read_write.status.code(), Some(2));
    assert!(has_code(&read_write, "EXTEND_NON_EMPTY_READ_WRITE"));

    let classic = run_envelope(
        &extend(
            100,
            vec![LedgerKey::Account(LedgerKeyAccount::default())],
            Vec::new(),
        ),
        &[],
    );
    assert_eq!(classic.status.code(), Some(2));
    assert!(has_code(&classic, "EXTEND_FORBIDDEN_CLASSIC_KEY"));

    let bad_resources = make_envelope(
        vec![extend_operation(0)],
        footprint(
            vec![contract_data_key(ContractDataDurability::Persistent)],
            Vec::new(),
        ),
        1,
        8,
        1,
        0,
        SorobanTransactionDataExt::V1(SorobanResourcesExtV0 {
            archived_soroban_entries: vec![0].try_into().expect("one index"),
        }),
    );
    let bad_output = run_envelope(&bad_resources, &[]);
    assert_eq!(bad_output.status.code(), Some(2));
    assert!(has_code(&bad_output, "EXTEND_TTL_ZERO"));
    assert!(has_code(&bad_output, "EXTEND_NON_ZERO_INSTRUCTIONS"));
    assert!(has_code(&bad_output, "EXTEND_NON_ZERO_WRITE_BYTES"));
    assert!(has_code(&bad_output, "EXTEND_NON_POSITIVE_RESOURCE_FEE"));
    assert!(has_code(
        &bad_output,
        "EXTEND_FORBIDDEN_ARCHIVED_ENTRIES_EXT"
    ));
}

#[test]
fn snapshot_rejects_duplicate_and_inconsistent_key_identifiers() {
    let key = contract_data_key(ContractDataDurability::Persistent);
    let entry: serde_json::Value =
        serde_json::from_slice(&sample_snapshot_for_key(&key, 150).expect("snapshot fixture"))
            .expect("snapshot JSON");
    let one_entry = entry["entries"][0].clone();
    let duplicate_snapshot = serde_json::json!({
        "schema": "soroban-restore-verify/snapshot/1.0",
        "entries": [one_entry.clone(), one_entry.clone()]
    });
    let mismatched_snapshot = serde_json::json!({
        "schema": "soroban-restore-verify/snapshot/1.0",
        "entries": [{
            "key_xdr_base64": one_entry["key_xdr_base64"],
            "key_sha256": "0000000000000000000000000000000000000000000000000000000000000000",
            "live_until_ledger_seq": 150
        }]
    });
    let dir = tempfile::tempdir().expect("temporary directory");
    for (name, snapshot, expected_code) in [
        (
            "duplicate.json",
            duplicate_snapshot,
            "SNAPSHOT_DUPLICATE_KEY",
        ),
        (
            "mismatch.json",
            mismatched_snapshot,
            "SNAPSHOT_KEY_FORMS_MISMATCH",
        ),
    ] {
        let path = dir.path().join(name);
        fs::write(
            &path,
            serde_json::to_vec(&snapshot).expect("encode snapshot"),
        )
        .expect("write snapshot");
        let output = run(&[
            "--input".to_owned(),
            fixture_path("valid-extend.xdr").display().to_string(),
            "--snapshot".to_owned(),
            path.display().to_string(),
            "--current-ledger".to_owned(),
            "100".to_owned(),
        ]);
        assert_eq!(output.status.code(), Some(3));
        assert!(String::from_utf8_lossy(&output.stderr).contains(expected_code));
    }
}

fn limits() -> Limits {
    Limits {
        depth: 64,
        len: 1_048_576,
    }
}
