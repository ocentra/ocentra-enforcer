//! Domain values for records accepted into the local memory graph.

/// A record accepted into the local memory domain.
///
/// The JSON shape belongs to [`crate::boundary::record::MemoryRecordDto`]. Keeping that payload behind
/// this domain value prevents graph, learning, recall and redaction code from
/// accidentally treating an externally supplied DTO as already-trusted domain
/// state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryRecord {
    pub(crate) dto: crate::boundary::record::MemoryRecordDto,
}
