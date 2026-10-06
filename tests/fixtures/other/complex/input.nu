# Complex Nushell constructs for formatter testing
# Module definition
module math {
    export def add [a: int, b: int] { $a + $b }
    export def multiply [a: int, b: int] { $a * $b }
}
# Using modules
use math
# Custom commands with type annotations
def process-data [data: list<record<name: string, value: int>>, --verbose(-v), --output(-o): string = result.txt] {
    if $verbose { print "Processing data..." }
    $data
}
# Nested data structures
let complex_data = {
    users: [
        {
            name: "Alice"
            age: 30
            scores: [95, 87, 92]
        }
        {
            name: "Bob"
            age: 25
            scores: [88, 91, 85]
        }
    ]
    metadata: {version: "1.0", created: 2024-01-01}
}
# Match expressions
def classify [x: int] { match $x {
    0 => "zero"
    1..10 => "small"
    _ => "large"
} }
# Closures with multiple parameters
let transform = {|x, y, z| ($x + $y) * $z }
# Pipelines with closures
let processed = [1, 2, 3, 4, 5] | each {|n| $n * 2 } | where {|n| $n > 4 }
# Error handling with catch
def safe_divide [a: int, b: int] {
    try {
        if $b == 0 { error make {msg: "Division by zero"} }
        $a / $b
    } catch {
        print $"Error: ($err.msg)"
        null
    }
}
# Conditional with else if
def grade [score: int] {
    if $score >= 90 { "A" } else if $score >= 80 { "B" } else if $score >= 70 { "C" } else { "F" }
}
# Nested loops
for i in 1..3 {
    for j in 1..3 { print $"($i), ($j)" }
}
# Table with complex data
let report = [[name, department, salary]; ["Alice", "Engineering", 100000], ["Bob", "Marketing", 80000], ["Carol", "Engineering", 95000]]
# String with multiple interpolations
let message = $"User ($complex_data.users.0.name) has scores: ($complex_data.users.0.scores | str join ', ')"
# Range operations
let numbers = 1..100 | where {|n| $n mod 2 == 0 } | take 10
# Record spread
let base_config = {host: "localhost", port: 8080}
let full_config = {...$base_config, debug: true, timeout: 30}
# List spread
let list1 = [1, 2, 3]
let list2 = [0, ...$list1, 4, 5]
# Multiline string (if supported)
let long_text = "This is a
multiline
string"
# Binary operations with parentheses
let result = (1 + 2) * (3 + 4) / 2
# Comparison chains
let in_range = $result > 0 and $result < 100
# Null coalescing (if supported)
let value = $env.? | default "fallback"
# Return from function
def early_return [x: int] {
    if $x < 0 { return "negative" }
    if $x == 0 { return "zero" }
    "positive"
}
# Final comment
# End of complex test file
