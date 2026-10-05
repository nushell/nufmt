if true {
    foo
} else {
    bar | baz | qux
}
try {
    foo
} catch {
    bar | baz
}
try {
    foo
} catch {|e|
    bar | baz
}
try {
    foo
} catch {
    bar
} finally {
    baz | qux
}
if a {
    x
} else if b {
    y | z
} else {
    w
}
let v = if true {
    foo
} else {
    bar | baz
}
if ($xs | any {|x| $x > 1 }) {
    a
} else {
    b | c
}
if true { foo } else { bar }
if true { } else {
    bar | baz
}
if true {
    foo
} else { bar }
