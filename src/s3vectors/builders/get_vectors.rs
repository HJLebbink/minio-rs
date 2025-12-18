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

//! Builder for GetVectors operation.

use crate::s3::error::ValidationErr;
use crate::s3vectors::client::VectorsClient;
use crate::s3vectors::error::VectorsValidationErr;
use crate::s3vectors::response::GetVectorsResponse;
use crate::s3vectors::types::{
    BucketName, IndexArn, IndexName, ToVectorsRequest, VectorKey, VectorsApi, VectorsRequest,
};
use typed_builder::TypedBuilder;

/// Builder for the [`GetVectors`](https://docs.aws.amazon.com/AmazonS3/latest/API/API_S3VectorBuckets_GetVectors.html) S3Vectors API operation.
///
/// Retrieves vectors by their keys.
#[derive(Clone, Debug, TypedBuilder)]
pub struct GetVectors {
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

    /// The keys of vectors to retrieve.
    #[builder(!default)]
    keys: Vec<VectorKey>,

    /// Whether to include vector data in the response.
    #[builder(default = false)]
    return_data: bool,

    /// Whether to include metadata in the response.
    #[builder(default = false)]
    return_metadata: bool,
}

/// Builder type alias for [`GetVectors`].
pub type GetVectorsBldr = GetVectorsBuilder<(
    (VectorsClient,),
    (Option<BucketName>,),
    (Option<IndexName>,),
    (),
    (Vec<VectorKey>,),
    (),
    (),
)>;

impl VectorsApi for GetVectors {
    type VectorsResponse = GetVectorsResponse;
}

impl ToVectorsRequest for GetVectors {
    fn to_vectors_request(self) -> Result<VectorsRequest, ValidationErr> {
        if self.keys.is_empty() {
            return Err(VectorsValidationErr::MissingRequiredField("keys".to_string()).into());
        }

        let key_strings: Vec<String> = self.keys.into_iter().map(|k| k.into_inner()).collect();

        let mut body = serde_json::json!({
            "keys": key_strings,
            "returnData": self.return_data,
            "returnMetadata": self.return_metadata,
        });

        if let Some(arn) = self.index_arn {
            body["indexArn"] = serde_json::Value::String(arn.into_inner());
        } else if let (Some(bucket), Some(idx)) = (self.bucket, self.index) {
            body["vectorBucketName"] = serde_json::Value::String(bucket.into_inner());
            body["indexName"] = serde_json::Value::String(idx.into_inner());
        } else {
            return Err(VectorsValidationErr::MissingRequiredField(
                "indexArn or (vectorBucketName and indexName)".to_string(),
            )
            .into());
        }

        Ok(VectorsRequest::builder()
            .client(self.client)
            .path("/GetVectors".to_string())
            .body(Some(body))
            .build())
    }
}
