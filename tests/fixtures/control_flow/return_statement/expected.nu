def returns_nothing [] { return }
def returns_one [] { return 1 }
def returns_42 [] { return 42 }
def returns_var [x] { return $x }
def returns_expr [x, y] { return ($x + $y) }
def returns_string [] { return "hello" }
def returns_list [] { return [1, 2, 3] }
def returns_record [] { return {a: 1, b: 2} }
def foo [] { return 1 }
def bar [x] {
    if $x > 0 { return "positive" }
    return "not positive"
}
def early [x] {
    if $x == null { return }
    $x
}
