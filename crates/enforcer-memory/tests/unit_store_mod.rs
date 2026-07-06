use enforcer_domain::paths::RepoRoot;
use enforcer_memory::error::{MemoryError, Result};
use enforcer_memory::ids::ProjectId;
use enforcer_memory::store::Store;
use std::path::PathBuf;

fn temp_dir(name: &str) -> PathBuf {
    let unique = format!(
        "enforcer-memory-store-{}-{}-{name}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default()
    );
    std::env::temp_dir().join(unique)
}

#[test]
fn opening_an_unknown_project_errors_and_creates_nothing() -> Result<()> {
    let stores_dir = temp_dir("unknown");
    let root: RepoRoot =
        "C:/Projects/never-initialized"
            .parse()
            .map_err(|source| MemoryError::InvalidPath {
                path: "C:/Projects/never-initialized".to_owned(),
                source,
            })?;
    let outcome = Store::open(&stores_dir, &root);
    assert!(matches!(outcome, Err(MemoryError::UnknownProject { .. })));
    assert!(
        !stores_dir.exists(),
        "opening an unknown project must not create any directory"
    );
    Ok(())
}

#[test]
fn init_then_open_round_trips_the_same_project() -> Result<()> {
    let stores_dir = temp_dir("roundtrip");
    let root: RepoRoot =
        "C:/Projects/roundtrip-demo"
            .parse()
            .map_err(|source| MemoryError::InvalidPath {
                path: "C:/Projects/roundtrip-demo".to_owned(),
                source,
            })?;
    {
        let mut store = Store::init(&stores_dir, &root, "2026-07-04T00:00:00Z")?;
        assert_eq!(store.observation_log_mut().high_watermark(), 0);
    }
    let store = Store::open(&stores_dir, &root)?;
    assert_eq!(
        store.project_id().as_str(),
        ProjectId::from_repo_root(&root).as_str()
    );
    std::fs::remove_dir_all(&stores_dir).map_err(|source| MemoryError::Io {
        path: stores_dir,
        source,
    })?;
    Ok(())
}

#[test]
fn init_is_idempotent() -> Result<()> {
    let stores_dir = temp_dir("idempotent");
    let root: RepoRoot =
        "C:/Projects/idempotent-demo"
            .parse()
            .map_err(|source| MemoryError::InvalidPath {
                path: "C:/Projects/idempotent-demo".to_owned(),
                source,
            })?;
    Store::init(&stores_dir, &root, "2026-07-04T00:00:00Z")?;
    // Second init on the same project must not error and must not
    // reset anything.
    let mut store = Store::init(&stores_dir, &root, "2026-07-04T01:00:00Z")?;
    assert_eq!(store.observation_log_mut().high_watermark(), 0);
    std::fs::remove_dir_all(&stores_dir).map_err(|source| MemoryError::Io {
        path: stores_dir,
        source,
    })?;
    Ok(())
}

#[test]
fn windows_backslash_and_posix_forward_slash_roots_map_to_the_same_store() -> Result<()> {
    let stores_dir = temp_dir("path-normalize");
    let backslash: RepoRoot = r"C:\Projects\path-normalize-demo"
        .parse()
        .map_err(|source| MemoryError::InvalidPath {
            path: r"C:\Projects\path-normalize-demo".to_owned(),
            source,
        })?;
    Store::init(&stores_dir, &backslash, "2026-07-04T00:00:00Z")?;

    let forward_slash: RepoRoot = "C:/Projects/path-normalize-demo"
        .parse()
        .map_err(|source| MemoryError::InvalidPath {
            path: "C:/Projects/path-normalize-demo".to_owned(),
            source,
        })?;
    // Opening with the forward-slash spelling of the SAME root must
    // find the store the backslash spelling created -- proving path
    // normalization, not just id equality.
    let store = Store::open(&stores_dir, &forward_slash)?;
    assert_eq!(
        store.project_id().as_str(),
        ProjectId::from_repo_root(&backslash).as_str()
    );
    std::fs::remove_dir_all(&stores_dir).map_err(|source| MemoryError::Io {
        path: stores_dir,
        source,
    })?;
    Ok(())
}
