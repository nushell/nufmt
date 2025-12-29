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
[nushell-badge]: https://img.shields.io/badge/nushell-v0.109.1-green
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

### Using Cargo

```bash
cargo install nufmt
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
    line_length: 80
    margin: 1
    exclude: ["vendor/**", "target/**"]
}
```

Configuration options:

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `indent` | int | 4 | Number of spaces per indentation level |
| `line_length` | int | 80 | Maximum line length (advisory) |
| `margin` | int | 1 | Number of blank lines between top-level items |
| `exclude` | list\<string\> | [] | Glob patterns for files to exclude |

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

`nufmt` has a comprehensive ground truth testing system that verifies the formatter produces correct output for all supported Nushell constructs.

### Ground Truth Tests

The testing system uses two sets of fixture files:

- **Input files** (`tests/fixtures/input/`): Contain valid Nushell code with intentional formatting issues (extra spaces, inconsistent indentation, etc.)
- **Expected files** (`tests/fixtures/expected/`): Contain the correctly formatted version of each input file

When tests run, the formatter processes each input file and compares the output against the corresponding expected file. This ensures:

1. The formatter produces consistent, correct output
2. Formatting changes are intentional and reviewed
3. Regressions are caught immediately

**Important**: Input files should always differ from expected files. Input files represent "what not to do" - poorly formatted but valid Nushell code that the formatter should fix.

### Test Categories

Tests are organized by Nushell construct category:

| Category | Constructs |
|----------|------------|
| `core` | `let_statement`, `mut_statement`, `const_statement`, `def_statement` |
| `control_flow` | `if_else`, `for_loop`, `while_loop`, `loop_statement`, `match_expr`, `try_catch`, `break_continue`, `return_statement` |
| `data_structures` | `list`, `record`, `table`, `nested_structures` |
| `pipelines_expressions` | `pipeline`, `multiline_pipeline`, `closure`, `subexpression`, `binary_ops`, `range`, `cell_path`, `spread` |
| `strings_interpolation` | `string_interpolation`, `comment` |
| `types_values` | `value_with_unit`, `datetime`, `nothing`, `glob_pattern` |
| `modules_imports` | `module`, `use_statement`, `export`, `source`, `hide`, `overlay` |
| `commands_definitions` | `alias`, `extern`, `external_call` |
| `special_constructs` | `do_block`, `where_clause`, `error_make` |

### Running Tests

#### Rust Tests

```bash
# Run all tests (unit + ground truth + idempotency)
cargo test

# Run only ground truth tests
cargo test --test ground_truth

# Run with verbose output
cargo test -- --nocapture
```

#### Nushell Test Runner

A Nushell script (`tests/run_ground_truth_tests.nu`) provides a more interactive testing experience:

```bash
# Build the release binary first
cargo build --release

# Run all tests
nu tests/run_ground_truth_tests.nu

# Run with verbose output (show diffs on failure)
nu tests/run_ground_truth_tests.nu --verbose

# Run only ground truth tests (skip idempotency)
nu tests/run_ground_truth_tests.nu --ground-truth

# Run only idempotency tests
nu tests/run_ground_truth_tests.nu --idempotency

# Run tests for a specific category
nu tests/run_ground_truth_tests.nu --category control_flow

# Run a single test
nu tests/run_ground_truth_tests.nu --test let_statement

# List all available tests
nu tests/run_ground_truth_tests.nu --list

# List available categories
nu tests/run_ground_truth_tests.nu --list-categories

# Check which test files exist
nu tests/run_ground_truth_tests.nu --check-files
```

### Adding New Tests

To add a test for a new construct:

1. Create an input file at `tests/fixtures/input/<construct_name>.nu` with poorly-formatted but valid Nushell code
2. Create an expected file at `tests/fixtures/expected/<construct_name>.nu` with the correctly formatted version
3. Add the construct name to the appropriate category in `tests/run_ground_truth_tests.nu`
4. Run the tests to verify

Example input file (`tests/fixtures/input/my_construct.nu`):
```nu
let x  =  1
let y   =   2
```

Example expected file (`tests/fixtures/expected/my_construct.nu`):
```nu
let x = 1
let y = 2
```

## Contributing

Contributions are welcome! Please see our [contribution guide](docs/CONTRIBUTING.md).

### Reporting issues

If you encounter formatting issues, please:

1. Check if the script is valid Nushell syntax
2. Provide a minimal reproduction case
3. Include your `nufmt` version and Nushell version

## License

MIT License - see [LICENSE](LICENSE) for details.
