def to-names [] {
    match ($in | describe) {
        "string" => ($in | from json)
        "record" => {
            $in | get name
        }
        _ => ($in | get name)
    }
}
