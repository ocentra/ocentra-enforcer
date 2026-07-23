//! BOUNDARY-INVARIANT: this boundary module validates raw wire values and converts only through typed domain contracts.
//! Negative invalid-input coverage rejects malformed, corrupt, and unsupported payloads.
//! Explicit ownership and numeric-conversion boundaries shared by Memory subsystems.
//!
//! These helpers name the places where Memory intentionally materializes owned
//! state or projects a value into another numeric representation.

use std::fmt::Display;

pub(crate) trait Retained: ToOwned {
    fn retained(&self) -> Self::Owned;
}

impl<T> Retained for T
where
    T: ToOwned + ?Sized,
{
    fn retained(&self) -> Self::Owned {
        // ALLOC-JUSTIFICATION: this is the single explicit Memory boundary
        // for values retained after the caller's borrow ends.
        self.to_owned()
    }
}

pub(crate) trait RetainedDisplay: Display {
    fn retained_display(&self) -> String;
}

impl<T> RetainedDisplay for T
where
    T: Display + ?Sized,
{
    fn retained_display(&self) -> String {
        // ALLOC-JUSTIFICATION: durable diagnostics must own their rendered
        // source after the borrowed error or scalar expires.
        format!("{self}")
    }
}

pub(crate) fn usize_to_f64(value: usize) -> f64 {
    // CAST-JUSTIFICATION: scoring and telemetry ratios deliberately project a
    // bounded in-memory count into floating-point space.
    value as f64
}

pub(crate) fn u128_to_f64(value: u128) -> f64 {
    // CAST-JUSTIFICATION: elapsed milliseconds feed approximate throughput
    // telemetry, so projecting the bounded duration into f64 is intentional.
    value as f64
}

#[cfg(feature = "ort-models")]
pub(crate) fn usize_to_f32(value: usize) -> f32 {
    // CAST-JUSTIFICATION: tensor normalization projects a bounded sequence
    // length into the model's f32 arithmetic domain.
    value as f32
}

pub(crate) fn u64_to_usize(value: u64) -> usize {
    usize::try_from(value).unwrap_or(usize::MAX)
}

pub(crate) fn usize_to_u64(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}
