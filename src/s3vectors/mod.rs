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

//! S3Vectors API implementation for Amazon S3 Vector storage.
//!
//! This module provides support for [AWS S3Vectors](https://docs.aws.amazon.com/AmazonS3/latest/API/API_Operations_Amazon_S3_Vectors.html),
//! a purpose-built vector storage service for AI/ML workloads that enables
//! storing and querying vector embeddings with sub-second search times.
//!
//! S3Vectors uses a separate `s3vectors` service namespace from standard S3.
//!
//! # Key Components
//!
//! - **Vector Buckets**: Container for vector indexes
//! - **Vector Indexes**: Organized collections of vectors with defined dimensions and distance metrics
//! - **Vectors**: Individual vector embeddings with optional metadata
//!
//! # Example
//!
//! ```no_run
//! use minio::s3vectors::{VectorsClient, DistanceMetric, VectorData, VectorsApi};
//! use minio::s3::creds::StaticProvider;
//! use minio::s3::http::BaseUrl;
//! use minio::s3::types::Region;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create client (uses same credentials as S3)
//! let base_url = "http://localhost:9000".parse::<BaseUrl>()?;
//! let provider = StaticProvider::new("minioadmin", "minioadmin", None);
//! let region = Region::new("us-east-1")?;
//! let client = VectorsClient::new(base_url, Some(provider), Some(region))?;
//!
//! // Create a vector bucket
//! client.create_vector_bucket("my-vectors")?.build().send().await?;
//!
//! // Create an index with 1024 dimensions using cosine similarity
//! client.create_index("my-vectors", "embeddings", 1024, DistanceMetric::Cosine)?.build().send().await?;
//!
//! // Query for similar vectors
//! let query_embedding = vec![0.1_f32; 1024];
//! let results = client.query_vectors(
//!     "my-vectors",
//!     "embeddings",
//!     VectorData { float32: query_embedding },
//!     10,
//! )?.build().send().await?;
//! # Ok(())
//! # }
//! ```

pub mod builders;
pub mod client;
pub mod error;
pub mod response;
#[macro_use]
pub mod response_traits;
pub mod types;

// Re-export commonly used types
pub use client::VectorsClient;
pub use error::VectorsValidationErr;
pub use types::{
    BucketName, DataType, Dimension, DistanceMetric, EncryptionConfiguration, HnswConfig,
    IndexAlgorithm, IndexName, IndexStatus, IndexSummary, MetadataConfiguration, QueryOutputVector,
    SseType, TopK, VamanaConfig, Vector, VectorBucketSummary, VectorData, VectorKey, VectorsApi,
};
