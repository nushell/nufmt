if true {
try {
  do $thing
} catch {|err|
  # TODO: proper logging
  print --stderr $"thing callback crashed ($err)"
}
}
