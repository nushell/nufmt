#!/usr/bin/env nu

use std/log

const repo_dir = path self .

# Development tools of nufmt.
#
# Run `use toolkit.nu` in Nushell, then run `toolkit` to list commands.
export def main [] {
    let commands = (scope commands
        | where name starts-with 'toolkit '
        | select name description)

    if ($commands | is-empty) {
        return 'Run `use toolkit.nu` in Nushell, then run `toolkit` to list commands.'
    }

    $"Usage: toolkit <command>\n\n($commands | table)"
}

# Run rustfmt, lint, documentation, test, and benchmark checks.
export def ci [] {
    cd $repo_dir

    cargo fmt --check
    clippy
    rustdoc
    test
    msrv
}

# Run Clippy on all targets and deny warnings.
export def clippy [] {
    cd $repo_dir

    cargo clippy --all-targets -- -D warnings
}

# Build Rust documentation, including private items, and deny warnings.
export def rustdoc [] {
    cd $repo_dir

    let rustdoc_env = if "CARGO_ENCODED_RUSTDOCFLAGS" in $env {
        let separator = (char --unicode 001f)
        let flags = ([$env.CARGO_ENCODED_RUSTDOCFLAGS '-D' warnings]
            | where {|flag| $flag != "" }
            | str join $separator)
        {CARGO_ENCODED_RUSTDOCFLAGS: $flags}
    } else {
        {RUSTDOCFLAGS: $"($env.RUSTDOCFLAGS? | default "") -D warnings"}
    }
    with-env $rustdoc_env { cargo doc --no-deps --document-private-items }
}

# Run Rust tests, including doctests and benchmark checks.
export def test [] {
    cd $repo_dir

    if (which ^cargo-nextest | is-not-empty) {
        cargo nextest run --all-targets
    } else {
        cargo test --all-targets
    }
    cargo test --doc
}

# Check the workspace with the minimum supported Rust version.
export def msrv [] {
    cd $repo_dir

    let version = open Cargo.toml | get package.rust-version
    with-env {CARGO_BUILD_WARNINGS: deny} {
        rustup run --install $version cargo check --locked --workspace --all-targets --all-features
    }
}

# Bump Nushell badge in README.md and Nushell dependencies in Cargo.toml.
export def bump-nu [version: string] {
    cd $repo_dir

    log info "Updating Nushell badge in README.md."
    let badge_pattern = 'https://img.shields.io/badge/nushell-v\d+\.\d+\.\d+-green'
    let badge_replacement = $"https://img.shields.io/badge/nushell-v($version)-green"
    open --raw README.md
    | str replace --regex $badge_pattern $badge_replacement
    | save --force README.md

    for dependency in [nu-cmd-lang nu-parser nu-protocol nu-utils nuon] {
        cargo add $"($dependency)@($version)"
    }
}
