use std::path::Path;

pub(crate) fn json_string(value: impl AsRef<str>) -> String {
    let value = value.as_ref();
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

pub(crate) fn normalize_path(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

pub(crate) fn stable_hash_hex(text: &str) -> String {
    // Deterministic non-cryptographic 128-bit FNV-1a pair. Do not use for security.
    let mut a: u64 = 0xcbf29ce484222325;
    let mut b: u64 = 0x84222325cbf29ce4;
    for byte in text.as_bytes() {
        a ^= u64::from(*byte);
        a = a.wrapping_mul(0x100000001b3);
        b ^= u64::from(*byte).rotate_left(1);
        b = b.wrapping_mul(0x100000001b3);
    }
    format!("{a:016x}{b:016x}")
}
