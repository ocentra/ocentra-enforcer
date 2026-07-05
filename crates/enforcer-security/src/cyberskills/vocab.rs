//! `h03` threat-vocabulary SEED (h11 workpack, "h03 vocab seed"): a Rust
//! frontmatter parser that unions `mitre_attack` + `nist_csf` ids across
//! the vendored `anthropic-cybersecurity-skills` corpus's 817 SKILL.md
//! files, producing the canonical [`ThreatId`] dictionary `h03`'s
//! [`enforcer_security::rules::threat_test_mapping`] validators consume
//! (SEEDS the dictionary — this module does not redefine h03's own
//! enforcement, which stays h03's).
//!
//! # Well-formedness (always enforced, corpus-independent)
//!
//! [`is_known_mitre_attack_id`] and [`is_known_nist_csf_id`] are the
//! practical enforcement predicates [`super::frontmatter_lint`] calls:
//! - ATT&CK: `T\d{4}(\.\d{3})?` (e.g. `T1190`, `T1078.004`).
//! - NIST-CSF: `(GV|ID|PR|DE|RS|RC)\.[A-Z]{2}(-\d{2})?` (e.g. `DE.CM-01`,
//!   `ID.RA`).
//!
//! Format well-formedness alone is checked by these two functions with NO
//! corpus dependency — a syntactically valid id is accepted even if it
//! happens not to appear anywhere in the 817-skill sample, because the
//! MITRE ATT&CK / NIST-CSF namespaces are far larger than what one
//! 817-skill corpus happens to cite (261+ ATT&CK ids sampled is not the
//! full ATT&CK matrix). This mirrors the workpack's own framing: the
//! corpus is a CROSS-CHECK / harvest source for the dictionary, not a
//! closed enumeration of "every id that will ever be valid".
//!
//! # Corpus union (the actual "seed" step)
//!
//! [`union_frontmatter_ids`] performs the seeding step proper: scan a
//! directory of `SKILL.md` files, extract every `mitre_attack`/`nist_csf`
//! id via [`super::frontmatter_lint`]'s frontmatter parser, and return the
//! deduplicated, well-formed union — the concrete [`CorpusVocab`] a caller
//! (h03, or an offline seeding job) persists as the dictionary's initial
//! content. Malformed ids found in the corpus are reported separately
//! (`malformed`) rather than silently dropped, so a corpus quality
//! regression is visible.

use std::collections::BTreeSet;
use std::path::Path;

/// True when `id` is a well-formed MITRE ATT&CK technique id
/// (`T\d{4}(\.\d{3})?`). Hand-written matcher (no regex compilation) since
/// the shape is fixed and simple enough to check character-by-character
/// without a fallible `Regex::new` call in a non-fallible function.
pub fn is_known_mitre_attack_id(id: &str) -> bool {
    let Some(rest) = id.strip_prefix('T') else {
        return false;
    };
    let mut parts = rest.splitn(2, '.');
    let base = parts.next().unwrap_or_default();
    let sub = parts.next();
    let base_ok = base.len() == 4 && base.chars().all(|c| c.is_ascii_digit());
    let sub_ok = sub.is_none_or(|s| s.len() == 3 && s.chars().all(|c| c.is_ascii_digit()));
    base_ok && sub_ok
}

/// True when `id` is a well-formed NIST-CSF subcategory/category id
/// (`(GV|ID|PR|DE|RS|RC)\.[A-Z]{2}(-\d{2})?`). Hand-written matcher for the
/// same reason as [`is_known_mitre_attack_id`].
pub fn is_known_nist_csf_id(id: &str) -> bool {
    const FUNCTIONS: &[&str] = &["GV", "ID", "PR", "DE", "RS", "RC"];
    let Some((function, rest)) = id.split_once('.') else {
        return false;
    };
    if !FUNCTIONS.contains(&function) {
        return false;
    }
    let mut chars = rest.chars();
    let category: String = chars.by_ref().take(2).collect();
    if category.len() != 2 || !category.chars().all(|c| c.is_ascii_uppercase()) {
        return false;
    }
    let remainder: String = chars.collect();
    if remainder.is_empty() {
        return true;
    }
    let Some(suffix) = remainder.strip_prefix('-') else {
        return false;
    };
    suffix.len() == 2 && suffix.chars().all(|c| c.is_ascii_digit())
}

/// The 46-entry canonical subdomain allowlist (`validate-skill.py`'s
/// `ALLOWED_SUBDOMAINS`, canonical forms only — aliases normalize to one
/// of these before membership is checked, mirroring the corpus
/// validator's alias->canonical table without porting the WARN-only alias
/// UX, since this validator's contract is pass/fail, not advisory).
pub const ALLOWED_SUBDOMAINS: &[&str] = &[
    "identity-access-management",
    "zero-trust-architecture",
    "ot-ics-security",
    "soc-operations",
    "red-teaming",
    "web-application-security",
    "network-security",
    "penetration-testing",
    "digital-forensics",
    "malware-analysis",
    "threat-intelligence",
    "cloud-security",
    "container-security",
    "cryptography",
    "vulnerability-management",
    "compliance-governance",
    "devsecops",
    "threat-hunting",
    "incident-response",
    "endpoint-security",
    "phishing-defense",
    "api-security",
    "mobile-security",
    "iot-security",
    "blockchain-security",
    "data-security",
    "email-security",
    "physical-security",
    "supply-chain-security",
    "security-awareness-training",
    "privacy-engineering",
    "secure-coding",
    "security-architecture",
    "risk-management",
    "insider-threat",
    "disaster-recovery",
    "business-continuity",
    "security-automation",
    "ai-security",
    "quantum-security",
    "critical-infrastructure-security",
    "wireless-security",
    "database-security",
    "virtualization-security",
    "firmware-security",
    "social-engineering-defense",
];

/// The deduplicated union of every well-formed `mitre_attack`/`nist_csf`
/// id found across a scanned corpus, plus the malformed ids encountered
/// (surfaced, never silently dropped).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct CorpusVocab {
    /// Every distinct well-formed MITRE ATT&CK id found.
    pub mitre_attack: BTreeSet<String>,
    /// Every distinct well-formed NIST-CSF id found.
    pub nist_csf: BTreeSet<String>,
    /// Every distinct MALFORMED id found (either axis), so a corpus
    /// regression is visible rather than silently dropped.
    pub malformed: BTreeSet<String>,
    /// Count of `SKILL.md` files successfully scanned.
    pub skills_scanned: usize,
}

/// Extract the `mitre_attack:`/`nist_csf:` list items from one SKILL.md's
/// raw text, using the same minimal frontmatter line-scan
/// [`super::frontmatter_lint`] uses (no shared parser struct is exposed
/// there — both call sites independently parse the same simple `key:` /
/// `- item` shape rather than sharing a `pub` type across modules, keeping
/// this module's public surface to the vocab concern only).
fn extract_list(source: &str, key: &str) -> Vec<String> {
    let mut lines = source.lines();
    if lines.next().map(str::trim) != Some("---") {
        return Vec::new();
    }
    let mut items = Vec::new();
    let mut in_target_list = false;
    for line in lines {
        if line.trim() == "---" {
            break;
        }
        if let Some(item) = line.trim_start().strip_prefix("- ") {
            if in_target_list {
                items.push(item.trim().trim_matches(['\'', '"']).to_owned());
            }
            continue;
        }
        if let Some((field, value)) = line.split_once(':') {
            let field = field.trim();
            in_target_list = field == key && value.trim().is_empty();
        }
    }
    items
}

/// Union `mitre_attack`/`nist_csf` ids across every `SKILL.md` found by
/// recursively walking `corpus_skills_dir` (typically
/// `vendor/anthropic-cybersecurity-skills/skills`). I/O errors reading an
/// individual file are skipped (best-effort seeding over whatever files
/// are readable), never a hard failure of the whole scan.
pub fn union_frontmatter_ids(corpus_skills_dir: &Path) -> CorpusVocab {
    let mut vocab = CorpusVocab::default();
    for path in find_skill_md_files(corpus_skills_dir) {
        let Ok(source) = std::fs::read_to_string(&path) else {
            continue;
        };
        vocab.skills_scanned += 1;
        for id in extract_list(&source, "mitre_attack") {
            if is_known_mitre_attack_id(&id) {
                vocab.mitre_attack.insert(id);
            } else {
                vocab.malformed.insert(id);
            }
        }
        for id in extract_list(&source, "nist_csf") {
            if is_known_nist_csf_id(&id) {
                vocab.nist_csf.insert(id);
            } else {
                vocab.malformed.insert(id);
            }
        }
    }
    vocab
}

/// Recursively find every `SKILL.md` under `root`. Pure filesystem walk,
/// no symlink following beyond what `std::fs::read_dir` itself does.
fn find_skill_md_files(root: &Path) -> Vec<std::path::PathBuf> {
    let mut found = Vec::new();
    let Ok(entries) = std::fs::read_dir(root) else {
        return found;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            found.extend(find_skill_md_files(&path));
        } else if path.file_name().and_then(|n| n.to_str()) == Some("SKILL.md") {
            found.push(path);
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::{
        extract_list, is_known_mitre_attack_id, is_known_nist_csf_id, union_frontmatter_ids,
    };
    use std::path::PathBuf;

    #[test]
    fn mitre_attack_id_format_accepts_valid_and_rejects_malformed() {
        assert!(is_known_mitre_attack_id("T1190"));
        assert!(is_known_mitre_attack_id("T1078.004"));
        assert!(!is_known_mitre_attack_id("T99"));
        assert!(!is_known_mitre_attack_id("X1190"));
    }

    #[test]
    fn nist_csf_id_format_accepts_valid_and_rejects_malformed() {
        assert!(is_known_nist_csf_id("DE.CM-01"));
        assert!(is_known_nist_csf_id("ID.RA"));
        assert!(!is_known_nist_csf_id("ZZ.CM-01"));
        assert!(!is_known_nist_csf_id("DE-CM-01"));
    }

    #[test]
    fn extract_list_reads_mitre_attack_block() {
        let source = "---\nmitre_attack:\n- T1190\n- T1078.004\nnist_csf:\n- DE.CM-01\n---\nbody\n";
        assert_eq!(
            extract_list(source, "mitre_attack"),
            vec!["T1190".to_owned(), "T1078.004".to_owned()]
        );
        assert_eq!(
            extract_list(source, "nist_csf"),
            vec!["DE.CM-01".to_owned()]
        );
    }

    /// Repo-level acceptance proof (h11 workpack "h03 vocab seed"): scan
    /// the actual vendored corpus (skipped gracefully if the vendor dir
    /// is absent, per the workpack's L12 vendor-absent honesty
    /// protocol) and assert the union is non-trivial and self-consistent
    /// (skills_scanned > 0 implies at least one id axis populated across
    /// 817 skills).
    #[test]
    fn corpus_union_seeds_a_non_trivial_dictionary_when_vendor_present() {
        let corpus_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../vendor/anthropic-cybersecurity-skills/skills");
        if !corpus_dir.is_dir() {
            // Vendor-absent: nothing to assert (L12 honesty protocol — do
            // not fabricate corpus content).
            return;
        }
        let vocab = union_frontmatter_ids(&corpus_dir);
        assert!(
            vocab.skills_scanned > 0,
            "expected to scan at least one SKILL.md"
        );
        assert!(
            !vocab.mitre_attack.is_empty(),
            "expected at least one well-formed mitre_attack id across the corpus"
        );
        assert!(
            !vocab.nist_csf.is_empty(),
            "expected at least one well-formed nist_csf id across the corpus"
        );
    }
}
