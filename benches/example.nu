alias ll = ls -l


[[status]; [UP] [UP]] | all {|el| $el.status == UP }
[foo bar 2 baz] | all {|| ($in | describe) == 'string' }
[0 2 4 6] | enumerate | all {|i| $i.item == $i.index * 2 }
let cond = {|el| ($el mod 2) == 0 }; [2 4 6 8] | all $cond


ansi green
ansi reset
$'(ansi red_bold)Hello(ansi reset) (ansi green_dimmed)Nu(ansi reset) (ansi purple_italic)World(ansi reset)'
$'(ansi rb)Hello(ansi reset) (ansi gd)Nu(ansi reset) (ansi pi)World(ansi reset)'
$"(ansi -e '3;93;41m')Hello(ansi reset)"  # italic bright yellow on red background
let bold_blue_on_red = {  # `fg`, `bg`, `attr` are the acceptable keys, all other keys are considered invalid and will throw errors.
        fg: '#0000ff'
        bg: '#ff0000'
        attr: b
    }
    $"(ansi -e $bold_blue_on_red)Hello Nu World(ansi reset)"


'Hello, Nushell! This is a gradient.' | ansi gradient --fgstart '0x40c9ff' --fgend '0xe81cff'
'Hello, Nushell! This is a gradient.' | ansi gradient --fgstart '0x40c9ff' --fgend '0xe81cff' --bgstart '0xe81cff' --bgend '0x40c9ff'
'Hello, Nushell! This is a gradient.' | ansi gradient --fgstart '0x40c9ff'
'Hello, Nushell! This is a gradient.' | ansi gradient --fgend '0xe81cff'


'file:///file.txt' | ansi link --text 'Open Me!'
'https://www.nushell.sh/' | ansi link
[[url text]; [https://example.com Text]] | ansi link url


$'(ansi green)(ansi cursor_on)hello' | ansi strip


[[status]; [UP] [DOWN] [UP]] | any {|el| $el.status == DOWN }
[1 2 3 4] | any {|| ($in | describe) == 'string' }
[9 8 7 6] | enumerate | any {|i| $i.item == $i.index * 2 }
let cond = {|e| $e mod 2 == 1 }; [2 4 1 6 8] | any $cond


[0,1,2,3] | append 4
[0,1] | append [2,3,4]
[0,1] | append [2,nu,4,shell]


ast 'hello'
ast 'ls | where name =~ README'
ast 'for x in 1..10 { echo $x '




2 | bits and 2
[4 3 2] | bits and 2


[4 3 2] | bits not
[4 3 2] | bits not -n '2'
[4 3 2] | bits not -s


2 | bits or 6
[8 3 2] | bits or 2


17 | bits rol 2
[5 3 2] | bits rol 2


17 | bits ror 60
[15 33 92] | bits ror 2 -n '1'


2 | bits shl 7
2 | bits shl 7 -n '1'
0x7F | bits shl 1 -s
[5 3 2] | bits shl 2


8 | bits shr 2
[15 35 2] | bits shr 2


2 | bits xor 2
[8 3 2] | bits xor 2


loop { break }




0x[1F FF AA AA] | bytes add 0x[AA]
0x[1F FF AA AA] | bytes add 0x[AA BB] -i 1
0x[FF AA AA] | bytes add 0x[11] -e
0x[FF AA BB] | bytes add 0x[11 22 33] -e -i 1


 0x[33 44 55 10 01 13] | bytes at 3..<4
 0x[33 44 55 10 01 13] | bytes at 3..6
 0x[33 44 55 10 01 13] | bytes at 3..
 0x[33 44 55 10 01 13] | bytes at ..<4
 [[ColA ColB ColC]; [0x[11 12 13] 0x[14 15 16] 0x[17 18 19]]] | bytes at 1.. ColB ColC


bytes build 0x[01 02] 0x[03] 0x[04]


[0x[11] 0x[13 15]] | bytes collect
[0x[11] 0x[33] 0x[44]] | bytes collect 0x[01]


0x[1F FF AA AA] | bytes ends-with 0x[AA]
0x[1F FF AA AA] | bytes ends-with 0x[FF AA AA]
0x[1F FF AA AA] | bytes ends-with 0x[11]


 0x[33 44 55 10 01 13 44 55] | bytes index-of 0x[44 55]
 0x[33 44 55 10 01 13 44 55] | bytes index-of -e 0x[44 55]
 0x[33 44 55 10 01 33 44 33 44] | bytes index-of -a 0x[33 44]
 0x[33 44 55 10 01 33 44 33 44] | bytes index-of -a -e 0x[33 44]
 [[ColA ColB ColC]; [0x[11 12 13] 0x[14 15 16] 0x[17 18 19]]] | bytes index-of 0x[11] ColA ColC


0x[1F FF AA AB] | bytes length
[0x[1F FF AA AB] 0x[1F]] | bytes length


0x[10 AA FF AA FF] | bytes remove 0x[10 AA]
0x[10 AA 10 BB 10] | bytes remove -a 0x[10]
0x[10 AA 10 BB CC AA 10] | bytes remove -e 0x[10]
[[ColA ColB ColC]; [0x[11 12 13] 0x[14 15 16] 0x[17 18 19]]] | bytes remove 0x[11] ColA ColC


0x[10 AA FF AA FF] | bytes replace 0x[10 AA] 0x[FF]
0x[10 AA 10 BB 10] | bytes replace -a 0x[10] 0x[A0]
[[ColA ColB ColC]; [0x[11 12 13] 0x[14 15 16] 0x[17 18 19]]] | bytes replace -a 0x[11] 0x[13] ColA ColC


0x[1F FF AA AA] | bytes reverse
0x[FF AA AA] | bytes reverse


0x[1F FF AA AA] | bytes starts-with 0x[1F FF AA]
0x[1F FF AA AA] | bytes starts-with 0x[1F]
0x[1F FF AA AA] | bytes starts-with 0x[11]


cal
cal --full-year 2012
cal --week-start monday


cd ~
cd d/s/9
cd -


char newline
char --list
(char prompt) + (char newline) + (char hamburger)
char -u 1f378
char -i (0x60 + 1) (0x60 + 2)
char -u 1F468 200D 1F466 200D 1F466


clear


[1 2 3] | collect { |x| $x.1 }


{ acronym:PWD, meaning:'Print Working Directory' } | columns
[[name,age,grade]; [bill,20,a]] | columns
[[name,age,grade]; [bill,20,a]] | columns | first
[[name,age,grade]; [bill,20,a]] | columns | select 1




[["Hello" "World"]; [null 3]] | compact Hello
[["Hello" "World"]; [null 3]] | compact World
[1, null, 2] | compact


^external arg1 | complete
do { ^external arg1 } | complete




config env


config nu


config reset


const x = 10
const x = { a: 10, b: 20 }


for i in 1..10 { if $i == 5 { continue }; print $i }


cp myfile dir_b
cp -r dir_a dir_b
cp -r -v dir_a dir_b
cp *.txt dir_a




"2021-10-22 20:00:12 +01:00" | date format
date now | date format "%Y-%m-%d %H:%M:%S"
date now | date format "%Y-%m-%d %H:%M:%S"
"2021-10-22 20:00:12 +01:00" | date format "%Y-%m-%d"


"2021-10-22 20:00:12 +01:00" | date humanize


date list-timezone | where timezone =~ Shanghai


date now | date format "%Y-%m-%d %H:%M:%S"
(date now) - 2019-05-01
(date now) - 2019-05-01T04:12:05.20+08:00
date now | debug


date to-record
date now | date to-record
'2020-04-12T22:10:57.123+02:00' | date to-record


date to-table
date now | date to-table
2020-04-12T22:10:57.000000789+02:00 | date to-table


date now | date to-timezone '+0500'
date now | date to-timezone local
date now | date to-timezone US/Hawaii
"2020-10-10 10:00:00 +02:00" | date to-timezone "+0500"


'hello' | debug
['hello'] | debug
[[version patch]; ['0.1.0' false] ['0.1.1' true] ['0.2.0' false]] | debug


^cat myfile.q | decode utf-8
0x[00 53 00 6F 00 6D 00 65 00 20 00 44 00 61 00 74 00 61] | decode utf-16be


'U29tZSBEYXRh' | decode base64
'U29tZSBEYXRh' | decode base64 --binary


'0102030A0a0B' | decode hex
'01 02  03 0A 0a 0B' | decode hex


def say-hi [] { echo 'hi' }; say-hi
def say-sth [sth: string] { echo $sth }; say-sth hi


def-env foo [] { let-env BAR = "BAZ" }; foo; $env.BAR


ls -la | default 'nothing' target 
$env | get -i MY_ENV | default 'abc'
[1, 2, null, 4] | default 3


'hello' | describe


'a b c' | detect columns -n
$'c1 c2 c3(char nl)a b c' | detect columns


[[a b]; [1 2] [1 4] [2 6] [2 4]]
    | dfr into-df
    | dfr group-by a
    | dfr agg [
        (dfr col b | dfr min | dfr as "b_min")
        (dfr col b | dfr max | dfr as "b_max")
        (dfr col b | dfr sum | dfr as "b_sum")
     ]
[[a b]; [1 2] [1 4] [2 6] [2 4]]
    | dfr into-lazy
    | dfr group-by a
    | dfr agg [
        (dfr col b | dfr min | dfr as "b_min")
        (dfr col b | dfr max | dfr as "b_max")
        (dfr col b | dfr sum | dfr as "b_sum")
     ]
    | dfr collect





[false false false] | dfr into-df | dfr all-false
let s = ([5 6 2 10] | dfr into-df);
    let res = ($s > 9);
    $res | dfr all-false


[true true true] | dfr into-df | dfr all-true
let s = ([5 6 2 8] | dfr into-df);
    let res = ($s > 9);
    $res | dfr all-true


let a = ([[a b]; [1 2] [3 4]] | dfr into-df);
    $a | dfr append $a
let a = ([[a b]; [1 2] [3 4]] | dfr into-df);
    $a | dfr append $a --col


[1 3 2] | dfr into-df | dfr arg-max


[1 3 2] | dfr into-df | dfr arg-min


[1 2 2 3 3] | dfr into-df | dfr arg-sort
[1 2 2 3 3] | dfr into-df | dfr arg-sort -r


[false true false] | dfr into-df | dfr arg-true


[1 2 2 3 3] | dfr into-df | dfr arg-unique


let df = ([[a b]; [one 1] [two 2] [three 3]] | dfr into-df);
    $df | dfr select (dfr arg-where ((dfr col b) >= 2) | dfr as b_arg)


dfr col a | dfr as new_a | dfr into-nu


["2021-12-30" "2021-12-31"] | dfr into-df | dfr as-datetime "%Y-%m-%d"


["2021-12-30 00:00:00" "2021-12-31 00:00:00"] | dfr into-df | dfr as-datetime "%Y-%m-%d %H:%M:%S"


[[a b]; [6 2] [4 2] [2 2]] | dfr into-df | dfr reverse | dfr cache


dfr col a | dfr into-nu


[[a b]; [1 2] [3 4]] | dfr into-lazy | dfr collect


[[a b]; [1 2] [3 4]] | dfr into-df | dfr columns


let df = ([[a b c]; [one two 1] [three four 2]] | dfr into-df);
    $df | dfr with-column ((dfr concat-str "-" [(dfr col a) (dfr col b) ((dfr col c) * 2)]) | dfr as concat)


let other = ([za xs cd] | dfr into-df);
    [abc abc abc] | dfr into-df | dfr concatenate $other


[abc acb acb] | dfr into-df | dfr contains ab





let s = ([1 1 0 0 3 3 4] | dfr into-df);
    ($s / $s) | dfr count-null


[1 2 3 4 5] | dfr into-df | dfr cumulative sum


[[a b]; [1 2] [3 4]] | dfr into-df | dfr drop a


[[a b]; [1 2] [3 4] [1 2]] | dfr into-df | dfr drop-duplicates


let df = ([[a b]; [1 2] [3 0] [1 2]] | dfr into-df);
    let res = ($df.b / $df.b);
    let a = ($df | dfr with-column $res --name res);
    $a | dfr drop-nulls
let s = ([1 2 0 0 3 4] | dfr into-df);
    ($s / $s) | dfr drop-nulls


[[a b]; [1 2] [3 4]] | dfr into-df | dfr dtypes


[[a b]; [1 2] [3 4]] | dfr into-df | dfr dummies
[1 2 2 3 3] | dfr into-df | dfr dummies





(dfr col a) > 2) | dfr expr-not


[[a b]; [6 2] [4 2] [2 2]] | dfr into-df | dfr fetch 2


[1 2 NaN 3 NaN] | dfr into-df | dfr fill-nan 0
[[a b]; [0.2 1] [0.1 NaN]] | dfr into-df | dfr fill-nan 0


[1 2 2 3 3] | dfr into-df | dfr shift 2 | dfr fill-null 0


[[a b]; [6 2] [4 2] [2 2]] | dfr into-df | dfr filter ((dfr col a) >= 4)


let mask = ([true false] | dfr into-df);
    [[a b]; [1 2] [3 4]] | dfr into-df | dfr filter-with $mask
[[a b]; [1 2] [3 4]] | dfr into-df | dfr filter-with ((dfr col a) > 1)


dfr col a | dfr first


[[a b]; [1 2] [3 4]] | dfr into-df | dfr first
[[a b]; [1 2] [3 4]] | dfr into-df | dfr first 2





[[a b]; [1 2] [3 4]] | dfr into-df | dfr get a


let dt = ('2020-08-04T16:39:18+00:00' | into datetime -z 'UTC');
    let df = ([$dt $dt] | dfr into-df);
    $df | dfr get-day


let dt = ('2020-08-04T16:39:18+00:00' | into datetime -z 'UTC');
    let df = ([$dt $dt] | dfr into-df);
    $df | dfr get-hour


let dt = ('2020-08-04T16:39:18+00:00' | into datetime -z 'UTC');
    let df = ([$dt $dt] | dfr into-df);
    $df | dfr get-minute


let dt = ('2020-08-04T16:39:18+00:00' | into datetime -z 'UTC');
    let df = ([$dt $dt] | dfr into-df);
    $df | dfr get-month


let dt = ('2020-08-04T16:39:18+00:00' | into datetime -z 'UTC');
    let df = ([$dt $dt] | dfr into-df);
    $df | dfr get-nanosecond


let dt = ('2020-08-04T16:39:18+00:00' | into datetime -z 'UTC');
    let df = ([$dt $dt] | dfr into-df);
    $df | dfr get-ordinal


let dt = ('2020-08-04T16:39:18+00:00' | into datetime -z 'UTC');
    let df = ([$dt $dt] | dfr into-df);
    $df | dfr get-second


let dt = ('2020-08-04T16:39:18+00:00' | into datetime -z 'UTC');
    let df = ([$dt $dt] | dfr into-df);
    $df | dfr get-week


let dt = ('2020-08-04T16:39:18+00:00' | into datetime -z 'UTC');
    let df = ([$dt $dt] | dfr into-df);
    $df | dfr get-weekday


let dt = ('2020-08-04T16:39:18+00:00' | into datetime -z 'UTC');
    let df = ([$dt $dt] | dfr into-df);
    $df | dfr get-year


[[a b]; [1 2] [1 4] [2 6] [2 4]]
    | dfr into-df
    | dfr group-by a
    | dfr agg [
        (dfr col b | dfr min | dfr as "b_min")
        (dfr col b | dfr max | dfr as "b_max")
        (dfr col b | dfr sum | dfr as "b_sum")
     ]
[[a b]; [1 2] [1 4] [2 6] [2 4]]
    | dfr into-lazy
    | dfr group-by a
    | dfr agg [
        (dfr col b | dfr min | dfr as "b_min")
        (dfr col b | dfr max | dfr as "b_max")
        (dfr col b | dfr sum | dfr as "b_sum")
     ]
    | dfr collect


[[a b];[1 2] [3 4]] | dfr into-df
[[1 2 a] [3 4 b] [5 6 c]] | dfr into-df
[a b c] | dfr into-df
[true true false] | dfr into-df


[[a b];[1 2] [3 4]] | dfr into-lazy


dfr col a | dfr into-nu


[[a b]; [1 2] [3 4]] | dfr into-df | dfr into-nu
[[a b]; [1 2] [5 6] [3 4]] | dfr into-df | dfr into-nu -t -n 1


[5 6 6 6 8 8 8] | dfr into-df | dfr is-duplicated
[[a, b]; [1 2] [1 2] [3 3] [3 3] [1 1]] | dfr into-df | dfr is-duplicated


let other = ([1 3 6] | dfr into-df);
    [5 6 6 6 8 8 8] | dfr into-df | dfr is-in $other


let df = ([[a b]; [one 1] [two 2] [three 3]] | dfr into-df);
    $df | dfr with-column (dfr col a | dfr is-in [one two] | dfr as a_in)


dfr col a | dfr is-not-null


let s = ([5 6 0 8] | dfr into-df);
    let res = ($s / $s);
    $res | dfr is-not-null


dfr col a | dfr is-null


let s = ([5 6 0 8] | dfr into-df);
    let res = ($s / $s);
    $res | dfr is-null


[5 6 6 6 8 8 8] | dfr into-df | dfr is-unique
[[a, b]; [1 2] [1 2] [3 3] [3 3] [1 1]] | dfr into-df | dfr is-unique


let df_a = ([[a b c];[1 "a" 0] [2 "b" 1] [1 "c" 2] [1 "c" 3]] | dfr into-lazy);
    let df_b = ([["foo" "bar" "ham"];[1 "a" "let"] [2 "c" "var"] [3 "c" "const"]] | dfr into-lazy);
    $df_a | dfr join $df_b a foo | dfr collect
let df_a = ([[a b c];[1 "a" 0] [2 "b" 1] [1 "c" 2] [1 "c" 3]] | dfr into-df);
    let df_b = ([["foo" "bar" "ham"];[1 "a" "let"] [2 "c" "var"] [3 "c" "const"]] | dfr into-lazy);
    $df_a | dfr join $df_b a foo


dfr col a | dfr last


[[a b]; [1 2] [3 4]] | dfr into-df | dfr last 1





dfr lit 2 | dfr into-nu


[Abc aBc abC] | dfr into-df | dfr lowercase


let test = ([[a b];[1 2] [3 4]] | dfr into-df);
    ls


[[a b]; [6 2] [1 4] [4 1]] | dfr into-df | dfr max


[[a b]; [one 2] [one 4] [two 1]]
    | dfr into-df
    | dfr group-by a
    | dfr agg (dfr col b | dfr max)


[[a b]; [one 2] [one 4] [two 1]]
    | dfr into-df
    | dfr group-by a
    | dfr agg (dfr col b | dfr mean)


[[a b]; [6 2] [4 2] [2 2]] | dfr into-df | dfr mean


[[a b]; [6 2] [4 2] [2 2]] | dfr into-df | dfr median


[[a b]; [one 2] [one 4] [two 1]]
    | dfr into-df
    | dfr group-by a
    | dfr agg (dfr col b | dfr median)


[[a b c d]; [x 1 4 a] [y 2 5 b] [z 3 6 c]] | dfr into-df | dfr melt -c [b c] -v [a d]


[[a b]; [one 2] [one 4] [two 1]]
    | dfr into-df
    | dfr group-by a
    | dfr agg (dfr col b | dfr min)


[[a b]; [6 2] [1 4] [4 1]] | dfr into-df | dfr min


dfr col a | dfr n-unique


[1 1 2 2 3 3 4] | dfr into-df | dfr n-unique


[true false true] | dfr into-df | dfr not


dfr open test.csv


dfr when ((dfr col a) > 2) 4 | dfr otherwise 5
dfr when ((dfr col a) > 2) 4 | dfr when ((dfr col a) < 0) 6 | dfr otherwise 0
[[a b]; [6 2] [1 4] [4 1]]
   | dfr into-lazy
   | dfr with-column (
    dfr when ((dfr col a) > 2) 4 | dfr otherwise 5 | dfr as c
     )
   | dfr with-column (
    dfr when ((dfr col a) > 5) 10 | dfr when ((dfr col a) < 2) 6 | dfr otherwise 0 | dfr as d
     )
   | dfr collect


[[a b]; [one 2] [one 4] [two 1]]
    | dfr into-df
    | dfr group-by a
    | dfr agg (dfr col b | dfr quantile 0.5)


[[a b]; [6 2] [1 4] [4 1]] | dfr into-df | dfr quantile 0.5


[[a b]; [1 2] [3 4]] | dfr into-df | dfr query 'select a from df'


[5 6 7 8] | dfr into-df | dfr rename '0' new_name
[[a b]; [1 2] [3 4]] | dfr into-df | dfr rename a a_new
[[a b]; [1 2] [3 4]] | dfr into-df | dfr rename [a b] [a_new b_new]


[abc abc abc] | dfr into-df | dfr replace -p ab -r AB


[abac abac abac] | dfr into-df | dfr replace-all -p a -r A


[[a b]; [6 2] [4 2] [2 2]] | dfr into-df | dfr reverse


[1 2 3 4 5] | dfr into-df | dfr rolling sum 2 | dfr drop-nulls
[1 2 3 4 5] | dfr into-df | dfr rolling max 2 | dfr drop-nulls


[[a b]; [1 2] [3 4]] | dfr into-df | dfr sample -n 1
[[a b]; [1 2] [3 4] [5 6]] | dfr into-df | dfr sample -f 0.5 -e


[[a b]; [6 2] [4 2] [2 2]] | dfr into-df | dfr select a


let s = ([1 2 2 3 3] | dfr into-df | dfr shift 2);
    let mask = ($s | dfr is-null);
    $s | dfr set 0 --mask $mask


let series = ([4 1 5 2 4 3] | dfr into-df);
    let indices = ([0 2] | dfr into-df);
    $series | dfr set-with-idx 6 -i $indices


[[a b]; [1 2] [3 4]] | dfr into-df | dfr shape


[1 2 2 3 3] | dfr into-df | dfr shift 2 | dfr drop-nulls


[[a b]; [1 2] [3 4]] | dfr into-df | dfr slice 0 1


[[a b]; [6 2] [1 4] [4 1]] | dfr into-df | dfr sort-by a
[[a b]; [6 2] [1 1] [1 4] [2 4]] | dfr into-df | dfr sort-by [a b] -r [false true]


[[a b]; [one 2] [one 2] [two 1] [two 1]]
    | dfr into-df
    | dfr group-by a
    | dfr agg (dfr col b | dfr std)


[[a b]; [6 2] [4 2] [2 2]] | dfr into-df | dfr std


[a ab abc] | dfr into-df | dfr str-lengths


[abcded abc321 abc123] | dfr into-df | dfr str-slice 1 -l 2


let dt = ('2020-08-04T16:39:18+00:00' | into datetime -z 'UTC');
    let df = ([$dt $dt] | dfr into-df);
    $df | dfr strftime "%Y/%m/%d"


[[a b]; [one 2] [one 4] [two 1]]
    | dfr into-df
    | dfr group-by a
    | dfr agg (dfr col b | dfr sum)


[[a b]; [6 2] [1 4] [4 1]] | dfr into-df | dfr sum


[[a b]; [1 1] [1 1]] | dfr into-df | dfr summary


let df = ([[a b]; [4 1] [5 2] [4 3]] | dfr into-df);
    let indices = ([0 2] | dfr into-df);
    $df | dfr take $indices
let series = ([4 1 5 2 4 3] | dfr into-df);
    let indices = ([0 2] | dfr into-df);
    $series | dfr take $indices


[[a b]; [1 2] [3 4]] | dfr into-df | dfr to-arrow test.arrow


[[a b]; [1 2] [3 4]] | dfr into-df | dfr to-csv test.csv
[[a b]; [1 2] [3 4]] | dfr into-df | dfr to-csv test.csv -d '|'


[[a b]; [1 2] [3 4]] | dfr into-df | dfr to-parquet test.parquet


[2 2 2 2 2] | dfr into-df | dfr unique
col a | unique


[Abc aBc abC] | dfr into-df | dfr uppercase


[5 5 5 5 6 6] | dfr into-df | dfr value-counts


[[a b]; [6 2] [4 2] [2 2]] | dfr into-df | dfr var


[[a b]; [one 2] [one 2] [two 1] [two 1]]
    | dfr into-df
    | dfr group-by a
    | dfr agg (dfr col b | dfr var)


dfr when ((dfr col a) > 2) 4
dfr when ((dfr col a) > 2) 4 | dfr when ((dfr col a) < 0) 6
[[a b]; [6 2] [1 4] [4 1]]
   | dfr into-lazy
   | dfr with-column (
    dfr when ((dfr col a) > 2) 4 | dfr otherwise 5 | dfr as c
     )
   | dfr with-column (
    dfr when ((dfr col a) > 5) 10 | dfr when ((dfr col a) < 2) 6 | dfr otherwise 0 | dfr as d
     )
   | dfr collect


[[a b]; [1 2] [3 4]]
    | dfr into-df
    | dfr with-column ([5 6] | dfr into-df) --name c
[[a b]; [1 2] [3 4]]
    | dfr into-lazy
    | dfr with-column [
        ((dfr col a) * 2 | dfr as "c")
        ((dfr col a) * 3 | dfr as "d")
      ]
    | dfr collect


do { echo hello }
let text = "I am enclosed"; let hello = {|| echo $text}; do $hello
do -i { thisisnotarealcommand }
do -s { thisisnotarealcommand }
do -p { nu -c 'exit 1' }; echo "I'll still run"
do -c { nu -c 'exit 1' } | myscarycommand
do {|x| 100 + $x } 77
77 | do {|x| 100 + $in }


[0,1,2,3] | drop
[0,1,2,3] | drop 0
[0,1,2,3] | drop 2
[[a, b]; [1, 2] [3, 4]] | drop 1


[[lib, extension]; [nu-lib, rs] [nu-core, rb]] | drop column


[sam,sarah,2,3,4,5] | drop nth 0 1 2
[0,1,2,3,4,5] | drop nth 0 1 2
[0,1,2,3,4,5] | drop nth 0 2 4
[0,1,2,3,4,5] | drop nth 2 0 4
[first second third fourth fifth] | drop nth (1..3)
[0,1,2,3,4,5] | drop nth 1..
[0,1,2,3,4,5] | drop nth 3..


du


[1 2 3] | each {|e| 2 * $e }
{major:2, minor:1, patch:4} | values | each {|| into string }
[1 2 3 2] | each {|e| if $e == 2 { "two" } }
[1 2 3] | enumerate | each {|e| if $e.item == 2 { $"found 2 at ($e.index)!"} }
[1 2 3] | each --keep-empty {|e| if $e == 2 { "found 2!"} }


[1 2 3 2 1] | each while {|e| if $e < 3 { $e * 2 } }
[1 2 stop 3 4] | each while {|e| if $e != 'stop' { $"Output: ($e)" } }
[1 2 3] | enumerate | each while {|e| if $e.item < 2 { $"value ($e.item) at ($e.index)!"} }


echo 1 2 3
echo $in


"負けると知って戦うのが、遥かに美しいのだ" | encode shift-jis
"🎈" | encode -i shift-jis


0x[09 F9 11 02 9D 74 E3 5B D8 41 56 C5 63 56 88 C0] | encode base64
'Some Data' | encode base64
'Some Data' | encode base64 --character-set binhex


0x[09 F9 11 02 9D 74 E3 5B D8 41 56 C5 63 56 88 C0] | encode hex


enter ../dir-foo


[a, b, c] | enumerate 


error make {msg: "my custom error message"}
error make {
        msg: "my custom error message"
        label: {
            text: "my custom label text"  # not mandatory unless $.label exists
            start: 123  # not mandatory unless $.label.end is set
            end: 456  # not mandatory unless $.label.start is set
        }
    }
def foo [x] {
        let span = (metadata $x).span;
        error make {
            msg: "this is fishy"
            label: {
                text: "fish right here"
                start: $span.start
                end: $span.end
            }
        }
    }


[1 2 3 4 5] | every 2
[1 2 3 4 5] | every 2 --skip


exec ps aux
exec nautilus


exit
exit --now


explain {|| ls | sort-by name type -i | get name } | table -e


sys | explore
ls | explore --head false
glob *.md | each {|| open } | explore -i
open file.json | explore -p | to json | save part.json


module utils { export def my-command [] { "hello" } }; use utils my-command; my-command


module spam { export alias ll = ls -l }


module spam { export def foo [] { "foo" } }; use spam foo; foo


module foo { export def-env bar [] { let-env FOO_BAR = "BAZ" } }; use foo bar; bar; $env.FOO_BAR


export extern echo [text: string]


export old-alias ll = ls -l


module spam { export def foo [] { "foo" } }
    module eggs { export use spam foo }
    use eggs foo
    foo
            


export-env { let-env SPAM = 'eggs' }
export-env { let-env SPAM = 'eggs' }; $env.SPAM


extern echo [text: string]


'nushell' | fill -a l -c '─' -w 15
'nushell' | fill -a r -c '─' -w 15
'nushell' | fill -a m -c '─' -w 15
1 | fill --alignment right --character '0' --width 5
1.1 | fill --alignment center --character '0' --width 5
1kib | fill --alignment middle --character '0' --width 10


[1 2] | filter {|x| $x > 1}
[{a: 1} {a: 2}] | filter {|x| $x.a > 1}
let cond = {|x| $x.a > 1}; [{a: 1} {a: 2}] | filter $cond


ls | find toml md sh
'Cargo.toml' | find toml
[1 5 3kb 4 3Mb] | find 5 3kb
[moe larry curly] | find l
[abc bde arc abf] | find --regex "ab"
[aBc bde Arc abf] | find --regex "ab" -i
[[version name]; ['0.1.0' nushell] ['0.1.1' fish] ['0.2.0' zsh]] | find -r "nu"
[[foo bar]; [abc 123] [def 456]] | find 123 | get bar | ansi strip


[1 2 3] | first
[1 2 3] | first 2
0x[01 23 45] | first 2


[[N, u, s, h, e, l, l]] | flatten 
[[N, u, s, h, e, l, l]] | flatten | first
[[origin, people]; [Ecuador, ([[name, meal]; ['Andres', 'arepa']])]] | flatten --all | get meal
[[origin, crate, versions]; [World, ([[name]; ['nu-cli']]), ['0.21', '0.22']]] | flatten versions --all | last | get versions
{ a: b, d: [ 1 2 3 4 ],  e: [ 4 3  ] } | flatten d --all


42 | fmt


for x in [1 2 3] { print ($x * $x) }
for $x in 1..3 { print $x }
for $it in ['bob' 'fred'] --numbered { print $"($it.index) is ($it.item)" }


ls | format '{name}: {size}'
[[col1, col2]; [v1, v2] [v3, v4]] | format '{col2}'


ls | format filesize KB size
du | format filesize B apparent
4Gb | format filesize MB




"ColA,ColB
1,2" | from csv
open data.txt | from csv --noheaders
open data.txt | from csv --separator ';'
open data.txt | from csv --comment '#'
open data.txt | from csv --trim all
open data.txt | from csv --trim headers
open data.txt | from csv --trim fields


'From: test@email.com
Subject: Welcome
To: someone@somewhere.com
Test' | from eml
'From: test@email.com
Subject: Welcome
To: someone@somewhere.com
Test' | from eml -b 1


'BEGIN:VCALENDAR
            END:VCALENDAR' | from ics


'[foo]
a=1
b=2' | from ini


'{ "a": 1 }' | from json
'{ "a": 1, "b": [1, 2] }' | from json


'{ a:1 }' | from nuon
'{ a:1, b: [1, 2] }' | from nuon


open --raw test.ods | from ods
open --raw test.ods | from ods -s [Spreadsheet1]


open --raw file.parquet | from parquet
open file.parquet




'FOO   BAR
1   2' | from ssv
'FOO   BAR
1   2' | from ssv -n


'a = 1' | from toml
'a = 1
b = [1, 2]' | from toml


"ColA	ColB
1	2" | from tsv
$'c1(char tab)c2(char tab)c3(char nl)1(char tab)2(char tab)3' | save tsv-data | open tsv-data | from tsv
$'a1(char tab)b1(char tab)c1(char nl)a2(char tab)b2(char tab)c2' | save tsv-data | open tsv-data | from tsv -n
$'a1(char tab)b1(char tab)c1(char nl)a2(char tab)b2(char tab)c2' | save tsv-data | open tsv-data | from tsv --trim all
$'a1(char tab)b1(char tab)c1(char nl)a2(char tab)b2(char tab)c2' | save tsv-data | open tsv-data | from tsv --trim headers
$'a1(char tab)b1(char tab)c1(char nl)a2(char tab)b2(char tab)c2' | save tsv-data | open tsv-data | from tsv --trim fields


'bread=baguette&cheese=comt%C3%A9&meat=ham&fat=butter' | from url


'BEGIN:VCARD
N:Foo
FN:Bar
EMAIL:foo@bar.com
END:VCARD' | from vcf


open --raw test.xlsx | from xlsx
open --raw test.xlsx | from xlsx -s [Spreadsheet1]


'<?xml version="1.0" encoding="UTF-8"?>
<note>
  <remember>Event</remember>
</note>' | from xml


'a: 1' | from yaml
'[ a: 1, b: [1, 2] ]' | from yaml


'a: 1' | from yaml
'[ a: 1, b: [1, 2] ]' | from yaml


g
mkdir foo bar; enter foo; enter ../bar; g 1
shells; g 2
mkdir foo bar; enter foo; enter ../bar; g -


[0 1 2] | get 1
[{A: A0}] | get A
[{A: A0}] | get 0.A
ls | get name.2
ls | get 2.name
sys | get cpu
$env | get paTH
$env | get -s Path


glob *.rs
glob **/*.{rs,toml} --depth 2
glob "[Cc]*"
glob "{a?c,x?z}"
glob "(?i)c*"
glob "[!cCbMs]*"
glob <a*:3>
glob <[a-d]:1,10>
glob "[A-Z]*" --no-file --no-symlink


[1 2 3 a b c] | grid
[1 2 3 a b c] | wrap name | grid
{name: 'foo', b: 1, c: 2} | grid
[{name: 'A', v: 1} {name: 'B', v: 2} {name: 'C', v: 3}] | grid
[[name patch]; [0.1.0 false] [0.1.1 true] [0.2.0 false]] | grid


[1 2 3 4] | group 2


ls | group-by type
['1' '3' '1' '3' '2' '1' '1'] | group-by








'abcdefghijklmnopqrstuvwxyz' | hash md5
'abcdefghijklmnopqrstuvwxyz' | hash md5 --binary
open ./nu_0_24_1_windows.zip | hash md5


'abcdefghijklmnopqrstuvwxyz' | hash sha256
'abcdefghijklmnopqrstuvwxyz' | hash sha256 --binary
open ./nu_0_24_1_windows.zip | hash sha256


"a b c|1 2 3" | split row "|" | split column " " | headers
"a b c|1 2 3|1 2 3 4" | split row "|" | split column " " | headers


help match
help str lpad
help --find char


help aliases
help aliases my-alias
help aliases --find my-alias




help externs
help externs smth
help externs --find smth


help modules
help modules my-module
help modules --find my-module




alias lll = ls -l; hide lll
def say-hi [] { echo 'Hi!' }; hide say-hi


let-env HZ_ENV_ABC = 1; hide-env HZ_ENV_ABC; 'HZ_ENV_ABC' in (env).name




ls | histogram type
ls | histogram type freq
[1 2 1] | histogram
[1 2 3 1 1 1 2 2 1 1] | histogram --percentage-type relative


history | length
history | last 5
history | wrap cmd | where cmd =~ cargo


history session




http delete https://www.example.com
http delete -u myuser -p mypass https://www.example.com
http delete -H [my-header-key my-header-value] https://www.example.com
http delete -d 'body' https://www.example.com
http delete -t application/json -d { field: value } https://www.example.com


http get https://www.example.com
http get -u myuser -p mypass https://www.example.com
http get -H [my-header-key my-header-value] https://www.example.com


http head https://www.example.com
http head -u myuser -p mypass https://www.example.com
http head -H [my-header-key my-header-value] https://www.example.com


http patch https://www.example.com 'body'
http patch -u myuser -p mypass https://www.example.com 'body'
http patch -H [my-header-key my-header-value] https://www.example.com
http patch -t application/json https://www.example.com { field: value }


http post https://www.example.com 'body'
http post -u myuser -p mypass https://www.example.com 'body'
http post -H [my-header-key my-header-value] https://www.example.com
http post -t application/json https://www.example.com { field: value }


http put https://www.example.com 'body'
http put -u myuser -p mypass https://www.example.com 'body'
http put -H [my-header-key my-header-value] https://www.example.com
http put -t application/json https://www.example.com { field: value }


if 2 < 3 { 'yes!' }
if 5 < 3 { 'yes!' } else { 'no!' }
if 5 < 3 { 'yes!' } else if 4 < 5 { 'no!' } else { 'okay!' }


echo done | ignore




let user_input = (input)


{'name': 'nu', 'stars': 5} | insert alias 'Nushell'
[[project, lang]; ['Nushell', 'Rust']] | insert type 'shell'
[[foo]; [7] [8] [9]] | enumerate | insert bar {|e| $e.item.foo + $e.index } | flatten


ls | inspect | get name | inspect




'This is a string that is exactly 52 characters long.' | into binary
1 | into binary
true | into binary
ls | where name == LICENSE | get size | into binary
ls | where name == LICENSE | get name | path expand | into binary
1.234 | into binary


[[value]; ['false'] ['1'] [0] [1.0] [true]] | into bool value
true | into bool
1 | into bool
0.3 | into bool
'0.0' | into bool
'true' | into bool


'27.02.2021 1:55 pm +0000' | into datetime
'2021-02-27T13:55:40.2246+00:00' | into datetime
'20210227_135540+0000' | into datetime -f '%Y%m%d_%H%M%S%z'
1614434140123456789 | into datetime --offset -5
1614434140 * 1_000_000_000 | into datetime


[[num]; ['5.01']] | into decimal num
'1.345' | into decimal
'-5.9' | into decimal
true | into decimal


[[value]; ['1sec'] ['2min'] ['3hr'] ['4day'] ['5wk']] | into duration value
'7min' | into duration
'7min' | into duration --convert sec
420sec | into duration
420sec | into duration --convert ms
1000000µs | into duration --convert sec
1sec | into duration --convert µs
1sec | into duration --convert us


[[bytes]; ['5'] [3.2] [4] [2kb]] | into filesize bytes
'2' | into filesize
8.3 | into filesize
5 | into filesize
4KB | into filesize


[[num]; ['-5'] [4] [1.5]] | into int num
'2' | into int
5.9 | into int
'5.9' | into int
4KB | into int
[false, true] | into int
1983-04-13T12:09:14.123456789-05:00 | into int
'1101' | into int -r 2
'FF' |  into int -r 16
'0o10132' | into int
'0010132' | into int
'0010132' | into int -r 8


[[value]; [false]] | into record
[1 2 3] | into record
0..2 | into record
-500day | into record
{a: 1, b: 2} | into record
2020-04-12T22:10:57+02:00 | into record


ls | into sqlite my_ls.db
ls | into sqlite my_ls.db -t my_table
[[name]; [-----] [someone] [=====] [somename] ['(((((']] | into sqlite filename.db
[one 2 5.2 six true 100mib 25sec] | into sqlite variety.db


5 | into string -d 3
1.7 | into string -d 0
1.7 | into string -d 1
1.734 | into string -d 2
1.734 | into string -d -2
4.3 | into string
'1234' | into string
true | into string
ls Cargo.toml | get name | into string
1KiB | into string


if is-admin { "iamroot" } else { "iamnotroot" }


'' | is-empty
[] | is-empty
[[meal size]; [arepa small] [taco '']] | is-empty meal size


{ new: york, san: francisco } | items {|key, value| echo $'($key) ($value)' }


[{a: 1 b: 2}] | join [{a: 1 c: 3}] a


open -r test.json | json path '$.store.book[*].author'




keybindings default


keybindings list -m
keybindings list -e -d
keybindings list


keybindings listen


ps | sort-by mem | last | kill $in.pid
kill --force 12345
kill -s 2 12345


[1,2,3] | last 2
[1,2,3] | last


[1 2 3 4 5] | length
[{columnA: A0 columnB: B0}] | length -c


let x = 10
let x = 10 + 100
let x = if false { -1 } else { 1 }


let-env MY_ENV_VAR = 1; $env.MY_ENV_VAR


$"two\nlines" | lines


{NAME: ABE, AGE: UNKNOWN} | load-env; $env.NAME
load-env {NAME: ABE, AGE: UNKNOWN}; $env.NAME


mut x = 0; loop { if $x > 10 { break }; $x = $x + 1 }; $x


ls
ls subdir
ls -f ..
ls *.rs
ls -s | where name !~ bar
ls -a ~ | where type == dir
ls -as ~ | where type == dir and modified < ((date now) - 7day)
['/path/to/directory' '/path/to/file'] | each {|| ls -D $in } | flatten


match 3 { 1..10 => 'yes!' }
match {a: 100} { {a: $my_value} => { $my_value } }
match 3 { 1 => { 'yes!' }, _ => { 'no!' } }
match [1, 2, 3] { [$a, $b, $c] => { $a + $b + $c }, _ => 0 }
{a: {b: 3}} | match $in {{a: { $b }} => ($b + 10) }




[-50 -100.0 25] | math abs


1 | math arccos
-1 | math arccos -d


1 | math arccosh


1 | math arcsin
1 | math arcsin -d


0 | math arcsinh


1 | math arctan
-1 | math arctan -d


1 | math arctanh


[-50 100.0 25] | math avg


[1.5 2.3 -3.1] | math ceil


math pi | math cos
[0 90 180 270 360] | math cos -d


1 | math cosh


math e | math round --precision 3




0 | math exp
1 | math exp


[1.5 2.3 -3.1] | math floor


math e | math ln


100 | math log 10
[16 8 4] | math log 2


[-50 100 25] | math max
[{a: 1 b: 3} {a: 2 b: -1}] | math max


[3 8 9 12 12 15] | math median
[{a: 1 b: 3} {a: 2 b: -1} {a: -3 b: 5}] | math median


[-50 100 25] | math min
[{a: 1 b: 3} {a: 2 b: -1}] | math min


[3 3 9 12 12 15] | math mode
[{a: 1 b: 3} {a: 2 b: -1} {a: 1 b: 5}] | math mode


math pi | math round --precision 2


[2 3 3 4] | math product


[1.5 2.3 -3.1] | math round
[1.555 2.333 -3.111] | math round -p 2


(math pi) / 2 | math sin
[0 90 180 270 360] | math sin -d | math round --precision 4


1 | math sinh


[9 16] | math sqrt


[1 2 3 4 5] | math stddev
[1 2 3 4 5] | math stddev -s


[1 2 3] | math sum
ls | get size | math sum


(math pi) / 4 | math tan
[-45 0 45] | math tan -d


(math pi) * 10 | math tanh


math tau | math round --precision 2


[1 2 3 4 5] | math variance
[1 2 3 4 5] | math variance -s


[a b c] | wrap name | merge ( [1 2 3] | wrap index )
{a: 1, b: 2} | merge {c: 3}
[{columnA: A0 columnB: B0}] | merge [{columnA: 'A0*'}]


let a = 42; metadata $a
ls | metadata


mkdir foo
mkdir -v foo/bar foo2


module spam { export def foo [] { "foo" } }; use spam foo; foo
module foo { export-env { let-env FOO = "BAZ" } }; use foo; $env.FOO
module foo { export def-env bar [] { let-env FOO_BAR = "BAZ" } }; use foo bar; bar; $env.FOO_BAR


[[name value index]; [foo a 1] [bar b 2] [baz c 3]] | move index --before name
[[name value index]; [foo a 1] [bar b 2] [baz c 3]] | move value name --after index
{ name: foo, value: a, index: 1 } | move name --before index


mut x = 10; $x = 12
mut a = {b:{c:1}}; $a.b.c = 2
mut x = 10 + 100
mut x = if false { -1 } else { 1 }


mv before.txt after.txt
mv test.txt my/subdirectory
mv *.txt my/subdirectory


mkdir foo bar; enter foo; enter ../bar; n
n


nu-check script.nu
nu-check --as-module module.nu
nu-check -d script.nu
open foo.nu | nu-check -d script.nu
open module.nu | lines | nu-check -d --as-module module.nu
$'two(char nl)lines' | nu-check 
nu-check -a script.nu
open foo.nu | lines | nu-check -ad


'let x = 3' | nu-highlight


old-alias ll = ls -l
old-alias customs = ($nu.scope.commands | where is_custom | get command)


open myfile.json
open myfile.json --raw
'myfile.txt' | open
open myfile.txt --raw | decode utf-8




module spam { export def foo [] { "foo" } }
    overlay use spam
    def bar [] { "bar" }
    overlay hide spam --keep-custom
    bar
    
'export alias f = "foo"' | save spam.nu
    overlay use spam.nu
    overlay hide spam
module spam { export-env { let-env FOO = "foo" } }
    overlay use spam
    overlay hide
overlay new spam
    cd some-dir
    overlay hide --keep-env [ PWD ] spam


module spam { export def foo [] { "foo" } }
    overlay use spam
    overlay list | last


overlay new spam


module spam { export def foo [] { "foo" } }
    overlay use spam
    foo
module spam { export def foo [] { "foo" } }
    overlay use spam as spam_new
    foo
'export def foo { "foo" }'
    overlay use --prefix spam
    spam foo
'export-env { let-env FOO = "foo" }' | save spam.nu
    overlay use spam.nu
    $env.FOO


mkdir foo bar; enter foo; enter ../bar; p
p


[1 2 3] | par-each {|| 2 * $in }
[foo bar baz] | par-each {|e| $e + '!' } | sort
1..3 | enumerate | par-each {|p| update item ($p.item * 2)} | sort-by item | get item
[1 2 3] | enumerate | par-each { |e| if $e.item == 2 { $"found 2 at ($e.index)!"} }


"hi there" | parse "{foo} {bar}"
"hi there" | parse -r '(?P<foo>\w+) (?P<bar>\w+)'
"foo bar." | parse -r '\s*(?<name>\w+)(?=\.)'
"foo! bar." | parse -r '(\w+)(?=\.)|(\w+)(?=!)'
" @another(foo bar)   " | parse -r '\s*(?<=[() ])(@\w+)(\([^)]*\))?\s*'
"abcd" | parse -r '^a(bc(?=d)|b)cd$'




'/home/joe/test.txt' | path basename
[[name];[/home/joe]] | path basename -c [ name ]
'/home/joe/test.txt' | path basename -r 'spam.png'


'/home/joe/code/test.txt' | path dirname
ls ('.' | path expand) | path dirname -c [ name ]
'/home/joe/code/test.txt' | path dirname -n 2
'/home/joe/code/test.txt' | path dirname -n 2 -r /home/viking


'/home/joe/todo.txt' | path exists
ls | path exists -c [ name ]


'/home/joe/foo/../bar' | path expand
ls | path expand -c [ name ]
'foo/../bar' | path expand


'/home/viking' | path join spam.txt
'/home/viking' | path join spams this_spam.txt
ls | path join spam.txt -c [ name ]
[ '/' 'home' 'viking' 'spam.txt' ] | path join
[[ parent stem extension ]; [ '/home/viking' 'spam' 'txt' ]] | path join


'/home/viking/spam.txt' | path parse
'/home/viking/spam.tar.gz' | path parse -e tar.gz | upsert extension { 'txt' }
'/etc/conf.d' | path parse -e ''
ls | path parse -c [ name ]


'/home/viking' | path relative-to '/home'
ls ~ | path relative-to ~ -c [ name ]
'eggs/bacon/sausage/spam' | path relative-to 'eggs/bacon/sausage'


'/home/viking/spam.txt' | path split
ls ('.' | path expand) | path split -c [ name ]


'.' | path type
ls | path type -c [ name ]






pnet


port 3121 4000
port


[1,2,3,4] | prepend 0
[2,3,4] | prepend [0,1]
[2,nu,4,shell] | prepend [0,1,rocks]


print "hello world"
print (2 + 3)


def spam [] { "spam" }; profile {|| spam | str length } -d 2 --source


ps
ps | sort-by mem | last 5
ps | sort-by cpu | last 3
ps | where name =~ 'nu'
ps | where pid == $nu.pid | get ppid




open foo.db | query db "SELECT * FROM Bar"




http get https://phoronix.com | query web -q 'header'
http get https://en.wikipedia.org/wiki/List_of_cities_in_India_by_population
    | query web -t [Rank City 'Population(2011)[3]' 'Population(2001)' 'State or union territory']
http get https://www.nushell.sh | query web -q 'h2, h2 + p' | group 2 | each {rotate --ccw tagline description} | flatten
http get https://example.org | query web --query a --attribute href






random bool
random bool --bias 0.75


random chars
random chars -l 20


random decimal
random decimal ..500
random decimal 100000..
random decimal 1.0..1.1


random dice
random dice -d 10 -s 12


random integer
random integer ..500
random integer 100000..
random integer 1..10


random uuid


[0,1,2,3,4,5] | range 4..5
[0,1,2,3,4,5] | range (-2)..
[0,1,2,3,4,5] | range (-3)..-2


[ 1 2 3 4 ] | reduce {|it, acc| $it + $acc }
[ 8 7 6 ] | enumerate | reduce -f 0 {|it, acc| $acc + $it.item + $it.index }
[ 1 2 3 4 ] | reduce -f 10 {|it, acc| $acc + $it }
[ i o t ] | reduce -f "Arthur, King of the Britons" {|it, acc| $acc | str replace -a $it "X" }
['foo.gz', 'bar.gz', 'baz.gz'] | enumerate | reduce -f '' {|str all| $"($all)(if $str.index != 0 {'; '})($str.index + 1)-($str.item)" }


"hello world" | regex '(?P<first>\w+) (?P<second>\w+)'


register ~/.cargo/bin/nu_plugin_query
let plugin = ((which nu).path.0 | path dirname | path join 'nu_plugin_query'); nu -c $'register ($plugin); version'


ls | reject modified
[[a, b]; [1, 2]] | reject a
{a: 1, b: 2} | reject a
{a: {b: 3, c: 5}} | reject a.b


[[a, b]; [1, 2]] | rename my_column
[[a, b, c]; [1, 2, 3]] | rename eggs ham bacon
[[a, b, c]; [1, 2, 3]] | rename -c [a ham]
{a: 1 b: 2} | rename x y


def foo [] { return }


[0,1,2,3] | reverse
[{a: 1} {a: 2}] | reverse


rm file.txt
rm --trash file.txt
rm --permanent file.txt
rm --force file.txt
ls | where size == 0KB and type == file | each { rm $in.name } | null




[[a b]; [1 2] [3 4] [5 6]] | roll down


{a:1 b:2 c:3} | roll left
[[a b c]; [1 2 3] [4 5 6]] | roll left
[[a b c]; [1 2 3] [4 5 6]] | roll left --cells-only


{a:1 b:2 c:3} | roll right
[[a b c]; [1 2 3] [4 5 6]] | roll right
[[a b c]; [1 2 3] [4 5 6]] | roll right --cells-only


[[a b]; [1 2] [3 4] [5 6]] | roll up


{a:1, b:2} | rotate
[[a b]; [1 2] [3 4] [5 6]] | rotate
[[a b]; [1 2]] | rotate col_a col_b
[[a b]; [1 2]] | rotate --ccw
[[a b]; [1 2] [3 4] [5 6]] | rotate --ccw
[[a b]; [1 2]] | rotate --ccw col_a col_b


run-external "echo" "-n" "hello"
run-external --redirect-stdout "echo" "-n" "hello" | split chars


'save me' | save foo.txt
'append me' | save --append foo.txt
{ a: 1, b: 2 } | save foo.json
do -i {} | save foo.txt --stderr foo.txt
do -i {} | save foo.txt --stderr bar.txt


open foo.db | schema


[{a: a b: b}] | select a
{a: a b: b} | select a
ls | select name
ls | select 0 1 2 3


seq 1 10
seq 1.0 0.1 2.0
seq 1 5 | str join '|'


seq char a e
seq char a e | str join '|'


seq date --days 10
seq date --days 10 -r
seq date --days 10 -o '%m/%d/%Y' -r
seq date -b '2020-01-01' -e '2020-01-10'
seq date -b '2020-01-01' -e '2020-01-31' -n 5


enter ..; shells
shells | where active == true


[[version patch]; ['1.0.0' false] ['3.0.1' true] ['2.0.0' false]] | shuffle


"There are seven words in this sentence" | size
'今天天气真好' | size 
"Amélie Amelie" | size


[2 4 6 8] | skip 1
[[editions]; [2015] [2018] [2021]] | skip 2


[-2 0 2 -1] | skip until {|x| $x > 0 }
let cond = {|x| $x > 0 }; [-2 0 2 -1] | skip until $cond
[{a: -2} {a: 0} {a: 2} {a: -1}] | skip until {|x| $x.a > 0 }


[-2 0 2 -1] | skip while {|x| $x < 0 }
let cond = {|x| $x < 0 }; [-2 0 2 -1] | skip while $cond
[{a: -2} {a: 0} {a: 2} {a: -1}] | skip while {|x| $x.a < 0 }


sleep 1sec


[2 0 1] | sort
[2 0 1] | sort -r
[betty amy sarah] | sort
[betty amy sarah] | sort -r
[airplane Truck Car] | sort -i
[airplane Truck Car] | sort -i -r
{b: 3, a: 4} | sort
{b: 4, a: 3, c:1} | sort -v


ls | sort-by modified
ls | sort-by name -i
[[fruit count]; [apple 9] [pear 3] [orange 7]] | sort-by fruit -r


source foo.nu
source ./foo.nu; say-hi


source-env foo.nu




'hello' | split chars
'🇯🇵ほげ' | split chars -g


'a--b--c' | split column '--'
'abc' | split column -c ''
['a-b' 'c-d'] | split column -
['a -  b' 'c  -    d'] | split column -r '\s*-\s*'


[a, b, c, d, e, f, g] | split list d
[[1,2], [2,3], [3,4]] | split list [2,3]
[a, b, c, d, a, e, f, g] | split list a
[a, b, c, d, a, e, f, g] | split list -r '(b|e)'


'abc' | split row ''
'a--b--c' | split row '--'
'-a-b-c-' | split row '-'
'a   b       c' | split row -r '\s+'


'hello world' | split words
'hello to the world' | split words -l 3
http get https://www.gutenberg.org/files/11/11-0.txt | str downcase | split words -l 2 | uniq -c | sort-by count --reverse | first 10


{
        '2019': [
          { name: 'andres', lang: 'rb', year: '2019' },
          { name: 'jt', lang: 'rs', year: '2019' }
        ],
        '2021': [
          { name: 'storm', lang: 'rs', 'year': '2021' }
        ]
    } | split-by lang


start file.txt
start file.jpg
start .
start file.pdf
start https://www.nushell.sh




 'NuShell' | str camel-case
'this-is-the-first-case' | str camel-case
 'this_is_the_second_case' | str camel-case
[[lang, gems]; [nu_test, 100]] | str camel-case lang


'good day' | str capitalize
'anton' | str capitalize
[[lang, gems]; [nu_test, 100]] | str capitalize lang




'my_library.rb' | str contains '.rb'
'my_library.rb' | str contains -i '.RB'
 [[ColA ColB]; [test 100]] | str contains 'e' ColA
 [[ColA ColB]; [test 100]] | str contains -i 'E' ColA
 [[ColA ColB]; [test hello]] | str contains 'e' ColA ColB
'hello' | str contains 'banana'
[one two three] | str contains o
[one two three] | str contains -n o


'nushell' | str distance 'nutshell'
[{a: 'nutshell' b: 'numetal'}] | str distance 'nushell' 'a' 'b'


'NU' | str downcase
'TESTa' | str downcase
[[ColA ColB]; [Test ABC]] | str downcase ColA
[[ColA ColB]; [Test ABC]] | str downcase ColA ColB


'my_library.rb' | str ends-with '.rb'
'my_library.rb' | str ends-with '.txt'
'my_library.rb' | str ends-with -i '.RB'




 'my_library.rb' | str index-of '.rb'
'🇯🇵ほげ ふが ぴよ' | str index-of -g 'ふが'
 '.rb.rb' | str index-of '.rb' -r 1..
 '123456' | str index-of '6' -r ..4
 '123456' | str index-of '3' -r 1..4
 '/this/is/some/path/file.txt' | str index-of '/' -e


['nu', 'shell'] | str join
['nu', 'shell'] | str join '-'


'NuShell' | str kebab-case
'thisIsTheFirstCase' | str kebab-case
'THIS_IS_THE_SECOND_CASE' | str kebab-case
[[lang, gems]; [nuTest, 100]] | str kebab-case lang


'hello' | str length
'🇯🇵ほげ ふが ぴよ' | str length -g
['hi' 'there'] | str length




'nu-shell' | str pascal-case
'this-is-the-first-case' | str pascal-case
'this_is_the_second_case' | str pascal-case
[[lang, gems]; [nu_test, 100]] | str pascal-case lang


'my_library.rb' | str replace '(.+).rb' '$1.nu'
'abc abc abc' | str replace -a 'b' 'z'
[[ColA ColB ColC]; [abc abc ads]] | str replace -a 'b' 'z' ColA ColC
'dogs_$1_cats' | str replace '\$1' '$2' -n
'c:\some\cool\path' | str replace 'c:\some\cool' '~' -s
'abc abc abc' | str replace -a 'b' 'z' -s
'a successful b' | str replace '\b([sS])uc(?:cs|s?)e(ed(?:ed|ing|s?)|ss(?:es|ful(?:ly)?|i(?:ons?|ve(?:ly)?)|ors?)?)\b' '${1}ucce$2'
'GHIKK-9+*' | str replace '[*[:xdigit:]+]' 'z'


'Nushell' | str reverse
['Nushell' 'is' 'cool'] | str reverse




 "NuShell" | str screaming-snake-case
 "this_is_the_second_case" | str screaming-snake-case
"this-is-the-first-case" | str screaming-snake-case
[[lang, gems]; [nu_test, 100]] | str screaming-snake-case lang


 "NuShell" | str snake-case
 "this_is_the_second_case" | str snake-case
"this-is-the-first-case" | str snake-case
[[lang, gems]; [nuTest, 100]] | str snake-case lang


'my_library.rb' | str starts-with 'my'
'Cargo.toml' | str starts-with 'Car'
'Cargo.toml' | str starts-with '.toml'
'Cargo.toml' | str starts-with -i 'cargo'


 'good nushell' | str substring 5..12
 '🇯🇵ほげ ふが ぴよ' | str substring -g 4..6


'nu-shell' | str title-case
'this is a test case' | str title-case
[[title, count]; ['nu test', 100]] | str title-case title








'Nu shell ' | str trim
'=== Nu shell ===' | str trim -c '=' | str trim
' Nu shell ' | str trim -l
'=== Nu shell ===' | str trim -c '='
' Nu shell ' | str trim -r
'=== Nu shell ===' | str trim -r -c '='


'nu' | str upcase


sys
(sys).host | get name
(sys).host.name


ls | table -n 1
[[a b]; [1 2] [3 4]] | table
[[a b]; [1 2] [2 [4 4]]] | table --expand
[[a b]; [1 2] [2 [4 4]]] | table --collapse


[1 2 3] | take 1
[1 2 3] | take 2
[[editions]; [2015] [2018] [2021]] | take 2
0x[01 23 45] | take 2
1..10 | take 3


[-1 -2 9 1] | take until {|x| $x > 0 }
let cond = {|x| $x > 0 }; [-1 -2 9 1] | take until $cond
[{a: -1} {a: -2} {a: 9} {a: 1}] | take until {|x| $x.a > 0 }


[-1 -2 9 1] | take while {|x| $x < 0 }
let cond = {|x| $x < 0 }; [-1 -2 9 1] | take while $cond
[{a: -1} {a: -2} {a: 9} {a: 1}] | take while {|x| $x.a < 0 }


term size
(term size).columns
(term size).rows


timeit { sleep 500ms }
http get https://www.nushell.sh/book/ | timeit { split chars }
timeit ls -la




[[foo bar]; [1 2]] | to csv
[[foo bar]; [1 2]] | to csv -s ';' 
{a: 1 b: 2} | to csv


[[foo bar]; [1 2]] | to html
[[foo bar]; [1 2]] | to html --partial
[[foo bar]; [1 2]] | to html --dark


[a b c] | to json
[Joe Bob Sam] | to json -i 4
[1 2 3] | to json -r


[[foo bar]; [1 2]] | to md
[[foo bar]; [1 2]] | to md --pretty
[{"H1": "Welcome to Nushell" } [[foo bar]; [1 2]]] | to md --per-element --pretty
[0 1 2] | to md --pretty


[1 2 3] | to nuon
[1 2 3] | to nuon --indent 2
[1 2 3] | to nuon --indent 2 --raw
{date: 2000-01-01, data: [1 [2 3] 4.56]} | to nuon --indent 2


1 | to text
git help -a | lines | find -r '^ ' | to text
ls | to text


{foo: 1 bar: 'qwe'} | to toml


[[foo bar]; [1 2]] | to tsv
{a: 1 b: 2} | to tsv


{tag: note attributes: {} content : [{tag: remember attributes: {} content : [{tag: null attrs: null content : Event}]}]} | to xml
{tag: note content : [{tag: remember content : [Event]}]} | to xml
{tag: note content : [{tag: remember content : [Event]}]} | to xml -p 3


[[foo bar]; ["1" "2"]] | to yaml


touch fixture.json
touch a b c
touch -m fixture.json
touch -m -d "yesterday" a b c
touch -m -r fixture.json d e
touch -a -d "August 24, 2019; 12:30:30" fixture.json


[[c1 c2]; [1 2]] | transpose
[[c1 c2]; [1 2]] | transpose key val
[[c1 c2]; [1 2]] | transpose -i val
{c1: 1, c2: 2} | transpose | transpose -i -r -d


try { asdfasdf }
try { asdfasdf } catch { echo 'missing' } 


tutor begin
tutor -f "$in"


[2 3 3 4] | uniq
[1 2 2] | uniq -d
[1 2 2] | uniq -u
['hello' 'goodbye' 'Hello'] | uniq -i
[1 2 2] | uniq -c


[[fruit count]; [apple 9] [apple 2] [pear 3] [orange 7]] | uniq-by fruit


{'name': 'nu', 'stars': 5} | update name 'Nushell'
[[count fruit]; [1 'apple']] | enumerate | update item.count {|e| ($e.item.fruit | str length) + $e.index } | get item
[[project, authors]; ['nu', ['Andrés', 'JT', 'Yehuda']]] | update authors {|row| $row.authors | str join ','}
[[project, authors]; ['nu', ['Andrés', 'JT', 'Yehuda']]] | update authors {|| str join ','}


[
        ["2021-04-16", "2021-06-10", "2021-09-18", "2021-10-15", "2021-11-16", "2021-11-17", "2021-11-18"];
        [          37,            0,            0,            0,           37,            0,            0]
    ] | update cells { |value|
          if $value == 0 {
            ""
          } else {
            $value
          }
    }
[
        ["2021-04-16", "2021-06-10", "2021-09-18", "2021-10-15", "2021-11-16", "2021-11-17", "2021-11-18"];
        [          37,            0,            0,            0,           37,            0,            0]
    ] | update cells -c ["2021-11-18", "2021-11-17"] { |value|
            if $value == 0 {
              ""
            } else {
              $value
            }
    }


{'name': 'nu', 'stars': 5} | upsert name 'Nushell'
[[name lang]; [Nushell ''] [Reedline '']] | upsert lang 'Rust'
{'name': 'nu', 'stars': 5} | upsert language 'Rust'
[[count fruit]; [1 'apple']] | enumerate | upsert item.count {|e| ($e.item.fruit | str length) + $e.index } | get item
[1 2 3] | upsert 0 2
[1 2 3] | upsert 3 4




{ mode:normal userid:31415 } | url build-query
[[foo bar]; ["1" "2"]] | url build-query
{a:"AT&T", b: "AT T"} | url build-query


'https://example.com/foo bar' | url encode
['https://example.com/foo bar' 'https://example.com/a>b' '中文字/eng/12 34'] | url encode
'https://example.com/foo bar' | url encode --all


{
        "scheme": "http",
        "username": "",
        "password": "",
        "host": "www.pixiv.net",
        "port": "",
        "path": "/member_illust.php",
        "query": "mode=medium&illust_id=99260204",
        "fragment": "",
        "params":
        {
            "mode": "medium",
            "illust_id": "99260204"
        }
    } | url join
{
        "scheme": "http",
        "username": "user",
        "password": "pwd",
        "host": "www.pixiv.net",
        "port": "1234",
        "query": "test=a",
        "fragment": ""
    } | url join
{
        "scheme": "http",
        "host": "www.pixiv.net",
        "port": "1234",
        "path": "user",
        "fragment": "frag"
    } | url join


'http://user123:pass567@www.example.com:8081/foo/bar?param1=section&p2=&f[name]=vldc#hello' | url parse


module spam { export def foo [] { "foo" } }; use spam foo; foo
module foo { export def-env bar [] { let-env FOO_BAR = "BAZ" } }; use foo bar; bar; $env.FOO_BAR


{ mode:normal userid:31415 } | values
{ f:250 g:191 c:128 d:1024 e:2000 a:16 b:32 } | values
[[name meaning]; [ls list] [mv move] [cd 'change directory']] | values


version




view files


let abc = {|| echo 'hi' }; view source $abc
def hi [] { echo 'Hi!' }; view source hi
def-env foo [] { let-env BAR = 'BAZ' }; view source foo
module mod-foo { export-env { let-env FOO_ENV = 'BAZ' } }; view source mod-foo
alias hello = echo hi; view source hello


some | pipeline | or | variable | debug -r; view span 1 2


watch . --glob=**/*.rs {|| cargo test }
watch . { |op, path, new_path| $"($op) ($path) ($new_path)"}
watch /foo/bar { |op, path| $"($op) - ($path)(char nl)" | save --append changes_in_bar.log }


[{a: 1} {a: 2}] | where a > 1
[1 2] | where {|x| $x > 1}
ls | where size > 2kb
ls | where type == file
ls | where name =~ "Car"
ls | where modified >= (date now) - 2wk
ls | where type == file | sort-by name -n | enumerate | where {|e| $e.item.name !~ $'^($e.index + 1)' } | each {|| get item }


which myapp


mut x = 0; while $x < 10 { $x = $x + 1 }


[1 2 3 4] | window 2
[1, 2, 3, 4, 5, 6, 7, 8] | window 2 --stride 3
[1, 2, 3, 4, 5] | window 3 --stride 3 --remainder


with-env [MYENV "my env value"] { $env.MYENV }
with-env [X Y W Z] { $env.X }
with-env [[X W]; [Y Z]] { $env.W }
with-env {X: "Y", W: "Z"} { [$env.X $env.W] }


[1 2 3] | wrap num
1..3 | wrap num




[1 2] | zip [3 4]
1..3 | zip 4..6
glob *.ogg | zip ['bang.ogg', 'fanfare.ogg', 'laser.ogg'] | each {|| mv $in.0 $in.1 }


