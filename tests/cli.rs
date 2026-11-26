//! End-to-end checks for the `railid` command-line helper.

use std::process::Command;

use railid::RailId;

fn railid() -> Command {
    Command::new(env!("CARGO_BIN_EXE_railid"))
}

#[test]
fn version_flag_prints_crate_version() {
    let output = railid().arg("--version").output().expect("run railid");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("utf-8");
    assert!(stdout.starts_with("railid "));
    assert!(stdout.contains(railid::VERSION));
}

#[test]
fn help_is_printed_for_no_arguments() {
    let output = railid().output().expect("run railid");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("utf-8");
    assert!(stdout.contains("railid generate"));
}

#[test]
fn degraded_generation_and_batch_are_valid() {
    let single = railid()
        .args(["generate", "--degraded"])
        .output()
        .expect("run railid");
    assert!(single.status.success());
    let single = String::from_utf8(single.stdout).expect("utf-8");
    let single = single.trim();
    assert_eq!(single.len(), 32);
    RailId::decode(single).expect("valid generated ID");

    let batch = railid()
        .args(["generate-many", "3", "--degraded"])
        .output()
        .expect("run railid");
    assert!(batch.status.success());
    let batch = String::from_utf8(batch.stdout).expect("utf-8");
    let lines: Vec<_> = batch.lines().collect();
    assert_eq!(lines.len(), 3);
    for line in lines {
        assert_eq!(line.len(), 32);
        RailId::decode(line).expect("valid batch ID");
    }
}

#[test]
fn decode_and_inspect_round_trip() {
    let id = RailId::from_parts(42, [0u8; 19]).expect("known ID");
    let encoded = id.encode();

    let decoded = railid()
        .args(["decode", &encoded])
        .output()
        .expect("run railid");
    assert!(decoded.status.success());
    assert_eq!(
        String::from_utf8(decoded.stdout).expect("utf-8").trim(),
        encoded
    );

    let inspected = railid()
        .args(["inspect", &encoded])
        .output()
        .expect("run railid");
    assert!(inspected.status.success());
    let inspected = String::from_utf8(inspected.stdout).expect("utf-8");
    assert!(inspected.contains("timestamp_ticks: 42"));
    assert!(inspected.contains(&encoded));
}

#[test]
fn invalid_input_exits_with_code_two() {
    let invalid = railid()
        .args(["decode", "not-a-railid"])
        .output()
        .expect("run railid");
    assert_eq!(invalid.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&invalid.stderr).contains("decode failed"));

    let unknown = railid().arg("wat").output().expect("run railid");
    assert_eq!(unknown.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&unknown.stderr).contains("unknown command"));
}
