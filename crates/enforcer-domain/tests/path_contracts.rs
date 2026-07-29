// contractHash: path_contracts.rs
// sourceOwner: enforcer-domain
use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::paths::{RelPath, RepoRoot};

#[test]
fn repo_root_accepts_absolute_forms_and_normalizes() -> Result<(), DecodeError> {
    let win: RepoRoot = r"C:\Projects\enforcer".parse()?;
    assert_eq!(win.as_str(), "C:/Projects/enforcer");
    let posix: RepoRoot = "/home/user/repo".parse()?;
    assert_eq!(posix.as_str(), "/home/user/repo");
    let unc: RepoRoot = r"\\server\share\repo".parse()?;
    assert_eq!(unc.as_str(), "//server/share/repo");
    Ok(())
}

#[test]
fn repo_root_rejects_relative_and_empty() {
    for bad in ["", "  ", "relative/path", "./here", "C:"] {
        let outcome: Result<RepoRoot, _> = bad.parse();
        assert_eq!(
            outcome.as_ref().err().map(|error| error.path.as_str()),
            Some("repoRoot"),
            "should reject {bad:?}"
        );
    }
}

#[test]
fn rel_path_accepts_relative_and_normalizes_backslashes() -> Result<(), DecodeError> {
    let p: RelPath = r"crates\enforcer-domain\src\lib.rs".parse()?;
    assert_eq!(p.as_str(), "crates/enforcer-domain/src/lib.rs");
    let dotted: RelPath = "a/./b".parse()?;
    assert_eq!(dotted.as_str(), "a/./b");
    let contained: RelPath = "a/b/../c".parse()?;
    assert_eq!(contained.as_str(), "a/b/../c");
    Ok(())
}

#[test]
fn rel_path_rejects_absolute_and_escaping() {
    for bad in ["", "/abs/path", r"C:\abs", "../escape", "a/../../escape"] {
        let outcome: Result<RelPath, _> = bad.parse();
        assert_eq!(
            outcome.as_ref().err().map(|error| error.path.as_str()),
            Some("relPath"),
            "should reject {bad:?}"
        );
    }
}

#[test]
fn conversion_boundary_enforces_path_rules() -> Result<(), DecodeError> {
    let ok = RelPath::try_from(String::from("src/lib.rs"))?;
    assert_eq!(ok.as_str(), "src/lib.rs");
    assert_eq!(
        RelPath::try_from(String::from("/abs"))
            .as_ref()
            .err()
            .map(|error| error.path.as_str()),
        Some("relPath")
    );
    assert_eq!(
        RepoRoot::try_from(String::from("not-absolute"))
            .as_ref()
            .err()
            .map(|error| error.path.as_str()),
        Some("repoRoot")
    );
    Ok(())
}

#[test]
fn resolve_joins_root_and_rel_typed_so_only_relpath_is_accepted() -> Result<(), DecodeError> {
    let root: RepoRoot = r"C:\Projects\enforcer".parse()?;
    let rel: RelPath = r"crates\enforcer-domain\src\lib.rs".parse()?;
    assert_eq!(
        root.resolve(&rel),
        "C:/Projects/enforcer/crates/enforcer-domain/src/lib.rs"
    );
    // NOTE: `RepoRoot::resolve` takes `&RelPath` by type, not `&str`;
    // there is no overload accepting a second `RepoRoot`, so
    // `root.resolve(&other_root)` is a compile error, not a runtime
    // check. See `tests/compile_reject` fixtures for the enforced case.
    Ok(())
}

#[test]
fn relativize_strips_root_and_validates_the_remainder() -> Result<(), DecodeError> {
    let root: RepoRoot = "/home/user/repo".parse()?;
    let rel = root.relativize("/home/user/repo/crates/enforcer-domain/src/lib.rs")?;
    assert_eq!(rel.as_str(), "crates/enforcer-domain/src/lib.rs");

    let win_root: RepoRoot = r"C:\Projects\enforcer".parse()?;
    let win_rel = win_root.relativize(r"C:\Projects\enforcer\crates\x\src\lib.rs")?;
    assert_eq!(win_rel.as_str(), "crates/x/src/lib.rs");
    Ok(())
}

#[test]
fn relativize_rejects_paths_outside_the_root_or_equal_to_it() -> Result<(), DecodeError> {
    let root: RepoRoot = "/home/user/repo".parse()?;
    assert_eq!(
        root.relativize("/home/user/other/file.rs")
            .as_ref()
            .err()
            .map(|error| error.path.as_str()),
        Some("relPath")
    );
    assert_eq!(
        root.relativize("/completely/different")
            .as_ref()
            .err()
            .map(|error| error.path.as_str()),
        Some("relPath")
    );
    // Equal to the root itself: no `/` remainder to strip -> rejected,
    // not silently accepted as an empty RelPath.
    assert_eq!(
        root.relativize("/home/user/repo")
            .as_ref()
            .err()
            .map(|error| error.path.as_str()),
        Some("relPath")
    );
    Ok(())
}

#[test]
fn resolve_and_relativize_round_trip() -> Result<(), DecodeError> {
    let root: RepoRoot = "/home/user/repo".parse()?;
    let rel: RelPath = "crates/enforcer-domain/src/paths.rs".parse()?;
    let abs = root.resolve(&rel);
    let round_tripped = root.relativize(&abs)?;
    assert_eq!(round_tripped, rel);
    Ok(())
}
