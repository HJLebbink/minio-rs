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

//! Builder for ListIndexes operation.

use crate::s3::error::ValidationErr;
use crate::s3vectors::client::VectorsClient;
use crate::s3vectors::error::VectorsValidationErr;
use crate::s3vectors::response::ListIndexesResponse;
use crate::s3vectors::types::{
    BucketName, ToVectorsRequest, VectorBucketArn, VectorsApi, VectorsRequest,
};
use typed_builder::TypedBuilder;

/// Builder for the [`ListIndexes`](https://docs.aws.amazon.com/AmazonS3/latest/API/API_S3VectorBuckets_ListIndexes.html) S3Vectors API operation.
///
/// Lists all vector indexes in a vector bucket.
#[derive(Clone, Debug, TypedBuilder)]
pub struct ListIndexes {
    #[builder(!default)]
    client: VectorsClient,

    /// The name of the vector bucket.
    #[builder(default, setter(into))]
    bucket: Option<BucketName>,

    /// The ARN of the vector bucket (alternative to name).
    #[builder(default, setter(into))]
    vector_bucket_arn: Option<VectorBucketArn>,

    /// Maximum number of indexes to return.
    #[builder(default, setter(into))]
    max_results: Option<u32>,

    /// Continuation token for pagination.
    #[builder(default, setter(into))]
    next_token: Option<String>,

    /// Filter indexes by name prefix.
    #[builder(default, setter(into))]
    prefix: Option<String>,
}

/// Builder type alias for [`ListIndexes`].
pub type ListIndexesBldr =
    ListIndexesBuilder<((VectorsClient,), (Option<BucketName>,), (), (), (), ())>;

impl VectorsApi for ListIndexes {
    type VectorsResponse = ListIndexesResponse;
}

impl ToVectorsRequest for ListIndexes {
    fn to_vectors_request(self) -> Result<VectorsRequest, ValidationErr> {
        let mut body = serde_json::json!({});

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

        if let Some(max) = self.max_results {
            body["maxResults"] = serde_json::Value::Number(max.into());
        }

        if let Some(token) = self.next_token {
            body["nextToken"] = serde_json::Value::String(token);
        }

        if let Some(prefix) = self.prefix {
            body["prefix"] = serde_json::Value::String(prefix);
        }

        Ok(VectorsRequest::builder()
            .client(self.client)
            .path("/ListIndexes".to_string())
            .body(Some(body))
            .build())
    }
}
