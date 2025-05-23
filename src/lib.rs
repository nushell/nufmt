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
#[derive(Debug, Clone, Default, PartialEq, Eq)]
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

/// format a Nushell file in place. Do not write in dry-run mode.
#[must_use]
pub fn format_single_file(
    file: PathBuf,
    config: &Config,
    mode: &Mode,
) -> (PathBuf, FileDiagnostic) {
    let contents = match std::fs::read(&file) {
        Ok(content) => content,
        Err(err) => {
            return (file, FileDiagnostic::Failure(err.to_string()));
        }
    };

    let formatted_bytes = add_newline_at_end_of_file(match format_inner(&contents, config) {
        Ok(bytes) => bytes,
        Err(err) => {
            return (file, FileDiagnostic::Failure(err.to_string()));
        }
    });

    if formatted_bytes == contents {
        debug!("File is already formatted correctly.");
        return (file, FileDiagnostic::AlreadyFormatted);
    }

    match mode {
        Mode::DryRun => {
            debug!("File not formatted because running in dry run, but would be reformatted in normal mode.");
        }
        Mode::Normal => {
            let mut writer = match File::create(&file) {
                Ok(file) => file,
                Err(err) => {
                    return (file, FileDiagnostic::Failure(err.to_string()));
                }
            };
            let file_bytes = formatted_bytes.as_slice();
            if let Err(err) = writer.write_all(file_bytes) {
                return (file, FileDiagnostic::Failure(err.to_string()));
            }
            debug!("File formatted.");
        }
    }
    (file, FileDiagnostic::Reformatted)
}

/// format a string of Nushell code
pub fn format_string(input_string: &str, config: &Config) -> Result<String, FormatError> {
    let contents = input_string.as_bytes();
    let formatted_bytes = format_inner(contents, config)?;
    Ok(String::from_utf8(formatted_bytes)
        .expect("Formatted string could not be converted to a UTF-8 string"))
}

#[cfg(test)]
mod test {
    use super::*;

    /// test that
    /// 1. formatting the input gives the expected result
    /// 2. formatting the output of `nufmt` a second time does not change the content
    fn run_test(input: &str, expected: &str) {
        let formatted = format_string(input, &Config::default()).unwrap();

        assert_eq!(expected.to_string(), formatted);
        assert_eq!(
            formatted,
            format_string(&formatted, &Config::default()).unwrap()
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
