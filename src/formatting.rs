use crate::config::Config;
use log::trace;
use nu_parser::{flatten_block, parse, FlatShape};
use nu_protocol::engine::{self, StateWorkingSet};

// Format an entire crate (or subset of the module tree)
pub fn format_inner(contents: &[u8], _config: &Config) -> Vec<u8> {
    // nice place to measure parsing and formatting time
    // let mut timer = Timer::start();
    // parsing starts

    let engine_state = engine::EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    let parsed_block = parse(&mut working_set, None, contents, false);
    trace!("parsed block:\n{:?}\n", &parsed_block);
    // flat is a list of (Span , Flatshape)
    //
    // Span is the piece of code. You can stringfy the contents.
    // Flatshape is an enum of the type of token read by the AST.
    let flat = flatten_block(&working_set, &parsed_block);
    trace!("flattened block:\n{:#?}\n", &flat);
    // timer = timer.done_parsing()

    // formatting starts
    let mut out: Vec<u8> = vec![];

    for (span, shape) in flat {
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
                out.extend(c_bites)
            }
            FlatShape::Pipe => {
                // here you don't have to strip the whitespace.
                // The pipe is just a pipe `|`.
                //
                // return the pipe AND a space after that
                out.extend("| ".to_string().bytes())
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
    }
    // just before writing, append a new line to the file.
    out = insert_newline(out);

    // timer = timer.done_formatting()
    out
}

fn insert_newline(mut bytes: Vec<u8>) -> Vec<u8> {
    // If I need cfg windows, then I need \r\n
    // let newline = vec![b'\r', b'\n'];
    let newline = vec![b'\n'];
    bytes.extend(newline.iter());
    bytes
}

pub fn trim_ascii_whitespace(x: &[u8]) -> &[u8] {
    let from = match x.iter().position(|x| !x.is_ascii_whitespace()) {
        Some(i) => i,
        None => return &x[0..0],
    };
    let to = x.iter().rposition(|x| !x.is_ascii_whitespace()).unwrap();
    &x[from..=to]
}
