//! This is the nufmt binary documentation
//!
//! # Usage
//!
//! `nufmt` is inteded to be used as this:
//!
//! To format a single file
//! ```shell
//! nufmt file1.nu
//! ```
//!
//! `TODO!`
//!
//! Set options file
//! ```shell
//! nufmt <file> --config nufmt.nuon
//! ```

// for debug purposes, allow unused imports and variables
#[allow(unused)]
#[allow(unused_imports)]
#[allow(unused_import_braces)]
use anyhow::Result;
use clap::Parser;
use env_logger;
use nufmt::config::Config;
use nufmt::{Input, Session};
use std::error::Error;
use std::io::{stdout, Write};
use std::path::PathBuf;

#[derive(Parser)]
#[command(author, version, about)]
struct Cli {
    files: Vec<PathBuf>,
    #[arg(short, long)]
    config: Option<PathBuf>,
}

fn main() -> Result<(), Box<dyn Error>> {
    // set up logger
    env_logger::init();

    let cli = Cli::parse();

    let exit_code = match execute(cli.files, Config::default()) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("{:#}", e);
            1
        }
    };
    // Make sure standard output is flushed before we exit.
    std::io::stdout().flush().unwrap();

    // Exit with given exit code.
    //
    // NOTE: this immediately terminates the process without doing any cleanup,
    // so make sure to finish all necessary cleanup before this is called.
    std::process::exit(exit_code);
}

/// sends the files to format in lib.rs
fn execute(files: Vec<PathBuf>, mut options: Config) -> Result<i32> {
    options = Config::default();

    // open a session
    let out = &mut stdout();
    let mut session = Session::new(options, Some(out));

    for file in files {
        // TODO: this would be a great place to create an enum like
        // enum
        // enum File {
        //     stdin,
        //     single_file,
        //     folder,
        //     _mod,
        // }
        if !file.exists() {
            eprintln!("Error: {} not found!", file.to_str().unwrap());
            session.add_operational_error()
        } else if file.is_dir() {
            // TODO: recursive search
            eprintln!(
                "Error: {} is a directory. Please pass files only.",
                file.to_str().unwrap()
            );
            session.add_operational_error()
        } else {
            // send the file to lib.rs
            println!("formatting file: {:?}", file);
            format_and_emit_report(&mut session, Input::File(file));
        }
    }

    let exit_code = if session.has_operational_errors() {
        1
    } else {
        0
    };

    Ok(exit_code)
}

fn format_and_emit_report<T: Write>(session: &mut Session<'_, T>, input: Input) {
    match session.format(input) {
        _ => todo!("Here `nufmt` gives you a FormatReport"),
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn todo() {
        todo!("First fix the library fixes, then we can do the binary tests.")
    }
}
