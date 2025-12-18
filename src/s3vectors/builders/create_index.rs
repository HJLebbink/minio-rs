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

//! Builder for CreateIndex operation.

use crate::s3::error::ValidationErr;
use crate::s3vectors::client::VectorsClient;
use crate::s3vectors::error::VectorsValidationErr;
use crate::s3vectors::response::CreateIndexResponse;
use crate::s3vectors::types::{
    BucketName, DataType, Dimension, DistanceMetric, EncryptionConfiguration, HnswConfig,
    IndexAlgorithm, IndexName, MetadataConfiguration, ToVectorsRequest, VamanaConfig,
    VectorBucketArn, VectorsApi, VectorsRequest,
};
use std::collections::HashMap;
use typed_builder::TypedBuilder;

/// Builder for the [`CreateIndex`](https://docs.aws.amazon.com/AmazonS3/latest/API/API_S3VectorBuckets_CreateIndex.html) S3Vectors API operation.
///
/// Creates a vector index within a vector bucket.
#[derive(Clone, Debug, TypedBuilder)]
pub struct CreateIndex {
    #[builder(!default)]
    client: VectorsClient,

    /// The name of the vector bucket containing the index.
    #[builder(default, setter(into))]
    bucket: Option<BucketName>,

    /// The ARN of the vector bucket (alternative to name).
    #[builder(default, setter(into))]
    vector_bucket_arn: Option<VectorBucketArn>,

    /// The name of the index to create (3-63 characters).
    #[builder(!default)]
    index: IndexName,

    /// The dimension of vectors (1-4096).
    #[builder(!default)]
    dimension: Dimension,

    /// The distance metric for similarity search.
    #[builder(!default)]
    distance_metric: DistanceMetric,

    /// The data type of vectors. Defaults to Float32.
    #[builder(default = DataType::Float32)]
    data_type: DataType,

    /// Optional encryption configuration.
    #[builder(default, setter(into))]
    encryption_configuration: Option<EncryptionConfiguration>,

    /// Optional metadata configuration (non-filterable keys).
    #[builder(default, setter(into))]
    metadata_configuration: Option<MetadataConfiguration>,

    /// Optional algorithm selection.
    ///
    /// **MinIO Extension**: This is a MinIO-specific extension to the S3 Vectors API.
    /// AWS S3 Vectors does NOT expose algorithm selection.
    ///
    /// Available algorithms:
    /// - `BruteForce` (default): Exact exhaustive search, O(n) complexity
    /// - `Hnsw`: Hierarchical Navigable Small World graph, fast approximate search
    /// - `Vamana`: Single-layer graph with diversity pruning, high-recall approximate search
    #[builder(default, setter(into))]
    algorithm: Option<IndexAlgorithm>,

    /// Optional HNSW algorithm configuration.
    ///
    /// **MinIO Extension**: Only used when `algorithm` is `Hnsw`.
    /// Controls the trade-off between index quality, search accuracy, and performance.
    /// If not specified, server defaults are used.
    #[builder(default, setter(into))]
    hnsw_config: Option<HnswConfig>,

    /// Optional Vamana algorithm configuration.
    ///
    /// **MinIO Extension**: Only used when `algorithm` is `Vamana`.
    /// Controls the trade-off between index quality, search accuracy, and performance.
    /// If not specified, server defaults are used.
    #[builder(default, setter(into))]
    vamana_config: Option<VamanaConfig>,

    /// Optional tags for the index.
    #[builder(default, setter(into))]
    tags: Option<HashMap<String, String>>,
}

/// Builder type alias for [`CreateIndex`].
pub type CreateIndexBldr = CreateIndexBuilder<(
    (VectorsClient,),      // client
    (Option<BucketName>,), // bucket
    (),                    // vector_bucket_arn
    (IndexName,),          // index
    (Dimension,),          // dimension
    (DistanceMetric,),     // distance_metric
    (),                    // data_type
    (),                    // encryption_configuration
    (),                    // metadata_configuration
    (),                    // algorithm
    (),                    // hnsw_config
    (),                    // vamana_config
    (),                    // tags
)>;

impl VectorsApi for CreateIndex {
    type VectorsResponse = CreateIndexResponse;
}

impl ToVectorsRequest for CreateIndex {
    fn to_vectors_request(self) -> Result<VectorsRequest, ValidationErr> {
        let mut body = serde_json::json!({
            "indexName": self.index.as_str(),
            "dimension": self.dimension.as_u32(),
            "distanceMetric": self.distance_metric.as_str(),
            "dataType": self.data_type.as_str(),
        });

        if let Some(name) = self.bucket {
            body["vectorBucketName"] = serde_json::Value::String(name.into_inner());
        } else if let Some(arn) = self.vector_bucket_arn {
            body["vectorBucketArn"] = serde_json::Value::String(arn.into_inner());
        } else {
            return Err(VectorsValidationErr::MissingRequiredField(
                "vectorBucketName or vectorBucketArn".to_string(),
            )
            .into());
        }

        if let Some(enc) = self.encryption_configuration {
            body["encryptionConfiguration"] = serde_json::to_value(enc)?;
        }

        if let Some(meta) = self.metadata_configuration {
            body["metadataConfiguration"] = serde_json::to_value(meta)?;
        }

        if let Some(algo) = self.algorithm {
            body["algorithm"] = serde_json::Value::String(algo.as_str().to_string());
        }

        if let Some(hnsw) = self.hnsw_config {
            body["hnswConfiguration"] = serde_json::to_value(hnsw)?;
        }

        if let Some(vamana) = self.vamana_config {
            body["vamanaConfiguration"] = serde_json::to_value(vamana)?;
        }

        if let Some(tags) = self.tags
            && !tags.is_empty()
        {
            body["tags"] = serde_json::to_value(tags)?;
        }

        Ok(VectorsRequest::builder()
            .client(self.client)
            .path("/CreateIndex".to_string())
            .body(Some(body))
            .build())
    }
}
