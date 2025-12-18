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

use minio::s3::creds::StaticProvider;
use minio::s3::http::BaseUrl;
use minio::s3::types::{BucketName, Region};
use minio::s3vectors::{VectorsApi, VectorsClient};
use uuid::Uuid;

/// Test context for S3Vectors integration tests.
#[derive(Clone)]
pub struct VectorsTestContext {
    pub client: VectorsClient,
    pub base_url: BaseUrl,
    pub access_key: String,
    pub secret_key: String,
}

impl VectorsTestContext {
    /// Creates a new test context from environment variables or defaults.
    pub fn new_from_env() -> Self {
        let run_on_ci: bool = std::env::var("CI")
            .unwrap_or("false".into())
            .parse()
            .unwrap_or(false);

        if run_on_ci {
            let host = std::env::var("SERVER_ENDPOINT").unwrap();
            let access_key = std::env::var("ACCESS_KEY").unwrap();
            let secret_key = std::env::var("SECRET_KEY").unwrap();
            let secure = std::env::var("ENABLE_HTTPS").is_ok();
            let region = std::env::var("SERVER_REGION").ok();

            let mut base_url: BaseUrl = host.parse().unwrap();
            base_url.https = secure;
            if let Some(v) = region {
                base_url.region = Region::try_from(v.as_str()).unwrap();
            }

            let static_provider = StaticProvider::new(&access_key, &secret_key, None);
            let client = VectorsClient::new(
                base_url.clone(),
                Some(static_provider),
                Some(base_url.region.clone()),
            )
            .unwrap();

            Self {
                client,
                base_url,
                access_key,
                secret_key,
            }
        } else {
            const DEFAULT_SERVER_ENDPOINT: &str = "http://localhost:9000/";
            const DEFAULT_ACCESS_KEY: &str = "minioadmin";
            const DEFAULT_SECRET_KEY: &str = "minioadmin";
            const DEFAULT_ENABLE_HTTPS: &str = "false";
            const DEFAULT_SERVER_REGION: &str = "us-east-1";

            let host: String =
                std::env::var("SERVER_ENDPOINT").unwrap_or(DEFAULT_SERVER_ENDPOINT.to_string());
            log::debug!("SERVER_ENDPOINT={host}");
            let access_key: String =
                std::env::var("ACCESS_KEY").unwrap_or(DEFAULT_ACCESS_KEY.to_string());
            log::debug!("ACCESS_KEY={access_key}");
            let secret_key: String =
                std::env::var("SECRET_KEY").unwrap_or(DEFAULT_SECRET_KEY.to_string());
            log::debug!("SECRET_KEY=*****");
            let secure: bool = std::env::var("ENABLE_HTTPS")
                .unwrap_or(DEFAULT_ENABLE_HTTPS.to_string())
                .parse()
                .unwrap_or(false);
            log::debug!("ENABLE_HTTPS={secure}");
            let region_str: String =
                std::env::var("SERVER_REGION").unwrap_or(DEFAULT_SERVER_REGION.to_string());
            log::debug!("SERVER_REGION={region_str:?}");

            let mut base_url: BaseUrl = host.parse().unwrap();
            base_url.https = secure;
            base_url.region = Region::try_from(region_str.as_str()).unwrap();

            let static_provider = StaticProvider::new(&access_key, &secret_key, None);
            let client = VectorsClient::new(
                base_url.clone(),
                Some(static_provider),
                Some(base_url.region.clone()),
            )
            .unwrap();

            Self {
                client,
                base_url,
                access_key,
                secret_key,
            }
        }
    }
}

/// Generates a random vector bucket name for testing.
//TODO remove this function and replace with rand_bucket_name
pub fn rand_vectors_bucket() -> BucketName {
    BucketName::new(format!("test-vbucket-{}", Uuid::new_v4())).unwrap()
}

/// Cleanup helper for vector buckets.
/// Deletes all indexes in the bucket, then deletes the bucket itself.
pub async fn cleanup_vector_bucket(client: &VectorsClient, bucket: &BucketName) {
    // First, list and delete all indexes in the bucket
    if let Ok(list_indexes) = client.list_indexes(bucket) {
        match list_indexes.build().send().await {
            Ok(resp) => {
                if let Ok(indexes) = resp.indexes() {
                    for idx in indexes {
                        if let Ok(delete_index) = client.delete_index(bucket, &idx.index) {
                            let _ = delete_index.build().send().await;
                        }
                    }
                }
            }
            Err(e) => {
                log::debug!("Failed to list indexes for cleanup: {e}");
            }
        }
    }

    // Then delete the bucket
    if let Ok(delete_bucket) = client.delete_vector_bucket(bucket) {
        match delete_bucket.build().send().await {
            Ok(_) => {
                log::debug!("Vector bucket '{}' deleted successfully", bucket);
            }
            Err(e) => {
                log::debug!("Failed to delete vector bucket '{}': {e}", bucket);
            }
        }
    }
}
