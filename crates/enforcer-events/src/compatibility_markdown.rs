const CELL_ESCAPE_TARGET: &str = "|";
const CELL_ESCAPE_REPLACEMENT: &str = "\\|";

pub(crate) fn escape_cell(value: &str) -> String {
    value.replace(CELL_ESCAPE_TARGET, CELL_ESCAPE_REPLACEMENT)
}
