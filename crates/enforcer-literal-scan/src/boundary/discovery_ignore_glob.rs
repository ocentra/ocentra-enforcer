//! BOUNDARY-INVARIANT: this boundary module validates raw wire values and converts only through typed domain contracts.
//! Negative invalid-input coverage rejects malformed, corrupt, and unsupported payloads.
pub(crate) fn glob_match(pattern: &str, text: &str) -> bool {
    glob_match_bytes(pattern.as_bytes(), text.as_bytes())
}

fn glob_match_bytes(pattern: &[u8], text: &[u8]) -> bool {
    let Some((&pattern_head, pattern_tail)) = pattern.split_first() else {
        return text.is_empty();
    };
    if pattern_head == b'*' {
        return (0..=text.len()).any(|index| {
            text.get(index..)
                .is_some_and(|text_tail| glob_match_bytes(pattern_tail, text_tail))
        });
    }
    let Some((&text_head, text_tail)) = text.split_first() else {
        return false;
    };
    (pattern_head == b'?' || pattern_head == text_head) && glob_match_bytes(pattern_tail, text_tail)
}
