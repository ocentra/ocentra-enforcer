use enforcer_memory::search::{estimate_tokens, TokenReductionEstimate};

#[test]
fn token_reduction_ratio_is_zero_for_empty_context() {
    let estimate = TokenReductionEstimate {
        naive_tokens: 1000.into(),
        context_tokens: 0.into(),
    };
    assert_eq!(estimate.ratio(), 0.0);
}

#[test]
fn token_reduction_ratio_reflects_savings() {
    let estimate = TokenReductionEstimate {
        naive_tokens: 10_000.into(),
        context_tokens: 500.into(),
    };
    assert_eq!(estimate.ratio(), 20.0);
}

#[test]
fn estimate_tokens_is_never_zero_for_nonempty_text() {
    assert!(estimate_tokens("a").get() >= 1);
    assert!(estimate_tokens("").get() >= 1);
}
