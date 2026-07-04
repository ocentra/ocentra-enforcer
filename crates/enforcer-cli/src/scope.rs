//! Bridges the clap-parsed [`crate::cli::ScopeArgs`] to
//! `enforcer_scan::scope::ScopeRequest`. The `enforcer-scan` `ArgGroup`
//! already enforces "exactly one of paths/all/base+head" at parse time;
//! this module only maps the already-validated shape across the crate
//! boundary and catches the one case clap's group cannot express: `check`
//! invoked with none of the three (empty paths, no `--all`, no
//! `--base`/`--head`), which is a usage error, not a silent
//! "scan nothing".

use enforcer_scan::scope::ScopeRequest;

use crate::cli::ScopeArgs;

/// Map already-clap-validated [`ScopeArgs`] to a [`ScopeRequest`].
///
/// # Errors
/// Returns a usage-error message when none of the three scope modes was
/// supplied (empty paths, no `--all`, no `--base`/`--head`).
pub fn resolve_request(args: &ScopeArgs) -> Result<ScopeRequest, String> {
    if let (Some(base), Some(head)) = (&args.base, &args.head) {
        return Ok(ScopeRequest::Diff {
            base: base
                .parse()
                .map_err(|e: enforcer_core::error::DecodeError| e.to_string())?,
            head: head
                .parse()
                .map_err(|e: enforcer_core::error::DecodeError| e.to_string())?,
        });
    }
    if args.all {
        return Ok(ScopeRequest::All);
    }
    if !args.paths.is_empty() {
        return Ok(ScopeRequest::Paths(args.paths.clone()));
    }
    Err("no scope given: pass <paths...>, --all, or --base <sha> --head <sha>".to_owned())
}

#[cfg(test)]
mod tests {
    use super::resolve_request;
    use crate::cli::ScopeArgs;
    use enforcer_scan::scope::ScopeRequest;
    use std::path::PathBuf;

    fn args(paths: Vec<PathBuf>, all: bool, base: Option<&str>, head: Option<&str>) -> ScopeArgs {
        ScopeArgs {
            paths,
            all,
            base: base.map(str::to_owned),
            head: head.map(str::to_owned),
        }
    }

    #[test]
    fn paths_mode_maps_through() -> Result<(), Box<dyn std::error::Error>> {
        let request = resolve_request(&args(vec![PathBuf::from("src/lib.rs")], false, None, None))?;
        assert!(matches!(request, ScopeRequest::Paths(p) if p.len() == 1));
        Ok(())
    }

    #[test]
    fn all_mode_maps_through() -> Result<(), Box<dyn std::error::Error>> {
        let request = resolve_request(&args(vec![], true, None, None))?;
        assert!(matches!(request, ScopeRequest::All));
        Ok(())
    }

    #[test]
    fn diff_mode_maps_through() -> Result<(), Box<dyn std::error::Error>> {
        let request = resolve_request(&args(vec![], false, Some("main"), Some("HEAD")))?;
        match request {
            ScopeRequest::Diff { base, head } => {
                assert_eq!(base.as_str(), "main");
                assert_eq!(head.as_str(), "HEAD");
            }
            other => return Err(format!("expected Diff, got {other:?}").into()),
        }
        Ok(())
    }

    #[test]
    fn no_scope_at_all_is_a_usage_error() {
        assert!(resolve_request(&args(vec![], false, None, None)).is_err());
    }
}
