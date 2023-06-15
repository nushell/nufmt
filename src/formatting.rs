//! Formatting module
//!
//! In this module occurs most of the magic in `nufmt`.
//! It has functions to format slice of bytes and some help functions to separate concerns while doing the job.
//!
use crate::config::Config;
use log::{info, trace};
use nu_parser::{flatten_block, parse, FlatShape};
use nu_protocol::{
    ast::Block,
    engine::{self, StateWorkingSet},
    Span,
};

/// Format an array of bytes
///
/// Reading the file gives you a list of bytes
pub fn format_inner(contents: &[u8], _config: &Config) -> Vec<u8> {
    // nice place to measure formatting time
    // let mut timer = Timer::start();

    // parsing starts
    let engine_state = engine::EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    let parsed_block = parse(&mut working_set, None, contents, false);
    trace!("parsed block:\n{:?}", &parsed_block);

    // check if the block has at least 1 pipeline
    if !block_has_pipelines(&parsed_block) {
        trace!("block has no pipelines!");
        info!("File has no code to format.");
        return contents.to_vec();
    }
    // flat is a list of (Span , Flatshape)
    //
    // Span is the piece of code. You can stringfy the contents.
    // Flatshape is an enum of the type of token read by the AST.
    let flat = flatten_block(&working_set, &parsed_block);
    trace!("flattened block:\n{:?}", &flat);
    // timer = timer.done_parsing()

    // formatting starts
    let mut out: Vec<u8> = vec![];

    let mut start = 0;
    let end_of_file = contents.len();

    for (span, shape) in flat.clone() {
        // check if span skipped some bytes before the current span
        if span.start > start {
            trace!(
                "Span didn't started on the beginning! span {0}, start: {1}",
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

        // get the span contents and format it
        let mut c_bites = working_set.get_span_contents(span);
        let content = String::from_utf8_lossy(c_bites).to_string();
        trace!("shape is {shape}");
        trace!("shape contents: {:?}", &content);
        match shape {
            // if its one of these types, just do nothing. Write it away.
            FlatShape::String | FlatShape::Int | FlatShape::Nothing => out.extend(c_bites),
            FlatShape::List | FlatShape::Record => {
                c_bites = trim_ascii_whitespace(c_bites);
                let printable = String::from_utf8_lossy(c_bites).to_string();
                trace!("stripped the whitespace, result: {:?}", printable);
                out.extend(c_bites);
            }
            FlatShape::Pipe => {
                // here you don't have to strip the whitespace.
                // The pipe is just a pipe `|`.
                //
                // return the pipe AND a space after that
                out.extend("| ".to_string().bytes());
            }
            FlatShape::External => {
                // External are some key commands
                //
                // List of what I've found: seq, each, str,
                out.extend(c_bites);
                // It doen't have a space after it. You have to add it here.
                out.extend([b' '].iter());
            }
            FlatShape::ExternalArg => {
                // This shape is the argument of an External command (see previous case).
                //
                // As a result, ExternalArg may be an entire expression.
                // like: "{ |row|\r\n    let row_data = (seq ... r\n}"
                out.extend(c_bites);
                // It doen't have a space after it. You have to add it here.
                out.extend([b' '].iter());
            }
            FlatShape::Garbage => {
                // Garbage is not garbage at all
                //
                // IDK what is it. I groups a bunch of commands like let my_var = 3
                out.extend(c_bites);
                out = insert_newline(out);
            }

            _ => out.extend(c_bites),
        }

        // check if span skipped some bytes between the final spann and the end of file
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

        // cleanup
        start = span.end + 1;
    }
    // just before writing, check if a new line is needed.
    out = add_newline_at_end_of_file(out);

    // timer = timer.done_formatting()
    out
}

/// A wrapper to insert a new line
///
/// It is used frequently in `nufmt`, so
/// we have a wrapper to improve readability of the code.
fn insert_newline(mut bytes: Vec<u8>) -> Vec<u8> {
    // If I need cfg windows, then I need \r\n
    // let newline = vec![b'\r', b'\n'];
    let newline = vec![b'\n'];
    bytes.extend(newline.iter());
    bytes
}

/// Checks if it missing a new line. If true, adds it.
fn add_newline_at_end_of_file(out: Vec<u8>) -> Vec<u8> {
    match out.last() {
        Some(&b'\n') => out,
        _ => insert_newline(out),
    }
}

/// Given a slice of bytes, strip all spaces, new lines and tabs found within
///
/// Because you don't know how the incoming code is formatted,
/// the best way to format is to strip all the whitespace
/// and afterwards include the new lines and indentation correctly
/// according to the configuration
pub fn trim_ascii_whitespace(x: &[u8]) -> &[u8] {
    let Some(from) = x.iter().position(|x| !x.is_ascii_whitespace()) else { return &x[0..0] };
    let to = x.iter().rposition(|x| !x.is_ascii_whitespace()).unwrap();
    &x[from..=to]
}

/// Returns true if the Block has at least 1 Pipeline
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

/// Returns true if the `Span` is the last Span in the slice of `flat`
fn is_last_span(span: Span, flat: &[(Span, FlatShape)]) -> bool {
    span == flat.last().unwrap().0
}
