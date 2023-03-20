# Specification

This specification serves as a starting point to document how `nufmt` should work when formatting nushell scripts. It's meant to be very basic at this point, gradually covering more of the language features.


## Supported Features

### Indentation

There should be an `--indent` parameter to allow one to specify the number of spaces to use for indentation.

### Supported Commands

This is the list of the supported commands and their idiomatic formatting.

#### if

The if clause should look like this. Indentation will be covered by the `--indent` flag but for these examples, 2 spaces will be used.

```rust
if condition {
  // some thing
} else if {
  // some other thing
} else {
  // else the last thing
}
```