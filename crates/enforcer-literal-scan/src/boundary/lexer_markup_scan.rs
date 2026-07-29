//! BOUNDARY-INVARIANT: this boundary module validates raw wire values and converts only through typed domain contracts.
//! Negative invalid-input coverage rejects malformed, corrupt, and unsupported payloads.
use crate::lexer_markup_attr::consume_attribute_literal;
use crate::LiteralCandidate;
use enforcer_domain::scan_types::LiteralSourceLine;

pub(crate) fn lex_markup(source: &str) -> Vec<LiteralCandidate> {
    let mut out = Vec::new();
    for (line_index, line) in source.lines().enumerate() {
        out.extend(collect_line_literals(line, line_index + 1));
    }
    out
}

fn collect_line_literals(line: &str, line_number: usize) -> Vec<LiteralCandidate> {
    let mut out = Vec::new();
    let mut search = line;
    let mut offset = 0usize;
    while let Some(eq) = search.find('=') {
        let step = next_line_step(line, line_number, search, eq, &mut offset);
        if let Some(literal) = step.literal {
            out.push(literal);
        }
        if step.stop {
            break;
        }
        offset += step.advance;
        let Some(remaining) = search.get(step.advance..) else {
            break;
        };
        search = remaining;
    }
    out
}

struct LineStep {
    literal: Option<LiteralCandidate>,
    advance: usize,
    stop: bool,
}

fn next_line_step(
    line: &str,
    line_number: usize,
    search: &str,
    eq: usize,
    offset: &mut usize,
) -> LineStep {
    let Some((mut literal, advance)) = consume_attribute_literal(line, search, eq, offset) else {
        return LineStep {
            literal: None,
            advance: eq + 1,
            stop: false,
        };
    };
    literal.line = LiteralSourceLine::from_one_based(line_number);
    LineStep {
        literal: Some(literal),
        advance,
        stop: advance >= search.len(),
    }
}
