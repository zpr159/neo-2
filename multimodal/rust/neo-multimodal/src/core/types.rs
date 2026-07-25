#!\[forbid(unsafe_code)\]
#![deny(
    missing_docs,
    warnings,
    trivial_casts,
    trivial_numeric_casts,
    unused_import_braces,
    unused_extern_crates
)]

//! Core types and base definitions for the Multimodal Intelligence System.
//!
//! This module defines the fundamental data structures and traits used across
//! all modality processors, embedding systems, and runtime infrastructure.

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use std::collections::{HashMap, HashSet};
use async_trait::async_trait;
use derive_more::Display;
use crate::error::{MultimodalError, MultimodalResult};

/// **Modality** represents a specific data type or format that can be processed
/// by the multimodal intelligence system (e.g., Text, Image, Audio, Video, etc.).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Display)]
pub enum Modality {
    #[display(fmt = "text")]
    Text,
    #[display(fmt = "image")]
    Image,
    #[display(fmt = "audio")]
    Audio,
    #[display(fmt = "video")]
    Video,
    #[display(fmt = "pdf")]
    Pdf,
    #[display(fmt = "docx")]
    Docx,
    #[display(fmt = "pptx")]
    Pptx,
    #[display(fmt = "xlsx")]
    Xlsx,
    #[display(fmt = "csv")]
    Csv,
    #[display(fmt = "html")]
    Html,
    #[display(fmt = "markdown")]
    Markdown,
    #[display(fmt = "ocr")]
    Ocr,
    #[display(fmt = "speech")]
    Speech,
    #[display(fmt = "video")]
    VideoAnalysis,
    #[display(fmt = "ui")]
    Ui,
    #[display(fmt = "screenshot")]
    Screenshot,
    #[display(fmt = "file")]
    File,
    #[display(fmt = "buffer")]
    Buffer,
    #[display(fmt = "stream")]
    Stream,
    #[display(fmt = "custom")]
    Custom(String),
}

/// Unique identifier for a media asset in the system.
pub use crate::id::MediaAssetId;

/// Metadata describing the properties and characteristics of a media asset.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaMetadata {
    pub id: MediaAssetId,
    pub name: String,
    pub description: Option<String>,
    pub modality: Modality,
    pub format: MediaFormat,
    pub source: MediaSource,
    pub size_bytes: u64,
    pub mime_type: String,
    pub encoding: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub modified_by: Option<String>,
    pub metadata: HashMap<String, serde_json::Value>,
    pub security_level: SecurityLevel,
    pub retention_days: Option<u32>,
}

/// Media format information including container and codec details.
#[derive(Debug, Clone, Serialize, Deserialize, Display)]
pub enum MediaFormat {
    #[display(fmt = "JPEG")]
    Jpeg,
    #[display(fmt = "PNG")]
    Png,
    #[display(fmt = "GIF")]
    Gif,
    #[display(fmt = "WEBP")]
    Webp,
    #[display(fmt = "TIFF")]
    Tiff,
    #[display(fmt = "BMP")]
    Bmp,
    #[display(fmt = "SVG")]
    Svg,
    #[display(fmt = "RAW")]
    Raw,
    #[display(fmt = "HEIC")]
    Heic,
    #[display(fmt = "AVIF")]
    Avif,
    #[display(fmt = "MP4")]
    Mp4,
    #[display(fmt = "AVI")]
    Avi,
    #[display(fmt = "MOV")]
    Mov,
    #[display(fmt = "MKV")]
    Mkv,
    #[display(fmt = "WEBM")]
    Webm,
    #[display(fmt = "FLV")]
    Flv,
    #[display(fmt = "WMV")]
    Wmv,
    #[display(fmt = "MP3")]
    Mp3,
    #[display(fmt = "WAV")]
    Wav,
    #[display(fmt = "FLAC")]
    Flac,
    #[display(fmt = "AAC")]
    Aac,
    #[display(fmt = "OGG")]
    Ogg,
    #[display(fmt = "OPUS")]
    Opus,
    #[display(fmt = "PDF")]
    Pdf,
    #[display(fmt = "DOCX")]
    Docx,
    #[display(fmt = "PPTX")]
    Pptx,
    #[display(fmt = "XLSX")]
    Xlsx,
    #[display(fmt = "CSV")]
    Csv,
    #[display(fmt = "HTML")]
    Html,
    #[display(fmt = "MD")]
    Markdown,
    #[display(fmt = "TXT")]
    Txt,
    #[display(fmt = "JSON")]
    Json,
    #[display(fmt = "YAML")]
    Yaml,
    #[display(fmt = "XML")]
    Xml,
    #[display(fmt = "PARQUET")]
    Parquet,
    #[display(fmt = "ORC")]
    Orc,
    #[display(fmt = "AVRO")]
    Avro,
    #[display(fmt = "CUSTOM")]
    Custom(String),
}

/// Media source information indicating where the media originated.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MediaSource {
    LocalDisk { path: String },
    CloudStorage { bucket: String, key: String },
    HttpUrl { url: String },
    Stream { uri: String },
    Upload { filename: String, content_type: String },
    Capture { device: String, parameters: HashMap<String, serde_json::Value> },
    Generated { generator: String, parameters: HashMap<String, serde_json::Value> },
}

/// Security level classification for media assets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Display)]
pub enum SecurityLevel {
    #[display(fmt = "public")]
    Public,
    #[display(fmt = "internal")]
    Internal,
    #[display(fmt = "confidential")]
    Confidential,
    #[display(fmt = "restricted")]
    Restricted,
    #[display(fmt = "secret")]
    Secret,
    #[display(fmt = "top_secret")]
    TopSecret,
}

/// Media reference for lightweight linking to assets without full data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaReference {
    pub id: MediaAssetId,
    pub modality: Modality,
    pub metadata: MediaMetadata,
    pub thumbnail_url: Option<String>,
    pub version: String,
    pub created_at: DateTime<Utc>,
}

/// Collection of related media assets with metadata and management information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaCollection {
    pub id: MediaCollectionId,
    pub name: String,
    pub description: Option<String>,
    pub owner: Option<String>,
    pub tags: Vec<String>,
    pub assets: Vec<MediaAssetId>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Unique identifier for a media collection.
pub use crate::id::MediaCollectionId;

/// Media pipeline for processing assets through multiple steps.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaPipeline {
    pub id: MediaPipelineId,
    pub name: String,
    pub description: Option<String>,
    pub steps: Vec<ProcessingStep>,
    pub input_modality: Modality,
    pub output_modality: Modality,
    pub configuration: HashMap<String, serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub status: PipelineStatus,
}

/// Processing step in a media pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingStep {
    pub id: ProcessingStepId,
    pub name: String,
    pub processor: String,
    pub parameters: HashMap<String, serde_json::Value>,
    pub dependencies: Vec<ProcessingStepId>,
    pub output: Option<Modality>,
    pub order: u32,
    pub retry_count: u32,
}

/// Status of a media pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Display)]
pub enum PipelineStatus {
    #[display(fmt = "pending")]
    Pending,
    #[display(fmt = "running")]
    Running,
    #[display(fmt = "completed")]
    Completed,
    #[display(fmt = "failed")]
    Failed,
    #[display(fmt = "cancelled")]
    Cancelled,
    #[display(fmt = "paused")]
    Paused,
}

/// Unique identifier for a processing step.
pub use crate::id::ProcessingStepId;

/// Result of media processing operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaResult {
    pub success: bool,
    pub modality: Modality,
    pub format: MediaFormat,
    pub data: Option<serde_json::Value>,
    pub metadata: MediaMetadata,
    pub processing_time_ms: u64,
    pub confidence_score: f32,
    pub error: Option<String>,
    pub warnings: Vec<String>,
    pub created_at: DateTime<Utc>,
}

/// Statistics about media processing operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaStatistics {
    pub total_processed: u64,
    pub successful_processing: u64,
    pub failed_processing: u64,
    pub processing_time_ms: u64,
    pub average_confidence_score: f32,
    pub modality_breakdown: HashMap<Modality, ModalityStats>,
    pub format_breakdown: HashMap<String, u64>,
    pub error_breakdown: HashMap<String, u64>,
    pub storage_usage_mb: f64,
    pub bandwidth_used_mb: f64,
    pub generated_at: DateTime<Utc>,
}

/// Statistics for a specific modality.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModalityStats {
    pub processed: u64,
    pub successful: u64,
    pub average_confidence: f32,
    pub average_processing_time_ms: u64,
}

/// Metrics for media processing operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaMetrics {
    pub throughput_rpm: f32,
    pub latency_ms: f32,
    pub quality_score: f32,
    pub reliability: f32,
    pub utilization: f32,
    pub cost_per_operation: f64,
    pub storage_efficiency: f32,
    pub bandwidth_efficiency: f32,
}

/// Context for media processing operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaContext {
    pub session_id: MediaSessionId,
    pub user_id: Option<String>,
    pub workflow_id: Option<String>,
    pub priority: ProcessingPriority,
    pub constraints: HashMap<String, serde_json::Value>,
    pub environment: HashMap<String, serde_json::Value>,
    pub capabilities: Vec<String>,
    pub limits: ProcessingLimits,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Priority level for media processing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Display)]
pub enum ProcessingPriority {
    #[display(fmt = "low")]
    Low,
    #[display(fmt = "normal")]
    Normal,
    #[display(fmt = "high")]
    High,
    #[display(fmt = "critical")]
    Critical,
    #[display(fmt = "urgent")]
    Urgent,
}

/// Resource limits for processing operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingLimits {
    pub max_memory_mb: u32,
    pub max_cpu_percent: u32,
    pub max_storage_mb: u32,
    pub max_processing_time_ms: u64,
    pub max_bandwidth_mb: u32,
    pub max_concurrent_operations: u32,
}

impl Default for ProcessingLimits {
    fn default() -> Self {
        Self {
            max_memory_mb: 1024,
            max_cpu_percent: 80,
            max_storage_mb: 10240,
            max_processing_time_ms: 300000,
            max_bandwidth_mb: 100,
            max_concurrent_operations: 10,
        }
    }
}

/// Unique identifier for a processing session.
pub use crate::id::MediaSessionId;
