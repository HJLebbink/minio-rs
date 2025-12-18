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

//! Integration tests for S3Vectors vector data operations.

use minio::s3::types::BucketName;
use minio::s3vectors::{DistanceMetric, IndexName, Vector, VectorData, VectorKey, VectorsApi};
use minio_common::utils::rand_bucket_name;
use minio_common::vectors_test_context::{VectorsTestContext, cleanup_vector_bucket};

/// Helper to create a test index and return the bucket and index names.
async fn setup_test_index(
    ctx: &VectorsTestContext,
    dimension: u32,
    metric: DistanceMetric,
) -> Result<(BucketName, IndexName), minio::s3::error::Error> {
    let bucket = rand_bucket_name();
    let index = IndexName::new("test-vectors-index").unwrap();

    // Create vector bucket
    ctx.client
        .create_vector_bucket(&bucket)?
        .build()
        .send()
        .await
        .expect("Failed to create vector bucket");

    // Create index
    ctx.client
        .create_index(&bucket, &index, dimension, metric)?
        .build()
        .send()
        .await
        .expect("Failed to create index");

    Ok((bucket, index))
}

/// Test putting a single vector.
#[tokio::test]
async fn test_put_single_vector() {
    let ctx = VectorsTestContext::new_from_env();
    let (bucket, index) = setup_test_index(&ctx, 128, DistanceMetric::Euclidean)
        .await
        .expect("Failed to setup test index");

    let vector = Vector::new(
        VectorKey::new("vec-001").unwrap(),
        VectorData::new(vec![0.1; 128]).unwrap(),
    );

    let result = ctx
        .client
        .put_vectors(&bucket, &index, vec![vector])
        .expect("Failed to build put_vectors request")
        .build()
        .send()
        .await;

    assert!(result.is_ok(), "Failed to put vector: {result:?}");

    // Cleanup
    cleanup_vector_bucket(&ctx.client, &bucket).await;
}

/// Test putting multiple vectors.
#[tokio::test]
async fn test_put_multiple_vectors() {
    let ctx = VectorsTestContext::new_from_env();
    let (bucket, index) = setup_test_index(&ctx, 64, DistanceMetric::Euclidean)
        .await
        .expect("Failed to setup test index");

    let vectors: Vec<Vector> = (0..10)
        .map(|i| {
            Vector::new(
                VectorKey::new(format!("vec-{:03}", i)).unwrap(),
                VectorData::new(vec![i as f32 * 0.1; 64]).unwrap(),
            )
        })
        .collect();

    let result = ctx
        .client
        .put_vectors(&bucket, &index, vectors)
        .expect("Failed to build put_vectors request")
        .build()
        .send()
        .await;

    assert!(result.is_ok(), "Failed to put multiple vectors: {result:?}");

    // Cleanup
    cleanup_vector_bucket(&ctx.client, &bucket).await;
}

/// Test putting vectors with metadata.
#[tokio::test]
async fn test_put_vectors_with_metadata() {
    let ctx = VectorsTestContext::new_from_env();
    let (bucket, index) = setup_test_index(&ctx, 128, DistanceMetric::Euclidean)
        .await
        .expect("Failed to setup test index");

    let vectors = vec![
        Vector::with_metadata(
            VectorKey::new("doc-001").unwrap(),
            VectorData::new(vec![0.1; 128]).unwrap(),
            serde_json::json!({
                "title": "Document 1",
                "category": "science"
            }),
        ),
        Vector::with_metadata(
            VectorKey::new("doc-002").unwrap(),
            VectorData::new(vec![0.2; 128]).unwrap(),
            serde_json::json!({
                "title": "Document 2",
                "category": "technology"
            }),
        ),
    ];

    let result = ctx
        .client
        .put_vectors(&bucket, &index, vectors)
        .expect("Failed to build put_vectors request")
        .build()
        .send()
        .await;

    assert!(
        result.is_ok(),
        "Failed to put vectors with metadata: {result:?}"
    );

    // Cleanup
    cleanup_vector_bucket(&ctx.client, &bucket).await;
}

/// Test querying vectors with Euclidean distance.
#[tokio::test]
async fn test_query_vectors_euclidean() {
    let ctx = VectorsTestContext::new_from_env();
    let (bucket, index) = setup_test_index(&ctx, 128, DistanceMetric::Euclidean)
        .await
        .expect("Failed to setup test index");

    // Put some vectors
    let vectors: Vec<Vector> = (0..5)
        .map(|i| {
            Vector::new(
                VectorKey::new(format!("vec-{:03}", i)).unwrap(),
                VectorData::new(vec![i as f32 * 0.1; 128]).unwrap(),
            )
        })
        .collect();

    ctx.client
        .put_vectors(&bucket, &index, vectors)
        .expect("Failed to build put_vectors request")
        .build()
        .send()
        .await
        .expect("Failed to put vectors");

    // Query for similar vectors
    let query_vector = VectorData::new(vec![0.05; 128]).unwrap();
    let result = ctx
        .client
        .query_vectors(&bucket, &index, query_vector, 3)
        .unwrap()
        .return_distance(true)
        .build()
        .send()
        .await;

    match result {
        Ok(resp) => {
            let vectors = resp.vectors().unwrap();
            assert!(!vectors.is_empty(), "Expected query results");
            assert!(vectors.len() <= 3, "Expected at most 3 results");

            // Each result should have a distance
            for vec in &vectors {
                assert!(vec.distance.is_some(), "Expected distance in result");
            }
        }
        Err(e) => {
            panic!("Failed to query vectors: {e}");
        }
    }

    // Cleanup
    cleanup_vector_bucket(&ctx.client, &bucket).await;
}

/// Test querying vectors with Cosine distance.
#[tokio::test]
async fn test_query_vectors_cosine() {
    let ctx = VectorsTestContext::new_from_env();
    let (bucket, index) = setup_test_index(&ctx, 64, DistanceMetric::Cosine)
        .await
        .expect("Failed to setup test index");

    // Put some vectors (non-zero for cosine)
    let vectors: Vec<Vector> = (0..5)
        .map(|i| {
            Vector::new(
                VectorKey::new(format!("vec-{:03}", i)).unwrap(),
                VectorData::new(vec![1.0 + i as f32 * 0.1; 64]).unwrap(),
            )
        })
        .collect();

    ctx.client
        .put_vectors(&bucket, &index, vectors)
        .expect("Failed to build put_vectors request")
        .build()
        .send()
        .await
        .expect("Failed to put vectors");

    // Query for similar vectors
    let query_vector = VectorData::new(vec![1.0; 64]).unwrap();
    let result = ctx
        .client
        .query_vectors(&bucket, &index, query_vector, 3)
        .unwrap()
        .return_distance(true)
        .build()
        .send()
        .await;

    match result {
        Ok(resp) => {
            let vectors = resp.vectors().unwrap();
            assert!(!vectors.is_empty(), "Expected query results");
        }
        Err(e) => {
            panic!("Failed to query vectors with cosine: {e}");
        }
    }

    // Cleanup
    cleanup_vector_bucket(&ctx.client, &bucket).await;
}

/// Test querying vectors with metadata return.
#[tokio::test]
async fn test_query_vectors_return_metadata() {
    let ctx = VectorsTestContext::new_from_env();
    let (bucket, index) = setup_test_index(&ctx, 64, DistanceMetric::Euclidean)
        .await
        .expect("Failed to setup test index");

    // Put vectors with metadata
    let vectors = vec![
        Vector::with_metadata(
            VectorKey::new("doc-001").unwrap(),
            VectorData::new(vec![0.1; 64]).unwrap(),
            serde_json::json!({"category": "A"}),
        ),
        Vector::with_metadata(
            VectorKey::new("doc-002").unwrap(),
            VectorData::new(vec![0.2; 64]).unwrap(),
            serde_json::json!({"category": "B"}),
        ),
    ];

    ctx.client
        .put_vectors(&bucket, &index, vectors)
        .expect("Failed to build put_vectors request")
        .build()
        .send()
        .await
        .expect("Failed to put vectors");

    // Query with metadata return
    let query_vector = VectorData::new(vec![0.15; 64]).unwrap();
    let result = ctx
        .client
        .query_vectors(&bucket, &index, query_vector, 5)
        .unwrap()
        .return_metadata(true)
        .build()
        .send()
        .await;

    match result {
        Ok(resp) => {
            let vectors = resp.vectors().unwrap();
            assert!(!vectors.is_empty(), "Expected query results");

            // Results should have metadata
            for vec in &vectors {
                assert!(vec.metadata.is_some(), "Expected metadata in result");
            }
        }
        Err(e) => {
            panic!("Failed to query vectors with metadata: {e}");
        }
    }

    // Cleanup
    cleanup_vector_bucket(&ctx.client, &bucket).await;
}

/// Test getting vectors by key.
#[tokio::test]
async fn test_get_vectors() {
    let ctx = VectorsTestContext::new_from_env();
    let (bucket, index) = setup_test_index(&ctx, 128, DistanceMetric::Euclidean)
        .await
        .expect("Failed to setup test index");

    // Put vectors
    let vectors = vec![
        Vector::new(
            VectorKey::new("get-001").unwrap(),
            VectorData::new(vec![0.1; 128]).unwrap(),
        ),
        Vector::new(
            VectorKey::new("get-002").unwrap(),
            VectorData::new(vec![0.2; 128]).unwrap(),
        ),
    ];

    ctx.client
        .put_vectors(&bucket, &index, vectors)
        .expect("Failed to build put_vectors request")
        .build()
        .send()
        .await
        .expect("Failed to put vectors");

    // Get vectors by key
    let keys = vec![
        VectorKey::new("get-001").unwrap(),
        VectorKey::new("get-002").unwrap(),
    ];

    let result = ctx
        .client
        .get_vectors(&bucket, &index, keys)
        .unwrap()
        .return_data(true)
        .return_metadata(true)
        .build()
        .send()
        .await;

    match result {
        Ok(resp) => {
            let vectors = resp.vectors().unwrap();
            assert_eq!(vectors.len(), 2, "Expected 2 vectors");

            // Check keys match
            let keys_found: Vec<&str> = vectors.iter().map(|v| v.key.as_str()).collect();
            assert!(keys_found.contains(&"get-001"));
            assert!(keys_found.contains(&"get-002"));
        }
        Err(e) => {
            panic!("Failed to get vectors: {e}");
        }
    }

    // Cleanup
    cleanup_vector_bucket(&ctx.client, &bucket).await;
}

/// Test getting a non-existent vector.
#[tokio::test]
async fn test_get_nonexistent_vector() {
    let ctx = VectorsTestContext::new_from_env();
    let (bucket, index) = setup_test_index(&ctx, 128, DistanceMetric::Euclidean)
        .await
        .expect("Failed to setup test index");

    // Try to get a vector that doesn't exist
    let keys = vec![VectorKey::new("nonexistent-key").unwrap()];

    let result = ctx
        .client
        .get_vectors(&bucket, &index, keys)
        .expect("Failed to build get_vectors request")
        .build()
        .send()
        .await;

    // This might return empty results or error depending on server implementation
    match result {
        Ok(resp) => {
            let vectors = resp.vectors().unwrap();
            // Either empty or the vector should not be found
            assert!(
                vectors.is_empty() || vectors.iter().all(|v| v.data.is_none()),
                "Expected no data for nonexistent vector"
            );
        }
        Err(_) => {
            // Also acceptable - server might return error for nonexistent vectors
        }
    }

    // Cleanup
    cleanup_vector_bucket(&ctx.client, &bucket).await;
}

/// Test listing vectors.
#[tokio::test]
async fn test_list_vectors() {
    let ctx = VectorsTestContext::new_from_env();
    let (bucket, index) = setup_test_index(&ctx, 64, DistanceMetric::Euclidean)
        .await
        .expect("Failed to setup test index");

    // Put vectors
    let vectors: Vec<Vector> = (0..5)
        .map(|i| {
            Vector::new(
                VectorKey::new(format!("list-{:03}", i)).unwrap(),
                VectorData::new(vec![i as f32 * 0.1; 64]).unwrap(),
            )
        })
        .collect();

    ctx.client
        .put_vectors(&bucket, &index, vectors)
        .expect("Failed to build put_vectors request")
        .build()
        .send()
        .await
        .expect("Failed to put vectors");

    // List vectors
    let result = ctx
        .client
        .list_vectors(&bucket, &index)
        .expect("Failed to build list_vectors request")
        .build()
        .send()
        .await;

    match result {
        Ok(resp) => {
            let vectors = resp.vectors().unwrap();
            assert!(vectors.len() >= 5, "Expected at least 5 vectors");
        }
        Err(e) => {
            panic!("Failed to list vectors: {e}");
        }
    }

    // Cleanup
    cleanup_vector_bucket(&ctx.client, &bucket).await;
}

/// Test listing vectors with pagination.
#[tokio::test]
async fn test_list_vectors_pagination() {
    let ctx = VectorsTestContext::new_from_env();
    let (bucket, index) = setup_test_index(&ctx, 64, DistanceMetric::Euclidean)
        .await
        .expect("Failed to setup test index");

    // Put vectors
    let vectors: Vec<Vector> = (0..10)
        .map(|i| {
            Vector::new(
                VectorKey::new(format!("page-{:03}", i)).unwrap(),
                VectorData::new(vec![i as f32 * 0.1; 64]).unwrap(),
            )
        })
        .collect();

    ctx.client
        .put_vectors(&bucket, &index, vectors)
        .expect("Failed to build put_vectors request")
        .build()
        .send()
        .await
        .expect("Failed to put vectors");

    // List with max_results
    let result = ctx
        .client
        .list_vectors(&bucket, &index)
        .unwrap()
        .max_results(3u32)
        .build()
        .send()
        .await;

    match result {
        Ok(resp) => {
            let vectors = resp.vectors().unwrap();
            assert!(vectors.len() <= 3, "Expected at most 3 vectors");
        }
        Err(e) => {
            panic!("Failed to list vectors with pagination: {e}");
        }
    }

    // Cleanup
    cleanup_vector_bucket(&ctx.client, &bucket).await;
}

/// Test listing vectors with data return.
#[tokio::test]
async fn test_list_vectors_return_data() {
    let ctx = VectorsTestContext::new_from_env();
    let (bucket, index) = setup_test_index(&ctx, 64, DistanceMetric::Euclidean)
        .await
        .expect("Failed to setup test index");

    // Put vectors
    let vectors = vec![Vector::new(
        VectorKey::new("data-001").unwrap(),
        VectorData::new(vec![0.5; 64]).unwrap(),
    )];

    ctx.client
        .put_vectors(&bucket, &index, vectors)
        .expect("Failed to build put_vectors request")
        .build()
        .send()
        .await
        .expect("Failed to put vectors");

    // List with return_data
    let result = ctx
        .client
        .list_vectors(&bucket, &index)
        .unwrap()
        .return_data(true)
        .build()
        .send()
        .await;

    match result {
        Ok(resp) => {
            let vectors = resp.vectors().unwrap();
            assert!(!vectors.is_empty(), "Expected vectors");

            // Results should have data
            for vec in &vectors {
                assert!(vec.data.is_some(), "Expected data in result");
            }
        }
        Err(e) => {
            panic!("Failed to list vectors with data: {e}");
        }
    }

    // Cleanup
    cleanup_vector_bucket(&ctx.client, &bucket).await;
}

/// Test deleting vectors.
#[tokio::test]
async fn test_delete_vectors() {
    let ctx = VectorsTestContext::new_from_env();
    let (bucket, index) = setup_test_index(&ctx, 128, DistanceMetric::Euclidean)
        .await
        .expect("Failed to setup test index");

    // Put vectors
    let vectors = vec![
        Vector::new(
            VectorKey::new("del-001").unwrap(),
            VectorData::new(vec![0.1; 128]).unwrap(),
        ),
        Vector::new(
            VectorKey::new("del-002").unwrap(),
            VectorData::new(vec![0.2; 128]).unwrap(),
        ),
        Vector::new(
            VectorKey::new("del-003").unwrap(),
            VectorData::new(vec![0.3; 128]).unwrap(),
        ),
    ];

    ctx.client
        .put_vectors(&bucket, &index, vectors)
        .expect("Failed to build put_vectors request")
        .build()
        .send()
        .await
        .expect("Failed to put vectors");

    // Delete some vectors
    let keys_to_delete = vec![
        VectorKey::new("del-001").unwrap(),
        VectorKey::new("del-002").unwrap(),
    ];

    let result = ctx
        .client
        .delete_vectors(&bucket, &index, keys_to_delete)
        .expect("Failed to build delete_vectors request")
        .build()
        .send()
        .await;

    assert!(result.is_ok(), "Failed to delete vectors: {result:?}");

    // Verify deleted vectors are gone
    let remaining = ctx
        .client
        .list_vectors(&bucket, &index)
        .expect("Failed to build list_vectors request")
        .build()
        .send()
        .await
        .expect("Failed to list vectors");

    let remaining_keys: Vec<String> = remaining
        .vectors()
        .unwrap()
        .iter()
        .map(|v| v.key.as_str().to_string())
        .collect();

    assert!(
        !remaining_keys.contains(&"del-001".to_string()),
        "del-001 should be deleted"
    );
    assert!(
        !remaining_keys.contains(&"del-002".to_string()),
        "del-002 should be deleted"
    );

    // Cleanup
    cleanup_vector_bucket(&ctx.client, &bucket).await;
}

/// Test updating vectors (put with same key).
#[tokio::test]
async fn test_update_vectors() {
    let ctx = VectorsTestContext::new_from_env();
    let (bucket, index) = setup_test_index(&ctx, 64, DistanceMetric::Euclidean)
        .await
        .expect("Failed to setup test index");

    // Put initial vector
    let vectors = vec![Vector::new(
        VectorKey::new("update-001").unwrap(),
        VectorData::new(vec![0.1; 64]).unwrap(),
    )];

    ctx.client
        .put_vectors(&bucket, &index, vectors)
        .expect("Failed to build put_vectors request")
        .build()
        .send()
        .await
        .expect("Failed to put initial vector");

    // Update with new values
    let updated_vectors = vec![Vector::new(
        VectorKey::new("update-001").unwrap(),
        VectorData::new(vec![0.9; 64]).unwrap(),
    )];

    let result = ctx
        .client
        .put_vectors(&bucket, &index, updated_vectors)
        .expect("Failed to build put_vectors request")
        .build()
        .send()
        .await;

    assert!(result.is_ok(), "Failed to update vector: {result:?}");

    // Verify the update
    let keys = vec![VectorKey::new("update-001").unwrap()];
    let get_result = ctx
        .client
        .get_vectors(&bucket, &index, keys)
        .unwrap()
        .return_data(true)
        .build()
        .send()
        .await;

    match get_result {
        Ok(resp) => {
            let vectors = resp.vectors().unwrap();
            assert_eq!(vectors.len(), 1);

            if let Some(data) = &vectors[0].data {
                // Check that values were updated (should be 0.9, not 0.1)
                assert!(
                    (data.float32[0] - 0.9).abs() < 0.01,
                    "Vector should be updated to 0.9"
                );
            }
        }
        Err(e) => {
            panic!("Failed to get updated vector: {e}");
        }
    }

    // Cleanup
    cleanup_vector_bucket(&ctx.client, &bucket).await;
}

/// Test putting maximum batch size vectors.
#[tokio::test]
async fn test_put_max_batch_vectors() {
    let ctx = VectorsTestContext::new_from_env();
    let (bucket, index) = setup_test_index(&ctx, 32, DistanceMetric::Euclidean)
        .await
        .expect("Failed to setup test index");

    // Put max batch size (500 vectors)
    let vectors: Vec<Vector> = (0..500)
        .map(|i| {
            Vector::new(
                VectorKey::new(format!("batch-{:04}", i)).unwrap(),
                VectorData::new(vec![i as f32 * 0.001; 32]).unwrap(),
            )
        })
        .collect();

    let result = ctx
        .client
        .put_vectors(&bucket, &index, vectors)
        .expect("Failed to build put_vectors request")
        .build()
        .send()
        .await;

    assert!(
        result.is_ok(),
        "Failed to put max batch vectors: {result:?}"
    );

    // Cleanup
    cleanup_vector_bucket(&ctx.client, &bucket).await;
}

/// Test querying empty index.
#[tokio::test]
async fn test_query_empty_index() {
    let ctx = VectorsTestContext::new_from_env();
    let (bucket, index) = setup_test_index(&ctx, 64, DistanceMetric::Euclidean)
        .await
        .expect("Failed to setup test index");

    // Query empty index
    let query_vector = VectorData::new(vec![0.5; 64]).unwrap();
    let result = ctx
        .client
        .query_vectors(&bucket, &index, query_vector, 10)
        .expect("Failed to build query_vectors request")
        .build()
        .send()
        .await;

    match result {
        Ok(resp) => {
            let vectors = resp.vectors().unwrap();
            assert!(
                vectors.is_empty(),
                "Expected empty results from empty index"
            );
        }
        Err(e) => {
            panic!("Failed to query empty index: {e}");
        }
    }

    // Cleanup
    cleanup_vector_bucket(&ctx.client, &bucket).await;
}
