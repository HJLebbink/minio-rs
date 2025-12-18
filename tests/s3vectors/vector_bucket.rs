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

//! Integration tests for S3Vectors vector bucket operations.

use minio::s3::types::BucketName;
use minio::s3vectors::VectorsApi;
use minio_common::utils::rand_bucket_name;
use minio_common::vectors_test_context::{VectorsTestContext, cleanup_vector_bucket};

/// Test creating a vector bucket.
#[tokio::test]
async fn test_create_vector_bucket() -> Result<(), Box<dyn std::error::Error>> {
    let ctx = VectorsTestContext::new_from_env();
    let bucket = rand_bucket_name();

    // Create vector bucket
    let result = ctx
        .client
        .create_vector_bucket(&bucket)?
        .build()
        .send()
        .await;

    match result {
        Ok(resp) => {
            let arn = resp.vector_bucket_arn().unwrap();
            assert!(!arn.as_str().is_empty());

            // Cleanup
            cleanup_vector_bucket(&ctx.client, &bucket).await;
        }
        Err(e) => {
            panic!("Failed to create vector bucket: {e}");
        }
    }
    Ok(())
}

/// Test getting a vector bucket.
#[tokio::test]
async fn test_get_vector_bucket() -> Result<(), Box<dyn std::error::Error>> {
    let ctx = VectorsTestContext::new_from_env();
    let bucket = rand_bucket_name();

    // Create vector bucket first
    ctx.client
        .create_vector_bucket(&bucket)?
        .build()
        .send()
        .await
        .expect("Failed to create vector bucket");

    // Get vector bucket details
    let result = ctx.client.get_vector_bucket(&bucket)?.build().send().await;

    match result {
        Ok(resp) => {
            let vbucket = resp.vector_bucket().unwrap();
            assert_eq!(vbucket.bucket.as_str(), bucket.as_str());
            assert!(!vbucket.vector_bucket_arn.as_str().is_empty());
        }
        Err(e) => {
            panic!("Failed to get vector bucket: {e}");
        }
    }

    // Cleanup
    cleanup_vector_bucket(&ctx.client, &bucket).await;
    Ok(())
}

/// Test deleting a vector bucket.
#[tokio::test]
async fn test_delete_vector_bucket() -> Result<(), Box<dyn std::error::Error>> {
    let ctx = VectorsTestContext::new_from_env();
    let bucket = rand_bucket_name();

    // Create vector bucket first
    ctx.client
        .create_vector_bucket(&bucket)?
        .build()
        .send()
        .await
        .expect("Failed to create vector bucket");

    // Delete vector bucket
    let result = ctx
        .client
        .delete_vector_bucket(&bucket)?
        .build()
        .send()
        .await;

    assert!(result.is_ok(), "Failed to delete vector bucket: {result:?}");

    // Verify bucket no longer exists by trying to get it
    let get_result = ctx.client.get_vector_bucket(&bucket)?.build().send().await;

    assert!(
        get_result.is_err(),
        "Expected error when getting deleted bucket"
    );
    Ok(())
}

/// Test listing vector buckets.
#[tokio::test]
async fn test_list_vector_buckets() -> Result<(), Box<dyn std::error::Error>> {
    let ctx = VectorsTestContext::new_from_env();
    let bucket = rand_bucket_name();

    // Create a vector bucket first
    ctx.client
        .create_vector_bucket(&bucket)?
        .build()
        .send()
        .await
        .expect("Failed to create vector bucket");

    // List vector buckets
    let result = ctx.client.list_vector_buckets().build().send().await;

    match result {
        Ok(resp) => {
            let buckets = resp.vector_buckets().unwrap();
            // Check that our bucket is in the list
            let found = buckets.iter().any(|b| b.bucket.as_str() == bucket.as_str());
            assert!(found, "Created bucket not found in list");
        }
        Err(e) => {
            panic!("Failed to list vector buckets: {e}");
        }
    }

    // Cleanup
    cleanup_vector_bucket(&ctx.client, &bucket).await;
    Ok(())
}

/// Test listing vector buckets with prefix filter.
#[tokio::test]
async fn test_list_vector_buckets_with_prefix() -> Result<(), Box<dyn std::error::Error>> {
    let ctx = VectorsTestContext::new_from_env();
    let bucket = rand_bucket_name();

    // Create a vector bucket
    ctx.client
        .create_vector_bucket(&bucket)?
        .build()
        .send()
        .await
        .expect("Failed to create vector bucket");

    // List with prefix that matches our bucket
    let result = ctx
        .client
        .list_vector_buckets()
        .prefix("test-vbucket-".to_string())
        .build()
        .send()
        .await;

    match result {
        Ok(resp) => {
            let buckets = resp.vector_buckets().unwrap();
            // All returned buckets should start with prefix
            for bucket in &buckets {
                assert!(
                    bucket.bucket.as_str().starts_with("test-vbucket-"),
                    "Bucket name doesn't match prefix: {}",
                    bucket.bucket
                );
            }
        }
        Err(e) => {
            panic!("Failed to list vector buckets with prefix: {e}");
        }
    }

    // Cleanup
    cleanup_vector_bucket(&ctx.client, &bucket).await;
    Ok(())
}

/// Test creating a vector bucket that already exists.
#[tokio::test]
async fn test_create_duplicate_vector_bucket() -> Result<(), Box<dyn std::error::Error>> {
    let ctx = VectorsTestContext::new_from_env();
    let bucket = rand_bucket_name();

    // Create vector bucket first
    ctx.client
        .create_vector_bucket(&bucket)?
        .build()
        .send()
        .await
        .expect("Failed to create vector bucket");

    // Try to create the same bucket again
    let result = ctx
        .client
        .create_vector_bucket(&bucket)?
        .build()
        .send()
        .await;

    // Should fail because bucket already exists
    assert!(
        result.is_err(),
        "Expected error when creating duplicate bucket"
    );

    // Cleanup
    cleanup_vector_bucket(&ctx.client, &bucket).await;
    Ok(())
}

/// Test deleting a non-existent vector bucket.
#[tokio::test]
async fn test_delete_nonexistent_vector_bucket() -> Result<(), Box<dyn std::error::Error>> {
    let ctx = VectorsTestContext::new_from_env();
    let bucket = BucketName::new("nonexistent-bucket-12345").unwrap();

    // Try to delete a bucket that doesn't exist
    let result = ctx
        .client
        .delete_vector_bucket(&bucket)?
        .build()
        .send()
        .await;

    // Should fail because bucket doesn't exist
    assert!(
        result.is_err(),
        "Expected error when deleting nonexistent bucket"
    );
    Ok(())
}

/// Test getting a non-existent vector bucket.
#[tokio::test]
async fn test_get_nonexistent_vector_bucket() -> Result<(), Box<dyn std::error::Error>> {
    let ctx = VectorsTestContext::new_from_env();
    let bucket = BucketName::new("nonexistent-bucket-12345").unwrap();

    // Try to get a bucket that doesn't exist
    let result = ctx.client.get_vector_bucket(&bucket)?.build().send().await;

    // Should fail because bucket doesn't exist
    assert!(
        result.is_err(),
        "Expected error when getting nonexistent bucket"
    );
    Ok(())
}

/// Test creating a vector bucket with tags.
#[tokio::test]
async fn test_create_vector_bucket_with_tags() -> Result<(), Box<dyn std::error::Error>> {
    let ctx = VectorsTestContext::new_from_env();
    let bucket = rand_bucket_name();

    let mut tags = std::collections::HashMap::new();
    tags.insert("environment".to_string(), "test".to_string());
    tags.insert("project".to_string(), "minio-rs".to_string());

    // Create vector bucket with tags
    let result = ctx
        .client
        .create_vector_bucket(&bucket)?
        .tags(tags)
        .build()
        .send()
        .await;

    match result {
        Ok(resp) => {
            let arn = resp.vector_bucket_arn().unwrap();
            assert!(!arn.as_str().is_empty());
        }
        Err(e) => {
            panic!("Failed to create vector bucket with tags: {e}");
        }
    }

    // Cleanup
    cleanup_vector_bucket(&ctx.client, &bucket).await;
    Ok(())
}

/// Test list vector buckets with pagination.
#[tokio::test]
async fn test_list_vector_buckets_pagination() -> Result<(), Box<dyn std::error::Error>> {
    let ctx = VectorsTestContext::new_from_env();
    let buckets: Vec<BucketName> = (0..3).map(|_| rand_bucket_name()).collect();

    // Create multiple buckets
    for bucket in &buckets {
        ctx.client
            .create_vector_bucket(bucket)?
            .build()
            .send()
            .await
            .expect("Failed to create vector bucket");
    }

    // List with max_results = 1 to test pagination
    let result = ctx
        .client
        .list_vector_buckets()
        .max_results(1u32)
        .build()
        .send()
        .await;

    match result {
        Ok(resp) => {
            let buckets = resp.vector_buckets().unwrap();
            assert!(
                buckets.len() <= 1,
                "Expected at most 1 bucket, got {}",
                buckets.len()
            );
            // Note: There may be more buckets, but we limited to 1
        }
        Err(e) => {
            panic!("Failed to list vector buckets with pagination: {e}");
        }
    }

    // Cleanup
    for bucket in &buckets {
        cleanup_vector_bucket(&ctx.client, bucket).await;
    }
    Ok(())
}
