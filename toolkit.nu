# this module regroups a bunch of development tools to make the development
# process easier for anyone.
#
# the main purpose of `toolkit` is to offer an easy to use interface for the
# developer during a PR cycle, namely to (**1**) format the source base,
# (**2**) catch classical flaws in the new changes with *clippy* and (**3**)
# make sure all the tests pass.
# print the pipe input inside backticks, dimmed and italic, as a pretty command
def pretty-print-command [] { ((($"`(ansi default_dimmed)(ansi default_italic)($in)(ansi reset)`"))) }
# check standard code formatting and apply the changes
export def fmt [--check, --verbose] {
    # do not apply the format changes, only check the syntax
    # print extra information about the command's progress
    # do not apply the format changes, only check the syntax
    # print extra information about the command's progress
    if $verbose { print $"running ('toolkit fmt' | pretty-print-command)" }
    if $check {
        try { cargo fmt --all -- --check } catch { error make --unspanned {
            msg: $"\nplease run ('toolkit fmt' | pretty-print-command) to fix formatting!"
        } }
    } else { cargo fmt --all }
}
# check that you're using the standard code style
#
# > it is important to make `clippy` happy :relieved:
export def clippy [--verbose] {
    # print extra information about the command's progress
    # print extra information about the command's progress
    if $verbose { print $"running ('toolkit clippy' | pretty-print-command)" }
    try { (cargo clippy --all-targets --no-deps --workspace -- -D warnings -D rustdoc::broken_intra_doc_links -W clippy::explicit_iter_loop -W clippy::explicit_into_iter_loop -W clippy::semicolon_if_nothing_returned -W clippy::doc_markdown -W clippy::manual_let_else) } catch { error make --unspanned {
        msg: $"\nplease fix the above ('clippy' | pretty-print-command) errors before continuing!"
    } }
}
# check that all the tests pass
export def test [--fast] {
    # use the "nextext" `cargo` subcommand to speed up the tests (see [`cargo-nextest`](https://nexte.st/) and [`nextest-rs/nextest`](https://github.com/nextest-rs/nextest))
    # use the "nextext" `cargo` subcommand to speed up the tests (see [`cargo-nextest`](https://nexte.st/) and [`nextest-rs/nextest`](https://github.com/nextest-rs/nextest))
    if $fast { cargo nextest run --all } else { cargo test --workspace }
}
export def main [] { help toolkit }
