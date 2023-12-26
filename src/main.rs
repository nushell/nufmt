#![doc = include_str!("../README.md")]

use clap::Parser;
use clap_stdin::MaybeStdin;
use log::{error, info, trace};
use nu_formatter::config::Config;
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

enum ExitCode {
    Success,
    Failure,
}

/// the CLI signature of the `nufmt` executable.
#[derive(Parser)]
#[command(author, version, about)]
struct Cli {
    #[arg(
        required_unless_present("stdin"),
        help = "one of more Nushell files/folders you want to format"
    )]
    files: Vec<PathBuf>,
    #[clap(default_value = "-")]
    #[arg(
        short,
        long,
        conflicts_with = "files",
        help = "a string of Nushell directly given to the formatter"
    )]
    stdin: MaybeStdin<String>,
    #[arg(short, long, help = "the configuration file")]
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
        [] => format_string(cli.stdin, &cli_config),
        _ => format_files(cli.files, &cli_config),
    };

    std::io::stdout().flush().unwrap();

    exit_with_code(exit_code);
}

/// format a string passed via stdin and output it directly to stdout
fn format_string(string: MaybeStdin<String>, options: &Config) -> ExitCode {
    let output = nu_formatter::format_string(&string, options);
    println!("output: \n{output}");

    ExitCode::Success
}

/// format a list of files, possibly one, and modify them inplace
fn format_files(files: Vec<PathBuf>, options: &Config) -> ExitCode {
    for file in &files {
        if !file.exists() {
            error!("Error: {} not found!", file.to_str().unwrap());
            return ExitCode::Failure;
        } else if file.is_dir() {
            for path in recurse_files(file).unwrap() {
                if is_file_extension(&path, ".nu") {
                    info!("formatting file: {:?}", &path);
                    nu_formatter::format_single_file(&path, options);
                } else {
                    info!("not nu file: skipping");
                }
            }
            // Files only
        } else {
            info!("formatting file: {:?}", file);
            nu_formatter::format_single_file(file, options);
        }
    }

    ExitCode::Success
}

fn recurse_files(path: impl AsRef<Path>) -> std::io::Result<Vec<PathBuf>> {
    let mut buf = vec![];
    let entries = fs::read_dir(path)?;

    for entry in entries {
        let entry = entry?;
        let meta = entry.metadata()?;

        if meta.is_dir() {
            let mut subdir = recurse_files(entry.path())?;
            buf.append(&mut subdir);
        }

        if meta.is_file() {
            buf.push(entry.path());
        }
    }

    Ok(buf)
}

/// Get the file extension
fn is_file_extension(file: &Path, extension: &str) -> bool {
    String::from(file.to_str().unwrap()).ends_with(extension)
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_cmd::Command as AssertCommand;
    use clap::CommandFactory;

    #[test]
    fn clap_cli_construction() {
        Cli::command().debug_assert();
    }

    #[test]
    fn pipe_stdin_to_cli() {
        // TODO: create a file instead of reading one in the repo
        let mut binding = AssertCommand::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
        let result = dbg!(binding
            .arg("-s")
            .arg("-")
            .pipe_stdin("./simplefile.txt")
            .ok());

        result.unwrap().assert().success();
    }
}
