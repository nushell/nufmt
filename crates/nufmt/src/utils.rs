use std::{
    fs,
    path::{Path, PathBuf},
};

use nu_parser::FlatShape;
use nu_protocol::{ast::Block, Span};

/// Check if the file matches the extension
pub(crate) fn is_file_extension(file: &Path, extension: &str) -> bool {
    String::from(file.to_str().unwrap()).ends_with(extension)
}

/// Walks down directory structure and returns all files
pub(crate) fn recurse_directory(path: impl AsRef<Path>) -> std::io::Result<Vec<PathBuf>> {
    let mut buf = vec![];
    let entries = fs::read_dir(path)?;

    for entry in entries {
        let entry = entry?;
        let meta = entry.metadata()?;

        if meta.is_dir() {
            let mut subdir = recurse_directory(entry.path())?;
            buf.append(&mut subdir);
        }

        if meta.is_file() {
            buf.push(entry.path());
        }
    }

    Ok(buf)
}

/// return true if the Nushell block has at least 1 pipeline
///
/// This function exists because sometimes is passed to `nufmt` an empty String,
/// or a nu code which the parser can't identify something runnable
/// (like a list of comments)
///
/// We don't want to return a blank file if that is the case,
/// so this check gives the opportunity to `nufmt`
/// to know when not to touch the file at all in the implementation.
pub(crate) fn block_has_pipelines(block: &Block) -> bool {
    !block.pipelines.is_empty()
}

/// return true if the given span is the last one
pub(crate) fn is_last_span(span: Span, flat: &[(Span, FlatShape)]) -> bool {
    span == flat.last().unwrap().0
}
