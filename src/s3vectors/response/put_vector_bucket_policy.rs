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

//! Response for PutVectorBucketPolicy operation.

use crate::s3vectors::types::VectorsRequest;
use crate::{impl_from_vectors_response, impl_has_vectors_fields};
use bytes::Bytes;
use http::HeaderMap;

/// Response from the PutVectorBucketPolicy operation.
#[derive(Debug, Clone)]
pub struct PutVectorBucketPolicyResponse {
    request: VectorsRequest,
    headers: HeaderMap,
    body: Bytes,
}

impl_from_vectors_response!(PutVectorBucketPolicyResponse);
impl_has_vectors_fields!(PutVectorBucketPolicyResponse);
