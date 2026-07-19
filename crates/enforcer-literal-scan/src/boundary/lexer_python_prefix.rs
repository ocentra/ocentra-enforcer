pub(crate) fn python_string_prefix_len(rest: &str) -> usize {
    let mut len = 0usize;
    for ch in rest.chars().take(3) {
        if matches!(ch, 'r' | 'R' | 'u' | 'U' | 'b' | 'B' | 'f' | 'F') {
            len += ch.len_utf8();
        } else {
            break;
        }
    }
    let next = rest.get(len..).and_then(|suffix| suffix.chars().next());
    if matches!(next, Some('"') | Some('\'')) {
        len
    } else {
        0
    }
}

pub(crate) fn prefix_has_f(prefix: &str) -> bool {
    prefix.chars().any(|c| c == 'f' || c == 'F')
}
