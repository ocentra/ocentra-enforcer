use std::io::{self, Write};

use enforcer_events::compatibility::EventCompatibilityMatrix;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matrix = EventCompatibilityMatrix::ocentra_games_lineage();
    io::stdout()
        .lock()
        .write_all(matrix.render_markdown().as_bytes())?;
    Ok(())
}
