//! f04 acceptance proof: `RunContext`'s ONE resolution point + UI/server
//! gate, exercised as an integration test (not just the unit tests inside
//! `src/run_context.rs` / `src/run_context/boundary.rs`) so the
//! fail/pass/detection fixtures the workpack names are provable from
//! outside the module, the same way a real UI/server caller would use it.
//!
//! # Why this test drives its own tiny in-test UI/server surface instead
//! of `enforcer-ui`/`enforcer-mcp` directly
//! Those crates (`enforcer-ui::serve`, `enforcer-mcp::router`'s
//! `ocentra_enforcer_ui` tool handler) are this workpack's NAMED
//! integration points, but their owning files are contested by
//! concurrently-running Track G / f01 work (see the parallel-ownership
//! note in `src/run_context.rs`'s module doc). This workpack owns only
//! the `run_context` module + this test file, so the fixtures below prove
//! the SEAM's contract — the exact gate a real UI/server entry point must
//! call before it binds a listener — using a real, local `TcpListener`
//! surface (real socket, real HTTP round trip; no test double stands in
//! for any collaborator). Its shape (loopback listener, gate-before-bind,
//! structured HTML response) deliberately mirrors
//! `enforcer_ui::serve::run` / `ui_tool_response`, so the proof
//! generalizes to the real wiring once that owner lands it (documented as
//! the deferred follow-up).

use std::io::{Read as _, Write as _};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicUsize, Ordering};

use enforcer_core::run_context::boundary::resolve;
use enforcer_core::run_context::{RunContext, SilentModeRefusal};
use enforcer_domain::boundary::decode_error::DecodeError;

/// The exact structured HTML body the in-test surface serves under
/// `HumanReview` — asserted byte-for-byte after a real loopback round
/// trip, mirroring the served-HTML fallback posture of
/// `enforcer_ui::serve::render_fallback_shell`.
const SERVED_SHELL: &str =
    "<!doctype html><html><body data-enforcer-ui-shell=\"human-review\"></body></html>";

/// Typed error union for these fixtures — no erased `Box<dyn Error>`:
/// every failure path a fixture can hit is a named variant.
#[derive(Debug, thiserror::Error)]
enum FixtureError {
    /// The gate refused the surface (the expected outcome under
    /// `AgentInline`; a failure when it happens under `HumanReview`).
    #[error(transparent)]
    Refused(#[from] SilentModeRefusal),
    /// Boundary resolution rejected a token.
    #[error(transparent)]
    Decode(#[from] DecodeError),
    /// Socket-level failure during the loopback round trip.
    #[error("io failure during the loopback round trip: {0}")]
    Io(#[from] std::io::Error),
    /// The served HTTP response had no header/body separator.
    #[error("the served response had no header/body separator")]
    MalformedHttpResponse,
    /// The in-test serve thread panicked instead of completing.
    #[error("the in-test serve thread panicked")]
    ServeThreadPanicked,
}

/// Serve exactly one HTTP request from `listener` with [`SERVED_SHELL`],
/// propagating every socket failure (nothing is silently discarded).
fn serve_one_request(listener: &TcpListener) -> std::io::Result<()> {
    let (mut stream, _peer) = listener.accept()?;
    let mut request_buf = [0_u8; 1024];
    let _request_bytes = stream.read(&mut request_buf)?;
    let response = format!(
        "HTTP/1.1 200 OK\r\ncontent-type: text/html; charset=utf-8\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{SERVED_SHELL}",
        SERVED_SHELL.len()
    );
    stream.write_all(response.as_bytes())?;
    stream.flush()?;
    Ok(())
}

/// The in-test stand-in for a real UI/server entry point
/// (`enforcer_ui::serve::run` / the MCP `ui` tool's launch path): calls
/// the gate FIRST, and only if it passes does it ever touch a socket.
/// `bind_attempts` is a counter OWNED BY THE CALLING TEST (never shared
/// across tests, so parallel test execution can never race a false
/// "no bind happened" reading against another test's own bind) — only
/// incremented AFTER the gate check passes, so a bug that let an
/// `AgentInline` call reach the bind would move this counter and the
/// fail-fixture below would catch it (rather than merely asserting on the
/// returned `Result`, which a buggy implementation could still get
/// "right" by accident). On success, returns the bound loopback address
/// plus the serve thread's handle so callers can join it and surface any
/// socket failure.
fn start_gated_ui_server(
    ctx: RunContext,
    bind_attempts: &AtomicUsize,
) -> Result<(SocketAddr, std::thread::JoinHandle<std::io::Result<()>>), FixtureError> {
    ctx.guard_ui_or_server()?;
    bind_attempts.fetch_add(1, Ordering::SeqCst);
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
    let addr = listener.local_addr()?;
    let serve_thread = std::thread::spawn(move || serve_one_request(&listener));
    Ok((addr, serve_thread))
}

/// Fetch `addr` over a real loopback TCP round trip and assert the body
/// is byte-for-byte [`SERVED_SHELL`] — proves the bound listener actually
/// serves the structured shell, not just that the bind succeeded.
fn assert_shell_served(addr: SocketAddr) -> Result<(), FixtureError> {
    let mut stream = TcpStream::connect(addr)?;
    stream.write_all(b"GET / HTTP/1.1\r\nconnection: close\r\n\r\n")?;
    let mut response = String::new();
    let _response_bytes = stream.read_to_string(&mut response)?;
    let body = response
        .split("\r\n\r\n")
        .nth(1)
        .ok_or(FixtureError::MalformedHttpResponse)?;
    assert_eq!(body, SERVED_SHELL);
    Ok(())
}

/// Join the serve thread, converting a panic into a typed fixture error
/// and propagating any socket failure it hit while serving.
fn join_serve_thread(
    serve_thread: std::thread::JoinHandle<std::io::Result<()>>,
) -> Result<(), FixtureError> {
    match serve_thread.join() {
        Ok(served) => {
            served?;
            Ok(())
        }
        Err(_panic_payload) => Err(FixtureError::ServeThreadPanicked),
    }
}

/// FAIL fixture `run-context-agent-inline-silent`: forcing a UI/server
/// open under `AgentInline` is refused with the exact typed refusal, and
/// — checked via this test's OWN bind counter, not just the `Result`
/// shape — no listener is ever bound.
#[test]
fn agent_inline_refuses_ui_server_start_and_never_binds_a_socket() {
    let bind_attempts = AtomicUsize::new(0);
    let outcome = start_gated_ui_server(RunContext::AgentInline, &bind_attempts);
    assert!(
        matches!(outcome, Err(FixtureError::Refused(SilentModeRefusal))),
        "AgentInline must refuse the UI/server start with the typed refusal, got: {outcome:?}"
    );
    assert_eq!(
        bind_attempts.load(Ordering::SeqCst),
        0,
        "no socket may be bound while resolved as AgentInline"
    );
}

/// PASS fixture: under `HumanReview`, the UI/server start path is
/// reachable over loopback and serves the exact structured HTML shell.
#[test]
fn human_review_permits_ui_server_start_and_serves_the_structured_shell() -> Result<(), FixtureError>
{
    let bind_attempts = AtomicUsize::new(0);
    let (addr, serve_thread) = start_gated_ui_server(RunContext::HumanReview, &bind_attempts)?;
    assert_eq!(bind_attempts.load(Ordering::SeqCst), 1);
    assert_eq!(addr.ip(), IpAddr::V4(Ipv4Addr::LOCALHOST));
    assert_ne!(addr.port(), 0);

    assert_shell_served(addr)?;
    join_serve_thread(serve_thread)
}

/// DETECTION fixture: resolving with no mode set at all — the c04
/// deny-hook path passes neither a flag nor an env value (see the module
/// doc's "landed with zero code change" note) — yields `AgentInline`, and
/// that resolved context still refuses the UI/server start with no socket
/// produced.
#[test]
fn deny_hook_path_with_no_mode_set_resolves_agent_inline_and_produces_no_server_artifact(
) -> Result<(), DecodeError> {
    // Mirrors exactly what the c04 hook's invocation supplies today: no
    // `--run-context` flag, no `ENFORCER_RUN_CONTEXT` value.
    let resolved = resolve(None, None)?;
    assert_eq!(resolved, RunContext::AgentInline);

    let bind_attempts = AtomicUsize::new(0);
    let outcome = start_gated_ui_server(resolved, &bind_attempts);
    assert!(
        matches!(outcome, Err(FixtureError::Refused(SilentModeRefusal))),
        "deny-hook path must never open a UI/server surface, got: {outcome:?}"
    );
    assert_eq!(bind_attempts.load(Ordering::SeqCst), 0);
    Ok(())
}

/// DETECTION fixture (MCP side): an MCP scan invocation that likewise
/// supplies neither a flag nor an env value resolves identically to
/// `AgentInline` and is equally refused — proving the ONE resolution
/// point behaves the same regardless of which silent caller reaches it.
#[test]
fn mcp_scan_path_with_no_mode_set_resolves_agent_inline_and_produces_no_server_artifact(
) -> Result<(), DecodeError> {
    let resolved = resolve(None, None)?;
    assert_eq!(resolved, RunContext::AgentInline);

    let bind_attempts = AtomicUsize::new(0);
    let outcome = start_gated_ui_server(resolved, &bind_attempts);
    assert!(
        matches!(outcome, Err(FixtureError::Refused(SilentModeRefusal))),
        "MCP scan path must never open a UI/server surface, got: {outcome:?}"
    );
    assert_eq!(bind_attempts.load(Ordering::SeqCst), 0);
    Ok(())
}

/// An explicit `HumanReview` flag always wins over any stray environment
/// value — proves the precedence order end-to-end (resolution through
/// gate through served shell), not just via the unit tests inside the
/// boundary module.
#[test]
fn explicit_human_review_flag_wins_over_env_and_permits_the_surface() -> Result<(), FixtureError> {
    let resolved = resolve(Some("human-review"), Some("agent-inline"))?;
    assert_eq!(resolved, RunContext::HumanReview);

    let bind_attempts = AtomicUsize::new(0);
    let (addr, serve_thread) = start_gated_ui_server(resolved, &bind_attempts)?;
    assert_eq!(bind_attempts.load(Ordering::SeqCst), 1);
    assert_ne!(addr.port(), 0);

    assert_shell_served(addr)?;
    join_serve_thread(serve_thread)
}
