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

//! VectorsRequest struct for executing S3Vectors API requests.

use crate::s3::error::Error;
use crate::s3::multimap_ext::Multimap;
use crate::s3vectors::client::VectorsClient;
use bytes::Bytes;
use typed_builder::TypedBuilder;

/// Request object for S3Vectors API operations.
///
/// Unlike standard S3 which uses path-based routing, S3Vectors uses
/// a REST API with JSON request/response bodies and operation-specific
/// endpoints (e.g., POST /CreateVectorBucket). All operations use POST.
#[derive(Clone, Debug, TypedBuilder)]
pub struct VectorsRequest {
    /// The client to use for executing the request.
    #[builder(!default)]
    pub(crate) client: VectorsClient,

    /// The API operation path (e.g., "/CreateVectorBucket").
    #[builder(!default)]
    pub(crate) path: String,

    /// JSON request body.
    #[builder(default)]
    pub(crate) body: Option<serde_json::Value>,

    /// Extra headers to include in the request.
    #[builder(default)]
    pub(crate) extra_headers: Option<Multimap>,
}

impl VectorsRequest {
    /// Returns the operation path.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Returns the request body as JSON.
    pub fn body(&self) -> Option<&serde_json::Value> {
        self.body.as_ref()
    }

    /// Returns the extra headers.
    pub fn extra_headers(&self) -> Option<&Multimap> {
        self.extra_headers.as_ref()
    }

    /// Executes the request and returns the raw HTTP response.
    pub async fn execute(&mut self) -> Result<reqwest::Response, Error> {
        self.client.execute_request(self).await
    }

    /// Converts the body to bytes for sending.
    pub(crate) fn body_bytes(&self) -> Option<Bytes> {
        self.body
            .as_ref()
            .map(|b| Bytes::from(serde_json::to_vec(b).unwrap_or_default()))
    }
}
