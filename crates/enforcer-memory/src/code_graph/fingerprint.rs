//! Source-body fingerprint construction for graph callables.
//!
//! This boundary owns token normalization, body shingles, and the persisted
//! MinHash evidence used by the graph's similarity projections.

use std::collections::{BTreeSet, HashMap};

use sha2::{Digest, Sha256};

use crate::parsers;
use enforcer_domain::memory_types::{
    ComplexitySourceBytes, GraphSourceLine, MemoryFingerprintBodyGram, MemoryFingerprintHashCount,
    MemoryFingerprintLexeme, MemoryFingerprintLexemes, MemoryFingerprintSourceHash,
    MemoryFingerprintValue, ParsedSymbolName, ParserSourceText, SnippetByteOffset,
};

const MINHASH_K: usize = 64;
const MINHASH_MIN_TOKENS: usize = 30;
const MINHASH_HEX_LEN: usize = MINHASH_K * 8;

/// Persisted source/body evidence for callable similarity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceBodyFingerprint {
    pub source_hash: MemoryFingerprintSourceHash,
    pub fp: Option<MemoryFingerprintValue>,
    pub k: Option<MemoryFingerprintHashCount>,
    pub body_grams: BTreeSet<MemoryFingerprintBodyGram>,
}

pub(crate) fn source_body_fingerprints_for_symbols(
    text: ParserSourceText<'_>,
    symbols: &[parsers::SymbolRef],
) -> HashMap<(ParsedSymbolName, GraphSourceLine), SourceBodyFingerprint> {
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
            // CLONE-JUSTIFICATION: the ordered parser view is borrowed from the
            // source document; the fingerprint map must own each key after this
            // loop returns.
            fingerprints.insert((symbol.name.clone(), symbol.line), fingerprint);
        }
    }
    fingerprints
}

pub(crate) fn hash_bytes(bytes: ComplexitySourceBytes<'_>) -> MemoryFingerprintSourceHash {
    let mut hasher = Sha256::new();
    hasher.update(bytes.as_bytes());
    let digest = hasher.finalize();
    let mut encoded = String::with_capacity(digest.len() * 2);
    for byte in digest {
        encoded.push_str(&format!("{byte:02x}"));
    }
    encoded.into()
}

fn line_start_offsets(text: ParserSourceText<'_>) -> Vec<SnippetByteOffset> {
    let mut offsets = vec![0usize.into()];
    for (index, byte) in text.as_str().bytes().enumerate() {
        if byte == b'\n' {
            offsets.push((index + 1).into());
        }
    }
    offsets
}

fn source_body_fingerprint(
    text: ParserSourceText<'_>,
    line_offsets: &[SnippetByteOffset],
    start_line: GraphSourceLine,
    next_line: Option<GraphSourceLine>,
) -> Option<SourceBodyFingerprint> {
    let start = line_offsets
        .get(start_line.get().saturating_sub(1))
        .copied()?
        .get();
    let end = next_line
        .and_then(|line| line_offsets.get(line.get().saturating_sub(1)).copied())
        .map(SnippetByteOffset::get)
        .unwrap_or(text.as_str().len());
    if start >= end || end > text.as_str().len() {
        return None;
    }
    let window = text.as_str().get(start..end).map(ParserSourceText::from)?;
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
        source_hash: hash_bytes(ComplexitySourceBytes::from(normalized.as_bytes())),
        fp: fingerprint.map(Into::into),
        k: fingerprint_k.map(Into::into),
        body_grams: body_grams.into_iter().map(Into::into).collect(),
    })
}

fn braced_body(window: ParserSourceText<'_>) -> Option<ParserSourceText<'_>> {
    let window = window.as_str();
    let open = window.find('{')?;
    let mut depth = 0usize;
    for (offset, character) in window.get(open..)?.char_indices() {
        match character {
            '{' => depth += 1,
            '}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return window
                        .get(open + 1..open + offset)
                        .map(ParserSourceText::from);
                }
            }
            _ => {}
        }
    }
    None
}

fn normalize_source_tokens(body: ParserSourceText<'_>) -> MemoryFingerprintLexemes {
    let mut tokens = Vec::new();
    let mut current = String::new();
    for character in body.as_str().chars() {
        if character.is_ascii_alphanumeric() || character == '_' {
            current.push(character.to_ascii_lowercase());
        } else if !current.is_empty() {
            tokens.push(MemoryFingerprintLexeme::from(std::mem::take(&mut current)));
        }
    }
    if !current.is_empty() {
        tokens.push(MemoryFingerprintLexeme::from(current));
    }
    tokens.into()
}

fn structural_trigrams(tokens: &[MemoryFingerprintLexeme]) -> Vec<MemoryFingerprintLexeme> {
    tokens
        .windows(3)
        .filter_map(|window| match window {
            [first, second, third] => Some(MemoryFingerprintLexeme::from(format!(
                "{first}|{second}|{third}"
            ))),
            _ => None,
        })
        .collect()
}

fn minhash_hex(trigrams: &[MemoryFingerprintLexeme]) -> String {
    let seeds = minhash_seeds();
    let mut minimums = [u32::MAX; MINHASH_K];
    for trigram in trigrams {
        for (index, seed) in seeds.iter().enumerate() {
            let hash = seeded_trigram_hash(trigram.as_bytes(), *seed);
            if let Some(minimum) = minimums.get_mut(index) {
                if hash < *minimum {
                    *minimum = hash;
                }
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
    let lower_seed = u32::try_from(seed & u64::from(u32::MAX)).unwrap_or(0);
    let mut hash = 0x811c_9dc5u32 ^ lower_seed;
    for byte in bytes {
        hash ^= u32::from(*byte);
        hash = hash.wrapping_mul(0x0100_0193);
    }
    let upper_seed = u32::try_from(seed >> 32).unwrap_or(0);
    hash ^ upper_seed
}

fn body_shingles(normalized: &str) -> BTreeSet<String> {
    let tokens: Vec<&str> = normalized.split_whitespace().collect();
    match tokens.len() {
        0 => BTreeSet::new(),
        1..=4 => [tokens.join(" ")].into_iter().collect(),
        _ => tokens.windows(5).map(|window| window.join(" ")).collect(),
    }
}
