use std::{fs, process::Command};
use tempfile::tempdir;

const INVALID: &str = "# beginning of script comment

let one = 1
";
const VALID: &str = "# beginning of script comment
let one = 1
";

#[test]
fn failure_with_invalid_config() {
    let dir = tempdir().unwrap();
    let config_file = dir.path().join("nufmt.nuon");
    fs::write(&config_file, r#"{unknown: 1}"#).unwrap();

    let output = Command::new("target/debug/nufmt")
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
    let output = Command::new("target/debug/nufmt")
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
    let output = Command::new("target/debug/nufmt")
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

    let output = Command::new("target/debug/nufmt")
        .arg("--check")
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

    let output = Command::new("target/debug/nufmt")
        .arg("--config")
        .arg(config_file.to_str().unwrap())
        .arg("--check")
        .arg(dir.path().to_str().unwrap())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("warning"));
    assert_eq!(output.status.code(), Some(0));
}
