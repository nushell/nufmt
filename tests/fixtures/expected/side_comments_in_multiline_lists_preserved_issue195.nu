let flags = [
    # opener-side comment should survive
    "--name" # keep this side comment
    "alice" # and this one too
    # keep this standalone comment
    "--age"
    42 # keep numeric side comment
]

let single = [
    1 # keep single-item side comment
]

const SIT = [ # SUPPORTED_INSTALLATION_TYPES
    'cargo-git'
    'crate'
    'git-module'
    'git-script'
]
