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

//! Builder for ListTagsForResource operation.

use crate::s3::error::ValidationErr;
use crate::s3vectors::client::VectorsClient;
use crate::s3vectors::response::ListTagsForResourceResponse;
use crate::s3vectors::types::{ToVectorsRequest, VectorsApi, VectorsRequest};
use typed_builder::TypedBuilder;

/// Builder for the ListTagsForResource S3Vectors API operation.
///
/// Lists tags on a vector bucket or index.
#[derive(Clone, Debug, TypedBuilder)]
pub struct ListTagsForResource {
    #[builder(!default)]
    client: VectorsClient,

    /// The ARN of the resource to list tags for.
    #[builder(!default)]
    resource_arn: String,
}

/// Builder type alias for [`ListTagsForResource`].
pub type ListTagsForResourceBldr = ListTagsForResourceBuilder<((VectorsClient,), (String,))>;

impl VectorsApi for ListTagsForResource {
    type VectorsResponse = ListTagsForResourceResponse;
}

impl ToVectorsRequest for ListTagsForResource {
    fn to_vectors_request(self) -> Result<VectorsRequest, ValidationErr> {
        let body = serde_json::json!({
            "resourceArn": self.resource_arn,
        });

        Ok(VectorsRequest::builder()
            .client(self.client)
            .path("/ListTagsForResource".to_string())
            .body(Some(body))
            .build())
    }
}
