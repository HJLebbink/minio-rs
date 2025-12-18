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

//! Integration tests for S3Vectors index operations.

use minio::s3vectors::{DistanceMetric, IndexName, VectorsApi};
use minio_common::utils::rand_bucket_name;
use minio_common::vectors_test_context::{VectorsTestContext, cleanup_vector_bucket};

/// Test creating an index with Euclidean distance metric.
#[tokio::test]
async fn test_create_index_euclidean() {
    let ctx = VectorsTestContext::new_from_env();
    let bucket = rand_bucket_name();
    let index = IndexName::new("test-index-euclidean").unwrap();

    // Create vector bucket first
    ctx.client
        .create_vector_bucket(&bucket)
        .expect("Invalid arguments")
        .build()
        .send()
        .await
        .expect("Failed to create vector bucket");

    // Create index with Euclidean distance
    let result = ctx
        .client
        .create_index(&bucket, &index, 128, DistanceMetric::Euclidean)
        .expect("Invalid arguments")
        .build()
        .send()
        .await;

    match result {
        Ok(resp) => {
            let arn = resp.index_arn().unwrap();
            assert!(!arn.as_str().is_empty());
        }
        Err(e) => {
            panic!("Failed to create index: {e}");
        }
    }

    // Cleanup
    cleanup_vector_bucket(&ctx.client, &bucket).await;
}

/// Test creating an index with Cosine distance metric.
#[tokio::test]
async fn test_create_index_cosine() {
    let ctx = VectorsTestContext::new_from_env();
    let bucket = rand_bucket_name();
    let index = IndexName::new("test-index-cosine").unwrap();

    // Create vector bucket first
    ctx.client
        .create_vector_bucket(&bucket)
        .expect("Invalid arguments")
        .build()
        .send()
        .await
        .expect("Failed to create vector bucket");

    // Create index with Cosine distance
    let result = ctx
        .client
        .create_index(&bucket, &index, 256, DistanceMetric::Cosine)
        .expect("Invalid arguments")
        .build()
        .send()
        .await;

    match result {
        Ok(resp) => {
            let arn = resp.index_arn().unwrap();
            assert!(!arn.as_str().is_empty());
        }
        Err(e) => {
            panic!("Failed to create index with cosine metric: {e}");
        }
    }

    // Cleanup
    cleanup_vector_bucket(&ctx.client, &bucket).await;
}

/// Test getting an index.
#[tokio::test]
async fn test_get_index() {
    let ctx = VectorsTestContext::new_from_env();
    let bucket = rand_bucket_name();
    let index = IndexName::new("test-get-index").unwrap();

    // Create vector bucket first
    ctx.client
        .create_vector_bucket(&bucket)
        .expect("Invalid arguments")
        .build()
        .send()
        .await
        .expect("Failed to create vector bucket");

    // Create index
    ctx.client
        .create_index(&bucket, &index, 512, DistanceMetric::Euclidean)
        .expect("Invalid arguments")
        .build()
        .send()
        .await
        .expect("Failed to create index");

    // Get index details
    let result = ctx
        .client
        .get_index(&bucket, &index)
        .expect("Invalid arguments")
        .build()
        .send()
        .await;

    match result {
        Ok(resp) => {
            let index_info = resp.index().unwrap();
            assert_eq!(index_info.index.as_str(), index.as_str());
            assert_eq!(index_info.dimension, 512);
            assert_eq!(index_info.distance_metric, DistanceMetric::Euclidean);
        }
        Err(e) => {
            panic!("Failed to get index: {e}");
        }
    }

    // Cleanup
    cleanup_vector_bucket(&ctx.client, &bucket).await;
}

/// Test deleting an index.
#[tokio::test]
async fn test_delete_index() {
    let ctx = VectorsTestContext::new_from_env();
    let bucket = rand_bucket_name();
    let index = IndexName::new("test-delete-index").unwrap();

    // Create vector bucket first
    ctx.client
        .create_vector_bucket(&bucket)
        .expect("Invalid arguments")
        .build()
        .send()
        .await
        .expect("Failed to create vector bucket");

    // Create index
    ctx.client
        .create_index(&bucket, &index, 128, DistanceMetric::Euclidean)
        .expect("Invalid arguments")
        .build()
        .send()
        .await
        .expect("Failed to create index");

    // Delete index
    let result = ctx
        .client
        .delete_index(&bucket, &index)
        .expect("Invalid arguments")
        .build()
        .send()
        .await;

    assert!(result.is_ok(), "Failed to delete index: {result:?}");

    // Verify index no longer exists
    let get_result = ctx
        .client
        .get_index(&bucket, &index)
        .expect("Invalid arguments")
        .build()
        .send()
        .await;

    assert!(
        get_result.is_err(),
        "Expected error when getting deleted index"
    );

    // Cleanup bucket
    cleanup_vector_bucket(&ctx.client, &bucket).await;
}

/// Test listing indexes in a bucket.
#[tokio::test]
async fn test_list_indexes() {
    let ctx = VectorsTestContext::new_from_env();
    let bucket = rand_bucket_name();
    let index1 = IndexName::new("test-list-index-1").unwrap();
    let index2 = IndexName::new("test-list-index-2").unwrap();

    // Create vector bucket first
    ctx.client
        .create_vector_bucket(&bucket)
        .expect("Invalid arguments")
        .build()
        .send()
        .await
        .expect("Failed to create vector bucket");

    // Create two indexes
    ctx.client
        .create_index(&bucket, &index1, 128, DistanceMetric::Euclidean)
        .expect("Invalid arguments")
        .build()
        .send()
        .await
        .expect("Failed to create index 1");

    ctx.client
        .create_index(&bucket, &index2, 256, DistanceMetric::Cosine)
        .expect("Invalid arguments")
        .build()
        .send()
        .await
        .expect("Failed to create index 2");

    // List indexes
    let result = ctx
        .client
        .list_indexes(&bucket)
        .expect("Invalid arguments")
        .build()
        .send()
        .await;

    match result {
        Ok(resp) => {
            let indexes = resp.indexes().unwrap();
            assert!(indexes.len() >= 2, "Expected at least 2 indexes");

            let found1 = indexes.iter().any(|i| i.index.as_str() == index1.as_str());
            let found2 = indexes.iter().any(|i| i.index.as_str() == index2.as_str());

            assert!(found1, "Index 1 not found in list");
            assert!(found2, "Index 2 not found in list");
        }
        Err(e) => {
            panic!("Failed to list indexes: {e}");
        }
    }

    // Cleanup
    cleanup_vector_bucket(&ctx.client, &bucket).await;
}

/// Test listing indexes with prefix filter.
#[tokio::test]
#[ignore = "ListIndexes prefix filtering not implemented in MinIO server"]
async fn test_list_indexes_with_prefix() {
    let ctx = VectorsTestContext::new_from_env();
    let bucket = rand_bucket_name();
    let index1 = IndexName::new("prefix-test-1").unwrap();
    let index2 = IndexName::new("prefix-test-2").unwrap();
    let index3 = IndexName::new("other-index").unwrap();

    // Create vector bucket first
    ctx.client
        .create_vector_bucket(&bucket)
        .expect("Invalid arguments")
        .build()
        .send()
        .await
        .expect("Failed to create vector bucket");

    // Create indexes with different prefixes
    for index in [&index1, &index2, &index3] {
        ctx.client
            .create_index(&bucket, index, 128, DistanceMetric::Euclidean)
            .expect("Invalid arguments")
            .build()
            .send()
            .await
            .expect("Failed to create index");
    }

    // List indexes with prefix
    let result = ctx
        .client
        .list_indexes(&bucket)
        .expect("Invalid arguments")
        .prefix("prefix-".to_string())
        .build()
        .send()
        .await;

    match result {
        Ok(resp) => {
            let indexes = resp.indexes().unwrap();
            // All returned indexes should start with prefix
            for index in &indexes {
                assert!(
                    index.index.as_str().starts_with("prefix-"),
                    "Index name doesn't match prefix: {}",
                    index.index
                );
            }
            // Should have exactly 2 indexes with this prefix
            assert_eq!(indexes.len(), 2, "Expected 2 indexes with prefix");
        }
        Err(e) => {
            panic!("Failed to list indexes with prefix: {e}");
        }
    }

    // Cleanup
    cleanup_vector_bucket(&ctx.client, &bucket).await;
}

/// Test creating duplicate index.
#[tokio::test]
async fn test_create_duplicate_index() {
    let ctx = VectorsTestContext::new_from_env();
    let bucket = rand_bucket_name();
    let index = IndexName::new("duplicate-test").unwrap();

    // Create vector bucket first
    ctx.client
        .create_vector_bucket(&bucket)
        .expect("Invalid arguments")
        .build()
        .send()
        .await
        .expect("Failed to create vector bucket");

    // Create index
    ctx.client
        .create_index(&bucket, &index, 128, DistanceMetric::Euclidean)
        .expect("Invalid arguments")
        .build()
        .send()
        .await
        .expect("Failed to create index");

    // Try to create the same index again
    let result = ctx
        .client
        .create_index(&bucket, &index, 128, DistanceMetric::Euclidean)
        .expect("Invalid arguments")
        .build()
        .send()
        .await;

    assert!(
        result.is_err(),
        "Expected error when creating duplicate index"
    );

    // Cleanup
    cleanup_vector_bucket(&ctx.client, &bucket).await;
}

/// Test getting a non-existent index.
#[tokio::test]
async fn test_get_nonexistent_index() {
    let ctx = VectorsTestContext::new_from_env();
    let bucket = rand_bucket_name();
    let index = IndexName::new("nonexistent-index").unwrap();

    // Create vector bucket first
    ctx.client
        .create_vector_bucket(&bucket)
        .expect("Invalid arguments")
        .build()
        .send()
        .await
        .expect("Failed to create vector bucket");

    // Try to get an index that doesn't exist
    let result = ctx
        .client
        .get_index(&bucket, &index)
        .expect("Invalid arguments")
        .build()
        .send()
        .await;

    assert!(
        result.is_err(),
        "Expected error when getting nonexistent index"
    );

    // Cleanup
    cleanup_vector_bucket(&ctx.client, &bucket).await;
}

/// Test deleting a non-existent index.
#[tokio::test]
async fn test_delete_nonexistent_index() {
    let ctx = VectorsTestContext::new_from_env();
    let bucket = rand_bucket_name();
    let index = IndexName::new("nonexistent-index").unwrap();

    // Create vector bucket first
    ctx.client
        .create_vector_bucket(&bucket)
        .expect("Invalid arguments")
        .build()
        .send()
        .await
        .expect("Failed to create vector bucket");

    // Try to delete an index that doesn't exist
    let result = ctx
        .client
        .delete_index(&bucket, &index)
        .expect("Invalid arguments")
        .build()
        .send()
        .await;

    assert!(
        result.is_err(),
        "Expected error when deleting nonexistent index"
    );

    // Cleanup
    cleanup_vector_bucket(&ctx.client, &bucket).await;
}

/// Test creating index with different dimensions.
#[tokio::test]
async fn test_create_index_various_dimensions() {
    let ctx = VectorsTestContext::new_from_env();
    let bucket = rand_bucket_name();

    // Create vector bucket first
    ctx.client
        .create_vector_bucket(&bucket)
        .expect("Invalid arguments")
        .build()
        .send()
        .await
        .expect("Failed to create vector bucket");

    // Test various valid dimensions
    let dimensions = [1, 64, 128, 256, 512, 1024, 2048, 4096];

    for (i, dim) in dimensions.iter().enumerate() {
        let index = IndexName::new(format!("dim-test-{}", i)).unwrap();

        let result = ctx
            .client
            .create_index(&bucket, &index, *dim, DistanceMetric::Euclidean)
            .expect("Invalid arguments")
            .build()
            .send()
            .await;

        assert!(
            result.is_ok(),
            "Failed to create index with dimension {}: {:?}",
            dim,
            result
        );
    }

    // Cleanup
    cleanup_vector_bucket(&ctx.client, &bucket).await;
}

/// Test creating index with tags.
#[tokio::test]
async fn test_create_index_with_tags() {
    let ctx = VectorsTestContext::new_from_env();
    let bucket = rand_bucket_name();
    let index = IndexName::new("tagged-index").unwrap();

    // Create vector bucket first
    ctx.client
        .create_vector_bucket(&bucket)
        .expect("Invalid arguments")
        .build()
        .send()
        .await
        .expect("Failed to create vector bucket");

    let mut tags = std::collections::HashMap::new();
    tags.insert("environment".to_string(), "test".to_string());
    tags.insert("model".to_string(), "embeddings-v1".to_string());

    // Create index with tags
    let result = ctx
        .client
        .create_index(&bucket, &index, 128, DistanceMetric::Euclidean)
        .expect("Invalid arguments")
        .tags(tags)
        .build()
        .send()
        .await;

    match result {
        Ok(resp) => {
            let arn = resp.index_arn().unwrap();
            assert!(!arn.as_str().is_empty());
        }
        Err(e) => {
            panic!("Failed to create index with tags: {e}");
        }
    }

    // Cleanup
    cleanup_vector_bucket(&ctx.client, &bucket).await;
}
