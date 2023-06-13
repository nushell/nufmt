//!
//! nufmt is a library for formatting nu.
//!
//! It does not do anything more than that, which makes it so fast.

use config::Config;
use formatting::format_inner;
use log::{debug, trace};
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

pub mod config;
pub mod formatting;

/// Reads a file and format it. Then writes the file inplace.
pub fn format_single_file(file: &PathBuf, config: &Config) {
    // read the contents of the file
    let contents = std::fs::read(file)
        .unwrap_or_else(|_| panic!("something went wrong reading the file {}", file.display()));

    // obtain the formatted file
    let formatted_bytes = format_inner(&contents, config);

    // compare the contents
    if formatted_bytes == contents {
        debug!("File is formatted correctly.");
    }

    // write down the file to path
    let mut writer = File::create(file).unwrap();
    let file_bites = formatted_bytes.as_slice();
    writer
        .write_all(file_bites)
        .expect("something went wrong writing");
    trace!("written");
}

/// Take a `String` and format it. Then returns a new `String`
pub fn format_string(input_string: &String, config: &Config) -> String {
    let contents = input_string.as_bytes();
    let formatted_bytes = format_inner(contents, config);
    String::from_utf8(formatted_bytes).unwrap()
}

#[cfg(test)]
mod test {
    use super::*;

    fn run_test(input: &str, expected: &str) {
        assert_eq!(
            expected.to_string(),
            format_string(&input.to_string(), &Config::default())
        );
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
        let expected = "[{\"a\":0},{},{\"a\":null}]\n";
        run_test(input, expected);
    }

    #[test]
    fn echoes_primitive() {
        let input = "1.35";
        let expected = "1.35\n";
        run_test(input, expected);
    }

    #[test]
    fn handle_escaped_strings() {
        let input = "  \"hallo\\\"\"";
        let expected = "\"hallo\\\"\"\n";
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
myfunc(one) 
# final comment\n";
        run_test(input, expected);
    }

    #[test]
    fn ignore_whitespace_in_string() {
        let input = "\" hallo \"";
        let expected = "\" hallo \"\n";
        run_test(input, expected);
    }

    #[test]
    fn add_new_line() {
        let input = "null";
        let expected = "null\n";
        run_test(input, expected);
    }

    #[test]
    fn remove_additional_lines() {
        let input = "let 'one' = 1\n\n\n";
        let expected = "let 'one' = 1\n";
        run_test(input, expected);
    }

    #[test]
    fn remove_leading_whitespace() {
        let input = "   0";
        let expected = "0\n";
        run_test(input, expected);
    }
}
