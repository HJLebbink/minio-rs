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

//! Core traits for S3Vectors request and response handling.

use super::VectorsRequest;
use crate::s3::error::{Error, ValidationErr};
use async_trait::async_trait;

/// Trait for converting a request builder into a concrete S3Vectors HTTP request.
///
/// This trait is implemented by all S3Vectors request builders and serves as an
/// intermediate step in the request execution pipeline.
pub trait ToVectorsRequest: Sized {
    /// Consumes this request builder and returns a [`VectorsRequest`].
    ///
    /// This method transforms the request builder into a concrete HTTP request
    /// that can be executed against an S3Vectors-compatible service.
    fn to_vectors_request(self) -> Result<VectorsRequest, ValidationErr>;
}

/// Trait for converting HTTP responses into strongly typed S3Vectors response objects.
///
/// This trait is implemented by all S3Vectors response types and provides
/// a way to parse and validate raw HTTP responses.
#[async_trait]
pub trait FromVectorsResponse: Sized {
    /// Asynchronously converts an HTTP response into a strongly typed response.
    ///
    /// # Parameters
    ///
    /// * `request` - The original request that was executed
    /// * `response` - The result of the HTTP request execution
    ///
    /// # Returns
    ///
    /// * `Result<Self, Error>` - The typed response object on success, or an error
    async fn from_vectors_response(
        request: VectorsRequest,
        response: Result<reqwest::Response, Error>,
    ) -> Result<Self, Error>;
}

/// Trait that defines a common interface for all S3Vectors API request builders.
///
/// This trait provides a consistent way to send requests and get typed responses.
#[async_trait]
pub trait VectorsApi: ToVectorsRequest {
    /// The response type associated with this request builder.
    type VectorsResponse: FromVectorsResponse;

    /// Sends the S3Vectors API request and returns the corresponding typed response.
    ///
    /// This method consumes the request builder, converts it into a concrete HTTP
    /// request, executes the request, and then converts the HTTP response into
    /// the appropriate typed response.
    async fn send(self) -> Result<Self::VectorsResponse, Error> {
        let mut request = self.to_vectors_request()?;
        let response = request.execute().await;
        Self::VectorsResponse::from_vectors_response(request, response).await
    }
}
