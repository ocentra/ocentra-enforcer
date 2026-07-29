//! Workflow declarations converted into canonical GitHub check contexts.

//! BOUNDARY-INVARIANT: workflow text converts to validated check contexts.
//! Negative invalid inputs are rejected while constructing check contexts.
//!
use std::collections::BTreeSet;

use enforcer_domain::{boundary::decode_error::DecodeError, ids::GitHubCheckContext};

/// Raw workflow declaration used only to derive GitHub check contexts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowJobDeclaration {
    /// Workflow `name:` value.
    pub workflow_name: String,
    /// Workflow job identifier.
    pub job_id: String,
    /// Matrix values in GitHub's rendered order.
    pub matrix: Vec<String>,
}

impl TryFrom<WorkflowJobDeclaration> for BTreeSet<GitHubCheckContext> {
    type Error = DecodeError;

    fn try_from(dto: WorkflowJobDeclaration) -> Result<Self, Self::Error> {
        let values = if dto.matrix.is_empty() {
            vec![format!("{} / {}", dto.workflow_name, dto.job_id)]
        } else {
            dto.matrix
                .into_iter()
                .map(|value| format!("{} / {} ({value})", dto.workflow_name, dto.job_id))
                .collect()
        };
        values
            .into_iter()
            .map(GitHubCheckContext::try_from)
            .collect()
    }
}
