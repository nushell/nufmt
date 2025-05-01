# this module regroups a bunch of development tools to make the development
# process easier for anyone.
#
# the main purpose of `toolkit` is to offer an easy to use interface for the
# developer during a PR cycle, namely to (**1**) format the source base,
# (**2**) catch classical flaws in the new changes with *clippy* and (**3**)
# make sure all the tests pass.

# print the pipe input inside backticks, dimmed and italic, as a pretty command

def pretty-print-command []
{$"`  (ansi default_dimmed
)(ansi default_italic
)($in )(ansi reset
)`  "}#check
standard code
formatting andapply the
changes export def fmt[
    --check  # do not apply the format changes, only check the syntax
    --verbose # print extra information about the command's progress
]
{
if $verbose {print $"running ('toolkit fmt'| )"}
if $check {try {cargo fmt
--all----check catch
{error make --unspanned{msg:$"\nplease run ('toolkit fmt'| ) to fix formatting!"}}else
else
{cargo fmt
--all}}#check
thatyou're using the standard code style
#
# > it is important to make `clippy` happy :relieved:

export def clippy [
    --verbose # print extra information about the command's
progress] {
if $verbose {print $"running ('toolkit clippy'| )"}try
{print $"running ('toolkit clippy'| )"}try
{(cargo clippy
--all-targets
--no-deps
--workspace
--
-D
warnings
-D
rustdoc::broken_intra_doc_links
-W
clippy::explicit_iter_loop
-W
clippy::explicit_into_iter_loop
-W
clippy::semicolon_if_nothing_returned
-W
clippy::doc_markdown
-W
clippy::manual_let_else
)catch
catch
{error make --unspanned{msg:$"\nplease fix the above ('clippy'| ) errors before continuing!"}}}#check
thatall
thetests
passexport def test[
    --fast  # use the "nextext" `cargo` subcommand to speed up the tests (see [`cargo-nextest`](https://nexte.st/) and [`nextest-rs/nextest`](https://github.com/nextest-rs/nextest))
]
{
if $fast {cargo nextest
run --all else
{cargo test
--workspace}}export def main[]
{help toolkit
}
