use crate::EventingError;

pub(super) enum AttemptOutcome {
    Handled,
    Failed(EventingError),
    TimedOut,
    Panicked,
}
