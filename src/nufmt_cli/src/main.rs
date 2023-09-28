use clap::Parser;

use log::{error, info, trace};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

use nufmt::{config::Config, format_directory, format_string};

use crate::utils::*;

/// the CLI signature of the `nufmt` executable.
#[derive(Parser)]
#[command(author, version, about)]
struct Cli {
    #[arg(
        required_unless_present("stdin"),
        help = "one of more Nushell files you want to format"
    )]
    files: Vec<PathBuf>,
    #[arg(
        short,
        long,
        conflicts_with = "files",
        help = "a string of Nushell directly given to the formatter"
    )]
    stdin: Option<String>,
    #[arg(short, long, help = "the configuration file")]
    config: Option<PathBuf>,
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

    match cli.files[..] {
        [] => {
            format_string(&cli.stdin.unwrap(), &cli_config);
        }
        _ => {
            format_directory(cli.files, &cli_config);
        }
    };

    std::io::stdout().flush().unwrap();
}

mod utils;
