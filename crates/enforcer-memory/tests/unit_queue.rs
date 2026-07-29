use enforcer_domain::memory_types::{MemoryPriority, RetryAttemptCount};
use enforcer_memory::queue::{DeadLetterQueue, FailedTask, RetryPolicy, WeaverEvent, WeaverQueue};
use std::error::Error;
use std::future::Future;
use std::task::{Context, Poll, Waker};

type TestResult = Result<(), Box<dyn Error>>;

#[test]
fn retry_policy_backs_off_and_caps() {
    let policy = RetryPolicy::bounded_default();
    let d0 = policy.delay_for(RetryAttemptCount::ZERO);
    let d1 = policy.delay_for(RetryAttemptCount::ZERO.next());
    let d2 = policy.delay_for(RetryAttemptCount::ZERO.next().next());
    assert!(d1 > d0, "delay must grow with attempt number");
    assert!(d2 > d1, "delay must keep growing");
    assert!(
        policy.delay_for(RetryAttemptCount::BACKOFF_SATURATION_PROBE) <= policy.max_delay,
        "delay must respect the max cap even for large attempt numbers"
    );
}

#[test]
fn retry_policy_exhaustion_is_attempt_count_based() {
    let policy = RetryPolicy::bounded_default();
    assert!(!policy.is_exhausted(RetryAttemptCount::ZERO).is_exhausted());
    assert!(!policy
        .is_exhausted(
            policy
                .max_attempts
                .previous()
                .unwrap_or(RetryAttemptCount::ZERO)
        )
        .is_exhausted());
    assert!(policy.is_exhausted(policy.max_attempts).is_exhausted());
}

#[tokio::test]
async fn hot_task_is_received_before_previously_queued_warm_and_cold() -> TestResult {
    let mut queue = WeaverQueue::new();
    let handle = queue.handle();

    handle.send(
        WeaverEvent::RelinkRequested {
            node_id: "cold-1".to_owned().into(),
        },
        MemoryPriority::Cold,
    )?;
    handle.send(
        WeaverEvent::RelinkRequested {
            node_id: "warm-1".to_owned().into(),
        },
        MemoryPriority::Warm,
    )?;
    handle.send(
        WeaverEvent::RelinkRequested {
            node_id: "hot-1".to_owned().into(),
        },
        MemoryPriority::Hot,
    )?;

    let first = queue.recv_next().await.map(|t| t.priority);
    assert_eq!(first, Some(MemoryPriority::Hot));
    let second = queue.recv_next().await.map(|t| t.priority);
    assert_eq!(second, Some(MemoryPriority::Warm));
    let third = queue.recv_next().await.map(|t| t.priority);
    assert_eq!(third, Some(MemoryPriority::Cold));
    Ok(())
}

#[tokio::test]
async fn recv_next_cancellation_preserves_queued_work() -> TestResult {
    let mut queue = WeaverQueue::new();
    let handle = queue.handle();

    let mut pending_receive = Box::pin(queue.recv_next());
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);
    assert!(matches!(
        pending_receive.as_mut().poll(&mut context),
        Poll::Pending
    ));
    drop(pending_receive);

    handle.send(
        WeaverEvent::RelinkRequested {
            node_id: "after-cancel".to_owned().into(),
        },
        MemoryPriority::Hot,
    )?;
    let received = queue
        .recv_next()
        .await
        .ok_or("queue closed after cancelling a pending receive")?;
    assert_eq!(received.event.task_key(), "relink:after-cancel");
    Ok(())
}

#[test]
fn dead_letter_queue_finds_entries_by_task_key() {
    let mut dlq = DeadLetterQueue::new();
    assert!(bool::from(dlq.is_empty()));
    let event = WeaverEvent::FileChanged {
        rel_path: "src/lib.rs".to_owned().into(),
        content_hash: "abc".to_owned().into(),
    };
    dlq.push(FailedTask {
        event: event.clone(),
        attempts: RetryAttemptCount::DEFAULT_LIMIT,
        last_error: "boom".to_owned().into(),
    });
    assert_eq!(dlq.len(), 1);
    let found = dlq.find(&event.task_key());
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].last_error, "boom");
    assert!(dlq.find(&"no-such-key".into()).is_empty());
}
