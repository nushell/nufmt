{
    before: (
        $in
        | first
        | str trim
        | lines
    )
    after: (
        $in
        | last
        | str trim
        | split row (char --integer 0)
    )
}

let r = {
    a: ($in | first)
}
