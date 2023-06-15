#![doc = include_str!("../README.md")]

use clap::Parser;
use log::{error, info, trace};
use nu_formatter::config::Config;
use nu_formatter::{format_single_file, format_string};
use std::io::Write;
use std::path::PathBuf;

/// wrapper to the successful exit code
enum ExitCode {
    Success,
    Failure,
}

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

fn exit_with_code(exit_code: ExitCode) {
    let code = match exit_code {
        ExitCode::Success => 0,
        ExitCode::Failure => 1,
    };
    trace!("exit code: {code}");

    // NOTE: this immediately terminates the process without doing any cleanup,
    // so make sure to finish all necessary cleanup before this is called.
    std::process::exit(code);
}

fn main() {
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

    let exit_code = match cli.files[..] {
        [] => execute_string(cli.stdin, &cli_config),
        _ => execute_files(cli.files, &cli_config),
    };

    // Make sure standard output is flushed before we exit.
    std::io::stdout().flush().unwrap();

    exit_with_code(exit_code);
}

/// format a string passed via stdin and output it directly to stdout
fn execute_string(string: Option<String>, options: &Config) -> ExitCode {
    let output = format_string(&string.unwrap(), options);
    println!("output: \n{output}");

    ExitCode::Success
}

/// Sends the files to format in lib.rs
fn execute_files(files: Vec<PathBuf>, options: &Config) -> ExitCode {
    for file in &files {
        if !file.exists() {
            error!("Error: {} not found!", file.to_str().unwrap());
            return ExitCode::Failure;
        } else if file.is_dir() {
            error!(
                "Error: {} is a directory. Please pass files only.",
                file.to_str().unwrap()
            );
            return ExitCode::Failure;
        }
        // send the file to lib.rs
        info!("formatting file: {:?}", file);
        format_single_file(file, options);
    }

    ExitCode::Success
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
