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

//! Builder for ListVectorBuckets operation.

use crate::s3::error::ValidationErr;
use crate::s3vectors::client::VectorsClient;
use crate::s3vectors::response::ListVectorBucketsResponse;
use crate::s3vectors::types::{ToVectorsRequest, VectorsApi, VectorsRequest};
use typed_builder::TypedBuilder;

/// Builder for the [`ListVectorBuckets`](https://docs.aws.amazon.com/AmazonS3/latest/API/API_S3VectorBuckets_ListVectorBuckets.html) S3Vectors API operation.
///
/// Lists all vector buckets in the account.
#[derive(Clone, Debug, TypedBuilder)]
pub struct ListVectorBuckets {
    #[builder(!default)]
    client: VectorsClient,

    /// Maximum number of buckets to return.
    #[builder(default, setter(into))]
    max_results: Option<u32>,

    /// Continuation token for pagination.
    #[builder(default, setter(into))]
    next_token: Option<String>,

    /// Filter buckets by name prefix.
    #[builder(default, setter(into))]
    prefix: Option<String>,
}

/// Builder type alias for [`ListVectorBuckets`].
pub type ListVectorBucketsBldr = ListVectorBucketsBuilder<((VectorsClient,), (), (), ())>;

impl VectorsApi for ListVectorBuckets {
    type VectorsResponse = ListVectorBucketsResponse;
}

impl ToVectorsRequest for ListVectorBuckets {
    fn to_vectors_request(self) -> Result<VectorsRequest, ValidationErr> {
        let mut body = serde_json::json!({});

        if let Some(max) = self.max_results {
            body["maxResults"] = serde_json::Value::Number(max.into());
        }

        if let Some(token) = self.next_token {
            body["nextToken"] = serde_json::Value::String(token);
        }

        if let Some(prefix) = self.prefix {
            body["prefix"] = serde_json::Value::String(prefix);
        }

        Ok(VectorsRequest::builder()
            .client(self.client)
            .path("/ListVectorBuckets".to_string())
            .body(Some(body))
            .build())
    }
}
