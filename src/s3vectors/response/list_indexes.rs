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

//! Response for ListIndexes operation.

use crate::s3::error::ValidationErr;
use crate::s3vectors::types::{IndexSummary, VectorsRequest};
use crate::{impl_from_vectors_response, impl_has_vectors_fields};
use bytes::Bytes;
use http::HeaderMap;

/// Response from the ListIndexes operation.
#[derive(Debug, Clone)]
pub struct ListIndexesResponse {
    request: VectorsRequest,
    headers: HeaderMap,
    body: Bytes,
}

impl_from_vectors_response!(ListIndexesResponse);
impl_has_vectors_fields!(ListIndexesResponse);

impl ListIndexesResponse {
    /// Returns the list of index summaries.
    pub fn indexes(&self) -> Result<Vec<IndexSummary>, ValidationErr> {
        let json: serde_json::Value = serde_json::from_slice(&self.body)?;
        let arr = json
            .get("indexes")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        let mut indexes = Vec::with_capacity(arr.len());
        for v in arr {
            let index: IndexSummary = serde_json::from_value(v)?;
            indexes.push(index);
        }
        Ok(indexes)
    }

    /// Returns the next token for pagination, if available.
    pub fn next_token(&self) -> Result<Option<String>, ValidationErr> {
        let json: serde_json::Value = serde_json::from_slice(&self.body)?;
        Ok(json
            .get("nextToken")
            .and_then(|v| v.as_str())
            .map(String::from))
    }

    /// Returns true if there are more results to fetch.
    pub fn has_more(&self) -> Result<bool, ValidationErr> {
        Ok(self.next_token()?.is_some())
    }
}
