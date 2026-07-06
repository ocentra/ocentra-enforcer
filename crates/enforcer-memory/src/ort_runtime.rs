//! ONNX Runtime-backed embedding/reranker implementations.
//!
//! Compiled only with `ort-models`. The default build never links ORT.

#[cfg(feature = "ort-models")]
mod real {
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use ort::{
        memory::Allocator,
        session::{builder::GraphOptimizationLevel, RunOptions, Session, SessionInputValue},
        value::{Shape, Tensor},
    };
    use tokenizers::Tokenizer;

    use crate::embed::{Embedder, EmbeddingModelInfo, LoadState, ResourceClass};
    use crate::error::{MemoryError, Result};
    use crate::model_runtime::{
        validate_embedding_output, validate_file_hash, validate_reranker_scores, ModelSpec,
        ProviderKind, DEFAULT_EMBEDDING_MODEL_ID, DEFAULT_RERANKER_MODEL_ID,
    };
    use crate::ranking::RankedHit;
    use crate::rerank::Reranker;

    const QWEN3_LAYER_COUNT: usize = 28;
    const QWEN3_KV_HEAD_COUNT: usize = 8;
    const QWEN3_HEAD_DIM: usize = 128;

    #[derive(Clone)]
    pub struct OrtRuntime {
        model_path: PathBuf,
        tokenizer_path: PathBuf,
        provider: ProviderKind,
        session: Arc<Mutex<Session>>,
        tokenizer: Arc<Tokenizer>,
    }

    pub struct OrtEmbedder {
        runtime: OrtRuntime,
        model_info: EmbeddingModelInfo,
    }

    pub struct OrtReranker {
        runtime: OrtRuntime,
    }

    impl OrtRuntime {
        pub fn load(spec: &ModelSpec, provider: ProviderKind) -> Result<Self> {
            validate_file_hash(
                &spec.artifact_path,
                &spec.artifact_sha256,
                "validate-ort-model-hash",
            )?;
            validate_file_hash(
                &spec.tokenizer_path,
                &spec.tokenizer_sha256,
                "validate-ort-tokenizer-hash",
            )?;
            let tokenizer = Tokenizer::from_file(&spec.tokenizer_path).map_err(|source| {
                model_error(
                    "load-ort-tokenizer",
                    format!("failed to load tokenizer: {source}"),
                )
            })?;
            let session = build_session(&spec.artifact_path, provider)?;
            Ok(Self {
                model_path: spec.artifact_path.clone(),
                tokenizer_path: spec.tokenizer_path.clone(),
                provider,
                session: Arc::new(Mutex::new(session)),
                tokenizer: Arc::new(tokenizer),
            })
        }

        pub fn model_path(&self) -> &Path {
            &self.model_path
        }

        pub fn tokenizer_path(&self) -> &Path {
            &self.tokenizer_path
        }

        pub fn provider(&self) -> ProviderKind {
            self.provider
        }

        pub fn encode_embedding(&self, text: &str, expected_dimension: usize) -> Result<Vec<f32>> {
            self.encode_embedding_inner(text, expected_dimension, None)
        }

        pub fn encode_embedding_with_timeout(
            &self,
            text: &str,
            expected_dimension: usize,
            timeout: Duration,
        ) -> Result<Vec<f32>> {
            self.encode_embedding_inner(text, expected_dimension, Some(timeout))
        }

        fn encode_embedding_inner(
            &self,
            text: &str,
            expected_dimension: usize,
            timeout: Option<Duration>,
        ) -> Result<Vec<f32>> {
            let encoding = self.tokenizer.encode(text, true).map_err(|source| {
                model_error(
                    "encode-ort-input",
                    format!("tokenizer encode failed: {source}"),
                )
            })?;
            let ids = encoding.get_ids();
            let embedding = run_embedding_session(&self.session, ids, timeout)?;
            validate_embedding_output(&embedding, expected_dimension)?;
            Ok(embedding)
        }

        pub fn score_pair(&self, query: &str, candidate: &str) -> Result<f32> {
            self.score_pair_inner(query, candidate, None)
        }

        pub fn score_pair_with_timeout(
            &self,
            query: &str,
            candidate: &str,
            timeout: Duration,
        ) -> Result<f32> {
            self.score_pair_inner(query, candidate, Some(timeout))
        }

        fn score_pair_inner(
            &self,
            query: &str,
            candidate: &str,
            timeout: Option<Duration>,
        ) -> Result<f32> {
            let prompt = qwen3_reranker_prompt(query, candidate);
            let encoding = self.tokenizer.encode(prompt, true).map_err(|source| {
                model_error(
                    "encode-ort-rerank-input",
                    format!("tokenizer encode failed: {source}"),
                )
            })?;
            let yes_token_id = self
                .tokenizer
                .token_to_id("yes")
                .ok_or_else(|| model_error("resolve-ort-rerank-yes-token", "missing yes token"))?;
            let no_token_id = self
                .tokenizer
                .token_to_id("no")
                .ok_or_else(|| model_error("resolve-ort-rerank-no-token", "missing no token"))?;
            run_score_session(
                &self.session,
                encoding.get_ids(),
                yes_token_id,
                no_token_id,
                timeout,
            )
        }
    }

    impl OrtEmbedder {
        pub fn load(spec: &ModelSpec, provider: ProviderKind) -> Result<Self> {
            let runtime = OrtRuntime::load(spec, provider)?;
            Ok(Self {
                runtime,
                model_info: EmbeddingModelInfo {
                    embedding_model: spec.model_id.clone(),
                    dimension: spec.dimension,
                    dtype: spec.dtype.clone(),
                    similarity_metric: "cosine".to_owned(),
                    normalization: "model-output".to_owned(),
                    formatter_version: "1".to_owned(),
                    chunker_version: "1".to_owned(),
                    parser_version: "1".to_owned(),
                },
            })
        }

        pub fn embed_with_timeout(&self, text: &str, timeout: Duration) -> Result<Vec<f32>> {
            self.runtime
                .encode_embedding_with_timeout(text, self.model_info.dimension, timeout)
        }
    }

    impl Embedder for OrtEmbedder {
        fn embed(&self, text: &str) -> Result<Vec<f32>> {
            self.runtime
                .encode_embedding(text, self.model_info.dimension)
        }

        fn model_info(&self) -> EmbeddingModelInfo {
            self.model_info.clone()
        }

        fn state(&self) -> LoadState {
            LoadState::Loaded
        }

        fn resource_class(&self) -> ResourceClass {
            self.runtime.provider.resource_class()
        }
    }

    impl OrtReranker {
        pub fn load(spec: &ModelSpec, provider: ProviderKind) -> Result<Self> {
            Ok(Self {
                runtime: OrtRuntime::load(spec, provider)?,
            })
        }

        pub fn rerank_with_timeout(
            &self,
            query: &str,
            candidates: &[RankedHit],
            timeout: Duration,
        ) -> Result<Vec<RankedHit>> {
            let mut reranked = Vec::with_capacity(candidates.len());
            let mut scores = Vec::with_capacity(candidates.len());
            for hit in candidates {
                let score = self
                    .runtime
                    .score_pair_with_timeout(query, &hit.snippet, timeout)?;
                scores.push(score);
                let mut next = hit.clone();
                next.score = f64::from(score);
                reranked.push(next);
            }
            validate_reranker_scores(&scores, candidates.len())?;
            reranked.sort_by(|a, b| {
                b.score
                    .partial_cmp(&a.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| a.doc_id.cmp(&b.doc_id))
            });
            Ok(reranked)
        }
    }

    impl Reranker for OrtReranker {
        fn rerank(&self, query: &str, candidates: &[RankedHit]) -> Result<Vec<RankedHit>> {
            let mut reranked = Vec::with_capacity(candidates.len());
            let mut scores = Vec::with_capacity(candidates.len());
            for hit in candidates {
                let score = self.runtime.score_pair(query, &hit.snippet)?;
                scores.push(score);
                let mut next = hit.clone();
                next.score = f64::from(score);
                reranked.push(next);
            }
            validate_reranker_scores(&scores, candidates.len())?;
            reranked.sort_by(|a, b| {
                b.score
                    .partial_cmp(&a.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| a.doc_id.cmp(&b.doc_id))
            });
            Ok(reranked)
        }

        fn state(&self) -> LoadState {
            LoadState::Loaded
        }
    }

    fn build_session(model_path: &Path, provider: ProviderKind) -> Result<Session> {
        let providers = ort_providers(provider);
        Session::builder()
            .map_err(|source| model_error("create-ort-session", source.to_string()))?
            .with_execution_providers(&providers)
            .map_err(|source| model_error("configure-ort-providers", source.to_string()))?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(|source| model_error("configure-ort-optimization", source.to_string()))?
            .with_intra_threads(4)
            .map_err(|source| model_error("configure-ort-intra-threads", source.to_string()))?
            .with_inter_threads(2)
            .map_err(|source| model_error("configure-ort-inter-threads", source.to_string()))?
            .commit_from_file(model_path)
            .map_err(|source| model_error("load-ort-model", source.to_string()))
    }

    fn ort_providers(
        provider: ProviderKind,
    ) -> Vec<ort::execution_providers::ExecutionProviderDispatch> {
        let mut providers = Vec::new();
        match provider {
            ProviderKind::Cpu => {}
            ProviderKind::DirectMl => {
                #[cfg(windows)]
                providers
                    .push(ort::execution_providers::DirectMLExecutionProvider::default().build());
            }
            ProviderKind::OpenVino => {
                providers
                    .push(ort::execution_providers::OpenVINOExecutionProvider::default().build());
            }
            ProviderKind::Cuda => {
                providers.push(ort::execution_providers::CUDAExecutionProvider::default().build());
            }
            ProviderKind::Vulkan => {}
            ProviderKind::CoreMl => {
                #[cfg(target_os = "macos")]
                providers
                    .push(ort::execution_providers::CoreMLExecutionProvider::default().build());
            }
            ProviderKind::Npu => {}
        }
        providers.push(ort::execution_providers::CPUExecutionProvider::default().build());
        providers
    }

    fn run_embedding_session(
        session: &Arc<Mutex<Session>>,
        token_ids: &[u32],
        timeout: Option<Duration>,
    ) -> Result<Vec<f32>> {
        let seq_len = token_ids.len();
        let shape = [1usize, seq_len];
        let input_ids: Vec<i64> = token_ids.iter().map(|&id| i64::from(id)).collect();
        let attention_mask: Vec<i64> = vec![1; seq_len];
        let position_ids: Vec<i64> = (0..seq_len as i64).collect();
        let input_ids = Tensor::from_array((shape, input_ids))
            .map_err(|source| model_error("build-ort-input-ids", source.to_string()))?;
        let attention_mask = Tensor::from_array((shape, attention_mask))
            .map_err(|source| model_error("build-ort-attention-mask", source.to_string()))?;
        let position_ids = Tensor::from_array((shape, position_ids))
            .map_err(|source| model_error("build-ort-position-ids", source.to_string()))?;
        let inputs = qwen3_inputs(input_ids, attention_mask, position_ids)?;
        let mut locked = session
            .lock()
            .map_err(|source| model_error("lock-ort-session", source.to_string()))?;
        let run_options = run_options_with_optional_terminator(timeout, "create-ort-run-options")?;
        let outputs = locked
            .run_with_options(inputs, &run_options)
            .map_err(|source| model_error("run-ort-embedding", source.to_string()))?;
        let output = &outputs[0];
        let (shape, data) = output
            .try_extract_tensor::<f32>()
            .map_err(|source| model_error("read-ort-embedding-output", source.to_string()))?;
        mean_pool_embedding(shape, data, seq_len)
    }

    fn run_score_session(
        session: &Arc<Mutex<Session>>,
        token_ids: &[u32],
        yes_token_id: u32,
        no_token_id: u32,
        timeout: Option<Duration>,
    ) -> Result<f32> {
        let seq_len = token_ids.len();
        let shape = [1usize, seq_len];
        let input_ids: Vec<i64> = token_ids.iter().map(|&id| i64::from(id)).collect();
        let attention_mask: Vec<i64> = vec![1; seq_len];
        let position_ids: Vec<i64> = (0..seq_len as i64).collect();
        let input_ids = Tensor::from_array((shape, input_ids))
            .map_err(|source| model_error("build-ort-rerank-input-ids", source.to_string()))?;
        let attention_mask = Tensor::from_array((shape, attention_mask))
            .map_err(|source| model_error("build-ort-rerank-attention-mask", source.to_string()))?;
        let position_ids = Tensor::from_array((shape, position_ids))
            .map_err(|source| model_error("build-ort-rerank-position-ids", source.to_string()))?;
        let inputs = qwen3_inputs(input_ids, attention_mask, position_ids)?;
        let mut locked = session
            .lock()
            .map_err(|source| model_error("lock-ort-rerank-session", source.to_string()))?;
        let run_options =
            run_options_with_optional_terminator(timeout, "create-ort-rerank-run-options")?;
        let outputs = locked
            .run_with_options(inputs, &run_options)
            .map_err(|source| model_error("run-ort-reranker", source.to_string()))?;
        let output = &outputs[0];
        let (shape, data) = output
            .try_extract_tensor::<f32>()
            .map_err(|source| model_error("read-ort-reranker-output", source.to_string()))?;
        qwen3_reranker_yes_probability(shape, data, seq_len, yes_token_id, no_token_id)
    }

    fn run_options_with_optional_terminator(
        timeout: Option<Duration>,
        operation: &'static str,
    ) -> Result<Arc<RunOptions>> {
        let run_options = Arc::new(
            RunOptions::new().map_err(|source| model_error(operation, source.to_string()))?,
        );
        if let Some(timeout) = timeout {
            let terminator = Arc::clone(&run_options);
            std::thread::spawn(move || {
                std::thread::sleep(timeout);
                let _ = terminator.terminate();
            });
        }
        Ok(run_options)
    }

    fn qwen3_inputs(
        input_ids: Tensor<i64>,
        attention_mask: Tensor<i64>,
        position_ids: Tensor<i64>,
    ) -> Result<Vec<(String, SessionInputValue<'static>)>> {
        let mut inputs = Vec::with_capacity(3 + QWEN3_LAYER_COUNT * 2);
        inputs.push(("input_ids".to_owned(), input_ids.into()));
        inputs.push(("attention_mask".to_owned(), attention_mask.into()));
        inputs.push(("position_ids".to_owned(), position_ids.into()));
        for layer_index in 0..QWEN3_LAYER_COUNT {
            inputs.push((
                format!("past_key_values.{layer_index}.key"),
                empty_qwen3_past_tensor("build-ort-past-key")?.into(),
            ));
            inputs.push((
                format!("past_key_values.{layer_index}.value"),
                empty_qwen3_past_tensor("build-ort-past-value")?.into(),
            ));
        }
        Ok(inputs)
    }

    fn empty_qwen3_past_tensor(operation: &'static str) -> Result<Tensor<f32>> {
        Tensor::<f32>::new(
            &Allocator::default(),
            Shape::new([1, QWEN3_KV_HEAD_COUNT as i64, 0, QWEN3_HEAD_DIM as i64]),
        )
        .map_err(|source| model_error(operation, source.to_string()))
    }

    fn qwen3_reranker_prompt(query: &str, document: &str) -> String {
        const SYSTEM_PROMPT: &str =
            "Judge whether the Document meets the requirements based on the Query and the Instruct provided. \
             Note that the answer can only be \"yes\" or \"no\".";
        const INSTRUCTION: &str =
            "Given a web search query, retrieve relevant passages that answer the query";
        format!(
            "<|im_start|>system\n{SYSTEM_PROMPT}<|im_end|>\n\
             <|im_start|>user\n<Instruct>: {INSTRUCTION}\n\n<Query>: {query}\n\n<Document>: {document}<|im_end|>\n\
             <|im_start|>assistant\n<think>\n\n</think>\n"
        )
    }

    fn qwen3_reranker_yes_probability(
        shape: &Shape,
        data: &[f32],
        requested_seq_len: usize,
        yes_token_id: u32,
        no_token_id: u32,
    ) -> Result<f32> {
        let shape = shape.as_ref();
        if shape.len() < 3 {
            return Err(model_error(
                "score-ort-reranker-output",
                format!("expected rank-3 logits, got shape {shape:?}"),
            ));
        }
        let seq_len = usize::try_from(shape[1])
            .map_err(|source| model_error("score-ort-reranker-output", source.to_string()))?;
        let vocab_size = usize::try_from(shape[2])
            .map_err(|source| model_error("score-ort-reranker-output", source.to_string()))?;
        let active_seq_len = requested_seq_len.min(seq_len).max(1);
        let yes_index = yes_token_id as usize;
        let no_index = no_token_id as usize;
        if yes_index >= vocab_size || no_index >= vocab_size {
            return Err(model_error(
                "score-ort-reranker-output",
                format!(
                    "yes/no token ids out of vocabulary bounds: yes={yes_index}, no={no_index}, vocab={vocab_size}"
                ),
            ));
        }
        let last_token_offset = (active_seq_len - 1) * vocab_size;
        let yes_logit = *data
            .get(last_token_offset + yes_index)
            .ok_or_else(|| model_error("score-ort-reranker-output", "missing yes logit"))?;
        let no_logit = *data
            .get(last_token_offset + no_index)
            .ok_or_else(|| model_error("score-ort-reranker-output", "missing no logit"))?;
        let max_logit = yes_logit.max(no_logit);
        let yes_exp = (yes_logit - max_logit).exp();
        let no_exp = (no_logit - max_logit).exp();
        Ok(yes_exp / (yes_exp + no_exp))
    }

    fn mean_pool_embedding(
        shape: &Shape,
        data: &[f32],
        requested_seq_len: usize,
    ) -> Result<Vec<f32>> {
        let shape = shape.as_ref();
        if shape.len() < 3 {
            return Err(model_error(
                "pool-ort-embedding",
                format!("expected rank-3 output, got shape {shape:?}"),
            ));
        }
        let seq_len = usize::try_from(shape[1])
            .map_err(|source| model_error("pool-ort-embedding", source.to_string()))?;
        let dim = usize::try_from(shape[2])
            .map_err(|source| model_error("pool-ort-embedding", source.to_string()))?;
        let active_seq_len = requested_seq_len.min(seq_len).max(1);
        let mut pooled = vec![0.0f32; dim];
        for token_index in 0..active_seq_len {
            for (dim_index, pooled_value) in pooled.iter_mut().enumerate().take(dim) {
                let data_index = token_index * dim + dim_index;
                if let Some(value) = data.get(data_index) {
                    *pooled_value += *value;
                }
            }
        }
        for value in &mut pooled {
            *value /= active_seq_len as f32;
        }
        Ok(pooled)
    }

    pub fn default_embedding_model_id() -> &'static str {
        DEFAULT_EMBEDDING_MODEL_ID
    }

    pub fn default_reranker_model_id() -> &'static str {
        DEFAULT_RERANKER_MODEL_ID
    }

    fn model_error(operation: &'static str, reason: impl Into<String>) -> MemoryError {
        MemoryError::ModelRuntime {
            operation,
            reason: reason.into(),
        }
    }
}

#[cfg(feature = "ort-models")]
pub use real::{
    default_embedding_model_id, default_reranker_model_id, OrtEmbedder, OrtReranker, OrtRuntime,
};
