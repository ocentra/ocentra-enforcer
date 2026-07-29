//! Explicit Hugging Face download/cache support for real model proof.
//!
//! This module is intentionally feature-gated by `model-downloads`.
//! Default Enforcer builds never call the network and never read model
//! provider secrets.

use std::path::{Path, PathBuf};

use enforcer_domain::memory_types::{
    ChatModelArchitecture, HfFilePath, HfModelId, HfRepositoryId, HfRevision,
};
use enforcer_domain::memory_types::{StreamingArtifactKey, StreamingRelativePath};

use crate::boundary::model_cache::{ModelCacheArtifactEntryDto, ModelCacheManifestDto};
use crate::error::{MemoryError, Result};
use crate::model_cache::{load_model_cache_manifest, MODEL_CACHE_SCHEMA_VERSION};
use crate::model_runtime::{
    sha256_file, ModelSpecDto, DEFAULT_EMBEDDING_GGUF_FILE, DEFAULT_EMBEDDING_GGUF_REPO,
    DEFAULT_EMBEDDING_ONNX_FILE, DEFAULT_EMBEDDING_ONNX_REPO, DEFAULT_MODEL_REVISION,
    DEFAULT_ORNITH_GGUF_FILE, DEFAULT_ORNITH_GGUF_REPO, DEFAULT_RERANKER_ONNX_FILE,
    DEFAULT_RERANKER_ONNX_REPO,
};
use crate::streaming_cache::{should_chunk_file, stream_file_into_chunks};
use enforcer_domain::memory_types::ModelTask;
use enforcer_domain::memory_types::{
    LocalRuntimeAcceleration, LocalRuntimeArtifactKind, LocalRuntimeKind,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HfFileSpec {
    pub path: HfFilePath,
    pub kind: LocalRuntimeArtifactKind,
}

impl HfFileSpec {
    pub fn from_path(path: HfFilePath, kind: LocalRuntimeArtifactKind) -> Self {
        Self { path, kind }
    }

    fn from_boundary(path: impl Into<String>, kind: LocalRuntimeArtifactKind) -> Result<Self> {
        Ok(Self::from_path(hf_file_path(path.into())?, kind))
    }
}

use crate::boundary::huggingface::{HfRepoFileDto, HfRepoMetadataDto};
use crate::owned_boundary::{Retained, RetainedDisplay};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HfModelSpec {
    pub repo_id: HfRepositoryId,
    pub revision: HfRevision,
    pub backend: LocalRuntimeKind,
    pub task: ModelTask,
    pub model_id: HfModelId,
    pub acceleration: LocalRuntimeAcceleration,
    pub files: Vec<HfFileSpec>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HfSingleFileSpecInput {
    pub repo_id: HfRepositoryId,
    pub revision: HfRevision,
    pub backend: LocalRuntimeKind,
    pub task: ModelTask,
    pub model_id: HfModelId,
    pub acceleration: LocalRuntimeAcceleration,
    pub file_path: HfFilePath,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct X06ModelLineup {
    pub chat_generation: HfModelSpec,
    pub embedding_onnx: HfModelSpec,
    pub embedding_gguf: HfModelSpec,
    pub reranker_onnx: HfModelSpec,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatModelCandidate {
    pub spec: HfModelSpec,
    pub architecture: ChatModelArchitecture,
    pub quantization: String,
    pub total_parameter_count_millions: Option<u64>,
    pub active_parameter_count_millions: Option<u64>,
    pub estimated_size_bytes: u64,
    pub required_free_vram_mib: u64,
    pub preference_rank: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatModelSelection {
    pub selected: HfModelSpec,
    pub selected_quantization: String,
    pub detected_free_vram_mib: Option<u64>,
    pub reason: String,
    pub candidates: Vec<ChatModelCandidate>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HfDownloadedFile {
    pub source_path: HfFilePath,
    pub local_path: PathBuf,
    pub sha256: String,
    pub size_bytes: u64,
    pub streaming_manifest_path: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HfDownloadReport {
    pub repo_id: HfRepositoryId,
    pub revision: HfRevision,
    pub cache_dir: PathBuf,
    pub manifest_path: PathBuf,
    pub downloaded_files: Vec<HfDownloadedFile>,
}

impl HfModelSpec {
    pub fn with_single_model_file(input: HfSingleFileSpecInput) -> Result<Self> {
        Ok(Self {
            repo_id: input.repo_id,
            revision: input.revision,
            backend: input.backend,
            task: input.task,
            model_id: input.model_id,
            acceleration: input.acceleration,
            files: vec![HfFileSpec::from_path(
                input.file_path,
                LocalRuntimeArtifactKind::Model,
            )],
        })
    }

    pub fn with_onnx_model_file(
        repo_id: impl Into<String>,
        revision: impl Into<String>,
        task: ModelTask,
        model_id: impl Into<String>,
        file_path: impl Into<String>,
    ) -> Result<Self> {
        let file_path = hf_file_path(file_path.into())?;
        Ok(Self {
            repo_id: hf_repository_id(repo_id.into())?,
            revision: hf_revision(revision.into())?,
            backend: LocalRuntimeKind::OnnxOrt,
            task,
            model_id: hf_model_id(model_id.into())?,
            acceleration: LocalRuntimeAcceleration::Cpu,
            files: onnx_support_files(&file_path)?,
        })
    }

    pub fn ornith_generation_gguf() -> Result<Self> {
        Self::with_single_model_file(HfSingleFileSpecInput {
            repo_id: hf_repository_id(DEFAULT_ORNITH_GGUF_REPO.retained())?,
            revision: hf_revision(DEFAULT_MODEL_REVISION.retained())?,
            backend: LocalRuntimeKind::LlamaCpp,
            task: ModelTask::Summarization,
            model_id: hf_model_id(DEFAULT_ORNITH_GGUF_REPO.retained())?,
            acceleration: LocalRuntimeAcceleration::Auto,
            file_path: hf_file_path(DEFAULT_ORNITH_GGUF_FILE.retained())?,
        })
    }

    pub fn qwen3_30b_a3b_chat_gguf() -> Result<Self> {
        Self::with_single_model_file(HfSingleFileSpecInput {
            repo_id: hf_repository_id("Qwen/Qwen3-30B-A3B-GGUF".retained())?,
            revision: hf_revision(DEFAULT_MODEL_REVISION.retained())?,
            backend: LocalRuntimeKind::LlamaCpp,
            task: ModelTask::Summarization,
            model_id: hf_model_id("Qwen/Qwen3-30B-A3B-GGUF:Q4_K_M".retained())?,
            acceleration: LocalRuntimeAcceleration::Auto,
            file_path: hf_file_path("Qwen3-30B-A3B-Q4_K_M.gguf".retained())?,
        })
    }

    pub fn qwen3_4b_chat_gguf() -> Result<Self> {
        Self::with_single_model_file(HfSingleFileSpecInput {
            repo_id: hf_repository_id("Qwen/Qwen3-4B-GGUF".retained())?,
            revision: hf_revision(DEFAULT_MODEL_REVISION.retained())?,
            backend: LocalRuntimeKind::LlamaCpp,
            task: ModelTask::Summarization,
            model_id: hf_model_id("Qwen/Qwen3-4B-GGUF:Q4_K_M".retained())?,
            acceleration: LocalRuntimeAcceleration::Auto,
            file_path: hf_file_path("Qwen3-4B-Q4_K_M.gguf".retained())?,
        })
    }

    pub fn gemma3_4b_chat_gguf() -> Result<Self> {
        Self::with_single_model_file(HfSingleFileSpecInput {
            repo_id: hf_repository_id("bartowski/google_gemma-3-4b-it-GGUF".retained())?,
            revision: hf_revision(DEFAULT_MODEL_REVISION.retained())?,
            backend: LocalRuntimeKind::LlamaCpp,
            task: ModelTask::Summarization,
            model_id: hf_model_id("bartowski/google_gemma-3-4b-it-GGUF:Q4_K_M".retained())?,
            acceleration: LocalRuntimeAcceleration::Auto,
            file_path: hf_file_path("google_gemma-3-4b-it-Q4_K_M.gguf".retained())?,
        })
    }

    pub fn gemma4_e4b_chat_gguf() -> Result<Self> {
        Self::with_single_model_file(HfSingleFileSpecInput {
            repo_id: hf_repository_id("unsloth/gemma-4-E4B-it-GGUF".retained())?,
            revision: hf_revision(DEFAULT_MODEL_REVISION.retained())?,
            backend: LocalRuntimeKind::LlamaCpp,
            task: ModelTask::Summarization,
            model_id: hf_model_id("unsloth/gemma-4-E4B-it-GGUF:Q4_K_M".retained())?,
            acceleration: LocalRuntimeAcceleration::Auto,
            file_path: hf_file_path("gemma-4-E4B-it-Q4_K_M.gguf".retained())?,
        })
    }

    pub fn qwen3_embedding_gguf() -> Result<Self> {
        Self::with_single_model_file(HfSingleFileSpecInput {
            repo_id: hf_repository_id(DEFAULT_EMBEDDING_GGUF_REPO.retained())?,
            revision: hf_revision(DEFAULT_MODEL_REVISION.retained())?,
            backend: LocalRuntimeKind::LlamaCpp,
            task: ModelTask::Embedding,
            model_id: hf_model_id(DEFAULT_EMBEDDING_GGUF_REPO.retained())?,
            acceleration: LocalRuntimeAcceleration::Auto,
            file_path: hf_file_path(DEFAULT_EMBEDDING_GGUF_FILE.retained())?,
        })
    }

    pub fn qwen3_embedding_onnx() -> Result<Self> {
        Self::with_onnx_model_file(
            DEFAULT_EMBEDDING_ONNX_REPO,
            DEFAULT_MODEL_REVISION,
            ModelTask::Embedding,
            DEFAULT_EMBEDDING_ONNX_REPO,
            DEFAULT_EMBEDDING_ONNX_FILE,
        )
    }

    pub fn qwen3_reranker_onnx() -> Result<Self> {
        Self::with_onnx_model_file(
            DEFAULT_RERANKER_ONNX_REPO,
            DEFAULT_MODEL_REVISION,
            ModelTask::Reranking,
            DEFAULT_RERANKER_ONNX_REPO,
            DEFAULT_RERANKER_ONNX_FILE,
        )
    }

    pub fn validate(&self) -> Result<()> {
        if self.files.is_empty() {
            return Err(model_error(
                "validate-hf-model-spec",
                "model spec has no files",
            ));
        }
        Ok(())
    }
}

pub fn x06_chat_model_candidates() -> Result<Vec<ChatModelCandidate>> {
    Ok(vec![
        ChatModelCandidate {
            spec: HfModelSpec::qwen3_30b_a3b_chat_gguf()?,
            architecture: ChatModelArchitecture::Moe,
            quantization: "Q4_K_M".retained(),
            total_parameter_count_millions: Some(30_000),
            active_parameter_count_millions: Some(3_000),
            estimated_size_bytes: 19_500_000_000,
            required_free_vram_mib: 22_528,
            preference_rank: 120,
        },
        ChatModelCandidate {
            spec: HfModelSpec::ornith_generation_gguf()?,
            architecture: ChatModelArchitecture::Dense,
            quantization: "Q4_K_M".retained(),
            total_parameter_count_millions: Some(9_000),
            active_parameter_count_millions: None,
            estimated_size_bytes: 5_629_108_704,
            required_free_vram_mib: 12_288,
            preference_rank: 100,
        },
        ChatModelCandidate {
            spec: HfModelSpec::gemma4_e4b_chat_gguf()?,
            architecture: ChatModelArchitecture::Dense,
            quantization: "Q4_K_M".retained(),
            total_parameter_count_millions: Some(4_000),
            active_parameter_count_millions: None,
            estimated_size_bytes: 4_977_169_568,
            required_free_vram_mib: 8_192,
            preference_rank: 90,
        },
        ChatModelCandidate {
            spec: HfModelSpec::qwen3_4b_chat_gguf()?,
            architecture: ChatModelArchitecture::Dense,
            quantization: "Q4_K_M".retained(),
            total_parameter_count_millions: Some(4_000),
            active_parameter_count_millions: None,
            estimated_size_bytes: 2_497_280_256,
            required_free_vram_mib: 4_096,
            preference_rank: 80,
        },
        ChatModelCandidate {
            spec: HfModelSpec::gemma3_4b_chat_gguf()?,
            architecture: ChatModelArchitecture::Dense,
            quantization: "Q4_K_M".retained(),
            total_parameter_count_millions: Some(4_000),
            active_parameter_count_millions: None,
            estimated_size_bytes: 2_489_758_112,
            required_free_vram_mib: 4_096,
            preference_rank: 70,
        },
    ])
}

pub fn select_x06_chat_model_for_hardware(
    free_vram_mib: Option<u64>,
) -> Result<ChatModelSelection> {
    let candidates = x06_chat_model_candidates()?;
    let selected = free_vram_mib
        .and_then(|free| {
            candidates
                .iter()
                .filter(|candidate| candidate.required_free_vram_mib <= free)
                .max_by_key(|candidate| candidate.preference_rank)
        })
        .or_else(|| {
            candidates
                .iter()
                .filter(|candidate| candidate.quantization.starts_with("Q4"))
                .min_by_key(|candidate| candidate.estimated_size_bytes)
        })
        .cloned()
        .ok_or_else(|| model_error("select-x06-chat-model", "canonical candidate set is empty"))?;
    let reason = match free_vram_mib {
        Some(free) if selected.required_free_vram_mib <= free => format!(
            "selected {} because detected free VRAM is {free} MiB and required free VRAM is {} MiB",
            selected.spec.model_id, selected.required_free_vram_mib
        ),
        Some(free) => format!(
            "selected smallest Q4 chat fallback {} because detected free VRAM is only {free} MiB",
            selected.spec.model_id
        ),
        None => format!(
            "selected smallest Q4 chat fallback {} because no llama.cpp GPU memory report was available",
            selected.spec.model_id
        ),
    };
    Ok(ChatModelSelection {
        selected: selected.spec,
        selected_quantization: selected.quantization,
        detected_free_vram_mib: free_vram_mib,
        reason,
        candidates,
    })
}

impl X06ModelLineup {
    pub fn defaults() -> Result<Self> {
        Ok(Self {
            chat_generation: HfModelSpec::ornith_generation_gguf()?,
            embedding_onnx: HfModelSpec::qwen3_embedding_onnx()?,
            embedding_gguf: HfModelSpec::qwen3_embedding_gguf()?,
            reranker_onnx: HfModelSpec::qwen3_reranker_onnx()?,
        })
    }

    pub fn from_env() -> Result<Self> {
        let defaults = Self::defaults()?;
        let lineup = Self {
            chat_generation: override_single_file(
                &defaults.chat_generation,
                "ENFORCER_X06_CHAT_MODEL_REPO",
                "ENFORCER_X06_CHAT_MODEL_FILE",
                "ENFORCER_X06_CHAT_MODEL_ID",
                "ENFORCER_X06_CHAT_MODEL_REVISION",
            )?,
            embedding_onnx: override_onnx_file(
                &defaults.embedding_onnx,
                "ENFORCER_X06_EMBEDDING_ONNX_REPO",
                "ENFORCER_X06_EMBEDDING_ONNX_FILE",
                "ENFORCER_X06_EMBEDDING_ONNX_MODEL_ID",
                "ENFORCER_X06_EMBEDDING_ONNX_REVISION",
            )?,
            embedding_gguf: override_single_file(
                &defaults.embedding_gguf,
                "ENFORCER_X06_EMBEDDING_GGUF_REPO",
                "ENFORCER_X06_EMBEDDING_GGUF_FILE",
                "ENFORCER_X06_EMBEDDING_GGUF_MODEL_ID",
                "ENFORCER_X06_EMBEDDING_GGUF_REVISION",
            )?,
            reranker_onnx: override_onnx_file(
                &defaults.reranker_onnx,
                "ENFORCER_X06_RERANKER_ONNX_REPO",
                "ENFORCER_X06_RERANKER_ONNX_FILE",
                "ENFORCER_X06_RERANKER_ONNX_MODEL_ID",
                "ENFORCER_X06_RERANKER_ONNX_REVISION",
            )?,
        };
        lineup.validate()?;
        Ok(lineup)
    }

    pub fn validate(&self) -> Result<()> {
        self.chat_generation.validate()?;
        self.embedding_onnx.validate()?;
        self.embedding_gguf.validate()?;
        self.reranker_onnx.validate()
    }
}

fn override_single_file(
    default: &HfModelSpec,
    repo_env: &str,
    file_env: &str,
    model_id_env: &str,
    revision_env: &str,
) -> Result<HfModelSpec> {
    let repo_id = hf_repository_id(env_or(repo_env, default.repo_id.as_str()))?;
    let revision = hf_revision(env_or(revision_env, default.revision.as_str()))?;
    let model_id = hf_model_id(env_or(model_id_env, repo_id.as_str()))?;
    let default_file = default
        .files
        .first()
        .ok_or_else(|| model_error("resolve-hf-model-spec", "model spec has no files"))?;
    let file_path = hf_file_path(env_or(file_env, default_file.path.as_str()))?;
    let spec = HfModelSpec::with_single_model_file(HfSingleFileSpecInput {
        repo_id,
        revision,
        backend: default.backend,
        task: default.task,
        model_id,
        acceleration: default.acceleration,
        file_path,
    })?;
    spec.validate()?;
    Ok(spec)
}

fn override_onnx_file(
    default: &HfModelSpec,
    repo_env: &str,
    file_env: &str,
    model_id_env: &str,
    revision_env: &str,
) -> Result<HfModelSpec> {
    let repo_id = env_or(repo_env, default.repo_id.as_str());
    let revision = env_or(revision_env, default.revision.as_str());
    let model_id = env_or(model_id_env, default.model_id.as_str());
    let model_file = default
        .files
        .iter()
        .find(|file| file.path.as_str().ends_with(".onnx"))
        .map(|file| file.path.as_str())
        .unwrap_or("onnx/model_q4.onnx");
    let file_path = env_or(file_env, model_file);
    let spec =
        HfModelSpec::with_onnx_model_file(repo_id, revision, default.task, model_id, file_path)?;
    spec.validate()?;
    Ok(spec)
}

fn env_or(name: &str, fallback: &str) -> String {
    std::env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| fallback.retained())
}

fn onnx_support_files(model_path: &HfFilePath) -> Result<Vec<HfFileSpec>> {
    Ok(vec![
        HfFileSpec::from_path(model_path.retained(), LocalRuntimeArtifactKind::Model),
        HfFileSpec::from_boundary("tokenizer.json", LocalRuntimeArtifactKind::Tokenizer)?,
        HfFileSpec::from_boundary("tokenizer_config.json", LocalRuntimeArtifactKind::Tokenizer)?,
        HfFileSpec::from_boundary("config.json", LocalRuntimeArtifactKind::Config)?,
        HfFileSpec::from_boundary(
            "special_tokens_map.json",
            LocalRuntimeArtifactKind::Tokenizer,
        )?,
        HfFileSpec::from_boundary("vocab.json", LocalRuntimeArtifactKind::Tokenizer)?,
        HfFileSpec::from_boundary("merges.txt", LocalRuntimeArtifactKind::Tokenizer)?,
    ])
}

fn dedupe_files(files: Vec<HfFileSpec>) -> Vec<HfFileSpec> {
    let mut deduped = Vec::new();
    for file in files {
        if !deduped
            .iter()
            .any(|entry: &HfFileSpec| entry.path == file.path)
        {
            deduped.push(file);
        }
    }
    deduped
}

pub fn resolve_onnx_external_data_files(
    selected_onnx_path: &HfFilePath,
    repo_files: &[HfRepoFileDto],
) -> Result<Vec<HfFileSpec>> {
    let mut files = vec![HfFileSpec::from_path(
        selected_onnx_path.retained(),
        LocalRuntimeArtifactKind::Model,
    )];
    for support_file in onnx_support_files(selected_onnx_path)?
        .into_iter()
        .filter(|file| file.path != *selected_onnx_path)
    {
        if repo_files
            .iter()
            .any(|repo_file| repo_file.path == support_file.path.as_str())
        {
            files.push(support_file);
        }
    }
    let base_name = selected_onnx_path
        .as_str()
        .strip_suffix(".onnx")
        .unwrap_or(selected_onnx_path.as_str());
    let data_path_underscore = format!("{base_name}.onnx_data");
    let data_path_dot = format!("{base_name}.onnx.data");

    for repo_file in repo_files {
        if repo_file.path == data_path_underscore || repo_file.path == data_path_dot {
            files.push(HfFileSpec::from_boundary(
                repo_file.path.retained(),
                LocalRuntimeArtifactKind::ExternalData,
            )?);
        }
    }

    Ok(dedupe_files(files))
}

pub fn expand_onnx_spec_from_metadata(
    mut spec: HfModelSpec,
    metadata: &HfRepoMetadataDto,
) -> Result<HfModelSpec> {
    let selected_onnx_path = spec
        .files
        .iter()
        .find(|file| file.path.as_str().ends_with(".onnx"))
        .map(|file| file.path.retained())
        .ok_or_else(|| model_error("expand-onnx-spec", "spec has no ONNX model file"))?;

    if !metadata
        .siblings
        .iter()
        .any(|file| file.path == selected_onnx_path.as_str())
    {
        return Err(model_error(
            "expand-onnx-spec",
            format!("selected ONNX file {selected_onnx_path:?} was not found in repo metadata"),
        ));
    }

    spec.files = resolve_onnx_external_data_files(&selected_onnx_path, &metadata.siblings)?;
    spec.validate()?;
    Ok(spec)
}

pub fn model_cache_dir(
    cache_root: &Path,
    repo_id: &HfRepositoryId,
    revision: &HfRevision,
) -> PathBuf {
    cache_root
        .join("hf")
        .join(repo_id.as_str().replace('/', "--"))
        .join(revision.as_str())
}

#[cfg(feature = "model-downloads")]
pub fn fetch_hf_repo_metadata(
    repo_id: &str,
    auth_token: Option<&str>,
) -> Result<HfRepoMetadataDto> {
    validate_hf_repo_id(repo_id)?;
    let client = reqwest::blocking::Client::builder()
        .user_agent("ocentra-enforcer-memory/0.1.0")
        .build()
        .map_err(|source| {
            model_error(
                "build-hf-client",
                format!("failed to build HTTP client: {source}"),
            )
        })?;
    let auth_header = auth_token
        .filter(|token| !token.trim().is_empty())
        .map(ToOwned::to_owned)
        .or_else(env_hf_token);
    let mut request = client.get(format!("https://huggingface.co/api/models/{repo_id}"));
    if let Some(token) = auth_header.as_deref() {
        request = request.bearer_auth(token);
    }
    let response = request.send().map_err(|source| {
        model_error(
            "fetch-hf-repo-metadata",
            format!("failed to fetch metadata for {repo_id}: {source}"),
        )
    })?;
    if !response.status().is_success() {
        return Err(model_error(
            "fetch-hf-repo-metadata",
            format!(
                "HTTP {} while fetching metadata for {repo_id}",
                response.status()
            ),
        ));
    }
    response.json::<HfRepoMetadataDto>().map_err(|source| {
        model_error(
            "fetch-hf-repo-metadata",
            format!("failed to parse metadata for {repo_id}: {source}"),
        )
    })
}

#[cfg(not(feature = "model-downloads"))]
pub fn fetch_hf_repo_metadata(
    _repo_id: &str,
    _auth_token: Option<&str>,
) -> Result<HfRepoMetadataDto> {
    Err(model_error(
        "fetch-hf-repo-metadata",
        "model-downloads feature is not enabled",
    ))
}

pub fn validate_hf_repo_id(repo_id: &str) -> Result<()> {
    let parts: Vec<&str> = repo_id.split('/').collect();
    let valid = parts.len() == 2
        && parts.iter().all(|part| !part.is_empty())
        && !repo_id.contains("..")
        && !repo_id.contains("//")
        && repo_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '/'));
    if valid {
        Ok(())
    } else {
        Err(model_error(
            "validate-hf-repo-id",
            format!("invalid Hugging Face repo id: {repo_id:?}"),
        ))
    }
}

pub fn validate_hf_file_path(path: &str) -> Result<()> {
    let valid = !path.trim().is_empty()
        && !path.contains("..")
        && !path.contains('\0')
        && !path.starts_with('/')
        && !path.starts_with('\\')
        && !path.contains("//")
        && !path.contains("\\\\");
    if valid {
        Ok(())
    } else {
        Err(model_error(
            "validate-hf-file-path",
            format!("unsafe Hugging Face file path: {path:?}"),
        ))
    }
}

#[cfg(feature = "model-downloads")]
pub fn download_hf_model(
    spec: &HfModelSpec,
    cache_root: &Path,
    auth_token: Option<&str>,
) -> Result<HfDownloadReport> {
    let spec = if spec.backend == LocalRuntimeKind::OnnxOrt {
        let metadata = fetch_hf_repo_metadata(spec.repo_id.as_str(), auth_token)?;
        expand_onnx_spec_from_metadata(spec.retained(), &metadata)?
    } else {
        spec.retained()
    };
    validate_hf_repo_id(spec.repo_id.as_str())?;
    for file in &spec.files {
        validate_hf_file_path(file.path.as_str())?;
    }

    let cache_dir = model_cache_dir(cache_root, &spec.repo_id, &spec.revision);
    std::fs::create_dir_all(&cache_dir).map_err(|source| MemoryError::Io {
        path: cache_dir.retained().into(),
        source,
    })?;

    let client = reqwest::blocking::Client::builder()
        .user_agent("ocentra-enforcer-memory/0.1.0")
        .build()
        .map_err(|source| {
            model_error(
                "build-hf-client",
                format!("failed to build HTTP client: {source}"),
            )
        })?;

    let mut downloaded_files = Vec::new();
    for file in &spec.files {
        let local_path = cache_dir.join(file.path.as_str());
        if let Some(parent) = local_path.parent() {
            std::fs::create_dir_all(parent).map_err(|source| MemoryError::Io {
                path: parent.to_path_buf().into(),
                source,
            })?;
        }

        if !local_path.exists() {
            let url = hf_resolve_url(
                spec.repo_id.as_str(),
                spec.revision.as_str(),
                file.path.as_str(),
            );
            let auth_header = auth_token
                .filter(|token| !token.trim().is_empty())
                .map(ToOwned::to_owned)
                .or_else(env_hf_token);
            let mut request = client.get(url);
            if let Some(token) = auth_header.as_deref() {
                request = request.bearer_auth(token);
            }
            let mut response = request.send().map_err(|source| {
                model_error(
                    "download-hf-file",
                    format!("failed to request {}: {source}", file.path.as_str()),
                )
            })?;
            if !response.status().is_success() {
                return Err(model_error(
                    "download-hf-file",
                    format!(
                        "HTTP {} while downloading {}",
                        response.status(),
                        file.path.as_str()
                    ),
                ));
            }
            let partial_path = local_path.with_extension(format!(
                "{}download",
                local_path
                    .extension()
                    .and_then(|extension| extension.to_str())
                    .map(|extension| format!("{extension}."))
                    .unwrap_or_default()
            ));
            let mut output =
                std::fs::File::create(&partial_path).map_err(|source| MemoryError::Io {
                    path: partial_path.retained().into(),
                    source,
                })?;
            std::io::copy(&mut response, &mut output).map_err(|source| MemoryError::Io {
                path: partial_path.retained().into(),
                source,
            })?;
            std::fs::rename(&partial_path, &local_path).map_err(|source| MemoryError::Io {
                path: partial_path.into(),
                source,
            })?;
        }

        let size_bytes = local_path
            .metadata()
            .map_err(|source| MemoryError::Io {
                path: local_path.retained().into(),
                source,
            })?
            .len();
        let sha256 = if strict_cache_hash_enabled() {
            sha256_file(&local_path)?
        } else {
            match cached_manifest_sha256(&spec, cache_root, file.path.as_str())? {
                Some(hash) if is_sha256_hex(&hash) => hash,
                _ => sha256_file(&local_path)?,
            }
        };
        let streaming_manifest_path =
            if streaming_sidecars_enabled() && should_chunk_file(size_bytes.into()).is_required() {
                Some(
                    stream_file_into_chunks(
                        &local_path,
                        &cache_dir.join("streaming"),
                        &streaming_artifact_key(spec.repo_id.as_str())?,
                        &streaming_relative_path(file.path.as_str())?,
                    )?
                    .manifest_path
                    .into(),
                )
            } else {
                None
            };
        downloaded_files.push(HfDownloadedFile {
            source_path: file.path.retained(),
            local_path,
            sha256,
            size_bytes,
            streaming_manifest_path,
        });
    }

    let manifest_path = cache_dir.join("manifest.json");
    write_cache_manifest(&spec, &downloaded_files, &manifest_path)?;
    Ok(HfDownloadReport {
        repo_id: spec.repo_id.retained(),
        revision: spec.revision,
        cache_dir,
        manifest_path,
        downloaded_files,
    })
}

#[cfg(feature = "model-downloads")]
fn streaming_sidecars_enabled() -> bool {
    std::env::var("ENFORCER_X06_STREAMING_SIDECARS")
        .map(|value| {
            matches!(
                value.as_str(),
                "1" | "true" | "TRUE" | "yes" | "YES" | "on" | "ON"
            )
        })
        .unwrap_or(false)
}

#[cfg(feature = "model-downloads")]
fn strict_cache_hash_enabled() -> bool {
    std::env::var("ENFORCER_X06_STRICT_CACHE_HASH")
        .map(|value| {
            matches!(
                value.as_str(),
                "1" | "true" | "TRUE" | "yes" | "YES" | "on" | "ON"
            )
        })
        .unwrap_or(true)
}

#[cfg(feature = "model-downloads")]
fn cached_manifest_sha256(
    spec: &HfModelSpec,
    cache_root: &Path,
    source_path: &str,
) -> Result<Option<String>> {
    let report = match resolve_cached_hf_model_from_manifest(spec, cache_root) {
        Ok(report) => report,
        Err(_) => return Ok(None),
    };
    Ok(report
        .downloaded_files
        .into_iter()
        .find(|file| file.source_path.as_str() == source_path)
        .map(|file| file.sha256))
}

#[cfg(not(feature = "model-downloads"))]
pub fn download_hf_model(
    _spec: &HfModelSpec,
    _cache_root: &Path,
    _auth_token: Option<&str>,
) -> Result<HfDownloadReport> {
    Err(model_error(
        "download-hf-model",
        "model-downloads feature is not enabled",
    ))
}

pub fn resolve_cached_hf_model(spec: &HfModelSpec, cache_root: &Path) -> Result<HfDownloadReport> {
    validate_hf_repo_id(spec.repo_id.as_str())?;
    for file in &spec.files {
        validate_hf_file_path(file.path.as_str())?;
    }

    let cache_dir = model_cache_dir(cache_root, &spec.repo_id, &spec.revision);
    if !cache_dir.is_dir() {
        return Err(model_error(
            "resolve-cached-hf-model",
            format!("model cache directory is missing: {}", cache_dir.display()),
        ));
    }

    let mut downloaded_files = Vec::new();
    for file in &spec.files {
        let local_path = cache_dir.join(file.path.as_str());
        if !local_path.is_file() {
            return Err(model_error(
                "resolve-cached-hf-model",
                format!("cached model file is missing: {}", local_path.display()),
            ));
        }
        let sha256 = sha256_file(&local_path)?;
        let size_bytes = local_path
            .metadata()
            .map_err(|source| MemoryError::Io {
                path: local_path.retained().into(),
                source,
            })?
            .len();
        let streaming_manifest_path = streaming_manifest_for_file(
            &local_path,
            &cache_dir,
            spec.repo_id.as_str(),
            file.path.as_str(),
            size_bytes,
        )?;
        downloaded_files.push(HfDownloadedFile {
            source_path: file.path.retained(),
            local_path,
            sha256,
            size_bytes,
            streaming_manifest_path,
        });
    }

    let manifest_path = cache_dir.join("manifest.json");
    write_cache_manifest(spec, &downloaded_files, &manifest_path)?;
    Ok(HfDownloadReport {
        repo_id: spec.repo_id.retained(),
        revision: spec.revision.retained(),
        cache_dir,
        manifest_path,
        downloaded_files,
    })
}

pub fn resolve_cached_hf_model_from_manifest(
    spec: &HfModelSpec,
    cache_root: &Path,
) -> Result<HfDownloadReport> {
    spec.validate()?;
    let cache_dir = model_cache_dir(cache_root, &spec.repo_id, &spec.revision);
    let manifest_path = cache_dir.join("manifest.json");
    let manifest = load_model_cache_manifest(&manifest_path)?;
    let model_id_matches =
        manifest.model_id == spec.model_id.as_str() || manifest.model_id == spec.repo_id.as_str();
    if manifest.backend != spec.backend
        || manifest.task != spec.task
        || !model_id_matches
        || manifest.revision != spec.revision.as_str()
    {
        return Err(model_error(
            "resolve-hf-cache-manifest",
            "cache manifest does not match requested model spec",
        ));
    }

    let mut downloaded_files = Vec::with_capacity(manifest.artifacts.len());
    for artifact in manifest.artifacts {
        let local_path = cache_dir.join(artifact.path.as_str());
        if !local_path.is_file() {
            return Err(MemoryError::Io {
                path: local_path.into(),
                source: std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "manifest artifact is missing",
                ),
            });
        }
        let size_bytes = match artifact.size_bytes {
            Some(size_bytes) => size_bytes.into(),
            None => local_path
                .metadata()
                .map_err(|source| MemoryError::Io {
                    path: local_path.retained().into(),
                    source,
                })?
                .len(),
        };
        let streaming_manifest_path = artifact
            .streaming_manifest_path
            .map(|path| PathBuf::from(path.as_str()));
        if let Some(path) = &streaming_manifest_path {
            if !path.is_file() {
                return Err(MemoryError::Io {
                    path: path.retained().into(),
                    source: std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        "streaming manifest is missing",
                    ),
                });
            }
        }
        let sha256 = if is_sha256_hex(&artifact.sha256) {
            artifact.sha256.retained_display()
        } else {
            sha256_file(&local_path)?
        };
        downloaded_files.push(HfDownloadedFile {
            source_path: hf_file_path(artifact.path.retained_display())?,
            local_path,
            sha256,
            size_bytes,
            streaming_manifest_path,
        });
    }

    Ok(HfDownloadReport {
        repo_id: spec.repo_id.retained(),
        revision: spec.revision.retained(),
        cache_dir,
        manifest_path,
        downloaded_files,
    })
}

pub fn resolve_cached_hf_model_spec(
    spec: &HfModelSpec,
    cache_root: &Path,
    dimension: usize,
) -> Result<ModelSpecDto> {
    let report = resolve_cached_hf_model_from_manifest(spec, cache_root)
        .or_else(|_| resolve_cached_hf_model(spec, cache_root))?;
    let artifact = report
        .downloaded_files
        .iter()
        .find(|file| {
            spec.files
                .iter()
                .any(|entry| entry.path == file.source_path)
        })
        .ok_or_else(|| {
            model_error(
                "resolve-cached-hf-model-spec",
                "resolved cache report did not contain the requested model artifact",
            )
        })?;
    let tokenizer_path = report.cache_dir.join("tokenizer.json");
    if !tokenizer_path.is_file() {
        return Err(model_error(
            "resolve-cached-hf-model-spec",
            format!("cached tokenizer is missing: {}", tokenizer_path.display()),
        ));
    }
    let tokenizer_sha256 = sha256_file(&tokenizer_path)?;
    Ok(ModelSpecDto {
        model_id: spec.model_id.as_str().retained(),
        revision: spec.revision.as_str().retained(),
        artifact_path: artifact.local_path.retained(),
        artifact_sha256: artifact.sha256.retained(),
        tokenizer_path,
        tokenizer_sha256,
        dtype: "f32".retained(),
        dimension,
        task: spec.task,
    })
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

pub fn write_cache_manifest(
    spec: &HfModelSpec,
    downloaded_files: &[HfDownloadedFile],
    manifest_path: &Path,
) -> Result<()> {
    let artifacts = downloaded_files
        .iter()
        .map(|file| {
            let kind = spec
                .files
                .iter()
                .find(|entry| entry.path == file.source_path)
                .map(|entry| entry.kind)
                .unwrap_or(LocalRuntimeArtifactKind::Unknown);
            ModelCacheArtifactEntryDto {
                kind: Some(kind),
                path: file.source_path.as_str().retained().into(),
                sha256: file.sha256.retained().into(),
                size_bytes: Some(file.size_bytes.into()),
                streaming_manifest_path: file
                    .streaming_manifest_path
                    .as_ref()
                    .map(|path| path.display().retained_display().into()),
            }
        })
        .collect();
    let manifest = ModelCacheManifestDto {
        schema_version: MODEL_CACHE_SCHEMA_VERSION.into(),
        backend: spec.backend,
        task: spec.task,
        model_id: spec.model_id.as_str().retained().into(),
        revision: spec.revision.as_str().retained().into(),
        acceleration: spec.acceleration,
        artifacts,
    };
    let text = serde_json::to_string_pretty(&manifest)?;
    std::fs::write(manifest_path, text).map_err(|source| MemoryError::Io {
        path: manifest_path.to_path_buf().into(),
        source,
    })
}

fn streaming_manifest_for_file(
    local_path: &Path,
    cache_dir: &Path,
    repo_id: &str,
    file_path: &str,
    size_bytes: u64,
) -> Result<Option<PathBuf>> {
    if should_chunk_file(size_bytes.into()).is_required() {
        Ok(Some(
            stream_file_into_chunks(
                local_path,
                &cache_dir.join("streaming"),
                &streaming_artifact_key(repo_id)?,
                &streaming_relative_path(file_path)?,
            )?
            .manifest_path
            .to_path_buf(),
        ))
    } else {
        Ok(None)
    }
}

#[cfg(feature = "model-downloads")]
fn hf_resolve_url(repo_id: &str, revision: &str, file_path: &str) -> String {
    format!("https://huggingface.co/{repo_id}/resolve/{revision}/{file_path}")
}

#[cfg(feature = "model-downloads")]
fn env_hf_token() -> Option<String> {
    std::env::var("HF_TOKEN")
        .ok()
        .or_else(|| std::env::var("HUGGINGFACE_TOKEN").ok())
        .filter(|value| !value.trim().is_empty())
}

fn model_error(operation: &'static str, reason: impl Into<String>) -> MemoryError {
    MemoryError::ModelRuntime {
        operation: operation.into(),
        reason: reason.into().into(),
    }
}

fn hf_repository_id(value: String) -> Result<HfRepositoryId> {
    HfRepositoryId::try_new(value).ok_or_else(|| {
        model_error(
            "decode-hf-repository-id",
            "repository coordinate must be a safe owner/model value",
        )
    })
}

fn streaming_artifact_key(value: &str) -> Result<StreamingArtifactKey> {
    StreamingArtifactKey::try_new(value.retained())
        .ok_or_else(|| model_error("stream-file-into-chunks", "artifact id must not be empty"))
}

fn streaming_relative_path(value: &str) -> Result<StreamingRelativePath> {
    StreamingRelativePath::try_new(value.retained())
        .ok_or_else(|| model_error("stream-file-into-chunks", "relative path must not be empty"))
}

fn hf_file_path(value: String) -> Result<HfFilePath> {
    HfFilePath::try_new(value).ok_or_else(|| {
        model_error(
            "decode-hf-file-path",
            "artifact path must be safe and repository-relative",
        )
    })
}

fn hf_revision(value: String) -> Result<HfRevision> {
    HfRevision::try_new(value).ok_or_else(|| {
        model_error(
            "decode-hf-revision",
            "revision must not be empty or traversal-like",
        )
    })
}

fn hf_model_id(value: String) -> Result<HfModelId> {
    HfModelId::try_new(value)
        .ok_or_else(|| model_error("decode-hf-model-id", "model id must not be empty"))
}
