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