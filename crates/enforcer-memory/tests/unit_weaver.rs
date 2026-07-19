use enforcer_domain::memory_types::{
    EmbeddingGeneration, EmbeddingGenerationId, TaskOutcome, WeaverNodeId,
};
use enforcer_memory::code_graph::IndexReport;
use enforcer_memory::enrichment::{Embedder, NullEmbedder};
use enforcer_memory::queue::WeaverEvent;
use enforcer_memory::weaver::{MigrationError, Weaver};
use std::error::Error;
use std::sync::Arc;

type TestResult = Result<(), Box<dyn Error>>;

#[tokio::test]
async fn node_created_event_triggers_embedding_task() -> TestResult {
    let embedder = Arc::new(NullEmbedder::new());
    let channel = Weaver::builder()
        .with_embedder(Arc::clone(&embedder) as Arc<dyn Embedder>)
        .with_outcome_channel();
    let weaver = channel.builder.build();
    let mut outcomes = channel.outcomes;

    weaver.enqueue(WeaverEvent::NodeChanged {
        node_id: "sym:src/lib.rs:1:foo".to_owned().into(),
        rel_path: "src/lib.rs".to_owned().into(),
        content_hash: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
            .to_owned()
            .into(),
    })?;

    // Deterministic synchronization: wait for the task's own
    // `Succeeded` outcome instead of polling `embedder.calls()` on
    // a sleep.
    let outcome = outcomes.recv().await;
    assert_eq!(
        outcome,
        Some(TaskOutcome::Succeeded {
            task_key: "node-changed:sym:src/lib.rs:1:foo".into()
        })
    );

    let calls = embedder.calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].node_id.as_str(), "sym:src/lib.rs:1:foo");

    weaver.shutdown().await;
    Ok(())
}

#[tokio::test]
async fn file_changed_event_invalidates_cached_summary() -> TestResult {
    let channel = Weaver::builder().with_outcome_channel();
    let weaver = channel.builder.build();
    let mut outcomes = channel.outcomes;
    weaver.with_summaries(|store| store.set_summary("src/lib.rs", "an old summary"));

    weaver.enqueue(WeaverEvent::FileChanged {
        rel_path: "src/lib.rs".to_owned().into(),
        content_hash: "hash-b".to_owned().into(),
    })?;

    // Deterministic synchronization: wait for this task's
    // `Succeeded` outcome instead of polling `is_stale` on a sleep.
    let outcome = outcomes.recv().await;
    assert_eq!(
        outcome,
        Some(TaskOutcome::Succeeded {
            task_key: "file-changed:src/lib.rs".into()
        })
    );

    assert!(weaver.with_summaries(|store| store.is_stale("src/lib.rs").is_stale()));
    weaver.shutdown().await;
    Ok(())
}

#[tokio::test]
async fn index_report_translates_into_file_and_node_events() -> TestResult {
    let embedder = Arc::new(NullEmbedder::new());
    let channel = Weaver::builder()
        .with_embedder(Arc::clone(&embedder) as Arc<dyn Embedder>)
        .with_outcome_channel();
    let weaver = channel.builder.build();
    let mut outcomes = channel.outcomes;

    let report = IndexReport {
        unchanged: vec!["src/untouched.rs".to_owned()],
        changed: vec!["src/lib.rs".to_owned()],
        added: vec!["src/new.rs".to_owned()],
        deleted: vec!["src/old.rs".to_owned()],
    };

    weaver.enqueue_index_report(
        &report,
        |_rel_path| {
            "7777777777777777777777777777777777777777777777777777777777777777"
                .to_owned()
                .into()
        },
        |rel_path| vec![WeaverNodeId::from(format!("sym:{rel_path}:1:main"))].into(),
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
    assert!(weaver.with_summaries(|store| store.get("src/old.rs").is_none()));

    weaver.shutdown().await;
    Ok(())
}

#[tokio::test]
async fn migration_cutover_never_mixes_generations() -> TestResult {
    let weaver = Weaver::builder()
        .with_embedding_version(EmbeddingGenerationId::INITIAL)
        .build();
    assert_eq!(
        weaver.embedding_generation(),
        EmbeddingGeneration::Stable {
            active: EmbeddingGenerationId::INITIAL
        }
    );

    weaver.begin_embedding_migration(EmbeddingGenerationId::from_nonzero(
        std::num::NonZeroU32::new(2).ok_or("generation must be non-zero")?,
    ))?;
    let generation = weaver.embedding_generation();
    assert!(generation.is_migrating());
    assert_eq!(
        u32::from(generation.active()),
        1,
        "active generation stays 1 during migration"
    );
    assert_eq!(generation.next().map(u32::from), Some(2));

    // Cannot start a second migration while one is in flight.
    assert!(weaver
        .begin_embedding_migration(EmbeddingGenerationId::from_nonzero(
            std::num::NonZeroU32::new(3).ok_or("generation must be non-zero")?,
        ))
        .is_err());

    let cutover = weaver.complete_embedding_migration()?;
    assert_eq!(u32::from(cutover), 2);
    assert_eq!(
        weaver.embedding_generation(),
        EmbeddingGeneration::Stable { active: cutover }
    );

    // Cannot complete a migration that is not in flight.
    assert!(matches!(
        weaver.complete_embedding_migration(),
        Err(MigrationError::NotMigrating)
    ));

    weaver.shutdown().await;
    Ok(())
}
