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

use minio::s3::bucket_policy_config::BucketPolicyConfig;
use minio::s3::builders::VersioningStatus;
use minio::s3::client::DEFAULT_REGION;
use minio::s3::error::{Error, ErrorCode};
use minio::s3::response::a_response_traits::{HasBucket, HasRegion};
use minio::s3::response::{
    DeleteBucketReplicationResponse, GetBucketReplicationResponse, GetBucketVersioningResponse,
    PutBucketPolicyResponse, PutBucketReplicationResponse, PutBucketVersioningResponse,
};
use minio::s3::types::{ReplicationConfig, S3Api};
use minio_common::example::{
    create_bucket_policy_config_example_for_replication, create_bucket_replication_config_example,
    create_bucket_replication_config_example2,
};
use minio_common::test_context::TestContext;

#[minio_macros::test(skip_if_express)]
async fn bucket_replication_s3(ctx: TestContext, bucket_name: String) {
    //let (bucket_name_src, _cleanup) = ctx.create_bucket_helper().await;
    let bucket_name_src = "replication-src".to_string();
    //let ctx2 = TestContext::new_from_env();
    //let (bucket_name_dst, _cleanup2) = ctx2.create_bucket_helper().await;
    let bucket_name_dst = "replication-dst".to_string();

    // set the versioning on the buckets, and the bucket policy
    {
        let resp: PutBucketVersioningResponse = ctx
            .client
            .put_bucket_versioning(&bucket_name_src)
            .versioning_status(VersioningStatus::Enabled)
            .send()
            .await
            .unwrap();
        assert_eq!(resp.bucket, bucket_name_src);
        assert_eq!(resp.region(), DEFAULT_REGION);

        let resp: PutBucketVersioningResponse = ctx
            .client
            .put_bucket_versioning(&bucket_name_dst)
            .versioning_status(VersioningStatus::Enabled)
            .send()
            .await
            .unwrap();
        assert_eq!(resp.bucket, bucket_name_dst);
        assert_eq!(resp.region(), DEFAULT_REGION);

        let config: BucketPolicyConfig = create_bucket_policy_config_example_for_replication();

        let resp: PutBucketPolicyResponse = ctx
            .client
            .put_bucket_policy(&bucket_name_src)
            .config(config.clone())
            .send()
            .await
            .unwrap();
        assert_eq!(resp.bucket, bucket_name_src);
        assert_eq!(resp.region, DEFAULT_REGION);
    }

    if true {
        //let remote_target_id = "dadddae7-f1d7-440f-b5d6-651aa9a8c8a7";
        //let remote_target_arn = format!("arn:minio:replication::{remote_target_id}:{bucket_name_dst}");
        let config: ReplicationConfig = create_bucket_replication_config_example2();

        println!("bucket_name_src={}", bucket_name_src);
        println!("bucket_name_dst={}", bucket_name_dst);
        //println!("remote_target_arn={}", remote_target_arn);

        println!("Config: {:#?}", config);

        // TODO panic: called `Result::unwrap()` on an `Err` value: S3Error(ErrorResponse { code: "XMinioAdminRemoteTargetNotFoundError", message: "The remote target does not exist",
        let resp: PutBucketReplicationResponse = ctx
            .client
            .put_bucket_replication(&bucket_name_src)
            .replication_config(config.clone())
            .send()
            .await
            .unwrap();
        println!("response of setting replication: resp={:?}", resp);
        assert_eq!(resp.bucket, bucket_name_dst);
        assert_eq!(resp.region(), DEFAULT_REGION);

        let resp: GetBucketReplicationResponse = ctx
            .client
            .get_bucket_replication(&bucket_name_src)
            .send()
            .await
            .unwrap();
        //assert_eq!(resp.config, config); //TODO
        assert_eq!(resp.bucket, bucket_name_src);
        assert_eq!(resp.region(), DEFAULT_REGION);

        // TODO called `Result::unwrap()` on an `Err` value: S3Error(ErrorResponse { code: "XMinioAdminRemoteTargetNotFoundError", message: "The remote target does not exist",
        let resp: DeleteBucketReplicationResponse = ctx
            .client
            .delete_bucket_replication(&bucket_name_src)
            .send()
            .await
            .unwrap();
        println!("response of deleting replication: resp={:?}", resp);
    }
    let _resp: GetBucketVersioningResponse = ctx
        .client
        .get_bucket_versioning(&bucket_name_src)
        .send()
        .await
        .unwrap();
    cleanup2.cleanup().await;
    //println!("response of getting replication: resp={:?}", resp);
}

#[minio_macros::test(skip_if_not_express)]
async fn bucket_replication_s3express(ctx: TestContext, bucket_name: String) {
    let (bucket_name_src, _cleanup) = ctx.create_bucket_helper().await;
    let ctx2 = TestContext::new_from_env();
    let (bucket_name_dst, _cleanup) = ctx2.create_bucket_helper().await;
    let remote_target_arn = "arn:minio:replication::remote-target-id:target";
    let config: ReplicationConfig =
        create_bucket_replication_config_example(&bucket_name_dst, remote_target_arn);

    let resp: Result<PutBucketReplicationResponse, Error> = ctx
        .client
        .put_bucket_replication(&bucket_name_src)
        .replication_config(config.clone())
        .send()
        .await;
    match resp {
        Err(Error::S3Error(e)) => assert_eq!(e.code, ErrorCode::NotSupported),
        v => panic!("Expected error S3Error(NotSupported): but got {:?}", v),
    }

    let resp: Result<GetBucketReplicationResponse, Error> = ctx
        .client
        .get_bucket_replication(&bucket_name_src)
        .send()
        .await;
    match resp {
        Err(Error::S3Error(e)) => assert_eq!(e.code, ErrorCode::NotSupported),
        v => panic!("Expected error S3Error(NotSupported): but got {:?}", v),
    }

    let resp: Result<DeleteBucketReplicationResponse, Error> = ctx
        .client
        .delete_bucket_replication(&bucket_name_src)
        .send()
        .await;
    match resp {
        Err(Error::S3Error(e)) => assert_eq!(e.code, ErrorCode::NotSupported),
        v => panic!("Expected error S3Error(NotSupported): but got {:?}", v),
    }
}
