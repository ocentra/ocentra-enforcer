use crate::risk_heuristics::{is_logging_context, is_magic_string_comparison};
use crate::{FileRole, LiteralCandidate, RiskCategory};

const ROLE_ADJUSTMENTS: [i16; 10] = [20, 5, -5, -30, -50, -10, -10, -50, -50, 0];
const CATEGORY_ADJUSTMENTS: [i16; 16] = [
    100, 45, 40, 30, 35, 30, 45, 60, 70, 45, 20, 5, -20, -60, -30, 0,
];

pub(crate) fn score_literal(
    candidate: &LiteralCandidate,
    role: FileRole,
    category: RiskCategory,
    repeated_files: usize,
) -> u8 {
    let mut score = 5 + ROLE_ADJUSTMENTS[role as usize] + CATEGORY_ADJUSTMENTS[category as usize];
    if is_magic_string_comparison(&candidate.context) {
        score += 35;
    }
    if is_logging_context(&candidate.context) {
        score -= 20;
    }
    if repeated_files >= 3 {
        score += 20;
    } else if repeated_files == 2 {
        score += 10;
    }
    score.clamp(0, 100) as u8
}
