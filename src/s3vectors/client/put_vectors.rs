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

//! Client method for PutVectors operation.

use crate::s3::error::ValidationErr;
use crate::s3vectors::builders::{PutVectors, PutVectorsBldr};
use crate::s3vectors::client::VectorsClient;
use crate::s3vectors::types::{BucketName, IndexName, Vector};

impl VectorsClient {
    /// Creates a builder for the PutVectors operation.
    ///
    /// # Arguments
    ///
    /// * `bucket` - Name of the vector bucket
    /// * `index` - Name of the index
    /// * `vectors` - Vectors to insert (1-500 per request)
    ///
    /// # Example
    ///
    /// ```no_run
    /// use minio::s3vectors::{VectorsClient, Vector, VectorKey, VectorData, VectorsApi};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client: VectorsClient = todo!();
    /// let vectors = vec![
    ///     Vector::new(VectorKey::new("vec-1")?, VectorData::new(vec![0.1, 0.2, 0.3])?),
    /// ];
    /// let resp = client
    ///     .put_vectors("my-bucket", "my-index", vectors)?
    ///     .build()
    ///     .send()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn put_vectors<B, I>(
        &self,
        bucket: B,
        index: I,
        vectors: Vec<Vector>,
    ) -> Result<PutVectorsBldr, ValidationErr>
    where
        B: TryInto<BucketName>,
        B::Error: Into<ValidationErr>,
        I: TryInto<IndexName>,
        I::Error: Into<ValidationErr>,
    {
        Ok(PutVectors::builder()
            .client(self.clone())
            .bucket(Some(bucket.try_into().map_err(Into::into)?))
            .index(Some(index.try_into().map_err(Into::into)?))
            .vectors(vectors))
    }
}
