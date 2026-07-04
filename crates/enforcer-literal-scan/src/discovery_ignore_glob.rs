pub(crate) fn glob_match(pattern: &str, text: &str) -> bool {
    glob_match_bytes(pattern.as_bytes(), text.as_bytes())
}

fn glob_match_bytes(pattern: &[u8], text: &[u8]) -> bool {
    if pattern.is_empty() {
        return text.is_empty();
    }
    if pattern[0] == b'*' {
        return (0..=text.len()).any(|index| glob_match_bytes(&pattern[1..], &text[index..]));
    }
    if !text.is_empty() && (pattern[0] == b'?' || pattern[0] == text[0]) {
        return glob_match_bytes(&pattern[1..], &text[1..]);
    }
    false
}
