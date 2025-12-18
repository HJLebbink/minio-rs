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

//! Integration tests for S3Vectors tagging operations.
//!
//! NOTE: These tests are ignored because the TagResource, UntagResource, and
//! ListTagsForResource APIs are not yet implemented in the MinIO server.

use minio::s3vectors::{DistanceMetric, IndexName, VectorsApi};
use minio_common::utils::rand_bucket_name;
use minio_common::vectors_test_context::{VectorsTestContext, cleanup_vector_bucket};
use std::collections::HashMap;

/// Test tagging a vector bucket.
#[tokio::test]
#[ignore = "TagResource API not implemented in MinIO server"]
async fn test_tag_vector_bucket() {
    let ctx = VectorsTestContext::new_from_env();
    let bucket = rand_bucket_name();

    // Create vector bucket first
    let create_resp = ctx
        .client
        .create_vector_bucket(&bucket)
        .expect("Failed to build create vector bucket request")
        .build()
        .send()
        .await
        .expect("Failed to create vector bucket");

    let bucket_arn = create_resp.vector_bucket_arn().unwrap();

    // Add tags to the bucket
    let mut tags = HashMap::new();
    tags.insert("environment".to_string(), "test".to_string());
    tags.insert("team".to_string(), "ml-platform".to_string());

    let result = ctx
        .client
        .tag_resource(bucket_arn.as_str(), tags)
        .build()
        .send()
        .await;

    assert!(result.is_ok(), "Failed to tag vector bucket: {result:?}");

    // Cleanup
    cleanup_vector_bucket(&ctx.client, &bucket).await;
}

/// Test listing tags on a vector bucket.
#[tokio::test]
#[ignore = "ListTagsForResource API not implemented in MinIO server"]
async fn test_list_tags_for_vector_bucket() {
    let ctx = VectorsTestContext::new_from_env();
    let bucket = rand_bucket_name();

    // Create vector bucket first
    let create_resp = ctx
        .client
        .create_vector_bucket(&bucket)
        .expect("Failed to build create vector bucket request")
        .build()
        .send()
        .await
        .expect("Failed to create vector bucket");

    let bucket_arn = create_resp.vector_bucket_arn().unwrap();

    // Add tags
    let mut tags = HashMap::new();
    tags.insert("key1".to_string(), "value1".to_string());
    tags.insert("key2".to_string(), "value2".to_string());

    ctx.client
        .tag_resource(bucket_arn.as_str(), tags.clone())
        .build()
        .send()
        .await
        .expect("Failed to tag resource");

    // List tags
    let result = ctx
        .client
        .list_tags_for_resource(bucket_arn.as_str())
        .build()
        .send()
        .await;

    match result {
        Ok(resp) => {
            let retrieved_tags = resp.tags().unwrap();
            assert_eq!(retrieved_tags.get("key1"), Some(&"value1".to_string()));
            assert_eq!(retrieved_tags.get("key2"), Some(&"value2".to_string()));
        }
        Err(e) => {
            panic!("Failed to list tags: {e}");
        }
    }

    // Cleanup
    cleanup_vector_bucket(&ctx.client, &bucket).await;
}

/// Test untagging a vector bucket.
#[tokio::test]
#[ignore = "UntagResource API not implemented in MinIO server"]
async fn test_untag_vector_bucket() {
    let ctx = VectorsTestContext::new_from_env();
    let bucket = rand_bucket_name();

    // Create vector bucket first
    let create_resp = ctx
        .client
        .create_vector_bucket(&bucket)
        .expect("Failed to build create vector bucket request")
        .build()
        .send()
        .await
        .expect("Failed to create vector bucket");

    let bucket_arn = create_resp.vector_bucket_arn().unwrap();

    // Add tags
    let mut tags = HashMap::new();
    tags.insert("key1".to_string(), "value1".to_string());
    tags.insert("key2".to_string(), "value2".to_string());
    tags.insert("key3".to_string(), "value3".to_string());

    ctx.client
        .tag_resource(bucket_arn.as_str(), tags)
        .build()
        .send()
        .await
        .expect("Failed to tag resource");

    // Remove some tags
    let keys_to_remove = vec!["key1".to_string(), "key2".to_string()];

    let result = ctx
        .client
        .untag_resource(bucket_arn.as_str(), keys_to_remove)
        .build()
        .send()
        .await;

    assert!(result.is_ok(), "Failed to untag resource: {result:?}");

    // Verify remaining tags
    let list_result = ctx
        .client
        .list_tags_for_resource(bucket_arn.as_str())
        .build()
        .send()
        .await
        .expect("Failed to list tags");

    let remaining_tags = list_result.tags().unwrap();
    assert!(
        !remaining_tags.contains_key("key1"),
        "key1 should be removed"
    );
    assert!(
        !remaining_tags.contains_key("key2"),
        "key2 should be removed"
    );
    assert_eq!(
        remaining_tags.get("key3"),
        Some(&"value3".to_string()),
        "key3 should remain"
    );

    // Cleanup
    cleanup_vector_bucket(&ctx.client, &bucket).await;
}

/// Test tagging an index.
#[tokio::test]
#[ignore = "TagResource API not implemented in MinIO server"]
async fn test_tag_index() {
    let ctx = VectorsTestContext::new_from_env();
    let bucket = rand_bucket_name();
    let index = IndexName::new("tagged-index").unwrap();

    // Create vector bucket first
    ctx.client
        .create_vector_bucket(&bucket)
        .expect("Failed to build create vector bucket request")
        .build()
        .send()
        .await
        .expect("Failed to create vector bucket");

    // Create index
    let create_index_resp = ctx
        .client
        .create_index(&bucket, &index, 128, DistanceMetric::Euclidean)
        .expect("Failed to build create index request")
        .build()
        .send()
        .await
        .expect("Failed to create index");

    let index_arn = create_index_resp.index_arn().unwrap();

    // Add tags to the index
    let mut tags = HashMap::new();
    tags.insert("model".to_string(), "embeddings-v1".to_string());
    tags.insert("dimension".to_string(), "128".to_string());

    let result = ctx
        .client
        .tag_resource(index_arn.as_str(), tags)
        .build()
        .send()
        .await;

    assert!(result.is_ok(), "Failed to tag index: {result:?}");

    // Cleanup
    cleanup_vector_bucket(&ctx.client, &bucket).await;
}

/// Test listing tags on an index.
#[tokio::test]
#[ignore = "ListTagsForResource API not implemented in MinIO server"]
async fn test_list_tags_for_index() {
    let ctx = VectorsTestContext::new_from_env();
    let bucket = rand_bucket_name();
    let index = IndexName::new("list-tagged-index").unwrap();

    // Create vector bucket first
    ctx.client
        .create_vector_bucket(&bucket)
        .expect("Failed to build create vector bucket request")
        .build()
        .send()
        .await
        .expect("Failed to create vector bucket");

    // Create index
    let create_index_resp = ctx
        .client
        .create_index(&bucket, &index, 64, DistanceMetric::Cosine)
        .expect("Failed to build create index request")
        .build()
        .send()
        .await
        .expect("Failed to create index");

    let index_arn = create_index_resp.index_arn().unwrap();

    // Add tags
    let mut tags = HashMap::new();
    tags.insert("purpose".to_string(), "testing".to_string());

    ctx.client
        .tag_resource(index_arn.as_str(), tags.clone())
        .build()
        .send()
        .await
        .expect("Failed to tag index");

    // List tags
    let result = ctx
        .client
        .list_tags_for_resource(index_arn.as_str())
        .build()
        .send()
        .await;

    match result {
        Ok(resp) => {
            let retrieved_tags = resp.tags().unwrap();
            assert_eq!(retrieved_tags.get("purpose"), Some(&"testing".to_string()));
        }
        Err(e) => {
            panic!("Failed to list index tags: {e}");
        }
    }

    // Cleanup
    cleanup_vector_bucket(&ctx.client, &bucket).await;
}

/// Test untagging an index.
#[tokio::test]
#[ignore = "UntagResource API not implemented in MinIO server"]
async fn test_untag_index() {
    let ctx = VectorsTestContext::new_from_env();
    let bucket = rand_bucket_name();
    let index = IndexName::new("untag-test-index").unwrap();

    // Create vector bucket first
    ctx.client
        .create_vector_bucket(&bucket)
        .expect("Failed to build create vector bucket request")
        .build()
        .send()
        .await
        .expect("Failed to create vector bucket");

    // Create index
    let create_index_resp = ctx
        .client
        .create_index(&bucket, &index, 128, DistanceMetric::Euclidean)
        .expect("Failed to build create index request")
        .build()
        .send()
        .await
        .expect("Failed to create index");

    let index_arn = create_index_resp.index_arn().unwrap();

    // Add tags
    let mut tags = HashMap::new();
    tags.insert("tag1".to_string(), "value1".to_string());
    tags.insert("tag2".to_string(), "value2".to_string());

    ctx.client
        .tag_resource(index_arn.as_str(), tags)
        .build()
        .send()
        .await
        .expect("Failed to tag index");

    // Remove a tag
    let result = ctx
        .client
        .untag_resource(index_arn.as_str(), vec!["tag1".to_string()])
        .build()
        .send()
        .await;

    assert!(result.is_ok(), "Failed to untag index: {result:?}");

    // Verify remaining tags
    let list_result = ctx
        .client
        .list_tags_for_resource(index_arn.as_str())
        .build()
        .send()
        .await
        .expect("Failed to list tags");

    let remaining_tags = list_result.tags().unwrap();
    assert!(
        !remaining_tags.contains_key("tag1"),
        "tag1 should be removed"
    );
    assert_eq!(
        remaining_tags.get("tag2"),
        Some(&"value2".to_string()),
        "tag2 should remain"
    );

    // Cleanup
    cleanup_vector_bucket(&ctx.client, &bucket).await;
}

/// Test listing tags on resource without tags.
#[tokio::test]
#[ignore = "ListTagsForResource API not implemented in MinIO server"]
async fn test_list_tags_empty() {
    let ctx = VectorsTestContext::new_from_env();
    let bucket = rand_bucket_name();

    // Create vector bucket without tags
    let create_resp = ctx
        .client
        .create_vector_bucket(&bucket)
        .expect("Failed to build create vector bucket request")
        .build()
        .send()
        .await
        .expect("Failed to create vector bucket");

    let bucket_arn = create_resp.vector_bucket_arn().unwrap();

    // List tags (should be empty)
    let result = ctx
        .client
        .list_tags_for_resource(bucket_arn.as_str())
        .build()
        .send()
        .await;

    match result {
        Ok(resp) => {
            let tags = resp.tags().unwrap();
            assert!(tags.is_empty(), "Expected empty tags");
        }
        Err(_) => {
            // Some implementations might return error for no tags
            // This is acceptable behavior
        }
    }

    // Cleanup
    cleanup_vector_bucket(&ctx.client, &bucket).await;
}

/// Test adding multiple tags in succession.
#[tokio::test]
#[ignore = "TagResource API not implemented in MinIO server"]
async fn test_add_tags_multiple_times() {
    let ctx = VectorsTestContext::new_from_env();
    let bucket = rand_bucket_name();

    // Create vector bucket first
    let create_resp = ctx
        .client
        .create_vector_bucket(&bucket)
        .expect("Failed to build create vector bucket request")
        .build()
        .send()
        .await
        .expect("Failed to create vector bucket");

    let bucket_arn = create_resp.vector_bucket_arn().unwrap();

    // Add first set of tags
    let mut tags1 = HashMap::new();
    tags1.insert("key1".to_string(), "value1".to_string());

    ctx.client
        .tag_resource(bucket_arn.as_str(), tags1)
        .build()
        .send()
        .await
        .expect("Failed to add first tags");

    // Add second set of tags
    let mut tags2 = HashMap::new();
    tags2.insert("key2".to_string(), "value2".to_string());

    ctx.client
        .tag_resource(bucket_arn.as_str(), tags2)
        .build()
        .send()
        .await
        .expect("Failed to add second tags");

    // List all tags
    let result = ctx
        .client
        .list_tags_for_resource(bucket_arn.as_str())
        .build()
        .send()
        .await
        .expect("Failed to list tags");

    let tags = result.tags().unwrap();
    // Both tags should be present
    assert_eq!(tags.get("key1"), Some(&"value1".to_string()));
    assert_eq!(tags.get("key2"), Some(&"value2".to_string()));

    // Cleanup
    cleanup_vector_bucket(&ctx.client, &bucket).await;
}

/// Test updating tag value.
#[tokio::test]
#[ignore = "TagResource API not implemented in MinIO server"]
async fn test_update_tag_value() {
    let ctx = VectorsTestContext::new_from_env();
    let bucket = rand_bucket_name();

    // Create vector bucket first
    let create_resp = ctx
        .client
        .create_vector_bucket(&bucket)
        .expect("Failed to build create vector bucket request")
        .build()
        .send()
        .await
        .expect("Failed to create vector bucket");

    let bucket_arn = create_resp.vector_bucket_arn().unwrap();

    // Add initial tag
    let mut tags1 = HashMap::new();
    tags1.insert("version".to_string(), "v1".to_string());

    ctx.client
        .tag_resource(bucket_arn.as_str(), tags1)
        .build()
        .send()
        .await
        .expect("Failed to add tag");

    // Update tag with new value
    let mut tags2 = HashMap::new();
    tags2.insert("version".to_string(), "v2".to_string());

    ctx.client
        .tag_resource(bucket_arn.as_str(), tags2)
        .build()
        .send()
        .await
        .expect("Failed to update tag");

    // Verify updated value
    let result = ctx
        .client
        .list_tags_for_resource(bucket_arn.as_str())
        .build()
        .send()
        .await
        .expect("Failed to list tags");

    let tags = result.tags().unwrap();
    assert_eq!(
        tags.get("version"),
        Some(&"v2".to_string()),
        "Tag should be updated"
    );

    // Cleanup
    cleanup_vector_bucket(&ctx.client, &bucket).await;
}
