//! Retrieval-quality metric family required by
//! `MEMORY_RETRIEVAL_QA_PROOF_GATE.md` §"Per-row proof requirements":
//! Recall@k, Precision@k, MRR@k, nDCG@k, reranker lift, and the
//! token-reduction ratio. Every function here is pure (no I/O, no graph
//! dependency) so it is spot-checkable by hand and reusable by both the
//! row runners (`runners.rs`) and the proof emitters (`proof.rs`).
//!
//! All functions treat `expected`/`actual` as ordered lists of stable
//! string ids: `actual` is the candidate/result order (rank 1 first),
//! `expected` is the unordered set of ids a correct answer must contain.
//! An empty `actual` always scores 0.0 for every ranked metric rather
//! than dividing by zero or fabricating a score (anti-vacuous doctrine,
//! matching `recall.rs`'s existing "no fallback-to-all-nodes" contract).

use std::collections::HashSet;

/// `|expected ∩ actual[..k]| / |expected|`. `0.0` when `expected` is
/// empty (nothing to recall) rather than `NaN`/`1.0` -- a row with no
/// expected ids is a malformed row, not a trivially-passing one, so
/// callers should treat this as a signal to check the row, not a pass.
pub fn recall_at_k(expected: &[String], actual: &[String], k: usize) -> f64 {
    if expected.is_empty() {
        return 0.0;
    }
    let expected_set: HashSet<&str> = expected.iter().map(String::as_str).collect();
    let top_k: HashSet<&str> = actual.iter().take(k).map(String::as_str).collect();
    let hits = expected_set.intersection(&top_k).count();
    hits as f64 / expected_set.len() as f64
}

/// `|expected ∩ actual[..k]| / k`. `0.0` when `k` is `0` or `actual` is
/// empty.
pub fn precision_at_k(expected: &[String], actual: &[String], k: usize) -> f64 {
    if k == 0 || actual.is_empty() {
        return 0.0;
    }
    let expected_set: HashSet<&str> = expected.iter().map(String::as_str).collect();
    let top_k: Vec<&str> = actual.iter().take(k).map(String::as_str).collect();
    if top_k.is_empty() {
        return 0.0;
    }
    let hits = top_k.iter().filter(|id| expected_set.contains(*id)).count();
    hits as f64 / top_k.len() as f64
}

/// Mean Reciprocal Rank within the top `k`: `1 / rank` of the FIRST
/// expected id found in `actual[..k]` (rank is 1-based), or `0.0` if no
/// expected id appears in the top `k` at all. ("Mean" refers to
/// averaging across a queryset in the caller -- this function computes
/// one query's reciprocal rank.)
pub fn mrr_at_k(expected: &[String], actual: &[String], k: usize) -> f64 {
    if expected.is_empty() {
        return 0.0;
    }
    let expected_set: HashSet<&str> = expected.iter().map(String::as_str).collect();
    for (index, id) in actual.iter().take(k).enumerate() {
        if expected_set.contains(id.as_str()) {
            return 1.0 / (index as f64 + 1.0);
        }
    }
    0.0
}

/// Normalized Discounted Cumulative Gain at `k`, with binary relevance
/// (1.0 if `actual[i]` is in `expected`, else 0.0) and the standard
/// `1/log2(rank+1)` discount. `DCG / IDCG`, where `IDCG` is the DCG of
/// the ideal ordering (all relevant docs first). `0.0` when `expected`
/// is empty (`IDCG` would be zero).
pub fn ndcg_at_k(expected: &[String], actual: &[String], k: usize) -> f64 {
    if expected.is_empty() {
        return 0.0;
    }
    let expected_set: HashSet<&str> = expected.iter().map(String::as_str).collect();

    let dcg: f64 = actual
        .iter()
        .take(k)
        .enumerate()
        .map(|(index, id)| {
            let relevance = if expected_set.contains(id.as_str()) {
                1.0
            } else {
                0.0
            };
            let rank = index + 1;
            relevance / (rank as f64 + 1.0).log2()
        })
        .sum();

    let ideal_hits = expected_set.len().min(k);
    let idcg: f64 = (1..=ideal_hits)
        .map(|rank| 1.0 / (rank as f64 + 1.0).log2())
        .sum();

    if idcg == 0.0 {
        0.0
    } else {
        dcg / idcg
    }
}

/// Reranker lift: the change in nDCG@k caused by reranking, i.e.
/// `ndcg_at_k(expected, post_rerank, k) - ndcg_at_k(expected,
/// pre_rerank, k)`. Positive means reranking improved ranking quality;
/// this is the exact quantity `MEMORY_RETRIEVAL_QA_BENCHMARKS.md`'s
/// global scoring gate calls `reranker_lift_at_10` (threshold `>=
/// 0.05` on semantic rows) and QA-097/QA-206/QA-207/QA-225 measure
/// directly.
pub fn reranker_lift(
    expected: &[String],
    pre_rerank: &[String],
    post_rerank: &[String],
    k: usize,
) -> f64 {
    ndcg_at_k(expected, post_rerank, k) - ndcg_at_k(expected, pre_rerank, k)
}

/// Token-reduction ratio: `naive_tokens / context_tokens`, matching
/// `enforcer_memory::search::TokenReductionEstimate::ratio`'s exact
/// contract (this harness re-derives the same formula independently so
/// a bug in the library's own `ratio()` cannot silently launder into a
/// passing QA-098/QA-213 proof row) -- `0.0` when `context_tokens` is
/// `0` rather than dividing by zero.
pub fn token_reduction_ratio(naive_tokens: usize, context_tokens: usize) -> f64 {
    if context_tokens == 0 {
        0.0
    } else {
        naive_tokens as f64 / context_tokens as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(values: &[&str]) -> Vec<String> {
        values.iter().map(|s| s.to_string()).collect()
    }

    // --- recall_at_k -----------------------------------------------

    #[test]
    fn recall_at_k_hand_computed_two_of_three() {
        // expected = {a, b, c}; actual top-5 contains a, b but not c.
        // recall@5 = 2/3.
        let expected = ids(&["a", "b", "c"]);
        let actual = ids(&["a", "x", "b", "y", "z"]);
        let recall = recall_at_k(&expected, &actual, 5);
        assert!((recall - (2.0 / 3.0)).abs() < 1e-9);
    }

    #[test]
    fn recall_at_k_respects_k_cutoff() {
        // expected = {a}; a is only at rank 3, so recall@2 = 0.
        let expected = ids(&["a"]);
        let actual = ids(&["x", "y", "a"]);
        assert_eq!(recall_at_k(&expected, &actual, 2), 0.0);
        assert_eq!(recall_at_k(&expected, &actual, 3), 1.0);
    }

    #[test]
    fn recall_at_k_empty_expected_is_zero_not_nan() {
        assert_eq!(recall_at_k(&[], &ids(&["a"]), 5), 0.0);
    }

    // --- precision_at_k ----------------------------------------------

    #[test]
    fn precision_at_k_hand_computed_one_of_four() {
        // expected = {a}; top-4 = [x, a, y, z] -> 1 hit / 4 = 0.25.
        let expected = ids(&["a"]);
        let actual = ids(&["x", "a", "y", "z"]);
        assert!((precision_at_k(&expected, &actual, 4) - 0.25).abs() < 1e-9);
    }

    #[test]
    fn precision_at_k_zero_k_is_zero() {
        assert_eq!(precision_at_k(&ids(&["a"]), &ids(&["a"]), 0), 0.0);
    }

    // --- mrr_at_k ------------------------------------------------------

    #[test]
    fn mrr_at_k_hand_computed_rank_three() {
        // First expected hit at rank 3 -> reciprocal rank = 1/3.
        let expected = ids(&["c"]);
        let actual = ids(&["a", "b", "c", "d"]);
        let mrr = mrr_at_k(&expected, &actual, 10);
        assert!((mrr - (1.0 / 3.0)).abs() < 1e-9);
    }

    #[test]
    fn mrr_at_k_no_hit_in_window_is_zero() {
        let expected = ids(&["z"]);
        let actual = ids(&["a", "b", "c"]);
        assert_eq!(mrr_at_k(&expected, &actual, 3), 0.0);
    }

    #[test]
    fn mrr_at_k_first_hit_wins_when_multiple_expected_present() {
        // expected = {b, c}; b at rank 2 comes first, so MRR = 1/2 even
        // though c (rank 3) is also relevant.
        let expected = ids(&["b", "c"]);
        let actual = ids(&["a", "b", "c"]);
        assert!((mrr_at_k(&expected, &actual, 3) - 0.5).abs() < 1e-9);
    }

    // --- ndcg_at_k -----------------------------------------------------

    #[test]
    fn ndcg_at_k_perfect_ranking_is_one() {
        // expected = {a, b}; actual = [a, b, c] -- both relevant docs
        // occupy the two best possible ranks, so nDCG must be exactly 1.
        let expected = ids(&["a", "b"]);
        let actual = ids(&["a", "b", "c"]);
        let ndcg = ndcg_at_k(&expected, &actual, 3);
        assert!((ndcg - 1.0).abs() < 1e-9, "got {ndcg}");
    }

    #[test]
    fn ndcg_at_k_hand_computed_single_relevant_at_rank_two() {
        // expected = {b}; actual = [a, b, c].
        // DCG = 1/log2(3) (relevance at rank 2 only).
        // IDCG = 1/log2(2) (ideal: the one relevant doc at rank 1).
        // nDCG = log2(2)/log2(3) = 1/log2(3).
        let expected = ids(&["b"]);
        let actual = ids(&["a", "b", "c"]);
        let ndcg = ndcg_at_k(&expected, &actual, 3);
        let hand_computed = (1.0 / 3.0_f64.log2()) / (1.0 / 2.0_f64.log2());
        assert!((ndcg - hand_computed).abs() < 1e-9);
        // log2(3) ~= 1.584962500721156, so nDCG ~= 0.6309...
        assert!((ndcg - 0.630_929_753_571_457_2).abs() < 1e-9);
    }

    #[test]
    fn ndcg_at_k_empty_expected_is_zero() {
        assert_eq!(ndcg_at_k(&[], &ids(&["a"]), 5), 0.0);
    }

    #[test]
    fn ndcg_at_k_no_relevant_hits_is_zero() {
        let expected = ids(&["z"]);
        let actual = ids(&["a", "b", "c"]);
        assert_eq!(ndcg_at_k(&expected, &actual, 3), 0.0);
    }

    // --- reranker_lift -------------------------------------------------

    #[test]
    fn reranker_lift_hand_computed_positive_lift() {
        // expected = {a}. pre_rerank puts a at rank 3, post_rerank
        // promotes it to rank 1 -- lift must be positive and equal the
        // exact nDCG delta.
        let expected = ids(&["a"]);
        let pre = ids(&["x", "y", "a"]);
        let post = ids(&["a", "x", "y"]);
        let lift = reranker_lift(&expected, &pre, &post, 3);
        let expected_lift = ndcg_at_k(&expected, &post, 3) - ndcg_at_k(&expected, &pre, 3);
        assert!((lift - expected_lift).abs() < 1e-9);
        assert!(
            lift > 0.0,
            "promoting the only relevant doc to rank 1 must lift nDCG"
        );
        // nDCG@post = 1.0 (perfect), nDCG@pre = 1/log2(4) ~= 0.5.
        assert!((lift - (1.0 - 1.0 / 4.0_f64.log2())).abs() < 1e-9);
    }

    #[test]
    fn reranker_lift_is_zero_when_ranking_unchanged() {
        let expected = ids(&["a", "b"]);
        let same = ids(&["a", "b", "c"]);
        assert_eq!(reranker_lift(&expected, &same, &same, 3), 0.0);
    }

    #[test]
    fn reranker_lift_is_negative_when_reranking_demotes_relevant_doc() {
        let expected = ids(&["a"]);
        let pre = ids(&["a", "x", "y"]);
        let post = ids(&["x", "y", "a"]);
        let lift = reranker_lift(&expected, &pre, &post, 3);
        assert!(
            lift < 0.0,
            "demoting the only relevant doc must be a negative lift"
        );
    }

    // --- token_reduction_ratio ------------------------------------------

    #[test]
    fn token_reduction_ratio_hand_computed_20x() {
        assert!((token_reduction_ratio(10_000, 500) - 20.0).abs() < 1e-9);
    }

    #[test]
    fn token_reduction_ratio_zero_context_tokens_is_zero_not_infinite() {
        assert_eq!(token_reduction_ratio(10_000, 0), 0.0);
    }

    #[test]
    fn token_reduction_ratio_matches_library_estimate_formula() {
        // Cross-check against enforcer_memory::search::TokenReductionEstimate::ratio's
        // documented formula (`naive_tokens / context_tokens`) so a
        // change to that library type would need a matching change
        // here to stay green.
        let estimate = enforcer_memory::search::TokenReductionEstimate {
            naive_tokens: 8000,
            context_tokens: 400,
        };
        assert_eq!(
            token_reduction_ratio(estimate.naive_tokens, estimate.context_tokens),
            estimate.ratio()
        );
    }
}
