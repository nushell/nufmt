use std::{fs, process::Command};
use tempfile::tempdir;

const INVALID: &str = "# beginning of script comment

let one = 1
";
const VALID: &str = "# beginning of script comment
let one = 1
";
const TEST_BINARY: &'static str = "target/debug/nufmt";

#[test]
fn failure_with_invalid_config() {
    let dir = tempdir().unwrap();
    let config_file = dir.path().join("nufmt.nuon");
    fs::write(&config_file, r#"{unknown: 1}"#).unwrap();

    let output = Command::new(TEST_BINARY)
        .arg("--config")
        .arg(config_file.to_str().unwrap())
        .arg(dir.path().to_str().unwrap())
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(stderr.contains("error"));
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn failure_with_invalid_config_file() {
    let output = Command::new(TEST_BINARY)
        .arg("--config")
        .arg("path/that/does/not/exist/nufmt.nuon")
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(stderr.contains("error"));
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn failure_with_invalid_file_to_format() {
    let output = Command::new(TEST_BINARY)
        .arg("path/that/does/not/exist/a.nu")
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(stderr.contains("error"));
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn warning_when_no_files_are_detected() {
    let dir = tempdir().unwrap();

    let output = Command::new(TEST_BINARY)
        .arg("--dry-run")
        .arg(dir.path().to_str().unwrap())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("warning"));
    assert_eq!(output.status.code(), Some(0));
}

#[test]
fn warning_is_displayed_when_no_files_are_detected_with_excluded_files() {
    let dir = tempdir().unwrap();
    let config_file = dir.path().join("nufmt.nuon");
    let file_a = dir.path().join("a.nu");
    fs::write(&config_file, r#"{exclude: ["a*"]}"#).unwrap();
    fs::write(&file_a, INVALID).unwrap();

    let output = Command::new(TEST_BINARY)
        .arg("--config")
        .arg(config_file.to_str().unwrap())
        .arg("--dry-run")
        .arg(dir.path().to_str().unwrap())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("warning"));
    assert_eq!(output.status.code(), Some(0));
}

#[test]
fn files_are_reformatted() {
    let dir = tempdir().unwrap();
    let config_file = dir.path().join("nufmt.nuon");
    let file_a = dir.path().join("a.nu");
    let file_b = dir.path().join("b.nu");
    fs::write(&config_file, r#"{exclude: ["a*"]}"#).unwrap();
    fs::write(&file_a, INVALID).unwrap();
    fs::write(&file_b, INVALID).unwrap();

    let output = Command::new(TEST_BINARY)
        .arg("--config")
        .arg(config_file.to_str().unwrap())
        .arg(dir.path().to_str().unwrap())
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    let file_a_content = fs::read_to_string(file_a).unwrap();
    let file_b_content = fs::read_to_string(file_b).unwrap();
    assert_eq!(file_a_content.as_str(), INVALID);
    assert_eq!(file_b_content.as_str(), VALID);
}

#[test]
fn files_are_checked() {
    let dir = tempdir().unwrap();
    let config_file = dir.path().join("nufmt.nuon");
    let file_a = dir.path().join("a.nu");
    let file_b = dir.path().join("b.nu");
    fs::write(&config_file, r#"{exclude: ["a*"]}"#).unwrap();
    fs::write(&file_a, INVALID).unwrap();
    fs::write(&file_b, INVALID).unwrap();

    let output = Command::new(TEST_BINARY)
        .arg("--config")
        .arg(config_file.to_str().unwrap())
        .arg("--dry-run")
        .arg(dir.path().to_str().unwrap())
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1));
    let file_a_content = fs::read_to_string(file_a).unwrap();
    let file_b_content = fs::read_to_string(file_b).unwrap();
    assert_eq!(file_a_content.as_str(), INVALID);
    assert_eq!(file_b_content.as_str(), INVALID);
}
