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

//! Response for QueryVectors operation.

use crate::s3::error::ValidationErr;
use crate::s3vectors::types::{DistanceMetric, QueryOutputVector, VectorsRequest};
use crate::{impl_from_vectors_response, impl_has_vectors_fields};
use bytes::Bytes;
use http::HeaderMap;

/// Response from the QueryVectors operation.
#[derive(Debug, Clone)]
pub struct QueryVectorsResponse {
    request: VectorsRequest,
    headers: HeaderMap,
    body: Bytes,
}

impl_from_vectors_response!(QueryVectorsResponse);
impl_has_vectors_fields!(QueryVectorsResponse);

impl QueryVectorsResponse {
    /// Returns the query results (approximate nearest neighbors).
    pub fn vectors(&self) -> Result<Vec<QueryOutputVector>, ValidationErr> {
        let json: serde_json::Value = serde_json::from_slice(&self.body)?;
        let vectors = json
            .get("vectors")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| serde_json::from_value(v.clone()).ok())
                    .collect()
            })
            .unwrap_or_default();
        Ok(vectors)
    }

    /// Returns the distance metric used for the query.
    pub fn distance_metric(&self) -> Result<Option<DistanceMetric>, ValidationErr> {
        let json: serde_json::Value = serde_json::from_slice(&self.body)?;
        let metric = json
            .get("distanceMetric")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse().ok());
        Ok(metric)
    }
}
