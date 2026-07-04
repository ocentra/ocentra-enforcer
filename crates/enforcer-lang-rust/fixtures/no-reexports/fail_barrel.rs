// FAIL fixture for T1-NOREEXPORT.1: a barrel module re-exporting through
// `pub use` — banned by the no-reexports discipline.
mod inner {
    pub struct Thing;
}

pub use inner::Thing;
