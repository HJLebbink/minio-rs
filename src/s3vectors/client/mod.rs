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

//! S3Vectors client implementation.

mod create_index;
mod create_vector_bucket;
mod delete_index;
mod delete_vector_bucket;
mod delete_vector_bucket_policy;
mod delete_vectors;
mod get_index;
mod get_vector_bucket;
mod get_vector_bucket_policy;
mod get_vectors;
mod list_indexes;
mod list_tags_for_resource;
mod list_vector_buckets;
mod list_vectors;
mod put_vector_bucket_policy;
mod put_vectors;
mod query_vectors;
mod tag_resource;
mod untag_resource;

use crate::s3::creds::Provider;
use crate::s3::error::{Error, NetworkError, ValidationErr};
use crate::s3::http::BaseUrl;
use crate::s3::multimap_ext::{Multimap, MultimapExt};
use crate::s3::signer::SigningKeyCache;
use crate::s3::types::Region;
use crate::s3::utils::{sha256_hash, to_amz_date, utc_now};
use crate::s3vectors::types::VectorsRequest;
use http::HeaderMap;
use reqwest::Client;
use std::sync::{Arc, RwLock};

/// Client for S3Vectors API operations.
///
/// This client provides methods for interacting with S3Vectors, including
/// vector bucket management, index management, and vector operations.
///
/// # Example
///
/// ```no_run
/// use minio::s3vectors::VectorsClient;
/// use minio::s3::creds::StaticProvider;
/// use minio::s3::http::BaseUrl;
/// use minio::s3::types::Region;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let base_url = "http://localhost:9000".parse::<BaseUrl>()?;
/// let provider = StaticProvider::new("minioadmin", "minioadmin", None);
/// let region = Region::new("us-east-1")?;
///
/// let client = VectorsClient::new(base_url, Some(provider), Some(region))?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Debug)]
pub struct VectorsClient {
    /// Base URL for the S3Vectors service.
    base_url: BaseUrl,

    /// Credential provider for signing requests.
    provider: Option<Arc<dyn Provider + Send + Sync>>,

    /// AWS region.
    region: Region,

    /// HTTP client for making requests.
    http_client: Client,

    /// Signing key cache for performance.
    signing_key_cache: Arc<RwLock<SigningKeyCache>>,
}

impl VectorsClient {
    /// Creates a new S3Vectors client.
    ///
    /// # Arguments
    ///
    /// * `base_url` - The base URL of the S3Vectors service
    /// * `provider` - Optional credential provider for authentication
    /// * `region` - Optional region (defaults to "us-east-1")
    pub fn new(
        base_url: BaseUrl,
        provider: Option<impl Provider + Send + Sync + 'static>,
        region: Option<Region>,
    ) -> Result<Self, Error> {
        let provider = provider.map(|p| Arc::new(p) as Arc<dyn Provider + Send + Sync>);
        let region = region
            .unwrap_or_else(|| Region::new("us-east-1").unwrap_or_else(|_| Region::new_empty()));

        let http_client = Client::builder()
            .build()
            .map_err(|e| Error::Network(NetworkError::ReqwestError(e)))?;

        Ok(Self {
            base_url,
            provider,
            region,
            http_client,
            signing_key_cache: Arc::new(RwLock::new(SigningKeyCache::new())),
        })
    }

    /// Returns the base URL.
    pub fn base_url(&self) -> &BaseUrl {
        &self.base_url
    }

    /// Returns the region.
    pub fn region(&self) -> &Region {
        &self.region
    }

    /// Executes a VectorsRequest and returns the HTTP response.
    pub(crate) async fn execute_request(
        &self,
        request: &VectorsRequest,
    ) -> Result<reqwest::Response, Error> {
        let base_url_str = self.base_url.to_url_string();
        // MinIO S3Vectors API uses /_vectors prefix for all operations
        let path = format!("/_vectors{}", request.path());
        let url = format!("{}{}", base_url_str.trim_end_matches('/'), path);

        let body_bytes = request.body_bytes();
        let body_for_signing = body_bytes.as_ref().map(|b| b.as_ref()).unwrap_or(&[]);
        let content_sha256 = sha256_hash(body_for_signing);

        let mut headers = Multimap::new();
        headers.add("content-type", "application/json");

        // Add host header
        let host = self.get_host_header();
        headers.add("host", host.clone());

        // Add extra headers from request
        if let Some(extra) = request.extra_headers() {
            for (key, values) in extra.iter_all() {
                for value in values {
                    headers.add(key, value);
                }
            }
        }

        // Sign the request if we have credentials
        if let Some(ref provider) = self.provider {
            let creds = provider.fetch();
            let date = utc_now();

            // Add x-amz-date header (required for AWS SigV4)
            headers.add("x-amz-date", to_amz_date(date));
            headers.add("x-amz-content-sha256", content_sha256.clone());

            // Sign with just the path, not the full URL
            crate::s3::signer::sign_v4_s3_vectors(
                &self.signing_key_cache,
                &http::Method::POST,
                &path,
                &self.region,
                &mut headers,
                &Multimap::new(),
                &creds.access_key,
                &creds.secret_key,
                &content_sha256,
                date,
            );

            // Add session token if present
            if let Some(ref token) = creds.session_token {
                headers.add("x-amz-security-token", token.clone());
            }
        }

        // Convert Multimap headers to reqwest HeaderMap
        let mut header_map = HeaderMap::new();
        for (key, values) in headers.iter_all() {
            for value in values {
                if let (Ok(name), Ok(val)) = (
                    http::header::HeaderName::try_from(key.as_str()),
                    http::header::HeaderValue::try_from(value.as_str()),
                ) {
                    header_map.insert(name, val);
                }
            }
        }

        let mut req_builder = self.http_client.post(&url).headers(header_map);

        if let Some(body) = body_bytes {
            req_builder = req_builder.body(body);
        }

        let response = req_builder
            .send()
            .await
            .map_err(|e| Error::Network(NetworkError::ReqwestError(e)))?;

        // Check for error response
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();

            // Try to parse as JSON error
            if let Ok(error_json) = serde_json::from_str::<serde_json::Value>(&body) {
                let error_type = error_json
                    .get("__type")
                    .or_else(|| error_json.get("code"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("UnknownError")
                    .to_string();

                let message = error_json
                    .get("message")
                    .or_else(|| error_json.get("Message"))
                    .and_then(|v| v.as_str())
                    .unwrap_or(&body)
                    .to_string();

                return Err(Error::Validation(ValidationErr::StrError {
                    message: format!(
                        "S3Vectors error ({}): {} - {}",
                        status.as_u16(),
                        error_type,
                        message
                    ),
                    source: None,
                }));
            }

            return Err(Error::Validation(ValidationErr::StrError {
                message: format!("S3Vectors error ({}): {}", status.as_u16(), body),
                source: None,
            }));
        }

        Ok(response)
    }

    fn get_host_header(&self) -> String {
        // Build host header from base_url
        let base_url_str = self.base_url.to_url_string();
        // Remove scheme and trailing slash
        let host = base_url_str
            .strip_prefix("https://")
            .or_else(|| base_url_str.strip_prefix("http://"))
            .unwrap_or(&base_url_str);
        host.trim_end_matches('/').to_string()
    }
}
