//! Ground truth tests for nufmt
//!
//! These tests compare formatter output against expected ground truth files.
//! Each construct has a separate input and expected file for easy editing.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

const TEST_BINARY: &str = "target/debug/nufmt";

/// Helper to run the formatter on input and compare with expected output
fn run_ground_truth_test(name: &str) {
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
    let output = Command::new(TEST_BINARY)
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

    // Check for errors
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!(
            "Formatter failed for {}: exit code {:?}\nstderr: {}",
            name,
            output.status.code(),
            stderr
        );
    }

    // Get formatted output
    let formatted = String::from_utf8(output.stdout).expect("Invalid UTF-8 in output");

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
fn run_idempotency_test(name: &str) {
    let input_path = PathBuf::from(format!("tests/fixtures/input/{}.nu", name));

    if !input_path.exists() {
        return; // Skip if input doesn't exist
    }

    let input = fs::read_to_string(&input_path).expect("Failed to read input file");

    // First format
    let first_output = format_via_stdin(&input);
    if first_output.is_err() {
        return; // Skip if formatting fails
    }
    let first = first_output.unwrap();

    // Second format
    let second_output = format_via_stdin(&first);
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

fn format_via_stdin(input: &str) -> Result<String, String> {
    let output = Command::new(TEST_BINARY)
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
    run_ground_truth_test("let_statement");
}

#[test]
fn ground_truth_mut_statement() {
    run_ground_truth_test("mut_statement");
}

#[test]
fn ground_truth_const_statement() {
    run_ground_truth_test("const_statement");
}

#[test]
fn ground_truth_def_statement() {
    run_ground_truth_test("def_statement");
}

// ============================================================================
// Ground Truth Tests - Control Flow
// ============================================================================

#[test]
fn ground_truth_if_else() {
    run_ground_truth_test("if_else");
}

#[test]
fn ground_truth_for_loop() {
    run_ground_truth_test("for_loop");
}

#[test]
fn ground_truth_while_loop() {
    run_ground_truth_test("while_loop");
}

#[test]
fn ground_truth_loop_statement() {
    run_ground_truth_test("loop_statement");
}

#[test]
fn ground_truth_match_expr() {
    run_ground_truth_test("match_expr");
}

#[test]
fn ground_truth_try_catch() {
    run_ground_truth_test("try_catch");
}

#[test]
fn ground_truth_break_continue() {
    run_ground_truth_test("break_continue");
}

#[test]
fn ground_truth_return_statement() {
    run_ground_truth_test("return_statement");
}

// ============================================================================
// Ground Truth Tests - Data Structures
// ============================================================================

#[test]
fn ground_truth_list() {
    run_ground_truth_test("list");
}

#[test]
fn ground_truth_record() {
    run_ground_truth_test("record");
}

#[test]
fn ground_truth_table() {
    run_ground_truth_test("table");
}

#[test]
fn ground_truth_nested_structures() {
    run_ground_truth_test("nested_structures");
}

// ============================================================================
// Ground Truth Tests - Pipelines and Expressions
// ============================================================================

#[test]
fn ground_truth_pipeline() {
    run_ground_truth_test("pipeline");
}

#[test]
fn ground_truth_multiline_pipeline() {
    run_ground_truth_test("multiline_pipeline");
}

#[test]
fn ground_truth_closure() {
    run_ground_truth_test("closure");
}

#[test]
fn ground_truth_subexpression() {
    run_ground_truth_test("subexpression");
}

#[test]
fn ground_truth_binary_ops() {
    run_ground_truth_test("binary_ops");
}

#[test]
fn ground_truth_range() {
    run_ground_truth_test("range");
}

#[test]
fn ground_truth_cell_path() {
    run_ground_truth_test("cell_path");
}

#[test]
fn ground_truth_spread() {
    run_ground_truth_test("spread");
}

// ============================================================================
// Ground Truth Tests - Strings and Interpolation
// ============================================================================

#[test]
fn ground_truth_string_interpolation() {
    run_ground_truth_test("string_interpolation");
}

#[test]
fn ground_truth_comment() {
    run_ground_truth_test("comment");
}

// ============================================================================
// Ground Truth Tests - Types and Values
// ============================================================================

#[test]
fn ground_truth_value_with_unit() {
    run_ground_truth_test("value_with_unit");
}

#[test]
fn ground_truth_datetime() {
    run_ground_truth_test("datetime");
}

#[test]
fn ground_truth_nothing() {
    run_ground_truth_test("nothing");
}

#[test]
fn ground_truth_glob_pattern() {
    run_ground_truth_test("glob_pattern");
}

// ============================================================================
// Ground Truth Tests - Modules and Imports
// ============================================================================

#[test]
fn ground_truth_module() {
    run_ground_truth_test("module");
}

#[test]
fn ground_truth_use_statement() {
    run_ground_truth_test("use_statement");
}

#[test]
fn ground_truth_export() {
    run_ground_truth_test("export");
}

#[test]
fn ground_truth_source() {
    run_ground_truth_test("source");
}

#[test]
fn ground_truth_hide() {
    run_ground_truth_test("hide");
}

#[test]
fn ground_truth_overlay() {
    run_ground_truth_test("overlay");
}

// ============================================================================
// Ground Truth Tests - Commands and Definitions
// ============================================================================

#[test]
fn ground_truth_alias() {
    run_ground_truth_test("alias");
}

#[test]
fn ground_truth_extern() {
    run_ground_truth_test("extern");
}

#[test]
fn ground_truth_external_call() {
    run_ground_truth_test("external_call");
}

// ============================================================================
// Ground Truth Tests - Special Constructs
// ============================================================================

#[test]
fn ground_truth_do_block() {
    run_ground_truth_test("do_block");
}

#[test]
fn ground_truth_where_clause() {
    run_ground_truth_test("where_clause");
}

#[test]
fn ground_truth_error_make() {
    run_ground_truth_test("error_make");
}

// ============================================================================
// Idempotency Tests - Core Language Constructs
// ============================================================================

#[test]
fn idempotency_let_statement() {
    run_idempotency_test("let_statement");
}

#[test]
fn idempotency_mut_statement() {
    run_idempotency_test("mut_statement");
}

#[test]
fn idempotency_const_statement() {
    run_idempotency_test("const_statement");
}

#[test]
fn idempotency_def_statement() {
    run_idempotency_test("def_statement");
}

// ============================================================================
// Idempotency Tests - Control Flow
// ============================================================================

#[test]
fn idempotency_if_else() {
    run_idempotency_test("if_else");
}

#[test]
fn idempotency_for_loop() {
    run_idempotency_test("for_loop");
}

#[test]
fn idempotency_while_loop() {
    run_idempotency_test("while_loop");
}

#[test]
fn idempotency_loop_statement() {
    run_idempotency_test("loop_statement");
}

#[test]
fn idempotency_match_expr() {
    run_idempotency_test("match_expr");
}

#[test]
fn idempotency_try_catch() {
    run_idempotency_test("try_catch");
}

#[test]
fn idempotency_break_continue() {
    run_idempotency_test("break_continue");
}

#[test]
fn idempotency_return_statement() {
    run_idempotency_test("return_statement");
}

// ============================================================================
// Idempotency Tests - Data Structures
// ============================================================================

#[test]
fn idempotency_list() {
    run_idempotency_test("list");
}

#[test]
fn idempotency_record() {
    run_idempotency_test("record");
}

#[test]
fn idempotency_table() {
    run_idempotency_test("table");
}

#[test]
fn idempotency_nested_structures() {
    run_idempotency_test("nested_structures");
}

// ============================================================================
// Idempotency Tests - Pipelines and Expressions
// ============================================================================

#[test]
fn idempotency_pipeline() {
    run_idempotency_test("pipeline");
}

#[test]
fn idempotency_multiline_pipeline() {
    run_idempotency_test("multiline_pipeline");
}

#[test]
fn idempotency_closure() {
    run_idempotency_test("closure");
}

#[test]
fn idempotency_subexpression() {
    run_idempotency_test("subexpression");
}

#[test]
fn idempotency_binary_ops() {
    run_idempotency_test("binary_ops");
}

#[test]
fn idempotency_range() {
    run_idempotency_test("range");
}

#[test]
fn idempotency_cell_path() {
    run_idempotency_test("cell_path");
}

#[test]
fn idempotency_spread() {
    run_idempotency_test("spread");
}

// ============================================================================
// Idempotency Tests - Strings and Interpolation
// ============================================================================

#[test]
fn idempotency_string_interpolation() {
    run_idempotency_test("string_interpolation");
}

#[test]
fn idempotency_comment() {
    run_idempotency_test("comment");
}

// ============================================================================
// Idempotency Tests - Types and Values
// ============================================================================

#[test]
fn idempotency_value_with_unit() {
    run_idempotency_test("value_with_unit");
}

#[test]
fn idempotency_datetime() {
    run_idempotency_test("datetime");
}

#[test]
fn idempotency_nothing() {
    run_idempotency_test("nothing");
}

#[test]
fn idempotency_glob_pattern() {
    run_idempotency_test("glob_pattern");
}

// ============================================================================
// Idempotency Tests - Modules and Imports
// ============================================================================

#[test]
fn idempotency_module() {
    run_idempotency_test("module");
}

#[test]
fn idempotency_use_statement() {
    run_idempotency_test("use_statement");
}

#[test]
fn idempotency_export() {
    run_idempotency_test("export");
}

#[test]
fn idempotency_source() {
    run_idempotency_test("source");
}

#[test]
fn idempotency_hide() {
    run_idempotency_test("hide");
}

#[test]
fn idempotency_overlay() {
    run_idempotency_test("overlay");
}

// ============================================================================
// Idempotency Tests - Commands and Definitions
// ============================================================================

#[test]
fn idempotency_alias() {
    run_idempotency_test("alias");
}

#[test]
fn idempotency_extern() {
    run_idempotency_test("extern");
}

#[test]
fn idempotency_external_call() {
    run_idempotency_test("external_call");
}

// ============================================================================
// Idempotency Tests - Special Constructs
// ============================================================================

#[test]
fn idempotency_do_block() {
    run_idempotency_test("do_block");
}

#[test]
fn idempotency_where_clause() {
    run_idempotency_test("where_clause");
}

#[test]
fn idempotency_error_make() {
    run_idempotency_test("error_make");
}
