// MinIO Rust Library for Amazon S3 Compatible Cloud Storage
// Copyright 2025 MinIO, Inc.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Configuration types for S3Vectors API.

use serde::{Deserialize, Serialize};
use typed_builder::TypedBuilder;

use super::SseType;

/// Server-side encryption configuration for vector buckets and indexes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EncryptionConfiguration {
    /// The type of server-side encryption.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sse_type: Option<SseType>,

    /// The ARN of the KMS key to use for SSE-KMS encryption.
    /// Required when `sse_type` is `SseKms`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kms_key_arn: Option<String>,
}

impl EncryptionConfiguration {
    /// Creates a new encryption configuration using SSE-S3 (AES256).
    pub fn sse_s3() -> Self {
        Self {
            sse_type: Some(SseType::SseS3),
            kms_key_arn: None,
        }
    }

    /// Creates a new encryption configuration using SSE-KMS with the specified key ARN.
    pub fn sse_kms(kms_key_arn: impl Into<String>) -> Self {
        Self {
            sse_type: Some(SseType::SseKms),
            kms_key_arn: Some(kms_key_arn.into()),
        }
    }
}

impl Default for EncryptionConfiguration {
    fn default() -> Self {
        Self::sse_s3()
    }
}

/// Metadata configuration for vector indexes.
///
/// Defines which metadata keys should be non-filterable.
/// Non-filterable metadata keys can be stored and retrieved but
/// cannot be used in query filters.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataConfiguration {
    /// List of metadata keys that should not be filterable.
    /// These keys can be retrieved but cannot be used in query filters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub non_filterable_metadata_keys: Option<Vec<String>>,
}

impl MetadataConfiguration {
    /// Creates a new metadata configuration with the specified non-filterable keys.
    pub fn new(non_filterable_keys: Vec<String>) -> Self {
        Self {
            non_filterable_metadata_keys: if non_filterable_keys.is_empty() {
                None
            } else {
                Some(non_filterable_keys)
            },
        }
    }

    /// Adds a non-filterable metadata key.
    pub fn add_non_filterable_key(&mut self, key: impl Into<String>) {
        let keys = self
            .non_filterable_metadata_keys
            .get_or_insert_with(Vec::new);
        keys.push(key.into());
    }

    /// Returns the list of non-filterable metadata keys.
    pub fn non_filterable_keys(&self) -> &[String] {
        self.non_filterable_metadata_keys.as_deref().unwrap_or(&[])
    }
}

/// HNSW (Hierarchical Navigable Small World) algorithm configuration.
///
/// **MinIO Extension**: This is a MinIO-specific extension to the S3 Vectors API.
/// AWS S3 Vectors does NOT support user-configurable HNSW parameters - they are
/// managed internally by AWS. MinIO AIStor supports these parameters for fine-tuning
/// index performance.
///
/// # Parameters
///
/// - **`m`**: Number of bi-directional links per node (default: 16, range: 2-100).
///   Higher values create a denser graph with better recall but more memory usage.
///
/// - **`ef_construction`**: Search depth during index building (default: 100, range: 1-1000).
///   Higher values improve index quality but increase build time.
///
/// - **`ef_search`**: Default search depth during queries (default: varies, range: 1-1000).
///   Higher values improve recall but increase query latency.
///   Should be >= top_k for best results. Can be overridden per-query.
///
/// # Example
///
/// ```
/// use minio::s3vectors::HnswConfig;
///
/// // High-quality index with fast queries
/// let config = HnswConfig::builder()
///     .m(32)
///     .ef_construction(200)
///     .ef_search(100)
///     .build();
///
/// // Memory-efficient index
/// let compact = HnswConfig::builder()
///     .m(8)
///     .ef_construction(64)
///     .build();
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TypedBuilder)]
#[serde(rename_all = "camelCase")]
pub struct HnswConfig {
    /// Number of bi-directional links per node in the HNSW graph.
    ///
    /// Higher values create a denser graph with better recall but more memory.
    /// Typical values: 8-64. Default: 16.
    #[builder(default, setter(strip_option))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub m: Option<u32>,

    /// Search depth during index construction.
    ///
    /// Higher values improve index quality but increase build time.
    /// Should be >= m for good results. Typical values: 64-512. Default: 100.
    #[builder(default, setter(strip_option))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ef_construction: Option<u32>,

    /// Search depth during query execution.
    ///
    /// Higher values improve recall but increase query latency.
    /// Should be >= top_k. Typical values: 50-500. Default: varies.
    #[builder(default, setter(strip_option))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ef_search: Option<u32>,
}

impl HnswConfig {
    /// Creates a new HNSW configuration with all parameters specified.
    pub fn new(m: u32, ef_construction: u32, ef_search: u32) -> Self {
        Self {
            m: Some(m),
            ef_construction: Some(ef_construction),
            ef_search: Some(ef_search),
        }
    }

    /// Creates a configuration optimized for high recall.
    ///
    /// Uses higher M and ef values for better accuracy at the cost of
    /// more memory and slower queries.
    pub fn high_recall() -> Self {
        Self {
            m: Some(32),
            ef_construction: Some(256),
            ef_search: Some(200),
        }
    }

    /// Creates a configuration optimized for fast queries.
    ///
    /// Uses lower ef_search for faster queries, may sacrifice some recall.
    pub fn fast_search() -> Self {
        Self {
            m: Some(16),
            ef_construction: Some(128),
            ef_search: Some(50),
        }
    }

    /// Creates a memory-efficient configuration.
    ///
    /// Uses lower M value to reduce memory footprint.
    pub fn memory_efficient() -> Self {
        Self {
            m: Some(8),
            ef_construction: Some(64),
            ef_search: Some(100),
        }
    }

    /// Returns the M parameter if set.
    pub fn m(&self) -> Option<u32> {
        self.m
    }

    /// Returns the ef_construction parameter if set.
    pub fn ef_construction(&self) -> Option<u32> {
        self.ef_construction
    }

    /// Returns the ef_search parameter if set.
    pub fn ef_search(&self) -> Option<u32> {
        self.ef_search
    }
}

/// Vamana algorithm configuration.
///
/// **MinIO Extension**: This is a MinIO-specific extension to the S3 Vectors API.
/// AWS S3 Vectors does NOT support Vamana or user-configurable algorithm parameters.
/// MinIO AIStor supports these parameters for fine-tuning index performance.
///
/// # Parameters
///
/// - **`l`**: Search queue size during building and queries (default: 50, range: 10-500).
///   Higher values improve recall but increase latency.
///
/// - **`r`**: Maximum number of neighbors per node (default: 32, range: 10-100).
///   Higher values create a denser graph with better recall.
///
/// - **`alpha`**: Diversity factor for neighbor pruning (default: 1.2, range: 1.0-2.0).
///   Higher values favor more diverse neighbors.
///
/// # Example
///
/// ```
/// use minio::s3vectors::VamanaConfig;
///
/// // High-recall configuration
/// let config = VamanaConfig::builder()
///     .l(100)
///     .r(64)
///     .alpha(1.4)
///     .build();
///
/// // Balanced configuration (uses defaults)
/// let balanced = VamanaConfig::default();
/// ```
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, TypedBuilder)]
#[serde(rename_all = "camelCase")]
pub struct VamanaConfig {
    /// Search queue size during building and queries.
    ///
    /// Higher values improve recall but increase latency.
    /// Typical values: 50-200. Default: 50.
    #[builder(default, setter(strip_option))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub l: Option<u32>,

    /// Maximum number of neighbors per node.
    ///
    /// Higher values create a denser graph with better recall but more memory.
    /// Typical values: 32-64. Default: 32.
    #[builder(default, setter(strip_option))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r: Option<u32>,

    /// Diversity factor for neighbor pruning.
    ///
    /// Higher values favor more diverse neighbors over closer ones.
    /// Typical values: 1.0-2.0. Default: 1.2.
    #[builder(default, setter(strip_option))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alpha: Option<f32>,
}

impl VamanaConfig {
    /// Creates a new Vamana configuration with all parameters specified.
    pub fn new(l: u32, r: u32, alpha: f32) -> Self {
        Self {
            l: Some(l),
            r: Some(r),
            alpha: Some(alpha),
        }
    }

    /// Creates a configuration optimized for high recall.
    ///
    /// Uses higher L and R values for better accuracy at the cost of
    /// more memory and slower queries.
    pub fn high_recall() -> Self {
        Self {
            l: Some(100),
            r: Some(64),
            alpha: Some(1.4),
        }
    }

    /// Creates a configuration optimized for fast queries.
    ///
    /// Uses lower L value for faster queries, may sacrifice some recall.
    pub fn fast_search() -> Self {
        Self {
            l: Some(30),
            r: Some(32),
            alpha: Some(1.2),
        }
    }

    /// Returns the L parameter if set.
    pub fn l(&self) -> Option<u32> {
        self.l
    }

    /// Returns the R parameter if set.
    pub fn r(&self) -> Option<u32> {
        self.r
    }

    /// Returns the alpha parameter if set.
    pub fn alpha(&self) -> Option<f32> {
        self.alpha
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encryption_configuration_sse_s3() {
        let config = EncryptionConfiguration::sse_s3();
        assert_eq!(config.sse_type, Some(SseType::SseS3));
        assert!(config.kms_key_arn.is_none());
    }

    #[test]
    fn test_encryption_configuration_sse_kms() {
        let config = EncryptionConfiguration::sse_kms(
            "arn:aws:kms:us-east-1:123456789012:key/12345678-1234-1234-1234-123456789012",
        );
        assert_eq!(config.sse_type, Some(SseType::SseKms));
        assert!(config.kms_key_arn.is_some());
    }

    #[test]
    fn test_encryption_configuration_serialization() {
        let config = EncryptionConfiguration::sse_s3();
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("AES256") || json.contains("sseType"));
    }

    #[test]
    fn test_metadata_configuration() {
        let mut config = MetadataConfiguration::default();
        config.add_non_filterable_key("source_text");
        config.add_non_filterable_key("raw_content");

        assert_eq!(config.non_filterable_keys().len(), 2);
        assert!(
            config
                .non_filterable_keys()
                .contains(&"source_text".to_string())
        );
    }

    #[test]
    fn test_metadata_configuration_serialization() {
        let config = MetadataConfiguration::new(vec!["key1".to_string(), "key2".to_string()]);
        let json = serde_json::to_string(&config).unwrap();
        let parsed: MetadataConfiguration = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.non_filterable_keys().len(), 2);
    }

    #[test]
    fn test_vamana_config_new() {
        let config = VamanaConfig::new(100, 64, 1.4);
        assert_eq!(config.l(), Some(100));
        assert_eq!(config.r(), Some(64));
        assert_eq!(config.alpha(), Some(1.4));
    }

    #[test]
    fn test_vamana_config_presets() {
        let high_recall = VamanaConfig::high_recall();
        assert_eq!(high_recall.l(), Some(100));
        assert_eq!(high_recall.r(), Some(64));
        assert_eq!(high_recall.alpha(), Some(1.4));

        let fast = VamanaConfig::fast_search();
        assert_eq!(fast.l(), Some(30));
        assert_eq!(fast.r(), Some(32));
    }

    #[test]
    fn test_vamana_config_builder() {
        let config = VamanaConfig::builder().l(75).r(48).alpha(1.3).build();
        assert_eq!(config.l(), Some(75));
        assert_eq!(config.r(), Some(48));
        assert_eq!(config.alpha(), Some(1.3));
    }

    #[test]
    fn test_vamana_config_serialization() {
        let config = VamanaConfig::new(50, 32, 1.2);
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("\"l\":50"));
        assert!(json.contains("\"r\":32"));
        assert!(json.contains("\"alpha\":1.2"));

        let parsed: VamanaConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.l(), Some(50));
        assert_eq!(parsed.r(), Some(32));
        assert_eq!(parsed.alpha(), Some(1.2));
    }
}
