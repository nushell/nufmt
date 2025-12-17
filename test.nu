# function comment
def fun1 [text: string] { print $"fun1: ($text)" }
# this is a
# multi-line comment
# before a function
def fun2 [text: string] { print $"fun2: ($text)" }
# call the functions
fun1 "hello"
fun2 "world"
