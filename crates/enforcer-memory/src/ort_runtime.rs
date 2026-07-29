//! ONNX Runtime-backed embedding/reranker implementations.
//!
//! Compiled only with `ort-models`. The default build never links ORT.

#[cfg(feature = "ort-models")]
pub mod real {
    use std::path::{Path, PathBuf};
    use std::sync::mpsc::{sync_channel, SyncSender};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use ort::{
        memory::Allocator,
        session::{builder::GraphOptimizationLevel, RunOptions, Session, SessionInputValue},
        value::{Shape, Tensor},
    };
    use rten_text::Tokenizer;

    use crate::embed::{Embedder, EmbeddingModelInfo};
    use crate::error::{MemoryError, Result};
    use crate::model_runtime::{
        validate_embedding_output, validate_file_hash, validate_reranker_scores, ModelSpecDto,
        DEFAULT_EMBEDDING_MODEL_ID, DEFAULT_RERANKER_MODEL_ID,
    };
    use crate::owned_boundary::{Retained, RetainedDisplay};
    use crate::ranking::RankedHit;
    use crate::rerank::Reranker;
    use enforcer_domain::memory_types::ProviderKind;
    use enforcer_domain::memory_types::{LoadState, ResourceClass};

    const TOKENIZER_QUEUE_CAPACITY: usize = 32;
    const QWEN3_LAYER_COUNT: usize = 28;
    const QWEN3_KV_HEAD_COUNT: usize = 8;
    const QWEN3_HEAD_DIM: usize = 128;

    enum TokenizerRequest {
        Encode {
            text: String,
            reply: SyncSender<std::result::Result<Vec<u32>, String>>,
        },
        ResolveToken {
            token: String,
            reply: SyncSender<std::result::Result<u32, String>>,
        },
    }

    impl std::fmt::Debug for TokenizerRequest {
        fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::Encode { .. } => formatter
                    .debug_struct("TokenizerRequest::Encode")
                    .field("text", &"[REDACTED]")
                    .field("reply", &"[REDACTED]")
                    .finish(),
                Self::ResolveToken { .. } => formatter
                    .debug_struct("TokenizerRequest::ResolveToken")
                    .field("token", &"[REDACTED]")
                    .field("reply", &"[REDACTED]")
                    .finish(),
            }
        }
    }

    impl std::fmt::Display for TokenizerRequest {
        fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let kind = match self {
                Self::Encode { .. } => "Encode",
                Self::ResolveToken { .. } => "ResolveToken",
            };
            write!(formatter, "TokenizerRequest::{kind}[REDACTED]")
        }
    }

    #[derive(Clone)]
    pub struct OrtTokenizer {
        requests: SyncSender<TokenizerRequest>,
        end_of_text_token_id: u32,
    }

    impl std::fmt::Debug for OrtTokenizer {
        fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter
                .debug_struct("OrtTokenizer")
                .field("requests", &"[REDACTED]")
                .field("end_of_text_token_id", &self.end_of_text_token_id)
                .finish()
        }
    }

    impl std::fmt::Display for OrtTokenizer {
        fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("OrtTokenizer[REDACTED]")
        }
    }

    impl OrtTokenizer {
        pub fn load(path: PathBuf) -> Result<Self> {
            let (requests, receiver) = sync_channel(TOKENIZER_QUEUE_CAPACITY);
            let (initialized, initialization) = sync_channel(1);
            std::thread::Builder::new()
                .name("enforcer-memory-tokenizer".to_owned())
                .spawn(move || {
                    let tokenizer = match Tokenizer::from_file(path) {
                        Ok(tokenizer) => tokenizer,
                        Err(source) => {
                            let _ = initialized.send(Err(source.to_string()));
                            return;
                        }
                    };
                    let end_of_text_token_id = match tokenizer.get_token_id("<|endoftext|>") {
                        Ok(token_id) => token_id,
                        Err(source) => {
                            let _ = initialized.send(Err(source.to_string()));
                            return;
                        }
                    };
                    if initialized.send(Ok(end_of_text_token_id)).is_err() {
                        return;
                    }
                    while let Ok(request) = receiver.recv() {
                        match request {
                            TokenizerRequest::Encode { text, reply } => {
                                let encoded = tokenizer
                                    .encode(text.as_str(), None)
                                    .map(|encoding| encoding.token_ids().to_vec())
                                    .map_err(|source| source.to_string());
                                let _ = reply.send(encoded);
                            }
                            TokenizerRequest::ResolveToken { token, reply } => {
                                let resolved = tokenizer
                                    .get_token_id(token.as_str())
                                    .map_err(|source| source.to_string());
                                let _ = reply.send(resolved);
                            }
                        }
                    }
                })
                .map_err(|source| {
                    model_error(
                        "load-ort-tokenizer",
                        format!("failed to start tokenizer worker: {source}"),
                    )
                })?;
            let end_of_text_token_id = initialization
                .recv()
                .map_err(|source| {
                    model_error(
                        "load-ort-tokenizer",
                        format!("tokenizer worker initialization failed: {source}"),
                    )
                })?
                .map_err(|source| model_error("load-ort-tokenizer", source))?;
            Ok(Self {
                requests,
                end_of_text_token_id,
            })
        }

        pub fn encode_with_end_of_text(&self, text: &str) -> Result<Vec<u32>> {
            let (reply, response) = sync_channel(1);
            self.requests
                .send(TokenizerRequest::Encode {
                    text: text.to_owned(),
                    reply,
                })
                .map_err(|source| model_error("encode-ort-input", source.to_string()))?;
            let mut token_ids = response
                .recv()
                .map_err(|source| model_error("encode-ort-input", source.to_string()))?
                .map_err(|source| model_error("encode-ort-input", source))?;
            token_ids.push(self.end_of_text_token_id);
            Ok(token_ids)
        }

        pub fn token_id(&self, token: &str, operation: &'static str) -> Result<u32> {
            let (reply, response) = sync_channel(1);
            self.requests
                .send(TokenizerRequest::ResolveToken {
                    token: token.to_owned(),
                    reply,
                })
                .map_err(|source| model_error(operation, source.to_string()))?;
            response
                .recv()
                .map_err(|source| model_error(operation, source.to_string()))?
                .map_err(|source| model_error(operation, source))
        }
    }

    #[derive(Clone)]
    pub struct OrtRuntime {
        model_path: PathBuf,
        tokenizer_path: PathBuf,
        provider: ProviderKind,
        session: Arc<Mutex<Session>>,
        tokenizer: OrtTokenizer,
    }

    pub struct OrtEmbedder {
        runtime: OrtRuntime,
        model_info: EmbeddingModelInfo,
    }

    pub struct OrtReranker {
        runtime: OrtRuntime,
    }

    impl OrtRuntime {
        pub fn load(spec: &ModelSpecDto, provider: ProviderKind) -> Result<Self> {
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
            let tokenizer = OrtTokenizer::load(spec.tokenizer_path.clone())?;
            let session = build_session(&spec.artifact_path, provider)?;
            Ok(Self {
                model_path: spec.artifact_path.retained(),
                tokenizer_path: spec.tokenizer_path.retained(),
                provider,
                session: Arc::new(Mutex::new(session)),
                tokenizer,
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
            let ids = self.tokenizer.encode_with_end_of_text(text)?;
            let embedding = run_embedding_session(&self.session, &ids, timeout)?;
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
            let ids = self.tokenizer.encode_with_end_of_text(prompt.as_str())?;
            let yes_token_id = self
                .tokenizer
                .token_id("yes", "resolve-ort-rerank-yes-token")?;
            let no_token_id = self
                .tokenizer
                .token_id("no", "resolve-ort-rerank-no-token")?;
            run_score_session(&self.session, &ids, yes_token_id, no_token_id, timeout)
        }
    }

    impl OrtEmbedder {
        pub fn load(spec: &ModelSpecDto, provider: ProviderKind) -> Result<Self> {
            let runtime = OrtRuntime::load(spec, provider)?;
            Ok(Self {
                runtime,
                model_info: EmbeddingModelInfo {
                    embedding_model: spec.model_id.retained().into(),
                    dimension: spec.dimension.into(),
                    dtype: spec.dtype.retained().into(),
                    similarity_metric: "cosine".retained().into(),
                    normalization: "model-output".retained().into(),
                    formatter_version: "1".retained().into(),
                    chunker_version: "1".retained().into(),
                    parser_version: "1".retained().into(),
                },
            })
        }

        pub fn embed_with_timeout(&self, text: &str, timeout: Duration) -> Result<Vec<f32>> {
            self.runtime.encode_embedding_with_timeout(
                text,
                self.model_info.dimension.get(),
                timeout,
            )
        }
    }

    impl Embedder for OrtEmbedder {
        fn embed(
            &self,
            text: enforcer_domain::memory_types::ParserSourceText<'_>,
        ) -> Result<enforcer_domain::memory_types::EmbeddingVector> {
            self.runtime
                .encode_embedding(text.as_str(), self.model_info.dimension.get())
                .map(Into::into)
        }

        fn model_info(&self) -> EmbeddingModelInfo {
            self.model_info.retained()
        }

        fn state(&self) -> LoadState {
            LoadState::Loaded
        }

        fn resource_class(&self) -> ResourceClass {
            self.runtime.provider.resource_class()
        }
    }

    impl OrtReranker {
        pub fn load(spec: &ModelSpecDto, provider: ProviderKind) -> Result<Self> {
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
                let mut next = hit.retained();
                next.score = f64::from(score).into();
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
        fn rerank(
            &self,
            query: enforcer_domain::memory_types::ParserSourceText<'_>,
            candidates: &[RankedHit],
        ) -> Result<Vec<RankedHit>> {
            let mut reranked = Vec::with_capacity(candidates.len());
            let mut scores = Vec::with_capacity(candidates.len());
            for hit in candidates {
                let score = self.runtime.score_pair(query.as_str(), &hit.snippet)?;
                scores.push(score);
                let mut next = hit.retained();
                next.score = f64::from(score).into();
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
            .map_err(|source| model_error("create-ort-session", source.retained_display()))?
            .with_execution_providers(&providers)
            .map_err(|source| model_error("configure-ort-providers", source.retained_display()))?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(|source| model_error("configure-ort-optimization", source.retained_display()))?
            .with_intra_threads(4)
            .map_err(|source| {
                model_error("configure-ort-intra-threads", source.retained_display())
            })?
            .with_inter_threads(2)
            .map_err(|source| {
                model_error("configure-ort-inter-threads", source.retained_display())
            })?
            .with_parallel_execution(true)
            .map_err(|source| {
                model_error(
                    "configure-ort-parallel-execution",
                    source.retained_display(),
                )
            })?
            .with_memory_pattern(true)
            .map_err(|source| {
                model_error("configure-ort-memory-pattern", source.retained_display())
            })?
            .commit_from_file(model_path)
            .map_err(|source| model_error("load-ort-model", source.retained_display()))
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
        let seq_len_i64 = i64::try_from(seq_len).unwrap_or(i64::MAX);
        let position_ids: Vec<i64> = (0..seq_len_i64).collect();
        let input_ids = Tensor::from_array((shape, input_ids))
            .map_err(|source| model_error("build-ort-input-ids", source.retained_display()))?;
        let attention_mask = Tensor::from_array((shape, attention_mask))
            .map_err(|source| model_error("build-ort-attention-mask", source.retained_display()))?;
        let position_ids = Tensor::from_array((shape, position_ids))
            .map_err(|source| model_error("build-ort-position-ids", source.retained_display()))?;
        let inputs = qwen3_inputs(input_ids, attention_mask, position_ids)?;
        let mut locked = session
            .lock()
            .map_err(|source| model_error("lock-ort-session", source.retained_display()))?;
        let run_options = run_options_with_optional_terminator(timeout, "create-ort-run-options")?;
        let outputs = locked
            .run_with_options(inputs, &run_options)
            .map_err(|source| model_error("run-ort-embedding", source.retained_display()))?;
        let output = outputs.values().next().ok_or_else(|| {
            model_error(
                "read-ort-embedding-output",
                "ONNX Runtime returned no embedding output",
            )
        })?;
        let (shape, data) = output.try_extract_tensor::<f32>().map_err(|source| {
            model_error("read-ort-embedding-output", source.retained_display())
        })?;
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
        let seq_len_i64 = i64::try_from(seq_len).unwrap_or(i64::MAX);
        let position_ids: Vec<i64> = (0..seq_len_i64).collect();
        let input_ids = Tensor::from_array((shape, input_ids)).map_err(|source| {
            model_error("build-ort-rerank-input-ids", source.retained_display())
        })?;
        let attention_mask = Tensor::from_array((shape, attention_mask)).map_err(|source| {
            model_error("build-ort-rerank-attention-mask", source.retained_display())
        })?;
        let position_ids = Tensor::from_array((shape, position_ids)).map_err(|source| {
            model_error("build-ort-rerank-position-ids", source.retained_display())
        })?;
        let inputs = qwen3_inputs(input_ids, attention_mask, position_ids)?;
        let mut locked = session
            .lock()
            .map_err(|source| model_error("lock-ort-rerank-session", source.retained_display()))?;
        let run_options =
            run_options_with_optional_terminator(timeout, "create-ort-rerank-run-options")?;
        let outputs = locked
            .run_with_options(inputs, &run_options)
            .map_err(|source| model_error("run-ort-reranker", source.retained_display()))?;
        let output = outputs.values().next().ok_or_else(|| {
            model_error(
                "read-ort-reranker-output",
                "ONNX Runtime returned no reranker output",
            )
        })?;
        let (shape, data) = output
            .try_extract_tensor::<f32>()
            .map_err(|source| model_error("read-ort-reranker-output", source.retained_display()))?;
        qwen3_reranker_yes_probability(shape, data, seq_len, yes_token_id, no_token_id)
    }

    fn run_options_with_optional_terminator(
        timeout: Option<Duration>,
        operation: &'static str,
    ) -> Result<Arc<RunOptions>> {
        let run_options = Arc::new(
            RunOptions::new()
                .map_err(|source| model_error(operation, source.retained_display()))?,
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
        inputs.push(("input_ids".retained(), input_ids.into()));
        inputs.push(("attention_mask".retained(), attention_mask.into()));
        inputs.push(("position_ids".retained(), position_ids.into()));
        (0..QWEN3_LAYER_COUNT).try_for_each(|layer_index| -> Result<()> {
            inputs.push((
                format!("past_key_values.{layer_index}.key"),
                empty_qwen3_past_tensor("build-ort-past-key")?.into(),
            ));
            inputs.push((
                format!("past_key_values.{layer_index}.value"),
                empty_qwen3_past_tensor("build-ort-past-value")?.into(),
            ));
            Ok(())
        })?;
        Ok(inputs)
    }

    fn empty_qwen3_past_tensor(operation: &'static str) -> Result<Tensor<f32>> {
        Tensor::<f32>::new(
            &Allocator::default(),
            Shape::new([
                1,
                i64::try_from(QWEN3_KV_HEAD_COUNT).unwrap_or(i64::MAX),
                0,
                i64::try_from(QWEN3_HEAD_DIM).unwrap_or(i64::MAX),
            ]),
        )
        .map_err(|source| model_error(operation, source.retained_display()))
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
        let seq_len = usize::try_from(*shape.get(1).ok_or_else(|| {
            model_error(
                "score-ort-reranker-output",
                "missing sequence dimension in rank-3 logits",
            )
        })?)
        .map_err(|source| model_error("score-ort-reranker-output", source.retained_display()))?;
        let vocab_size = usize::try_from(*shape.get(2).ok_or_else(|| {
            model_error(
                "score-ort-reranker-output",
                "missing vocabulary dimension in rank-3 logits",
            )
        })?)
        .map_err(|source| model_error("score-ort-reranker-output", source.retained_display()))?;
        let active_seq_len = requested_seq_len.min(seq_len).max(1);
        let yes_index = usize::try_from(yes_token_id).unwrap_or(usize::MAX);
        let no_index = usize::try_from(no_token_id).unwrap_or(usize::MAX);
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
        let seq_len = usize::try_from(*shape.get(1).ok_or_else(|| {
            model_error(
                "pool-ort-embedding",
                "missing sequence dimension in rank-3 embedding output",
            )
        })?)
        .map_err(|source| model_error("pool-ort-embedding", source.retained_display()))?;
        let dim = usize::try_from(*shape.get(2).ok_or_else(|| {
            model_error(
                "pool-ort-embedding",
                "missing embedding dimension in rank-3 output",
            )
        })?)
        .map_err(|source| model_error("pool-ort-embedding", source.retained_display()))?;
        let active_seq_len = requested_seq_len.min(seq_len).max(1);
        let mut pooled = vec![0.0f32; dim];
        if dim > 0 {
            for values in data.chunks(dim).take(active_seq_len) {
                for (pooled_value, value) in pooled.iter_mut().zip(values) {
                    *pooled_value += *value;
                }
            }
        }
        for value in &mut pooled {
            *value /= crate::owned_boundary::usize_to_f32(active_seq_len);
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
            operation: operation.into(),
            reason: reason.into().into(),
        }
    }
}
