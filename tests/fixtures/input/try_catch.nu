try { error make {msg: "test"} }
try { error make {msg: "test"} }
try { error make {msg: "test"} } catch { print "caught" }
try { 1 / 0 } catch { print "error" }
try {
    risky_operation
} catch {
    print "error occurred"
}
try { risky } catch {|err| print $err.msg }
