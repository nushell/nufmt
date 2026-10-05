def check [x] {
    if ($x | is-empty) {  # nothing to do
        return 1
    }
    if ($x) { # truthy
        2
    }
}
