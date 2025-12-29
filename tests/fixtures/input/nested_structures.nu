# Nested structures ground truth
# Nested records
  {
    a: {
        b: {c: 1}
    }
}
   {
    a: {
        b: {c: 1}
    }
}
  {
    outer: {
        middle: {inner: "value"}
    }
}
# Nested lists
  [
    [1, 2]
    [3, 4]
    [5, 6]
]
   [
    [1, 2]
    [3, 4]
    [5, 6]
]
  [
    [1, 2, 3]
    [4, 5, 6]
    [7, 8, 9]
]
# Records containing lists
  {
    names: ["Alice", "Bob"]
    ages: [30, 25]
}
   {
    data: [1, 2, 3]
    labels: ["a", "b", "c"]
}
# Lists containing records
  [
    {name: "Alice"}
    {name: "Bob"}
]
   [
    {id: 1, value: "first"}
    {id: 2, value: "second"}
]
# Deeply nested mixed structures
  {
    users: [
        {
            name: "Alice"
            scores: [95, 87, 92]
            metadata: {active: true}
        }
        {
            name: "Bob"
            scores: [88, 91, 85]
            metadata: {active: false}
        }
    ]
    config: {
        version: "1.0"
        settings: {debug: true, verbose: false}
    }
}
# Nested closures in data
  let transform = {|data|
    $data | each {|item| {|x| $x * $item } }
}
# Nested control flow
  if $outer {
    if $inner {
        if $deep { "very deep" } else { "deep" }
    }
}
# Nested function definitions
  def outer [] {
    def inner [] { "inner result" }
    inner
}
# Complex pipeline with nested structures
  $data | each {|row| {name: $row.name, values: ($row.items | each {|i| $i * 2 })} } | where {|r| ($r.values | length) > 0 }
