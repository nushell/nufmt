def set-tab-color [idx: int] { ansi -e ( ["2;15;", ($idx | into string), (",|")] | str join ) }

def main [name: string] {
    if $name == "x" { return (ls | get name | each {|it| $it | path expand} | prepend "y") }
    []
}

def shapes [a] {
    if $a { ansi -e ["2;15;", ($a | into string), (",|")] }
    if $a { ^foo ["2;15;", ($a | into string), (",|")] }
    if $a { ^foo [1 2 3 4 5] }
    if $a { $a + (ls | get name | each {|it| $it} | prepend "y") }
    if $a { not (ls | get name | each {|it| $it} | is-empty | all {|x| $x}) }
    if $a { print ($in | get name | each {|it| $it} | prepend "y") }
    do { print ($a | into string | str trim | str upcase | str length) }
    let plain = { apply-style ($a | into string | str trim | str upcase | str length) dim }
    let keep = {|x| apply-style ($x | into string | str trim | str upcase | str length) dim }
}

def records [a] {
    do { { name: ($a | into string | str trim | str upcase | str length), size: 1 } }
    if $a { { name: ($a | get name | each {|it| $it} | prepend "y" | str join), size: 1 } }
    do { error make { msg: ($a | into string | str trim | str upcase | str length) } }
    if $a { { name: "x", size: 1 } }
}

def matches [a] {
    do { match $a { 1 => ($a | into string | str trim | str upcase | str length), _ => "other" } }
    if $a { match $a { 1 => "one", _ => ($a | get name | each {|it| $it} | prepend "y") } }
    try { match $a { 1 => { print ($a | into string | str trim | str upcase | str length) }, _ => null } }
    if $a { match $a { 1 => "one", _ => "other" } }
}
