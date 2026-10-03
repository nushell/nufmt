def my-ls [
    path: string@nu-complete-ls
] {
    print $path
}

def my-flags [
    --name: string@nu-complete-names
] { }

def my-list [
    name: string@[ "alice" "bob" "eve" ]
] { }
