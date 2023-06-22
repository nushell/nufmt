//! In this module occurs most of the magic in `nufmt`.
//!
//! It has functions to format slice of bytes and some help functions to separate concerns while doing the job.
use crate::config::Config;
use log::{error, info, trace};
use nu_parser::{flatten_block, parse, FlatShape};
use nu_protocol::{
    ast::Block,
    engine::{EngineState, StateWorkingSet},
    Span,
};

fn get_engine_state() -> EngineState {
    nu_cmd_lang::create_default_context()
}

/// format an array of bytes
///
/// Reading the file gives you a list of bytes
pub(crate) fn format_inner(contents: &[u8], _config: &Config) -> Vec<u8> {
    let engine_state = get_engine_state();
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

            if skipped_contents.contains(&b'#') {
                trace!("This have a comment. Writing.");
                out.extend(trim_ascii_whitespace(skipped_contents));
                out.push(b'\n');
            } else {
                trace!("The contents doesn't have a '#'. Skipping.");
            }
        }

        let mut c_bites = working_set.get_span_contents(span);
        let content = String::from_utf8_lossy(c_bites).to_string();
        let mut inside_string_interpolation = false;
        trace!("shape is {shape}");
        trace!("shape contents: {:?}", &content);

        match shape {
            FlatShape::Int | FlatShape::Nothing => out.extend(c_bites),
            FlatShape::StringInterpolation => {
                out.extend(c_bites);
                inside_string_interpolation = !inside_string_interpolation;
                trace!("inside_string_interpolation 🚚: {inside_string_interpolation}")
            }
            FlatShape::List | FlatShape::Record => {
                c_bites = trim_ascii_whitespace(c_bites);
                let printable = String::from_utf8_lossy(c_bites).to_string();
                trace!("stripped the whitespace, result: {:?}", printable);
                out.extend(c_bites);
            }
            FlatShape::String => {
                out.extend(c_bites);
                // add a space after the string only if NOT inside a string interpolation block $" ()"
                if !inside_string_interpolation {
                    // add a space after the string, so the parser doen't misleads the string into garbage
                    out.extend(b" ");
                }
            }
            FlatShape::Pipe => {
                out.extend(b"| ");
            }
            FlatShape::InternalCall(declid) => {
                trace!("Called Internal call with {declid}");
                out.extend(c_bites);
                // add a space after "external def", etc
                out.extend(b" ");
            }
            FlatShape::External
            | FlatShape::ExternalArg
            | FlatShape::Signature
            | FlatShape::Keyword
            | FlatShape::Block => {
                out.extend(c_bites);
                out.extend(b" ");
            }
            FlatShape::VarDecl(varid) | FlatShape::Variable(varid) => {
                trace!("Called variable or vardecl with {varid}");
                out.extend(c_bites);
                out.extend(b" ");
            }
            FlatShape::Garbage => {
                error!("found garbage 😢 {content}");
                out.extend(c_bites);
                out = insert_newline(out);
            }

            _ => out.extend(c_bites),
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

            if remaining_contents.contains(&b'#') {
                trace!("This have a comment. Writing.");
                out.push(b'\n');
                out.extend(trim_ascii_whitespace(remaining_contents));
            } else {
                trace!("The contents doesn't have a '#'. Skipping.");
            }
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
    let Some(from) = x.iter().position(|x| !x.is_ascii_whitespace()) else { return &x[0..0] };
    let to = x.iter().rposition(|x| !x.is_ascii_whitespace()).unwrap();
    &x[from..=to]
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
