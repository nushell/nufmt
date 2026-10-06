<div align="center">

# `nufmt`: the nushell formatter

[![MIT licensed][mit-badge]][mit-url]
[![Discord chat][discord-badge]][discord-url]
[![CI on main][ci-badge]][ci-url]
[![nushell version][nushell-badge]][nushell-url]

[mit-badge]: https://img.shields.io/badge/license-MIT-blue.svg?color=brightgreen
[mit-url]: https://github.com/nushell/nufmt/blob/main/LICENSE
[discord-badge]: https://img.shields.io/discord/678763474494423051?logo=discord&label=discord&color=brightgreen
[discord-url]: https://discord.gg/NtAbbGn
[ci-badge]: https://github.com/nushell/nufmt/actions/workflows/main.yml/badge.svg
[ci-url]: https://github.com/nushell/nufmt/actions/workflows/main.yml
[nushell-badge]: https://img.shields.io/badge/nushell-v0.116.1-green
[nushell-url]: https://crates.io/crates/nu

</div>

## Table of contents

- [Features](#features)
- [Installation](#installation)
- [Usage](#usage)
  - [Files](#files)
  - [Options](#options)
  - [Configuration](#configuration)
- [Supported Constructs](#supported-constructs)
- [Testing](#testing)
  - [Ground Truth Tests](#ground-truth-tests)
  - [Running Tests](#running-tests)
- [Contributing](#contributing)

## Features

`nufmt` is a formatter for Nushell scripts, built entirely on Nushell's own parsing infrastructure (`nu-parser`, `nu-protocol`). It provides:

- **AST-based formatting**: Uses Nushell's actual parser for accurate code understanding
- **Idempotent output**: Running the formatter twice produces the same result
- **Comment preservation**: Comments are preserved in their original positions
- **Configurable**: Supports configuration via NUON files
- **Fast**: Parallel file processing with Rayon

## Installation

### From source

```bash
cargo install --git https://github.com/nushell/nufmt
```

### Using Nix

```bash
nix run github:nushell/nufmt
```

## Usage

```text
nufmt [OPTIONS] [FILES]...
```

### Files

Format one or more Nushell files:

```bash
# Format a single file
nufmt script.nu

# Format multiple files
nufmt file1.nu file2.nu file3.nu

# Format all .nu files in a directory
nufmt src/
```

### Options

| Option | Short | Description |
|--------|-------|-------------|
| `--dry-run` | | Check files without modifying them. Returns exit code 1 if files would be reformatted. |
| `--stdin` | | Read from stdin and write formatted output to stdout. Cannot be combined with file arguments. |
| `--config` | `-c` | Path to a configuration file (NUON format). |
| `--help` | `-h` | Show help and exit. |
| `--version` | `-v` | Print version and exit. |

### Examples

```bash
# Format files in place
nufmt *.nu

# Check if files need formatting (CI mode)
nufmt --dry-run src/

# Format stdin
echo 'let x=1' | nufmt --stdin

# Use custom config
nufmt --config nufmt.nuon src/
```

### Configuration

Create a `nufmt.nuon` file in your project root:

```nuon
{
    indent: 4
    indent_char: "space"
    line_length: 80
    margin: 1
    exclude: ["vendor/**", "target/**"]
    consistent_branches: "single_line"
}
```

Configuration options:

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `indent` | int | 4 | Visual indentation width per level (tab width when `indent_char = "tab"`) |
| `indent_char` | string | `"space"` | Indentation character: `"space"` or `"tab"` |
| `line_length` | int | 80 | Maximum line length (advisory) |
| `margin` | int | 1 | Number of blank lines between top-level items |
| `exclude` | list\<string\> | [] | Glob patterns for files to exclude |
| `consistent_branches` | string | `"single_line"` | When one branch of an `if`/`else` or `try`/`catch` chain spans several lines, put every branch on its own lines: `"single_line"` for chains written on one line, `"always"` for every chain, `"never"` to lay out each branch independently |

### Exit codes

| Code | Description |
|------|-------------|
| 0 | Success (files formatted or already formatted) |
| 1 | Dry-run mode: at least one file would be reformatted |
| 2 | Error: invalid configuration, CLI options, or parse error |

## Supported Constructs

`nufmt` properly formats the following Nushell constructs:

- ✅ Variable declarations (`let`, `mut`, `const`)
- ✅ Function definitions (`def`, `def-env`, `export def`)
- ✅ Control flow (`if`/`else`, `match`, `for`, `while`, `loop`)
- ✅ Pipelines with proper spacing around `|`
- ✅ Lists and records
- ✅ Closures with parameters (`{|x| ... }`)
- ✅ String interpolation (`$"Hello ($name)"`)
- ✅ Modules (`module`, `use`, `export`)
- ✅ Error handling (`try`/`catch`)
- ✅ Comments (preserved in output)
- ✅ Ranges (`1..10`, `1..2..10`)
- ✅ Binary operations with proper spacing

## How It Works

Unlike tree-sitter based formatters, `nufmt` uses Nushell's own `nu-parser` crate to parse scripts into an AST. This ensures:

1. **Accuracy**: The same parser that runs your scripts formats them
2. **Compatibility**: Always in sync with Nushell's syntax
3. **Error detection**: Invalid syntax is detected before formatting

The formatter walks the AST and emits properly formatted code with consistent:
- Indentation (configurable)
- Spacing around operators and keywords
- Brace placement for blocks
- Comment placement

## Testing

The Rust tests and Nushell runner automatically discover fixture directories exactly two levels below `tests/fixtures/`:

```text
tests/fixtures/<category>/<case>/
├── expected.nu       # Required reference output
├── input.nu          # Optional input with formatting issues
└── config.noun       # Optional formatter configuration for this case
```

Each case first formats `expected.nu` **once** and compares the result byte for byte with the original file. A mismatch fails the idempotency check and saves the actual output to `not_idempotent.nu`; a match removes that file if it exists.

If `input.nu` exists, the runner then formats it **once** and compares the result byte for byte with the original `expected.nu`. A mismatch saves the actual output to `unexpected.nu`; a match removes that file if it exists. Cases without an input only run the expected-file check and leave any existing `unexpected.nu` untouched.

Both checks use the case's `config.noun` when present. Comparisons preserve whitespace and line endings, including the final newline. All cases run before failures are reported, and known non-idempotent cases fail normally.

### Running Tests

#### Rust Tests

```bash
# Run all tests and continue to other test binaries after a failure
cargo test --no-fail-fast

# Run fixture checks
cargo test --test ground_truth

# Show detailed output
cargo test --test ground_truth -- --nocapture
```

#### Nushell Test Runner

```bash
# Build the release binary first
cargo build --release

# Run all fixture checks
nu tests/run_ground_truth_tests.nu

# Show failure details
nu tests/run_ground_truth_tests.nu --verbose

# Only check existing input.nu files against expected.nu
nu tests/run_ground_truth_tests.nu --ground-truth

# Only check expected.nu for idempotency
nu tests/run_ground_truth_tests.nu --idempotency

# Filter by category directory
nu tests/run_ground_truth_tests.nu --category control_flow

# Filter by category/case, or by a unique case name
nu tests/run_ground_truth_tests.nu --test core_language_constructs/let_statement
nu tests/run_ground_truth_tests.nu --test let_statement

# Discover available cases and categories without building the binary
nu tests/run_ground_truth_tests.nu --list
nu tests/run_ground_truth_tests.nu --list-categories

# Check required files; input.nu is optional
nu tests/run_ground_truth_tests.nu --check-files
```

The two check-only flags are mutually exclusive. Ambiguous case names must be qualified with their category.

### Adding New Tests

Create `tests/fixtures/<category>/<case>/expected.nu` with the reference output, including its final newline. Add `input.nu` when a distinct input is useful, and `config.noun` when the case needs custom settings. No test registration or code changes are required. Deeper directories are not discovered as additional cases.

## Contributing

Contributions are welcome! Please see our [contribution guide](docs/CONTRIBUTING.md).

### Reporting issues

If you encounter formatting issues, please:

1. Check if the script is valid Nushell syntax
2. Provide a minimal reproduction case
3. Include your `nufmt` version and Nushell version

## License

MIT License - see [LICENSE](LICENSE) for details.
