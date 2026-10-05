let a = "hello "
let b = $"($a)(curl "https://jsonplaceholder.typicode.com/users" -q | from json | get 0.name)"
print $"("a:")"
print $"(http get "https://google.com")"
print $"(ansi "#ff0000")red" # comment after nested quotes
print $'(echo "x: y")' # comment after single-quoted interpolation
# example: if(true){1}else{2}
echo `it's here` # comment after backtick string
let r = {a: $"("b:")" c: 1}
