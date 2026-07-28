//! Property coverage for every public parser and normalizer contract.
//!
//! Registration keys are deliberately source-qualified. The Enforcer evidence
//! resolver requires an exact `src/path.rs::function` entry, so a property for
//! one of the many `parse` functions cannot accidentally cover another.
// contractHash: property_parser_contracts.rs
// sourceOwner: enforcer-memory

use enforcer_memory::{
    analysis::query,
    cli, ingest,
    languages::{self, generic},
    lesson, llama_cpp, parsers,
};
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
                    assert!(
                        false,
                        "failed to resolve parser test executable: {error}"
                    );
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

        #[test]
        fn registered_parsers_share_process_for_safe_source() {
            let _guard = parser_test_guard();
            const SAFE_SOURCE: &str = "fn main() {}";
            for parser_key in PROPERTY_PARSER_KEYS {
                assert!(
                    exercise_registered_parser(parser_key, SAFE_SOURCE),
                    "registered parser key was not found: {parser_key}"
                );
            }
        }
    };
}

property_parser_contracts! {
    "src/analysis/query.rs::parse" => query::parse,
    "src/cli.rs::parse_cli_args" => |source: &str| cli::parse_cli_args(&[source.to_owned()]),
    "src/ingest.rs::parse_ndjson" => ingest::parse_ndjson,
    "src/languages/c.rs::parse" => |source: &str| languages::c::parse(source, false),
    "src/languages/cpp.rs::parse" => |source: &str| languages::cpp::parse(source, false),
    "src/languages/csharp.rs::parse" => languages::csharp::parse,
    "src/languages/generic.rs::parse_with_spec" => |source: &str| {
        let language: tree_sitter::Language = tree_sitter_rust::LANGUAGE.into();
        generic::parse_with_spec(
            source,
            &language,
            &languages::spec::LangSpec::rust(),
            &generic::Quirks::default(),
            false,
        )
    },
    "src/languages/generic.rs::parse_rust" => generic::parse_rust,
    "src/languages/generic.rs::parse_typescript" => generic::parse_typescript,
    "src/languages/generic.rs::parse_tsx" => generic::parse_tsx,
    "src/languages/generic.rs::parse_python" => generic::parse_python,
    "src/languages/generic.rs::parse_java" => generic::parse_java,
    "src/languages/generic.rs::parse_c" => |source: &str| generic::parse_c(source, false),
    "src/languages/generic.rs::parse_cpp" => |source: &str| generic::parse_cpp(source, false),
    "src/languages/generic.rs::parse_kotlin" => generic::parse_kotlin,
    "src/languages/generic.rs::parse_swift" => generic::parse_swift,
    "src/languages/generic.rs::parse_solidity" => generic::parse_solidity,
    "src/languages/generic.rs::parse_gdscript" => generic::parse_gdscript,
    "src/languages/generic.rs::parse_dart" => generic::parse_dart,
    "src/languages/generic.rs::parse_scala" => generic::parse_scala,
    "src/languages/generic.rs::parse_groovy" => generic::parse_groovy,
    "src/languages/generic.rs::parse_ruby" => generic::parse_ruby,
    "src/languages/generic.rs::parse_zig" => generic::parse_zig,
    "src/languages/generic.rs::parse_objc" => generic::parse_objc,
    "src/languages/generic.rs::parse_bash" => generic::parse_bash,
    "src/languages/generic.rs::parse_lua" => generic::parse_lua,
    "src/languages/generic.rs::parse_elixir" => generic::parse_elixir,
    "src/languages/generic.rs::parse_haskell" => generic::parse_haskell,
    "src/languages/generic.rs::parse_ocaml" => generic::parse_ocaml,
    "src/languages/generic.rs::parse_erlang" => generic::parse_erlang,
    "src/languages/generic.rs::parse_cuda" => |source: &str| generic::parse_cuda(source, false),
    "src/languages/generic.rs::parse_d" => generic::parse_d,
    "src/languages/generic.rs::parse_powershell" => generic::parse_powershell,
    "src/languages/generic.rs::parse_fsharp" => generic::parse_fsharp,
    "src/languages/generic.rs::parse_gleam" => generic::parse_gleam,
    "src/languages/generic.rs::parse_glsl" => generic::parse_glsl,
    "src/languages/generic.rs::parse_ada" => generic::parse_ada,
    "src/languages/generic.rs::parse_apex" => generic::parse_apex,
    "src/languages/generic.rs::parse_crystal" => generic::parse_crystal,
    "src/languages/generic.rs::parse_r" => generic::parse_r,
    "src/languages/generic.rs::parse_perl" => generic::parse_perl,
    "src/languages/generic.rs::parse_clojure" => generic::parse_clojure,
    "src/languages/generic.rs::parse_julia" => generic::parse_julia,
    "src/languages/generic.rs::parse_odin" => generic::parse_odin,
    "src/languages/generic.rs::parse_pascal" => generic::parse_pascal,
    "src/languages/generic.rs::parse_qml" => generic::parse_qml,
    "src/languages/generic.rs::parse_rescript" => generic::parse_rescript,
    "src/languages/generic.rs::parse_squirrel" => generic::parse_squirrel,
    "src/languages/generic.rs::parse_sway" => generic::parse_sway,
    "src/languages/generic.rs::parse_starlark" => generic::parse_starlark,
    "src/languages/generic.rs::parse_templ" => generic::parse_templ,
    "src/languages/generic.rs::parse_typst" => generic::parse_typst,
    "src/languages/generic.rs::parse_wgsl" => generic::parse_wgsl,
    "src/languages/generic.rs::parse_wolfram" => generic::parse_wolfram,
    "src/languages/generic.rs::parse_slang" => generic::parse_slang,
    "src/languages/generic.rs::parse_scss" => generic::parse_scss,
    "src/languages/generic.rs::parse_cmake" => generic::parse_cmake,
    "src/languages/generic.rs::parse_makefile" => generic::parse_makefile,
    "src/languages/generic.rs::parse_fortran" => generic::parse_fortran,
    "src/languages/generic.rs::parse_vimscript" => generic::parse_vimscript,
    "src/languages/generic.rs::parse_puppet" => generic::parse_puppet,
    "src/languages/generic.rs::parse_elm" => generic::parse_elm,
    "src/languages/generic.rs::parse_bicep" => generic::parse_bicep,
    "src/languages/generic.rs::parse_bitbake" => generic::parse_bitbake,
    "src/languages/generic.rs::parse_cairo" => generic::parse_cairo,
    "src/languages/generic.rs::parse_cfscript" => generic::parse_cfscript,
    "src/languages/generic.rs::parse_func" => generic::parse_func,
    "src/languages/generic.rs::parse_move" => generic::parse_move,
    "src/languages/generic.rs::parse_nickel" => generic::parse_nickel,
    "src/languages/generic.rs::parse_jsonnet" => generic::parse_jsonnet,
    "src/languages/generic.rs::parse_commonlisp" => generic::parse_commonlisp,
    "src/languages/generic.rs::parse_lean" => generic::parse_lean,
    "src/languages/generic.rs::parse_tlaplus" => generic::parse_tlaplus,
    "src/languages/generic.rs::parse_verilog" => generic::parse_verilog,
    "src/languages/generic.rs::parse_vhdl" => generic::parse_vhdl,
    "src/languages/generic.rs::parse_systemverilog" => generic::parse_systemverilog,
    "src/languages/generic.rs::parse_cobol" => generic::parse_cobol,
    "src/languages/generic.rs::parse_just" => generic::parse_just,
    "src/languages/generic.rs::parse_hlsl" => generic::parse_hlsl,
    "src/languages/generic.rs::parse_ispc" => generic::parse_ispc,
    "src/languages/generic.rs::parse_purescript" => generic::parse_purescript,
    "src/languages/generic.rs::parse_magma" => generic::parse_magma,
    "src/languages/generic.rs::parse_hare" => generic::parse_hare,
    "src/languages/generic.rs::parse_pony" => generic::parse_pony,
    "src/languages/generic.rs::parse_nasm" => generic::parse_nasm,
    "src/languages/generic.rs::parse_emacslisp" => generic::parse_emacslisp,
    "src/languages/generic.rs::parse_capnp" => generic::parse_capnp,
    "src/languages/generic.rs::parse_matlab" => generic::parse_matlab,
    "src/languages/generic.rs::parse_luau" => generic::parse_luau,
    "src/languages/generic.rs::parse_teal" => generic::parse_teal,
    "src/languages/generic.rs::parse_fennel" => generic::parse_fennel,
    "src/languages/generic.rs::parse_meson" => generic::parse_meson,
    "src/languages/generic.rs::parse_kconfig" => generic::parse_kconfig,
    "src/languages/generic.rs::parse_awk" => generic::parse_awk,
    "src/languages/generic.rs::parse_fish" => generic::parse_fish,
    "src/languages/generic.rs::parse_zsh" => generic::parse_zsh,
    "src/languages/generic.rs::parse_tcl" => generic::parse_tcl,
    "src/languages/generic.rs::parse_scheme" => generic::parse_scheme,
    "src/languages/generic.rs::parse_racket" => generic::parse_racket,
    "src/languages/generic.rs::parse_smithy" => generic::parse_smithy,
    "src/languages/generic.rs::parse_pine" => generic::parse_pine,
    "src/languages/generic.rs::parse_hcl" => generic::parse_hcl,
    "src/languages/generic.rs::parse_nix" => generic::parse_nix,
    "src/languages/generic.rs::parse_sql" => generic::parse_sql,
    "src/languages/generic.rs::parse_protobuf" => generic::parse_protobuf,
    "src/languages/generic.rs::parse_prisma" => generic::parse_prisma,
    "src/languages/generic.rs::parse_pkl" => generic::parse_pkl,
    "src/languages/generic.rs::parse_thrift" => generic::parse_thrift,
    "src/languages/generic.rs::parse_wit" => generic::parse_wit,
    "src/languages/generic.rs::parse_llvm_ir" => generic::parse_llvm_ir,
    "src/languages/generic.rs::parse_tablegen" => generic::parse_tablegen,
    "src/languages/generic.rs::parse_cfml" => generic::parse_cfml,
    "src/languages/generic.rs::parse_gotemplate" => generic::parse_gotemplate,
    "src/languages/generic.rs::parse_devicetree" => generic::parse_devicetree,
    "src/languages/generic.rs::parse_smali" => generic::parse_smali,
    "src/languages/generic.rs::parse_requirements" => generic::parse_requirements,
    "src/languages/generic.rs::parse_ron" => generic::parse_ron,
    "src/languages/generic.rs::parse_rst" => generic::parse_rst,
    "src/languages/generic.rs::parse_soql" => generic::parse_soql,
    "src/languages/generic.rs::parse_sosl" => generic::parse_sosl,
    "src/languages/generic.rs::parse_sshconfig" => generic::parse_sshconfig,
    "src/languages/generic.rs::parse_svelte" => generic::parse_svelte,
    "src/languages/generic.rs::parse_toml" => generic::parse_toml,
    "src/languages/generic.rs::parse_vue" => generic::parse_vue,
    "src/languages/generic.rs::parse_xml" => generic::parse_xml,
    "src/languages/generic.rs::parse_yaml" => generic::parse_yaml,
    "src/languages/generic.rs::parse_json5" => generic::parse_json5,
    "src/languages/generic.rs::parse_kdl" => generic::parse_kdl,
    "src/languages/generic.rs::parse_linkerscript" => generic::parse_linkerscript,
    "src/languages/generic.rs::parse_liquid" => generic::parse_liquid,
    "src/languages/generic.rs::parse_markdown" => generic::parse_markdown,
    "src/languages/generic.rs::parse_mermaid" => generic::parse_mermaid,
    "src/languages/generic.rs::parse_po" => generic::parse_po,
    "src/languages/generic.rs::parse_properties" => generic::parse_properties,
    "src/languages/generic.rs::parse_regex" => generic::parse_regex,
    "src/languages/generic.rs::parse_gitignore" => generic::parse_gitignore,
    "src/languages/generic.rs::parse_gn" => generic::parse_gn,
    "src/languages/generic.rs::parse_gomod" => generic::parse_gomod,
    "src/languages/generic.rs::parse_graphql" => generic::parse_graphql,
    "src/languages/generic.rs::parse_html" => generic::parse_html,
    "src/languages/generic.rs::parse_hyprlang" => generic::parse_hyprlang,
    "src/languages/generic.rs::parse_ini" => generic::parse_ini,
    "src/languages/generic.rs::parse_janet" => generic::parse_janet,
    "src/languages/generic.rs::parse_jinja2" => generic::parse_jinja2,
    "src/languages/generic.rs::parse_jsdoc" => generic::parse_jsdoc,
    "src/languages/generic.rs::parse_json" => generic::parse_json,
    "src/languages/generic.rs::parse_assembly" => generic::parse_assembly,
    "src/languages/generic.rs::parse_astro" => generic::parse_astro,
    "src/languages/generic.rs::parse_beancount" => generic::parse_beancount,
    "src/languages/generic.rs::parse_bibtex" => generic::parse_bibtex,
    "src/languages/generic.rs::parse_blade" => generic::parse_blade,
    "src/languages/generic.rs::parse_css" => generic::parse_css,
    "src/languages/generic.rs::parse_csv" => generic::parse_csv,
    "src/languages/generic.rs::parse_diff" => generic::parse_diff,
    "src/languages/generic.rs::parse_dockerfile" => generic::parse_dockerfile,
    "src/languages/generic.rs::parse_dotenv" => generic::parse_dotenv,
    "src/languages/generic.rs::parse_gitattributes" => generic::parse_gitattributes,
    "src/languages/generic.rs::parse_agda" => generic::parse_agda,
    "src/languages/generic.rs::parse_form" => generic::parse_form,
    "src/languages/generic/csharp.rs::parse_csharp" => generic::csharp::parse_csharp,
    "src/languages/generic/go.rs::parse_go" => |source: &str| generic::go::parse_go(source, false),
    "src/languages/generic/php.rs::parse_php" => generic::php::parse_php,
    "src/languages/go.rs::parse" => |source: &str| languages::go::parse(source, false),
    "src/languages/java.rs::parse" => languages::java::parse,
    "src/languages/php.rs::parse" => languages::php::parse,
    "src/languages/python.rs::parse" => languages::python::parse,
    "src/languages/rust.rs::parse" => languages::rust::parse,
    "src/languages/typescript.rs::parse" => |source: &str| {
        languages::typescript::parse(source, parsers::Language::TypeScript)
    },
    "src/lesson.rs::parse_ledger" => lesson::parse_ledger,
    "src/llama_cpp.rs::parse_llama_cpp_devices" => llama_cpp::parse_llama_cpp_devices,
    "src/llama_cpp.rs::parse_generation_rate" => llama_cpp::parse_generation_rate,
    "src/parsers/mod.rs::parse_file" => |source: &str| {
        parsers::parse_file(parsers::Language::Rust, source, "property.rs")
    },
}

#[test]
fn parser_property_generation_is_reproducible() {
    let config = property_parser_config();
    assert_eq!(config.cases, PROPERTY_PARSER_CASES);
    assert_eq!(config.rng_seed, RngSeed::Fixed(PROPERTY_PARSER_SEED));
}

#[test]
fn direct_tree_sitter_entrypoints_reject_binary_and_control_input() {
    let _guard = parser_test_guard();
    let source = "fn hostile() {}\0";
    assert_eq!(languages::c::parse(source, false), Default::default());
    assert_eq!(languages::cpp::parse(source, false), Default::default());
    assert_eq!(languages::csharp::parse(source), Default::default());
    assert_eq!(languages::go::parse(source, false), Default::default());
    assert_eq!(languages::java::parse(source), Default::default());
    assert_eq!(languages::php::parse(source), Default::default());
    assert_eq!(languages::python::parse(source), Default::default());
    assert_eq!(languages::rust::parse(source), Default::default());
    assert_eq!(
        languages::typescript::parse(source, parsers::Language::TypeScript),
        Default::default()
    );

    let hostile_control_source = "fn hostile() {}\u{1b}\u{7f}";
    assert_eq!(generic::parse_d(hostile_control_source), Default::default());
    // U+202E is a bidi format control rather than `char::is_control()` and
    // previously reached tree-sitter-just's native scanner, which could
    // segfault on Linux instead of returning an empty parse.
    assert_eq!(generic::parse_just("\u{202e}%&/"), Default::default());
    assert_eq!(
        generic::parse_just("\u{fbd6e}\u{44dfc}\r\u{10f335}%$"),
        Default::default()
    );
    assert_eq!(
        generic::parse_odin("\u{90a47}\u{57257}"),
        Default::default()
    );
    assert_eq!(generic::parse_d("module café;"), Default::default());
}
