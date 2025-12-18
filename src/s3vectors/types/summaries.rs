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

//! Summary types for list operations.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::{
    BucketName, DataType, DistanceMetric, IndexArn, IndexName, IndexStatus, VectorBucketArn,
};

/// Custom deserialization for epoch time (Unix timestamp as float)
/// The MinIO server sends creation times as epoch seconds (float), not RFC 3339 strings.
mod epoch_time {
    use super::*;

    pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Error;

        // Try to deserialize as f64 (epoch seconds)
        let epoch: f64 = f64::deserialize(deserializer)?;
        let secs = epoch.trunc() as i64;
        let nsecs = ((epoch.fract()) * 1_000_000_000.0) as u32;

        DateTime::from_timestamp(secs, nsecs)
            .ok_or_else(|| D::Error::custom(format!("Invalid timestamp: {}", epoch)))
    }

    pub fn serialize<S>(dt: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let epoch = dt.timestamp() as f64 + (dt.timestamp_subsec_nanos() as f64 / 1_000_000_000.0);
        serializer.serialize_f64(epoch)
    }
}

/// Summary information for a vector bucket returned by ListVectorBuckets.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VectorBucketSummary {
    /// The name of the vector bucket.
    #[serde(rename = "vectorBucketName")]
    pub bucket: BucketName,

    /// The ARN of the vector bucket.
    pub vector_bucket_arn: VectorBucketArn,

    /// The creation time of the vector bucket (epoch seconds).
    #[serde(with = "epoch_time")]
    pub creation_time: DateTime<Utc>,
}

/// Summary information for a vector index returned by ListIndexes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexSummary {
    /// The name of the vector index.
    #[serde(rename = "indexName")]
    pub index: IndexName,

    /// The ARN of the vector index.
    pub index_arn: IndexArn,

    /// The creation time of the vector index (epoch seconds).
    #[serde(with = "epoch_time")]
    pub creation_time: DateTime<Utc>,
}

/// Full details for a vector bucket returned by GetVectorBucket.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VectorBucket {
    /// The name of the vector bucket.
    #[serde(rename = "vectorBucketName")]
    pub bucket: BucketName,

    /// The ARN of the vector bucket.
    pub vector_bucket_arn: VectorBucketArn,

    /// The creation time of the vector bucket (epoch seconds).
    #[serde(with = "epoch_time")]
    pub creation_time: DateTime<Utc>,

    /// The encryption configuration for the vector bucket.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encryption_configuration: Option<super::EncryptionConfiguration>,
}

/// Full details for a vector index returned by GetIndex.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Index {
    /// The name of the vector index.
    #[serde(rename = "indexName")]
    pub index: IndexName,

    /// The ARN of the vector index.
    pub index_arn: IndexArn,

    /// The name of the vector bucket containing this index.
    #[serde(rename = "vectorBucketName")]
    pub bucket: BucketName,

    /// The creation time of the vector index (epoch seconds).
    #[serde(with = "epoch_time")]
    pub creation_time: DateTime<Utc>,

    /// The dimension of vectors in this index.
    pub dimension: u32,

    /// The distance metric used for similarity search.
    pub distance_metric: DistanceMetric,

    /// The data type of vectors in this index.
    pub data_type: DataType,

    /// The number of vectors currently in the index.
    /// Use this to track indexing progress by comparing against your expected total.
    #[serde(default)]
    pub vector_count: i64,

    /// The current status of the index.
    /// - `Creating`: Index is being created
    /// - `Active`: Index is ready for queries
    /// - `Deleting`: Index is being deleted
    #[serde(default)]
    pub status: IndexStatus,

    /// The encryption configuration for the index.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encryption_configuration: Option<super::EncryptionConfiguration>,

    /// The metadata configuration for the index.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata_configuration: Option<super::MetadataConfiguration>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vector_bucket_summary_serialization() {
        let summary = VectorBucketSummary {
            bucket: BucketName::new("test-bucket").unwrap(),
            vector_bucket_arn: VectorBucketArn::new(
                "arn:aws:s3vectors:us-east-1:123456789012:bucket/test-bucket",
            ),
            creation_time: Utc::now(),
        };

        let json = serde_json::to_string(&summary).unwrap();
        let parsed: VectorBucketSummary = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.bucket.as_str(), "test-bucket");
    }

    #[test]
    fn test_index_summary_serialization() {
        let summary = IndexSummary {
            index: IndexName::new("test-index").unwrap(),
            index_arn: IndexArn::new(
                "arn:aws:s3vectors:us-east-1:123456789012:bucket/test-bucket/index/test-index",
            ),
            creation_time: Utc::now(),
        };

        let json = serde_json::to_string(&summary).unwrap();
        let parsed: IndexSummary = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.index.as_str(), "test-index");
    }
}
