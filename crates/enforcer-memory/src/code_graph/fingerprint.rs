//! Source-body fingerprint construction for graph callables.
//!
//! This boundary owns token normalization, body shingles, and the persisted
//! MinHash evidence used by the graph's similarity projections.

use std::collections::{BTreeSet, HashMap};

use sha2::{Digest, Sha256};

use crate::parsers;

const MINHASH_K: usize = 64;
const MINHASH_MIN_TOKENS: usize = 30;
const MINHASH_HEX_LEN: usize = MINHASH_K * 8;

/// Persisted source/body evidence for callable similarity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceBodyFingerprint {
    pub source_hash: String,
    pub fp: Option<String>,
    pub k: Option<usize>,
    pub body_grams: BTreeSet<String>,
}

pub(crate) fn source_body_fingerprints_for_symbols(
    text: &str,
    symbols: &[parsers::SymbolRef],
) -> HashMap<(String, usize), SourceBodyFingerprint> {
    let mut ordered: Vec<&parsers::SymbolRef> = symbols
        .iter()
        .filter(|symbol| {
            matches!(
                symbol.kind,
                parsers::SymbolKind::Function
                    | parsers::SymbolKind::Method
                    | parsers::SymbolKind::Test
                    | parsers::SymbolKind::Lambda
            )
        })
        .collect();
    ordered.sort_by_key(|symbol| symbol.line);

    let line_offsets = line_start_offsets(text);
    let mut fingerprints = HashMap::new();
    for (index, symbol) in ordered.iter().enumerate() {
        let next_line = ordered.get(index + 1).map(|next| next.line);
        if let Some(fingerprint) =
            source_body_fingerprint(text, &line_offsets, symbol.line, next_line)
        {
            fingerprints.insert((symbol.name.clone(), symbol.line), fingerprint);
        }
    }
    fingerprints
}

pub(crate) fn hash_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    let mut encoded = String::with_capacity(digest.len() * 2);
    for byte in digest {
        encoded.push_str(&format!("{byte:02x}"));
    }
    encoded
}

fn line_start_offsets(text: &str) -> Vec<usize> {
    let mut offsets = vec![0];
    for (index, byte) in text.bytes().enumerate() {
        if byte == b'\n' {
            offsets.push(index + 1);
        }
    }
    offsets
}

fn source_body_fingerprint(
    text: &str,
    line_offsets: &[usize],
    start_line: usize,
    next_line: Option<usize>,
) -> Option<SourceBodyFingerprint> {
    let start = line_offsets.get(start_line.saturating_sub(1)).copied()?;
    let end = next_line
        .and_then(|line| line_offsets.get(line.saturating_sub(1)).copied())
        .unwrap_or(text.len());
    if start >= end || end > text.len() {
        return None;
    }
    let window = text.get(start..end)?;
    let body = braced_body(window).unwrap_or(window);
    let tokens = normalize_source_tokens(body);
    if tokens.is_empty() {
        return None;
    }
    let normalized = tokens.join(" ");
    let body_grams = body_shingles(&normalized);
    if body_grams.is_empty() {
        return None;
    }
    let fingerprint = (tokens.len() >= MINHASH_MIN_TOKENS)
        .then(|| structural_trigrams(&tokens))
        .filter(|trigrams| !trigrams.is_empty())
        .map(|trigrams| minhash_hex(&trigrams));
    let fingerprint_k = fingerprint.as_ref().map(|_| MINHASH_K);
    Some(SourceBodyFingerprint {
        source_hash: hash_bytes(normalized.as_bytes()),
        fp: fingerprint,
        k: fingerprint_k,
        body_grams,
    })
}

fn braced_body(window: &str) -> Option<&str> {
    let open = window.find('{')?;
    let mut depth = 0usize;
    for (offset, character) in window.get(open..)?.char_indices() {
        match character {
            '{' => depth += 1,
            '}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return window.get(open + 1..open + offset);
                }
            }
            _ => {}
        }
    }
    None
}

fn normalize_source_tokens(body: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    for character in body.chars() {
        if character.is_ascii_alphanumeric() || character == '_' {
            current.push(character.to_ascii_lowercase());
        } else if !current.is_empty() {
            tokens.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

fn structural_trigrams(tokens: &[String]) -> Vec<String> {
    tokens
        .windows(3)
        .map(|window| format!("{}|{}|{}", window[0], window[1], window[2]))
        .collect()
}

fn minhash_hex(trigrams: &[String]) -> String {
    let seeds = minhash_seeds();
    let mut minimums = [u32::MAX; MINHASH_K];
    for trigram in trigrams {
        for (index, seed) in seeds.iter().enumerate() {
            let hash = seeded_trigram_hash(trigram.as_bytes(), *seed);
            if hash < minimums[index] {
                minimums[index] = hash;
            }
        }
    }
    let mut encoded = String::with_capacity(MINHASH_HEX_LEN);
    for value in minimums {
        encoded.push_str(&format!("{value:08x}"));
    }
    encoded
}

fn minhash_seeds() -> [u64; MINHASH_K] {
    let mut seeds = [0u64; MINHASH_K];
    let mut state = 0x9e37_79b9_7f4a_7c15u64;
    for seed in &mut seeds {
        state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut mixed = state;
        mixed = (mixed ^ (mixed >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        mixed = (mixed ^ (mixed >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        *seed = mixed ^ (mixed >> 31);
    }
    seeds
}

fn seeded_trigram_hash(bytes: &[u8], seed: u64) -> u32 {
    let mut hash = 0x811c_9dc5u32 ^ (seed as u32);
    for byte in bytes {
        hash ^= u32::from(*byte);
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash ^ ((seed >> 32) as u32)
}

fn body_shingles(normalized: &str) -> BTreeSet<String> {
    let tokens: Vec<&str> = normalized.split_whitespace().collect();
    match tokens.len() {
        0 => BTreeSet::new(),
        1..=4 => [tokens.join(" ")].into_iter().collect(),
        _ => tokens.windows(5).map(|window| window.join(" ")).collect(),
    }
}
