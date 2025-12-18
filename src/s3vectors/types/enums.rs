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

//! Enumerations for S3Vectors API.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

use crate::s3vectors::error::VectorsValidationErr;

/// Data type for vector values.
///
/// AWS S3Vectors supports `Float32`. MinIO additionally supports `Float16` and `BFloat16`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum DataType {
    /// 32-bit floating point values (default).
    #[serde(rename = "float32")]
    #[default]
    Float32,

    /// 16-bit half-precision floating point values.
    ///
    /// **MinIO Extension**: Not supported by AWS S3Vectors.
    /// Provides 50% storage savings with some precision loss.
    #[serde(rename = "float16")]
    Float16,

    /// 16-bit brain floating point values.
    ///
    /// **MinIO Extension**: Not supported by AWS S3Vectors.
    /// ML-optimized format with same exponent range as float32.
    #[serde(rename = "bfloat16")]
    BFloat16,
}

impl DataType {
    /// Returns the string representation for API requests.
    pub fn as_str(&self) -> &'static str {
        match self {
            DataType::Float32 => "float32",
            DataType::Float16 => "float16",
            DataType::BFloat16 => "bfloat16",
        }
    }
}

impl fmt::Display for DataType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for DataType {
    type Err = VectorsValidationErr;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "float32" => Ok(DataType::Float32),
            "float16" => Ok(DataType::Float16),
            "bfloat16" => Ok(DataType::BFloat16),
            _ => Err(VectorsValidationErr::InvalidVectorDataType(s.to_string())),
        }
    }
}

/// Distance metric for similarity search.
///
/// Determines how distance between vectors is calculated during queries.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DistanceMetric {
    /// Euclidean (L2) distance - straight-line distance between vectors.
    #[serde(rename = "euclidean")]
    Euclidean,

    /// Cosine similarity - measures angle between vectors.
    /// Commonly used for text embeddings and semantic similarity.
    #[serde(rename = "cosine")]
    Cosine,
}

impl DistanceMetric {
    /// Returns the string representation for API requests.
    pub fn as_str(&self) -> &'static str {
        match self {
            DistanceMetric::Euclidean => "euclidean",
            DistanceMetric::Cosine => "cosine",
        }
    }
}

impl fmt::Display for DistanceMetric {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for DistanceMetric {
    type Err = VectorsValidationErr;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "euclidean" => Ok(DistanceMetric::Euclidean),
            "cosine" => Ok(DistanceMetric::Cosine),
            _ => Err(VectorsValidationErr::InvalidDistanceMetric(s.to_string())),
        }
    }
}

/// Server-side encryption type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SseType {
    /// Server-side encryption with Amazon S3 managed keys (SSE-S3).
    #[serde(rename = "AES256")]
    SseS3,

    /// Server-side encryption with AWS KMS managed keys (SSE-KMS).
    #[serde(rename = "aws:kms")]
    SseKms,
}

impl SseType {
    /// Returns the string representation for API requests.
    pub fn as_str(&self) -> &'static str {
        match self {
            SseType::SseS3 => "AES256",
            SseType::SseKms => "aws:kms",
        }
    }
}

impl fmt::Display for SseType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for SseType {
    type Err = VectorsValidationErr;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "AES256" | "SSE_S3" | "SSE-S3" => Ok(SseType::SseS3),
            "aws:kms" | "SSE_KMS" | "SSE-KMS" => Ok(SseType::SseKms),
            _ => Err(VectorsValidationErr::InvalidVectorSseType(s.to_string())),
        }
    }
}

/// Algorithm used for vector similarity search.
///
/// **MinIO Extension**: This is a MinIO-specific extension to the S3 Vectors API.
/// AWS S3 Vectors does NOT expose algorithm selection - it is managed internally.
/// MinIO AIStor supports multiple algorithms for different use cases.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum IndexAlgorithm {
    /// HNSW (Hierarchical Navigable Small World) - multi-layer graph for fast approximate search.
    /// Good balance of speed and recall.
    #[serde(rename = "hnsw")]
    Hnsw,

    /// Vamana - single-layer graph with diversity pruning.
    /// Focus on high-recall through diverse neighbor selection.
    #[serde(rename = "vamana")]
    Vamana,

    /// Brute-force exhaustive search - exact results with O(n) complexity.
    /// Best for small datasets or when exact results are required.
    /// This is the default algorithm.
    #[serde(rename = "bruteforce")]
    #[default]
    BruteForce,
}

impl IndexAlgorithm {
    /// Returns the string representation for API requests.
    pub fn as_str(&self) -> &'static str {
        match self {
            IndexAlgorithm::Hnsw => "hnsw",
            IndexAlgorithm::Vamana => "vamana",
            IndexAlgorithm::BruteForce => "bruteforce",
        }
    }
}

impl fmt::Display for IndexAlgorithm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for IndexAlgorithm {
    type Err = VectorsValidationErr;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "hnsw" => Ok(IndexAlgorithm::Hnsw),
            "vamana" => Ok(IndexAlgorithm::Vamana),
            "bruteforce" => Ok(IndexAlgorithm::BruteForce),
            _ => Err(VectorsValidationErr::InvalidIndexAlgorithm(s.to_string())),
        }
    }
}

/// Status of a vector index.
///
/// Indicates the current lifecycle state of the index.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum IndexStatus {
    /// Index is being created and is not yet ready for use.
    Creating,

    /// Index is active and ready for queries and vector operations.
    #[default]
    Active,

    /// Index is being deleted.
    Deleting,
}

impl IndexStatus {
    /// Returns the string representation for API requests.
    pub fn as_str(&self) -> &'static str {
        match self {
            IndexStatus::Creating => "CREATING",
            IndexStatus::Active => "ACTIVE",
            IndexStatus::Deleting => "DELETING",
        }
    }

    /// Returns true if the index is ready for queries.
    pub fn is_ready(&self) -> bool {
        matches!(self, IndexStatus::Active)
    }
}

impl fmt::Display for IndexStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for IndexStatus {
    type Err = VectorsValidationErr;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "CREATING" => Ok(IndexStatus::Creating),
            "ACTIVE" => Ok(IndexStatus::Active),
            "DELETING" => Ok(IndexStatus::Deleting),
            _ => Err(VectorsValidationErr::InvalidIndexStatus(s.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_type_serialization() {
        assert_eq!(DataType::Float32.as_str(), "float32");
        assert_eq!(DataType::Float32.to_string(), "float32");
        assert_eq!(DataType::Float16.as_str(), "float16");
        assert_eq!(DataType::Float16.to_string(), "float16");
        assert_eq!(DataType::BFloat16.as_str(), "bfloat16");
        assert_eq!(DataType::BFloat16.to_string(), "bfloat16");
    }

    #[test]
    fn test_data_type_parsing() {
        assert_eq!("float32".parse::<DataType>().unwrap(), DataType::Float32);
        assert_eq!("FLOAT32".parse::<DataType>().unwrap(), DataType::Float32);
        assert_eq!("float16".parse::<DataType>().unwrap(), DataType::Float16);
        assert_eq!("FLOAT16".parse::<DataType>().unwrap(), DataType::Float16);
        assert_eq!("bfloat16".parse::<DataType>().unwrap(), DataType::BFloat16);
        assert_eq!("BFLOAT16".parse::<DataType>().unwrap(), DataType::BFloat16);
        assert!("invalid".parse::<DataType>().is_err());
    }

    #[test]
    fn test_distance_metric_serialization() {
        assert_eq!(DistanceMetric::Euclidean.as_str(), "euclidean");
        assert_eq!(DistanceMetric::Cosine.as_str(), "cosine");
    }

    #[test]
    fn test_distance_metric_parsing() {
        assert_eq!(
            "euclidean".parse::<DistanceMetric>().unwrap(),
            DistanceMetric::Euclidean
        );
        assert_eq!(
            "cosine".parse::<DistanceMetric>().unwrap(),
            DistanceMetric::Cosine
        );
        assert_eq!(
            "COSINE".parse::<DistanceMetric>().unwrap(),
            DistanceMetric::Cosine
        );
        assert!("invalid".parse::<DistanceMetric>().is_err());
    }

    #[test]
    fn test_sse_type_serialization() {
        assert_eq!(SseType::SseS3.as_str(), "AES256");
        assert_eq!(SseType::SseKms.as_str(), "aws:kms");
    }

    #[test]
    fn test_sse_type_parsing() {
        assert_eq!("AES256".parse::<SseType>().unwrap(), SseType::SseS3);
        assert_eq!("SSE_S3".parse::<SseType>().unwrap(), SseType::SseS3);
        assert_eq!("aws:kms".parse::<SseType>().unwrap(), SseType::SseKms);
        assert!("invalid".parse::<SseType>().is_err());
    }

    #[test]
    fn test_index_algorithm_serialization() {
        assert_eq!(IndexAlgorithm::Hnsw.as_str(), "hnsw");
        assert_eq!(IndexAlgorithm::Vamana.as_str(), "vamana");
        assert_eq!(IndexAlgorithm::BruteForce.as_str(), "bruteforce");
    }

    #[test]
    fn test_index_algorithm_parsing() {
        assert_eq!(
            "hnsw".parse::<IndexAlgorithm>().unwrap(),
            IndexAlgorithm::Hnsw
        );
        assert_eq!(
            "vamana".parse::<IndexAlgorithm>().unwrap(),
            IndexAlgorithm::Vamana
        );
        assert_eq!(
            "bruteforce".parse::<IndexAlgorithm>().unwrap(),
            IndexAlgorithm::BruteForce
        );
        assert_eq!(
            "HNSW".parse::<IndexAlgorithm>().unwrap(),
            IndexAlgorithm::Hnsw
        );
        assert!("invalid".parse::<IndexAlgorithm>().is_err());
    }

    #[test]
    fn test_index_algorithm_default() {
        assert_eq!(IndexAlgorithm::default(), IndexAlgorithm::BruteForce);
    }
}
