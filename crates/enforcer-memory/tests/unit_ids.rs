use enforcer_memory::error::Result;
use enforcer_memory::ids::{repo_root, ArtifactId, ProjectId, Seq};

#[test]
fn artifact_id_is_deterministic_and_content_addressed() {
    let a = ArtifactId::from_content(b"hello");
    let b = ArtifactId::from_content(b"hello");
    let c = ArtifactId::from_content(b"world");
    assert_eq!(a, b);
    assert_ne!(a, c);
    assert!(a.as_str().starts_with("sha256:"));
}

#[test]
fn artifact_id_from_digest_wraps_without_recomputing() {
    let computed = ArtifactId::from_content(b"round trip me");
    let rewrapped = ArtifactId::from_digest(computed.digest().clone());
    assert_eq!(computed, rewrapped);
}

#[test]
fn project_id_is_stable_for_the_same_root_and_windows_path_forms() -> Result<()> {
    let a = repo_root(r"C:\Projects\enforcer")?;
    let b = repo_root("C:/Projects/enforcer")?;
    assert_eq!(
        ProjectId::from_repo_root(&a).as_str(),
        ProjectId::from_repo_root(&b).as_str(),
        "backslash and forward-slash forms of the same root must yield the same project id"
    );
    Ok(())
}

#[test]
fn project_id_differs_across_roots() -> Result<()> {
    let a = repo_root("C:/Projects/enforcer")?;
    let b = repo_root("C:/Projects/other")?;
    assert_ne!(
        ProjectId::from_repo_root(&a).as_str(),
        ProjectId::from_repo_root(&b).as_str()
    );
    Ok(())
}

#[test]
fn seq_advances_monotonically_from_genesis() {
    let s0 = Seq::GENESIS;
    let s1 = s0.next();
    let s2 = s1.next();
    assert_eq!(s0.0, 0);
    assert_eq!(s1.0, 1);
    assert_eq!(s2.0, 2);
    assert!(s2 > s1 && s1 > s0);
}
