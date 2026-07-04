// PASS fixture for T1-NOREEXPORT.1: callers import the concrete module
// path directly; only a private (non-reexporting) `use` appears here.
mod inner {
    pub struct Thing;
}

use inner::Thing;

pub fn make() -> Thing {
    Thing
}
