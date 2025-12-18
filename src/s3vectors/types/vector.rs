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

//! Vector data types for S3Vectors API.

use serde::{Deserialize, Serialize};

use super::{Dimension, VectorKey};
use crate::s3vectors::error::VectorsValidationErr;

/// Vector data containing float32 values.
///
/// Vectors must have between 1 and 4096 dimensions.
/// For cosine distance metric, zero vectors (all zeros) are not allowed.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VectorData {
    /// The vector values as 32-bit floats.
    pub float32: Vec<f32>,
}

impl VectorData {
    /// Creates new vector data after validation.
    pub fn new(values: Vec<f32>) -> Result<Self, VectorsValidationErr> {
        Self::validate(&values)?;
        Ok(Self { float32: values })
    }

    /// Returns the dimension (number of values) in this vector.
    pub fn dimension(&self) -> usize {
        self.float32.len()
    }

    /// Returns true if all values are zero.
    pub fn is_zero(&self) -> bool {
        self.float32.iter().all(|&v| v == 0.0)
    }

    /// Returns true if any value is NaN.
    pub fn has_nan(&self) -> bool {
        self.float32.iter().any(|v| v.is_nan())
    }

    /// Returns true if any value is infinite.
    pub fn has_infinite(&self) -> bool {
        self.float32.iter().any(|v| v.is_infinite())
    }

    /// Validates vector data.
    pub fn validate(values: &[f32]) -> Result<(), VectorsValidationErr> {
        let len = values.len() as u32;
        if !(Dimension::MIN..=Dimension::MAX).contains(&len) {
            return Err(VectorsValidationErr::InvalidDimension {
                dimension: len,
                reason: format!("must be between {} and {}", Dimension::MIN, Dimension::MAX),
            });
        }

        for (i, &v) in values.iter().enumerate() {
            if v.is_nan() {
                return Err(VectorsValidationErr::InvalidVectorValue {
                    index: i,
                    reason: "NaN values are not allowed".to_string(),
                });
            }
            if v.is_infinite() {
                return Err(VectorsValidationErr::InvalidVectorValue {
                    index: i,
                    reason: "Infinite values are not allowed".to_string(),
                });
            }
        }

        Ok(())
    }

    /// Validates vector data for use with cosine distance metric.
    /// Zero vectors are not allowed for cosine similarity.
    pub fn validate_for_cosine(&self) -> Result<(), VectorsValidationErr> {
        if self.is_zero() {
            return Err(VectorsValidationErr::ZeroVectorNotAllowed);
        }
        Ok(())
    }
}

impl From<Vec<f32>> for VectorData {
    fn from(values: Vec<f32>) -> Self {
        Self { float32: values }
    }
}

impl AsRef<[f32]> for VectorData {
    fn as_ref(&self) -> &[f32] {
        &self.float32
    }
}

/// A vector to be stored in an index.
///
/// Contains a unique key, the vector data, and optional metadata.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Vector {
    /// Unique identifier for this vector within the index.
    pub key: VectorKey,

    /// The vector data.
    pub data: VectorData,

    /// Optional metadata associated with the vector.
    /// Metadata can be used for filtering during queries.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

impl Vector {
    /// Creates a new vector.
    pub fn new(key: VectorKey, data: VectorData) -> Self {
        Self {
            key,
            data,
            metadata: None,
        }
    }

    /// Creates a new vector with metadata.
    pub fn with_metadata(key: VectorKey, data: VectorData, metadata: serde_json::Value) -> Self {
        Self {
            key,
            data,
            metadata: Some(metadata),
        }
    }

    /// Returns the dimension of this vector.
    pub fn dimension(&self) -> usize {
        self.data.dimension()
    }
}

/// Vector returned from a query operation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct QueryOutputVector {
    /// The vector key.
    pub key: VectorKey,

    /// The computed distance from the query vector.
    /// Only present if `return_distance` was true in the query.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub distance: Option<f64>,

    /// The vector metadata.
    /// Only present if `return_metadata` was true in the query.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// Vector returned from a GetVectors operation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GetOutputVector {
    /// The vector key.
    pub key: VectorKey,

    /// The vector data.
    /// Only present if `return_data` was true in the request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<VectorData>,

    /// The vector metadata.
    /// Only present if `return_metadata` was true in the request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// Vector returned from a ListVectors operation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ListOutputVector {
    /// The vector key.
    pub key: VectorKey,

    /// The vector data.
    /// Only present if `return_data` was true in the request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<VectorData>,

    /// The vector metadata.
    /// Only present if `return_metadata` was true in the request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vector_data_valid() {
        let data = VectorData::new(vec![0.1, 0.2, 0.3]).unwrap();
        assert_eq!(data.dimension(), 3);
        assert!(!data.is_zero());
        assert!(!data.has_nan());
        assert!(!data.has_infinite());
    }

    #[test]
    fn test_vector_data_zero() {
        let data = VectorData::new(vec![0.0, 0.0, 0.0]).unwrap();
        assert!(data.is_zero());
        assert!(data.validate_for_cosine().is_err());
    }

    #[test]
    fn test_vector_data_invalid_nan() {
        let result = VectorData::new(vec![0.1, f32::NAN, 0.3]);
        assert!(result.is_err());
    }

    #[test]
    fn test_vector_data_invalid_infinite() {
        let result = VectorData::new(vec![0.1, f32::INFINITY, 0.3]);
        assert!(result.is_err());
    }

    #[test]
    fn test_vector_data_dimension_limits() {
        // Empty vector not allowed
        let result = VectorData::new(vec![]);
        assert!(result.is_err());

        // Max dimension allowed
        let data = VectorData::new(vec![0.1; 4096]).unwrap();
        assert_eq!(data.dimension(), 4096);

        // Over max not allowed
        let result = VectorData::new(vec![0.1; 4097]);
        assert!(result.is_err());
    }

    #[test]
    fn test_vector_serialization() {
        let key = VectorKey::new("test-key").unwrap();
        let data = VectorData::new(vec![0.1, 0.2, 0.3]).unwrap();
        let vector =
            Vector::with_metadata(key, data, serde_json::json!({"title": "Test Document"}));

        let json = serde_json::to_string(&vector).unwrap();
        let parsed: Vector = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.key.as_str(), "test-key");
        assert_eq!(parsed.data.float32, vec![0.1, 0.2, 0.3]);
        assert!(parsed.metadata.is_some());
    }
}
