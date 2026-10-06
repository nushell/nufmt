#!/usr/bin/env nu
# Run with: nu tests/run_ground_truth_tests.nu

let FIXTURES_DIR = $env.FILE_PWD | path join "fixtures"
let PROJECT_DIR = $env.FILE_PWD | path dirname
let NUFMT_BINARY = $PROJECT_DIR | path join "target" "release" "nufmt"

def make_result [name: string, passed: bool, message: string = ""] {
    {name: $name, passed: $passed, message: $message}
}

def list_directories [directory: string] {
    try {
        {directories: (ls --all $directory | where type == dir | sort-by name), error: null}
    } catch {|err|
        {directories: [], error: $err.msg}
    }
}

# Only category directories and their immediate children are test directories.
def discover_tests [] {
    let categories = list_directories $FIXTURES_DIR
    mut tests = []
    mut errors = []
    if $categories.error != null {
        $errors = ($errors | append (make_result $FIXTURES_DIR false $categories.error))
    }
    for category in $categories.directories {
        let children = list_directories $category.name
        if $children.error != null {
            $errors = ($errors | append (make_result $category.name false $children.error))
        }
        let category_name = $category.name | path basename
        for child in $children.directories {
            let name = $child.name | path basename
            $tests = ($tests | append {
                name: $"($category_name)/($name)"
                category: $category_name
                path: $child.name
            })
        }
    }
    if ($tests | is-empty) and ($errors | is-empty) {
        $errors = [(make_result $FIXTURES_DIR false "No fixture directories found")]
    }
    {
        tests: $tests
        categories: ($categories.directories | get name | each {|p| $p | path basename})
        errors: $errors
    }
}

def check_format [
    binary: string
    test: record
    source: string
    expected: binary
    artifact: string
    config_args: list<string>
] {
    let source_path = $test.path | path join $source
    let name = $"($test.name)/($source)"
    try {
        let input = open --raw $source_path | into binary
        let output = do {
            cd $PROJECT_DIR
            $input | ^$binary --stdin ...$config_args
        } | complete
        if $output.exit_code != 0 {
            error make {msg: $"nufmt exited with ($output.exit_code): ($output.stderr)"}
        }

        let actual = $output.stdout | into binary
        let artifact_path = $test.path | path join $artifact
        if $actual != $expected {
            $actual | save --raw --force $artifact_path
            make_result $name false $"Differs after one format; see ($artifact_path)"
        } else {
            if ($artifact_path | path exists) {
                rm $artifact_path
            }
            make_result $name true
        }
    } catch {|err|
        make_result $name false $err.msg
    }
}

def run_fixture [
    binary: string
    test: record
    check_expected: bool
    check_input: bool
] {
    try {
        let config_path = $test.path | path join "config.noun"
        let config_args = if ($config_path | path exists) {
            ["--config" $config_path]
        } else {
            []
        }
        let expected = open --raw ($test.path | path join "expected.nu") | into binary
        mut results = []
        if $check_expected {
            $results = ($results | append (
                check_format $binary $test "expected.nu" $expected "not_idempotent.nu" $config_args
            ))
        }
        if $check_input and ($test.path | path join "input.nu" | path exists) {
            $results = ($results | append (
                check_format $binary $test "input.nu" $expected "unexpected.nu" $config_args
            ))
        }
        $results
    } catch {|err|
        [(make_result $test.name false $err.msg)]
    }
}

def main [
    --test(-t): string       # Run a case by category/name or its unique name
    --category(-c): string   # Run cases in this category directory
    --idempotency(-i)        # Only format expected.nu once
    --ground-truth(-g)       # Only format existing input.nu once
    --verbose(-v)            # Show failure details
    --list(-l)               # List discovered cases
    --list-categories        # List discovered category directories
    --check-files            # Check required expected.nu files; input.nu is optional
] {
    if $idempotency and $ground_truth {
        error make {msg: "--idempotency and --ground-truth cannot be used together"}
    }

    let discovery = discover_tests
    let tests = $discovery.tests
    if $list_categories {
        for category in $discovery.categories {
            let count = $tests | where category == $category | length
            print $"($category): ($count) fixtures"
        }
    } else if $list or $check_files {
        for case in $tests {
            let expected_exists = $case.path | path join "expected.nu" | path exists
            let input_exists = $case.path | path join "input.nu" | path exists
            let status = if $expected_exists {
                if $input_exists { "expected + input" } else { "expected only" }
            } else {
                "missing expected.nu"
            }
            if $list or not $expected_exists {
                print $"($case.name): ($status)"
            }
        }
        if $check_files {
            let missing = $tests | where {|case|
                not ($case.path | path join "expected.nu" | path exists)
            }
            if not ($missing | is-empty) {
                error make {msg: $"Missing expected.nu in ($missing | get name | str join ', ')"}
            }
            print $"Checked ($tests | length) fixtures; input.nu is optional"
        }
    }

    if $list or $list_categories or $check_files {
        if not ($discovery.errors | is-empty) {
            error make {msg: ($discovery.errors | get message | str join "\n")}
        }
        return
    }

    let selected = if $test != null {
        let matches = $tests | where {|case|
            $case.name == $test or ($case.path | path basename) == $test
        }
        if ($matches | length) > 1 {
            error make {msg: $"Ambiguous test name ($test); use ($matches | get name | str join ', ')"}
        }
        $matches
    } else if $category != null {
        $tests | where category == $category
    } else {
        $tests
    }
    if ($selected | is-empty) {
        error make {msg: "No matching fixtures found; use --list or --list-categories"}
    }
    if not ($NUFMT_BINARY | path exists) {
        error make {msg: $"nufmt binary not found at ($NUFMT_BINARY); run cargo build --release first"}
    }

    mut results = $discovery.errors
    for case in $selected {
        let checks = run_fixture $NUFMT_BINARY $case (not $ground_truth) (not $idempotency)
        for result in $checks {
            let status = if $result.passed { "PASS" } else { "FAIL" }
            print $"($status) ($result.name)"
            if $verbose and not $result.passed {
                print $result.message
            }
        }
        $results = ($results | append $checks)
    }

    let failed = $results | where passed == false
    print $"Fixtures: ($selected | length)"
    print $"Checks: ($results | length), passed: ($results | where passed | length), failed: ($failed | length)"
    for result in $failed {
        print $"FAIL ($result.name): ($result.message)"
    }
    if not ($failed | is-empty) {
        exit 1
    }
}
