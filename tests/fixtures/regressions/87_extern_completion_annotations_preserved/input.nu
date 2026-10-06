def "nu-complete cargo packages" [] { [] }
export extern "cargo build" [
    --package(-p): string@"nu-complete cargo packages"
    -h
    --help
]
