def has [cmd: string] {
    which $cmd | is-not-empty
}

if not (has systemctl) {
    print ok
}
