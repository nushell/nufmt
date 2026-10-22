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
