#![doc = include_str!("../README.md")]
#![deny(warnings, rustdoc::broken_intra_doc_links)]
#![warn(
    clippy::explicit_iter_loop,
    clippy::explicit_into_iter_loop,
    clippy::semicolon_if_nothing_returned,
    clippy::doc_markdown,
    clippy::manual_let_else
)]

use clap::Parser;
use ignore::{overrides::OverrideBuilder, DirEntry, WalkBuilder};
use log::{info, trace};
use nu_ansi_term::{Color, Style};
use nu_formatter::config::Config;
use nu_formatter::config_error::ConfigError;
use nu_formatter::FileDiagnostic;
use nu_formatter::Mode;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::convert::TryFrom;
use std::{
    io::{self, Write},
    path::{Path, PathBuf},
};

const DEFAULT_CONFIG_FILE: &str = "nufmt.nuon";

/// The possible exit codes
#[derive(Debug, Clone, PartialEq, Eq)]
enum ExitCode {
    /// nufmt terminates successfully, regardless of whether files or stdin were formatted.
    Success,
    /// only used in check mode: nufmt terminates successfully and at least one file would be formatted if check mode was off.
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
    dry_run: bool,

    #[arg(
        long,
        conflicts_with = "dry_run",
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

    let config_file = cli.config.or(find_in_parent_dirs(DEFAULT_CONFIG_FILE));
    let config = match config_file {
        None => Config::default(),
        Some(cli_config) => match read_config(&cli_config) {
            Ok(config) => config,
            Err(err) => {
                eprintln!("{}: {}", Color::LightRed.paint("error"), &err);
                return exit_with_code(ExitCode::Failure);
            }
        },
    };

    let exit_code = if cli.stdin {
        let stdin_input: String = io::stdin()
            .lines()
            .map(|x| x.unwrap())
            .collect::<Vec<_>>()
            .join("\n");
        format_string(stdin_input, &config)
    } else {
        let (target_files, invalid_files) = match discover_nu_files(cli.files, &config.excludes) {
            Ok(files) => files,
            Err(err) => {
                eprintln!("{}: {}", Color::LightRed.paint("error"), err);
                return exit_with_code(ExitCode::Failure);
            }
        };
        let mode = if cli.dry_run {
            Mode::DryRun
        } else {
            Mode::default()
        };
        let mut results = handle_invalid_file(invalid_files);
        results.extend(format_files(target_files, &config, &mode));
        display_diagnostic_and_compute_exit_code(&results, cli.dry_run)
    };

    std::io::stdout()
        .flush()
        .expect("Unexpected error occurred when flushing stdout");

    exit_with_code(exit_code);
}

fn read_config(path: &PathBuf) -> Result<Config, ConfigError> {
    let content = std::fs::read_to_string(path)?;
    let content_nuon = nuon::from_nuon(&content, None)?;
    Config::try_from(content_nuon)
}

/// format a string passed via stdin and output it directly to stdout
fn format_string(string: String, options: &Config) -> ExitCode {
    match nu_formatter::format_string(&string, options) {
        Ok(output) => {
            println!("{output}");
            ExitCode::Success
        }
        Err(err) => {
            eprintln!(
                "{}: {}",
                Color::LightRed.paint("Could not format stdin"),
                err
            );
            ExitCode::Failure
        }
    }
}

fn handle_invalid_file(files: Vec<PathBuf>) -> Vec<(PathBuf, FileDiagnostic)> {
    let mut results: Vec<(PathBuf, FileDiagnostic)> = vec![];
    for file in files {
        results.push((
            file,
            FileDiagnostic::Failure("cannot find the file specified".to_string()),
        ));
    }
    results
}

/// format a list of files, possibly one, and modify them in place
/// if check mode is on, only check the files but do not modify them in place
fn format_files(
    files: Vec<PathBuf>,
    options: &Config,
    mode: &Mode,
) -> Vec<(PathBuf, FileDiagnostic)> {
    files
        .into_par_iter()
        .map(|file| {
            info!("formatting file: {:?}", &file);
            nu_formatter::format_single_file(file, options, mode)
        })
        .collect()
}

/// Display results and return the appropriate exit code after formatting in check mode
fn display_diagnostic_and_compute_exit_code(
    results: &[(PathBuf, FileDiagnostic)],
    check_mode: bool,
) -> ExitCode {
    let mut already_formatted: usize = 0;
    let mut reformatted_or_would_reformat: usize = 0;
    let mut failures: usize = 0;
    let mut at_least_one_failure = false;
    let mut warning_messages: Vec<String> = vec![];

    let file_failed_msg = if check_mode {
        "Failed to check"
    } else {
        "Failed to format"
    };

    for (file, result) in results {
        match result {
            FileDiagnostic::AlreadyFormatted => already_formatted += 1,
            FileDiagnostic::Reformatted => {
                reformatted_or_would_reformat += 1;
                if check_mode {
                    warning_messages.push(format!(
                        "Would reformat: {}",
                        Style::new().bold().paint(make_relative(file))
                    ));
                };
            }
            FileDiagnostic::Failure(reason) => {
                failures += 1;
                eprintln!(
                    "{}: {} {}: {}",
                    Color::LightRed.paint("error"),
                    Style::new().bold().paint(file_failed_msg),
                    Style::new().bold().paint(make_relative(file)),
                    &reason
                );
                at_least_one_failure = true;
            }
        }
    }

    for msg in warning_messages {
        println!("{}", msg);
    }

    if already_formatted + reformatted_or_would_reformat + failures == 0 {
        print!(
            "{}: no Nushell files found under the given path(s)",
            Color::LightYellow.paint("warning"),
        );
        return ExitCode::Success;
    }

    if reformatted_or_would_reformat > 0 {
        let msg = if check_mode {
            "would be reformatted"
        } else {
            "were formatted"
        };
        println!(
            "{} file{} {}",
            reformatted_or_would_reformat,
            if reformatted_or_would_reformat == 1 {
                ""
            } else {
                "s"
            },
            msg,
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
    } else if check_mode && reformatted_or_would_reformat > 0 {
        ExitCode::CheckFailed
    } else {
        ExitCode::Success
    }
}

/// Return the different files to analyze, taking only files with .nu extension and discarding files excluded in the config
/// and the invalid paths provided
fn discover_nu_files(
    paths: Vec<PathBuf>,
    excludes: &Vec<String>,
) -> Result<(Vec<PathBuf>, Vec<PathBuf>), ConfigError> {
    let mut valid_paths: Vec<PathBuf> = vec![];
    let mut invalid_paths: Vec<PathBuf> = vec![];

    for path in paths {
        if path.exists() {
            valid_paths.push(path);
        } else {
            invalid_paths.push(path);
        }
    }

    let mut overrides = OverrideBuilder::new(".");
    for pattern in excludes {
        overrides.add(&format!("!{}", pattern))?;
    }
    let overrides = overrides.build()?;

    let nu_files = valid_paths
        .iter()
        .flat_map(|path| {
            WalkBuilder::new(path)
                .overrides(overrides.clone())
                .build()
                .filter_map(Result::ok)
                .filter(is_nu_file)
                .map(|path| path.into_path())
                .collect::<Vec<PathBuf>>()
        })
        .collect();

    Ok((nu_files, invalid_paths))
}

/// Return whether a `DirEntry` is a .nu file or not
fn is_nu_file(entry: &DirEntry) -> bool {
    entry.file_type().map(|ft| ft.is_file()).unwrap_or(false)
        && entry.path().extension().is_some_and(|ext| ext == "nu")
}

fn make_relative(path: &Path) -> String {
    let current = std::env::current_dir().unwrap_or(PathBuf::from("."));
    path.strip_prefix(&current)
        .unwrap_or(path)
        .display()
        .to_string()
        .replace("\\", "/")
        .trim_start_matches("./")
        .to_string()
}

/// Search for `filename` in current or any parent directories.
/// If `start_dir` is not provided, the current directory is used
fn find_in_parent_dirs(filename: &str) -> Option<PathBuf> {
    let start_dir = std::env::current_dir().unwrap_or(PathBuf::from("."));

    let mut dir = Some(start_dir.as_path());
    while let Some(current) = dir {
        let candidate = current.join(filename);
        if candidate.exists() {
            return Some(candidate);
        }
        dir = current.parent();
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn clap_cli_construction() {
        use clap::CommandFactory;
        Cli::command().debug_assert();
    }

    #[test]
    fn read_valid_config_empty() {
        let dir = tempdir().unwrap();
        let config_file = dir.path().join("nufmt.nuon");
        fs::write(&config_file, "").unwrap();

        let config = read_config(&config_file).expect("Config should be valid");
        assert_eq!(config, Config::default());
    }

    #[rstest]
    #[case(r#"{line_length: 120, exclude: ["a*nu", "b*.nu"]}"#)]
    fn read_valid_config(#[case] config_content: &str) {
        let dir = tempdir().unwrap();
        let config_file = dir.path().join("nufmt.nuon");
        fs::write(&config_file, config_content).unwrap();

        let config = read_config(&config_file).expect("Config should be valid");
        assert_eq!(config.line_length, 120_usize);
        assert_eq!(config.excludes.len(), 2_usize);
    }

    #[rstest]
    #[case(r#"some string"#)]
    #[case(r#"{unknown: 1}"#)]
    #[case(r#"{line_length: -1}"#)]
    #[case(r#"{line_length: "120"}"#)]
    #[case(r#"{exclude: "a*nu"}"#)]
    #[case(r#"{exclude: ["a*nu", 1]}"#)]
    fn read_invalid_config(#[case] config_content: &str) {
        let dir = tempdir().unwrap();
        let config_file = dir.path().join("nufmt.nuon");
        fs::write(&config_file, config_content).unwrap();

        let config = read_config(&config_file);
        assert!(config.is_err());
    }

    #[rstest]
    #[case(vec![
        (PathBuf::from("a.nu"), FileDiagnostic::AlreadyFormatted),
        (PathBuf::from("b.nu"), FileDiagnostic::AlreadyFormatted),], false, ExitCode::Success)]
    #[case(vec![
        (PathBuf::from("a.nu"), FileDiagnostic::AlreadyFormatted),
        (PathBuf::from("b.nu"), FileDiagnostic::AlreadyFormatted),], true, ExitCode::Success)]
    #[case(vec![
        (PathBuf::from("a.nu"), FileDiagnostic::AlreadyFormatted),
        (PathBuf::from("b.nu"), FileDiagnostic::Reformatted),], false, ExitCode::Success)]
    #[case(vec![
        (PathBuf::from("a.nu"), FileDiagnostic::AlreadyFormatted),
        (PathBuf::from("b.nu"), FileDiagnostic::Reformatted),], true, ExitCode::CheckFailed)]
    #[case(vec![
        (PathBuf::from("a.nu"), FileDiagnostic::AlreadyFormatted),
        (PathBuf::from("b.nu"), FileDiagnostic::Reformatted),
        (PathBuf::from("c.nu"), FileDiagnostic::Failure("some error".to_string())),], false, ExitCode::Failure)]
    #[case(vec![
        (PathBuf::from("a.nu"), FileDiagnostic::AlreadyFormatted),
        (PathBuf::from("b.nu"), FileDiagnostic::Reformatted),
        (PathBuf::from("c.nu"), FileDiagnostic::Failure("some error".to_string())),], true, ExitCode::Failure)]
    fn exit_code(
        #[case] results: Vec<(PathBuf, FileDiagnostic)>,
        #[case] check_mode: bool,
        #[case] expected: ExitCode,
    ) {
        let exit_code = display_diagnostic_and_compute_exit_code(&results, check_mode);
        assert_eq!(exit_code, expected);
    }
}
