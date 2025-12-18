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

//! S3Vectors-specific validation errors.

use thiserror::Error;

/// Validation errors specific to S3Vectors operations.
#[derive(Error, Debug)]
pub enum VectorsValidationErr {
    #[error("Invalid index name: '{name}' - {reason}")]
    InvalidIndexName { name: String, reason: String },

    #[error("Invalid vector key: '{key}' - {reason}")]
    InvalidVectorKey { key: String, reason: String },

    #[error("Invalid dimension {dimension}: {reason}")]
    InvalidDimension { dimension: u32, reason: String },

    #[error("Invalid vector value at index {index}: {reason}")]
    InvalidVectorValue { index: usize, reason: String },

    #[error("Zero vectors are not allowed when using cosine distance metric")]
    ZeroVectorNotAllowed,

    #[error("Vector dimension mismatch: expected {expected}, got {actual}")]
    VectorDimensionMismatch { expected: u32, actual: u32 },

    #[error("Too many vectors: {count} exceeds maximum of {max}")]
    TooManyVectors { count: usize, max: usize },

    #[error("Invalid top_k value: {0}; must be at least 1")]
    InvalidTopK(u32),

    #[error("Invalid vector data type: '{0}'")]
    InvalidVectorDataType(String),

    #[error("Invalid distance metric: '{0}'")]
    InvalidDistanceMetric(String),

    #[error("Invalid vector SSE type: '{0}'")]
    InvalidVectorSseType(String),

    #[error("Invalid index status: '{0}'")]
    InvalidIndexStatus(String),

    #[error("Invalid index algorithm: '{0}'")]
    InvalidIndexAlgorithm(String),

    #[error("Missing required field: {0}")]
    MissingRequiredField(String),
}
