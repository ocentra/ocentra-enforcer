use std::io::{self, Write};

use enforcer_events::compatibility::EventCompatibilityMatrix;

mod support;
use support::ExampleError;

fn main() -> Result<(), ExampleError> {
    let matrix = EventCompatibilityMatrix::ocentra_games_lineage()?;
    let markdown = matrix.render_markdown()?;
    io::stdout()
        .lock()
        .write_all(markdown.as_str().as_bytes())?;
    Ok(())
}
