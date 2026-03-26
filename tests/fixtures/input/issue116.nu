def foo []: bool -> bool {
    let yesno: bool = ($in | str trim) == "yes"
    $yesno
}
