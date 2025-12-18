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

//! Client method for GetIndex operation.

use crate::s3::error::ValidationErr;
use crate::s3vectors::builders::{GetIndex, GetIndexBldr};
use crate::s3vectors::client::VectorsClient;
use crate::s3vectors::types::{BucketName, IndexName};

impl VectorsClient {
    /// Creates a builder for the GetIndex operation.
    ///
    /// # Arguments
    ///
    /// * `bucket` - Name of the vector bucket
    /// * `index` - Name of the index
    ///
    /// # Example
    ///
    /// ```no_run
    /// use minio::s3vectors::{VectorsClient, VectorsApi};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client: VectorsClient = todo!();
    /// let resp = client
    ///     .get_index("my-bucket", "my-index")?
    ///     .build()
    ///     .send()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn get_index<B, I>(&self, bucket: B, index: I) -> Result<GetIndexBldr, ValidationErr>
    where
        B: TryInto<BucketName>,
        B::Error: Into<ValidationErr>,
        I: TryInto<IndexName>,
        I::Error: Into<ValidationErr>,
    {
        Ok(GetIndex::builder()
            .client(self.clone())
            .bucket(Some(bucket.try_into().map_err(Into::into)?))
            .index(Some(index.try_into().map_err(Into::into)?)))
    }
}
