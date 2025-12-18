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

//! Client method for TagResource operation.

use crate::s3vectors::builders::{TagResource, TagResourceBldr};
use crate::s3vectors::client::VectorsClient;
use std::collections::HashMap;

impl VectorsClient {
    /// Creates a builder for the TagResource operation.
    ///
    /// # Arguments
    ///
    /// * `resource_arn` - ARN of the resource to tag
    /// * `tags` - Tags to add
    ///
    /// # Example
    ///
    /// ```no_run
    /// use minio::s3vectors::{VectorsClient, VectorsApi};
    /// use std::collections::HashMap;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client: VectorsClient = todo!();
    /// let mut tags = HashMap::new();
    /// tags.insert("env".to_string(), "production".to_string());
    /// let resp = client
    ///     .tag_resource("arn:aws:s3vectors:us-east-1:123456789012:bucket/my-bucket", tags)
    ///     .build()
    ///     .send()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn tag_resource(
        &self,
        resource_arn: impl Into<String>,
        tags: HashMap<String, String>,
    ) -> TagResourceBldr {
        TagResource::builder()
            .client(self.clone())
            .resource_arn(resource_arn.into())
            .tags(tags)
    }
}
