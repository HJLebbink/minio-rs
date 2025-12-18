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

//! Response for CreateVectorBucket operation.

use crate::s3::error::ValidationErr;
use crate::s3vectors::error::VectorsValidationErr;
use crate::s3vectors::types::{VectorBucketArn, VectorsRequest};
use crate::{impl_from_vectors_response, impl_has_vectors_fields};
use bytes::Bytes;
use http::HeaderMap;

/// Response from the CreateVectorBucket operation.
#[derive(Debug, Clone)]
pub struct CreateVectorBucketResponse {
    request: VectorsRequest,
    headers: HeaderMap,
    body: Bytes,
}

impl_from_vectors_response!(CreateVectorBucketResponse);
impl_has_vectors_fields!(CreateVectorBucketResponse);

impl CreateVectorBucketResponse {
    /// Returns the ARN of the created vector bucket.
    pub fn vector_bucket_arn(&self) -> Result<VectorBucketArn, ValidationErr> {
        let json: serde_json::Value = serde_json::from_slice(&self.body)?;
        let arn = json
            .get("vectorBucketArn")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                ValidationErr::from(VectorsValidationErr::MissingRequiredField(
                    "vectorBucketArn".to_string(),
                ))
            })?;
        Ok(VectorBucketArn::new(arn))
    }
}
