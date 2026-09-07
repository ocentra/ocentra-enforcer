//! Property coverage for the parsers still owned by `enforcer-memory`.
//!
//! Syntax parser properties live with their owner in
//! `enforcer-syntax/tests/property_parser_contracts.rs`.
// contractHash: property_parser_contracts.rs
// sourceOwner: enforcer-memory

use enforcer_memory::{analysis::query, cli, ingest, lesson, llama_cpp};
use proptest::{
    prelude::any,
    prop_assert, prop_assert_eq, proptest,
    strategy::Strategy,
    test_runner::{Config as ProptestConfig, RngSeed},
};
use std::{
    panic::{catch_unwind, AssertUnwindSafe},
    process::Command,
    sync::{Mutex, MutexGuard},
};

const PROPERTY_PARSER_CASES: u32 = 64;
const PROPERTY_PARSER_SEED: u64 = 0x4f43_454e_5452_4150;
const PROPERTY_PARSER_KEY_ENV: &str = "OCENTRA_PROPERTY_PARSER_KEY";
const PROPERTY_PARSER_CHILD_TEST: &str = "property_parser_child";
const EXACT_TEST_ARGUMENT: &str = "--exact";
const NO_CAPTURE_ARGUMENT: &str = "--nocapture";
static PARSER_TEST_SERIALIZER: Mutex<()> = Mutex::new(());

fn parser_test_guard() -> MutexGuard<'static, ()> {
    match PARSER_TEST_SERIALIZER.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

fn property_parser_config() -> ProptestConfig {
    ProptestConfig {
        cases: PROPERTY_PARSER_CASES,
        rng_seed: RngSeed::Fixed(PROPERTY_PARSER_SEED),
        ..ProptestConfig::default()
    }
}

macro_rules! property_parser_contracts {
    ($($key:literal => $exercise:expr),+ $(,)?) => {
        const PROPERTY_PARSER_KEYS: &[&str] = &[$($key),+];

        fn exercise_registered_parser(parser_key: &str, source: &str) -> bool {
            match parser_key {
                $(
                    $key => {
                        let _ = ($exercise)(source);
                        true
                    }
                )+
                _ => false,
            }
        }

        proptest! {
            #![proptest_config(property_parser_config())]
            #[test]
            fn property_parser_child(
                source in proptest::collection::vec(any::<char>(), 0..128)
                    .prop_map(|characters| characters.into_iter().collect::<String>()),
            ) {
                let _guard = parser_test_guard();
                let selected_parser = match std::env::var(PROPERTY_PARSER_KEY_ENV) {
                    Ok(value) => value,
                    Err(_) => return Ok(()),
                };
                let outcome = catch_unwind(AssertUnwindSafe(|| {
                    exercise_registered_parser(&selected_parser, &source)
                }));
                let matched = match outcome {
                    Ok(value) => value,
                    Err(_) => {
                        prop_assert_eq!(selected_parser, "", "registered parser panicked");
                        false
                    }
                };
                prop_assert!(matched, "registered parser key was not found");
            }
        }

        #[test]
        fn every_registered_parser_is_total() {
            let _guard = parser_test_guard();
            let current_executable = match std::env::current_exe() {
                Ok(path) => path,
                Err(error) => {
                    assert!(false, "failed to resolve parser test executable: {error}");
                    return;
                }
            };
            for parser_key in PROPERTY_PARSER_KEYS {
                let output = match Command::new(&current_executable)
                    .arg(EXACT_TEST_ARGUMENT)
                    .arg(PROPERTY_PARSER_CHILD_TEST)
                    .arg(NO_CAPTURE_ARGUMENT)
                    .env(PROPERTY_PARSER_KEY_ENV, parser_key)
                    .output()
                {
                    Ok(value) => value,
                    Err(error) => {
                        assert!(
                            false,
                            "failed to start parser property child for {parser_key}: {error}"
                        );
                        return;
                    }
                };
                assert!(
                    output.status.success(),
                    "registered parser process failed for {parser_key}: {}\nstdout:\n{}\nstderr:\n{}",
                    output.status,
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr)
                );
            }
        }
    };
}

property_parser_contracts! {
    "src/analysis/query.rs::parse" => query::parse,
    "src/cli.rs::parse_cli_args" => |source: &str| cli::parse_cli_args(&[source.to_owned()]),
    "src/ingest.rs::parse_ndjson" => ingest::parse_ndjson,
    "src/lesson.rs::parse_ledger" => lesson::parse_ledger,
    "src/llama_cpp.rs::parse_llama_cpp_devices" => llama_cpp::parse_llama_cpp_devices,
    "src/llama_cpp.rs::parse_generation_rate" => llama_cpp::parse_generation_rate,
}

#[test]
fn parser_property_generation_is_reproducible() {
    let config = property_parser_config();
    assert_eq!(config.cases, PROPERTY_PARSER_CASES);
    assert_eq!(config.rng_seed, RngSeed::Fixed(PROPERTY_PARSER_SEED));
}
