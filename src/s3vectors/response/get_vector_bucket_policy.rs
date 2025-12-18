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

//! Response for GetVectorBucketPolicy operation.

use crate::s3::error::ValidationErr;
use crate::s3vectors::error::VectorsValidationErr;
use crate::s3vectors::types::VectorsRequest;
use crate::{impl_from_vectors_response, impl_has_vectors_fields};
use bytes::Bytes;
use http::HeaderMap;

/// Response from the GetVectorBucketPolicy operation.
#[derive(Debug, Clone)]
pub struct GetVectorBucketPolicyResponse {
    request: VectorsRequest,
    headers: HeaderMap,
    body: Bytes,
}

impl_from_vectors_response!(GetVectorBucketPolicyResponse);
impl_has_vectors_fields!(GetVectorBucketPolicyResponse);

impl GetVectorBucketPolicyResponse {
    /// Returns the bucket policy as a JSON string.
    pub fn policy(&self) -> Result<String, ValidationErr> {
        let json: serde_json::Value = serde_json::from_slice(&self.body)?;
        let policy = json.get("policy").and_then(|v| v.as_str()).ok_or_else(|| {
            ValidationErr::from(VectorsValidationErr::MissingRequiredField(
                "policy".to_string(),
            ))
        })?;
        Ok(policy.to_string())
    }

    /// Returns the bucket policy as a parsed JSON value.
    pub fn policy_json(&self) -> Result<serde_json::Value, ValidationErr> {
        let policy_str = self.policy()?;
        let policy: serde_json::Value = serde_json::from_str(&policy_str)?;
        Ok(policy)
    }
}
