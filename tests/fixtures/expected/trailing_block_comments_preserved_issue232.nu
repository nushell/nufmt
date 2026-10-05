export def func [] {
    let tmp = "abc"
    if ($tmp | str ends-with "c") {
        echo $tmp

        # this if block ends in a comment
    }

    # this function ends in a comment
}

# this file ends in a comment

def blank_line_before_closing_brace [] {
    ls # inline
    # trailing

    # trailing 2
}

do {
    ls
    # trailing in do
}

do {|x|
    $x
    # trailing in closure
}

if true {
    ls
    # trailing in then
} else {
    ls
    # trailing in else
}

match $x {
    1 => 2 # inline after arm
    3 => {
        foo
        # trailing in arm block
    }
    # before arm
    4 => 5 # inline after last arm
    # trailing in match
}
