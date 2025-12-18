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

//! Integration tests for S3Vectors bucket policy operations.
//!
//! NOTE: These tests are ignored because the PutVectorBucketPolicy, GetVectorBucketPolicy,
//! and DeleteVectorBucketPolicy APIs are not yet implemented in the MinIO server.

use minio::s3::types::BucketName;
use minio::s3vectors::VectorsApi;
use minio_common::utils::rand_bucket_name;
use minio_common::vectors_test_context::{VectorsTestContext, cleanup_vector_bucket};

/// Creates a sample bucket policy JSON string.
fn create_test_policy(bucket_arn: &str) -> String {
    serde_json::json!({
        "Version": "2012-10-17",
        "Statement": [
            {
                "Sid": "TestPolicy",
                "Effect": "Allow",
                "Principal": "*",
                "Action": [
                    "s3vectors:GetVectorBucket",
                    "s3vectors:ListIndexes"
                ],
                "Resource": bucket_arn
            }
        ]
    })
    .to_string()
}

/// Test putting a bucket policy.
#[tokio::test]
#[ignore = "PutVectorBucketPolicy API not implemented in MinIO server"]
async fn test_put_vector_bucket_policy() {
    let ctx = VectorsTestContext::new_from_env();
    let bucket = rand_bucket_name();

    // Create vector bucket first
    let create_resp = ctx
        .client
        .create_vector_bucket(&bucket)
        .expect("Failed to create builder")
        .build()
        .send()
        .await
        .expect("Failed to create vector bucket");

    let bucket_arn = create_resp.vector_bucket_arn().unwrap();
    let policy = create_test_policy(bucket_arn.as_str());

    // Put bucket policy
    let result = ctx
        .client
        .put_vector_bucket_policy(&bucket, policy)
        .expect("Failed to create builder")
        .build()
        .send()
        .await;

    assert!(result.is_ok(), "Failed to put bucket policy: {result:?}");

    // Cleanup
    cleanup_vector_bucket(&ctx.client, &bucket).await;
}

/// Test getting a bucket policy.
#[tokio::test]
#[ignore = "GetVectorBucketPolicy API not implemented in MinIO server"]
async fn test_get_vector_bucket_policy() {
    let ctx = VectorsTestContext::new_from_env();
    let bucket = rand_bucket_name();

    // Create vector bucket first
    let create_resp = ctx
        .client
        .create_vector_bucket(&bucket)
        .expect("Failed to create builder")
        .build()
        .send()
        .await
        .expect("Failed to create vector bucket");

    let bucket_arn = create_resp.vector_bucket_arn().unwrap();
    let policy = create_test_policy(bucket_arn.as_str());

    // Put bucket policy
    ctx.client
        .put_vector_bucket_policy(&bucket, policy.clone())
        .expect("Failed to create builder")
        .build()
        .send()
        .await
        .expect("Failed to put bucket policy");

    // Get bucket policy
    let result = ctx
        .client
        .get_vector_bucket_policy(&bucket)
        .expect("Failed to create builder")
        .build()
        .send()
        .await;

    match result {
        Ok(resp) => {
            let retrieved_policy = resp.policy().unwrap();
            assert!(!retrieved_policy.is_empty(), "Expected non-empty policy");

            // Parse to verify it's valid JSON
            let policy_json = resp.policy_json().unwrap();
            assert!(
                policy_json.get("Version").is_some(),
                "Expected Version in policy"
            );
            assert!(
                policy_json.get("Statement").is_some(),
                "Expected Statement in policy"
            );
        }
        Err(e) => {
            panic!("Failed to get bucket policy: {e}");
        }
    }

    // Cleanup
    cleanup_vector_bucket(&ctx.client, &bucket).await;
}

/// Test deleting a bucket policy.
#[tokio::test]
#[ignore = "DeleteVectorBucketPolicy API not implemented in MinIO server"]
async fn test_delete_vector_bucket_policy() {
    let ctx = VectorsTestContext::new_from_env();
    let bucket = rand_bucket_name();

    // Create vector bucket first
    let create_resp = ctx
        .client
        .create_vector_bucket(&bucket)
        .expect("Failed to create builder")
        .build()
        .send()
        .await
        .expect("Failed to create vector bucket");

    let bucket_arn = create_resp.vector_bucket_arn().unwrap();
    let policy = create_test_policy(bucket_arn.as_str());

    // Put bucket policy
    ctx.client
        .put_vector_bucket_policy(&bucket, policy)
        .expect("Failed to create builder")
        .build()
        .send()
        .await
        .expect("Failed to put bucket policy");

    // Delete bucket policy
    let result = ctx
        .client
        .delete_vector_bucket_policy(&bucket)
        .expect("Failed to create builder")
        .build()
        .send()
        .await;

    assert!(result.is_ok(), "Failed to delete bucket policy: {result:?}");

    // Verify policy is deleted by trying to get it
    let get_result = ctx
        .client
        .get_vector_bucket_policy(&bucket)
        .expect("Failed to create builder")
        .build()
        .send()
        .await;

    // Should fail because policy no longer exists
    assert!(
        get_result.is_err(),
        "Expected error when getting deleted policy"
    );

    // Cleanup
    cleanup_vector_bucket(&ctx.client, &bucket).await;
}

/// Test getting a bucket policy that doesn't exist.
#[tokio::test]
#[ignore = "GetVectorBucketPolicy API not implemented in MinIO server"]
async fn test_get_nonexistent_bucket_policy() {
    let ctx = VectorsTestContext::new_from_env();
    let bucket = rand_bucket_name();

    // Create vector bucket first (without a policy)
    ctx.client
        .create_vector_bucket(&bucket)
        .expect("Failed to create builder")
        .build()
        .send()
        .await
        .expect("Failed to create vector bucket");

    // Try to get policy that doesn't exist
    let result = ctx
        .client
        .get_vector_bucket_policy(&bucket)
        .expect("Failed to create builder")
        .build()
        .send()
        .await;

    // Should fail because no policy exists
    assert!(
        result.is_err(),
        "Expected error when getting nonexistent policy"
    );

    // Cleanup
    cleanup_vector_bucket(&ctx.client, &bucket).await;
}

/// Test deleting a bucket policy that doesn't exist.
#[tokio::test]
#[ignore = "DeleteVectorBucketPolicy API not implemented in MinIO server"]
async fn test_delete_nonexistent_bucket_policy() {
    let ctx = VectorsTestContext::new_from_env();
    let bucket = rand_bucket_name();

    // Create vector bucket first (without a policy)
    ctx.client
        .create_vector_bucket(&bucket)
        .expect("Failed to create builder")
        .build()
        .send()
        .await
        .expect("Failed to create vector bucket");

    // Try to delete policy that doesn't exist
    let result = ctx
        .client
        .delete_vector_bucket_policy(&bucket)
        .expect("Failed to create builder")
        .build()
        .send()
        .await;

    // May succeed silently or fail - depends on server implementation
    // Just verify we don't panic
    let _ = result;

    // Cleanup
    cleanup_vector_bucket(&ctx.client, &bucket).await;
}

/// Test putting a bucket policy on a nonexistent bucket.
#[tokio::test]
#[ignore = "PutVectorBucketPolicy API not implemented in MinIO server"]
async fn test_put_policy_nonexistent_bucket() {
    let ctx = VectorsTestContext::new_from_env();
    let bucket = BucketName::new("nonexistent-policy-bucket").unwrap();

    let policy = create_test_policy("arn:aws:s3vectors:us-east-1:123456789012:bucket/nonexistent");

    // Try to put policy on nonexistent bucket
    let result = ctx
        .client
        .put_vector_bucket_policy(&bucket, policy)
        .expect("Failed to create builder")
        .build()
        .send()
        .await;

    // Should fail because bucket doesn't exist
    assert!(
        result.is_err(),
        "Expected error when putting policy on nonexistent bucket"
    );
}

/// Test updating a bucket policy.
#[tokio::test]
#[ignore = "PutVectorBucketPolicy API not implemented in MinIO server"]
async fn test_update_vector_bucket_policy() {
    let ctx = VectorsTestContext::new_from_env();
    let bucket = rand_bucket_name();

    // Create vector bucket first
    let create_resp = ctx
        .client
        .create_vector_bucket(&bucket)
        .expect("Failed to create builder")
        .build()
        .send()
        .await
        .expect("Failed to create vector bucket");

    let bucket_arn = create_resp.vector_bucket_arn().unwrap();

    // Put initial policy
    let initial_policy = serde_json::json!({
        "Version": "2012-10-17",
        "Statement": [
            {
                "Sid": "InitialPolicy",
                "Effect": "Allow",
                "Principal": "*",
                "Action": ["s3vectors:GetVectorBucket"],
                "Resource": bucket_arn.as_str()
            }
        ]
    })
    .to_string();

    ctx.client
        .put_vector_bucket_policy(&bucket, initial_policy)
        .expect("Failed to create builder")
        .build()
        .send()
        .await
        .expect("Failed to put initial policy");

    // Update with new policy (different actions)
    let updated_policy = serde_json::json!({
        "Version": "2012-10-17",
        "Statement": [
            {
                "Sid": "UpdatedPolicy",
                "Effect": "Allow",
                "Principal": "*",
                "Action": [
                    "s3vectors:GetVectorBucket",
                    "s3vectors:ListIndexes",
                    "s3vectors:GetIndex"
                ],
                "Resource": bucket_arn.as_str()
            }
        ]
    })
    .to_string();

    let result = ctx
        .client
        .put_vector_bucket_policy(&bucket, updated_policy)
        .expect("Failed to create builder")
        .build()
        .send()
        .await;

    assert!(result.is_ok(), "Failed to update bucket policy: {result:?}");

    // Verify the update
    let get_result = ctx
        .client
        .get_vector_bucket_policy(&bucket)
        .expect("Failed to create builder")
        .build()
        .send()
        .await
        .expect("Failed to get updated policy");

    let policy_json = get_result.policy_json().unwrap();
    let statements = policy_json
        .get("Statement")
        .and_then(|s| s.as_array())
        .unwrap();

    assert!(!statements.is_empty(), "Expected statements in policy");

    // Check that the updated policy has the new Sid
    let sid = statements[0]
        .get("Sid")
        .and_then(|s| s.as_str())
        .unwrap_or("");
    assert_eq!(sid, "UpdatedPolicy", "Expected updated policy Sid");

    // Cleanup
    cleanup_vector_bucket(&ctx.client, &bucket).await;
}
