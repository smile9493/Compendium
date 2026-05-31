//! Knowledge entry types with standardized YAML front matter.
//!
//! Every Markdown file in the wiki must conform to this schema.

use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::path::PathBuf;

/// Custom serializer for DateTime<Utc> that adapts to the serialization format:
/// - Human-readable (YAML/JSON): RFC 3339 string
/// - Binary (bincode): i64 epoch seconds
fn serialize_utc_date<S>(date: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    if serializer.is_human_readable() {
        date.to_rfc3339().serialize(serializer)
    } else {
        date.timestamp().serialize(serializer)
    }
}

/// Custom deserializer for DateTime<Utc> that accepts:
/// - Human-readable (YAML/JSON): RFC 3339 string or "YYYY-MM-DD" date-only string
/// - Binary (bincode): i64 epoch seconds
fn deserialize_utc_date<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
where
    D: Deserializer<'de>,
{
    if deserializer.is_human_readable() {
        let s = String::deserialize(deserializer)?;
        if let Ok(dt) = DateTime::parse_from_rfc3339(&s) {
            return Ok(dt.with_timezone(&Utc));
        }
        if let Ok(naive) = NaiveDate::parse_from_str(&s, "%Y-%m-%d")
            && let Some(dt) = naive.and_hms_opt(0, 0, 0)
        {
            return Ok(DateTime::from_naive_utc_and_offset(dt, Utc));
        }
        Err(serde::de::Error::custom(format!(
            "invalid datetime format: '{}', expected RFC 3339 or YYYY-MM-DD",
            s
        )))
    } else {
        let ts = i64::deserialize(deserializer)?;
        Utc.timestamp_opt(ts, 0)
            .single()
            .ok_or_else(|| serde::de::Error::custom(format!("invalid timestamp: {}", ts)))
    }
}

/// Karpathy-style page type (orthogonal to `level`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum EntryType {
    #[default]
    Concept,
    Entity,
    #[serde(alias = "source-summary")]
    SourceSummary,
    Comparison,
    Overview,
}

/// Type of media attachment.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MediaType {
    Image,
    Table,
    Figure,
    Diagram,
    Code,
    Audio,
    Video,
}

/// A media attachment associated with a knowledge entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct MediaAttachment {
    /// Type of media.
    #[serde(rename = "type")]
    pub media_type: MediaType,
    /// Relative path to the media file (within wiki directory).
    pub path: String,
    /// Human-readable description of the media content.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Optional VLM-generated semantic description for accessibility.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alt_text: Option<String>,
}

/// Confidence in claims on this page (not PDF extraction quality).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum EntryConfidence {
    High,
    #[default]
    Medium,
    Low,
}

/// A single claim extracted from a knowledge entry, with supporting evidence.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Claim {
    /// The factual statement being made.
    pub statement: String,
    /// Supporting evidence or reference (e.g. "RFC 7540, Section 2").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence: Option<String>,
    /// Confidence in this specific claim.
    #[serde(default)]
    pub confidence: EntryConfidence,
}

/// Classification level of a knowledge entry in the compilation pyramid.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum EntryLevel {
    /// Raw extraction — direct PDF-to-text, lives in `raw/`.
    #[serde(alias = "L0")]
    L0,
    /// Atomic concept — single idea, lives in `wiki/<domain>/`.
    #[serde(alias = "L1")]
    #[default]
    L1,
    /// Aggregation — synthesis of multiple L1 entries on one sub-topic.
    #[serde(alias = "L2")]
    L2,
    /// Domain map — top-level navigation for an entire field.
    #[serde(alias = "L3")]
    L3,
}

impl std::fmt::Display for CompileStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "Pending"),
            Self::Compiling => write!(f, "Compiling"),
            Self::Compiled => write!(f, "Compiled"),
            Self::NeedsRecompile => write!(f, "NeedsRecompile"),
            Self::Failed => write!(f, "Failed"),
        }
    }
}

impl std::fmt::Display for EntryLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::L0 => write!(f, "L0"),
            Self::L1 => write!(f, "L1"),
            Self::L2 => write!(f, "L2"),
            Self::L3 => write!(f, "L3"),
        }
    }
}

/// Publication visibility (orthogonal to compilation lifecycle).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum PublishStatus {
    #[default]
    Draft,
    Published,
    Blocked,
}

impl std::fmt::Display for PublishStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Draft => write!(f, "draft"),
            Self::Published => write!(f, "published"),
            Self::Blocked => write!(f, "blocked"),
        }
    }
}

/// Compilation status tracking.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum CompileStatus {
    /// Newly extracted, awaiting AI compilation.
    #[serde(alias = "Pending")]
    #[default]
    Pending,
    /// Currently being compiled by AI.
    #[serde(alias = "Compiling")]
    Compiling,
    /// Successfully compiled into a wiki entry.
    #[serde(alias = "Compiled")]
    Compiled,
    /// Needs recompilation due to quality drift or instruction change.
    #[serde(alias = "NeedsRecompile")]
    NeedsRecompile,
    /// Compilation failed.
    #[serde(alias = "Failed")]
    Failed,
}

/// Standardized YAML front matter for every knowledge entry.
///
/// This is the single source of truth for entry metadata.
/// All indexes (Tantivy, petgraph) are derived from these fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeEntry {
    // === Identity ===
    /// Human-readable title of the concept.
    pub title: String,
    /// Domain classification, e.g. "IT", "Math", "Philosophy".
    pub domain: String,
    /// Hierarchical path within domain, e.g. "networking/http2".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,

    // === Source Provenance ===
    /// Relative path to the source PDF (e.g. "raw/paper_x.pdf").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// Page number or page range in source PDF where this concept originates.
    /// Accepts formats like "12", "70-198", "70-198,200-210".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page: Option<String>,
    /// SHA-256 hash of the source file at compilation time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_hash: Option<String>,

    // === Classification ===
    /// Free-form tags for cross-domain discovery.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Compilation level in the knowledge pyramid.
    #[serde(default)]
    pub level: EntryLevel,
    /// Page type (concept, entity, source-summary, comparison, overview).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entry_type: Option<EntryType>,
    /// Confidence in synthesized claims on this page.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<EntryConfidence>,
    /// Explicit importance score 0.0–1.0, used for skeleton indexing decisions.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub importance: Option<f32>,

    // === Linkage ===
    /// Paths to entries this entry explicitly contradicts.
    #[serde(default)]
    pub contradictions: Vec<String>,
    /// Paths to related entries (hand-authored or AI-suggested).
    #[serde(default)]
    pub related: Vec<String>,
    /// Paths to entries that this entry was aggregated from (for L2/L3).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub aggregated_from: Vec<String>,
    /// Structured claims with supporting evidence for provenance tracking.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub claims: Vec<Claim>,
    /// Media attachments (images, figures, tables) associated with this entry.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub media: Vec<MediaAttachment>,

    // === Quality & Status ===
    /// Quality score 0.0–1.0, assigned during compilation or quality check.
    #[serde(default = "default_quality")]
    pub quality_score: f32,
    /// Current compilation status.
    #[serde(default)]
    pub status: CompileStatus,
    /// Search/index visibility after quality gate.
    #[serde(default)]
    pub publish_status: PublishStatus,
    /// Version counter, incremented on each recompilation.
    #[serde(default)]
    pub version: u32,

    // === Timestamps ===
    #[serde(serialize_with = "serialize_utc_date", deserialize_with = "deserialize_utc_date")]
    pub created: DateTime<Utc>,
    #[serde(serialize_with = "serialize_utc_date", deserialize_with = "deserialize_utc_date")]
    pub updated: DateTime<Utc>,
    /// Last human or agent validation of claims on this page (decay tracking).
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        serialize_with = "serialize_opt_utc_date",
        deserialize_with = "deserialize_opt_utc_date"
    )]
    pub last_validated: Option<DateTime<Utc>>,
}

fn serialize_opt_utc_date<S>(date: &Option<DateTime<Utc>>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match date {
        Some(d) => serialize_utc_date(d, serializer),
        None => serializer.serialize_none(),
    }
}

fn deserialize_opt_utc_date<'de, D>(deserializer: D) -> Result<Option<DateTime<Utc>>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt = Option::<String>::deserialize(deserializer)?;
    match opt {
        None => Ok(None),
        Some(s) if s.is_empty() => Ok(None),
        Some(s) => {
            if let Ok(dt) = DateTime::parse_from_rfc3339(&s) {
                return Ok(Some(dt.with_timezone(&Utc)));
            }
            if let Ok(naive) = NaiveDate::parse_from_str(&s, "%Y-%m-%d")
                && let Some(dt) = naive.and_hms_opt(0, 0, 0)
            {
                return Ok(Some(DateTime::from_naive_utc_and_offset(dt, Utc)));
            }
            Err(serde::de::Error::custom(format!("invalid last_validated: '{s}'")))
        }
    }
}

fn default_quality() -> f32 {
    0.0
}

impl KnowledgeEntry {
    /// Create a new L1 entry with minimal required fields.
    pub fn new(title: impl Into<String>, domain: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            title: title.into(),
            domain: domain.into(),
            category: None,
            source: None,
            page: None,
            source_hash: None,
            tags: Vec::new(),
            level: EntryLevel::L1,
            entry_type: Some(EntryType::Concept),
            confidence: Some(EntryConfidence::Medium),
            importance: None,
            contradictions: Vec::new(),
            related: Vec::new(),
            aggregated_from: Vec::new(),
            claims: Vec::new(),
            media: Vec::new(),
            quality_score: 0.0,
            status: CompileStatus::Pending,
            publish_status: PublishStatus::Draft,
            version: 1,
            created: now,
            updated: now,
            last_validated: None,
        }
    }

    /// Serialize front matter to YAML string (without the `---` delimiters).
    pub fn to_yaml(&self) -> Result<String, serde_yaml::Error> {
        serde_yaml::to_string(self)
    }

    /// Parse front matter from YAML string (without the `---` delimiters).
    pub fn from_yaml(yaml: &str) -> Result<Self, serde_yaml::Error> {
        serde_yaml::from_str(yaml)
    }

    /// Extract front matter from a complete Markdown file content.
    /// Returns `None` if no valid front matter block is found.
    pub fn from_markdown(content: &str) -> Option<Self> {
        let _body = extract_markdown_body(content)?;
        let yaml = extract_front_matter_yaml(content)?;
        match Self::from_yaml(yaml) {
            Ok(entry) => Some(entry),
            Err(e) => {
                tracing::debug!(error = %e, yaml_len = yaml.len(), "Failed to parse YAML front matter");
                None
            }
        }
    }

    /// Body after the first closing `---` delimiter (same rules as [`from_markdown`]).
    ///
    /// Returns `None` only when front matter is present but not closed. Files without
    /// front matter return the full trimmed content.
    pub fn extract_markdown_body(content: &str) -> Option<&str> {
        extract_markdown_body(content)
    }
}

/// YAML between the opening and first closing `---` (no delimiters in string).
pub fn extract_front_matter_yaml(content: &str) -> Option<&str> {
    let content = content.trim_start();
    if !content.starts_with("---") {
        return None;
    }
    let after_first = &content[3..];
    let end = after_first.find("---")?;
    Some(after_first[..end].trim())
}

/// Markdown body after front matter; preserves `---` horizontal rules in the body.
pub fn extract_markdown_body(content: &str) -> Option<&str> {
    let content = content.trim_start();
    if !content.starts_with("---") {
        return Some(content);
    }
    let after_first = &content[3..];
    let end = after_first.find("---")?;
    let mut body = &after_first[end + 3..];
    if let Some(stripped) = body.strip_prefix("\r\n") {
        body = stripped;
    } else if let Some(stripped) = body.strip_prefix('\n') {
        body = stripped;
    }
    Some(body)
}

impl KnowledgeEntry {
    /// Build a complete Markdown file: front matter + body.
    pub fn to_markdown(&self, body: &str) -> Result<String, serde_yaml::Error> {
        let yaml = self.to_yaml()?;
        Ok(format!("---\n{}---\n\n{}", yaml, body))
    }

    /// Compute the expected filename: `[Domain] Title.md`
    pub fn filename(&self) -> String {
        let safe_title = self.title.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_");
        format!("[{}] {}.md", self.domain, safe_title)
    }

    /// Compute the relative path within wiki/: `<domain>/<filename>`
    pub fn relative_path(&self) -> PathBuf {
        let domain_dir = self.domain.to_lowercase().replace(' ', "_");
        PathBuf::from(domain_dir).join(self.filename())
    }

    /// Check if this entry has minimal quality (has title, domain, at least one tag).
    pub fn has_minimal_quality(&self) -> bool {
        !self.title.is_empty() && !self.domain.is_empty() && !self.tags.is_empty()
    }

    /// Bump the version and update the `updated` timestamp.
    pub fn touch(&mut self) {
        self.version += 1;
        self.updated = Utc::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_front_matter_roundtrip() {
        let entry = KnowledgeEntry {
            title: "HTTP/2 多路复用".into(),
            domain: "IT".into(),
            category: Some("networking/protocols".into()),
            source: Some("raw/rfc7540.pdf".into()),
            page: Some("12".to_string()),
            source_hash: Some("abc123".into()),
            tags: vec!["http".into(), "networking".into()],
            level: EntryLevel::L1,
            entry_type: Some(EntryType::Concept),
            confidence: Some(EntryConfidence::High),
            importance: None,
            contradictions: vec![],
            related: vec!["wiki/it/http1.md".into()],
            aggregated_from: vec![],
            claims: vec![],
            media: vec![],
            quality_score: 0.85,
            status: CompileStatus::Compiled,
            publish_status: PublishStatus::Published,
            version: 1,
            created: Utc::now(),
            updated: Utc::now(),
            last_validated: None,
        };

        let yaml = entry.to_yaml().unwrap();
        let parsed = KnowledgeEntry::from_yaml(&yaml).unwrap();
        assert_eq!(parsed.title, "HTTP/2 多路复用");
        assert_eq!(parsed.domain, "IT");
        assert_eq!(parsed.tags, vec!["http", "networking"]);
    }

    #[test]
    fn test_extract_markdown_body_preserves_horizontal_rules() {
        let md = r#"---
title: "HR"
domain: "IT"
tags: [a]
level: l1
status: compiled
quality_score: 0.5
created: 2026-01-01T00:00:00Z
updated: 2026-01-01T00:00:00Z
---

# Section

---

More content."#;
        let body = extract_markdown_body(md).expect("body");
        assert!(body.contains("# Section"));
        assert!(body.contains("---"));
        assert!(body.contains("More content"));
    }

    #[test]
    fn test_markdown_extraction() {
        let md = r#"---
title: "Test"
domain: "IT"
tags: ["a"]
level: l1
status: compiled
quality_score: 0.5
created: 2026-01-01T00:00:00Z
updated: 2026-01-01T00:00:00Z
---

# Test

Body content here."#;

        let entry = KnowledgeEntry::from_markdown(md).unwrap();
        assert_eq!(entry.title, "Test");
        assert_eq!(entry.domain, "IT");
    }

    #[test]
    fn test_date_only_and_page_range() {
        let md = r#"---
title: "Page Range Entry"
domain: "IT"
page: "70-198"
tags: ["nginx"]
level: l1
status: compiled
quality_score: 0.86
created: 2026-05-08
updated: 2026-05-08
related: []
---

# Test"#;

        let entry = KnowledgeEntry::from_markdown(md).unwrap();
        assert_eq!(entry.title, "Page Range Entry");
        assert_eq!(entry.page, Some("70-198".to_string()));
        assert_eq!(entry.domain, "IT");
        assert_eq!(entry.tags, vec!["nginx"]);
        assert_eq!(entry.quality_score, 0.86);
    }

    #[test]
    fn test_filename() {
        let mut entry = KnowledgeEntry::new("HTTP/2 多路复用", "IT");
        assert_eq!(entry.filename(), "[IT] HTTP_2 多路复用.md");
        entry.domain = "Math".into();
        assert_eq!(entry.relative_path(), PathBuf::from("math/[Math] HTTP_2 多路复用.md"));
    }
}
