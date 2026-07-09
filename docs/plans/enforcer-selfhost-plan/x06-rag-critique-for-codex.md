# x06 RAG/model-runtime critique — for Codex

Scope check first, so there's no confusion about who owns what:
- **Graph engine + MCP tools + language parity + continuous-learning ledger (learning.rs, lesson activation, recurrence curves) = mine.** Verified, real, gate-tested.
- **Everything below (embeddings, reranking, retrieval quality, model-runtime, hardware detection, model download/selection) = yours.** This is the critique of that half.

x06 is the core backbone. Half-working proof docs that read like it's done are worse than an honest "not done" — they cost us the moment we build on top of them assuming they work. Every claim below is cited to a real file/line/artifact, not a guess.

## 1. The wired-in embedder is fake

`crate::embed::HashingEmbedder` (`src/embed.rs`) is what's actually connected to the live retrieval path in `search/mod.rs` and `mcp.rs`. It's a deterministic hash-based stand-in — not a neural embedding model, not semantic in any sense. The real path exists (`src/ort_runtime.rs`, real `ort` crate / ONNX Runtime bindings, compiles clean under `--features real-models`) but it is **never connected** to the actual search/retrieval flow that ships. Every retrieval call today runs on the hash fallback regardless of what features are enabled.

**Ask: wire the real ORT embedder into the live retrieval path, with the hash embedder as an explicit, labeled degraded-mode fallback — not the default.**

## 2. The QA-250 proof is misleading as written

`proof/memory/x06-rag-qa.json` reports "205/250 rows green." Read closer:
- **Every single passing row has `"capabilityState": "degraded"`.** That means the graph/lexical fallback passed — not RAG. The proof doc currently has no way to distinguish "RAG passed" from "RAG never ran and the fallback covered for it."
- **45 of 250 rows (18%) have zero test runner wired at all** — verdict is literally `"unrunnable: no wired runner for category X"`. Broken down: Reranking 10/10 missing, Retrieval 8/8 missing, Performance 7, Symbol 4, CodeGraph 4, Architecture 5, Experience 3, Federation 2, TokenReduction 2.
- **Reranking and Retrieval are RAG's entire reason for existing, and both have 0% coverage.** Not failing — literally no attempt.

**Ask: (a) split the proof summary into "passed on real capability" vs "passed on degraded fallback" so nobody reads this doc and thinks RAG is proven; (b) wire real runners for all 9 currently-unrunnable categories, starting with Reranking and Retrieval.**

## 3. No real end-to-end model run has ever been proven

Every "live" model-run proof file shows `"skipped": true`:
- `proof/memory/x06-models-qwen3-embedding-gguf-vulkan-live.json`
- `proof/memory/x06-models-gemma3-4b-vulkan-live.json`
- `proof/memory/x06-models-gemma3-4b-download-live.json`

These files exist with full schema/policy scaffolding (probe order, timeout config, acceleration policy) but no executed result. Nobody has downloaded a real model, run real inference, and captured a passing result in what's committed to the repo. The architecture is real; the proof that it actually works with real weights on real hardware does not exist yet.

**Ask: run these for real, on at least one real machine, and commit the actual (non-skipped) result before calling any of this done.**

## 4. Cross-platform: hardcoded `.exe`, no OS branching — this WILL break on Mac/Linux

`src/runtime_probe.rs`, the default binary-resolution functions:
```rust
fn default_llama_cli() -> Option<PathBuf> { first_existing_model_bin("llama-cli.exe") }
fn default_llama_embedding() -> Option<PathBuf> { first_existing_model_bin("llama-embedding.exe") }
fn default_llama_server() -> Option<PathBuf> { first_existing_model_bin("llama-server.exe") }
```
And the final fallback when no env var is set: `.unwrap_or_else(|| PathBuf::from("llama-embedding.exe"))`. Zero `cfg(target_os)`, zero `EXE_SUFFIX`/`std::env::consts::EXE_SUFFIX` usage anywhere in this file. On Mac/Linux, llama.cpp binaries have no `.exe` extension — this code will never find them unless the user manually sets `ENFORCER_X06_LLAMA_*` env vars every time. That's not portable, that's "works on my Windows machine."

**Ask: use `std::env::consts::EXE_SUFFIX` (empty string on Unix, ".exe" on Windows) everywhere a binary name is constructed, and add a real cross-platform test (or at minimum, three separate CI legs) that doesn't just assume Windows.**

## 5. Cross-platform: the only real-model proof script is Windows PowerShell

`crates/enforcer-memory/scripts/x06-real-model-proof.ps1` — no `.sh` equivalent. If the actual verification procedure for "does the real model path work" only exists as a `.ps1`, it structurally cannot be run on Mac/Linux CI or by a Mac/Linux dev, which contradicts "has to work on all PCs."

**Ask: a portable (bash or Rust-native, e.g. a `cargo xtask` or the example binary itself with clear flags) equivalent that runs identically on Windows/Mac/Linux.**

## 6. llama.cpp integration is a subprocess wrapper, not FFI

`src/llama_cpp.rs` has zero `unsafe`/`extern "C"`/FFI calls — it shells out to an external `llama-cli`/`llama-server` binary and parses stdout/stderr text. This may be a deliberate, defensible choice (simpler, no build-time C++ toolchain dependency), but it's worth stating plainly: the original tab-agent setup reportedly handled llama.cpp via real FFI cleanly. If subprocess-wrapping was a deliberate simplification, fine — but confirm that, and confirm the subprocess approach is equally robust across platforms (binary discovery, process spawning, signal/timeout handling on Windows vs POSIX are not the same).

**Ask: confirm this was intentional, not a shortcut taken under time pressure, and that platform-specific process-handling (kill-on-timeout, path resolution) has been tested on all three OSes, not just assumed to work.**

## 7. Hardware-aware model selection exists only for chat models

`src/hf_cache.rs::select_x06_chat_model_for_hardware(free_vram_mib)` is real — it picks a chat model based on detected VRAM. There is no equivalent selection logic for embedding or reranker models; those appear to be fixed choices (`qwen-embedding-onnx`, `qwen-reranker-onnx`) regardless of what hardware is detected. If the plan includes "detect my system, recommend/auto-download the right model," that plan is currently implemented for exactly one of the three model categories.

**Ask: either extend hardware-aware selection to embedding/reranker models too, or explicitly document that those are fixed-size-only by design (and why).**

## 8. None of this is reachable through the actual product — no MCP tool exposure

This is the one that undercuts everything else: even where the download/detect/select code is real and working, **there is no MCP tool that exposes it.** The only entry points are:
- `crates/enforcer-memory/examples/x06_model_runtime_probe.rs` (a `cargo run --example` binary)
- `crates/enforcer-memory/scripts/x06-real-model-proof.ps1` (a manual Windows script)

Neither is something the actual MCP server (the thing Claude Code or any other harness talks to) can invoke. If the intended experience is "the tool notices it needs semantic search, checks the system, recommends/downloads the right model, and turns it on" — none of that exists as a callable capability today. It's proof-harness-only.

**Ask: at least one MCP tool (or a documented, deliberate decision not to expose one, with a rationale) that lets an actual session trigger system-check → model recommendation → download → activation, end to end.**

## Bottom line

The architecture underneath (ORT bindings, HF download client, hardware VRAM detection, RRF-based rank fusion) is real, not vaporware — that's worth crediting. But as of right now: the live product uses a fake embedder, the proof doc for retrieval quality can't distinguish real success from fallback success, two entire RAG-critical categories have zero test coverage, no real model run has ever been executed and captured, and the whole apparatus only runs on Windows and only from scripts nobody using the actual product would ever touch. This needs to be fixed for real, not documented around.
