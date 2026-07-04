use crate::LiteralKind;

pub(crate) fn rust_raw_prefix(rest: &str) -> Option<(usize, usize, LiteralKind)> {
    [("br", LiteralKind::Byte), ("r", LiteralKind::Raw)]
        .into_iter()
        .find_map(|(prefix, kind)| parse_raw_prefix(rest, prefix, kind))
}

pub(crate) fn is_rust_lifetime(rest: &str) -> bool {
    let mut chars = rest.chars();
    if chars.next() != Some('\'') {
        return false;
    }
    let Some(start) = chars.next() else {
        return false;
    };
    if !(start == '_' || start.is_ascii_alphabetic()) {
        return false;
    }
    let Some(next) = chars.next() else {
        return false;
    };
    next.is_ascii_alphanumeric() || next == '_'
}

fn parse_raw_prefix(
    rest: &str,
    prefix: &str,
    kind: LiteralKind,
) -> Option<(usize, usize, LiteralKind)> {
    let stripped = rest.strip_prefix(prefix)?;
    let hashes = stripped.chars().take_while(|c| *c == '#').count();
    if stripped.as_bytes().get(hashes) != Some(&b'"') {
        return None;
    }
    Some((prefix.len() + hashes + 1, hashes, kind))
}
