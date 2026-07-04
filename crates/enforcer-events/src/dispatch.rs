//! Panic-isolated event dispatch: `Sequential` and `Concurrent` strategies
//! that run a list of fallible handlers over one event WITHOUT letting one
//! handler's panic or error stop the others from running.
//!
//! SYNC-first (locked decision): no `tokio` here. `Concurrent` dispatch uses
//! plain OS threads (`std::thread::scope`) rather than a work-stealing pool,
//! since the handler counts this crate expects (scan/coordination/proof
//! subscribers) are small and bounded — pulling in `rayon` here would be
//! over-engineering for the enforcer's actual fan-out.
//!
//! See the `lib.rs` module doc for the vendoring-attribution note.

use std::panic::{catch_unwind, AssertUnwindSafe};

/// Outcome of dispatching one event to one handler.
#[derive(Debug)]
pub enum HandlerOutcome {
    /// The handler ran to completion without error.
    Ok,
    /// The handler returned an error (did not panic).
    Err(String),
    /// The handler panicked; the panic was caught and isolated.
    Panicked(String),
}

impl HandlerOutcome {
    /// True if the handler completed without error or panic.
    pub fn is_ok(&self) -> bool {
        matches!(self, HandlerOutcome::Ok)
    }
}

fn run_one<F>(handler: F) -> HandlerOutcome
where
    F: FnOnce() -> Result<(), String>,
{
    match catch_unwind(AssertUnwindSafe(handler)) {
        Ok(Ok(())) => HandlerOutcome::Ok,
        Ok(Err(reason)) => HandlerOutcome::Err(reason),
        Err(panic_payload) => {
            let message = panic_payload
                .downcast_ref::<&str>()
                .map(|s| (*s).to_owned())
                .or_else(|| panic_payload.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "handler panicked with a non-string payload".to_owned());
            HandlerOutcome::Panicked(message)
        }
    }
}

/// Run handlers one after another. Every handler runs regardless of whether
/// an earlier one panicked or errored; outcomes are returned in order.
pub fn dispatch_sequential<F>(handlers: Vec<F>) -> Vec<HandlerOutcome>
where
    F: FnOnce() -> Result<(), String>,
{
    handlers.into_iter().map(run_one).collect()
}

/// Run handlers concurrently on OS threads (bounded, SYNC-first — no async
/// runtime). Every handler runs regardless of whether another panicked or
/// errored; outcomes are returned in the SAME order the handlers were
/// supplied, not completion order.
pub fn dispatch_concurrent<F>(handlers: Vec<F>) -> Vec<HandlerOutcome>
where
    F: FnOnce() -> Result<(), String> + Send,
{
    std::thread::scope(|scope| {
        let joins: Vec<_> = handlers
            .into_iter()
            .map(|handler| scope.spawn(move || run_one(handler)))
            .collect();
        joins
            .into_iter()
            .map(|handle| {
                handle.join().unwrap_or_else(|_| {
                    HandlerOutcome::Panicked(
                        "handler thread panicked outside the isolation boundary".to_owned(),
                    )
                })
            })
            .collect()
    })
}

#[cfg(test)]
mod tests {
    use super::{dispatch_concurrent, dispatch_sequential, HandlerOutcome};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[test]
    // Deliberately triggers a real panic to prove `dispatch_sequential`
    // isolates it; the workspace-wide `clippy::panic` deny targets
    // production code, not this narrowly-scoped isolation proof.
    #[allow(clippy::panic)]
    fn sequential_runs_every_handler_despite_a_panic() {
        let ran = Arc::new(AtomicUsize::new(0));
        let ran_a = Arc::clone(&ran);
        let ran_c = Arc::clone(&ran);
        let handlers: Vec<Box<dyn FnOnce() -> Result<(), String>>> = vec![
            Box::new(move || {
                ran_a.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }),
            Box::new(|| panic!("boom")),
            Box::new(move || {
                ran_c.fetch_add(1, Ordering::SeqCst);
                Err("deliberate failure".to_owned())
            }),
        ];
        let outcomes = dispatch_sequential(
            handlers
                .into_iter()
                .map(|h| move || h())
                .collect::<Vec<_>>(),
        );
        assert_eq!(outcomes.len(), 3);
        assert!(outcomes[0].is_ok());
        assert!(matches!(outcomes[1], HandlerOutcome::Panicked(_)));
        assert!(matches!(outcomes[2], HandlerOutcome::Err(_)));
        // Both non-panicking handlers ran despite handler[1]'s panic.
        assert_eq!(ran.load(Ordering::SeqCst), 2);
    }

    #[test]
    // See the allow-rationale on `sequential_runs_every_handler_despite_a_panic`.
    #[allow(clippy::panic)]
    fn concurrent_runs_every_handler_despite_a_panic_and_preserves_order() {
        let handlers: Vec<Box<dyn FnOnce() -> Result<(), String> + Send>> = vec![
            Box::new(|| Ok(())),
            Box::new(|| panic!("boom")),
            Box::new(|| Err("deliberate failure".to_owned())),
            Box::new(|| Ok(())),
        ];
        let outcomes = dispatch_concurrent(
            handlers
                .into_iter()
                .map(|h| move || h())
                .collect::<Vec<_>>(),
        );
        assert_eq!(outcomes.len(), 4);
        assert!(outcomes[0].is_ok());
        assert!(matches!(outcomes[1], HandlerOutcome::Panicked(_)));
        assert!(matches!(outcomes[2], HandlerOutcome::Err(_)));
        assert!(outcomes[3].is_ok());
    }
}
