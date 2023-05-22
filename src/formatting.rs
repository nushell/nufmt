use crate::{config::Config, FileName, Input, Session};
use std::io::Write;

impl<'b, T: Write + 'b> Session<'b, T> {
    pub fn format_input_inner(&mut self, input: Input) {
        println!("formatting ...💭");
        let format_result = format_project(input, &self.config);
    }
}

// Format an entire crate (or subset of the module tree)
fn format_project(input: Input, config: &Config) {
    // let mut timer = Timer::start();

    let main_file = input.file_name();
    let input_is_stdin = main_file == FileName::Stdin;

    // parsing starts

    // timer = timer.done_parsing()
    // formatting starts

    // timer = timer.done_formatting()
}
