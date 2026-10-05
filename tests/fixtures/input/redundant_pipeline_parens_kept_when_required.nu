let i_key = $key | bits xor ((1..$block_size) | each {0x[36]} | bytes collect)
let s = $delta / (1.0 - ((2.0 * $l - 1.0) | math abs))
let d = (((unix-days $ctx) | math floor | into int) + 719163)
if $x == ((pwd) | str length) { print same }
f --dir=((pwd) | path join "x")
let p = ((pwd) | path join "x")
