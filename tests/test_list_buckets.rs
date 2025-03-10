use crate::common::{create_bucket_helper, CleanupGuard, TestContext};
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

use minio::s3::types::S3Api;

mod common;

#[tokio::test(flavor = "multi_thread", worker_threads = 10)]
async fn list_buckets() {
    const N_BUCKETS: usize = 3;
    let ctx = TestContext::new_from_env();

    let mut names: Vec<String> = Vec::new();
    let mut guards: Vec<CleanupGuard> = Vec::new();
    for _ in 1..=N_BUCKETS {
        let (bucket_name, guard) = create_bucket_helper(&ctx).await;
        names.push(bucket_name);
        guards.push(guard);
    }

    assert_eq!(names.len(), N_BUCKETS);

    let mut count = 0;
    let resp = ctx.client.list_buckets().send().await.unwrap();

    for bucket in resp.buckets.iter() {
        if names.contains(&bucket.name) {
            count += 1;
        }
    }
    assert_eq!(guards.len(), N_BUCKETS);
    assert_eq!(count, N_BUCKETS);
}
