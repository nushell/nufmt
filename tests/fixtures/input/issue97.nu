def start_zellij [] {
    if ($env.ZELLIJ_AUTO_ATTACH? | default "false") == "true" { zellij attach -c }
    if ($env.ZELLIJ_AUTO_EXIT? | default "false") == "true" { exit }
}
