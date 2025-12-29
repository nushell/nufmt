use std::{fs, path::PathBuf, process::Command};
use tempfile::tempdir;

const INVALID: &str = "# beginning of script comment

let one = 1
";
const VALID: &str = "# beginning of script comment
let one = 1
";
const TEST_BINARY: &str = "target/debug/nufmt";

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

#[test]
fn format_let_statement() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("test.nu");
    fs::write(&file, "let   x   =   1").unwrap();

    let output = Command::new(TEST_BINARY)
        .arg(file.to_str().unwrap())
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    let content = fs::read_to_string(&file).unwrap();
    assert_eq!(content.trim(), "let x = 1");
}

#[test]
fn format_def_statement() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("test.nu");
    fs::write(&file, "def foo [x: int] { $x + 1 }").unwrap();

    let output = Command::new(TEST_BINARY)
        .arg(file.to_str().unwrap())
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    let content = fs::read_to_string(&file).unwrap();
    assert!(content.contains("def foo"));
    assert!(content.contains("$x + 1"));
}

#[test]
fn format_if_else() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("test.nu");
    fs::write(&file, "if true { echo yes } else { echo no }").unwrap();

    let output = Command::new(TEST_BINARY)
        .arg(file.to_str().unwrap())
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    let content = fs::read_to_string(&file).unwrap();
    assert!(content.contains("if true"));
    assert!(content.contains("else"));
}

#[test]
fn format_pipeline() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("test.nu");
    fs::write(&file, "ls|get name").unwrap();

    let output = Command::new(TEST_BINARY)
        .arg(file.to_str().unwrap())
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    let content = fs::read_to_string(&file).unwrap();
    assert!(content.contains(" | "));
}

#[test]
fn format_preserves_comments() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("test.nu");
    fs::write(&file, "# comment\nlet x = 1").unwrap();

    let output = Command::new(TEST_BINARY)
        .arg(file.to_str().unwrap())
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    let content = fs::read_to_string(&file).unwrap();
    assert!(content.contains("# comment"));
    assert!(content.contains("let x = 1"));
}

#[test]
fn format_is_idempotent() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("test.nu");
    fs::write(&file, "let x = 1\nlet y = 2").unwrap();

    // First format
    Command::new(TEST_BINARY)
        .arg(file.to_str().unwrap())
        .output()
        .unwrap();
    let first = fs::read_to_string(&file).unwrap();

    // Second format
    Command::new(TEST_BINARY)
        .arg(file.to_str().unwrap())
        .output()
        .unwrap();
    let second = fs::read_to_string(&file).unwrap();

    assert_eq!(first, second, "Formatting should be idempotent");
}

#[test]
fn format_for_loop() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("test.nu");
    fs::write(&file, "for x in [1, 2, 3] { print $x }").unwrap();

    let output = Command::new(TEST_BINARY)
        .arg(file.to_str().unwrap())
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    let content = fs::read_to_string(&file).unwrap();
    assert!(content.contains("for x in"));
    assert!(content.contains("{ print"));
}

#[test]
fn format_closure() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("test.nu");
    fs::write(&file, "{|x| $x * 2 }").unwrap();

    let output = Command::new(TEST_BINARY)
        .arg(file.to_str().unwrap())
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    let content = fs::read_to_string(&file).unwrap();
    assert!(content.contains("{|x|"));
}

#[test]
fn format_fixtures_basic() {
    // Test that the basic fixture can be formatted without errors
    let fixture_path = PathBuf::from("tests/fixtures/basic.nu");
    if fixture_path.exists() {
        let output = Command::new(TEST_BINARY)
            .arg("--dry-run")
            .arg(fixture_path.to_str().unwrap())
            .output()
            .unwrap();

        // Should either succeed or report would-reformat
        assert!(output.status.code() == Some(0) || output.status.code() == Some(1));
    }
}
