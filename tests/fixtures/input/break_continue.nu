loop { break }
loop { break }
for x in [1, 2, 3] { if $x == 2 { break } }
for x in [1, 2, 3] { if $x == 2 { continue } }
while true { break }
while $x > 0 { if $x == 5 { continue }; $x = $x - 1 }
loop {
    if $done { break }
    continue
}
