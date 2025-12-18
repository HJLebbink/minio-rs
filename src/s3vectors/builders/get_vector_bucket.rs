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

//! Builder for GetVectorBucket operation.

use crate::s3::error::ValidationErr;
use crate::s3vectors::client::VectorsClient;
use crate::s3vectors::error::VectorsValidationErr;
use crate::s3vectors::response::GetVectorBucketResponse;
use crate::s3vectors::types::{
    BucketName, ToVectorsRequest, VectorBucketArn, VectorsApi, VectorsRequest,
};
use typed_builder::TypedBuilder;

/// Builder for the [`GetVectorBucket`](https://docs.aws.amazon.com/AmazonS3/latest/API/API_S3VectorBuckets_GetVectorBucket.html) S3Vectors API operation.
///
/// Returns vector bucket attributes.
#[derive(Clone, Debug, TypedBuilder)]
pub struct GetVectorBucket {
    #[builder(!default)]
    client: VectorsClient,

    /// The name of the vector bucket.
    #[builder(default, setter(into))]
    bucket: Option<BucketName>,

    /// The ARN of the vector bucket (alternative to name).
    #[builder(default, setter(into))]
    vector_bucket_arn: Option<VectorBucketArn>,
}

/// Builder type alias for [`GetVectorBucket`].
pub type GetVectorBucketBldr =
    GetVectorBucketBuilder<((VectorsClient,), (Option<BucketName>,), ())>;

impl VectorsApi for GetVectorBucket {
    type VectorsResponse = GetVectorBucketResponse;
}

impl ToVectorsRequest for GetVectorBucket {
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

        Ok(VectorsRequest::builder()
            .client(self.client)
            .path("/GetVectorBucket".to_string())
            .body(Some(body))
            .build())
    }
}
