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

//! S3 Vectors Speed Benchmark
//!
//! Benchmarks vector search performance across different CPU architectures
//! and data types, measuring QPS and recall with statistical significance.
//!
//! # Usage
//!
//! ```bash
//! cargo run --release --example s3vectors_speed
//! cargo run --release --example s3vectors_speed -- --num-vectors 50000 --dimension 768
//! cargo run --release --example s3vectors_speed -- --runs 30
//! ```

#[path = "common.rs"]
mod common;

use clap::Parser;
use common::{
    cleanup_bucket, compute_mean_std, generate_random_vectors, wait_for_index_exists,
    wait_for_index_ready, LocalVector,
};
use minio::s3::creds::StaticProvider;
use minio::s3::http::BaseUrl;
use minio::s3::types::Region;
use minio::s3vectors::{
    BucketName, DataType, DistanceMetric, IndexAlgorithm, IndexName, Vector, VectorData, VectorKey,
    VectorsApi, VectorsClient,
};
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::SeedableRng;
use std::time::Instant;

#[derive(Parser)]
#[command(name = "s3vectors_speed")]
#[command(about = "Benchmark vector search speed across CPU architectures and data types")]
struct Args {
    #[arg(long, default_value = "http://localhost:9000")]
    endpoint: String,

    #[arg(long, default_value = "minioadmin")]
    access_key: String,

    #[arg(long, default_value = "minioadmin")]
    secret_key: String,

    #[arg(long, default_value = "50000")]
    num_vectors: usize,

    #[arg(long, default_value = "768")]
    dimension: u32,

    #[arg(long, default_value = "100")]
    num_queries: usize,

    #[arg(long, default_value = "100")]
    top_k: u32,

    #[arg(long, default_value = "500")]
    batch_size: usize,

    #[arg(long, default_value = "42")]
    seed: u64,

    #[arg(long, default_value = "cosine")]
    distance: String,

    #[arg(long, default_value = "false")]
    skip_cleanup: bool,

    /// Number of benchmark runs per configuration (minimum 20 for statistical significance)
    #[arg(long, default_value = "20")]
    runs: usize,
}

/// Test configuration for a specific data type and CPU architecture combination.
struct TestConfig {
    data_type: DataType,
    cpu_arch: &'static str,
}

/// Result of a benchmark with statistics.
struct BenchResult {
    data_type: DataType,
    cpu_arch: String,
    qps_mean: f64,
    qps_std: f64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let args = Args::parse();

    if args.runs < 20 {
        eprintln!("Warning: --runs should be at least 20 for statistical significance");
    }

    let distance_metric = match args.distance.to_lowercase().as_str() {
        "cosine" => DistanceMetric::Cosine,
        "euclidean" => DistanceMetric::Euclidean,
        _ => {
            eprintln!(
                "Invalid distance metric: {}. Use 'cosine' or 'euclidean'.",
                args.distance
            );
            std::process::exit(1);
        }
    };

    // Print header
    println!("================================================================");
    println!("     S3 VECTORS SPEED BENCHMARK");
    println!("================================================================");
    println!("Endpoint: {}", args.endpoint);
    println!(
        "Dataset: {} vectors x {} dims, {} metric",
        args.num_vectors,
        args.dimension,
        args.distance.to_lowercase()
    );
    println!(
        "Queries: {}, top-{}, {} runs per config (brute-force)",
        args.num_queries, args.top_k, args.runs
    );
    println!("Batch size: {}, seed: {}", args.batch_size, args.seed);
    println!("================================================================\n");

    // Create client
    let base_url = args.endpoint.parse::<BaseUrl>()?;
    let provider = StaticProvider::new(&args.access_key, &args.secret_key, None);
    let client = VectorsClient::new(base_url, Some(provider), Some(Region::new("us-east-1")?))?;

    let bucket = BucketName::new("speed-benchmark")?;

    // Generate vectors once
    println!("Generating {} random vectors...", args.num_vectors);
    let start_gen = Instant::now();
    let mut rng = StdRng::seed_from_u64(args.seed);
    let vectors = generate_random_vectors(&mut rng, args.num_vectors, args.dimension);
    println!("  Generated in {:?}\n", start_gen.elapsed());

    // Select query vectors from the dataset
    let mut indices: Vec<usize> = (0..vectors.len()).collect();
    indices.shuffle(&mut rng);
    let query_vectors: Vec<LocalVector> = indices
        .into_iter()
        .take(args.num_queries)
        .map(|i| LocalVector {
            key: vectors[i].key.clone(),
            values: vectors[i].values.clone(),
        })
        .collect();

    // Test configurations: data type + CPU architectures
    let test_configs = vec![
        // Float32 tests
        TestConfig {
            data_type: DataType::Float32,
            cpu_arch: "scalar",
        },
        TestConfig {
            data_type: DataType::Float32,
            cpu_arch: "avx2",
        },
        TestConfig {
            data_type: DataType::Float32,
            cpu_arch: "avx512",
        },
        // Float16 tests
        TestConfig {
            data_type: DataType::Float16,
            cpu_arch: "scalar",
        },
        TestConfig {
            data_type: DataType::Float16,
            cpu_arch: "avx2",
        },
        TestConfig {
            data_type: DataType::Float16,
            cpu_arch: "avx512",
        },
        TestConfig {
            data_type: DataType::Float16,
            cpu_arch: "avx512fp16",
        },
        // BFloat16 tests
        TestConfig {
            data_type: DataType::BFloat16,
            cpu_arch: "scalar",
        },
        TestConfig {
            data_type: DataType::BFloat16,
            cpu_arch: "avx512bf16",
        },
    ];

    // Cleanup existing bucket
    println!("Cleaning up existing bucket...");
    cleanup_bucket(&client, &bucket).await;

    // Create bucket
    println!("Creating bucket...");
    client.create_vector_bucket(&bucket)?.build().send().await?;
    println!("Bucket created successfully.\n");

    let mut results: Vec<BenchResult> = Vec::new();

    // Run benchmarks for each configuration
    for config in &test_configs {
        let index_name = format!("idx-{}-{}", config.data_type, config.cpu_arch);
        let index = IndexName::new(&index_name)?;

        println!(
            "Testing: {} + {}",
            config.data_type.as_str(),
            config.cpu_arch
        );

        // Create index with specified data type (BruteForce algorithm for brute-force benchmark)
        client
            .create_index(&bucket, &index, args.dimension, distance_metric)?
            .algorithm(IndexAlgorithm::BruteForce)
            .data_type(config.data_type)
            .build()
            .send()
            .await?;
        wait_for_index_exists(&client, &bucket, &index).await?;

        // Upload vectors
        print!("  Uploading vectors...");
        let upload_start = Instant::now();
        for batch_start in (0..vectors.len()).step_by(args.batch_size) {
            let batch_end = (batch_start + args.batch_size).min(vectors.len());
            let batch: Vec<Vector> = vectors[batch_start..batch_end]
                .iter()
                .map(|v| {
                    Vector::new(
                        VectorKey::new(&v.key).unwrap(),
                        VectorData::new(v.values.clone()).unwrap(),
                    )
                })
                .collect();
            client
                .put_vectors(&bucket, &index, batch)?
                .build()
                .send()
                .await?;
        }
        println!(" {:.1}s", upload_start.elapsed().as_secs_f64());

        // Wait for index to be ready
        print!("  Waiting for index...");
        wait_for_index_ready(&client, &bucket, &index, args.num_vectors as i64).await?;
        println!(" ready");

        // Run multiple benchmark iterations
        print!("  Running {} benchmark iterations...", args.runs);
        let mut all_qps: Vec<f64> = Vec::with_capacity(args.runs);

        for run in 0..args.runs {
            let run_start = Instant::now();

            for qv in &query_vectors {
                let query_data = VectorData::new(qv.values.clone()).unwrap();
                let _resp = client
                    .query_vectors(&bucket, &index, query_data, args.top_k)?
                    .return_distance(true)
                    .cpu_arch(config.cpu_arch)
                    .build()
                    .send()
                    .await?;
            }

            let run_time = run_start.elapsed();
            let run_qps = args.num_queries as f64 / run_time.as_secs_f64();

            all_qps.push(run_qps);

            // Progress indicator
            if (run + 1) % 5 == 0 {
                print!(" {}", run + 1);
                std::io::Write::flush(&mut std::io::stdout()).ok();
            }
        }
        println!(" done");

        // Compute statistics
        let (qps_mean, qps_std) = compute_mean_std(&all_qps);

        results.push(BenchResult {
            data_type: config.data_type,
            cpu_arch: config.cpu_arch.to_string(),
            qps_mean,
            qps_std,
        });

        println!("  Result: QPS={:.1} +/- {:.1}\n", qps_mean, qps_std);

        // Delete this index before creating the next one
        let _ = client.delete_index(&bucket, &index)?.build().send().await;
    }

    // Print final results table
    println!("================================================================");
    println!("                     BENCHMARK RESULTS");
    println!("================================================================\n");
    println!("+----------+------------+------------------+");
    println!("| DataType | CPU Arch   |       QPS        |");
    println!("+----------+------------+------------------+");
    for r in &results {
        println!(
            "| {:>8} | {:>10} | {:>7.1} +/- {:>4.1} |",
            r.data_type.as_str(),
            r.cpu_arch,
            r.qps_mean,
            r.qps_std
        );
    }
    println!("+----------+------------+------------------+");
    println!("================================================================");

    // Cleanup
    if !args.skip_cleanup {
        println!("\nCleaning up...");
        cleanup_bucket(&client, &bucket).await;
        println!("Done.");
    }

    Ok(())
}
