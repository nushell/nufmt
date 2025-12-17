module mymod{export def foo []{1}}
module   mymod   {   export def foo [] { 1 }   }
module math {
export def add [a, b] { $a + $b }
export def sub [a, b] { $a - $b }
}
module utils {
export def greet [name] {
$"Hello ($name)"
}
export def farewell [name] {
$"Goodbye ($name)"
}
}
