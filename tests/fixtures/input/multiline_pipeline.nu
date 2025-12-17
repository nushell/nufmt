ls
    | where type == "file"
    | get name
ls | get name | first
[1, 2, 3]
    | each {|x| $x * 2 }
    | filter {|x| $x > 2 }
$data
    | group-by category
    | transpose key value
    | each {|row| {name: $row.key, count: ($row.value | length)} }
open file.txt | lines | length
ls | sort-by size | reverse | first 5
$table | select name age | where age > 18
