//! In this module occurs most of the magic in `nufmt`.
//!
//! It has functions to format slice of bytes and some help functions to separate concerns while doing the job.
use crate::config::Config;
use log::{error, info, trace};
use nu_parser::{flatten_block, parse, FlatShape};
use nu_protocol::{
    ast::Block,
    engine::{EngineState, StateWorkingSet},
    DeclId, Span,
};

#[derive(Debug, Clone)]
struct NewDeclId<'a> {
    decls: &'a Vec<(Vec<u8>, DeclId)>,
}

impl<'a> NewDeclId<'a> {
    pub fn new(decls: &'a Vec<(Vec<u8>, DeclId)>) -> NewDeclId<'a> {
        NewDeclId { decls }
    }

    pub fn get_decl_name(&'a self, decl_id: usize) -> Option<&'a str> {
        for decl in self.decls {
            if decl_id == decl.1.get() {
                return Some(std::str::from_utf8(&decl.0).expect("Failed to parse DeclId's name"));
            }
        }
        None
    }
}

fn get_engine_state() -> EngineState {
    nu_cmd_lang::create_default_context()
}

/// format an array of bytes
///
/// Reading the file gives you a list of bytes
pub(crate) fn format_inner(contents: &[u8], _config: &Config) -> Vec<u8> {
    let engine_state = get_engine_state();
    let decls_sorted: Vec<(Vec<u8>, nu_protocol::Id<nu_protocol::marker::Decl>)> =
        engine_state.get_decls_sorted(false);

    let decl_ids = NewDeclId::new(&decls_sorted);

    let mut working_set = StateWorkingSet::new(&engine_state);

    let parsed_block = parse(&mut working_set, None, contents, false);
    trace!("parsed block:\n{:?}", &parsed_block);

    if !block_has_pipelines(&parsed_block) {
        trace!("block has no pipelines!");
        info!("File has no code to format.");
        return contents.to_vec();
    }

    let flat = flatten_block(&working_set, &parsed_block);
    trace!("flattened block:\n{:?}", &flat);

    let mut out: Vec<u8> = vec![];
    let mut start = 0;
    let end_of_file = contents.len();

    let mut after_a_def = false;

    for (span, shape) in flat.clone() {
        if span.start > start {
            trace!(
                "Span does not start at the beginning! span {0}, start: {1}",
                span.start,
                start
            );

            let skipped_contents = &contents[start..span.start];
            let printable = String::from_utf8_lossy(skipped_contents).to_string();
            trace!("contents: {:?}", printable);

            out = write_only_if_have_hastag_or_equal(skipped_contents, out, true);
        }

        let mut bytes = working_set.get_span_contents(span);
        let content = String::from_utf8_lossy(bytes).to_string();
        trace!("shape is {shape}");
        trace!("shape contents: {:?}", &content);

        match shape {
            FlatShape::Int | FlatShape::Nothing => out.extend(bytes),
            FlatShape::StringInterpolation => {
                out.extend(bytes);
            }
            FlatShape::List | FlatShape::Record => {
                bytes = trim_ascii_whitespace(bytes);
                out.extend(bytes);
            }
            FlatShape::Block | FlatShape::Closure => {
                bytes = trim_ascii_whitespace(bytes);
                out.extend(bytes);
            }
            FlatShape::String => {
                out.extend(bytes);
                // if it'a string after a `def`, add a space before the `[`
                if after_a_def {
                    out.extend(b" ");
                }
            }
            FlatShape::Pipe => {
                out.extend(b"| ");
            }
            FlatShape::InternalCall(declid) => {
                let declid = declid.get();
                let decl_name = decl_ids.get_decl_name(declid);

                trace!("Called Internal call with {declid}");

                if let Some(decl_name) = decl_name {
                    out = resolve_call(bytes, decl_name, out);
                    after_a_def = decl_name == "def";
                }
            }
            FlatShape::External => out = resolve_external(bytes, out),
            FlatShape::ExternalArg | FlatShape::Signature | FlatShape::Keyword => {
                out.extend(bytes);
                out = insert_newline(out);
            }
            FlatShape::VarDecl(varid) | FlatShape::Variable(varid) => {
                trace!("Called variable or vardecl with {}", varid.get());
                out.extend(bytes);
                out.extend(b" ");
            }
            FlatShape::Garbage => {
                error!("found garbage 😢 {content}");
                out.extend(bytes);
                out = insert_newline(out);
            }

            _ => out.extend(bytes),
        }

        if is_last_span(span, &flat) && span.end < end_of_file {
            trace!(
                "The last span doesn't end the file! span: {0}, end: {1}",
                span.end,
                end_of_file
            );

            let remaining_contents = &contents[span.end..end_of_file];
            let printable = String::from_utf8_lossy(remaining_contents).to_string();
            trace!("contents: {:?}", printable);

            out = write_only_if_have_hastag_or_equal(remaining_contents, out, false);
        }

        start = span.end + 1;
    }

    out
}

/// insert a newline at the end of a buffer
fn insert_newline(mut bytes: Vec<u8>) -> Vec<u8> {
    bytes.extend(b"\n");
    bytes
}

/// given a list of `bytes` and a `out`put to write only the bytes if they contain `#` or `=`
///
/// One tiny little detail: the order of bytes is important to nufmt.
/// It is not the same to have
/// `bytes` + `out` (you have to put \n after bytes)
/// `bytes` + \n + `out`
///
/// than having:
/// `out` + `bytes` (you have to put \n before bytes)
/// `out` + \n + `bytes`
///
/// That's what `bytes_before_content` bool is for
fn write_only_if_have_hastag_or_equal(
    bytes: &[u8],
    mut out: Vec<u8>,
    bytes_before_content: bool,
) -> Vec<u8> {
    if bytes.contains(&b'#') {
        trace!("This have a comment. Writing.");
        if bytes_before_content {
            out.extend(trim_ascii_whitespace(bytes));
            out = insert_newline(out);
        } else {
            out = insert_newline(out);
            out.extend(trim_ascii_whitespace(bytes));
        }
    } else if bytes.contains(&b'=') {
        out.extend(trim_ascii_whitespace(bytes));
        out.extend(b" ");
    } else {
        trace!("The contents doesn't have a '#'. Skipping.");
    }
    out
}

#[allow(clippy::wildcard_in_or_patterns)]
fn resolve_call(c_bytes: &[u8], decl_name: &str, mut out: Vec<u8>) -> Vec<u8> {
    out = match decl_name {
        "if" => insert_newline(out),
        "def" => insert_newline(out),
        "export def" | _ => out,
    };
    out.extend(c_bytes);
    out.extend(b" ");
    out
}

fn resolve_external(c_bytes: &[u8], mut out: Vec<u8>) -> Vec<u8> {
    out = match c_bytes {
        [b'c', b'd'] => insert_newline(out),
        _ => out,
    };
    out.extend(c_bytes);
    out.extend(b" ");
    out
}

/// make sure there is a newline at the end of a buffer
pub(crate) fn add_newline_at_end_of_file(out: Vec<u8>) -> Vec<u8> {
    match out.last() {
        Some(&b'\n') => out,
        _ => insert_newline(out),
    }
}

/// strip all spaces, new lines and tabs found a sequence of bytes
///
/// Because you don't know how the incoming code is formatted,
/// the best way to format is to strip all the whitespace
/// and afterwards include the new lines and indentation correctly
/// according to the configuration
fn trim_ascii_whitespace(x: &[u8]) -> &[u8] {
    let Some(from) = x.iter().position(|x| !x.is_ascii_whitespace()) else {
        return &x[0..0];
    };
    let to = x.iter().rposition(|x| !x.is_ascii_whitespace()).unwrap();
    let result = &x[from..=to];
    let printable = String::from_utf8_lossy(result).to_string();
    trace!("stripped the whitespace, result: {:?}", printable);
    result
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
fn block_has_pipelines(block: &Block) -> bool {
    !block.pipelines.is_empty()
}

/// return true if the given span is the last one
fn is_last_span(span: Span, flat: &[(Span, FlatShape)]) -> bool {
    span == flat.last().unwrap().0
}
