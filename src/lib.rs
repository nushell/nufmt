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

/// Reads the file and format it. After that, writes the file inplace
pub fn format_single_file(file: &PathBuf, config: &Config) {
    // read the contents of the file
    let contents = std::fs::read(file)
        .unwrap_or_else(|_| panic!("something went wrong reading the file {}", file.display()));

    // obtain the formatted file
    let formatted_bytes = format_inner(&contents, config);

    // compare the contents
    if formatted_bytes == contents {
        debug!("File is formatted correctly.")
    }

    // write down the file to path
    let mut writer = File::create(file).unwrap();
    let file_bites = formatted_bytes.as_slice();
    trace!("writing {:?}", formatted_bytes);
    writer
        .write_all(file_bites)
        .expect("something went wrong writing");
    trace!("written")
}

pub fn format_string(input_string: &String, config: &Config) -> String {
    let contents = input_string.as_bytes();
    let formatted_bytes = format_inner(contents, config);
    String::from_utf8(formatted_bytes).unwrap()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn array_of_object() {
        let expected = String::from("[{\"a\":0},{},{\"a\":null}]\n");
        let nu = String::from(
            "[
  {
    \"a\": 0
  },
  {},
  {
    \"a\": null
  }
]",
        );
        assert_eq!(expected, format_string(&nu, &Config::default()));
    }

    #[test]
    fn echoes_primitive() {
        let nu = String::from("1.35\n");
        assert_eq!(nu, format_string(&nu, &Config::default()));
    }

    #[test]
    fn handle_escaped_strings() {
        let nu = String::from("  \" hallo \\\" \" \n");
        let expected = String::from("\" hallo \\\" \"\n");
        assert_eq!(expected, format_string(&nu, &Config::default()));
    }

    #[test]
    #[ignore = "comments aren't a part of Spans,"]
    fn ignore_comments() {
        let nu = String::from("# this is a comment");
        let expected = String::from("# this is a comment");
        assert_eq!(expected, format_string(&nu, &Config::default()));
    }

    #[test]
    fn ignore_whitespace_in_string() {
        let nu = String::from("\" hallo \"\n");
        assert_eq!(nu, format_string(&nu, &Config::default()));
    }

    #[test]
    fn remove_leading_whitespace() {
        let nu = String::from("   0");
        let expected = String::from("0\n");
        assert_eq!(expected, format_string(&nu, &Config::default()));
    }
}
