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

//! Builder for QueryVectors operation.

use crate::s3::error::ValidationErr;
use crate::s3::multimap_ext::Multimap;
use crate::s3vectors::client::VectorsClient;
use crate::s3vectors::error::VectorsValidationErr;
use crate::s3vectors::response::QueryVectorsResponse;
use crate::s3vectors::types::{
    BucketName, IndexArn, IndexName, ToVectorsRequest, TopK, VectorData, VectorsApi, VectorsRequest,
};
use typed_builder::TypedBuilder;

/// Builder for the [`QueryVectors`](https://docs.aws.amazon.com/AmazonS3/latest/API/API_S3VectorBuckets_QueryVectors.html) S3Vectors API operation.
///
/// Performs an approximate nearest neighbor search in a vector index.
#[derive(Clone, Debug, TypedBuilder)]
pub struct QueryVectors {
    #[builder(!default)]
    client: VectorsClient,

    /// The name of the vector bucket containing the index.
    #[builder(default, setter(into))]
    bucket: Option<BucketName>,

    /// The name of the index.
    #[builder(default, setter(into))]
    index: Option<IndexName>,

    /// The ARN of the index (alternative to bucket name + index name).
    #[builder(default, setter(into))]
    index_arn: Option<IndexArn>,

    /// The query vector (must match index dimensions).
    #[builder(!default)]
    query_vector: VectorData,

    /// Number of results to return (top K nearest neighbors).
    #[builder(!default)]
    top_k: TopK,

    /// Optional metadata filter for the query.
    #[builder(default, setter(into))]
    filter: Option<serde_json::Value>,

    /// Whether to include distance values in the response.
    #[builder(default = false)]
    return_distance: bool,

    /// Whether to include metadata in the response.
    #[builder(default = false)]
    return_metadata: bool,

    /// Optional ef_search parameter for query-time recall tuning.
    ///
    /// **NOTE**: As of 2025, the AWS S3 Vectors API does NOT support this parameter.
    /// This field is provided for alternative implementations (like MinIO) that
    /// may support query-time ef_search tuning.
    ///
    /// Search depth during query execution. Higher values improve recall
    /// but increase query latency. Should be >= top_k for best results.
    /// If not specified, uses the index's default ef_search value.
    #[builder(default, setter(strip_option))]
    ef_search: Option<u32>,

    /// Use brute-force search instead of approximate nearest neighbor search.
    ///
    /// **MinIO Extension**: This is a MinIO-specific extension to the S3 Vectors API.
    /// AWS S3 Vectors does NOT support this parameter.
    ///
    /// When enabled, performs an exhaustive search over all vectors in the index,
    /// returning exact (not approximate) nearest neighbors. This is useful for:
    /// - Validating HNSW recall by comparing against ground truth
    /// - Small datasets where exact search is acceptable
    /// - Debugging and testing
    ///
    /// Note: Brute-force search is O(n) and can be slow for large indexes.
    #[builder(default = false)]
    brute_force: bool,

    /// CPU architecture override for benchmarking SIMD implementations.
    ///
    /// **MinIO Extension**: This is a MinIO-specific extension for benchmarking.
    /// AWS S3 Vectors does NOT support this parameter.
    ///
    /// Valid values: "scalar", "avx2", "avx512", "avx512fp16", "avx512bf16"
    ///
    /// When set, forces the server to use the specified CPU architecture for
    /// distance computations, allowing benchmarking of different SIMD implementations.
    #[builder(default, setter(into, strip_option))]
    cpu_arch: Option<String>,
}

/// Builder type alias for [`QueryVectors`].
pub type QueryVectorsBldr = QueryVectorsBuilder<(
    (VectorsClient,),      // client
    (Option<BucketName>,), // bucket
    (Option<IndexName>,),  // index
    (),                    // index_arn
    (VectorData,),         // query_vector
    (TopK,),               // top_k
    (),                    // filter
    (),                    // return_distance
    (),                    // return_metadata
    (),                    // ef_search
    (),                    // brute_force
    (),                    // cpu_arch
)>;

impl VectorsApi for QueryVectors {
    type VectorsResponse = QueryVectorsResponse;
}

impl ToVectorsRequest for QueryVectors {
    fn to_vectors_request(self) -> Result<VectorsRequest, ValidationErr> {
        let mut body = serde_json::json!({
            "queryVector": self.query_vector,
            "topK": self.top_k.as_u32(),
            "returnDistance": self.return_distance,
            "returnMetadata": self.return_metadata,
        });

        if let Some(arn) = self.index_arn {
            body["indexArn"] = serde_json::Value::String(arn.into_inner());
        } else if let (Some(bucket), Some(index)) = (self.bucket, self.index) {
            body["vectorBucketName"] = serde_json::Value::String(bucket.into_inner());
            body["indexName"] = serde_json::Value::String(index.into_inner());
        } else {
            return Err(VectorsValidationErr::MissingRequiredField(
                "indexArn or (vectorBucketName and indexName)".to_string(),
            )
            .into());
        }

        if let Some(filter) = self.filter {
            body["filter"] = filter;
        }

        if let Some(ef) = self.ef_search {
            body["efSearch"] = serde_json::Value::Number(ef.into());
        }

        if self.brute_force {
            body["bruteForce"] = serde_json::Value::Bool(true);
        }

        // Build extra headers for MinIO extensions
        let extra_headers = if let Some(ref arch) = self.cpu_arch {
            let mut headers = Multimap::new();
            headers.insert("X-Minio-CPU-Arch".to_string(), arch.clone());
            Some(headers)
        } else {
            None
        };

        Ok(VectorsRequest::builder()
            .client(self.client)
            .path("/QueryVectors".to_string())
            .body(Some(body))
            .extra_headers(extra_headers)
            .build())
    }
}
