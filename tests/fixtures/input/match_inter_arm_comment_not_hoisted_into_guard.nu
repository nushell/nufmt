match $x {
    a => 1
    # a comment between arms, before a parenthesized guard
    b if ($y | foo) == 'z' => 2
    # before a bare guard
    c if $y == 'w' => 3
    _ => 4
}
