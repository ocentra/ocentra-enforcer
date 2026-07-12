use std::io::{self, Write};

use enforcer_events::compatibility::EventCompatibilityMatrix;

fn main() -> io::Result<()> {
    let matrix = EventCompatibilityMatrix::ocentra_games_lineage();
    io::stdout()
        .lock()
        .write_all(matrix.render_markdown().as_bytes())?;
    Ok(())
}
