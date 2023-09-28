use log::trace;

pub(crate) fn exit_with_code(exit_code: ExitCode) {
    let code = match exit_code {
        ExitCode::Success => 0,
        ExitCode::Failure => 1,
    };
    trace!("exit code: {code}");

    // NOTE: this immediately terminates the process without doing any cleanup,
    // so make sure to finish all necessary cleanup before this is called.
    std::process::exit(code);
}

pub(crate) enum ExitCode {
    Success,
    Failure,
}
