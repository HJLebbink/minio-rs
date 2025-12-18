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

//! Client method for CreateIndex operation.

use crate::s3::error::ValidationErr;
use crate::s3vectors::builders::{CreateIndex, CreateIndexBldr};
use crate::s3vectors::client::VectorsClient;
use crate::s3vectors::types::{BucketName, Dimension, DistanceMetric, IndexName};

impl VectorsClient {
    /// Creates a builder for the CreateIndex operation.
    ///
    /// # Arguments
    ///
    /// * `bucket` - Name of the vector bucket
    /// * `index` - Name of the index to create (3-63 characters)
    /// * `dimension` - Dimension of vectors (1-4096)
    /// * `distance_metric` - Distance metric for similarity search
    ///
    /// # Example
    ///
    /// ```no_run
    /// use minio::s3vectors::{VectorsClient, DistanceMetric, VectorsApi};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client: VectorsClient = todo!();
    /// // Simple usage
    /// let resp = client
    ///     .create_index("my-bucket", "my-index", 128, DistanceMetric::Cosine)?
    ///     .build()
    ///     .send()
    ///     .await?;
    ///
    /// // With optional HNSW config
    /// use minio::s3vectors::HnswConfig;
    /// let hnsw = HnswConfig::builder().m(32).ef_construction(100).build();
    /// let resp = client
    ///     .create_index("my-bucket", "my-index", 128, DistanceMetric::Cosine)?
    ///     .hnsw_config(hnsw)
    ///     .build()
    ///     .send()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn create_index<B, I, D>(
        &self,
        bucket: B,
        index: I,
        dimension: D,
        distance_metric: DistanceMetric,
    ) -> Result<CreateIndexBldr, ValidationErr>
    where
        B: TryInto<BucketName>,
        B::Error: Into<ValidationErr>,
        I: TryInto<IndexName>,
        I::Error: Into<ValidationErr>,
        D: TryInto<Dimension>,
        D::Error: Into<ValidationErr>,
    {
        Ok(CreateIndex::builder()
            .client(self.clone())
            .bucket(Some(bucket.try_into().map_err(Into::into)?))
            .index(index.try_into().map_err(Into::into)?)
            .dimension(dimension.try_into().map_err(Into::into)?)
            .distance_metric(distance_metric))
    }
}
