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

//! S3 Vectors HNSW Parameter Sweep (MinIO Extension)
//!
//! This example benchmarks recall and performance of S3 Vectors across different
//! HNSW algorithm parameters. It compares approximate nearest neighbor (ANN)
//! search results against brute-force ground truth.
//!
//! **MinIO Extension**: HNSW parameters (m, ef_construction, ef_search) are
//! MinIO-specific extensions. AWS S3 Vectors does NOT support user-configurable
//! HNSW parameters - they are managed internally.
//!
//! # What is HNSW?
//!
//! HNSW (Hierarchical Navigable Small World) is an algorithm for approximate
//! nearest neighbor search. Key parameters:
//!
//! - **m**: Number of bi-directional links per node (higher = better recall, more memory)
//! - **ef_construction**: Search depth during index building (higher = better index quality)
//! - **ef_search**: Search depth during queries (higher = better recall, slower queries)
//!
//! # What is Recall?
//!
//! Recall measures the overlap between exact (brute-force) search results and
//! approximate (index) search results:
//! ```text
//! Recall = |BruteForce ∩ Index| / |BruteForce| * 100%
//! ```
//!
//! The query vector does NOT need to be present in the database. Recall measures
//! how well the index approximates the true nearest neighbors.
//!
//! For example, if brute-force returns [A, B, C, D, E] and the index returns [A, B, X, Y, Z],
//! recall = 2/5 = 40% (only A and B overlap).
//!
//! # Usage
//!
//! ```bash
//! # Run with defaults (no HNSW sweep, just top_k)
//! cargo run --example s3vectors_param_sweep
//!
//! # MinIO HNSW parameter sweep
//! cargo run --example s3vectors_param_sweep -- \
//!     --m-values 8,16,32 \
//!     --ef-construction-values 64,128,256 \
//!     --ef-search-values 50,100,200 \
//!     --num-vectors 5000
//!
//! # Quick test with fixed HNSW params
//! cargo run --example s3vectors_param_sweep -- \
//!     --m-values 16 \
//!     --ef-construction-values 128 \
//!     --ef-search-values 50,100,200,400 \
//!     --num-vectors 1000
//! ```

#[path = "common.rs"]
mod common;

use clap::Parser;
use common::{
    calculate_recall, cleanup_bucket, compute_mean_std, generate_random_vectors,
    wait_for_index_exists, wait_for_index_ready, LocalVector,
};
use minio::s3::creds::StaticProvider;
use minio::s3::http::BaseUrl;
use minio::s3::types::Region;
use minio::s3vectors::{
    BucketName, DistanceMetric, HnswConfig, IndexAlgorithm, IndexName, Vector, VectorData,
    VectorKey, VectorsApi, VectorsClient,
};
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::SeedableRng;
use std::time::{Duration, Instant};

/// S3 Vectors HNSW Parameter Sweep (MinIO Extension)
#[derive(Parser)]
#[command(name = "s3vectors_param_sweep")]
#[command(about = "Benchmark S3 Vectors recall across HNSW parameters (MinIO extension)")]
struct Args {
    /// MinIO server endpoint
    #[arg(long, default_value = "http://localhost:9000")]
    endpoint: String,

    /// Access key
    #[arg(long, default_value = "minioadmin")]
    access_key: String,

    /// Secret key
    #[arg(long, default_value = "minioadmin")]
    secret_key: String,

    /// Number of vectors to insert
    #[arg(long, default_value = "10000")]
    num_vectors: usize,

    /// Vector dimension
    #[arg(long, default_value = "256")]
    dimension: u32,

    /// Number of query vectors
    #[arg(long, default_value = "200")]
    num_queries: usize,

    /// Batch size for PutVectors (max 500)
    #[arg(long, default_value = "500")]
    batch_size: usize,

    /// Random seed for reproducibility
    #[arg(long, default_value = "42")]
    seed: u64,

    /// Distance metric (cosine or euclidean)
    #[arg(long, default_value = "cosine")]
    distance: String,

    /// Comma-separated top_k values to test (used when no HNSW sweep)
    #[arg(long, default_value = "100")]
    top_k: u32,

    /// Comma-separated m values (MinIO extension)
    #[arg(long)]
    m_values: Option<String>,

    /// Comma-separated ef_construction values (MinIO extension)
    #[arg(long)]
    ef_construction_values: Option<String>,

    /// Comma-separated ef_search values (MinIO extension)
    #[arg(long)]
    ef_search_values: Option<String>,

    /// Output format: table, csv, or json
    #[arg(long, default_value = "table")]
    output: String,

    /// Enable verbose debug output
    #[arg(long, default_value = "false")]
    verbose: bool,

    /// Use brute-force search (MinIO extension)
    ///
    /// When enabled, uses exact brute-force search instead of HNSW.
    /// This provides 100% recall as a baseline comparison.
    #[arg(long, default_value = "false")]
    brute_force: bool,

    /// Use random query vectors not present in the database
    ///
    /// By default, query vectors are selected from the uploaded dataset.
    /// When this flag is set, query vectors are randomly generated and
    /// NOT present in the database. This tests recall for "external" queries.
    #[arg(long, default_value = "false")]
    external_queries: bool,
}

/// Results from a single HNSW configuration test
#[derive(Debug, Clone)]
struct TestResult {
    m: Option<u32>,
    ef_construction: Option<u32>,
    ef_search: Option<u32>,
    recall_mean: f64,
    recall_std_dev: f64,
    qps: f64,
}

/// Parse comma-separated u32 values
fn parse_values(s: &str) -> Vec<u32> {
    s.split(',').filter_map(|v| v.trim().parse().ok()).collect()
}

/// Run queries against an index and measure recall
#[allow(clippy::too_many_arguments)]
async fn run_queries(
    client: &VectorsClient,
    bucket: &BucketName,
    index: &IndexName,
    query_vectors: &[&LocalVector],
    ground_truths: &[Vec<String>],
    top_k: u32,
    ef_search: Option<u32>,
    brute_force: bool,
    verbose: bool,
) -> Result<(Vec<f64>, Vec<f64>, Duration), Box<dyn std::error::Error + Send + Sync>> {
    let mut recalls: Vec<f64> = Vec::new();
    let mut latencies: Vec<f64> = Vec::new();
    let mut total_query_time = Duration::ZERO;
    let mut debug_count = 0;

    for (query_idx, (qv, full_ground_truth)) in
        query_vectors.iter().zip(ground_truths.iter()).enumerate()
    {
        let ground_truth: Vec<String> = full_ground_truth
            .iter()
            .take(top_k as usize)
            .cloned()
            .collect();

        let start_query = Instant::now();
        let query_data = VectorData::new(qv.values.clone()).unwrap();

        // brute_force ignores ef_search, so only 3 cases needed
        let response = if brute_force {
            client
                .query_vectors(bucket, index, query_data, top_k)?
                .brute_force(true)
                .return_distance(true)
                .build()
                .send()
                .await?
        } else if let Some(ef) = ef_search {
            client
                .query_vectors(bucket, index, query_data, top_k)?
                .ef_search(ef)
                .return_distance(true)
                .build()
                .send()
                .await?
        } else {
            client
                .query_vectors(bucket, index, query_data, top_k)?
                .return_distance(true)
                .build()
                .send()
                .await?
        };

        let elapsed = start_query.elapsed();
        total_query_time += elapsed;
        latencies.push(elapsed.as_secs_f64() * 1000.0);

        let result_vectors = response.vectors()?;
        let api_results: Vec<String> = result_vectors
            .iter()
            .map(|v| v.key.as_str().to_string())
            .collect();

        let recall = calculate_recall(&ground_truth, &api_results);
        recalls.push(recall);

        // Debug output for first few queries
        if verbose && debug_count < 3 {
            println!();
            println!("    DEBUG Query {}: key={}", query_idx, qv.key);
            println!(
                "      Expected: {:?}",
                ground_truth.iter().take(5).collect::<Vec<_>>()
            );
            println!(
                "      Actual:   {:?}",
                api_results.iter().take(5).collect::<Vec<_>>()
            );
            if let Some(first) = result_vectors.first() {
                println!("      First result distance: {:?}", first.distance);
            }
            println!("      Recall: {:.1}%", recall);
            debug_count += 1;
        }
    }

    Ok((recalls, latencies, total_query_time))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();

    let args = Args::parse();

    // Parse HNSW parameter values
    let m_values = args.m_values.as_ref().map(|s| parse_values(s));
    let ef_construction_values = args
        .ef_construction_values
        .as_ref()
        .map(|s| parse_values(s));
    let ef_search_values = args.ef_search_values.as_ref().map(|s| parse_values(s));

    let is_hnsw_sweep =
        m_values.is_some() || ef_construction_values.is_some() || ef_search_values.is_some();

    // Parse distance metric
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

    println!("================================================================");
    println!("     S3 VECTORS HNSW PARAMETER SWEEP");
    println!("================================================================");
    println!(
        "Dataset: {} vectors × {} dims, {} metric",
        args.num_vectors,
        args.dimension,
        args.distance.to_lowercase()
    );
    println!(
        "Queries: {} queries, top-{}, seed={}",
        args.num_queries, args.top_k, args.seed
    );
    if args.external_queries {
        println!("Query mode: external (random vectors not in database)");
    }
    if args.brute_force {
        println!("Search mode: brute-force (exact search)");
    }
    println!();

    if is_hnsw_sweep {
        print!("Sweep: ");
        let mut parts = Vec::new();
        if let Some(ref m) = m_values {
            parts.push(format!("m={:?}", m));
        }
        if let Some(ref ef_c) = ef_construction_values {
            parts.push(format!("ef_construction={:?}", ef_c));
        }
        if let Some(ref ef_s) = ef_search_values {
            parts.push(format!("ef_search={:?}", ef_s));
        }
        println!("{}", parts.join(", "));
    } else {
        println!(
            "Mode: Single test (use --m-values, --ef-construction-values, --ef-search-values for sweep)"
        );
    }
    println!("================================================================\n");

    // Create client
    let base_url = args.endpoint.parse::<BaseUrl>()?;
    let provider = StaticProvider::new(&args.access_key, &args.secret_key, None);
    let region = Region::new("us-east-1")?;
    let client = VectorsClient::new(base_url, Some(provider), Some(region))?;

    let bucket = BucketName::new("param-sweep-bench")?;

    // Generate vectors
    println!("Step 1: Generating {} random vectors...", args.num_vectors);
    let mut rng = StdRng::seed_from_u64(args.seed);
    let vectors = generate_random_vectors(&mut rng, args.num_vectors, args.dimension);

    // Generate or select query vectors
    // For external queries, we generate new random vectors not in the database.
    // For internal queries, we select from the uploaded dataset.
    let external_query_vectors: Vec<LocalVector> = if args.external_queries {
        println!(
            "Step 2: Generating {} external query vectors (not in database)...",
            args.num_queries
        );
        generate_random_vectors(&mut rng, args.num_queries, args.dimension)
            .into_iter()
            .enumerate()
            .map(|(i, mut v)| {
                v.key = format!("query-{:06}", i);
                v
            })
            .collect()
    } else {
        println!(
            "Step 2: Selecting {} query vectors from dataset...",
            args.num_queries
        );
        Vec::new()
    };

    let query_vectors: Vec<&LocalVector> = if args.external_queries {
        external_query_vectors.iter().collect()
    } else {
        let mut indices: Vec<usize> = (0..vectors.len()).collect();
        indices.shuffle(&mut rng);
        indices
            .into_iter()
            .take(args.num_queries)
            .map(|i| &vectors[i])
            .collect()
    };

    // Prepare batch vectors for upload
    let batch_vectors: Vec<Vec<Vector>> = vectors
        .chunks(args.batch_size)
        .map(|chunk| {
            chunk
                .iter()
                .map(|v| {
                    Vector::new(
                        VectorKey::new(&v.key).unwrap(),
                        VectorData::new(v.values.clone()).unwrap(),
                    )
                })
                .collect()
        })
        .collect();

    // Create bucket and ground-truth index
    println!("Step 3: Setting up ground-truth index...");
    let gt_index = IndexName::new("ground-truth")?;

    // Clean up any existing resources from previous runs
    cleanup_bucket(&client, &bucket).await;

    // Create fresh bucket and index
    client.create_vector_bucket(&bucket)?.build().send().await?;
    client
        .create_index(&bucket, &gt_index, args.dimension, distance_metric)?
        .algorithm(IndexAlgorithm::Hnsw)
        .build()
        .send()
        .await?;

    // Wait for index to be ready to accept data
    wait_for_index_exists(&client, &bucket, &gt_index).await?;

    // Upload vectors to ground-truth index
    print!("  Uploading and indexing... ");
    std::io::Write::flush(&mut std::io::stdout())?;
    let index_start = Instant::now();
    for batch in &batch_vectors {
        client
            .put_vectors(&bucket, &gt_index, batch.clone())?
            .build()
            .send()
            .await?;
    }
    let expected_count = vectors.len() as i64;
    wait_for_index_ready(&client, &bucket, &gt_index, expected_count).await?;
    println!("{:.1}s", index_start.elapsed().as_secs_f64());

    // Compute ground truth using server brute-force
    let top_k = args.top_k;
    println!(
        "  Computing ground truth (server brute-force, top_k={})...",
        top_k
    );
    let mut ground_truths: Vec<Vec<String>> = Vec::with_capacity(query_vectors.len());
    for qv in &query_vectors {
        let resp = client
            .query_vectors(
                &bucket,
                &gt_index,
                VectorData::new(qv.values.clone()).unwrap(),
                top_k,
            )?
            .brute_force(true)
            .return_distance(true)
            .build()
            .send()
            .await?;
        let keys: Vec<String> = resp
            .vectors()?
            .iter()
            .map(|v| v.key.as_str().to_string())
            .collect();
        ground_truths.push(keys);
    }
    println!(
        "  Ground truth computed for {} queries",
        ground_truths.len()
    );

    // Delete ground-truth index (we'll create test indexes next)
    let _ = client
        .delete_index(&bucket, &gt_index)?
        .build()
        .send()
        .await;

    let mut results: Vec<TestResult> = Vec::new();

    // Build test configurations
    let m_list = m_values.unwrap_or_else(std::vec::Vec::new);
    let ef_c_list = ef_construction_values.unwrap_or_else(std::vec::Vec::new);
    let ef_s_list = ef_search_values.unwrap_or_else(std::vec::Vec::new);

    // If doing HNSW sweep, iterate over m x ef_construction combinations for index creation
    // and ef_search values for query-time
    if is_hnsw_sweep {
        // Build list of (m, ef_construction) pairs for index creation
        let mut index_configs: Vec<(Option<u32>, Option<u32>)> = Vec::new();

        if m_list.is_empty() && ef_c_list.is_empty() {
            // Only ef_search sweep - single index with defaults
            index_configs.push((None, None));
        } else if m_list.is_empty() {
            // Only ef_construction sweep
            for &ef_c in &ef_c_list {
                index_configs.push((None, Some(ef_c)));
            }
        } else if ef_c_list.is_empty() {
            // Only m sweep
            for &m in &m_list {
                index_configs.push((Some(m), None));
            }
        } else {
            // Full sweep of m x ef_construction
            for &m in &m_list {
                for &ef_c in &ef_c_list {
                    index_configs.push((Some(m), Some(ef_c)));
                }
            }
        }

        let ef_search_list = if ef_s_list.is_empty() {
            vec![None]
        } else {
            ef_s_list.iter().map(|&v| Some(v)).collect()
        };

        let total_tests = index_configs.len() * ef_search_list.len();
        let mut test_num = 0;

        println!();
        println!(
            "Running {} test configurations ({} indexes x {} ef_search values)...",
            total_tests,
            index_configs.len(),
            ef_search_list.len()
        );
        println!();

        for (m, ef_c) in &index_configs {
            let index = IndexName::new(format!(
                "idx-m{}-efc{}",
                m.map(|v| v.to_string())
                    .unwrap_or_else(|| "def".to_string()),
                ef_c.map(|v| v.to_string())
                    .unwrap_or_else(|| "def".to_string())
            ))?;

            // Clean up any existing index
            let _ = client.delete_index(&bucket, &index)?.build().send().await;

            // Create index with HNSW configuration
            println!("Creating index: m={:?}, ef_construction={:?}", m, ef_c);

            // Ensure bucket exists
            let _ = client.create_vector_bucket(&bucket)?.build().send().await;

            // Build HNSW config and create index based on which parameters are set
            match (m, ef_c) {
                (Some(m_val), Some(ef_c_val)) => {
                    let hnsw = HnswConfig::builder()
                        .m(*m_val)
                        .ef_construction(*ef_c_val)
                        .build();
                    client
                        .create_index(&bucket, &index, args.dimension, distance_metric)?
                        .algorithm(IndexAlgorithm::Hnsw)
                        .hnsw_config(hnsw)
                        .build()
                        .send()
                        .await?;
                }
                (Some(m_val), None) => {
                    let hnsw = HnswConfig::builder().m(*m_val).build();
                    client
                        .create_index(&bucket, &index, args.dimension, distance_metric)?
                        .algorithm(IndexAlgorithm::Hnsw)
                        .hnsw_config(hnsw)
                        .build()
                        .send()
                        .await?;
                }
                (None, Some(ef_c_val)) => {
                    let hnsw = HnswConfig::builder().ef_construction(*ef_c_val).build();
                    client
                        .create_index(&bucket, &index, args.dimension, distance_metric)?
                        .algorithm(IndexAlgorithm::Hnsw)
                        .hnsw_config(hnsw)
                        .build()
                        .send()
                        .await?;
                }
                (None, None) => {
                    client
                        .create_index(&bucket, &index, args.dimension, distance_metric)?
                        .algorithm(IndexAlgorithm::Hnsw)
                        .build()
                        .send()
                        .await?;
                }
            }

            // Wait for index to be ready to accept data
            wait_for_index_exists(&client, &bucket, &index).await?;

            // Upload vectors
            print!("  Indexing... ");
            std::io::Write::flush(&mut std::io::stdout())?;
            let index_start = Instant::now();
            for batch in &batch_vectors {
                client
                    .put_vectors(&bucket, &index, batch.clone())?
                    .build()
                    .send()
                    .await?;
            }
            let expected_count = vectors.len() as i64;
            wait_for_index_ready(&client, &bucket, &index, expected_count).await?;
            println!("{:.1}s", index_start.elapsed().as_secs_f64());

            // Test each ef_search value
            for ef_s in &ef_search_list {
                test_num += 1;
                print!(
                    "  [{}/{}] Testing ef_search={:?}... ",
                    test_num, total_tests, ef_s
                );
                std::io::Write::flush(&mut std::io::stdout())?;

                let (recalls, _, total_query_time) = run_queries(
                    &client,
                    &bucket,
                    &index,
                    &query_vectors,
                    &ground_truths,
                    args.top_k,
                    *ef_s,
                    args.brute_force,
                    args.verbose && test_num == 1,
                )
                .await?;

                let (recall_mean, recall_std_dev) = compute_mean_std(&recalls);
                let qps = args.num_queries as f64 / total_query_time.as_secs_f64();

                results.push(TestResult {
                    m: *m,
                    ef_construction: *ef_c,
                    ef_search: *ef_s,
                    recall_mean,
                    recall_std_dev,
                    qps,
                });

                println!(
                    "recall={:.1}% ± {:.1}%, QPS={:.1}",
                    recall_mean, recall_std_dev, qps
                );
            }

            // Clean up index
            let _ = client.delete_index(&bucket, &index)?.build().send().await;
        }
    } else {
        // Simple mode: single index, single test
        let index = IndexName::new("benchmark-index")?;

        println!("Step 4: Setting up index...");
        let _ = client.delete_index(&bucket, &index)?.build().send().await;
        let _ = client.delete_vector_bucket(&bucket)?.build().send().await;

        client.create_vector_bucket(&bucket)?.build().send().await?;

        client
            .create_index(&bucket, &index, args.dimension, distance_metric)?
            .algorithm(IndexAlgorithm::Hnsw)
            .build()
            .send()
            .await?;

        // Wait for index to be ready to accept data
        wait_for_index_exists(&client, &bucket, &index).await?;

        // Upload and index vectors
        print!("Step 5: Indexing... ");
        std::io::Write::flush(&mut std::io::stdout())?;
        let index_start = Instant::now();
        for batch in &batch_vectors {
            client
                .put_vectors(&bucket, &index, batch.clone())?
                .build()
                .send()
                .await?;
        }
        let expected_count = vectors.len() as i64;
        wait_for_index_ready(&client, &bucket, &index, expected_count).await?;
        println!("{:.1}s", index_start.elapsed().as_secs_f64());

        println!();
        println!("Step 6: Running recall benchmark...");

        let (recalls, _, total_query_time) = run_queries(
            &client,
            &bucket,
            &index,
            &query_vectors,
            &ground_truths,
            args.top_k,
            None,
            args.brute_force,
            args.verbose,
        )
        .await?;

        let (recall_mean, recall_std_dev) = compute_mean_std(&recalls);
        let qps = args.num_queries as f64 / total_query_time.as_secs_f64();

        results.push(TestResult {
            m: None,
            ef_construction: None,
            ef_search: None,
            recall_mean,
            recall_std_dev,
            qps,
        });

        // Cleanup
        let _ = client.delete_index(&bucket, &index)?.build().send().await;
    }

    // Clean up bucket
    cleanup_bucket(&client, &bucket).await;

    // Output results
    println!();
    match args.output.as_str() {
        "csv" => print_csv(&results),
        "json" => print_json(&results),
        _ => print_table(&results),
    }

    // Analysis
    println!();
    println!("════════════════════════════════════════════════════════════════════");
    println!("                          ANALYSIS");
    println!("════════════════════════════════════════════════════════════════════");
    println!();

    if is_hnsw_sweep {
        // Find best configuration
        if let Some(best) = results
            .iter()
            .filter(|r| r.recall_mean >= 90.0)
            .max_by(|a, b| {
                a.qps
                    .partial_cmp(&b.qps)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
        {
            println!("Best configuration (≥90% recall, max QPS):");
            println!(
                "  m={:?}, ef_construction={:?}, ef_search={:?}",
                best.m, best.ef_construction, best.ef_search
            );
            println!(
                "  Recall: {:.1}% ± {:.1}%, QPS: {:.1}",
                best.recall_mean, best.recall_std_dev, best.qps
            );
        } else if let Some(best) = results.iter().max_by(|a, b| {
            a.recall_mean
                .partial_cmp(&b.recall_mean)
                .unwrap_or(std::cmp::Ordering::Equal)
        }) {
            println!("Best recall achieved:");
            println!(
                "  m={:?}, ef_construction={:?}, ef_search={:?}",
                best.m, best.ef_construction, best.ef_search
            );
            println!(
                "  Recall: {:.1}% ± {:.1}%, QPS: {:.1}",
                best.recall_mean, best.recall_std_dev, best.qps
            );
        }

        // Show recall vs QPS trade-off
        println!();
        println!("Recall vs QPS trade-off:");
        let mut sorted_results = results.clone();
        sorted_results.sort_by(|a, b| {
            b.recall_mean
                .partial_cmp(&a.recall_mean)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        for r in sorted_results.iter().take(5) {
            println!(
                "  m={:>3?}, ef_c={:>4?}, ef_s={:>4?} -> recall={:.1}%, QPS={:.1}",
                r.m, r.ef_construction, r.ef_search, r.recall_mean, r.qps
            );
        }
    } else {
        let r = &results[0];
        println!("Single test result:");
        println!(
            "  Recall: {:.1}% ± {:.1}%, QPS: {:.1}",
            r.recall_mean, r.recall_std_dev, r.qps
        );
        println!();
        println!("To run HNSW parameter sweep (MinIO extension), use:");
        println!(
            "  --m-values 8,16,32 --ef-construction-values 64,128,256 --ef-search-values 50,100,200"
        );
    }

    println!();
    println!("════════════════════════════════════════════════════════════════════");
    println!("Done.");

    Ok(())
}

fn print_table(results: &[TestResult]) {
    println!("════════════════════════════════════════════════════════════════════");
    println!("                        RESULTS TABLE");
    println!("════════════════════════════════════════════════════════════════════");
    println!();
    println!(
        "{:>6} {:>6} {:>9} {:>10} {:>8} {:>10}",
        "m", "ef_c", "ef_s", "Recall%", "StdDev", "QPS"
    );
    println!("{}", "-".repeat(68));

    for r in results {
        let m_str =
            r.m.map(|v| v.to_string())
                .unwrap_or_else(|| "-".to_string());
        let ef_c_str = r
            .ef_construction
            .map(|v| v.to_string())
            .unwrap_or_else(|| "-".to_string());
        let ef_s_str = r
            .ef_search
            .map(|v| v.to_string())
            .unwrap_or_else(|| "-".to_string());

        println!(
            "{:>6} {:>6} {:>9} {:>9.1}% {:>7.1}% {:>10.1}",
            m_str, ef_c_str, ef_s_str, r.recall_mean, r.recall_std_dev, r.qps
        );
    }
}

fn print_csv(results: &[TestResult]) {
    println!("m,ef_construction,ef_search,recall_mean,recall_std_dev,qps");
    for r in results {
        let m_str = r.m.map(|v| v.to_string()).unwrap_or_default();
        let ef_c_str = r.ef_construction.map(|v| v.to_string()).unwrap_or_default();
        let ef_s_str = r.ef_search.map(|v| v.to_string()).unwrap_or_default();
        println!(
            "{},{},{},{:.2},{:.2},{:.2}",
            m_str, ef_c_str, ef_s_str, r.recall_mean, r.recall_std_dev, r.qps
        );
    }
}

fn print_json(results: &[TestResult]) {
    println!("[");
    for (i, r) in results.iter().enumerate() {
        let comma = if i < results.len() - 1 { "," } else { "" };
        let m_json =
            r.m.map(|v| v.to_string())
                .unwrap_or_else(|| "null".to_string());
        let ef_c_json = r
            .ef_construction
            .map(|v| v.to_string())
            .unwrap_or_else(|| "null".to_string());
        let ef_s_json = r
            .ef_search
            .map(|v| v.to_string())
            .unwrap_or_else(|| "null".to_string());
        println!(
            r#"  {{"m": {}, "ef_construction": {}, "ef_search": {}, "recall_mean": {:.2}, "recall_std_dev": {:.2}, "qps": {:.2}}}{}"#,
            m_json, ef_c_json, ef_s_json, r.recall_mean, r.recall_std_dev, r.qps, comma
        );
    }
    println!("]");
}
