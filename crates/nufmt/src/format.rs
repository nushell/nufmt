use crate::config::Config;
use crate::utils::*;

use log::{info, trace};
use nu_parser::{flatten_block, parse, FlatShape};
use nu_protocol::engine::{self, StateWorkingSet};

/// format an array of bytes
///
/// Reading the file gives you a list of bytes
pub(crate) fn format_inner(contents: &[u8], _config: &Config) -> Vec<u8> {
    let engine_state = engine::EngineState::new();
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

        let mut bytes = working_set.get_span_contents(span);
        let content = String::from_utf8_lossy(bytes).to_string();
        trace!("shape is {shape}");
        trace!("shape contents: {:?}", &content);

        match shape {
            FlatShape::String | FlatShape::Int | FlatShape::Nothing => out.extend(bytes),
            FlatShape::List | FlatShape::Record => {
                bytes = trim_ascii_whitespace(bytes);
                let printable = String::from_utf8_lossy(bytes).to_string();
                trace!("stripped the whitespace, result: {:?}", printable);
                out.extend(bytes);
            }
            FlatShape::Pipe => {
                out.extend(b"| ");
            }
            FlatShape::External | FlatShape::ExternalArg => {
                out.extend(bytes);
                out.extend(b" ");
            }
            FlatShape::Garbage => {
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
    let Some(from) = x.iter().position(|x| !x.is_ascii_whitespace()) else {
        return &x[0..0];
    };
    let to = x.iter().rposition(|x| !x.is_ascii_whitespace()).unwrap();
    &x[from..=to]
}
