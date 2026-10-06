" x " | %str trim
%echo ok
def f [arg: external_arg, --flag: list<external_arg>] { }
ls | each {|x|   $x.name} | where {|x| $x != ""}
foo --params {   key: 1 } ...{  b: 2 }
do {|x| $x } # comment after the closure
const LOG = {
    "A": (ansi red)
    "B": (ansi blue)
}
