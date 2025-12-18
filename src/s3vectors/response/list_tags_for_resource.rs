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

//! Response for ListTagsForResource operation.

use crate::s3::error::ValidationErr;
use crate::s3vectors::types::VectorsRequest;
use crate::{impl_from_vectors_response, impl_has_vectors_fields};
use bytes::Bytes;
use http::HeaderMap;
use std::collections::HashMap;

/// Response from the ListTagsForResource operation.
#[derive(Debug, Clone)]
pub struct ListTagsForResourceResponse {
    request: VectorsRequest,
    headers: HeaderMap,
    body: Bytes,
}

impl_from_vectors_response!(ListTagsForResourceResponse);
impl_has_vectors_fields!(ListTagsForResourceResponse);

impl ListTagsForResourceResponse {
    /// Returns the tags on the resource.
    pub fn tags(&self) -> Result<HashMap<String, String>, ValidationErr> {
        let json: serde_json::Value = serde_json::from_slice(&self.body)?;
        let tags = json
            .get("tags")
            .and_then(|v| v.as_object())
            .map(|obj| {
                obj.iter()
                    .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                    .collect()
            })
            .unwrap_or_default();
        Ok(tags)
    }
}
