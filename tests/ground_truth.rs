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
fn ground_truth_double_parentheses_for_subexpression_issue76() {
    let test_binary = get_test_binary();
    run_ground_truth_test(&test_binary, "double_parentheses_for_subexpression_issue76");
}

// Issue regression tests
fixture_tests!(
    (
        "custom_completion_signature_preserved_issue81",
        ground_truth_custom_completion_signature_preserved_issue81,
        idempotency_custom_completion_signature_preserved_issue81
    ),
    (
        "optional_access_question_mark_position_preserved_issue85",
        ground_truth_optional_access_question_mark_position_preserved_issue85,
        idempotency_optional_access_question_mark_position_preserved_issue85
    ),
    (
        "closure_type_hint_not_rewritten_as_call_issue86",
        ground_truth_closure_type_hint_not_rewritten_as_call_issue86,
        idempotency_closure_type_hint_not_rewritten_as_call_issue86
    ),
    (
        "extern_completion_annotations_preserved_issue87",
        ground_truth_extern_completion_annotations_preserved_issue87,
        idempotency_extern_completion_annotations_preserved_issue87
    ),
    (
        "pipeline_io_signature_preserved_issue92",
        ground_truth_pipeline_io_signature_preserved_issue92,
        idempotency_pipeline_io_signature_preserved_issue92
    ),
    (
        "if_pipeline_condition_parentheses_preserved_issue93",
        ground_truth_if_pipeline_condition_parentheses_preserved_issue93,
        idempotency_if_pipeline_condition_parentheses_preserved_issue93
    ),
    (
        "variable_type_annotations_preserved_issue94",
        ground_truth_variable_type_annotations_preserved_issue94,
        idempotency_variable_type_annotations_preserved_issue94
    ),
    (
        "flag_equals_subexpression_syntax_preserved_issue95",
        ground_truth_flag_equals_subexpression_syntax_preserved_issue95,
        idempotency_flag_equals_subexpression_syntax_preserved_issue95
    ),
    (
        "optional_access_order_preserved_issue97",
        ground_truth_optional_access_order_preserved_issue97,
        idempotency_optional_access_order_preserved_issue97
    ),
    (
        "at_category_attribute_preserved_issue100",
        ground_truth_at_category_attribute_preserved_issue100,
        idempotency_at_category_attribute_preserved_issue100
    ),
    (
        "where_in_def_does_not_emit_parser_errors_issue101",
        ground_truth_where_in_def_does_not_emit_parser_errors_issue101,
        idempotency_where_in_def_does_not_emit_parser_errors_issue101
    ),
    (
        "space_separated_list_literals_preserved_issue108",
        ground_truth_space_separated_list_literals_preserved_issue108,
        idempotency_space_separated_list_literals_preserved_issue108
    ),
    (
        "for_loop_multiline_block_body_preserved_issue109",
        ground_truth_for_loop_multiline_block_body_preserved_issue109,
        idempotency_for_loop_multiline_block_body_preserved_issue109
    ),
    (
        "multiline_call_arguments_preserved_issue110",
        ground_truth_multiline_call_arguments_preserved_issue110,
        idempotency_multiline_call_arguments_preserved_issue110
    ),
    (
        "let_rhs_pipeline_parentheses_preserved_issue116",
        ground_truth_let_rhs_pipeline_parentheses_preserved_issue116,
        idempotency_let_rhs_pipeline_parentheses_preserved_issue116
    ),
    (
        "if_pipeline_condition_avoids_parser_noise_issue119",
        ground_truth_if_pipeline_condition_avoids_parser_noise_issue119,
        idempotency_if_pipeline_condition_avoids_parser_noise_issue119
    ),
    (
        "tightly_packed_if_else_spacing_normalized_issue120",
        ground_truth_tightly_packed_if_else_spacing_normalized_issue120,
        idempotency_tightly_packed_if_else_spacing_normalized_issue120
    ),
    (
        "invalid_if_else_parse_recovery_is_safe_issue121",
        ground_truth_invalid_if_else_parse_recovery_is_safe_issue121,
        idempotency_invalid_if_else_parse_recovery_is_safe_issue121
    ),
    (
        "parse_recovery_preserves_record_strings_issue122",
        ground_truth_parse_recovery_preserves_record_strings_issue122,
        idempotency_parse_recovery_preserves_record_strings_issue122
    ),
    (
        "margin_two_keeps_adjacent_use_statements_tight_issue126",
        ground_truth_margin_two_keeps_adjacent_use_statements_tight_issue126,
        idempotency_margin_two_keeps_adjacent_use_statements_tight_issue126
    ),
    (
        "margin_one_preserves_vertical_spacing_groups_issue127",
        ground_truth_margin_one_preserves_vertical_spacing_groups_issue127,
        idempotency_margin_one_preserves_vertical_spacing_groups_issue127
    ),
    (
        "module_doc_comment_spacing_preserved_issue128",
        ground_truth_module_doc_comment_spacing_preserved_issue128,
        idempotency_module_doc_comment_spacing_preserved_issue128
    ),
    (
        "single_line_record_literals_preserved_issue129",
        ground_truth_single_line_record_literals_preserved_issue129,
        idempotency_single_line_record_literals_preserved_issue129
    ),
    (
        "empty_record_literals_normalized_issue130",
        ground_truth_empty_record_literals_normalized_issue130,
        idempotency_empty_record_literals_normalized_issue130
    ),
    (
        "return_subexpression_parentheses_preserved_issue131",
        ground_truth_return_subexpression_parentheses_preserved_issue131,
        idempotency_return_subexpression_parentheses_preserved_issue131
    ),
    (
        "for_loop_type_annotation_preserved_issue132",
        ground_truth_for_loop_type_annotation_preserved_issue132,
        idempotency_for_loop_type_annotation_preserved_issue132
    ),
    (
        "inline_comment_after_subexpression_preserved_issue133",
        ground_truth_inline_comment_after_subexpression_preserved_issue133,
        idempotency_inline_comment_after_subexpression_preserved_issue133
    ),
    (
        "pipeline_subexpression_parentheses_and_layout_preserved_issue134",
        ground_truth_pipeline_subexpression_parentheses_and_layout_preserved_issue134,
        idempotency_pipeline_subexpression_parentheses_and_layout_preserved_issue134
    ),
    (
        "mixed_use_and_def_does_not_emit_parser_errors_issue136",
        ground_truth_mixed_use_and_def_does_not_emit_parser_errors_issue136,
        idempotency_mixed_use_and_def_does_not_emit_parser_errors_issue136
    ),
    (
        "export_const_type_annotation_preserved_issue137",
        ground_truth_export_const_type_annotation_preserved_issue137,
        idempotency_export_const_type_annotation_preserved_issue137
    ),
    (
        "compact_function_parameter_list_preserved_issue138",
        ground_truth_compact_function_parameter_list_preserved_issue138,
        idempotency_compact_function_parameter_list_preserved_issue138
    ),
    (
        "match_guards_preserved_issue139",
        ground_truth_match_guards_preserved_issue139,
        idempotency_match_guards_preserved_issue139
    ),
    (
        "catch_block_indentation_and_closing_brace_preserved_issue140",
        ground_truth_catch_block_indentation_and_closing_brace_preserved_issue140,
        idempotency_catch_block_indentation_and_closing_brace_preserved_issue140
    ),
    (
        "cell_path_in_def_block_does_not_emit_parser_errors_issue141",
        ground_truth_cell_path_in_def_block_does_not_emit_parser_errors_issue141,
        idempotency_cell_path_in_def_block_does_not_emit_parser_errors_issue141
    ),
    (
        "compact_cell_path_lists_preserved_issue142",
        ground_truth_compact_cell_path_lists_preserved_issue142,
        idempotency_compact_cell_path_lists_preserved_issue142
    ),
    (
        "if_condition_call_parentheses_preserved_issue143",
        ground_truth_if_condition_call_parentheses_preserved_issue143,
        idempotency_if_condition_call_parentheses_preserved_issue143
    ),
    (
        "long_command_calls_wrap_to_line_length_issue144",
        ground_truth_long_command_calls_wrap_to_line_length_issue144,
        idempotency_long_command_calls_wrap_to_line_length_issue144
    ),
    (
        "redundant_pipeline_parentheses_simplified_issue145",
        ground_truth_redundant_pipeline_parentheses_simplified_issue145,
        idempotency_redundant_pipeline_parentheses_simplified_issue145
    ),
    (
        "if_else_comment_and_statement_placement_preserved_issue146",
        ground_truth_if_else_comment_and_statement_placement_preserved_issue146,
        idempotency_if_else_comment_and_statement_placement_preserved_issue146
    ),
    (
        "match_arm_alignment_preserved_issue106",
        ground_truth_match_arm_alignment_preserved_issue106,
        idempotency_match_arm_alignment_preserved_issue106
    ),
    (
        "comment_spacing_before_toplevel_statements_issue150",
        ground_truth_comment_spacing_before_toplevel_statements_issue150,
        idempotency_comment_spacing_before_toplevel_statements_issue150
    ),
    (
        "list_flag_value_pairing_preserved_issue151",
        ground_truth_list_flag_value_pairing_preserved_issue151,
        idempotency_list_flag_value_pairing_preserved_issue151
    ),
    (
        "multiline_list_layout_preserved_issue152",
        ground_truth_multiline_list_layout_preserved_issue152,
        idempotency_multiline_list_layout_preserved_issue152
    ),
    (
        "consecutive_let_const_grouping_normalized_issue153",
        ground_truth_consecutive_let_const_grouping_normalized_issue153,
        idempotency_consecutive_let_const_grouping_normalized_issue153
    ),
    (
        "margin_respected_inside_nested_blocks_issue154",
        ground_truth_margin_respected_inside_nested_blocks_issue154,
        idempotency_margin_respected_inside_nested_blocks_issue154
    ),
    (
        "nested_pipeline_expansion_rules_applied_issue155",
        ground_truth_nested_pipeline_expansion_rules_applied_issue155,
        idempotency_nested_pipeline_expansion_rules_applied_issue155
    ),
    (
        "assignment_pipeline_redundant_parens_removed_issue156",
        ground_truth_assignment_pipeline_redundant_parens_removed_issue156,
        idempotency_assignment_pipeline_redundant_parens_removed_issue156
    ),
    (
        "identifier_safe_match_patterns_unquoted_issue157",
        ground_truth_identifier_safe_match_patterns_unquoted_issue157,
        idempotency_identifier_safe_match_patterns_unquoted_issue157
    ),
    (
        "single_item_list_inline_and_if_layout_preserved_issue158",
        ground_truth_single_item_list_inline_and_if_layout_preserved_issue158,
        idempotency_single_item_list_inline_and_if_layout_preserved_issue158
    ),
    (
        "nested_closure_indentation_normalized_issue159",
        ground_truth_nested_closure_indentation_normalized_issue159,
        idempotency_nested_closure_indentation_normalized_issue159
    ),
    (
        "closure_argument_pipe_spacing_normalized_issue160",
        ground_truth_closure_argument_pipe_spacing_normalized_issue160,
        idempotency_closure_argument_pipe_spacing_normalized_issue160
    ),
    (
        "parens_stripping_boolean_exprs_issue162",
        ground_truth_parens_stripping_boolean_exprs_issue162,
        idempotency_parens_stripping_boolean_exprs_issue162
    ),
);
