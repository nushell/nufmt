extern "git" []
extern "git" []
extern "git status" [--short (-s)]
extern "cargo build" [--release --target: string]
extern "npm" [
    command: string
    --global (-g)
    --save-dev (-D)
]
extern "docker run" [
    image: string
    --detach (-d)
    --name: string
    --port (-p): string
    ...args: string
]
extern ls []
extern cat [file: path]
extern grep [
    pattern: string
    ...files: path
    -i
    -r
    -n
]
