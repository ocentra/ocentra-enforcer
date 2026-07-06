use enforcer_memory::queue::{
    DeadLetterQueue, FailedTask, Priority, RetryPolicy, WeaverEvent, WeaverQueue,
};
use std::error::Error;

type TestResult = Result<(), Box<dyn Error>>;

#[test]
fn retry_policy_backs_off_and_caps() {
    let policy = RetryPolicy::bounded_default();
    let d0 = policy.delay_for(0);
    let d1 = policy.delay_for(1);
    let d2 = policy.delay_for(2);
    assert!(d1 > d0, "delay must grow with attempt number");
    assert!(d2 > d1, "delay must keep growing");
    assert!(
        policy.delay_for(30) <= policy.max_delay,
        "delay must respect the max cap even for large attempt numbers"
    );
}

#[test]
fn retry_policy_exhaustion_is_attempt_count_based() {
    let policy = RetryPolicy::bounded_default();
    assert!(!policy.is_exhausted(0));
    assert!(!policy.is_exhausted(policy.max_attempts - 1));
    assert!(policy.is_exhausted(policy.max_attempts));
}

#[tokio::test]
async fn hot_task_is_received_before_previously_queued_warm_and_cold() -> TestResult {
    let mut queue = WeaverQueue::new();
    let handle = queue.handle();

    handle.send(
        WeaverEvent::RelinkRequested {
            node_id: "cold-1".to_owned(),
        },
        Priority::Cold,
    )?;
    handle.send(
        WeaverEvent::RelinkRequested {
            node_id: "warm-1".to_owned(),
        },
        Priority::Warm,
    )?;
    handle.send(
        WeaverEvent::RelinkRequested {
            node_id: "hot-1".to_owned(),
        },
        Priority::Hot,
    )?;

    let first = queue.recv_next().await.map(|t| t.priority);
    assert_eq!(first, Some(Priority::Hot));
    let second = queue.recv_next().await.map(|t| t.priority);
    assert_eq!(second, Some(Priority::Warm));
    let third = queue.recv_next().await.map(|t| t.priority);
    assert_eq!(third, Some(Priority::Cold));
    Ok(())
}

#[test]
fn dead_letter_queue_finds_entries_by_task_key() {
    let mut dlq = DeadLetterQueue::new();
    let event = WeaverEvent::FileChanged {
        rel_path: "src/lib.rs".to_owned(),
        content_hash: "abc".to_owned(),
    };
    dlq.push(FailedTask {
        event: event.clone(),
        attempts: 3,
        last_error: "boom".to_owned(),
    });
    assert_eq!(dlq.len(), 1);
    let found = dlq.find(&event.task_key());
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].last_error, "boom");
    assert!(dlq.find("no-such-key").is_empty());
}
