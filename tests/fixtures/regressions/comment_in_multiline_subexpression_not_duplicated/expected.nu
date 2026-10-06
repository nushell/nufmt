def f [] {
    if (try {
    # only once
    foo
  } catch { true }) {
        bar
    }
}
