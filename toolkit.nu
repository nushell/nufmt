#!/usr/bin/env nu

use std/log

const repo_dir = path self .

# Development tools of nufmt. Run `use toolkit.nu` in Nushell to load commands.
#
# Subcommands:
#   toolkit tests
#   toolkit check-msrv
#   toolkit bump-nu <version>
export def main [] {
    show-help
}

# Define this after main so Nushell can resolve its built-in --help flag.
def show-help [] {
    main --help
}

# Run rustfmt, lint, documentation, test, and benchmark checks.
export def tests [] {
    cd $repo_dir

    cargo fmt --check

    cargo clippy --all-targets -- -D warnings

    let rustdoc_flags = $"($env.RUSTDOCFLAGS? | default "") -D warnings"
    RUSTDOCFLAGS=$rustdoc_flags cargo doc --no-deps

    if (which ^cargo-nextest | is-not-empty) {
        cargo nextest run
        cargo test --doc
    } else {
        cargo test
    }

    cargo test --bench file-format-bench
}

# Check the workspace with the minimum supported Rust version.
export def check-msrv [] {
    cd $repo_dir

    let version = (open Cargo.toml | get package.rust-version)
    rustup run --install $version cargo check --locked --workspace --all-targets --all-features
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
