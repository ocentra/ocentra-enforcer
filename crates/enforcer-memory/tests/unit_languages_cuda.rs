//! Hard tests for CUDA, onboarded directly through the generic
//! spec-table engine ([`enforcer_memory::languages::generic::parse_cuda`])
//! by reusing [`enforcer_memory::languages::spec::LangSpec::cpp`]/
//! [`enforcer_memory::languages::generic::cpp_quirks`] verbatim -- there
//! is no bespoke `languages::cuda` extractor to prove zero-regression
//! against (CUDA has never had one in this crate, and the baseline
//! itself has no dedicated CUDA extractor either -- it reuses C++'s node-
//! type arrays too, see `LangSpec::cuda`'s own doc comment). These tests
//! assert against the grammar-shape ground truth recorded there: a
//! `__global__` kernel definition, a `__device__` free function, a class
//! with `__host__ __device__` methods, `#include`, an ordinary call, and
//! a kernel-launch call (`addKernel<<<1, n>>>(...)`) whose own
//! `kernel_call_syntax` launch-configuration clause must not disturb the
//! ordinary callee/argument extraction C++'s own quirk already performs.

use enforcer_memory::languages::generic::parse_cuda;
use enforcer_memory::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_cuda";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_memory::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_global_kernel_as_function() {
    let src = r#"
__global__ void addKernel(int *a, int n) {
}
"#;
    let parsed = parse_cuda(src, false);
    assert_eq!(
        symbol_kind(&parsed.symbols, "addKernel"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_device_function() {
    let src = r#"
__device__ int helper(int x) {
    return x * 2;
}
"#;
    let parsed = parse_cuda(src, false);
    assert_eq!(
        symbol_kind(&parsed.symbols, "helper"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_class_with_host_device_methods() {
    let src = r#"
class Widget {
public:
    __host__ __device__ Widget(float x) : x_(x) {}
    __host__ __device__ float value() const { return x_; }
private:
    float x_;
};
"#;
    let parsed = parse_cuda(src, false);
    let kinds: Vec<(&str, SymbolKind)> = parsed
        .symbols
        .iter()
        .map(|s| (s.name.as_str(), s.kind))
        .collect();
    assert!(kinds.contains(&("Widget", SymbolKind::Class)));
    assert!(kinds.contains(&("value", SymbolKind::Method)));
}

#[test]
fn extracts_include_import() {
    let src = r#"
#include <cuda_runtime.h>
"#;
    let parsed = parse_cuda(src, false);
    let paths: Vec<&str> = parsed
        .imports
        .iter()
        .map(|i| i.module_path.as_str())
        .collect();
    assert!(paths.contains(&"cuda_runtime.h"));
}

#[test]
fn extracts_ordinary_call() -> TestResult {
    let src = r#"
__device__ int helper(int x) {
    return x * 2;
}

void caller() {
    helper(5);
}
"#;
    let parsed = parse_cuda(src, false);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "helper")
        .ok_or("expected a helper call")?;
    assert_eq!(call.from_symbol.as_deref(), Some("caller"), "{call:?}");
    Ok(())
}

#[test]
fn kernel_launch_syntax_does_not_disturb_callee_or_args() -> TestResult {
    // `<<<...>>>` sits as an unfielded child of `call_expression` between
    // the callee and the argument list -- `cpp_call_override`/
    // `cpp_quirk`'s existing field-based reads (`function`/`arguments`)
    // must still find the bare callee and the full, correct argument
    // list, with the launch-configuration clause simply skipped over as
    // extra unread sibling data. See `LangSpec::cuda`'s own doc comment
    // for the real-parse-tree confirmation this test encodes.
    let src = r#"
void launch(int *d_a, int *d_b, int *d_c, int n) {
    addKernel<<<1, n>>>(d_a, d_b, d_c, n);
}
"#;
    let parsed = parse_cuda(src, false);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "addKernel")
        .ok_or("expected an addKernel call with the bare callee text")?;
    assert_eq!(call.arg_texts, vec!["d_a", "d_b", "d_c", "n"], "{call:?}");
    Ok(())
}

#[test]
fn is_test_file_reclassifies_functions_and_methods_as_test() {
    let src = r#"
__global__ void addKernel(int n) {
}
"#;
    let parsed = parse_cuda(src, true);
    assert_eq!(
        symbol_kind(&parsed.symbols, "addKernel"),
        Some(&SymbolKind::Test),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_cuda("__global__ void ((( this is not valid cuda @@@", false);
    let _ = parsed;
}

#[test]
fn fixture_file_parses_without_panic() -> TestResult {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture = manifest_dir.join(FIXTURE_DIR).join("widget.cu");
    let src = fs::read_to_string(&fixture)?;
    let parsed = parse_cuda(&src, false);
    assert!(
        parsed.symbols.iter().any(|s| s.name == "addKernel"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed.symbols.iter().any(|s| s.name == "helper"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed.symbols.iter().any(|s| s.name == "Widget"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed
            .imports
            .iter()
            .any(|i| i.module_path == "cuda_runtime.h"),
        "{:?}",
        parsed.imports
    );
    assert!(
        parsed.calls.iter().any(|c| c.callee == "addKernel"),
        "{:?}",
        parsed.calls
    );
    Ok(())
}
