# module doc

# next line comment
use b.nu [colors]
use a.nu # inline comment
use d.nu [thing]

# constants
const PRIORITY_ERROR: int = 3
const PRIORITY_INFO: int = 6
const PRIORITY_DEBUG: int = 7

# next multi
# line comment
const VOLATILE_FIELDS: list<string> = ["EXCEPTION" "EXCEPTION_DEBUG"]

# func comment
def level-to-max-priority [level: string]: nothing -> int {
  match ($level | str upcase) {
    ERROR => 3
    WARNING => 4
    INFO => 6
    _ => 7
  }
}

# func comment
def format-timestamp [ts_us: string]: nothing -> nothing { }
