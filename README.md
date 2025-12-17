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

## Contributing

Contributions are welcome! Please see our [contribution guide](docs/CONTRIBUTING.md).

### Running tests

```bash
# Run all tests
cargo test

# Run with verbose output
cargo test -- --nocapture
```

### Reporting issues

If you encounter formatting issues, please:

1. Check if the script is valid Nushell syntax
2. Provide a minimal reproduction case
3. Include your `nufmt` version and Nushell version

## License

MIT License - see [LICENSE](LICENSE) for details.
