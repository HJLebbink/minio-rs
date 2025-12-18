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

//! Response types for S3Vectors API operations.

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

// Re-export all response types
pub use create_index::CreateIndexResponse;
pub use create_vector_bucket::CreateVectorBucketResponse;
pub use delete_index::DeleteIndexResponse;
pub use delete_vector_bucket::DeleteVectorBucketResponse;
pub use delete_vector_bucket_policy::DeleteVectorBucketPolicyResponse;
pub use delete_vectors::DeleteVectorsResponse;
pub use get_index::GetIndexResponse;
pub use get_vector_bucket::GetVectorBucketResponse;
pub use get_vector_bucket_policy::GetVectorBucketPolicyResponse;
pub use get_vectors::GetVectorsResponse;
pub use list_indexes::ListIndexesResponse;
pub use list_tags_for_resource::ListTagsForResourceResponse;
pub use list_vector_buckets::ListVectorBucketsResponse;
pub use list_vectors::ListVectorsResponse;
pub use put_vector_bucket_policy::PutVectorBucketPolicyResponse;
pub use put_vectors::PutVectorsResponse;
pub use query_vectors::QueryVectorsResponse;
pub use tag_resource::TagResourceResponse;
pub use untag_resource::UntagResourceResponse;
