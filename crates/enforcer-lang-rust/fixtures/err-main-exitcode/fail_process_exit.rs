// FAIL fixture for RUST-ERR-MAIN-EXITCODE: scattered `std::process::exit`
// instead of `main` returning `ExitCode`/`anyhow::Result<()>`.
fn main() {
    if run().is_err() {
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    Ok(())
}
