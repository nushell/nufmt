# Changelog

All notable changes to nufmt are listed here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Fixed

- A block (or a closure without parameters) that stays on one line formats the same on every
  pass. The formatter now renders the statement to decide, instead of guessing from its shape, so
  a parenthesised pipeline or a list that is laid out across lines no longer leaves `{ ... }`
  inline on the first pass and expands it on the second. This covers unknown commands
  (`ansi -e [...]`), operators, `not`, and `$in`, for example
  `{ return (ls | get name | each {|it| $it} | prepend "x") }`.

## [0.1.6] - 2026-10-21

### Added

- Prebuilt binaries and shell/PowerShell installers for 7 targets, built by `dist` when a
  version tag is pushed ([#247](https://github.com/nushell/nufmt/pull/247)).
- This changelog.

### Changed

- Rust edition 2024, so building from source needs Rust 1.85 or newer. Dropped the deprecated
  `authors` field from `Cargo.toml` ([#238](https://github.com/nushell/nufmt/pull/238)).
- `toolkit bump-nu <version>` replaces `scripts/update-nushell-version` and now bumps `nu-utils`
  too. `toolkit clippy` also checks docs ([#246](https://github.com/nushell/nufmt/pull/246)).
- README nushell badge shows 0.116.1 ([#239](https://github.com/nushell/nufmt/pull/239)).

### Fixed

- Inline records longer than `line_length` now wrap to multiline, like lists and tables
  already did ([#243](https://github.com/nushell/nufmt/pull/243), fixes #241).
- A call with a multi-line record argument formats the same on every pass. The wrap decision
  now measures the formatted call, not its source
  ([#244](https://github.com/nushell/nufmt/pull/244), fixes #242).
- A one-line `if` block holding a list with a subexpression, such as `return [(...)]`, formats
  the same on every pass ([#248](https://github.com/nushell/nufmt/pull/248), fixes #245).

## [0.1.5] - 2026-10-05

### Added

- `--shebang`/`-s` also formats files without a `.nu` extension when their first line is a
  nu shebang ([#222](https://github.com/nushell/nufmt/pull/222), fixes #73).
- `consistent_branches` option in `nufmt.nuon` (`"never"`, `"single_line"`, `"always"`) makes
  the branches of an `if`/`else` or `try`/`catch` chain share one layout. The default,
  `"single_line"`, changes existing output: a chain written on one line now expands every
  branch when any branch spans several lines ([#237](https://github.com/nushell/nufmt/pull/237),
  fixes #217).

### Changed

- Nushell crates 0.112.2 -> 0.116.1 ([#237](https://github.com/nushell/nufmt/pull/237)).
- CI runs tests on Linux, macOS and Windows, and the Nix flake moved to nixos-26.05
  ([#216](https://github.com/nushell/nufmt/pull/216)). The Nix dev shell includes clippy
  ([#209](https://github.com/nushell/nufmt/pull/209)) and the flake has package metadata
  ([#205](https://github.com/nushell/nufmt/pull/205)).

### Fixed

- A file holding only comments was emptied (#231), and a comment just before a block's
  closing `}` was dropped or moved into the `else` block (#232)
  ([#237](https://github.com/nushell/nufmt/pull/237)).
- `export-env` blocks were mangled (#233), and a default after `: in -> out` was read from the
  wrong token, stripping the number from a `duration` default (#236)
  ([#237](https://github.com/nushell/nufmt/pull/237)).
- Inner quotes in interpolations like `$"(f "x:")"` were treated as string ends, and `#` inside
  a word like `nixpkgs#hello` started a comment ([#237](https://github.com/nushell/nufmt/pull/237),
  fixes #220).
- Completion directives on custom command parameters were removed
  ([#235](https://github.com/nushell/nufmt/pull/235), fixes #230).
- Default values of named flags were removed when they did not resolve
  ([#229](https://github.com/nushell/nufmt/pull/229), fixes #228).
- Signatures without commas attached defaults and rest parameters to the wrong parameter
  (#223), quotes were removed from cell path keys like `$env."KEY(with-parens)"` (#224), valid
  multiline records got commas added (#225), a blank line after a comment split a pipeline
  (#213), and parentheses around `$in` pipelines in `match` were removed (#226)
  ([#227](https://github.com/nushell/nufmt/pull/227)).
- End-of-line comments inside a multiline pipeline were dropped
  ([#210](https://github.com/nushell/nufmt/pull/210)).
- Default parameter values were rewritten (#204), multiline tables were misformatted (#198),
  comments in a list of closures were misformatted (#199), parentheses around commands in
  records were removed (#200), and `alias c = %clear` lost the alias (#211)
  ([#212](https://github.com/nushell/nufmt/pull/212)).
- `0..1..<3` became `0..<1..<3`, which is invalid
  ([#208](https://github.com/nushell/nufmt/pull/208), fixes #207).
- Comments were lost or moved in `match` arms with a parenthesized guard, in short nested
  records, and after `r#'...'#` raw strings ([#203](https://github.com/nushell/nufmt/pull/203)).
- The missing-comma repair inserted a comma into a `$"..."` interpolation that followed a `'"'`
  literal ([#202](https://github.com/nushell/nufmt/pull/202), fixes #201).

## [0.1.4] - 2026-05-24

### Added

- `indent_char` option in `nufmt.nuon`: `"space"` (default) or `"tab"`
  ([#197](https://github.com/nushell/nufmt/pull/197), fixes #196).

### Fixed

- Side comments in multiline lists were dropped. Lists with comments now stay multiline
  ([#197](https://github.com/nushell/nufmt/pull/197), fixes #195).

## [0.1.3] - 2026-05-18

### Changed

- Nushell crates 0.110.0 -> 0.112.2 ([#193](https://github.com/nushell/nufmt/pull/193)).

## [0.1.2] - 2026-05-18

### Fixed

- `list<closure>` became `list<closure()>` (#187), extra spaces in multi-word commands like
  `error  make` were kept (#188), `match` arms needed several passes to settle (#189), and CLI
  tests failed with `RUST_LOG=error` set (#190) ([#192](https://github.com/nushell/nufmt/pull/192)).
- `export module` blocks lost their braces ([#186](https://github.com/nushell/nufmt/pull/186),
  fixes #185).
- The summary said "were formatted" for a single file
  ([#184](https://github.com/nushell/nufmt/pull/184)).

## [0.1.1] - 2026-05-05

First version bump since 0.1.0. It also ships the formatter rework from early 2026, which is
only recorded in the git history. The entries below are from the release PR
([#183](https://github.com/nushell/nufmt/pull/183)).

### Changed

- Running `nufmt` with no paths no longer formats the current directory recursively. Use
  `--all` for the current directory or `--all-recurse` to include subdirectories (#182).
- Bumped `clap`, `env_logger`, `rayon`, `criterion` and `tempfile`.

### Fixed

- A blank line was inserted at the top of a `try` block before a comment (#178), quotes were
  lost in parameter defaults (#179), part of an alias body was appended to calls of that alias
  inside a `def` (#180), and `e>|` gained an extra `|` (#181).
