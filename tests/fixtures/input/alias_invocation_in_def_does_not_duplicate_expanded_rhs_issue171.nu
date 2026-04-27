alias ez = eza --git --git-repos --long --header --all --time-style=relative --group
def ezt [level: int = 2] {
    ez --git --git-repos --long --header --time-style=relative --group --tree --level $level
}
