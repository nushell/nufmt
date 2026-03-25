def do-thing [--check] { $check }
def run [--check] {
    let c: bool = $check
    do-thing --check=($c)
}
