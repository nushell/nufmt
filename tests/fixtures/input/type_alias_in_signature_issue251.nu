type Rectangle = record<width: int, height: int>

def print-area [rect: Rectangle] {
  print ($rect.width * $rect.height)
}

def enlarge []: Rectangle -> Rectangle {
  $in
}

def merge [...rects: Rectangle] {
  $rects
}

def typed-any [x: int]: nothing -> any {
  $x
}

def default-with-colon [pad: string = "12:30"]: int -> any {
  $pad
}

def flag-and-any [--flag: int]: nothing -> any {
  $flag
}

def optional-alias [r?: Rectangle] {
  $r
}

def flag-alias [--rect: Rectangle, --opt: any] {
  $rect
}

def nested-alias [rs: list<Rectangle>] {
  $rs
}

def written-any [p: any, q: any] {
  $p
}

def spaced-io []: nothing  ->   any {
}

def mixed-io [x]: int  ->  Rectangle {
  $x
}

def multi-line []: [
    nothing -> any
    int -> Rectangle
  ] {
  1
}

def record-io []: Rectangle -> record<r: int, g: int> {
  {r: 1, g: 2}
}

def spaced-comma []: [int -> Rect , string -> Rectangle] { 1 }
