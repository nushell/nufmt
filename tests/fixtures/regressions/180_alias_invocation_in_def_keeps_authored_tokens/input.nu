alias wds = systemd-run waydroid session start
alias wdo = waydroid session stop

def wdr [] {
    wdo
    wds
}
