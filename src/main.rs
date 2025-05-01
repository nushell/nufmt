#![doc = include_str!("../README.md")]

use clap::Parser;
use ignore::{DirEntry, WalkBuilder};
use log::{error, info, trace};
use nu_formatter::config::Config;
use nu_formatter::{CheckOutcome, FormatOutcome};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::{
    io::{self, Write},
    path::{Path, PathBuf},
};

/// The possible exit codes
enum ExitCode {
    /// nufmt terminates successfully, regardless of whether files or stdin were formatted.
    Success,
    /// only used in check mode: nufmt terminates successfully and at least one file or stdin would be formatted if check mode was off.
    CheckFailed,
    /// nufmt terminates abnormally due to invalid configuration, invalid CLI options, or an internal error.
    Failure,
}

impl ExitCode {
    /// Return the exit code to use.
    /// If check mode is off: return 2 if at least one file could not be formatted, 0 otherwise (regardless of whether any files were formatted).
    /// If check mode is on: return 1 if some files would be formatted if check mode was off, 0 otherwise.
    fn code(&self) -> i32 {
        match self {
            ExitCode::Success => 0,
            ExitCode::CheckFailed => 1,
            ExitCode::Failure => 2,
        }
    }
}

/// the CLI signature of the `nufmt` executable.
#[derive(Parser)]
#[command(author, version, about)]
struct Cli {
    #[arg(
        value_name = "FILES",
        default_value = ".",
        conflicts_with("stdin"),
        help = "One of more Nushell files or directories to format"
    )]
    files: Vec<PathBuf>,

    #[arg(
        long,
        conflicts_with = "stdin",
        help = "Avoid writing any formatted files back; instead, exit with a non-zero status code if any files would have been modified, and zero otherwise"
    )]
    check: bool,

    #[arg(
        long,
        conflicts_with = "check",
        conflicts_with = "files",
        help = "A string of Nushell directly given to the formatter"
    )]
    stdin: bool,

    #[arg(short, long, help = "nufmt configuration file")]
    config: Option<PathBuf>,
}

fn exit_with_code(exit_code: ExitCode) {
    let code = exit_code.code();
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

    let config = match cli.config {
        None => Config::default(),
        Some(input_cli) => {
            todo!(
                "cannot read from {:?} Reading a config from file not implemented!",
                input_cli
            )
        }
    };

    let exit_code = match cli.files[..] {
        [] if cli.stdin => {
            let stdin_input = io::stdin().lines().map(|x| x.unwrap()).collect();
            format_string(stdin_input, &config)
        }
        [] if cli.check => {
            let results = check_files(cli.files, &config);
            exit_from_check(&results)
        }
        _ => {
            let results = format_files(cli.files, &config);
            exit_from_format(&results)
        }
    };

    std::io::stdout().flush().unwrap();

    exit_with_code(exit_code);
}

/// format a string passed via stdin and output it directly to stdout
fn format_string(string: String, options: &Config) -> ExitCode {
    let output = nu_formatter::format_string(&string, options);
    println!("{output}");

    ExitCode::Success
}

/// check a list of files, possibly one
fn check_files(files: Vec<PathBuf>, options: &Config) -> Vec<(PathBuf, CheckOutcome)> {
    let target_files = discover_files(&files);
    target_files
        .into_par_iter()
        .map(|file| {
            info!("formatting file: {:?}", &file);
            nu_formatter::check_single_file(file, options)
        })
        .collect()
}

/// format a list of files, possibly one, and modify them in place
fn format_files(files: Vec<PathBuf>, options: &Config) -> Vec<(PathBuf, FormatOutcome)> {
    let target_files = discover_files(&files);
    target_files
        .into_par_iter()
        .map(|file| {
            info!("formatting file: {:?}", &file);
            nu_formatter::format_single_file(file, options)
        })
        .collect()
}

/// Display results and return the appropriate exit code after formatting in check mode
fn exit_from_check(results: &[(PathBuf, CheckOutcome)]) -> ExitCode {
    let mut already_formatted: usize = 0;
    let mut need_formatting: Vec<&PathBuf> = vec![];
    let mut at_least_one_failure = false;

    for (file, result) in results {
        match result {
            CheckOutcome::AlreadyFormatted => already_formatted += 1,
            CheckOutcome::NeedsFormatting => need_formatting.push(file),
            CheckOutcome::Failure(reason) => {
                error!(
                    "\x1b[1mFailed to check {}:\x1b[0m{}",
                    make_relative(file),
                    &reason
                );
                at_least_one_failure = true;
            }
        }
    }

    for file in &need_formatting {
        let file_rel = make_relative(file);
        println!("Would reformat: \x1b[1m{}\x1b[0m", file_rel);
    }

    let need_formatting_count = need_formatting.len();
    if need_formatting_count > 0 {
        println!(
            "{} file{} would be reformatted",
            need_formatting_count,
            if need_formatting_count == 1 { "" } else { "s" }
        );
    }

    if already_formatted > 0 {
        println!(
            "{} file{} already formatted",
            already_formatted,
            if already_formatted == 1 { "" } else { "s" }
        );
    };

    if at_least_one_failure {
        ExitCode::Failure
    } else if need_formatting_count > 0 {
        ExitCode::CheckFailed
    } else {
        ExitCode::Success
    }
}

/// Display results and return the appropriate exit code after formatting in format mode
fn exit_from_format(results: &[(PathBuf, FormatOutcome)]) -> ExitCode {
    let mut left_unchanged: usize = 0;
    let mut reformatted: usize = 0;
    let mut at_least_one_failure = false;

    for (file, result) in results {
        match result {
            FormatOutcome::AlreadyFormatted => left_unchanged += 1,
            FormatOutcome::Reformatted => reformatted += 1,
            FormatOutcome::Failure(reason) => {
                error!(
                    "\x1b[1mFailed to format {}:\x1b[0m{}",
                    make_relative(file),
                    &reason
                );
                at_least_one_failure = true;
            }
        }
    }

    if reformatted > 0 {
        println!(
            "{} file{} were reformatted",
            reformatted,
            if reformatted == 1 { "" } else { "s" }
        );
    }

    if left_unchanged > 0 {
        println!(
            "{} file{} already formatted",
            left_unchanged,
            if left_unchanged == 1 { "" } else { "s" }
        );
    };

    if at_least_one_failure {
        ExitCode::Failure
    } else {
        ExitCode::Success
    }
}

/// Return the different files to analyze, taking only files with .nu extension and discarding files in .nufmtignore
fn discover_files(paths: &[PathBuf]) -> Vec<PathBuf> {
    paths
        .iter()
        .flat_map(|path| {
            WalkBuilder::new(path)
                .add_custom_ignore_filename(".nufmtignore")
                .build()
                .filter_map(Result::ok)
                .filter(is_nu_file)
                .map(|path| path.into_path())
                .collect::<Vec<PathBuf>>()
        })
        .collect()
}

/// Return whether a DirEntry is a .nu file or not
fn is_nu_file(entry: &DirEntry) -> bool {
    entry.file_type().map(|ft| ft.is_file()).unwrap_or(false)
        && entry.path().extension().is_some_and(|ext| ext == "nu")
}

fn make_relative(path: &Path) -> String {
    let current = std::env::current_dir().expect("Failed to get current directory");
    path.strip_prefix(&current)
        .unwrap_or(path)
        .display()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clap_cli_construction() {
        use clap::CommandFactory;
        Cli::command().debug_assert();
    }
}
