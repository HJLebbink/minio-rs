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

//! S3 Vectors Recall Benchmark
//!
//! Measures recall quality of S3 Vectors ANN search comparing HNSW and Vamana
//! algorithms against brute-force ground truth.
//!
//! # Usage
//!
//! ```bash
//! cargo run --release --example s3vectors_recall
//! cargo run --release --example s3vectors_recall -- --num-vectors 50000 --dimension 256
//! ```

#[path = "common.rs"]
mod common;

use clap::Parser;
use common::{
    calculate_recall, cleanup_bucket, compute_mean_std, generate_random_vectors,
    wait_for_index_exists, LocalVector,
};
use minio::s3::creds::StaticProvider;
use minio::s3::http::BaseUrl;
use minio::s3::types::Region;
use minio::s3vectors::{
    BucketName, DistanceMetric, HnswConfig, IndexAlgorithm, IndexName, IndexStatus, VamanaConfig,
    Vector, VectorData, VectorKey, VectorsApi, VectorsClient,
};
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::SeedableRng;
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Parser)]
#[command(name = "s3vectors_recall")]
#[command(about = "Benchmark recall quality of S3 Vectors ANN search")]
struct Args {
    #[arg(long, default_value = "http://localhost:9000")]
    endpoint: String,

    #[arg(long, default_value = "minioadmin")]
    access_key: String,

    #[arg(long, default_value = "minioadmin")]
    secret_key: String,

    #[arg(long, default_value = "10000")]
    num_vectors: usize,

    #[arg(long, default_value = "1024")]
    dimension: u32,

    #[arg(long, default_value = "100")]
    num_queries: usize,

    #[arg(long, default_value = "100")]
    top_k: u32,

    #[arg(long, default_value = "250")]
    batch_size: usize,

    #[arg(long, default_value = "42")]
    seed: u64,

    #[arg(long, default_value = "cosine")]
    distance: String,

    #[arg(long, default_value = "false")]
    skip_cleanup: bool,

    #[arg(long, default_value = "true")]
    random_queries: bool,

    /// HNSW: M parameter (connections per node)
    #[arg(long, default_value = "32")]
    hnsw_m: u32,

    /// HNSW: efConstruction parameter (construction search depth)
    #[arg(long, default_value = "128")]
    hnsw_ef_construction: u32,

    /// HNSW: efSearch parameter (query search depth)
    #[arg(long, default_value = "256")]
    hnsw_ef_search: u32,

    /// Vamana: L parameter (search queue size)
    #[arg(long, default_value = "64")]
    vamana_l: u32,

    /// Vamana: R parameter (max neighbors per node)
    #[arg(long, default_value = "64")]
    vamana_r: u32,

    /// Vamana: Alpha parameter (diversity factor)
    #[arg(long, default_value = "1.2")]
    vamana_alpha: f32,

    /// CPU architecture to use (for benchmarking different SIMD implementations)
    /// Options: auto, scalar, avx2, avx512
    #[arg(long, default_value = "auto")]
    cpu_arch: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let args = Args::parse();

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
    println!("     S3 VECTORS RECALL: HNSW vs Vamana");
    println!("================================================================");
    println!(
        "Dataset: {} vectors × {} dims, {} metric",
        args.num_vectors,
        args.dimension,
        args.distance.to_lowercase()
    );
    println!(
        "Queries: {} queries, top-{}, seed={}, arch={}",
        args.num_queries, args.top_k, args.seed, args.cpu_arch
    );
    println!();

    // Build configs
    let hnsw_config =
        HnswConfig::new(args.hnsw_m, args.hnsw_ef_construction, args.hnsw_ef_search);
    let vamana_config = VamanaConfig::new(args.vamana_l, args.vamana_r, args.vamana_alpha);

    // Display parameters
    println!(
        "HNSW:   M={} efConstruction={} efSearch={}",
        args.hnsw_m, args.hnsw_ef_construction, args.hnsw_ef_search
    );
    println!(
        "Vamana: L={} R={} Alpha={:.1}",
        args.vamana_l, args.vamana_r, args.vamana_alpha
    );
    println!("================================================================\n");

    // Create client
    let base_url = args.endpoint.parse::<BaseUrl>()?;
    let provider = StaticProvider::new(&args.access_key, &args.secret_key, None);
    let client = VectorsClient::new(base_url, Some(provider), Some(Region::new("us-east-1")?))?;

    let bucket = BucketName::new("recall-benchmark")?;
    let index_hnsw = IndexName::new("benchmark-index-hnsw")?;
    let index_vamana = IndexName::new("benchmark-index-vamana")?;

    // Step 1: Create bucket and indexes
    println!("Step 1: Creating vector bucket and indexes...");

    // Cleanup existing
    cleanup_bucket(&client, &bucket).await;

    client.create_vector_bucket(&bucket)?.build().send().await?;

    // Create HNSW index
    client
        .create_index(&bucket, &index_hnsw, args.dimension, distance_metric)?
        .algorithm(IndexAlgorithm::Hnsw)
        .hnsw_config(hnsw_config)
        .build()
        .send()
        .await?;
    wait_for_index_exists(&client, &bucket, &index_hnsw).await?;

    // Create Vamana index
    client
        .create_index(&bucket, &index_vamana, args.dimension, distance_metric)?
        .algorithm(IndexAlgorithm::Vamana)
        .vamana_config(vamana_config)
        .build()
        .send()
        .await?;
    wait_for_index_exists(&client, &bucket, &index_vamana).await?;

    println!("  Created bucket '{}' with HNSW and Vamana indexes", bucket);

    // Step 2: Generate vectors
    println!(
        "\nStep 2: Generating {} random vectors...",
        args.num_vectors
    );
    let start_gen = Instant::now();
    let mut rng = StdRng::seed_from_u64(args.seed);
    let vectors = generate_random_vectors(&mut rng, args.num_vectors, args.dimension);
    println!("  Generated in {:?}", start_gen.elapsed());

    // Step 3: Upload vectors to both indexes in parallel
    // Each index gets its own upload task so we can measure build time independently
    println!(
        "\nStep 3: Uploading vectors to both indexes in parallel (batch size {})...",
        args.batch_size
    );

    // Convert vectors to API format once, share between tasks
    let api_vectors: Arc<Vec<Vector>> = Arc::new(
        vectors
            .iter()
            .map(|v| {
                Vector::new(
                    VectorKey::new(&v.key).unwrap(),
                    VectorData::new(v.values.clone()).unwrap(),
                )
            })
            .collect(),
    );

    let batch_size = args.batch_size;
    let num_batches = args.num_vectors.div_ceil(batch_size);

    // Upload vectors and poll until index is ready
    async fn upload_and_wait(
        client: VectorsClient,
        bucket: BucketName,
        index: IndexName,
        vecs: Arc<Vec<Vector>>,
        batch_size: usize,
        num_batches: usize,
    ) -> Duration {
        let start = Instant::now();
        for batch_idx in 0..num_batches {
            let batch_start = batch_idx * batch_size;
            let batch_end = (batch_start + batch_size).min(vecs.len());
            client
                .put_vectors(&bucket, &index, vecs[batch_start..batch_end].to_vec())
                .unwrap()
                .build()
                .send()
                .await
                .unwrap();
        }
        loop {
            let is_active = client
                .get_index(&bucket, &index)
                .unwrap()
                .build()
                .send()
                .await
                .ok()
                .and_then(|r| r.index().ok())
                .is_some_and(|i| matches!(i.status, IndexStatus::Active));
            if is_active {
                return start.elapsed();
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        }
    }

    let hnsw_task = tokio::spawn(upload_and_wait(
        client.clone(),
        bucket.clone(),
        index_hnsw.clone(),
        api_vectors.clone(),
        batch_size,
        num_batches,
    ));
    let vamana_task = tokio::spawn(upload_and_wait(
        client.clone(),
        bucket.clone(),
        index_vamana.clone(),
        api_vectors.clone(),
        batch_size,
        num_batches,
    ));

    let (hnsw_result, vamana_result) = tokio::join!(hnsw_task, vamana_task);
    let hnsw_build_secs = hnsw_result.unwrap().as_secs_f64();
    let vamana_build_secs = vamana_result.unwrap().as_secs_f64();

    println!("  HNSW:   {:.2}s (upload + build)", hnsw_build_secs);
    println!("  Vamana: {:.2}s (upload + build)", vamana_build_secs);

    // Step 5: Select query vectors
    let query_vectors = if args.random_queries {
        println!(
            "\nStep 5: Generating {} random query vectors...",
            args.num_queries
        );
        generate_random_vectors(&mut rng, args.num_queries, args.dimension)
    } else {
        println!(
            "\nStep 5: Selecting {} vectors from dataset as queries...",
            args.num_queries
        );
        let mut indices: Vec<usize> = (0..vectors.len()).collect();
        indices.shuffle(&mut rng);
        indices
            .into_iter()
            .take(args.num_queries)
            .map(|i| LocalVector {
                key: vectors[i].key.clone(),
                values: vectors[i].values.clone(),
            })
            .collect()
    };

    // Step 6: Run benchmark
    println!("\nStep 6: Running recall benchmark...");

    let mut hnsw_recalls = Vec::with_capacity(args.num_queries);
    let mut vamana_recalls = Vec::with_capacity(args.num_queries);
    let mut total_hnsw_time = Duration::ZERO;
    let mut total_vamana_time = Duration::ZERO;
    let mut total_bf_time = Duration::ZERO;

    for (i, qv) in query_vectors.iter().enumerate() {
        let query_data = VectorData::new(qv.values.clone()).unwrap();

        // Server brute-force = ground truth
        let start = Instant::now();
        let bf_resp = client
            .query_vectors(&bucket, &index_hnsw, query_data.clone(), args.top_k)?
            .brute_force(true)
            .return_distance(true)
            .build()
            .send()
            .await?;
        total_bf_time += start.elapsed();
        let ground_truth: Vec<String> = bf_resp
            .vectors()?
            .iter()
            .map(|v| v.key.as_str().to_string())
            .collect();

        // HNSW query
        let start = Instant::now();
        let hnsw_resp = client
            .query_vectors(&bucket, &index_hnsw, query_data.clone(), args.top_k)?
            .return_distance(true)
            .build()
            .send()
            .await?;
        total_hnsw_time += start.elapsed();
        let hnsw_results: Vec<String> = hnsw_resp
            .vectors()?
            .iter()
            .map(|v| v.key.as_str().to_string())
            .collect();
        hnsw_recalls.push(calculate_recall(&ground_truth, &hnsw_results));

        // Vamana query
        let start = Instant::now();
        let vamana_resp = client
            .query_vectors(&bucket, &index_vamana, query_data, args.top_k)?
            .return_distance(true)
            .build()
            .send()
            .await?;
        total_vamana_time += start.elapsed();
        let vamana_results: Vec<String> = vamana_resp
            .vectors()?
            .iter()
            .map(|v| v.key.as_str().to_string())
            .collect();
        vamana_recalls.push(calculate_recall(&ground_truth, &vamana_results));

        if (i + 1) % 10 == 0 || i == args.num_queries - 1 {
            println!(
                "  Query {}/{}: HNSW={:.1}% Vamana={:.1}%",
                i + 1,
                args.num_queries,
                hnsw_recalls[i],
                vamana_recalls[i]
            );
        }
    }

    // Compute stats
    let (hnsw_mean, hnsw_std) = compute_mean_std(&hnsw_recalls);
    let (vamana_mean, vamana_std) = compute_mean_std(&vamana_recalls);

    let hnsw_qps = args.num_queries as f64 / total_hnsw_time.as_secs_f64();
    let vamana_qps = args.num_queries as f64 / total_vamana_time.as_secs_f64();
    let bf_qps = args.num_queries as f64 / total_bf_time.as_secs_f64();

    // Print results
    println!("\n================================================================");
    println!("                   BENCHMARK RESULTS");
    println!("================================================================\n");

    // Display index parameters
    println!("Index Parameters:");
    println!("  HNSW:   M={}, efConstruction={}, efSearch={}",
        args.hnsw_m, args.hnsw_ef_construction, args.hnsw_ef_search);
    println!("  Vamana: L={}, R={}, Alpha={:.1}",
        args.vamana_l, args.vamana_r, args.vamana_alpha);
    println!();

    println!("┌─────────────┬───────────┬───────────┬──────────┬────────────┐");
    println!("│ Algorithm   │  Recall   │  Std Dev  │   QPS    │ Build Time │");
    println!("├─────────────┼───────────┼───────────┼──────────┼────────────┤");
    println!(
        "│ {:11} │ {:>7.2}%  │ {:>7.2}%  │ {:>8.1} │ {:>8.2}s │",
        "HNSW", hnsw_mean, hnsw_std, hnsw_qps, hnsw_build_secs
    );
    println!(
        "│ {:11} │ {:>7.2}%  │ {:>7.2}%  │ {:>8.1} │ {:>8.2}s │",
        "Vamana", vamana_mean, vamana_std, vamana_qps, vamana_build_secs
    );
    println!(
        "│ {:11} │ {:>9} │ {:>9} │ {:>8.1} │ {:>10} │",
        "Brute-force", "100.00%", "-", bf_qps, "-"
    );
    println!("└─────────────┴───────────┴───────────┴──────────┴────────────┘\n");
    println!(
        "Speedup vs Brute-force:  HNSW {:.1}x  Vamana {:.1}x",
        hnsw_qps / bf_qps,
        vamana_qps / bf_qps
    );
    println!("================================================================");

    // Cleanup
    if !args.skip_cleanup {
        println!("\nCleaning up...");
        cleanup_bucket(&client, &bucket).await;
        println!("Done.");
    }

    Ok(())
}
