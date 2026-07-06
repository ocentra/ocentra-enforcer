use enforcer_memory::code_graph::IndexReport;
use enforcer_memory::enrichment::{Embedder, NullEmbedder, TaskOutcome};
use enforcer_memory::queue::WeaverEvent;
use enforcer_memory::weaver::{EmbeddingGeneration, Weaver};
use std::error::Error;
use std::sync::{Arc, Mutex};

type TestResult = Result<(), Box<dyn Error>>;

/// See `enrichment::tests::lock_or_msg` -- `PoisonError`'s embedded
/// guard is not `'static`, so it cannot convert into `Box<dyn
/// Error>` via a bare `?`.
fn lock_or_msg<T>(mutex: &Mutex<T>) -> Result<std::sync::MutexGuard<'_, T>, Box<dyn Error>> {
    mutex
        .lock()
        .map_err(|_poison_error| "mutex poisoned".into())
}

#[tokio::test]
async fn node_created_event_triggers_embedding_task() -> TestResult {
    let embedder = Arc::new(NullEmbedder::new());
    let (builder, mut outcomes) = Weaver::builder()
        .with_embedder(Arc::clone(&embedder) as Arc<dyn Embedder>)
        .with_outcome_channel();
    let weaver = builder.build();

    weaver.enqueue(WeaverEvent::NodeChanged {
        node_id: "sym:src/lib.rs:1:foo".to_owned(),
        rel_path: "src/lib.rs".to_owned(),
        content_hash: "hash-a".to_owned(),
    })?;

    // Deterministic synchronization: wait for the task's own
    // `Succeeded` outcome instead of polling `embedder.calls()` on
    // a sleep.
    let outcome = outcomes.recv().await;
    assert_eq!(
        outcome,
        Some(TaskOutcome::Succeeded {
            task_key: "node-changed:sym:src/lib.rs:1:foo".to_owned()
        })
    );

    let calls = embedder.calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].node_id, "sym:src/lib.rs:1:foo");

    weaver.shutdown().await;
    Ok(())
}

#[tokio::test]
async fn file_changed_event_invalidates_cached_summary() -> TestResult {
    let (builder, mut outcomes) = Weaver::builder().with_outcome_channel();
    let weaver = builder.build();
    {
        let summaries = weaver.summaries();
        let mut store = lock_or_msg(&summaries)?;
        store.set_summary("src/lib.rs", "an old summary");
    }

    weaver.enqueue(WeaverEvent::FileChanged {
        rel_path: "src/lib.rs".to_owned(),
        content_hash: "hash-b".to_owned(),
    })?;

    // Deterministic synchronization: wait for this task's
    // `Succeeded` outcome instead of polling `is_stale` on a sleep.
    let outcome = outcomes.recv().await;
    assert_eq!(
        outcome,
        Some(TaskOutcome::Succeeded {
            task_key: "file-changed:src/lib.rs".to_owned()
        })
    );

    let summaries = weaver.summaries();
    assert!(lock_or_msg(&summaries)?.is_stale("src/lib.rs"));
    weaver.shutdown().await;
    Ok(())
}

#[tokio::test]
async fn index_report_translates_into_file_and_node_events() -> TestResult {
    let embedder = Arc::new(NullEmbedder::new());
    let (builder, mut outcomes) = Weaver::builder()
        .with_embedder(Arc::clone(&embedder) as Arc<dyn Embedder>)
        .with_outcome_channel();
    let weaver = builder.build();

    let report = IndexReport {
        unchanged: vec!["src/untouched.rs".to_owned()],
        changed: vec!["src/lib.rs".to_owned()],
        added: vec!["src/new.rs".to_owned()],
        deleted: vec!["src/old.rs".to_owned()],
    };

    weaver.enqueue_index_report(
        &report,
        |rel_path| format!("hash-of-{rel_path}"),
        |rel_path| vec![format!("sym:{rel_path}:1:main")],
    )?;

    // The report yields 5 tasks: 2 `FileChanged` (changed + added)
    // + 2 `NodeChanged` (one per changed+added file) + 1
    // `FileDeleted`. Deterministic synchronization: wait for all 5
    // `Succeeded` outcomes instead of polling on a sleep.
    let mut succeeded = 0;
    let mut unexpected = None;
    while succeeded < 5 {
        match outcomes.recv().await {
            Some(TaskOutcome::Succeeded { .. }) => succeeded += 1,
            other => {
                unexpected = Some(other);
                break;
            }
        }
    }
    assert_eq!(unexpected, None, "expected only Succeeded outcomes");

    let calls = embedder.calls();
    assert_eq!(calls.len(), 2, "one NodeChanged per changed+added file");
    let summaries = weaver.summaries();
    assert!(lock_or_msg(&summaries)?.get("src/old.rs").is_none());

    weaver.shutdown().await;
    Ok(())
}

#[tokio::test]
async fn migration_cutover_never_mixes_generations() -> TestResult {
    let weaver = Weaver::builder().with_embedding_version(1).build();
    assert_eq!(
        weaver.embedding_generation(),
        EmbeddingGeneration::Stable { active: 1 }
    );

    weaver.begin_embedding_migration(2)?;
    let generation = weaver.embedding_generation();
    assert!(generation.is_migrating());
    assert_eq!(
        generation.active(),
        1,
        "active generation stays 1 during migration"
    );
    assert_eq!(generation.next(), Some(2));

    // Cannot start a second migration while one is in flight.
    assert!(weaver.begin_embedding_migration(3).is_err());

    let cutover = weaver.complete_embedding_migration()?;
    assert_eq!(cutover, 2);
    assert_eq!(
        weaver.embedding_generation(),
        EmbeddingGeneration::Stable { active: 2 }
    );

    // Cannot complete a migration that is not in flight.
    assert!(weaver.complete_embedding_migration().is_err());

    weaver.shutdown().await;
    Ok(())
}
