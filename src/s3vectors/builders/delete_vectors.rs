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

//! Builder for DeleteVectors operation.

use crate::s3::error::ValidationErr;
use crate::s3vectors::client::VectorsClient;
use crate::s3vectors::error::VectorsValidationErr;
use crate::s3vectors::response::DeleteVectorsResponse;
use crate::s3vectors::types::{
    BucketName, IndexArn, IndexName, ToVectorsRequest, VectorKey, VectorsApi, VectorsRequest,
};
use typed_builder::TypedBuilder;

/// Builder for the [`DeleteVectors`](https://docs.aws.amazon.com/AmazonS3/latest/API/API_S3VectorBuckets_DeleteVectors.html) S3Vectors API operation.
///
/// Deletes vectors by their keys.
#[derive(Clone, Debug, TypedBuilder)]
pub struct DeleteVectors {
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

    /// The keys of vectors to delete.
    #[builder(!default)]
    keys: Vec<VectorKey>,
}

/// Builder type alias for [`DeleteVectors`].
pub type DeleteVectorsBldr = DeleteVectorsBuilder<(
    (VectorsClient,),
    (Option<BucketName>,),
    (Option<IndexName>,),
    (),
    (Vec<VectorKey>,),
)>;

impl VectorsApi for DeleteVectors {
    type VectorsResponse = DeleteVectorsResponse;
}

impl ToVectorsRequest for DeleteVectors {
    fn to_vectors_request(self) -> Result<VectorsRequest, ValidationErr> {
        if self.keys.is_empty() {
            return Err(VectorsValidationErr::MissingRequiredField("keys".to_string()).into());
        }

        let key_strings: Vec<String> = self.keys.into_iter().map(|k| k.into_inner()).collect();

        let mut body = serde_json::json!({
            "keys": key_strings,
        });

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
            .path("/DeleteVectors".to_string())
            .body(Some(body))
            .build())
    }
}
