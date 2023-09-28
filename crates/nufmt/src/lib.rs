use std::io::Write;
use std::{fs::File, path::PathBuf};

use log::{debug, error};
use log::{info, trace};

use config::Config;
use format::{add_newline_at_end_of_file, format_inner};
use utils::{is_file_extension, recurse_directory};

/// format a Nushell file inplace
pub fn format_file_inplace(file: &PathBuf, config: &Config) {
    let contents = std::fs::read(file)
        .unwrap_or_else(|_| panic!("something went wrong reading the file {}", file.display()));

    let formatted_bytes = add_newline_at_end_of_file(format_inner(&contents, config));

    if formatted_bytes == contents {
        debug!("File is already formatted correctly.");
    }

    let mut writer = File::create(file).unwrap();
    let file_bytes = formatted_bytes.as_slice();
    writer
        .write_all(file_bytes)
        .expect("something went wrong writing");
    trace!("written");
}

/// format a list of files, possibly one, and modify them inplace
pub fn format_directory(files: Vec<PathBuf>, options: &Config) {
    for file in &files {
        if !file.exists() {
            error!("Error: {} not found!", file.to_str().unwrap());
        } else if file.is_dir() {
            for path in recurse_directory(file).unwrap() {
                if is_file_extension(&path, ".nu") {
                    info!("formatting file: {:?}", &path);
                    format_file_inplace(&path, options);
                } else {
                    info!("not nu file: skipping");
                }
            }
            // Files only
        } else {
            info!("formatting file: {:?}", file);
            format_file_inplace(file, options);
        }
    }
}

/// format a string of Nushell code
pub fn format_string(input_string: &String, config: &Config) -> String {
    let contents = input_string.as_bytes();
    let formatted_bytes = format_inner(contents, config);
    String::from_utf8(formatted_bytes).unwrap()
}

pub mod config;
pub mod format;
pub mod utils;
