# Specification

This specification serves as a starting point to document how `nufmt` should work when formatting nushell scripts. It's meant to be very basic at this point, gradually covering more of the language features.

## Supported Features

### Indentation size

There should be an `--indent` parameter to allow one to specify the number of spaces to use for indentation.

### Indentation

There should be a CRLF and indentation after open braces ()`{`). (TBD - Does this apply to other symbols like `[`, `(`, `|`, etc?)

### Limit the characters per line

There should be a `--limit` parameter to limit to the number of characters per line.

### Number of empty lines before/after custom command

There should be a `--lines-before` and `--lines-after` parameter to set the number of empty lines before and after a custom command.

## Config file

As `rustfmt` does it with a `TOML` file, `nufmt` could have a config file, alongside the command line flags above to set options in stone.
We have identified the NUON format as a suitable data format for this project: after it's THE data format for `nushell`!

With the values above, it could look something like:
```nuon
{
    CRLF: false,
    indent: 4,
    limit: 100,
    lines: {
        after: 1,
        before: 1
    }
}
```

## Sensible (?) default and features

- :one: do not always add newlines when it can help understand the central point of a command call
```nushell
http get ({
    scheme: https,
    host: api.github.com,
    path: /users/nushell/repos,
    params: {
        sort: updated,
        per_page: 100
        page: 1
    }
} | url join)
```
to put the emphasis on the url structure
> **Note**  
> the  `({ ... } | url join)`

- :two: put `|` at the start of the lines for readability, creating a "wall of pipes"
```nushell
def "apply to" [
    file: path
    modification: closure
] {
    $file
    | path expand
    | open --raw
    | from toml
    | do $modification
    | save --force $file
}
```

- :three: ternary-like conditions when conditions and the two branches are short
```nushell
let sign = if $value < 0 { -1 } else { 1 }
```
instead of
```nushell
let sign = (
    if $value < 0 {
        -1
    } else {
        1
    }
)
```

- :four: a newline before a block of comments
```nushell
some command

# a comment to explain
another comment

# a block
# of comment
# for this
last command
```

:five: two spaces before and one space after a comment on the same line as a command
```nushell
my-command  # and some explaination
```
or in command arguments as well
```nushell
def foo [
    a: int  # my integer argument
    b: string  # my string integer
] {}
```

- :six: 4 spaces as the default indentation

- :seven: remove trailing whitespaces

- :eight: single quotes for single characters and double quotes for strings => that behaviour might change with string interpolation and paths.
> **Warning**  
> one should use single quotes (') or backticks (\`) to quote paths on Windows

## Supported Commands

This is the list of the supported commands and their idiomatic formatting. Indentation will be covered by the `--indent` flag but for these examples, 2 spaces will be used.

### if

```bash
if condition {
  # some thing
} else if {
  # some other thing
} else {
  # else the last thing
}
```

### for

```bash
for var in 0..100 {
  # do something here
}
```
