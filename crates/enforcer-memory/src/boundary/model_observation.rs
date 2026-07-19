//! Serde adapters for model-observation domain variants.
//!
//! BOUNDARY-INVARIANT: raw serialized recurrence values are converted directly
//! into the closed domain enum; unknown variant tags are rejected by serde.

pub(crate) mod recurrence_kind_wire {
    use enforcer_domain::memory_types::RecurrenceNegativeKind;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    #[derive(Serialize)]
    #[serde(rename_all = "kebab-case")]
    enum RecurrenceKindRef<'a> {
        RecurrenceCount {
            recurrence_count: usize,
            previous_count: Option<usize>,
        },
        NegativeEvidence {
            reason: &'a str,
        },
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "kebab-case")]
    enum RecurrenceKindOwned {
        RecurrenceCount {
            recurrence_count: usize,
            previous_count: Option<usize>,
        },
        NegativeEvidence {
            reason: String,
        },
    }

    pub(crate) fn serialize<S>(
        evidence: &RecurrenceNegativeKind,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match evidence {
            RecurrenceNegativeKind::RecurrenceCount {
                recurrence_count,
                previous_count,
            } => RecurrenceKindRef::RecurrenceCount {
                recurrence_count: recurrence_count.get(),
                previous_count: previous_count
                    .map(enforcer_domain::memory_types::MemoryEvidenceRecurrenceCount::get),
            }
            .serialize(serializer),
            RecurrenceNegativeKind::NegativeEvidence { reason } => {
                RecurrenceKindRef::NegativeEvidence {
                    reason: reason.as_str(),
                }
                .serialize(serializer)
            }
        }
    }

    pub(crate) fn deserialize<'de, D>(deserializer: D) -> Result<RecurrenceNegativeKind, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(match RecurrenceKindOwned::deserialize(deserializer)? {
            RecurrenceKindOwned::RecurrenceCount {
                recurrence_count,
                previous_count,
            } => RecurrenceNegativeKind::RecurrenceCount {
                recurrence_count: recurrence_count.into(),
                previous_count: previous_count.map(Into::into),
            },
            RecurrenceKindOwned::NegativeEvidence { reason } => {
                RecurrenceNegativeKind::NegativeEvidence {
                    reason: reason.into(),
                }
            }
        })
    }
}
