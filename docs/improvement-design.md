# PDF模块改进方案设计文档

## 项目现状分析

### 当前架构
```
pdf-module-rs/
├── crates/
│   ├── pdf-core/          # 核心PDF处理引擎
│   │   ├── engine/        # Pdfium引擎实现
│   │   ├── extractor.rs   # 提取器
│   │   └── vlm_pipeline.rs # VLM增强管道
│   ├── pdf-mcp/           # MCP服务器实现
│   ├── vlm-visual-gateway/ # VLM视觉网关
│   ├── pdf-common/        # 共享类型
│   └── pdf-dashboard/     # 仪表板
```

### 当前技术栈
- **PDF引擎**: pdfium-render 0.8 (sync模式)
- **异步运行时**: Tokio
- **VLM集成**: 已实现GLM-4.6V系列模型支持
- **并发控制**: Semaphore限流
- **进度追踪**: 基础统计 (ToolStats)

---

## 一、短期改进方案 (1-3个月)

### 1.1 并行处理优化 - Rayon集成

#### 设计目标
- 实现批量PDF文件的并行处理
- 页面级别的并行提取
- 保持内存效率

#### 技术方案

**1. 添加Rayon依赖**
```toml
# Cargo.toml
[workspace.dependencies]
rayon = "1.10"
num_cpus = "1.16"
```

**2. 批量文件并行处理架构**

```rust
// crates/pdf-core/src/parallel/batch_processor.rs
use rayon::prelude::*;
use std::path::PathBuf;
use std::sync::Arc;

pub struct BatchProcessor {
    pipeline: Arc<McpPdfPipeline>,
    config: BatchConfig,
}

#[derive(Debug, Clone)]
pub struct BatchConfig {
    pub max_files_parallel: usize,
    pub max_pages_parallel: usize,
    pub chunk_size: usize,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            max_files_parallel: num_cpus::get(),
            max_pages_parallel: 4,
            chunk_size: 10,
        }
    }
}

impl BatchProcessor {
    pub fn new(pipeline: Arc<McpPdfPipeline>, config: BatchConfig) -> Self {
        Self { pipeline, config }
    }

    pub fn process_batch(
        &self,
        files: &[PathBuf],
        options: &ExtractOptions,
    ) -> Vec<(PathBuf, Result<StructuredExtractionResult, PdfModuleError>)> {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(self.config.max_files_parallel)
            .thread_name(|i| format!("pdf-batch-{}", i))
            .build()
            .expect("Failed to create thread pool");

        pool.install(|| {
            files
                .par_iter()
                .map(|file_path| {
                    let rt = tokio::runtime::Handle::current();
                    let result = rt.block_on(self.pipeline.extract_structured(file_path, options));
                    (file_path.clone(), result)
                })
                .collect()
        })
    }
}
```

**3. 页面级并行提取**

```rust
// crates/pdf-core/src/engine/parallel_pdfium.rs
impl PdfiumEngine {
    pub fn extract_structured_parallel(
        data: &[u8],
        page_range: Option<(u32, u32)>,
        config: &ParallelConfig,
    ) -> PdfResult<StructuredExtractionResult> {
        let pdfium = Self::get_pdfium()?;
        let document = pdfium.load_pdf_from_byte_slice(data, None)?;
        let pages = document.pages();
        let total_pages = pages.len() as u32;

        let (start, end) = page_range.unwrap_or((0, total_pages));
        let page_indices: Vec<u32> = (start..end).collect();

        let page_metas: Vec<PageMetadata> = page_indices
            .par_iter()
            .map(|&i| {
                let page = pages.get(i as u16)?;
                let text = page.text()?;
                let text_str = text.all();

                Ok(PageMetadata {
                    page_number: i + 1,
                    text: text_str.trim().to_string(),
                    bbox: Some((0.0, 0.0, page.width().value as f64, page.height().value as f64)),
                    lines: vec![],
                })
            })
            .collect::<Result<Vec<_>, PdfiumError>>()
            .map_err(|e| PdfModuleError::Extraction(e.to_string()))?;

        let extracted_text = page_metas.iter()
            .map(|p| p.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");

        Ok(StructuredExtractionResult {
            extracted_text,
            page_count: total_pages,
            pages: page_metas,
            extraction_metadata: None,
            file_info: FileInfo::default(),
        })
    }
}
```

#### 性能指标
- **目标**: 4核CPU上批量处理速度提升 3-4x
- **内存**: 控制在单文件处理的 1.5x 以内
- **延迟**: 单文件处理延迟不增加

---

### 1.2 MCP采样能力实现

#### 设计目标
- 实现Server-initiated LLM调用
- 支持采样请求/响应协议
- 集成到现有MCP服务器

#### 技术方案

**1. MCP采样协议扩展**

```rust
// crates/pdf-mcp/src/sampling/mod.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct SamplingRequest {
    pub messages: Vec<SamplingMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_preferences: Option<ModelPreferences>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_context: Option<IncludeContext>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SamplingMessage {
    pub role: Role,
    pub content: SamplingContent,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SamplingContent {
    Text { text: String },
    Image { data: String, mime_type: String },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SamplingResponse {
    pub model: String,
    pub role: Role,
    pub content: SamplingContent,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<String>,
}
```

**2. 采样管理器实现**

```rust
// crates/pdf-mcp/src/sampling/manager.rs
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};

pub struct SamplingManager {
    request_tx: mpsc::Sender<SamplingTask>,
}

struct SamplingTask {
    request: SamplingRequest,
    response_tx: oneshot::Sender<Result<SamplingResponse, SamplingError>>,
}

impl SamplingManager {
    pub fn new() -> Self {
        let (request_tx, mut request_rx) = mpsc::channel::<SamplingTask>(100);

        tokio::spawn(async move {
            while let Some(task) = request_rx.recv().await {
                let response = Self::handle_sampling_request(task.request).await;
                let _ = task.response_tx.send(response);
            }
        });

        Self { request_tx }
    }

    pub async fn request_sampling(
        &self,
        request: SamplingRequest,
    ) -> Result<SamplingResponse, SamplingError> {
        let (response_tx, response_rx) = oneshot::channel();

        self.request_tx
            .send(SamplingTask {
                request,
                response_tx,
            })
            .await
            .map_err(|_| SamplingError::ChannelClosed)?;

        response_rx.await.map_err(|_| SamplingError::ResponseTimeout)?
    }
}
```

#### 应用场景
1. **智能摘要**: 对长文档发起LLM摘要请求
2. **质量评估**: 让LLM评估提取质量
3. **结构优化**: 请求LLM优化文档结构
4. **多语言翻译**: 请求翻译服务

---

### 1.3 进度追踪系统 - Indicatif集成

#### 设计目标
- 实时进度条显示
- 多任务并行进度追踪
- 详细的性能指标

#### 技术方案

**1. 添加依赖**

```toml
# Cargo.toml
[workspace.dependencies]
indicatif = "0.17"
console = "0.15"
```

**2. 进度追踪系统架构**

```rust
// crates/pdf-core/src/progress/tracker.rs
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::sync::Arc;

pub struct ProgressTracker {
    multi_progress: Arc<MultiProgress>,
    config: ProgressConfig,
}

impl ProgressTracker {
    pub fn new(config: ProgressConfig) -> Self {
        Self {
            multi_progress: Arc::new(MultiProgress::new()),
            config,
        }
    }

    pub fn create_file_progress(&self, file_name: &str, total_pages: u64) -> ProgressBar {
        let pb = self.multi_progress.add(ProgressBar::new(total_pages));
        pb.set_style(
            ProgressStyle::with_template(&self.config.template)
                .expect("Invalid template")
                .progress_chars("█▉▊▋▌▍▎▏  ")
        );
        pb.set_message(format!("Processing {}", file_name));
        pb
    }

    pub fn create_batch_progress(&self, total_files: u64) -> ProgressBar {
        let pb = self.multi_progress.add(ProgressBar::new(total_files));
        pb.set_style(
            ProgressStyle::with_template(
                "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} files ({percent}%) ETA: {eta}"
            )
            .expect("Invalid template")
            .progress_chars("█▉▊▋▌▍▎▏  ")
        );
        pb.set_message("Batch processing");
        pb
    }
}
```

---

## 二、中期改进方案 (3-6个月)

### 2.1 知识图谱增强

#### 设计目标
- 实现更智能的知识关联
- 支持语义相似度搜索
- 自动知识推理

#### 技术方案

**1. 向量嵌入集成**

```rust
// crates/pdf-core/src/knowledge/index/vector.rs
use ort::{GraphOptimizationLevel, Session};

pub struct VectorIndex {
    session: Session,
    dimension: usize,
}

impl VectorIndex {
    pub fn new(model_path: &Path) -> PdfResult<Self> {
        let session = Session::builder()
            .map_err(|e| PdfModuleError::VectorIndex(e.to_string()))?
            .with_optimization_level(GraphOptimizationLevel::All)
            .map_err(|e| PdfModuleError::VectorIndex(e.to_string()))?
            .commit_from_file(model_path)
            .map_err(|e| PdfModuleError::VectorIndex(e.to_string()))?;

        Ok(Self {
            session,
            dimension: 768, // 默认维度
        })
    }

    pub fn embed(&self, text: &str) -> PdfResult<Vec<f32>> {
        // 使用ONNX模型生成嵌入向量
        todo!()
    }

    pub fn search(&self, query: &[f32], top_k: usize) -> PdfResult<Vec<SearchHit>> {
        // 向量相似度搜索
        todo!()
    }
}
```

**2. 语义链接建议**

```rust
impl GraphIndex {
    pub fn suggest_semantic_links(
        &self,
        entry_path: &str,
        vector_index: &VectorIndex,
        top_k: usize,
    ) -> PdfResult<Vec<LinkSuggestion>> {
        // 结合图结构和语义相似度
        todo!()
    }
}
```

---

### 2.2 中文分词优化

#### 设计目标
- 从n-gram升级到专业分词
- 提升搜索精度
- 减少索引膨胀

#### 技术方案

```rust
// crates/pdf-core/src/knowledge/index/tokenizer.rs
use jieba_rs::Jieba;

pub struct JiebaTokenizer {
    jieba: Jieba,
}

impl JiebaTokenizer {
    pub fn new() -> Self {
        Self {
            jieba: Jieba::new(),
        }
    }

    pub fn tokenize(&self, text: &str) -> Vec<String> {
        self.jieba.cut(text, false)
            .into_iter()
            .filter(|s| !s.trim().is_empty())
            .map(|s| s.to_string())
            .collect()
    }
}
```

---

## 三、长期改进方案 (6-12个月)

### 3.1 分布式编译

#### 设计目标
- 支持大规模PDF编译
- 分布式任务调度
- 故障恢复

#### 技术方案

```rust
// crates/pdf-core/src/distributed/scheduler.rs
pub struct DistributedScheduler {
    workers: Vec<WorkerNode>,
    task_queue: TaskQueue,
}

impl DistributedScheduler {
    pub async fn schedule_batch(&self, files: &[PathBuf]) -> PdfResult<Vec<CompileResult>> {
        // 分片调度
        // 故障转移
        // 结果聚合
        todo!()
    }
}
```

---

### 3.2 知识推理引擎

#### 设计目标
- 自动知识推理
- 矛盾检测与解决
- 知识图谱补全

#### 技术方案

```rust
// crates/pdf-core/src/knowledge/reasoning/engine.rs
pub struct ReasoningEngine {
    graph: GraphIndex,
    rules: Vec<InferenceRule>,
}

impl ReasoningEngine {
    pub fn infer(&self, entry: &KnowledgeEntry) -> PdfResult<Vec<InferredKnowledge>> {
        // 基于规则的推理
        // 矛盾检测
        // 知识补全
        todo!()
    }
}
```

---

## 四、实施路线图

```
Phase 1 (1-3月):
├── Rayon并行处理
├── MCP采样协议
└── Indicatif进度追踪

Phase 2 (3-6月):
├── 向量嵌入索引
├── Jieba分词优化
└── 语义链接建议

Phase 3 (6-12月):
├── 分布式编译
├── 知识推理引擎
└── 多模态知识支持
```

---

## 五、性能基准

### 当前性能

| 操作 | 延迟 | 吞吐量 |
|------|------|--------|
| 单PDF提取 | 50-200ms | 5-20 docs/s |
| 全文搜索 | 1-10ms | 100+ queries/s |
| 图谱遍历 | <1ms | 1000+ ops/s |

### 目标性能

| 操作 | 当前 | 目标 | 提升 |
|------|------|------|------|
| 批量提取 | 5 docs/s | 20 docs/s | 4x |
| 中文搜索精度 | 70% | 90% | +20% |
| 语义搜索 | N/A | 支持 | 新增 |

---

## 六、风险评估

| 风险 | 影响 | 概率 | 缓解措施 |
|------|------|------|----------|
| FFI稳定性 | 高 | 中 | catch_unwind + 超时 |
| 内存膨胀 | 中 | 中 | 流式处理 + 限制 |
| 模型依赖 | 中 | 低 | 本地模型 + 降级 |

---

## 七、结论

本改进方案按照短期、中期、长期三个阶段规划，优先解决性能瓶颈和用户体验问题。关键改进点：

1. **并行处理**: Rayon集成提升批量处理效率
2. **智能采样**: MCP采样协议增强AI交互
3. **语义搜索**: 向量索引提升搜索质量
4. **专业分词**: Jieba替代n-gram

建议按照路线图逐步实施，每个阶段完成后进行性能基准测试验证。
