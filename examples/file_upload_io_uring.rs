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

use std::io::Read;
use clap::{Parser, ValueEnum};
use std::path::PathBuf;
use tracing::{Level, info};
use futures::{future::LocalBoxFuture, future::select_all};

use minio::s3::builders::ObjectContent;
use minio::s3::error::Error;
use minio::s3::response::{AppendObjectResponse, StatObjectResponse};
use minio::s3::response::a_response_traits::HasS3Fields;
use minio::s3::segmented_bytes::SegmentedBytes;
use minio::s3::types::S3Api;
use minio_common::rand_src::RandSrc;
use minio_common::test_context::TestContext;

mod common;

#[derive(Clone, Copy, Debug, ValueEnum)]
enum Backend {
    Tokio,
    Uring,
}

#[derive(Parser, Debug)]
struct Args {
    /// Backend to use
    #[arg(short = 'b', long, value_enum, default_value = "tokio")]
    backend: Backend,
    
    /// File to upload
    #[arg(short = 'f', long, default_value = "R:/Temp/test_file.bin")]
    file: PathBuf,

    /// File size (in MiB). Creates the file (to upload) with random data of size (in MiB). 
    #[arg(short = 's', long, default_value = "0")]
    file_size_mib: usize,
    
    /// Chunk size in MiB
    #[arg(short = 'x', long, default_value = "8")]
    chunk_mib: usize,

    /// Number of in-flight read requests (only for Uring backend)
    #[arg(short = 'c', long, default_value = "64")]
    concurrency: usize,
}

async fn create_file(args: &Args) {
    if args.file_size_mib > 0 {
        // current time:
        let now = std::time::SystemTime::now();
        if args.file.exists() {
            log::info!("File `{}` already exists, deleting it", &args.file.to_str().unwrap());
            std::fs::remove_file(&args.file).expect("Failed to delete existing file");
        }
        // Create a file with random data of specified size
        let size: u64 = (args.file_size_mib * 1024 * 1024) as u64;
        log::info!("Generating file `{}` with size {} MiB", &args.file.to_str().unwrap(), args.file_size_mib);
        let obj: ObjectContent = ObjectContent::new_from_stream(RandSrc::new(size), Some(size));
        obj.to_file(&args.file).await.unwrap();
        log::info!("File `{}` generated (took {} seconds)", &args.file.to_str().unwrap(), now.elapsed().unwrap().as_secs_f64());
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();
    let args = Args::parse();
    log::info!("args: {:?}", args);

    
    match args.backend {
        Backend::Tokio => tokio_example::run_example(args),
        Backend::Uring => uring_example::run_example(args),
    }

    Ok(())
}

mod tokio_example {
    use std::sync::Arc;
    use crate::{Args, create_file, dummy_upload};
    use minio_common::test_context::TestContext;
    use std::time::Instant;
    use tokio::io::AsyncReadExt;
    use tokio::sync::Semaphore;
    use tracing::info;
    use minio_common::utils::rand_object_name;

    pub(crate) fn run_example(args: Args) {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(run(args));
    }

    async fn run(args: Args) {
        let ctx = TestContext::new_from_env();
        let (bucket_name, _cleanup) = ctx.create_bucket_helper().await;
        create_file(&args).await;

        let file = tokio::fs::File::open(&args.file).await.unwrap();
        let mut reader = tokio::io::BufReader::new(file);

        let chunk_size_bytes: usize = args.chunk_mib * 1024 * 1024;
        let object_name: String = rand_object_name();

        let mut offset = 0u64;
        let mut chunk_index = 0;

        let start = Instant::now();
        loop {
            let start1 = Instant::now();
            let mut buf = vec![0u8; chunk_size_bytes]; // create a new buffer for each read since we move it later
            let n: usize = reader.read(&mut buf).await.unwrap();
            if n == 0 {
                break; // EOF
            }
            buf.truncate(n); // Remove unused space if not a full chunk

            println!("Read {} bytes at offset {} took {:.4?}", n, offset, start1.elapsed());

            let index = chunk_index;
            chunk_index += 1;

            let start2 = Instant::now();
            println!("Processing chunk {} at offset {}", index, offset);
            dummy_upload(buf, offset, &ctx, &bucket_name, &object_name).await;
            offset += n as u64;
            println!("Uploading {}/{} took {:.4?}", bucket_name, object_name, start2.elapsed());
        }
        info!("Tokio backend finished in {:.4?}", start.elapsed());
    }
}

mod uring_example {
    use crate::{Args, create_file, dummy_upload};
    use compio::BufResult;
    use compio::fs::{metadata};
    use compio::io::AsyncReadAt;
    use futures::FutureExt;
    use futures::future::{LocalBoxFuture, select_all};
    use minio_common::test_context::TestContext;
    use minio_common::utils::rand_object_name;
    use std::time::Instant;
    use tracing::info;

    pub(crate) fn run_example(args: Args) {
        compio::runtime::RuntimeBuilder::new()
            .build()
            .unwrap()
            .block_on(run(args))
    }

    fn wrap_read(
        file: &compio::fs::File,
        chunk_size: usize,
        offset: u64,
    ) -> LocalBoxFuture<(u64, Result<usize, std::io::Error>, Vec<u8>)> {
        async move {
            let BufResult(res, buf) = file.read_at(vec![0u8; chunk_size], offset).await;
            (offset, res, buf)
        }
        .boxed_local()
    }

    async fn run(args: Args) {
        //let ctx = TestContext::new_from_env();
        //let (bucket_name, _cleanup) = ctx.create_bucket_helper().await;
        create_file(&args).await;
        let object_name = rand_object_name();

        let file = compio::fs::File::open(&args.file).await.unwrap();
        let total_size = metadata(&args.file).await.unwrap().len(); // u64

        let chunk_size: usize = args.chunk_mib * 1024 * 1024;
        let mut offset: u64 = 0;
        let concurrency: usize = args.concurrency;

        let mut in_flight: Vec<LocalBoxFuture<'_, (u64, Result<usize, std::io::Error>, Vec<u8>)>> =
            Vec::with_capacity(concurrency);

        // Initial fill of in-flight reads
        for _ in 0..concurrency {
            if offset >= total_size {
                break;
            }
            in_flight.push(wrap_read(&file, chunk_size, offset));
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

            //dummy_upload(&buf[..n], off, &ctx, &bucket_name, &object_name).await;

            // Enqueue next chunk
            if offset < total_size {
                remaining.push(wrap_read(&file, chunk_size, offset));
                offset += chunk_size as u64;
            }
            in_flight = remaining;
        }

        let dur = start.elapsed();
        info!("compio pipeline finished in {:.2?}", dur);
    }
}

/// Simulates upload — replace this with your MinIO SDK call
async fn dummy_upload(
    data: Vec<u8>,
    offset_bytes: u64,
    ctx: &TestContext,
    bucket_name: &str,
    object_name: &str,
) {
    let resp: Result<AppendObjectResponse, Error> = ctx.client.append_object(
        bucket_name,
        object_name,
        SegmentedBytes::from(bytes::Bytes::from(data)),
        offset_bytes
    )
        .send()
        .await;

    match resp {
        Ok(_resp) => {
            //println!("Success: {:?}", resp.headers())
        },
        Err(e) => eprintln!("Failed to upload: {}", e),
    }
/*
    let resp: StatObjectResponse = ctx
        .client
        .stat_object(bucket_name, object_name)
        .send()
        .await
        .unwrap();
    println!("Uploaded {}", resp);
 */
    // tokio::task::yield_now().await;
}
