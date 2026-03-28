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
// Fixture test macros — generate paired ground truth + idempotency tests
// ============================================================================

/// Generate a ground-truth test and an idempotency test for each fixture.
macro_rules! fixture_tests {
    ($(($fixture:literal, $ground_truth_test:ident, $idempotency_test:ident)),+ $(,)?) => {
        $(
            #[test]
            fn $ground_truth_test() {
                let test_binary = get_test_binary();
                run_ground_truth_test(&test_binary, $fixture);
            }

            #[test]
            fn $idempotency_test() {
                let test_binary = get_test_binary();
                run_idempotency_test(&test_binary, $fixture);
            }
        )+
    };
}

// Core language constructs
fixture_tests!(
    (
        "let_statement",
        ground_truth_let_statement,
        idempotency_let_statement
    ),
    (
        "mut_statement",
        ground_truth_mut_statement,
        idempotency_mut_statement
    ),
    (
        "const_statement",
        ground_truth_const_statement,
        idempotency_const_statement
    ),
    (
        "def_statement",
        ground_truth_def_statement,
        idempotency_def_statement
    ),
);

// Control flow
fixture_tests!(
    ("if_else", ground_truth_if_else, idempotency_if_else),
    ("for_loop", ground_truth_for_loop, idempotency_for_loop),
    (
        "while_loop",
        ground_truth_while_loop,
        idempotency_while_loop
    ),
    (
        "loop_statement",
        ground_truth_loop_statement,
        idempotency_loop_statement
    ),
    (
        "match_expr",
        ground_truth_match_expr,
        idempotency_match_expr
    ),
    ("try_catch", ground_truth_try_catch, idempotency_try_catch),
    (
        "break_continue",
        ground_truth_break_continue,
        idempotency_break_continue
    ),
    (
        "return_statement",
        ground_truth_return_statement,
        idempotency_return_statement
    ),
);

// Data structures
fixture_tests!(
    ("list", ground_truth_list, idempotency_list),
    ("record", ground_truth_record, idempotency_record),
    ("table", ground_truth_table, idempotency_table),
    (
        "nested_structures",
        ground_truth_nested_structures,
        idempotency_nested_structures
    ),
);

// Pipelines, expressions, and operators
fixture_tests!(
    ("pipeline", ground_truth_pipeline, idempotency_pipeline),
    (
        "multiline_pipeline",
        ground_truth_multiline_pipeline,
        idempotency_multiline_pipeline
    ),
    ("closure", ground_truth_closure, idempotency_closure),
    (
        "subexpression",
        ground_truth_subexpression,
        idempotency_subexpression
    ),
    (
        "binary_ops",
        ground_truth_binary_ops,
        idempotency_binary_ops
    ),
    ("range", ground_truth_range, idempotency_range),
    (
        "cell_path_literals",
        ground_truth_cell_path_literals,
        idempotency_cell_path_literals
    ),
    ("cell_path", ground_truth_cell_path, idempotency_cell_path),
    ("spread", ground_truth_spread, idempotency_spread),
);

// Strings, comments, types, and values
fixture_tests!(
    (
        "string_interpolation",
        ground_truth_string_interpolation,
        idempotency_string_interpolation
    ),
    ("comment", ground_truth_comment, idempotency_comment),
    (
        "value_with_unit",
        ground_truth_value_with_unit,
        idempotency_value_with_unit
    ),
    ("datetime", ground_truth_datetime, idempotency_datetime),
    ("nothing", ground_truth_nothing, idempotency_nothing),
    (
        "glob_pattern",
        ground_truth_glob_pattern,
        idempotency_glob_pattern
    ),
);

// Modules and imports
fixture_tests!(
    ("module", ground_truth_module, idempotency_module),
    (
        "use_statement",
        ground_truth_use_statement,
        idempotency_use_statement
    ),
    ("export", ground_truth_export, idempotency_export),
    ("source", ground_truth_source, idempotency_source),
    ("hide", ground_truth_hide, idempotency_hide),
    ("overlay", ground_truth_overlay, idempotency_overlay),
);

// Commands, definitions, and special constructs
fixture_tests!(
    ("alias", ground_truth_alias, idempotency_alias),
    ("extern", ground_truth_extern, idempotency_extern),
    (
        "external_call",
        ground_truth_external_call,
        idempotency_external_call
    ),
    ("do_block", ground_truth_do_block, idempotency_do_block),
    (
        "where_clause",
        ground_truth_where_clause,
        idempotency_where_clause
    ),
    (
        "error_make",
        ground_truth_error_make,
        idempotency_error_make
    ),
    (
        "inline_param_comment",
        ground_truth_inline_param_comment_issue77,
        idempotency_inline_param_comment_issue77
    ),
);

// Ground-truth-only tests (no idempotency pair)
#[test]
fn ground_truth_def_with_pipeline() {
    let test_binary = get_test_binary();
    run_ground_truth_test(&test_binary, "def_with_pipeline_double_parens_issue82");
}

#[test]
fn issue76_test() {
    let test_binary = get_test_binary();
    run_ground_truth_test(&test_binary, "issue76");
}

// Issue regression tests
fixture_tests!(
    ("issue81", issue81_test, idempotency_issue81_test),
    ("issue85", issue85_test, idempotency_issue85_test),
    ("issue86", issue86_test, idempotency_issue86_test),
    ("issue87", issue87_test, idempotency_issue87_test),
    ("issue92", issue92_test, idempotency_issue92_test),
    ("issue93", issue93_test, idempotency_issue93_test),
    ("issue94", issue94_test, idempotency_issue94_test),
    ("issue95", issue95_test, idempotency_issue95_test),
    ("issue97", issue97_test, idempotency_issue97_test),
    ("issue100", issue100_test, idempotency_issue100_test),
    ("issue101", issue101_test, idempotency_issue101_test),
    ("issue108", issue108_test, idempotency_issue108_test),
    ("issue109", issue109_test, idempotency_issue109_test),
    ("issue110", issue110_test, idempotency_issue110_test),
    ("issue116", issue116_test, idempotency_issue116_test),
    ("issue119", issue119_test, idempotency_issue119_test),
    ("issue120", issue120_test, idempotency_issue120_test),
    ("issue121", issue121_test, idempotency_issue121_test),
    ("issue122", issue122_test, idempotency_issue122_test),
    ("issue126", issue126_test, idempotency_issue126_test),
    ("issue127", issue127_test, idempotency_issue127_test),
    ("issue128", issue128_test, idempotency_issue128_test),
    ("issue129", issue129_test, idempotency_issue129_test),
    ("issue130", issue130_test, idempotency_issue130_test),
    ("issue131", issue131_test, idempotency_issue131_test),
    ("issue132", issue132_test, idempotency_issue132_test),
    ("issue133", issue133_test, idempotency_issue133_test),
    ("issue134", issue134_test, idempotency_issue134_test),
    ("issue136", issue136_test, idempotency_issue136_test),
    ("issue137", issue137_test, idempotency_issue137_test),
    ("issue138", issue138_test, idempotency_issue138_test),
    ("issue139", issue139_test, idempotency_issue139_test),
    ("issue140", issue140_test, idempotency_issue140_test),
    ("issue141", issue141_test, idempotency_issue141_test),
    ("issue142", issue142_test, idempotency_issue142_test),
    ("issue143", issue143_test, idempotency_issue143_test),
    ("issue144", issue144_test, idempotency_issue144_test),
    ("issue145", issue145_test, idempotency_issue145_test),
    ("issue146", issue146_test, idempotency_issue146_test),
);
