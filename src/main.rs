#![doc = include_str!("../README.md")]

use anyhow::{Ok, Result};
use clap::Parser;
use log::trace;
use nu_formatter::config::Config;
use nu_formatter::{format_single_file, format_string};
use std::error::Error;
use std::io::Write;
use std::path::PathBuf;

/// wrapper to the successful exit code
const SUCCESSFUL_EXIT: i32 = 0;
/// wrapper to the failure exit code
const FAILED_EXIT: i32 = 1;

/// Main CLI struct.
///
/// The derive Clippy API starts from defining the CLI struct
#[derive(Parser)]
#[command(author, version, about)]
struct Cli {
    /// The list of files passed in the cmdline
    /// It is required and it cannot be used with `--stdin`
    #[arg(
        required_unless_present("stdin"),
        help = "The file or files you want to format in nu"
    )]
    files: Vec<PathBuf>,
    /// The string you pass in stdin. You can pass only one string.
    #[arg(
        short,
        long,
        conflicts_with = "files",
        help = "Format the code passed in stdin as a string."
    )]
    stdin: Option<String>,
    /// The optional config file you can pass in the cmdline
    /// You can only pass a file config, not a flag config
    #[arg(short, long, help = "The configuration file")]
    config: Option<PathBuf>,
}

fn main() -> Result<(), Box<dyn Error>> {
    // set up logger
    env_logger::init();

    let cli = Cli::parse();
    trace!("recieved cli.files: {:?}", cli.files);
    trace!("recieved cli.stdin: {:?}", cli.stdin);
    trace!("recieved cli.config: {:?}", cli.config);

    let cli_config = match cli.config {
        None => Config::default(),
        Some(input_cli) => {
            todo!(
                "cannot read from {:?} Reading a config from file not implemented!",
                input_cli
            )
        }
    };

    // Note the deref and reborrow here to obtain a slice
    // so rust doesnt complain for the [] arm
    let exit_code = match &*cli.files {
        // if cli.files is an empty list,
        // it means the flag --stdin was passed
        [] => execute_string(cli.stdin, &cli_config)?,
        _ => execute_files(cli.files, &cli_config)?,
    };

    // Make sure standard output is flushed before we exit.
    std::io::stdout().flush().unwrap();

    trace!("exit code: {exit_code}");
    // Exit with given exit code.
    //
    // NOTE: this immediately terminates the process without doing any cleanup,
    // so make sure to finish all necessary cleanup before this is called.
    std::process::exit(exit_code);
}

/// returns the string formatted to `stdout`
fn execute_string(string: Option<String>, options: &Config) -> Result<i32> {
    // format the string
    let output = format_string(&string.unwrap(), options);
    println!("output: \n{output}");

    Ok(SUCCESSFUL_EXIT)
}

/// Sends the files to format in lib.rs
fn execute_files(files: Vec<PathBuf>, options: &Config) -> Result<i32> {
    // walk the files in the vec of files
    for file in &files {
        if !file.exists() {
            eprintln!("Error: {} not found!", file.to_str().unwrap());
            return Ok(FAILED_EXIT);
        } else if file.is_dir() {
            eprintln!(
                "Error: {} is a directory. Please pass files only.",
                file.to_str().unwrap()
            );
            return Ok(FAILED_EXIT);
        }
        // send the file to lib.rs
        println!("formatting file: {:?}", file);
        format_single_file(file, options);
    }

    Ok(SUCCESSFUL_EXIT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clap_cli_construction() {
        use clap::CommandFactory;
        Cli::command().debug_assert()
    }
}
