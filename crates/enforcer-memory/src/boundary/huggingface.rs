//! BOUNDARY-INVARIANT: this boundary module validates raw wire values and converts only through typed domain contracts.
//! Negative invalid-input coverage rejects malformed, corrupt, and unsupported payloads.
//! Hugging Face HTTP metadata DTOs.
//!
//! These shapes model the provider response only. `hf_cache` validates their
//! raw fields into canonical memory-domain values before they reach the cache
//! or runtime APIs.

use serde::{Deserialize, Serialize};

#[cfg(feature = "real-models")]
use crate::hf_cache::{ChatModelCandidate, ChatModelSelection, HfFileSpec, HfModelSpec};
#[cfg(feature = "real-models")]
use enforcer_domain::memory_types::ChatModelArchitecture;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HfRepoMetadataDto {
    #[serde(rename = "modelId")]
    pub model_id: Option<String>,
    // DEFAULT-JUSTIFICATION: Hugging Face may omit `siblings` for an empty
    // repository response; the provider contract defines that as no files.
    #[serde(default)]
    pub siblings: Vec<HfRepoFileDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HfRepoFileDto {
    #[serde(rename = "rfilename")]
    pub path: String,
    pub size: Option<u64>,
}

#[cfg(feature = "real-models")]
pub(crate) fn model_spec_value(spec: &HfModelSpec) -> serde_json::Value {
    serde_json::json!({
        "repoId": spec.repo_id.as_str(),
        "revision": spec.revision.as_str(),
        "backend": spec.backend,
        "task": spec.task,
        "modelId": spec.model_id.as_str(),
        "acceleration": spec.acceleration,
        "files": spec.files.iter().map(file_spec_value).collect::<Vec<_>>(),
    })
}

#[cfg(feature = "real-models")]
pub(crate) fn chat_selection_value(selection: &ChatModelSelection) -> serde_json::Value {
    serde_json::json!({
        "selected": model_spec_value(&selection.selected),
        "selectedQuantization": selection.selected_quantization.as_str(),
        "detectedFreeVramMib": selection.detected_free_vram_mib,
        "reason": selection.reason.as_str(),
        "candidates": selection.candidates.iter().map(chat_candidate_value).collect::<Vec<_>>(),
    })
}

#[cfg(feature = "real-models")]
fn file_spec_value(file: &HfFileSpec) -> serde_json::Value {
    serde_json::json!({
        "path": file.path.as_str(),
        "kind": file.kind,
    })
}

#[cfg(feature = "real-models")]
fn chat_candidate_value(candidate: &ChatModelCandidate) -> serde_json::Value {
    let architecture = match candidate.architecture {
        ChatModelArchitecture::Dense => "dense",
        ChatModelArchitecture::Moe => "moe",
    };
    serde_json::json!({
        "spec": model_spec_value(&candidate.spec),
        "architecture": architecture,
        "quantization": candidate.quantization.as_str(),
        "totalParameterCountMillions": candidate.total_parameter_count_millions,
        "activeParameterCountMillions": candidate.active_parameter_count_millions,
        "estimatedSizeBytes": candidate.estimated_size_bytes,
        "requiredFreeVramMib": candidate.required_free_vram_mib,
        "preferenceRank": candidate.preference_rank,
    })
}
