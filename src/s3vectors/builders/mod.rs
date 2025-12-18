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

//! Request builders for S3Vectors API operations.

// Vector Bucket operations
mod create_vector_bucket;
mod delete_vector_bucket;
mod delete_vector_bucket_policy;
mod get_vector_bucket;
mod get_vector_bucket_policy;
mod list_vector_buckets;
mod put_vector_bucket_policy;

// Vector Index operations
mod create_index;
mod delete_index;
mod get_index;
mod list_indexes;

// Vector Data operations
mod delete_vectors;
mod get_vectors;
mod list_vectors;
mod put_vectors;
mod query_vectors;

// Tagging operations
mod list_tags_for_resource;
mod tag_resource;
mod untag_resource;

// Re-export all builders
pub use create_index::{CreateIndex, CreateIndexBldr};
pub use create_vector_bucket::{CreateVectorBucket, CreateVectorBucketBldr};
pub use delete_index::{DeleteIndex, DeleteIndexBldr};
pub use delete_vector_bucket::{DeleteVectorBucket, DeleteVectorBucketBldr};
pub use delete_vector_bucket_policy::{DeleteVectorBucketPolicy, DeleteVectorBucketPolicyBldr};
pub use delete_vectors::{DeleteVectors, DeleteVectorsBldr};
pub use get_index::{GetIndex, GetIndexBldr};
pub use get_vector_bucket::{GetVectorBucket, GetVectorBucketBldr};
pub use get_vector_bucket_policy::{GetVectorBucketPolicy, GetVectorBucketPolicyBldr};
pub use get_vectors::{GetVectors, GetVectorsBldr};
pub use list_indexes::{ListIndexes, ListIndexesBldr};
pub use list_tags_for_resource::{ListTagsForResource, ListTagsForResourceBldr};
pub use list_vector_buckets::{ListVectorBuckets, ListVectorBucketsBldr};
pub use list_vectors::{ListVectors, ListVectorsBldr};
pub use put_vector_bucket_policy::{PutVectorBucketPolicy, PutVectorBucketPolicyBldr};
pub use put_vectors::{PutVectors, PutVectorsBldr};
pub use query_vectors::{QueryVectors, QueryVectorsBldr};
pub use tag_resource::{TagResource, TagResourceBldr};
pub use untag_resource::{UntagResource, UntagResourceBldr};
