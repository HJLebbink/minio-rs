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

//! Builder for CreateVectorBucket operation.

use crate::s3::error::ValidationErr;
use crate::s3vectors::client::VectorsClient;
use crate::s3vectors::response::CreateVectorBucketResponse;
use crate::s3vectors::types::{
    BucketName, EncryptionConfiguration, ToVectorsRequest, VectorsApi, VectorsRequest,
};
use std::collections::HashMap;
use typed_builder::TypedBuilder;

/// Builder for the [`CreateVectorBucket`](https://docs.aws.amazon.com/AmazonS3/latest/API/API_S3VectorBuckets_CreateVectorBucket.html) S3Vectors API operation.
///
/// Creates a vector bucket in the specified AWS Region.
#[derive(Clone, Debug, TypedBuilder)]
pub struct CreateVectorBucket {
    #[builder(!default)]
    client: VectorsClient,

    /// The name of the vector bucket to create (3-63 characters).
    #[builder(!default)]
    bucket: BucketName,

    /// Optional encryption configuration. Defaults to SSE-S3 (AES256).
    #[builder(default, setter(into))]
    encryption_configuration: Option<EncryptionConfiguration>,

    /// Optional tags for the vector bucket.
    #[builder(default, setter(into))]
    tags: Option<HashMap<String, String>>,
}

/// Builder type alias for [`CreateVectorBucket`].
pub type CreateVectorBucketBldr =
    CreateVectorBucketBuilder<((VectorsClient,), (BucketName,), (), ())>;

impl VectorsApi for CreateVectorBucket {
    type VectorsResponse = CreateVectorBucketResponse;
}

impl ToVectorsRequest for CreateVectorBucket {
    fn to_vectors_request(self) -> Result<VectorsRequest, ValidationErr> {
        let mut body = serde_json::json!({
            "vectorBucketName": self.bucket.as_str(),
        });

        if let Some(enc) = self.encryption_configuration {
            body["encryptionConfiguration"] = serde_json::to_value(enc)?;
        }

        if let Some(tags) = self.tags
            && !tags.is_empty()
        {
            body["tags"] = serde_json::to_value(tags)?;
        }

        Ok(VectorsRequest::builder()
            .client(self.client)
            .path("/CreateVectorBucket".to_string())
            .body(Some(body))
            .build())
    }
}
