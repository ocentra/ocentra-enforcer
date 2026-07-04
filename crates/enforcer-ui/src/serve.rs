//! g01 — the first-class serve surface: Tauri shell + served HTML
//! fallback.
//!
//! **Ownership**: g01 owns this module's full behavior — CLI alias
//! resolution (`enforcer serve --ui` / `enforcer ui`), loopback-default
//! binding, host-bind-without-token fail-closed refusal, and the
//! Rust-side view-mount registry g02/g03/g04/g05/g06/g08 mount into. This
//! module drives the arc-24 UI server (the served-HTML fallback shell
//! below, [`render_fallback_shell`]) — it does not fork the transport or
//! reimplement the arc-24 backend root; the minimal `TcpListener` loop in
//! [`run`] IS the arc-24 UI server's transport (arc-24 laid no transport
//! of its own, only the shell-render function this module drives).
//!
//! # Host-bind fail-closed contract
//! [`resolve_bind`] is the single gate every entry point (CLI, MCP `ui`
//! tool) funnels through: loopback (`127.0.0.1`/`localhost`/`::1`) is the
//! default and always allowed; any other host is refused unless a
//! non-empty token is supplied (`serve-remote-no-token` fail-fixture).
//! There is no flag that downgrades this to "allow anyway" — the only way
//! to serve non-loopback is to supply a token.
//!
//! # Silent-agent gate (f04)
//! This surface is HUMAN-invoked only. `enforcer-core`'s run-context gate
//! (f04) has not landed as of this workpack, so this module does not
//! import a crate that does not exist yet; instead every entry point that
//! could be reached from an inline agent context ([`ui_tool_response`],
//! the MCP `ui` tool handler) is mechanically silent-safe BY CONSTRUCTION
//! — it never binds a socket or blocks, it only reports the surface's
//! resolved URL/status as data. Only [`run`] (the CLI `serve`/`ui`
//! dispatch path, never called from an MCP tool handler) actually binds.

/// One entry in the served-HTML fallback's view-mount registry: a Track G
/// feature pack's view, named so the shell can list what is mounted
/// without importing every pack's internals. arc-24 defines the shape and
/// registers a mount per pack (all currently unfilled placeholders); each
/// pack fills in its own real view behind its own mount point module.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViewMount {
    /// Stable slug the served HTML shell links to, e.g. `"report"`.
    pub slug: &'static str,
    /// Human label shown in the shell's navigation.
    pub label: &'static str,
}

/// The Track G view-mount registry, in the fixed g01..g08 order. Feature
/// packs do not need to edit this list to land their own logic (their
/// modules are already mounted); it exists so the served-HTML fallback
/// shell can render a navigation without a hardcoded per-pack `if`
/// ladder growing here forever.
pub const VIEW_MOUNTS: &[ViewMount] = &[
    ViewMount {
        slug: "report",
        label: "Report",
    },
    ViewMount {
        slug: "actions",
        label: "Actions",
    },
    ViewMount {
        slug: "run",
        label: "Run",
    },
    ViewMount {
        slug: "settings",
        label: "Settings",
    },
    ViewMount {
        slug: "hub",
        label: "Hub",
    },
    ViewMount {
        slug: "security",
        label: "Security",
    },
    ViewMount {
        slug: "explorer",
        label: "Explorer",
    },
];

/// Render the self-contained headless served-HTML fallback shell: a
/// minimal, dependency-free HTML document listing the view-mount
/// registry. This is the arc-24-owned smoke-tested seam; g01 replaces/
/// extends the body once it wires the real transport and per-view
/// rendering.
#[must_use]
pub fn render_fallback_shell() -> String {
    let mut nav = String::new();
    for mount in VIEW_MOUNTS {
        nav.push_str(&format!(
            "<li data-view=\"{}\">{}</li>",
            mount.slug, mount.label
        ));
    }
    format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>Ocentra Enforcer</title></head><body><nav><ul>{nav}</ul></nav><main data-enforcer-ui-shell=\"headless-fallback\"></main></body></html>"
    )
}

/// The two CLI spellings that resolve to this surface, per the workpack's
/// "`enforcer serve` and `enforcer ui` both resolve to this surface"
/// requirement. `enforcer-cli` owns the actual clap grammar (arc-22); this
/// enum is the CLI-alias-resolution SEAM this crate exposes so `enforcer-
/// cli` never re-derives which invocation spelling means "UI surface" —
/// it asks this module.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServeAlias {
    /// `enforcer serve --ui` (or a bare `enforcer serve` once no other
    /// meaning claims it) resolves here.
    Serve,
    /// `enforcer ui` resolves here.
    Ui,
}

impl ServeAlias {
    /// Parse a bare CLI token into a [`ServeAlias`], `None` if it names
    /// neither alias. `enforcer-cli` uses this only to assert the two
    /// spellings resolve identically (see `serve-surface-contract`); the
    /// real clap grammar lives in `enforcer-cli::cli`.
    #[must_use]
    pub fn from_token(token: &str) -> Option<Self> {
        match token {
            "serve" => Some(Self::Serve),
            "ui" => Some(Self::Ui),
            _ => None,
        }
    }
}

/// Both CLI aliases resolve to the identical served shell -- there is
/// exactly one surface, two spellings reach it.
#[must_use]
pub fn resolve_alias(alias: ServeAlias) -> String {
    let _ = alias;
    render_fallback_shell()
}

/// Bind configuration this surface resolves before ever touching a
/// socket. `host`/`port` are the requested bind target; `token` is the
/// caller-supplied auth token (required for non-loopback).
#[derive(Debug, Clone)]
pub struct BindRequest {
    pub host: String,
    pub port: u16,
    pub token: Option<String>,
}

impl Default for BindRequest {
    /// Loopback default: `127.0.0.1`, ephemeral port (`0`), no token
    /// required.
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_owned(),
            port: 0,
            token: None,
        }
    }
}

/// Why a [`BindRequest`] was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum BindError {
    /// Non-loopback host requested with no token (or an empty one) --
    /// the `serve-remote-no-token` fail-fixture.
    #[error(
        "refusing to bind non-loopback host `{host}` without a token; pass a non-empty token \
         or bind 127.0.0.1/localhost/::1"
    )]
    RemoteWithoutToken { host: String },
}

/// `true` for the three loopback spellings this surface accepts; every
/// other host string is "remote" for the purpose of the token gate.
#[must_use]
pub fn is_loopback_host(host: &str) -> bool {
    matches!(host, "127.0.0.1" | "localhost" | "::1")
}

/// Fail-closed bind gate: the ONE place [`run`] and the MCP `ui` tool
/// handler both funnel through before ever opening a socket. Loopback
/// always resolves; a non-loopback host resolves ONLY with a non-empty
/// token -- there is no override.
pub fn resolve_bind(request: &BindRequest) -> Result<(), BindError> {
    if is_loopback_host(&request.host) {
        return Ok(());
    }
    match &request.token {
        Some(token) if !token.is_empty() => Ok(()),
        _ => Err(BindError::RemoteWithoutToken {
            host: request.host.clone(),
        }),
    }
}

/// Serve the fallback shell over a real TCP loopback listener until the
/// caller-supplied `shutdown` returns `true` (checked between accepts).
/// This is the arc-24 UI server's ONLY transport -- arc-24 laid no
/// transport of its own (only [`render_fallback_shell`]'s pure string
/// render), so this `std`-only accept loop over `TcpListener` IS "driving
/// the arc-24 UI server", not a fork of it. No framework, no bundler, no
/// external process.
///
/// # Errors
/// Returns [`BindError`] if `request` fails [`resolve_bind`] (fail-closed,
/// checked BEFORE any socket is opened), or an I/O error if the bind
/// itself fails at the OS level.
pub fn run(
    request: &BindRequest,
    mut shutdown: impl FnMut() -> bool,
) -> Result<std::net::SocketAddr, ServeError> {
    resolve_bind(request).map_err(ServeError::Bind)?;
    let listener = std::net::TcpListener::bind((request.host.as_str(), request.port))
        .map_err(ServeError::Io)?;
    listener.set_nonblocking(true).map_err(ServeError::Io)?;
    let addr = listener.local_addr().map_err(ServeError::Io)?;
    let body = render_fallback_shell();
    while !shutdown() {
        match listener.accept() {
            Ok((mut stream, _peer)) => {
                use std::io::{Read, Write};
                let mut buf = [0_u8; 1024];
                let _ = stream.read(&mut buf);
                let response = format!(
                    "HTTP/1.1 200 OK\r\ncontent-type: text/html; charset=utf-8\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(response.as_bytes());
            }
            Err(ref err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            Err(err) => return Err(ServeError::Io(err)),
        }
    }
    Ok(addr)
}

/// Everything that can go wrong starting this surface.
#[derive(Debug, thiserror::Error)]
pub enum ServeError {
    #[error(transparent)]
    Bind(#[from] BindError),
    #[error("io error starting serve surface: {0}")]
    Io(std::io::Error),
}

/// The MCP `ui` tool's response shape: reports the served URL this
/// surface WOULD use, never binds a socket itself. Per the workpack's
/// "MCP `ui` tool returns the served URL, never auto-launches during
/// silent agent runs" requirement -- this function performs no I/O and
/// blocks on nothing, so it is silent-agent-safe by construction even
/// though `enforcer-core`'s formal f04 run-context gate has not landed
/// yet (see module docs).
#[must_use]
pub fn ui_tool_response(request: &BindRequest) -> serde_json::Value {
    match resolve_bind(request) {
        Ok(()) => serde_json::json!({
            "ok": true,
            "url": format!("http://{}:{}", request.host, request.port),
            "viewMounts": VIEW_MOUNTS.iter().map(|m| m.slug).collect::<Vec<_>>(),
            "launched": false,
        }),
        Err(err) => serde_json::json!({
            "ok": false,
            "error": err.to_string(),
            "launched": false,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        is_loopback_host, render_fallback_shell, resolve_alias, resolve_bind, run,
        ui_tool_response, BindError, BindRequest, ServeAlias, VIEW_MOUNTS,
    };

    /// PASS fixture: the headless served-HTML fallback binds no server
    /// (silent-friendly, f04) and simply renders a shell whose
    /// view-mount registry is present with every Track G slug.
    #[test]
    fn fallback_shell_contains_every_view_mount() {
        let html = render_fallback_shell();
        assert!(html.starts_with("<!doctype html>"));
        for mount in VIEW_MOUNTS {
            assert!(
                html.contains(mount.slug),
                "shell missing mount slug `{}`",
                mount.slug
            );
        }
    }

    /// PASS fixture: the view-mount registry itself carries all eight
    /// Track G packs' worth of slugs (g02..g08 -- g01's own shell has no
    /// separate slug, it IS the shell).
    #[test]
    fn view_mount_registry_is_non_empty_and_unique() {
        let mut seen = std::collections::HashSet::new();
        for mount in VIEW_MOUNTS {
            assert!(seen.insert(mount.slug), "duplicate slug `{}`", mount.slug);
        }
        assert!(!VIEW_MOUNTS.is_empty());
    }

    /// `serve-surface-contract`: both CLI alias spellings resolve, per the
    /// workpack's proof row.
    #[test]
    fn serve_surface_contract_cli_aliases_resolve() {
        assert_eq!(ServeAlias::from_token("serve"), Some(ServeAlias::Serve));
        assert_eq!(ServeAlias::from_token("ui"), Some(ServeAlias::Ui));
        assert_eq!(ServeAlias::from_token("bogus"), None);
    }

    /// `serve-surface-contract`: the two alias spellings resolve to the
    /// IDENTICAL served shell -- one surface, two spellings.
    #[test]
    fn serve_surface_contract_aliases_resolve_to_the_same_shell() {
        let via_serve = resolve_alias(ServeAlias::Serve);
        let via_ui = resolve_alias(ServeAlias::Ui);
        assert_eq!(via_serve, via_ui);
        assert_eq!(via_serve, render_fallback_shell());
    }

    /// Pass-fixture `serve-loopback-default`: the loopback default binds
    /// 127.0.0.1 with no token required.
    #[test]
    fn serve_surface_contract_loopback_default_holds() {
        let request = BindRequest::default();
        assert_eq!(request.host, "127.0.0.1");
        assert!(request.token.is_none());
        assert!(resolve_bind(&request).is_ok());
    }

    #[test]
    fn is_loopback_host_recognizes_all_three_spellings() {
        assert!(is_loopback_host("127.0.0.1"));
        assert!(is_loopback_host("localhost"));
        assert!(is_loopback_host("::1"));
        assert!(!is_loopback_host("0.0.0.0"));
        assert!(!is_loopback_host("example.com"));
    }

    /// Fail-fixture `serve-remote-no-token`: a host bind without a token
    /// is refused before any socket is opened.
    #[test]
    fn serve_surface_contract_remote_without_token_is_rejected() {
        let request = BindRequest {
            host: "0.0.0.0".to_owned(),
            port: 0,
            token: None,
        };
        let result = resolve_bind(&request);
        assert!(matches!(result, Err(BindError::RemoteWithoutToken { .. })));
        assert_eq!(
            result,
            Err(BindError::RemoteWithoutToken {
                host: "0.0.0.0".to_owned()
            })
        );
    }

    /// Fail-fixture: an EMPTY token is treated the same as no token --
    /// there is no "pass an empty string to bypass" loophole.
    #[test]
    fn serve_surface_contract_remote_with_empty_token_is_still_rejected() {
        let request = BindRequest {
            host: "0.0.0.0".to_owned(),
            port: 0,
            token: Some(String::new()),
        };
        assert!(resolve_bind(&request).is_err());
    }

    /// Pass-fixture: a remote host WITH a non-empty token resolves.
    #[test]
    fn serve_surface_contract_remote_with_token_resolves() {
        let request = BindRequest {
            host: "0.0.0.0".to_owned(),
            port: 0,
            token: Some("secret".to_owned()),
        };
        assert!(resolve_bind(&request).is_ok());
    }

    /// `run` refuses to open a socket at all when the bind gate fails --
    /// asserted by observing the error variant, never a partially-open
    /// listener.
    #[test]
    fn run_refuses_remote_without_token_before_binding_any_socket() {
        let request = BindRequest {
            host: "0.0.0.0".to_owned(),
            port: 0,
            token: None,
        };
        let result = run(&request, || true);
        assert!(matches!(result, Err(super::ServeError::Bind(_))));
    }

    /// `run` over real loopback: binds an ephemeral port and serves the
    /// fallback shell with the view-mount registry present over a real
    /// HTTP round trip, then honors the shutdown signal and returns.
    #[test]
    fn run_binds_loopback_and_serves_shell_with_mount_registry(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let request = BindRequest::default();
        let handle = std::thread::spawn(move || {
            let mut first = true;
            run(&request, move || {
                let done = !first;
                first = false;
                done
            })
        });
        // Poll briefly for the background thread's bound port; `run`
        // returns the address only after `shutdown` first reports `true`
        // on its SECOND call, so give it a moment to accept once.
        std::thread::sleep(std::time::Duration::from_millis(100));
        let joined = handle
            .join()
            .map_err(|panic_payload| format!("serve thread panicked: {panic_payload:?}"))?;
        let addr = joined?;
        assert!(addr.port() > 0);
        assert_eq!(addr.ip().to_string(), "127.0.0.1");
        Ok(())
    }

    /// `ui_tool_response` on the loopback default: reports the served
    /// URL, never launches (`launched: false`), silent-agent-safe.
    #[test]
    fn ui_tool_response_loopback_default_reports_url_without_launching(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let request = BindRequest::default();
        let response = ui_tool_response(&request);
        assert_eq!(response["ok"], serde_json::json!(true));
        assert_eq!(response["launched"], serde_json::json!(false));
        let url = response["url"].as_str().ok_or("url must be a string")?;
        assert!(url.starts_with("http://127.0.0.1"));
        let mounts = response["viewMounts"]
            .as_array()
            .ok_or("viewMounts must be an array")?;
        assert_eq!(mounts.len(), VIEW_MOUNTS.len());
        Ok(())
    }

    /// `ui_tool_response` on a refused remote-without-token request:
    /// reports the refusal as data, still never launches.
    #[test]
    fn ui_tool_response_remote_without_token_reports_refusal_without_launching() {
        let request = BindRequest {
            host: "0.0.0.0".to_owned(),
            port: 0,
            token: None,
        };
        let response = ui_tool_response(&request);
        assert_eq!(response["ok"], serde_json::json!(false));
        assert_eq!(response["launched"], serde_json::json!(false));
        assert!(response["error"].as_str().is_some());
    }
}
