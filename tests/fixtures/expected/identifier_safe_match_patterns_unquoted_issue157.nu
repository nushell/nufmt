let summary = match $decision {
    allow => "ok"
    deny  => "blocked"
    ask   => "queued"
    _     => $decision
}
