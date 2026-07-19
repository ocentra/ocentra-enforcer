#![cfg(feature = "ort-models")]

use enforcer_memory::ort_runtime::real::OrtTokenizer;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn tokenizer_pipeline_preserves_ids_and_appends_end_of_text() -> TestResult {
    let official_qwen = std::env::var_os("ENFORCER_QWEN_TOKENIZER_PATH");
    let (path, hello, yes, no) = match official_qwen {
        Some(path) => (path.into(), vec![14990, 151643], 9693, 2152),
        None => (
            "tests/fixtures/memory/tokenizer_contract.json".into(),
            vec![1, 3, 2, 6],
            4,
            5,
        ),
    };
    let tokenizer = OrtTokenizer::load(path)?;

    assert_eq!(tokenizer.encode_with_end_of_text("hello")?, hello);
    assert_eq!(tokenizer.token_id("yes", "resolve-test-yes-token")?, yes);
    assert_eq!(tokenizer.token_id("no", "resolve-test-no-token")?, no);
    Ok(())
}
