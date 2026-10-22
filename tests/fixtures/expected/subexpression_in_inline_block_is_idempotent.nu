def set-tab-color [idx: int] {
    ansi -e ([
        "2;15;"
        ($idx | into string)
        (",|")
    ] | str join)
}

def main [name: string] {
    if $name == "x" {
        return (
            ls
            | get name
            | each {|it| $it | path expand}
            | prepend "y"
        )
    }
    []
}

def shapes [a] {
    if $a {
        ansi -e [
            "2;15;"
            ($a | into string)
            (",|")
        ]
    }
    if $a {
        ^foo [
            "2;15;"
            ($a | into string)
            (",|")
        ]
    }
    if $a { ^foo [1 2 3 4 5] }
    if $a {
        $a + (
            ls
            | get name
            | each {|it| $it}
            | prepend "y"
        )
    }
    if $a {
        not (
            ls
            | get name
            | each {|it| $it}
            | is-empty
            | all {|x| $x}
        )
    }
    if $a {
        print (
            $in
            | get name
            | each {|it| $it}
            | prepend "y"
        )
    }
    do {
        print (
            $a
            | into string
            | str trim
            | str upcase
            | str length
        )
    }
    let plain = {
        apply-style (
            $a
            | into string
            | str trim
            | str upcase
            | str length
        ) dim
    }
    let keep = {|x| apply-style (
        $x
        | into string
        | str trim
        | str upcase
        | str length
    ) dim }
}
