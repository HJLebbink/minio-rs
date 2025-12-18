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

//! Client method for DeleteVectorBucketPolicy operation.

use crate::s3::error::ValidationErr;
use crate::s3vectors::builders::{DeleteVectorBucketPolicy, DeleteVectorBucketPolicyBldr};
use crate::s3vectors::client::VectorsClient;
use crate::s3vectors::types::BucketName;

impl VectorsClient {
    /// Creates a builder for the DeleteVectorBucketPolicy operation.
    ///
    /// # Arguments
    ///
    /// * `bucket` - Name of the vector bucket
    ///
    /// # Example
    ///
    /// ```no_run
    /// use minio::s3vectors::{VectorsClient, VectorsApi};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client: VectorsClient = todo!();
    /// let resp = client
    ///     .delete_vector_bucket_policy("my-bucket")?
    ///     .build()
    ///     .send()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn delete_vector_bucket_policy<B>(
        &self,
        bucket: B,
    ) -> Result<DeleteVectorBucketPolicyBldr, ValidationErr>
    where
        B: TryInto<BucketName>,
        B::Error: Into<ValidationErr>,
    {
        Ok(DeleteVectorBucketPolicy::builder()
            .client(self.clone())
            .bucket(Some(bucket.try_into().map_err(Into::into)?)))
    }
}
