//! Ownership helpers needed by the transferred syntax extractors.
//!
//! BOUNDARY-INVARIANT: these helpers only retain owned text or render a
//! display value. They do not parse, persist, classify, or silently recover
//! syntax input.
//!
//! INVALID-INPUT COVERAGE: this boundary has no decoder or parser input. The
//! transferred parser entrypoints own invalid-input behavior and their
//! negative binary/control-input fixtures remain in the memory test suite.

use std::fmt::Display;

pub(crate) trait Retained: ToOwned {
    fn retained(&self) -> Self::Owned {
        self.to_owned()
    }
}

impl<T> Retained for T where T: ToOwned + ?Sized {}

pub(crate) trait RetainedDisplay: Display {
    fn retained_display(&self) -> String {
        self.to_string()
    }
}

impl<T> RetainedDisplay for T where T: Display + ?Sized {}
