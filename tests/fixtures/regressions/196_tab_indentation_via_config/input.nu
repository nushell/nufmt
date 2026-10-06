def greet [name] {
  if $name != "" {
    print $"Hello ($name)"
  } else {
    print "Hello"
  }
}
