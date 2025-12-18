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

//! Common utilities for S3 Vectors benchmarks.

use minio::s3vectors::{BucketName, IndexName, IndexStatus, VectorsApi, VectorsClient};
use rand::rngs::StdRng;
use rand::Rng;
use std::collections::HashSet;
use std::time::Duration;

/// Local vector storage for benchmarks
pub struct LocalVector {
    pub key: String,
    pub values: Vec<f32>,
}

/// Generate random normalized vectors
pub fn generate_random_vectors(rng: &mut StdRng, count: usize, dimension: u32) -> Vec<LocalVector> {
    (0..count)
        .map(|i| {
            let mut values: Vec<f32> = (0..dimension)
                .map(|_| rng.random::<f32>() * 2.0 - 1.0)
                .collect();
            let norm: f32 = values.iter().map(|v| v * v).sum::<f32>().sqrt();
            if norm > 0.0 {
                values.iter_mut().for_each(|v| *v /= norm);
            }
            LocalVector {
                key: format!("vec-{:06}", i),
                values,
            }
        })
        .collect()
}

/// Calculate recall percentage between ground truth and approximate results
pub fn calculate_recall(ground_truth: &[String], approx_results: &[String]) -> f64 {
    if ground_truth.is_empty() {
        return 0.0;
    }
    let truth_set: HashSet<_> = ground_truth.iter().collect();
    let matches = approx_results
        .iter()
        .filter(|k| truth_set.contains(k))
        .count();
    (matches as f64 / ground_truth.len() as f64) * 100.0
}

/// Compute mean and standard deviation
pub fn compute_mean_std(values: &[f64]) -> (f64, f64) {
    if values.is_empty() {
        return (0.0, 0.0);
    }
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let std_dev = if values.len() > 1 {
        let sum_squares: f64 = values.iter().map(|v| (v - mean).powi(2)).sum();
        (sum_squares / (values.len() - 1) as f64).sqrt()
    } else {
        0.0
    };
    (mean, std_dev)
}

/// Wait for index to exist after creation
pub async fn wait_for_index_exists(
    client: &VectorsClient,
    bucket: &BucketName,
    index: &IndexName,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    loop {
        if let Ok(resp) = client.get_index(bucket, index)?.build().send().await {
            if resp.index().is_ok() {
                return Ok(());
            }
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

/// Wait for index to be ready by polling until vector_count matches expected
pub async fn wait_for_index_ready(
    client: &VectorsClient,
    bucket: &BucketName,
    index: &IndexName,
    expected_count: i64,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    loop {
        let response = client.get_index(bucket, index)?.build().send().await?;
        let index_info = response.index()?;
        if index_info.status == IndexStatus::Active && index_info.vector_count >= expected_count {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

/// Clean up a vector bucket and all its indexes
pub async fn cleanup_bucket(client: &VectorsClient, bucket: &BucketName) {
    // List and delete all indexes
    if let Ok(builder) = client.list_indexes(bucket) {
        if let Ok(list_resp) = builder.build().send().await {
            if let Ok(indexes) = list_resp.indexes() {
                for idx in indexes {
                    if let Ok(del_builder) = client.delete_index(bucket, &idx.index) {
                        let _ = del_builder.build().send().await;
                    }
                }
            }
        }
    }
    // Delete bucket
    if let Ok(del_builder) = client.delete_vector_bucket(bucket) {
        let _ = del_builder.build().send().await;
    }
}
