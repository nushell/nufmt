//! Discover fixtures at `fixtures/<category>/<case>` and check each file once.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Default)]
struct FixtureResults {
    fixtures: usize,
    checks: usize,
    failures: Vec<String>,
}

fn directories(path: &Path, failures: &mut Vec<String>) -> Vec<PathBuf> {
    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(err) => {
            failures.push(format!("Could not list {}: {err}", path.display()));
            return Vec::new();
        }
    };

    let mut directories = Vec::new();
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(err) => {
                failures.push(format!("Could not read entry in {}: {err}", path.display()));
                continue;
            }
        };
        match entry.file_type() {
            Ok(file_type) if file_type.is_dir() => directories.push(entry.path()),
            Ok(_) => {}
            Err(err) => failures.push(format!(
                "Could not inspect {}: {err}",
                entry.path().display()
            )),
        }
    }
    directories.sort();
    directories
}

fn run_fixtures(
    root: &Path,
    mut format: impl FnMut(&[u8], Option<&Path>) -> Result<Vec<u8>, String>,
) -> FixtureResults {
    let mut results = FixtureResults::default();
    for category in directories(root, &mut results.failures) {
        for fixture in directories(&category, &mut results.failures) {
            results.fixtures += 1;
            let mut run = || -> Result<(), String> {
                let config_path = fixture.join("config.noun");
                let config = config_path
                    .try_exists()
                    .map_err(|err| format!("Could not inspect {}: {err}", config_path.display()))?
                    .then_some(config_path.as_path());
                let expected_path = fixture.join("expected.nu");
                let expected = fs::read(&expected_path)
                    .map_err(|err| format!("Could not read {}: {err}", expected_path.display()))?;

                let mut check = |source: &Path, input: &[u8], artifact: &str| {
                    results.checks += 1;
                    let result = format(input, config).and_then(|actual| {
                        let artifact_path = fixture.join(artifact);
                        if actual != expected {
                            fs::write(&artifact_path, actual).map_err(|err| {
                                format!("Could not write {}: {err}", artifact_path.display())
                            })?;
                            Err(format!(
                                "differs after one format; see {}",
                                artifact_path.display()
                            ))
                        } else {
                            match fs::remove_file(&artifact_path) {
                                Ok(()) => Ok(()),
                                Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
                                Err(err) => Err(format!(
                                    "Could not remove {}: {err}",
                                    artifact_path.display()
                                )),
                            }
                        }
                    });
                    if let Err(err) = result {
                        results
                            .failures
                            .push(format!("{}: {err}", source.display()));
                    }
                };

                check(&expected_path, &expected, "not_idempotent.nu");
                let input_path = fixture.join("input.nu");
                if input_path
                    .try_exists()
                    .map_err(|err| format!("Could not inspect {}: {err}", input_path.display()))?
                {
                    let input = fs::read(&input_path)
                        .map_err(|err| format!("Could not read {}: {err}", input_path.display()))?;
                    check(&input_path, &input, "unexpected.nu");
                }
                Ok(())
            };
            if let Err(err) = run() {
                results.failures.push(err);
            }
        }
    }
    if results.fixtures == 0 {
        results.failures.push(format!(
            "No fixture directories found under {}",
            root.display()
        ));
    }
    results
}

fn format_via_stdin(input: &[u8], config: Option<&Path>) -> Result<Vec<u8>, String> {
    let mut command = Command::new(env!("CARGO_BIN_EXE_nufmt"));
    command
        .arg("--stdin")
        .current_dir(env!("CARGO_MANIFEST_DIR"));
    if let Some(config) = config {
        command.arg("--config").arg(config);
    }

    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| format!("Could not spawn nufmt: {err}"))?;
    let write_result = child
        .stdin
        .take()
        .expect("stdin should be piped")
        .write_all(input);
    let output = child
        .wait_with_output()
        .map_err(|err| format!("Could not wait for nufmt: {err}"))?;
    if !output.status.success() {
        return Err(format!(
            "nufmt exited with {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    write_result.map_err(|err| format!("Could not write stdin: {err}"))?;
    Ok(output.stdout)
}

#[test]
fn fixtures() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let results = run_fixtures(&root, format_via_stdin);
    eprintln!(
        "Checked {} fixtures ({} format calls)",
        results.fixtures, results.checks
    );
    assert!(
        results.failures.is_empty(),
        "{} fixture checks failed:\n{}",
        results.failures.len(),
        results.failures.join("\n")
    );
}

#[test]
fn fixture_runner_discovers_cases_and_updates_artifacts() {
    let root = tempfile::tempdir().unwrap();
    let passing = root.path().join("category/a_passing");
    let failing = root.path().join("category/b_failing");
    let expected_only = root.path().join("category/c_expected_only");
    for fixture in [&passing, &failing, &expected_only] {
        fs::create_dir_all(fixture).unwrap();
        fs::write(fixture.join("not_idempotent.nu"), b"stale").unwrap();
        fs::write(fixture.join("unexpected.nu"), b"stale").unwrap();
    }
    fs::write(passing.join("expected.nu"), b"passing\n").unwrap();
    fs::write(passing.join("input.nu"), b"input\n").unwrap();
    fs::write(passing.join("config.noun"), b"{indent: 2}").unwrap();
    fs::write(failing.join("expected.nu"), b"failing").unwrap();
    fs::write(failing.join("input.nu"), b"wrong\r\n").unwrap();
    fs::write(expected_only.join("expected.nu"), b"only\n").unwrap();
    fs::create_dir_all(expected_only.join("nested")).unwrap();
    fs::write(expected_only.join("nested/expected.nu"), b"ignored").unwrap();
    fs::write(root.path().join("category/not_a_case.nu"), b"ignored").unwrap();

    let mut calls = Vec::new();
    let results = run_fixtures(root.path(), |input, config| {
        calls.push((input.to_vec(), config.map(Path::to_path_buf)));
        Ok(match input {
            b"input\n" => b"passing\n".to_vec(),
            b"failing" => b"failing\n".to_vec(),
            b"wrong\r\n" => b"actual\r\n\n".to_vec(),
            _ => input.to_vec(),
        })
    });
    assert_eq!(results.fixtures, 3);
    assert_eq!(results.checks, 5);
    assert_eq!(results.failures.len(), 2);
    assert_eq!(
        calls,
        vec![
            (b"passing\n".to_vec(), Some(passing.join("config.noun"))),
            (b"input\n".to_vec(), Some(passing.join("config.noun"))),
            (b"failing".to_vec(), None),
            (b"wrong\r\n".to_vec(), None),
            (b"only\n".to_vec(), None),
        ]
    );
    assert!(!passing.join("not_idempotent.nu").exists());
    assert!(!passing.join("unexpected.nu").exists());
    assert_eq!(
        fs::read(failing.join("not_idempotent.nu")).unwrap(),
        b"failing\n"
    );
    assert_eq!(
        fs::read(failing.join("unexpected.nu")).unwrap(),
        b"actual\r\n\n"
    );
    assert!(!expected_only.join("not_idempotent.nu").exists());
    assert_eq!(
        fs::read(expected_only.join("unexpected.nu")).unwrap(),
        b"stale"
    );
}

#[test]
fn fixture_runner_continues_after_errors() {
    let root = tempfile::tempdir().unwrap();
    for name in [
        "a_missing_expected",
        "b_formatter_error",
        "c_write_error",
        "d_remove_error",
        "e_after",
    ] {
        let fixture = root.path().join("category").join(name);
        fs::create_dir_all(&fixture).unwrap();
        if name != "a_missing_expected" {
            fs::write(fixture.join("expected.nu"), name).unwrap();
        }
    }
    let formatter_error = root.path().join("category/b_formatter_error");
    fs::write(formatter_error.join("input.nu"), b"input").unwrap();
    fs::write(formatter_error.join("not_idempotent.nu"), b"stale").unwrap();
    let write_error = root.path().join("category/c_write_error");
    fs::create_dir(write_error.join("not_idempotent.nu")).unwrap();
    let remove_error = root.path().join("category/d_remove_error");
    fs::create_dir(remove_error.join("not_idempotent.nu")).unwrap();

    let mut calls = Vec::new();
    let results = run_fixtures(root.path(), |input, _| {
        calls.push(input.to_vec());
        if input == b"b_formatter_error" {
            Err("formatter error".into())
        } else if input == b"c_write_error" {
            Ok(b"changed".to_vec())
        } else if input == b"input" {
            Ok(b"b_formatter_error".to_vec())
        } else {
            Ok(input.to_vec())
        }
    });
    assert_eq!(results.fixtures, 5);
    assert_eq!(results.failures.len(), 4);
    assert_eq!(
        calls,
        vec![
            b"b_formatter_error".to_vec(),
            b"input".to_vec(),
            b"c_write_error".to_vec(),
            b"d_remove_error".to_vec(),
            b"e_after".to_vec()
        ]
    );
    assert_eq!(
        fs::read(formatter_error.join("not_idempotent.nu")).unwrap(),
        b"stale"
    );
}

#[test]
fn fixture_runner_compares_bytes_without_normalizing_whitespace() {
    let root = tempfile::tempdir().unwrap();
    for (name, expected) in [
        ("leading_space", b" echo hello\n".as_slice()),
        ("trailing_space", b"echo hello \n".as_slice()),
        ("crlf", b"echo hello\r\n".as_slice()),
        ("missing_newline", b"echo hello".as_slice()),
    ] {
        let fixture = root.path().join("category").join(name);
        fs::create_dir_all(&fixture).unwrap();
        fs::write(fixture.join("expected.nu"), expected).unwrap();
    }
    let results = run_fixtures(root.path(), |_, _| Ok(b"echo hello\n".to_vec()));
    assert_eq!(results.checks, 4);
    assert_eq!(results.failures.len(), 4);
    for fixture in directories(&root.path().join("category"), &mut Vec::new()) {
        assert_eq!(
            fs::read(fixture.join("not_idempotent.nu")).unwrap(),
            b"echo hello\n"
        );
    }
}

#[test]
fn invalid_config_fails_both_checks_and_continues_to_other_fixtures() {
    let root = tempfile::tempdir().unwrap();
    let invalid = root.path().join("category/a_invalid");
    let valid = root.path().join("category/b_valid");
    for fixture in [&invalid, &valid] {
        fs::create_dir_all(fixture).unwrap();
        fs::write(fixture.join("expected.nu"), b"let x = 1\n").unwrap();
        fs::write(fixture.join("not_idempotent.nu"), b"stale").unwrap();
    }
    fs::write(invalid.join("input.nu"), b"let  x = 1\n").unwrap();
    fs::write(invalid.join("config.noun"), b"{unknown: 1}").unwrap();
    let results = run_fixtures(root.path(), format_via_stdin);
    assert_eq!(results.fixtures, 2);
    assert_eq!(results.checks, 3);
    assert_eq!(results.failures.len(), 2);
    assert!(results.failures.iter().all(|err| err.contains("unknown")));
    assert_eq!(
        fs::read(invalid.join("not_idempotent.nu")).unwrap(),
        b"stale"
    );
    assert!(!valid.join("not_idempotent.nu").exists());
}
