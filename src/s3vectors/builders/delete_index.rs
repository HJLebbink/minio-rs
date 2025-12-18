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

//! Builder for DeleteIndex operation.

use crate::s3::error::ValidationErr;
use crate::s3vectors::client::VectorsClient;
use crate::s3vectors::error::VectorsValidationErr;
use crate::s3vectors::response::DeleteIndexResponse;
use crate::s3vectors::types::{
    BucketName, IndexArn, IndexName, ToVectorsRequest, VectorsApi, VectorsRequest,
};
use typed_builder::TypedBuilder;

/// Builder for the [`DeleteIndex`](https://docs.aws.amazon.com/AmazonS3/latest/API/API_S3VectorBuckets_DeleteIndex.html) S3Vectors API operation.
///
/// Deletes a vector index.
#[derive(Clone, Debug, TypedBuilder)]
pub struct DeleteIndex {
    #[builder(!default)]
    client: VectorsClient,

    /// The name of the vector bucket containing the index.
    #[builder(default, setter(into))]
    bucket: Option<BucketName>,

    /// The name of the index to delete.
    #[builder(default, setter(into))]
    index: Option<IndexName>,

    /// The ARN of the index (alternative to bucket name + index name).
    #[builder(default, setter(into))]
    index_arn: Option<IndexArn>,
}

/// Builder type alias for [`DeleteIndex`].
pub type DeleteIndexBldr = DeleteIndexBuilder<(
    (VectorsClient,),
    (Option<BucketName>,),
    (Option<IndexName>,),
    (),
)>;

impl VectorsApi for DeleteIndex {
    type VectorsResponse = DeleteIndexResponse;
}

impl ToVectorsRequest for DeleteIndex {
    fn to_vectors_request(self) -> Result<VectorsRequest, ValidationErr> {
        let mut body = serde_json::json!({});

        if let Some(arn) = self.index_arn {
            body["indexArn"] = serde_json::Value::String(arn.into_inner());
        } else if let (Some(bucket), Some(index)) = (self.bucket, self.index) {
            body["vectorBucketName"] = serde_json::Value::String(bucket.into_inner());
            body["indexName"] = serde_json::Value::String(index.into_inner());
        } else {
            return Err(VectorsValidationErr::MissingRequiredField(
                "indexArn or (bucket and index)".to_string(),
            )
            .into());
        }

        Ok(VectorsRequest::builder()
            .client(self.client)
            .path("/DeleteIndex".to_string())
            .body(Some(body))
            .build())
    }
}
