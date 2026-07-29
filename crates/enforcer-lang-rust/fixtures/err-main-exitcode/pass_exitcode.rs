// PASS fixture for RUST-ERR-MAIN-EXITCODE: `main` returns `ExitCode`, no
// scattered `process::exit`.
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(_) => ExitCode::FAILURE,
    }
}

fn run() -> Result<(), String> {
    Ok(())
}
