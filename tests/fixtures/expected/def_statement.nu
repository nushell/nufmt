def foo [] { 1 }
def bar [x] { $x }
def add [a: int, b: int] { $a + $b }
def greet [name: string] { $"Hello ($name)!" }
def with_default [x: int = 10] { $x * 2 }
def with_flag [--verbose (-v)] {
    if $verbose { print "verbose" }
}
def complex [
    a: int
    b: string
    --flag (-f)
    --value (-v): int = 5
] { print $"($a) ($b)" }
