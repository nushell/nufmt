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
