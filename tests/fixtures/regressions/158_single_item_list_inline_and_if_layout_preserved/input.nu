let a = [
  $"SESSION=($session)"
]
let session_args = if ($session | is-not-empty) { [
  $"SESSION=($session)"
] } else { [] }
