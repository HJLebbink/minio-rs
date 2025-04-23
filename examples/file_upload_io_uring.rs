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

use clap::{Parser, ValueEnum};
use std::{path::PathBuf, time::Instant};
use tokio::io::AsyncReadExt;
use tracing::{Level, info};

#[cfg(target_os = "linux")]
use futures::{FutureExt, future::LocalBoxFuture, future::select_all};
use minio::s3::builders::ObjectContent;
use minio::s3::types::S3Api;
use minio_common::rand_src::RandSrc;
use minio_common::test_context::TestContext;
#[cfg(target_os = "linux")]
use tokio_uring::fs::File;

mod common;

#[derive(Clone, Copy, Debug, ValueEnum)]
enum Backend {
    Tokio,
    Uring,
}

#[derive(Parser, Debug)]
struct Args {
    /// Backend to use
    #[arg(long, value_enum, default_value = "tokio")]
    backend: Backend,

    /// File to upload
    #[arg(long, default_value = "R:/Temp/test_file.bin")]
    file: PathBuf,

    /// Chunk size in MiB
    #[arg(long, default_value = "8")]
    chunk_mib: usize,

    /// Number of in-flight read requests (only for Uring backend)
    #[arg(long, default_value = "64")]
    concurrency: usize,
}

async fn create_file_if_not_exists(args: &Args) {
    if !args.file.exists() {
        log::info!("Generating file `{}`", &args.file.to_str().unwrap());
        let size: u64 = 16 + 5 * 1024 * 1024;
        let obj: ObjectContent = ObjectContent::new_from_stream(RandSrc::new(size), Some(size));
        obj.to_file(&args.file).await.unwrap();
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();
    let args = Args::parse();
    log::info!("args: {:?}", args);

    match args.backend {
        Backend::Tokio => tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(run_tokio(args)),
        Backend::Uring => {
            #[cfg(target_os = "linux")]
            {
                tokio_uring::start(async {
                    run_uring(args).await;
                });
            }
            #[cfg(not(target_os = "linux"))]
            {
                panic!("Uring backend is only supported on Linux");
            }
        }
    }

    Ok(())
}

async fn run_tokio(args: Args) {
    let ctx = TestContext::new_from_env();
    let (bucket_name, _cleanup) = ctx.create_bucket_helper().await;
    create_file_if_not_exists(&args).await;

    let file = tokio::fs::File::open(&args.file).await.unwrap();
    let mut reader = tokio::io::BufReader::new(file);
    let mut offset: u64 = 0;
    let chunk_size: usize = args.chunk_mib * 1024 * 1024;
    let mut buf = vec![0u8; chunk_size];
    let object_name: &str = args.file.to_str().unwrap();

    let start = Instant::now();
    loop {
        let n = reader.read(&mut buf).await.unwrap();
        if n == 0 {
            break;
        }
        dummy_upload(&buf[..n], offset, &ctx, &bucket_name, object_name).await;
        offset += n as u64;
    }
    let dur = start.elapsed();
    info!("Tokio backend finished in {:.2?}", dur);
}

#[cfg(target_os = "linux")]
async fn run_uring(args: Args) {
    let ctx = TestContext::new_from_env();
    let (bucket_name, _cleanup) = ctx.create_bucket_helper().await;
    crate::create_file_if_not_exists(&args).await;

    let file = File::open(&args.file).await.unwrap();
    let total_size = tokio::fs::metadata(&args.file).await.unwrap().len(); // already a u64

    let chunk_size: usize = args.chunk_mib * 1024 * 1024;
    let mut offset: u64 = 0;
    let concurrency = args.concurrency;

    // Define the correct output type for your futures
    let mut in_flight =
        Vec::<LocalBoxFuture<'_, (u64, Result<usize, std::io::Error>, Vec<u8>)>>::with_capacity(
            concurrency,
        );

    for _ in 0..concurrency {
        if offset >= total_size {
            break;
        }
        let off = offset;
        let fut = file
            .read_at(vec![0u8; chunk_size], off)
            .map(move |(res, buf)| (off, res, buf))
            .boxed_local();
        in_flight.push(fut);
        offset += chunk_size as u64;
    }

    let start = Instant::now();

    while !in_flight.is_empty() {
        let (outcome, _idx, mut remaining) = select_all(in_flight).await;
        let (off, res, buf) = outcome;
        let n = match res {
            Ok(0) => break, // EOF
            Ok(n) => n,
            Err(e) => panic!("read error at offset {off}: {e}"),
        };

        dummy_upload(&buf[..n], off).await;

        // Enqueue next chunk if any
        if offset < total_size {
            let off_next = offset;
            let fut = file
                .read_at(vec![0u8; chunk_size], off_next)
                .map(move |(res, buf)| (off_next, res, buf))
                .boxed_local();
            remaining.push(fut);
            offset += chunk_size as u64;
        }

        in_flight = remaining;
    }

    let dur = start.elapsed();
    info!("Tokio-uring pipeline finished in {:.2?}", dur);
}

/// Simulates upload — replace this with your MinIO SDK call
async fn dummy_upload(
    data: &[u8],
    offset: u64,
    ctx: &TestContext,
    bucket_name: &str,
    object_name: &str,
) {

    /*
        ctx.client.put_object_content(
                &bucket_name,
                &object_name,
                ObjectContent::new_from_stream(RandSrc::new(size), Some(size)),
            )
            .send()
            .await
            .unwrap();
        let resp = ctx
            .client
            .stat_object(&bucket_name, &object_name)
            .send()
            .await
            .unwrap();

    */
    // tokio::task::yield_now().await;
}
