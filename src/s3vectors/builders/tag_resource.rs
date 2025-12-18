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

//! Builder for TagResource operation.

use crate::s3::error::ValidationErr;
use crate::s3vectors::client::VectorsClient;
use crate::s3vectors::error::VectorsValidationErr;
use crate::s3vectors::response::TagResourceResponse;
use crate::s3vectors::types::{ToVectorsRequest, VectorsApi, VectorsRequest};
use std::collections::HashMap;
use typed_builder::TypedBuilder;

/// Builder for the TagResource S3Vectors API operation.
///
/// Adds tags to a vector bucket or index.
#[derive(Clone, Debug, TypedBuilder)]
pub struct TagResource {
    #[builder(!default)]
    client: VectorsClient,

    /// The ARN of the resource to tag.
    #[builder(!default)]
    resource_arn: String,

    /// Tags to add to the resource.
    #[builder(!default)]
    tags: HashMap<String, String>,
}

/// Builder type alias for [`TagResource`].
pub type TagResourceBldr =
    TagResourceBuilder<((VectorsClient,), (String,), (HashMap<String, String>,))>;

impl VectorsApi for TagResource {
    type VectorsResponse = TagResourceResponse;
}

impl ToVectorsRequest for TagResource {
    fn to_vectors_request(self) -> Result<VectorsRequest, ValidationErr> {
        if self.tags.is_empty() {
            return Err(VectorsValidationErr::MissingRequiredField("tags".to_string()).into());
        }

        let body = serde_json::json!({
            "resourceArn": self.resource_arn,
            "tags": self.tags,
        });

        Ok(VectorsRequest::builder()
            .client(self.client)
            .path("/TagResource".to_string())
            .body(Some(body))
            .build())
    }
}
