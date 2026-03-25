let result = $items | from json | if ($in | default [] | where value == "ERR" | is-empty) {
    $in
} else {
    null
}
