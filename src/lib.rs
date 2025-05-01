//! `nu_formatter` is a library for formatting nu.
//!
//! It does not do anything more than that, which makes it so fast.
use config::Config;
use formatting::{add_newline_at_end_of_file, format_inner};
use log::debug;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

pub mod config;
mod formatting;

/// The possible outcome of checking a file
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckOutcome {
    /// File is already correctly formatted
    AlreadyFormatted,
    /// File would be reformatted if check mode was off
    NeedsFormatting,
    /// An error occurred while trying to access or write to the file
    Failure(String),
}

/// The possible outcome of checking or formatting a file
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormatOutcome {
    /// File was left unchanged, as it is already correctly formatted
    AlreadyFormatted,
    /// File was formatted successfully
    Reformatted,
    /// An error occurred while trying to access or write to the file
    Failure(String),
}

/// check a Nushell file
pub fn check_single_file(file: PathBuf, config: &Config) -> (PathBuf, CheckOutcome) {
    let contents = match std::fs::read(&file) {
        Ok(content) => content,
        Err(err) => {
            return (file, CheckOutcome::Failure(err.to_string()));
        }
    };

    let formatted_bytes = add_newline_at_end_of_file(format_inner(&contents, config));

    if formatted_bytes == contents {
        (file, CheckOutcome::AlreadyFormatted)
    } else {
        (file, CheckOutcome::NeedsFormatting)
    }
}

/// format a Nushell file in place
pub fn format_single_file(file: PathBuf, config: &Config) -> (PathBuf, FormatOutcome) {
    let contents = match std::fs::read(&file) {
        Ok(content) => content,
        Err(err) => {
            return (file, FormatOutcome::Failure(err.to_string()));
        }
    };

    let formatted_bytes = add_newline_at_end_of_file(format_inner(&contents, config));

    if formatted_bytes == contents {
        debug!("File is already formatted correctly.");
        return (file, FormatOutcome::AlreadyFormatted);
    }

    let mut writer = File::create(&file).unwrap();
    let file_bytes = formatted_bytes.as_slice();
    writer
        .write_all(file_bytes)
        .expect("something went wrong writing");
    debug!("File formatted.");
    (file, FormatOutcome::Reformatted)
}

/// format a string of Nushell code
pub fn format_string(input_string: &String, config: &Config) -> String {
    let contents = input_string.as_bytes();
    let formatted_bytes = format_inner(contents, config);
    String::from_utf8(formatted_bytes).unwrap()
}

#[cfg(test)]
mod test {
    use super::*;

    /// test that
    /// 1. formatting the input gives the expected result
    /// 2. formatting the output of `nufmt` a second time does not change the content
    fn run_test(input: &str, expected: &str) {
        let formatted = format_string(&input.to_string(), &Config::default());

        assert_eq!(expected.to_string(), formatted);
        assert_eq!(formatted, format_string(&formatted, &Config::default()));
    }

    #[test]
    fn array_of_object() {
        let input = "[
  {
    \"a\": 0
  },
  {},
  {
    \"a\": null
  }
]";
        let expected = "[{\"a\":0},{},{\"a\":null}]";
        run_test(input, expected);
    }

    #[test]
    fn echoes_primitive() {
        let input = "1.35";
        let expected = input;
        run_test(input, expected);
    }

    #[test]
    fn handle_escaped_strings() {
        let input = "\"hallo\\\"\"";
        let expected = input;
        run_test(input, expected);
    }

    #[test]
    fn ignore_comments() {
        let input = "# beginning of script comment

let one = 1
def my-func [
    param1:int # inline comment
]{ print(param1) 
}
myfunc(one)





# final comment


";
        let expected = "# beginning of script comment
let one = 1
def my-func [
    param1:int # inline comment
]{ print(param1) 
}
myfunc (one )
# final comment";
        run_test(input, expected);
    }

    #[test]
    fn ignore_whitespace_in_string() {
        let input = "\" hallo \"";
        let expected = input;
        run_test(input, expected);
    }

    #[test]
    fn remove_additional_lines() {
        let input = "let one = 1\n\n\n";
        let expected = "let one = 1";
        run_test(input, expected);
    }

    #[test]
    fn remove_leading_whitespace() {
        let input = "   0";
        let expected = "0";
        run_test(input, expected);
    }
}
