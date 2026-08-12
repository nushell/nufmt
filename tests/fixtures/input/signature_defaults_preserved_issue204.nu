#!/usr/bin/env nu

let my_var = "/this/is/a/path"
const my_const = "/and/another/path"

def with_string [file: path = "./example.file"] {
    print $file
}

def with_nu_var [file: path = $nu.history-path] {
    print $file
}

def with_const [file: path = $my_const] {
    print $file
}

def with_var [file: path = $my_var] {
    print $file
}

def with_undefined [file: path = $fake_const] {
    print $file
}
