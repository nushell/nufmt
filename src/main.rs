use clap::clap_app;
use nufmt::{format_nu_buffered, Indentation};
use std::error::Error;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};

fn main() -> Result<(), Box<dyn Error>> {
    let matches = clap_app!(nufmt =>
        (version: "1.1")
        (author: "fdncred")
        (about: "Formats nu from stdin or from a file")
        (@arg stdout: -s --stdout "Output the result to stdout instead of the default output file. Windows only.")
        (@arg indentation: -i --indent +takes_value "Set the indentation used (\\s for space, \\t for tab)")
        (@arg output: -o --output +takes_value "The output file for the formatted nu")
        (@arg input: "The input file to format")
    )
    .get_matches();

    // Note: on-stack dynamic dispatch
    let (mut file, mut stdin);
    let reader: &mut dyn Read = match matches.value_of("input") {
        Some(path) => {
            file = File::open(path)?;
            &mut file
        }
        None => {
            stdin = std::io::stdin();
            &mut stdin
        }
    };

    let replaced_indent = matches.value_of("indentation").map(|value| {
        value
            .to_lowercase()
            .chars()
            .filter(|c| ['s', 't'].contains(c))
            .collect::<String>()
            .replace("s", " ")
            .replace("t", "\t")
    });

    let indent = match replaced_indent {
        Some(ref str) => Indentation::Custom(str),
        None => Indentation::Default,
    };

    let mut output = matches.value_of("output");
    let mut windows_output_default_file: Option<String> = None;

    #[cfg(windows)]
    if !matches.is_present("stdout") {
        if let Some(file) = matches.value_of("input") {
            // on windows, set the default output file if no stdout flag is provided
            // this makes it work with drag and drop in windows explorer
            windows_output_default_file = Some(file.replace(".nu", "_f.nu"))
        }
    }

    output = windows_output_default_file.as_deref().or(output);

    // Note: on-stack dynamic dispatch
    let (mut file, mut stdout);
    let writer: &mut dyn Write = match output {
        Some(filename) => {
            file = File::create(filename)?;
            &mut file
        }
        None => {
            stdout = std::io::stdout();
            &mut stdout
        }
    };

    let mut reader = BufReader::new(reader);
    let mut writer = BufWriter::new(writer);
    format_nu_buffered(&mut reader, &mut writer, indent)?;

    Ok(())
}
