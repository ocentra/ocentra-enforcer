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

/// The subdomain allowlist, ported 1:1 from `validate-skill.py`'s
/// `ALLOWED_SUBDOMAINS` — the FLAT set of every accepted value (each
/// canonical form PLUS all of its aliases in `_SUBDOMAIN_ALIASES`), 46
/// entries total. The vendor validator accepts a subdomain iff it is a
/// member of this flat set (it only WARNs, non-blocking, when an accepted
/// value is an alias rather than the canonical form); since this Rust
/// validator's contract is pass/fail, not advisory, membership in the flat
/// set is exactly the accept/reject decision — so the flat set, not the
/// canonical-only subset, is the faithful port. Grouped below by vendor
/// canonical (first entry) followed by its aliases, mirroring the
/// `_SUBDOMAIN_ALIASES` table so the port is auditable line-for-line
/// against `tools/validate-skill.py` (L21-62).
///
/// Kept as a sorted-within-group verbatim copy of the vendor set: a value
/// the corpus's 817 SKILL.md actually use (e.g. `security-operations`,
/// `ransomware-defense`, `threat-detection`, `application-security`,
/// `identity-and-access-management`) must be accepted, and a value the
/// vendor does NOT accept must be rejected — the earlier hand-authored
/// list diverged from the vendor set in both directions and is replaced
/// here.
pub const ALLOWED_SUBDOMAINS: &[&str] = &[
    // identity
    "identity-access-management",
    "identity-and-access-management",
    "identity-security",
    // zero-trust
    "zero-trust-architecture",
    "zero-trust",
    // OT/ICS
    "ot-ics-security",
    "ot-security",
    // SOC / security ops
    "soc-operations",
    "security-operations",
    // red team
    "red-teaming",
    "red-team",
    // web / application security
    "web-application-security",
    "application-security",
    "network-security",
    // pentest / offensive
    "penetration-testing",
    "offensive-security",
    "digital-forensics",
    "malware-analysis",
    "threat-intelligence",
    "cloud-security",
    "container-security",
    "cryptography",
    "vulnerability-management",
    // compliance / GRC
    "compliance-governance",
    "governance-risk-compliance",
    "devsecops",
    "threat-hunting",
    "incident-response",
    "endpoint-security",
    // phishing / social-engineering defense
    "phishing-defense",
    "social-engineering-defense",
    "api-security",
    "mobile-security",
    "ransomware-defense",
    "threat-detection",
    "blockchain-security",
    "data-protection",
    "deception-technology",
    // hardware / firmware
    "hardware-firmware-security",
    "firmware-analysis",
    "firmware-security",
    "privacy-compliance",
    "purple-team",
    "supply-chain-security",
    "wireless-security",
    "ai-security",
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
        extract_list, find_skill_md_files, is_known_mitre_attack_id, is_known_nist_csf_id,
        union_frontmatter_ids, ALLOWED_SUBDOMAINS,
    };
    use std::collections::BTreeSet;
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

    /// The allowlist is exactly the 46-value flat set `validate-skill.py`
    /// accepts, with no duplicates. Guards against re-introducing a
    /// hand-authored list of a different size/content.
    #[test]
    fn allowlist_is_the_46_value_vendor_flat_set_with_no_duplicates() {
        let unique: BTreeSet<&&str> = ALLOWED_SUBDOMAINS.iter().collect();
        assert_eq!(
            unique.len(),
            ALLOWED_SUBDOMAINS.len(),
            "ALLOWED_SUBDOMAINS has duplicate entries"
        );
        assert_eq!(
            ALLOWED_SUBDOMAINS.len(),
            46,
            "vendor validate-skill.py ALLOWED_SUBDOMAINS is a 46-value flat set"
        );
        // Vendor-accepted values the earlier fabricated list wrongly
        // rejected (real corpus subdomains) MUST be accepted now.
        for accepted in [
            "security-operations",
            "ransomware-defense",
            "threat-detection",
            "deception-technology",
            "application-security",
            "identity-and-access-management",
            "red-team",
            "zero-trust",
            "governance-risk-compliance",
            "offensive-security",
            "ot-security",
            "privacy-compliance",
            "purple-team",
            "hardware-firmware-security",
            "firmware-analysis",
            "data-protection",
        ] {
            assert!(
                ALLOWED_SUBDOMAINS.contains(&accepted),
                "vendor accepts '{accepted}' but our allowlist rejects it"
            );
        }
        // Values the earlier fabricated list invented but the vendor never
        // accepts MUST NOT be present (they would let invalid skills pass).
        for fabricated in [
            "iot-security",
            "data-security",
            "email-security",
            "physical-security",
            "security-awareness-training",
            "privacy-engineering",
            "secure-coding",
            "security-architecture",
            "risk-management",
            "insider-threat",
            "disaster-recovery",
            "business-continuity",
            "security-automation",
            "quantum-security",
            "critical-infrastructure-security",
            "database-security",
            "virtualization-security",
        ] {
            assert!(
                !ALLOWED_SUBDOMAINS.contains(&fabricated),
                "'{fabricated}' is not in vendor validate-skill.py's allowlist but ours accepts it"
            );
        }
    }

    /// Extract the scalar `subdomain:` value from a SKILL.md frontmatter
    /// block (the corpus never uses a list/folded form for this field).
    fn subdomain_of(source: &str) -> Option<String> {
        let mut lines = source.lines();
        if lines.next().map(str::trim) != Some("---") {
            return None;
        }
        for line in lines {
            if line.trim() == "---" {
                break;
            }
            if line.starts_with(char::is_whitespace) {
                continue;
            }
            if let Some((key, value)) = line.split_once(':') {
                if key.trim() == "subdomain" {
                    let v = value.trim().trim_matches(['\'', '"']).to_owned();
                    if !v.is_empty() {
                        return Some(v);
                    }
                }
            }
        }
        None
    }

    /// Corpus-backed parity proof (the real defect the earlier list had):
    /// every `subdomain:` value actually used across the vendored 817
    /// SKILL.md must be accepted by our allowlist — otherwise the linter
    /// false-rejects legitimate corpus skills. Skipped gracefully when the
    /// vendor dir is absent (L12 honesty protocol).
    #[test]
    fn allowlist_accepts_every_subdomain_the_corpus_actually_uses(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let corpus_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../vendor/anthropic-cybersecurity-skills/skills");
        if !corpus_dir.is_dir() {
            return Ok(());
        }
        let allow: BTreeSet<&str> = ALLOWED_SUBDOMAINS.iter().copied().collect();
        let mut rejected: BTreeSet<String> = BTreeSet::new();
        let mut seen = 0usize;
        for path in find_skill_md_files(&corpus_dir) {
            let Ok(source) = std::fs::read_to_string(&path) else {
                continue;
            };
            if let Some(subdomain) = subdomain_of(&source) {
                seen += 1;
                if !allow.contains(subdomain.as_str()) {
                    rejected.insert(subdomain);
                }
            }
        }
        assert!(seen > 0, "expected to read at least one SKILL.md subdomain");
        assert!(
            rejected.is_empty(),
            "allowlist rejects subdomains the corpus actually uses (linter would false-flag real \
             skills): {rejected:?}"
        );
        Ok(())
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
