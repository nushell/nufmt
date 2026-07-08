def mint-token [cache: string] {
    let remote = r#'
    set -euo pipefail
    cache="$1"
    # -print -quit: find exits after the first hit
    adm=$(find /nix/store -name atticadm -print -quit)
  '#
    # str trim: drop the trailing newline so it can't land inside the
    # secret and break the Authorization header.
    $remote | ssh (arca-ssh) bash -s $cache | str trim
}

# Cache names are validated to [a-z0-9-] — attic's own charset, and it blocks
# shell-metacharacter injection into the `ssh ... bash -s $cache` above.
def declared-caches [] {
    1
}
