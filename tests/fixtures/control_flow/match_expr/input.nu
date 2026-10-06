match $x {
    0=>"zero"  =>  1=>"one"
}
match $x   {
    0  =>  "zero"
    _  =>  "other"
}
match $x  {
    0  =>  "zero"
    1   =>   "one"
    _  =>  "other"
}
match $value   {
    "a"=>"alpha"  =>  "b"=>"beta"
}
match $num  {
    0  =>  "zero"
    1..10   =>   "small"
    _  =>  "large"
}
match $data   {
    {type:  "user"}  =>  "is user"
    {type:   "admin"}   =>   "is admin"
    _  =>  "unknown type"
}
match $list  {
    []  =>  "empty"
    [x]   =>   $"single: ($x)"
    [x,  y]  =>  $"pair: ($x), ($y)"
    _  =>  "many"
}
