# Nushell Format Specification

> rust has rsfmt

> go has it too

> formatting code is sweet

> and nushell needs one too

These are some guidelines to format your [nushell scripts](https://github.com/nushell/nushell). It is not a must for the code to compile, but are well know practices in almost all programming languages.

This document also serves as a starting point to specify how `nufmt` should work when formatting nushell scripts. It's meant to be very basic at this point, gradually covering more of the language features.

# Code layout:

### Use 4 spaces to indent your code:

```nu
def one-function [] {
    let number = 1
}
```

### Limit the characters per line

There should be a reasonable limit for characters. 100 characters is a common ground between too much and too little

```
# transform this
let resolve_this_really_long_line = ({|row| if $row.Type == DGRAM { $row | update Path { get I-Node } | update I-Node { get State } | update State "" } { $row } })

# to this
let resolve_this_really_long_line = (
        { |row| if $row.Type == DGRAM {
            $row | update Path { get I-Node } | update I-Node { get State } | update State "" }
            { $row }
        }
    )
```

### leave an empty line before writing a function

```nu
def first-function [] {
    # do something
}
# leave an empty line between fn, like between second and third
def second-function [] {
    # do something
}

def third-function [] {
    # do something
}
```

### Indent every `if`, `for` or `while` you have nested

```
while  {
    for number in [1,2,3,4] {
        if $number == 1 {
            echo 'found 1!'
        } else {
            echo 'scanning'
        }

    }
}
```

# If you opt-in `nufmt`, these are the supported features

### Indentation

There should be an `--indent` parameter to allow one to specify the number of spaces to use for indentation.

## Supported Commands

This is the list of the supported commands and their idiomatic formatting. Indentation will be covered by the `--indent` flag but for these examples, 4 spaces will be used.

## Contribute

We talk about this thing in [the Discord server](https://discord.gg/NtAbbGn) go to `#general` and `nufmt` thread.
