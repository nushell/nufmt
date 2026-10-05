def cmd [dt: duration = 7min]: nothing -> nothing { }

def spaced [dt: duration = 7min]: nothing -> nothing { }

def mixed [
    path: path = "."
    timeout: duration = 10sec
    --interval: duration = 40ms
    --size: filesize = 1kb
]: nothing -> table { }

def many [
    a: int
    b: duration = 5sec
    --c: filesize = 2kb
    --d: int = 1
]: nothing -> nothing, string -> int { }
