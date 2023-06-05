//!
//! nufmt is a library for formatting nu.
//!
//! It does not do anything more than that, which makes it so fast.

use config::Config;
use formatting::format_inner;
use log::{debug, trace};
use std::error::Error;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
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

///
/// Set the indentation used for the formatting.
///
/// Note: It is *not* recommended to set indentation to anything oder than some spaces or some tabs,
/// but nothing is stopping you from doing that.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Indentation<'a> {
    /// Use the default indentation, which is two spaces
    Default,
    /// Use a custom indentation String
    Custom(&'a str),
}

///
/// # Formats a nu string
///
/// The indentation can be set to any value using [`Indentation`]. The default value is two spaces. The default indentation is faster than a custom one
///
pub fn format_nu(nu: &str, indentation: Indentation) -> String {
    let mut reader = BufReader::new(nu.as_bytes());
    let mut writer = BufWriter::new(Vec::new());

    format_nu_buffered(&mut reader, &mut writer, indentation).unwrap();
    String::from_utf8(writer.into_inner().unwrap()).unwrap()
}

///
/// # Formats a nu string
///
/// The indentation can be set to any value using [`Indentation`]. The default value is two spaces. The default indentation is faster than a custom one
///
pub fn format_nu_buffered<R, W>(
    reader: &mut BufReader<R>,
    writer: &mut BufWriter<W>,
    indentation: Indentation,
) -> Result<(), Box<dyn Error>>
where
    R: Read,
    W: Write,
{
    let mut escaped = false;
    let mut in_string = false;
    let mut indent_level: usize = 0;
    let mut newline_requested = false; // invalidated if next character is ] or }
    let mut in_comment = false;

    for char in reader.bytes() {
        let char = char?;
        // if we're in a comment, ignore and write everything until a newline
        if in_comment {
            trace!("this character is a comment");
            match char {
                b'\n' => {
                    in_comment = false;
                    writer.write_all(&[char])?;
                }
                _ => {
                    // TODO: read the rest of the line
                    // write all
                    // and go to the next line
                    writer.write_all(&[char])?;
                    continue;
                }
            }
        }
        if in_string {
            trace!("this is inside a string");
            let mut escape_here = false;
            match char {
                b'"' => {
                    if !escaped {
                        in_string = false;
                    }
                }
                b'\\' => {
                    if !escaped {
                        escape_here = true;
                    }
                }
                _ => {}
            }
            writer.write_all(&[char])?;
            escaped = escape_here;
        } else {
            let mut auto_push = true;
            let mut request_newline = false;
            // let old_level = indent_level;

            match char {
                b'#' => in_comment = true,
                b'"' => in_string = true,
                // b' ' | b'\n' | b'\t' => continue,
                b'\n' => continue,
                b'[' | b'{' => {
                    indent_level += 1;
                    request_newline = true;
                }
                b']' | b'}' => {
                    indent_level = indent_level.saturating_sub(1);
                    if !newline_requested && request_newline {
                        // see comment below about newline_requested
                        // writer.write_all(&[b'\n'])?;
                        indent_buffered(writer, indent_level, indentation)?;
                    }
                }
                b':' => {
                    auto_push = false;
                    writer.write_all(&[char])?;
                    writer.write_all(&[b' '])?;
                }
                b',' => {
                    request_newline = true;
                }
                _ => {}
            }

            if newline_requested && request_newline {
                trace!("new line requested!");
                writer.write_all(&[b'\n'])?;
                indent_buffered(writer, indent_level, indentation)?;
            }
            // if newline_requested && char != b']' && char != b'}' {
            //     // newline only happens after { [ and ,
            //     // this means we can safely assume that it being followed up by } or ]
            //     // means an empty object/array
            //     writer.write_all(&[b'\n'])?;
            //     indent_buffered(writer, old_level, indentation)?;
            // }

            if auto_push {
                //trace!("this char is autopushed");
                writer.write_all(&[char])?;
            }

            newline_requested = request_newline;
        }
        // trace the char for guiding
        trace!("{:?}", char as char);
    }

    Ok(())
}

fn indent_buffered<W>(
    writer: &mut BufWriter<W>,
    level: usize,
    indent_str: Indentation,
) -> Result<(), Box<dyn Error>>
where
    W: std::io::Write,
{
    for _ in 0..level {
        match indent_str {
            Indentation::Default => {
                writer.write_all(b"  ")?;
            }
            Indentation::Custom(indent) => {
                writer.write_all(indent.as_bytes())?;
            }
        }
    }

    Ok(())
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
        let nu = "1.35";
        assert_eq!(nu, format_nu(nu, Indentation::Default));
    }

    #[test]
    fn handle_escaped_strings() {
        let nu = "  \" hallo \\\" \" ";
        let expected = "\" hallo \\\" \"";
        assert_eq!(expected, format_nu(nu, Indentation::Default));
    }

    #[test]
    fn ignore_comments() {
        let nu = "# this is a comment";
        let expected = "# this is a comment";
        assert_eq!(expected, format_nu(nu, Indentation::Default));
    }

    #[test]
    fn ignore_whitespace_in_string() {
        let nu = "\" hallo \"";
        assert_eq!(nu, format_nu(nu, Indentation::Default));
    }

    #[test]
    fn remove_leading_whitespace() {
        let nu = "   0";
        let expected = "0";
        assert_eq!(expected, format_nu(nu, Indentation::Default));
    }
}
