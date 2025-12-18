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

//! Builder for UntagResource operation.

use crate::s3::error::ValidationErr;
use crate::s3vectors::client::VectorsClient;
use crate::s3vectors::error::VectorsValidationErr;
use crate::s3vectors::response::UntagResourceResponse;
use crate::s3vectors::types::{ToVectorsRequest, VectorsApi, VectorsRequest};
use typed_builder::TypedBuilder;

/// Builder for the UntagResource S3Vectors API operation.
///
/// Removes tags from a vector bucket or index.
#[derive(Clone, Debug, TypedBuilder)]
pub struct UntagResource {
    #[builder(!default)]
    client: VectorsClient,

    /// The ARN of the resource to untag.
    #[builder(!default)]
    resource_arn: String,

    /// Tag keys to remove from the resource.
    #[builder(!default)]
    tag_keys: Vec<String>,
}

/// Builder type alias for [`UntagResource`].
pub type UntagResourceBldr = UntagResourceBuilder<((VectorsClient,), (String,), (Vec<String>,))>;

impl VectorsApi for UntagResource {
    type VectorsResponse = UntagResourceResponse;
}

impl ToVectorsRequest for UntagResource {
    fn to_vectors_request(self) -> Result<VectorsRequest, ValidationErr> {
        if self.tag_keys.is_empty() {
            return Err(VectorsValidationErr::MissingRequiredField("tagKeys".to_string()).into());
        }

        let body = serde_json::json!({
            "resourceArn": self.resource_arn,
            "tagKeys": self.tag_keys,
        });

        Ok(VectorsRequest::builder()
            .client(self.client)
            .path("/UntagResource".to_string())
            .body(Some(body))
            .build())
    }
}
