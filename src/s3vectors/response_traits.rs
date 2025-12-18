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

//! Response traits and macros for S3Vectors API.

use crate::s3vectors::types::VectorsRequest;
use bytes::Bytes;
use http::HeaderMap;

/// Trait for accessing the underlying fields of a S3Vectors response.
pub trait HasVectorsFields {
    /// Returns a reference to the original request.
    fn request(&self) -> &VectorsRequest;

    /// Returns a reference to the response headers.
    fn headers(&self) -> &HeaderMap;

    /// Returns a reference to the response body bytes.
    fn body(&self) -> &Bytes;
}

/// Macro to implement `HasVectorsFields` for a response type.
///
/// The response type must have `request`, `headers`, and `body` fields.
#[macro_export]
macro_rules! impl_has_vectors_fields {
    ($t:ty) => {
        impl $crate::s3vectors::response_traits::HasVectorsFields for $t {
            fn request(&self) -> &$crate::s3vectors::types::VectorsRequest {
                &self.request
            }

            fn headers(&self) -> &http::HeaderMap {
                &self.headers
            }

            fn body(&self) -> &bytes::Bytes {
                &self.body
            }
        }
    };
}

/// Macro to implement `FromVectorsResponse` for a response type.
///
/// The response type must have `request`, `headers`, and `body` fields.
#[macro_export]
macro_rules! impl_from_vectors_response {
    ($t:ty) => {
        #[async_trait::async_trait]
        impl $crate::s3vectors::types::FromVectorsResponse for $t {
            async fn from_vectors_response(
                request: $crate::s3vectors::types::VectorsRequest,
                response: Result<reqwest::Response, $crate::s3::error::Error>,
            ) -> Result<Self, $crate::s3::error::Error> {
                match response {
                    Ok(mut resp) => {
                        let headers = std::mem::take(resp.headers_mut());
                        let body = resp.bytes().await.map_err(|e| {
                            $crate::s3::error::Error::Network(
                                $crate::s3::error::NetworkError::ReqwestError(e),
                            )
                        })?;
                        Ok(Self {
                            request,
                            headers,
                            body,
                        })
                    }
                    Err(e) => Err(e),
                }
            }
        }
    };
}

pub use impl_from_vectors_response;
pub use impl_has_vectors_fields;
