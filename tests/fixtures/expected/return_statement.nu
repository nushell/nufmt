return
return 1
return 42
return $x
return $x + $y
return "hello"
return [1, 2, 3]
return {a: 1, b: 2}
def foo [] { return 1 }
def bar [x] {
    if $x > 0 { return "positive" }
    return "not positive"
}
def early [x] {
    if $x == null { return }
    process $x
}
