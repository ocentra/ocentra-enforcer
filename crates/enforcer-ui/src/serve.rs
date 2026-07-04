//! g01 mount point — Tauri shell + served HTML fallback.
//!
//! **Ownership**: g01 owns this module's full behavior (CLI alias
//! resolution, loopback-default binding, the vendored HTTP core reuse,
//! host-bind-without-token refusal). This workpack (arc-24) lays only the
//! minimal self-contained served-HTML fallback shell below — enough for
//! its own proof row's "served-HTML fallback smoke test passes" line —
//! and the view-mount registry g01/g02.../g08 register into. Do not add
//! Tauri command wiring or transport/bind logic here; that is g01's.

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

#[cfg(test)]
mod tests {
    use super::{render_fallback_shell, VIEW_MOUNTS};

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
}
