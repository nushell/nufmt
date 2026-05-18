def foo [--editor: string] {
    match $editor {
        _ => {
            error make {
                msg: "unsupported editor"
                label: {
                    span: (metadata $editor).span
                }
            }
        }
    }
}
