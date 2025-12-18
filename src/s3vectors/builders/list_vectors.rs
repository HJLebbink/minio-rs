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

//! Builder for ListVectors operation.

use crate::s3::error::ValidationErr;
use crate::s3vectors::client::VectorsClient;
use crate::s3vectors::error::VectorsValidationErr;
use crate::s3vectors::response::ListVectorsResponse;
use crate::s3vectors::types::{
    BucketName, IndexArn, IndexName, ToVectorsRequest, VectorsApi, VectorsRequest,
};
use typed_builder::TypedBuilder;

/// Builder for the [`ListVectors`](https://docs.aws.amazon.com/AmazonS3/latest/API/API_S3VectorBuckets_ListVectors.html) S3Vectors API operation.
///
/// Lists vectors in a vector index.
#[derive(Clone, Debug, TypedBuilder)]
pub struct ListVectors {
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

    /// Maximum number of vectors to return.
    #[builder(default, setter(into))]
    max_results: Option<u32>,

    /// Continuation token for pagination.
    #[builder(default, setter(into))]
    next_token: Option<String>,

    /// Whether to include vector data in the response.
    #[builder(default = false)]
    return_data: bool,

    /// Whether to include metadata in the response.
    #[builder(default = false)]
    return_metadata: bool,

    /// For parallel listing: total number of segments.
    #[builder(default, setter(into))]
    segment_count: Option<u32>,

    /// For parallel listing: index of this segment (0-based).
    #[builder(default, setter(into))]
    segment_index: Option<u32>,
}

/// Builder type alias for [`ListVectors`].
pub type ListVectorsBldr = ListVectorsBuilder<(
    (VectorsClient,),
    (Option<BucketName>,),
    (Option<IndexName>,),
    (),
    (),
    (),
    (),
    (),
    (),
    (),
)>;

impl VectorsApi for ListVectors {
    type VectorsResponse = ListVectorsResponse;
}

impl ToVectorsRequest for ListVectors {
    fn to_vectors_request(self) -> Result<VectorsRequest, ValidationErr> {
        let mut body = serde_json::json!({
            "returnData": self.return_data,
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

        if let Some(max) = self.max_results {
            body["maxResults"] = serde_json::Value::Number(max.into());
        }

        if let Some(token) = self.next_token {
            body["nextToken"] = serde_json::Value::String(token);
        }

        if let Some(count) = self.segment_count {
            body["segmentCount"] = serde_json::Value::Number(count.into());
        }

        if let Some(index) = self.segment_index {
            body["segmentIndex"] = serde_json::Value::Number(index.into());
        }

        Ok(VectorsRequest::builder()
            .client(self.client)
            .path("/ListVectors".to_string())
            .body(Some(body))
            .build())
    }
}
