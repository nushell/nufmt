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
[nushell-badge]: https://img.shields.io/badge/nushell-v0.103.0-green
[nushell-url]: https://crates.io/crates/nu

</div>

## Table of contents

- [Status](#status)
- [Usage](#usage)
  - [Files](#files)
  - [Options](#options)
- [Contributing](#contributing)

## Status

> [!IMPORTANT]
> `nufmt` is in a pre-alpha state, not suitable for usage at all.
> Do not use in productive nushell scripts!

Some of the outputs deletes comments, break the functionality of the script or doesn't format at all.

To use the formatter, test it first and use it with caution!.

## Installation

- `cargo install --git https://github.com/nushell/nufmt`
- Using our bundled Nix flake.

## Usage

If you still want to use it, or test it to contribute, this is the `--help`.

```text
nufmt [OPTIONS] [FILES] ...
```

### Files

`Files` are a list of files. It cannot be used combined with `--stdin`.
You can format many files with one command!. For example:

```text
nufmt my-file1.nu my-file2.nu my-file3.nu
```

### Options

- `--check` if you just want to check files without formatting them. It cannot be combined with stdin.
- `-s` or `--stdin` formats from `stdin`, returns to `stdout` as a String. It cannot be used combined with `files`.
- `-c` or `--config` pass the config file path.
  Sample:

  ```text
  nufmt <files> --config my-config.json
  ```

  or

  ```text
  nufmt --stdin <string> --config my-stdin-config.json
  ```
- `-h` or `--help` show help and exit
- `-v` or `--version` prints the version and exit

## Contributing

We have a [contribution guide](docs/CONTRIBUTING.md). If you still have doubts, you can mention @`AucaCoyan` who is active on this repo.
