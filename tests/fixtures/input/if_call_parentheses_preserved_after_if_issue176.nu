def is-wayland [] { $env.WAYLAND_DISPLAY? != null }

if (is-wayland) {
    print "wayland"
}

if not (is-wayland) {
    print "not wayland"
} else if (is-wayland) {
    print "else wayland"
}
