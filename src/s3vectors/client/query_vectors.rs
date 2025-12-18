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

//! Client method for QueryVectors operation.

use crate::s3::error::ValidationErr;
use crate::s3vectors::builders::{QueryVectors, QueryVectorsBldr};
use crate::s3vectors::client::VectorsClient;
use crate::s3vectors::types::{BucketName, IndexName, TopK, VectorData};

impl VectorsClient {
    /// Creates a builder for the QueryVectors operation.
    ///
    /// Performs an approximate nearest neighbor search.
    ///
    /// # Arguments
    ///
    /// * `bucket` - Name of the vector bucket
    /// * `index` - Name of the index
    /// * `query_vector` - Query vector (must match index dimensions)
    /// * `top_k` - Number of results to return
    ///
    /// # Example
    ///
    /// ```no_run
    /// use minio::s3vectors::{VectorsClient, VectorData, VectorsApi};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client: VectorsClient = todo!();
    /// let query = VectorData::new(vec![0.1, 0.2, 0.3])?;
    /// let resp = client
    ///     .query_vectors("my-bucket", "my-index", query, 10)?
    ///     .build()
    ///     .send()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn query_vectors<B, I, K>(
        &self,
        bucket: B,
        index: I,
        query_vector: VectorData,
        top_k: K,
    ) -> Result<QueryVectorsBldr, ValidationErr>
    where
        B: TryInto<BucketName>,
        B::Error: Into<ValidationErr>,
        I: TryInto<IndexName>,
        I::Error: Into<ValidationErr>,
        K: TryInto<TopK>,
        K::Error: Into<ValidationErr>,
    {
        Ok(QueryVectors::builder()
            .client(self.clone())
            .bucket(Some(bucket.try_into().map_err(Into::into)?))
            .index(Some(index.try_into().map_err(Into::into)?))
            .query_vector(query_vector)
            .top_k(top_k.try_into().map_err(Into::into)?))
    }
}
