# this module regroups a bunch of development tools to make the development
# process easier for anyone.
#
# the main purpose of `toolkit` is to offer an easy to use interface for the
# developer during a PR cycle, namely to (**1**) format the source base,
# (**2**) catch classical flaws in the new changes with *clippy* and (**3**)
# make sure all the tests pass.

# check standard code formatting and apply the changes
export def fmt [
    --check: bool  # do not apply the format changes, only check the syntax
    --verbose: bool # print extra information about the command's progress
] {
    if $verbose {
        print $"running ('toolkit fmt' | pretty-print-command)"
    }

    if $check {
        try {
            cargo fmt --all -- --check
        } catch {
            error make -u { msg: $"\nplease run ('toolkit fmt' | pretty-print-command) to fix formatting!" }
        }
    } else {
        cargo fmt --all
    }
}

# check that you're using the standard code style
#
# > it is important to make `clippy` happy :relieved:
export def clippy [
    --verbose: bool # print extra information about the command's progress
    --dataframe: bool # use the dataframe feature
] {
    if $verbose {
        print $"running ('toolkit clippy' | pretty-print-command)"
    }
    # clippy help
    # To allow or deny a lint from the command line you can use `cargo clippy --`
    # with:
    #     -W --warn OPT       Set lint warnings
    #     -A --allow OPT      Set lint allowed
    #     -D --deny OPT       Set lint denied
    #     -F --forbid OPT     Set lint forbidden
    try {
        if $dataframe {

            cargo clippy --workspace --features=dataframe -- -D warnings -D clippy::missing_docs_in_private_items -D clippy::explicit_iter_loop -D clippy::explicit_into_iter_loop -D clippy::semicolon_if_nothing_returned -D clippy::doc_markdown -D clippy::manual_let_else
        } else {
            cargo clippy --workspace -- -D warnings -D clippy::missing_docs_in_private_items -D clippy::explicit_iter_loop -D clippy::explicit_into_iter_loop -D clippy::semicolon_if_nothing_returned -D clippy::doc_markdown -D clippy::manual_let_else
        }
    } catch {
        error make -u { msg: $"\nplease fix the above ('clippy' | pretty-print-command) errors before continuing!" }
    }
}

# check that all the tests pass
export def test [
    --fast: bool  # use the "nextext" `cargo` subcommand to speed up the tests (see [`cargo-nextest`](https://nexte.st/) and [`nextest-rs/nextest`](https://github.com/nextest-rs/nextest))
    --dataframe: bool # use the dataframe feature
] {
    if ($fast and $dataframe) {
        cargo nextest run --all --features=dataframe
    } else if ($fast) {
        cargo nextest run --all
    } else if ($dataframe) {
        cargo test --workspace --features=dataframe
    } else {
        cargo test --workspace
    }
}

# print the pipe input inside backticks, dimmed and italic, as a pretty command
def pretty-print-command [] {
    $"`(ansi default_dimmed)(ansi default_italic)($in)(ansi reset)`"
}

# return a report about the check stage
#
# - fmt comes first
# - then clippy
# - and finally the tests
#
# without any option, `report` will return an empty report.
# otherwise, the truth values will be incremental, following
# the order above.
export def report [
    --fail-fmt: bool
    --fail-clippy: bool
    --fail-test: bool
    --no-fail: bool
] {
    [fmt clippy test]
    | wrap stage
    | merge (
        if $no_fail               { [true     true     true     true] }
        else if $fail_fmt         { [false    $nothing $nothing $nothing] }
        else if $fail_clippy      { [true     false    $nothing $nothing] }
        else if $fail_test        { [true     true     false    $nothing] }
        else                      { [$nothing $nothing $nothing $nothing] }
        | wrap success
    )
    | upsert emoji {|it|
        if ($it.success == $nothing) {
            ":black_circle:"
        } else if $it.success {
            ":green_circle:"
        } else {
            ":red_circle:"
        }
    }
    | each {|it|
        $"- ($it.emoji) `toolkit ($it.stage)`"
    }
    | to text
}

export def main [] { help toolkit }
