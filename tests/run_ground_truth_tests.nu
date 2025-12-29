#!/usr/bin/env nu
# Ground truth test runner for nufmt
# Run with: nu tests/run_ground_truth_tests.nu

# Configuration
const NUFMT_BINARY = "./target/release/nufmt"
const INPUT_DIR = "tests/fixtures/input"
const EXPECTED_DIR = "tests/fixtures/expected"

# All test constructs organized by category
const TEST_CONSTRUCTS = {
    core: ["let_statement", "mut_statement", "const_statement", "def_statement"]
    control_flow: [
        "if_else"
        "for_loop"
        "while_loop"
        "loop_statement"
        "match_expr"
        "try_catch"
        "break_continue"
        "return_statement"
    ]
    data_structures: ["list", "record", "table", "nested_structures"]
    pipelines_expressions: [
        "pipeline"
        "multiline_pipeline"
        "closure"
        "subexpression"
        "binary_ops"
        "range"
        "cell_path"
        "spread"
    ]
    strings_interpolation: ["string_interpolation", "comment"]
    types_values: ["value_with_unit", "datetime", "nothing", "glob_pattern"]
    modules_imports: [
        "module"
        "use_statement"
        "export"
        "source"
        "hide"
        "overlay"
    ]
    commands_definitions: ["alias", "extern", "external_call"]
    special_constructs: ["do_block", "where_clause", "error_make"]
}

# Colors for output
def green [text: string] { $"(ansi green)($text)(ansi reset)" }
def red [text: string] { $"(ansi red)($text)(ansi reset)" }
def yellow [text: string] { $"(ansi yellow)($text)(ansi reset)" }
def cyan [text: string] { $"(ansi cyan)($text)(ansi reset)" }
def bold [text: string] { $"(ansi white_bold)($text)(ansi reset)" }

# Test result record
def make_result [name: string, passed: bool, message: string = ""] {
    {name: $name, passed: $passed, message: $message}
}

# Run a single ground truth test
def run_test [name: string] {
    let input_file = $"($INPUT_DIR)/($name).nu"
    let expected_file = $"($EXPECTED_DIR)/($name).nu"

    # Check if files exist
    if not ($input_file | path exists) {
        return (make_result $name false $"Input file not found: ($input_file)")
    }
    if not ($expected_file | path exists) {
        return (make_result $name false $"Expected file not found: ($expected_file)")
    }

    # Read input
    let input = open $input_file

    # Run formatter
    let result = try {
        $input | ^$NUFMT_BINARY --stdin
    } catch {
        return (make_result $name false $"Formatter error")
    }

    # Read expected output
    let expected = open $expected_file

    # Compare (normalize line endings and trim)
    let formatted_normalized = $result | str trim
    let expected_normalized = $expected | str trim

    if $formatted_normalized == $expected_normalized {
        make_result $name true
    } else {
        let diff_msg = $"Output differs from expected.\n--- Expected ---\n($expected_normalized)\n--- Got ---\n($formatted_normalized)"
        make_result $name false $diff_msg
    }
}

# Run idempotency test
def run_idempotency_test [name: string] {
    let input_file = $"($INPUT_DIR)/($name).nu"

    if not ($input_file | path exists) {
        return (make_result $"($name)_idempotency" false $"Input file not found")
    }

    let input = open $input_file

    # First format
    let first = try {
        $input | ^$NUFMT_BINARY --stdin
    } catch {
        return (make_result $"($name)_idempotency" false "First format failed")
    }

    # Second format
    let second = try {
        $first | ^$NUFMT_BINARY --stdin
    } catch {
        return (make_result $"($name)_idempotency" false "Second format failed")
    }

    if ($first | str trim) == ($second | str trim) {
        make_result $"($name)_idempotency" true
    } else {
        make_result $"($name)_idempotency" false "Output changed on second format"
    }
}

# Get all test names from input directory
def get_test_names [] {
    ls $INPUT_DIR
    | where name =~ '\.nu$'
    | get name
    | each {|f| $f | path basename | str replace '.nu' ''}
}

# Get all test names from the constructs definition
def get_all_defined_tests [] {
    $TEST_CONSTRUCTS | values | flatten
}

# Get tests by category
def get_tests_by_category [category: string] {
    if ($category in $TEST_CONSTRUCTS) {
        $TEST_CONSTRUCTS | get $category
    } else {
        []
    }
}

# Print a section header
def print_section [title: string] {
    print ""
    print (bold $"── ($title) ──")
}

# Main test runner
def main [
    --test(-t): string       # Run specific test by name
    --category(-c): string   # Run tests in a specific category
    --idempotency(-i)        # Only run idempotency tests
    --ground-truth(-g)       # Only run ground truth tests
    --verbose(-v)            # Show detailed output for failures
    --list(-l)               # List available tests
    --list-categories        # List available categories
    --check-files            # Check which test files exist
] {
    print (bold "=== nufmt Ground Truth Test Runner ===")
    print ""

    # Check if binary exists
    if not ($NUFMT_BINARY | path exists) {
        print (red $"Error: nufmt binary not found at ($NUFMT_BINARY)")
        print "Run 'cargo build --release' first"
        exit 1
    }

    # List categories
    if $list_categories {
        print "Available test categories:"
        for category in ($TEST_CONSTRUCTS | columns) {
            let count = ($TEST_CONSTRUCTS | get $category | length)
            print $"  - (cyan $category) \(($count) tests\)"
        }
        return
    }

    # List tests
    if $list {
        print "Available tests by category:"
        for category in ($TEST_CONSTRUCTS | columns) {
            print_section $category
            for name in ($TEST_CONSTRUCTS | get $category) {
                let input_exists = ($"($INPUT_DIR)/($name).nu" | path exists)
                let expected_exists = ($"($EXPECTED_DIR)/($name).nu" | path exists)
                let status = if $input_exists and $expected_exists {
                    green "✓"
                } else if $input_exists {
                    yellow "○"  # missing expected
                } else {
                    red "✗"  # missing input
                }
                print $"  ($status) ($name)"
            }
        }
        return
    }

    # Check files
    if $check_files {
        print "Checking test file status..."
        print ""

        let defined_tests = get_all_defined_tests
        let existing_inputs = get_test_names

        mut missing_inputs = []
        mut missing_expected = []
        mut undefined_tests = []

        for test in $defined_tests {
            if not ($"($INPUT_DIR)/($test).nu" | path exists) {
                $missing_inputs = ($missing_inputs | append $test)
            }
            if not ($"($EXPECTED_DIR)/($test).nu" | path exists) {
                $missing_expected = ($missing_expected | append $test)
            }
        }

        for test in $existing_inputs {
            if not ($test in $defined_tests) {
                $undefined_tests = ($undefined_tests | append $test)
            }
        }

        if ($missing_inputs | length) > 0 {
            print (red "Missing input files:")
            for t in $missing_inputs { print $"  - ($t)" }
        }

        if ($missing_expected | length) > 0 {
            print (yellow "Missing expected files:")
            for t in $missing_expected { print $"  - ($t)" }
        }

        if ($undefined_tests | length) > 0 {
            print (cyan "Tests not in category definition:")
            for t in $undefined_tests { print $"  - ($t)" }
        }

        if ($missing_inputs | length) == 0 and ($missing_expected | length) == 0 {
            print (green "All defined tests have both input and expected files!")
        }
        return
    }

    # Determine which tests to run
    let tests_to_run = if $test != null {
        [$test]
    } else if $category != null {
        get_tests_by_category $category
    } else {
        get_all_defined_tests
    }

    if ($tests_to_run | is-empty) {
        print (red "No tests found to run")
        if $category != null {
            print $"Unknown category: ($category)"
            print "Use --list-categories to see available categories"
        }
        exit 1
    }

    mut results = []

    # Run ground truth tests
    if not $idempotency {
        print (bold "Running ground truth tests...")
        for name in $tests_to_run {
            # Check if files exist before running
            let input_exists = ($"($INPUT_DIR)/($name).nu" | path exists)
            let expected_exists = ($"($EXPECTED_DIR)/($name).nu" | path exists)

            if not $input_exists or not $expected_exists {
                let result = make_result $name false "Missing test files"
                $results = ($results | append $result)
                print $"  (yellow '○') ($name) - missing files"
                continue
            }

            let result = run_test $name
            $results = ($results | append $result)

            if $result.passed {
                print $"  (green '✓') ($name)"
            } else {
                print $"  (red '✗') ($name)"
                if $verbose {
                    print $"    ($result.message)"
                }
            }
        }
        print ""
    }

    # Run idempotency tests
    if not $ground_truth {
        print (bold "Running idempotency tests...")
        for name in $tests_to_run {
            # Check if input file exists
            if not ($"($INPUT_DIR)/($name).nu" | path exists) {
                # Skip silently for idempotency if no input
                continue
            }

            let result = run_idempotency_test $name
            $results = ($results | append $result)

            if $result.passed {
                print $"  (green '✓') ($result.name)"
            } else {
                print $"  (red '✗') ($result.name)"
                if $verbose {
                    print $"    ($result.message)"
                }
            }
        }
        print ""
    }

    # Summary
    let passed = $results | where passed | length
    let failed = $results | where {|r| not $r.passed} | length
    let total = $results | length

    print (bold "=== Summary ===")
    print $"Total: ($total)"
    print $"Passed: (green ($passed | into string))"
    print $"Failed: (if $failed > 0 { red ($failed | into string) } else { $failed | into string })"

    if $failed > 0 {
        print ""
        print (bold "Failed tests:")
        for result in ($results | where {|r| not $r.passed}) {
            print $"  - ($result.name)"
            if $verbose and ($result.message | str length) > 0 {
                print $"      ($result.message | str substring 0..200)..."
            }
        }
        exit 1
    }

    print ""
    print (green "All tests passed!")
}
