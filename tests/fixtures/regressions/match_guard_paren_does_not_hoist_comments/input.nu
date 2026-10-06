def first [] {
    # orphan comment that must stay put
    1
}

# a leading section comment
def z [] {
    match $rest {
        [] => { '~' }
        [$arg] if ($arg | path expand | path type) == 'dir' => { $arg }
        _ => { 0 }
    }
}
