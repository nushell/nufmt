//! Ground truth tests for nufmt
//!
//! These tests compare formatter output against expected ground truth files.
//! Each construct has a separate input and expected file for easy editing.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// Get the path to the test binary
pub fn get_test_binary() -> PathBuf {
    let exe_name = if cfg!(windows) { "nufmt.exe" } else { "nufmt" };

    // Try CARGO_TARGET_DIR first
    if let Ok(target_dir) = std::env::var("CARGO_TARGET_DIR") {
        let path = PathBuf::from(target_dir).join("debug").join(exe_name);
        if path.exists() {
            return path.canonicalize().unwrap_or(path);
        }
    }

    // Try default target directory
    let default_path = PathBuf::from("target").join("debug").join(exe_name);
    if default_path.exists() {
        default_path.canonicalize().unwrap_or(default_path)
    } else {
        panic!(
            "Test binary not found. Please build the project first to create {:?}",
            default_path
        );
    }
}

/// Helper to run the formatter on input and compare with expected output
fn run_ground_truth_test(test_binary: &PathBuf, name: &str) {
    let input_path = PathBuf::from(format!("tests/fixtures/input/{}.nu", name));
    let expected_path = PathBuf::from(format!("tests/fixtures/expected/{}.nu", name));

    // Ensure files exist
    assert!(
        input_path.exists(),
        "Input file not found: {:?}",
        input_path
    );
    assert!(
        expected_path.exists(),
        "Expected file not found: {:?}",
        expected_path
    );

    // Read input
    let input = fs::read_to_string(&input_path).expect("Failed to read input file");

    // Run formatter via stdin
    let formatted = match format_via_stdin(test_binary, &input) {
        Ok(output) => output,
        Err(err) => panic!("Formatter failed for {}: {}", name, err),
    };

    // Read expected output
    let expected = fs::read_to_string(&expected_path).expect("Failed to read expected file");

    // Compare (normalize line endings)
    let formatted_normalized = formatted.trim().replace("\r\n", "\n");
    let expected_normalized = expected.trim().replace("\r\n", "\n");

    if formatted_normalized != expected_normalized {
        // Print detailed diff
        eprintln!("=== Ground truth test failed for: {} ===", name);
        eprintln!("\n--- Expected ---");
        eprintln!("{}", expected_normalized);
        eprintln!("\n--- Got ---");
        eprintln!("{}", formatted_normalized);
        eprintln!("\n--- Diff ---");

        // Line by line diff
        let expected_lines: Vec<&str> = expected_normalized.lines().collect();
        let formatted_lines: Vec<&str> = formatted_normalized.lines().collect();

        let max_lines = expected_lines.len().max(formatted_lines.len());
        for i in 0..max_lines {
            let exp = expected_lines.get(i).unwrap_or(&"<missing>");
            let got = formatted_lines.get(i).unwrap_or(&"<missing>");
            if exp != got {
                eprintln!("Line {}: ", i + 1);
                eprintln!("  expected: {:?}", exp);
                eprintln!("  got:      {:?}", got);
            }
        }

        panic!("Ground truth mismatch for {}. See diff above.", name);
    }
}

/// Test that formatting is idempotent (formatting twice gives same result)
fn run_idempotency_test(test_binary: &PathBuf, name: &str) {
    let input_path = PathBuf::from(format!("tests/fixtures/input/{}.nu", name));

    if !input_path.exists() {
        return; // Skip if input doesn't exist
    }

    let input = fs::read_to_string(&input_path).expect("Failed to read input file");

    // First format
    let first_output = format_via_stdin(test_binary, &input);
    if first_output.is_err() {
        return; // Skip if formatting fails
    }
    let first = first_output.unwrap();

    // Second format
    let second_output = format_via_stdin(test_binary, &first);
    if second_output.is_err() {
        panic!("Second format failed for {}, but first succeeded", name);
    }
    let second = second_output.unwrap();

    if first != second {
        eprintln!("=== Idempotency test failed for: {} ===", name);
        eprintln!("\n--- First format ---");
        eprintln!("{}", first);
        eprintln!("\n--- Second format ---");
        eprintln!("{}", second);
        panic!("Formatting is not idempotent for {}", name);
    }
}

fn format_via_stdin(test_binary: &PathBuf, input: &str) -> Result<String, String> {
    let output = Command::new(test_binary)
        .arg("--stdin")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to spawn nufmt");

    use std::io::Write;
    output
        .stdin
        .as_ref()
        .unwrap()
        .write_all(input.as_bytes())
        .expect("Failed to write to stdin");

    let output = output.wait_with_output().expect("Failed to wait for nufmt");

    if output.status.success() {
        Ok(String::from_utf8(output.stdout).expect("Invalid UTF-8"))
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

// ============================================================================
// Ground Truth Tests - Core Language Constructs
// ============================================================================

#[test]
fn ground_truth_let_statement() {
    let test_binary = get_test_binary();
    run_ground_truth_test(&test_binary, "let_statement");
}

#[test]
fn ground_truth_mut_statement() {
    let test_binary = get_test_binary();
    run_ground_truth_test(&test_binary, "mut_statement");
}

#[test]
fn ground_truth_const_statement() {
    let test_binary = get_test_binary();
    run_ground_truth_test(&test_binary, "const_statement");
}

#[test]
fn ground_truth_def_statement() {
    let test_binary = get_test_binary();
    run_ground_truth_test(&test_binary, "def_statement");
}

// ============================================================================
// Ground Truth Tests - Control Flow
// ============================================================================

#[test]
fn ground_truth_if_else() {
    let test_binary = get_test_binary();
    run_ground_truth_test(&test_binary, "if_else");
}

#[test]
fn ground_truth_for_loop() {
    let test_binary = get_test_binary();
    run_ground_truth_test(&test_binary, "for_loop");
}

#[test]
fn ground_truth_while_loop() {
    let test_binary = get_test_binary();
    run_ground_truth_test(&test_binary, "while_loop");
}

#[test]
fn ground_truth_loop_statement() {
    let test_binary = get_test_binary();
    run_ground_truth_test(&test_binary, "loop_statement");
}

#[test]
fn ground_truth_match_expr() {
    let test_binary = get_test_binary();
    run_ground_truth_test(&test_binary, "match_expr");
}

#[test]
fn ground_truth_try_catch() {
    let test_binary = get_test_binary();
    run_ground_truth_test(&test_binary, "try_catch");
}

#[test]
fn ground_truth_break_continue() {
    let test_binary = get_test_binary();
    run_ground_truth_test(&test_binary, "break_continue");
}

#[test]
fn ground_truth_return_statement() {
    let test_binary = get_test_binary();
    run_ground_truth_test(&test_binary, "return_statement");
}

// ============================================================================
// Ground Truth Tests - Data Structures
// ============================================================================

#[test]
fn ground_truth_list() {
    let test_binary = get_test_binary();
    run_ground_truth_test(&test_binary, "list");
}

#[test]
fn ground_truth_record() {
    let test_binary = get_test_binary();
    run_ground_truth_test(&test_binary, "record");
}

#[test]
fn ground_truth_table() {
    let test_binary = get_test_binary();
    run_ground_truth_test(&test_binary, "table");
}

#[test]
fn ground_truth_nested_structures() {
    let test_binary = get_test_binary();
    run_ground_truth_test(&test_binary, "nested_structures");
}

// ============================================================================
// Ground Truth Tests - Pipelines and Expressions
// ============================================================================

#[test]
fn ground_truth_pipeline() {
    let test_binary = get_test_binary();
    run_ground_truth_test(&test_binary, "pipeline");
}

#[test]
fn ground_truth_multiline_pipeline() {
    let test_binary = get_test_binary();
    run_ground_truth_test(&test_binary, "multiline_pipeline");
}

#[test]
fn ground_truth_closure() {
    let test_binary = get_test_binary();
    run_ground_truth_test(&test_binary, "closure");
}

#[test]
fn ground_truth_subexpression() {
    let test_binary = get_test_binary();
    run_ground_truth_test(&test_binary, "subexpression");
}

#[test]
fn ground_truth_binary_ops() {
    let test_binary = get_test_binary();
    run_ground_truth_test(&test_binary, "binary_ops");
}

#[test]
fn ground_truth_range() {
    let test_binary = get_test_binary();
    run_ground_truth_test(&test_binary, "range");
}

#[test]
fn ground_truth_cell_path() {
    let test_binary = get_test_binary();
    run_ground_truth_test(&test_binary, "cell_path");
}

#[test]
fn ground_truth_spread() {
    let test_binary = get_test_binary();
    run_ground_truth_test(&test_binary, "spread");
}

// ============================================================================
// Ground Truth Tests - Strings and Interpolation
// ============================================================================

#[test]
fn ground_truth_string_interpolation() {
    let test_binary = get_test_binary();
    run_ground_truth_test(&test_binary, "string_interpolation");
}

#[test]
fn ground_truth_comment() {
    let test_binary = get_test_binary();
    run_ground_truth_test(&test_binary, "comment");
}

// ============================================================================
// Ground Truth Tests - Types and Values
// ============================================================================

#[test]
fn ground_truth_value_with_unit() {
    let test_binary = get_test_binary();
    run_ground_truth_test(&test_binary, "value_with_unit");
}

#[test]
fn ground_truth_datetime() {
    let test_binary = get_test_binary();
    run_ground_truth_test(&test_binary, "datetime");
}

#[test]
fn ground_truth_nothing() {
    let test_binary = get_test_binary();
    run_ground_truth_test(&test_binary, "nothing");
}

#[test]
fn ground_truth_glob_pattern() {
    let test_binary = get_test_binary();
    run_ground_truth_test(&test_binary, "glob_pattern");
}

// ============================================================================
// Ground Truth Tests - Modules and Imports
// ============================================================================

#[test]
fn ground_truth_module() {
    let test_binary = get_test_binary();
    run_ground_truth_test(&test_binary, "module");
}

#[test]
fn ground_truth_use_statement() {
    let test_binary = get_test_binary();
    run_ground_truth_test(&test_binary, "use_statement");
}

#[test]
fn ground_truth_export() {
    let test_binary = get_test_binary();
    run_ground_truth_test(&test_binary, "export");
}

#[test]
fn ground_truth_source() {
    let test_binary = get_test_binary();
    run_ground_truth_test(&test_binary, "source");
}

#[test]
fn ground_truth_hide() {
    let test_binary = get_test_binary();
    run_ground_truth_test(&test_binary, "hide");
}

#[test]
fn ground_truth_overlay() {
    let test_binary = get_test_binary();
    run_ground_truth_test(&test_binary, "overlay");
}

// ============================================================================
// Ground Truth Tests - Commands and Definitions
// ============================================================================

#[test]
fn ground_truth_alias() {
    let test_binary = get_test_binary();
    run_ground_truth_test(&test_binary, "alias");
}

#[test]
fn ground_truth_extern() {
    let test_binary = get_test_binary();
    run_ground_truth_test(&test_binary, "extern");
}

#[test]
fn ground_truth_external_call() {
    let test_binary = get_test_binary();
    run_ground_truth_test(&test_binary, "external_call");
}

// ============================================================================
// Ground Truth Tests - Special Constructs
// ============================================================================

#[test]
fn ground_truth_do_block() {
    let test_binary = get_test_binary();
    run_ground_truth_test(&test_binary, "do_block");
}

#[test]
fn ground_truth_where_clause() {
    let test_binary = get_test_binary();
    run_ground_truth_test(&test_binary, "where_clause");
}

#[test]
fn ground_truth_error_make() {
    let test_binary = get_test_binary();
    run_ground_truth_test(&test_binary, "error_make");
}

// ============================================================================
// Idempotency Tests - Core Language Constructs
// ============================================================================

#[test]
fn idempotency_let_statement() {
    let test_binary = get_test_binary();
    run_idempotency_test(&test_binary, "let_statement");
}

#[test]
fn idempotency_mut_statement() {
    let test_binary = get_test_binary();
    run_idempotency_test(&test_binary, "mut_statement");
}

#[test]
fn idempotency_const_statement() {
    let test_binary = get_test_binary();
    run_idempotency_test(&test_binary, "const_statement");
}

#[test]
fn idempotency_def_statement() {
    let test_binary = get_test_binary();
    run_idempotency_test(&test_binary, "def_statement");
}

// ============================================================================
// Idempotency Tests - Control Flow
// ============================================================================

#[test]
fn idempotency_if_else() {
    let test_binary = get_test_binary();
    run_idempotency_test(&test_binary, "if_else");
}

#[test]
fn idempotency_for_loop() {
    let test_binary = get_test_binary();
    run_idempotency_test(&test_binary, "for_loop");
}

#[test]
fn idempotency_while_loop() {
    let test_binary = get_test_binary();
    run_idempotency_test(&test_binary, "while_loop");
}

#[test]
fn idempotency_loop_statement() {
    let test_binary = get_test_binary();
    run_idempotency_test(&test_binary, "loop_statement");
}

#[test]
fn idempotency_match_expr() {
    let test_binary = get_test_binary();
    run_idempotency_test(&test_binary, "match_expr");
}

#[test]
fn idempotency_try_catch() {
    let test_binary = get_test_binary();
    run_idempotency_test(&test_binary, "try_catch");
}

#[test]
fn idempotency_break_continue() {
    let test_binary = get_test_binary();
    run_idempotency_test(&test_binary, "break_continue");
}

#[test]
fn idempotency_return_statement() {
    let test_binary = get_test_binary();
    run_idempotency_test(&test_binary, "return_statement");
}

// ============================================================================
// Idempotency Tests - Data Structures
// ============================================================================

#[test]
fn idempotency_list() {
    let test_binary = get_test_binary();
    run_idempotency_test(&test_binary, "list");
}

#[test]
fn idempotency_record() {
    let test_binary = get_test_binary();
    run_idempotency_test(&test_binary, "record");
}

#[test]
fn idempotency_table() {
    let test_binary = get_test_binary();
    run_idempotency_test(&test_binary, "table");
}

#[test]
fn idempotency_nested_structures() {
    let test_binary = get_test_binary();
    run_idempotency_test(&test_binary, "nested_structures");
}

// ============================================================================
// Idempotency Tests - Pipelines and Expressions
// ============================================================================

#[test]
fn idempotency_pipeline() {
    let test_binary = get_test_binary();
    run_idempotency_test(&test_binary, "pipeline");
}

#[test]
fn idempotency_multiline_pipeline() {
    let test_binary = get_test_binary();
    run_idempotency_test(&test_binary, "multiline_pipeline");
}

#[test]
fn idempotency_closure() {
    let test_binary = get_test_binary();
    run_idempotency_test(&test_binary, "closure");
}

#[test]
fn idempotency_subexpression() {
    let test_binary = get_test_binary();
    run_idempotency_test(&test_binary, "subexpression");
}

#[test]
fn idempotency_binary_ops() {
    let test_binary = get_test_binary();
    run_idempotency_test(&test_binary, "binary_ops");
}

#[test]
fn idempotency_range() {
    let test_binary = get_test_binary();
    run_idempotency_test(&test_binary, "range");
}

#[test]
fn idempotency_cell_path() {
    let test_binary = get_test_binary();
    run_idempotency_test(&test_binary, "cell_path");
}

#[test]
fn idempotency_spread() {
    let test_binary = get_test_binary();
    run_idempotency_test(&test_binary, "spread");
}

// ============================================================================
// Idempotency Tests - Strings and Interpolation
// ============================================================================

#[test]
fn idempotency_string_interpolation() {
    let test_binary = get_test_binary();
    run_idempotency_test(&test_binary, "string_interpolation");
}

#[test]
fn idempotency_comment() {
    let test_binary = get_test_binary();
    run_idempotency_test(&test_binary, "comment");
}

// ============================================================================
// Idempotency Tests - Types and Values
// ============================================================================

#[test]
fn idempotency_value_with_unit() {
    let test_binary = get_test_binary();
    run_idempotency_test(&test_binary, "value_with_unit");
}

#[test]
fn idempotency_datetime() {
    let test_binary = get_test_binary();
    run_idempotency_test(&test_binary, "datetime");
}

#[test]
fn idempotency_nothing() {
    let test_binary = get_test_binary();
    run_idempotency_test(&test_binary, "nothing");
}

#[test]
fn idempotency_glob_pattern() {
    let test_binary = get_test_binary();
    run_idempotency_test(&test_binary, "glob_pattern");
}

// ============================================================================
// Idempotency Tests - Modules and Imports
// ============================================================================

#[test]
fn idempotency_module() {
    let test_binary = get_test_binary();
    run_idempotency_test(&test_binary, "module");
}

#[test]
fn idempotency_use_statement() {
    let test_binary = get_test_binary();
    run_idempotency_test(&test_binary, "use_statement");
}

#[test]
fn idempotency_export() {
    let test_binary = get_test_binary();
    run_idempotency_test(&test_binary, "export");
}

#[test]
fn idempotency_source() {
    let test_binary = get_test_binary();
    run_idempotency_test(&test_binary, "source");
}

#[test]
fn idempotency_hide() {
    let test_binary = get_test_binary();
    run_idempotency_test(&test_binary, "hide");
}

#[test]
fn idempotency_overlay() {
    let test_binary = get_test_binary();
    run_idempotency_test(&test_binary, "overlay");
}

// ============================================================================
// Idempotency Tests - Commands and Definitions
// ============================================================================

#[test]
fn idempotency_alias() {
    let test_binary = get_test_binary();
    run_idempotency_test(&test_binary, "alias");
}

#[test]
fn idempotency_extern() {
    let test_binary = get_test_binary();
    run_idempotency_test(&test_binary, "extern");
}

#[test]
fn idempotency_external_call() {
    let test_binary = get_test_binary();
    run_idempotency_test(&test_binary, "external_call");
}

// ============================================================================
// Idempotency Tests - Special Constructs
// ============================================================================

#[test]
fn idempotency_do_block() {
    let test_binary = get_test_binary();
    run_idempotency_test(&test_binary, "do_block");
}

#[test]
fn idempotency_where_clause() {
    let test_binary = get_test_binary();
    run_idempotency_test(&test_binary, "where_clause");
}

#[test]
fn idempotency_error_make() {
    let test_binary = get_test_binary();
    run_idempotency_test(&test_binary, "error_make");
}

// ============================================================================
// Issue Tests -
// ============================================================================
#[test]
fn issue76_test() {
    let test_binary = get_test_binary();
    run_ground_truth_test(&test_binary, "issue76");
}
