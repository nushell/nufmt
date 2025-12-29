//! `nu_formatter` is a library for formatting nu.
//!
//! It does not do anything more than that, which makes it so fast.

use config::Config;
use format_error::FormatError;
use formatting::{add_newline_at_end_of_file, format_inner};
use log::debug;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

pub mod config;
pub mod config_error;
pub mod format_error;
mod formatting;

/// Possible modes the formatter can run on
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Mode {
    #[default]
    Normal,
    DryRun,
}

/// The possible outcome of formatting a file
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileDiagnostic {
    /// File was left unchanged, as it is already correctly formatted
    AlreadyFormatted,
    /// File was formatted successfully
    Reformatted,
    /// An error occurred while trying to access or write to the file
    Failure(String),
}

/// Format a Nushell file in place. Do not write in dry-run mode.
pub fn format_single_file(
    file: PathBuf,
    config: &Config,
    mode: &Mode,
) -> (PathBuf, FileDiagnostic) {
    let contents = match std::fs::read(&file) {
        Ok(content) => content,
        Err(err) => return (file, FileDiagnostic::Failure(err.to_string())),
    };

    let formatted_bytes = match format_inner(&contents, config) {
        Ok(bytes) => add_newline_at_end_of_file(bytes),
        Err(err) => return (file, FileDiagnostic::Failure(err.to_string())),
    };

    if formatted_bytes == contents {
        debug!("File is already formatted correctly.");
        return (file, FileDiagnostic::AlreadyFormatted);
    }

    if *mode == Mode::DryRun {
        debug!("File not formatted because running in dry run, but would be reformatted in normal mode.");
        return (file, FileDiagnostic::Reformatted);
    }

    // Normal mode: write the formatted content
    if let Err(err) = write_file(&file, &formatted_bytes) {
        return (file, FileDiagnostic::Failure(err.to_string()));
    }

    debug!("File formatted.");
    (file, FileDiagnostic::Reformatted)
}

/// Write bytes to a file
fn write_file(path: &PathBuf, contents: &[u8]) -> std::io::Result<()> {
    let mut writer = File::create(path)?;
    writer.write_all(contents)
}

/// Format a string of Nushell code
pub fn format_string(input_string: &str, config: &Config) -> Result<String, FormatError> {
    let contents = input_string.as_bytes();
    let formatted_bytes = format_inner(contents, config)?;
    Ok(String::from_utf8(formatted_bytes)
        .expect("Formatted string could not be converted to a UTF-8 string"))
}

#[cfg(test)]
mod test {
    use super::*;

    /// Test that:
    /// 1. formatting the input gives the expected result
    /// 2. formatting the output of `nufmt` a second time does not change the content (idempotency)
    fn run_test(input: &str, expected: &str) {
        let formatted = format_string(input, &Config::default()).unwrap();

        assert_eq!(expected, formatted);
        assert_eq!(
            formatted,
            format_string(&formatted, &Config::default()).unwrap(),
            "Formatting should be idempotent"
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
        // Formatter produces multiline output for complex lists
        let expected = "[
    {\"a\": 0}
    {}
    {\"a\": null}
]";
        run_test(input, expected);
    }

    #[test]
    fn echoes_primitive() {
        run_test("1.35", "1.35");
    }

    #[test]
    fn handle_escaped_strings() {
        run_test("\"hallo\\\"\"", "\"hallo\\\"\"");
    }

    #[test]
    fn ignore_comments() {
        let input = "# comment
let x = 1";
        let formatted = format_string(input, &Config::default()).unwrap();
        // Verify the comment is preserved and the let statement is formatted
        assert!(formatted.contains("# comment"));
        assert!(formatted.contains("let x ="));
    }

    #[test]
    fn ignore_whitespace_in_string() {
        run_test("\" hallo \"", "\" hallo \"");
    }

    #[test]
    fn remove_additional_lines() {
        run_test("let one = 1\n\n\n", "let one = 1");
    }

    #[test]
    fn remove_leading_whitespace() {
        run_test("   0", "0");
    }
}
