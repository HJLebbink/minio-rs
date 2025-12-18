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

//! Builder for PutVectors operation.

use crate::s3::error::ValidationErr;
use crate::s3vectors::client::VectorsClient;
use crate::s3vectors::error::VectorsValidationErr;
use crate::s3vectors::response::PutVectorsResponse;
use crate::s3vectors::types::{
    BucketName, IndexArn, IndexName, ToVectorsRequest, Vector, VectorsApi, VectorsRequest,
};
use typed_builder::TypedBuilder;

/// Maximum number of vectors per request.
pub const MAX_VECTORS_PER_REQUEST: usize = 500;

/// Builder for the [`PutVectors`](https://docs.aws.amazon.com/AmazonS3/latest/API/API_S3VectorBuckets_PutVectors.html) S3Vectors API operation.
///
/// Adds one or more vectors to a vector index.
#[derive(Clone, Debug, TypedBuilder)]
pub struct PutVectors {
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

    /// The vectors to insert (1-500 vectors).
    #[builder(!default)]
    vectors: Vec<Vector>,
}

/// Builder type alias for [`PutVectors`].
pub type PutVectorsBldr = PutVectorsBuilder<(
    (VectorsClient,),
    (Option<BucketName>,),
    (Option<IndexName>,),
    (),
    (Vec<Vector>,),
)>;

impl VectorsApi for PutVectors {
    type VectorsResponse = PutVectorsResponse;
}

impl ToVectorsRequest for PutVectors {
    fn to_vectors_request(self) -> Result<VectorsRequest, ValidationErr> {
        // Validate vector count
        if self.vectors.is_empty() {
            return Err(VectorsValidationErr::MissingRequiredField("vectors".to_string()).into());
        }

        if self.vectors.len() > MAX_VECTORS_PER_REQUEST {
            return Err(VectorsValidationErr::TooManyVectors {
                count: self.vectors.len(),
                max: MAX_VECTORS_PER_REQUEST,
            }
            .into());
        }

        let mut body = serde_json::json!({
            "vectors": self.vectors,
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

        Ok(VectorsRequest::builder()
            .client(self.client)
            .path("/PutVectors".to_string())
            .body(Some(body))
            .build())
    }
}
