# Basic Nushell constructs for formatter testing
# Simple variable declarations
let x = 1
let name = "world"
mut counter = 0
# Arithmetic expressions
let result = $x + 10
let sum = 1 + 2 + 3
# Function definition
def greet [name: string] { print $"Hello, ($name)!" }
# Function with multiple parameters
def add [a: int, b: int] { $a + $b }
# Function with default value
def greet_default [name: string = stranger] { print $"Hello, ($name)!" }
# Lists
let numbers = [1, 2, 3, 4, 5]
let mixed = [1, "two", 3.0]
let empty_list = []
# Records
let person = {name: "Alice", age: 30}
let config = {host: "localhost", port: 8080, debug: true}
let empty_record = {}
# Conditionals
if true { print "yes" } else { print "no" }
# Nested if
if $x > 0 {
    if $x > 10 { print "large" } else { print "small" }
}
# For loop
for item in [1, 2, 3] { print $item }
# While loop
while $counter < 5 { $counter = ($counter + 1) }
# Loop
loop {
    if $counter > 10 { break }
    $counter = ($counter + 1)
}
# Closures
let double = {|x| $x * 2 }
let add_one = {|n: int| $n + 1 }
# String interpolation
let greeting = $"Hello, ($name)!"
# Range
let range = 1..10
let range_step = 1,2..10
# Binary operations
let and_result = true and false
let or_result = true or false
let not_result = not true
# Match expression (if available in lang)
# match $x {
#     1 => { print "one" }
#     _ => { print "other" }
# }
# Error handling
try { error make {msg: "test error"} } catch { print "caught error" }
# Comments at end of file
# End of test file
