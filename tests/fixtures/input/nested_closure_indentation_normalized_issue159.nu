try {
  ^journalctl ...$all_args
    | lines
    | reduce --fold (make-state) {|line, state|
        let entry: record = try { $line | from json } | default {}
    }
}
