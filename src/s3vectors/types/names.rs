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

//! Name wrapper types for S3Vectors resources.

use serde::{Deserialize, Serialize};
use std::fmt;

use crate::s3vectors::error::VectorsValidationErr;

/// Vector bucket ARN.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct VectorBucketArn(String);

impl VectorBucketArn {
    /// Creates a new vector bucket ARN.
    pub fn new(arn: impl Into<String>) -> Self {
        Self(arn.into())
    }

    /// Returns the ARN as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consumes the wrapper and returns the inner string.
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl fmt::Display for VectorBucketArn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for VectorBucketArn {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for VectorBucketArn {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl AsRef<str> for VectorBucketArn {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// Vector index name (3-63 characters).
///
/// Index names must:
/// - Be between 3 and 63 characters long
/// - Start with a lowercase letter or number
/// - Contain only lowercase letters, numbers, hyphens, and periods
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct IndexName(String);

impl IndexName {
    /// Minimum allowed length for index names.
    pub const MIN_LENGTH: usize = 3;

    /// Maximum allowed length for index names.
    pub const MAX_LENGTH: usize = 63;

    /// Creates a new index name after validation.
    pub fn new(name: impl Into<String>) -> Result<Self, VectorsValidationErr> {
        let name = name.into();
        Self::validate(&name)?;
        Ok(Self(name))
    }

    /// Returns the name as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consumes the wrapper and returns the inner string.
    pub fn into_inner(self) -> String {
        self.0
    }

    fn validate(name: &str) -> Result<(), VectorsValidationErr> {
        if name.len() < Self::MIN_LENGTH {
            return Err(VectorsValidationErr::InvalidIndexName {
                name: name.to_string(),
                reason: format!("name must be at least {} characters", Self::MIN_LENGTH),
            });
        }

        if name.len() > Self::MAX_LENGTH {
            return Err(VectorsValidationErr::InvalidIndexName {
                name: name.to_string(),
                reason: format!("name must be at most {} characters", Self::MAX_LENGTH),
            });
        }

        let first_char = name.chars().next().unwrap();
        if !first_char.is_ascii_lowercase() && !first_char.is_ascii_digit() {
            return Err(VectorsValidationErr::InvalidIndexName {
                name: name.to_string(),
                reason: "name must start with a lowercase letter or digit".to_string(),
            });
        }

        for c in name.chars() {
            if !c.is_ascii_lowercase() && !c.is_ascii_digit() && c != '-' && c != '.' {
                return Err(VectorsValidationErr::InvalidIndexName {
                    name: name.to_string(),
                    reason: format!(
                        "invalid character '{}'; only lowercase letters, digits, hyphens, and periods allowed",
                        c
                    ),
                });
            }
        }

        Ok(())
    }
}

impl fmt::Display for IndexName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<&str> for IndexName {
    type Error = VectorsValidationErr;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<String> for IndexName {
    type Error = VectorsValidationErr;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<&IndexName> for IndexName {
    type Error = VectorsValidationErr;

    fn try_from(value: &IndexName) -> Result<Self, Self::Error> {
        Ok(value.clone())
    }
}

impl AsRef<str> for IndexName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// Vector index ARN.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct IndexArn(String);

impl IndexArn {
    /// Creates a new index ARN.
    pub fn new(arn: impl Into<String>) -> Self {
        Self(arn.into())
    }

    /// Returns the ARN as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consumes the wrapper and returns the inner string.
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl fmt::Display for IndexArn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for IndexArn {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for IndexArn {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl AsRef<str> for IndexArn {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// Vector key (1-1024 characters).
///
/// A unique identifier for a vector within an index.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct VectorKey(String);

impl VectorKey {
    /// Minimum allowed length for vector keys.
    pub const MIN_LENGTH: usize = 1;

    /// Maximum allowed length for vector keys.
    pub const MAX_LENGTH: usize = 1024;

    /// Creates a new vector key after validation.
    pub fn new(key: impl Into<String>) -> Result<Self, VectorsValidationErr> {
        let key = key.into();
        Self::validate(&key)?;
        Ok(Self(key))
    }

    /// Returns the key as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consumes the wrapper and returns the inner string.
    pub fn into_inner(self) -> String {
        self.0
    }

    fn validate(key: &str) -> Result<(), VectorsValidationErr> {
        if key.len() < Self::MIN_LENGTH {
            return Err(VectorsValidationErr::InvalidVectorKey {
                key: key.to_string(),
                reason: format!("key must be at least {} character", Self::MIN_LENGTH),
            });
        }

        if key.len() > Self::MAX_LENGTH {
            return Err(VectorsValidationErr::InvalidVectorKey {
                key: key.to_string(),
                reason: format!("key must be at most {} characters", Self::MAX_LENGTH),
            });
        }

        Ok(())
    }
}

impl fmt::Display for VectorKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<&str> for VectorKey {
    type Error = VectorsValidationErr;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<String> for VectorKey {
    type Error = VectorsValidationErr;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<&VectorKey> for VectorKey {
    type Error = VectorsValidationErr;

    fn try_from(value: &VectorKey) -> Result<Self, Self::Error> {
        Ok(value.clone())
    }
}

impl AsRef<str> for VectorKey {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// Vector dimension (1-4096).
///
/// Represents the number of dimensions in a vector. Must be between 1 and 4096.
///
/// # Example
///
/// ```
/// use minio::s3vectors::Dimension;
///
/// let dim = Dimension::new(1024).unwrap();
/// assert_eq!(dim.as_u32(), 1024);
///
/// // Zero is invalid
/// assert!(Dimension::new(0).is_err());
///
/// // Over 4096 is invalid
/// assert!(Dimension::new(5000).is_err());
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Dimension(u32);

impl Dimension {
    /// Minimum allowed dimension.
    pub const MIN: u32 = 1;

    /// Maximum allowed dimension.
    pub const MAX: u32 = 4096;

    /// Creates a new Dimension value after validation.
    ///
    /// # Errors
    ///
    /// Returns an error if the value is less than 1 or greater than 4096.
    pub fn new(value: u32) -> Result<Self, VectorsValidationErr> {
        if !(Self::MIN..=Self::MAX).contains(&value) {
            return Err(VectorsValidationErr::InvalidDimension {
                dimension: value,
                reason: format!("must be between {} and {}", Self::MIN, Self::MAX),
            });
        }
        Ok(Self(value))
    }

    /// Returns the value as u32.
    pub fn as_u32(&self) -> u32 {
        self.0
    }

    /// Consumes the wrapper and returns the inner value.
    pub fn into_inner(self) -> u32 {
        self.0
    }
}

impl fmt::Display for Dimension {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<u32> for Dimension {
    type Error = VectorsValidationErr;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<usize> for Dimension {
    type Error = VectorsValidationErr;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        Self::new(value as u32)
    }
}

impl TryFrom<i32> for Dimension {
    type Error = VectorsValidationErr;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        if value < 0 {
            return Err(VectorsValidationErr::InvalidDimension {
                dimension: 0,
                reason: "dimension cannot be negative".to_string(),
            });
        }
        Self::new(value as u32)
    }
}

impl From<&Dimension> for Dimension {
    fn from(value: &Dimension) -> Self {
        *value
    }
}

impl From<Dimension> for u32 {
    fn from(value: Dimension) -> Self {
        value.0
    }
}

/// Number of results to return (top K nearest neighbors).
///
/// TopK must be at least 1. This type validates the value at construction time.
///
/// # Example
///
/// ```
/// use minio::s3vectors::TopK;
///
/// let k = TopK::new(10).unwrap();
/// assert_eq!(k.as_u32(), 10);
///
/// // Zero is invalid
/// assert!(TopK::new(0).is_err());
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TopK(u32);

impl TopK {
    /// Creates a new TopK value after validation.
    ///
    /// # Errors
    ///
    /// Returns an error if the value is less than 1.
    pub fn new(value: u32) -> Result<Self, VectorsValidationErr> {
        if value < 1 {
            return Err(VectorsValidationErr::InvalidTopK(value));
        }
        Ok(Self(value))
    }

    /// Returns the value as u32.
    pub fn as_u32(&self) -> u32 {
        self.0
    }

    /// Consumes the wrapper and returns the inner value.
    pub fn into_inner(self) -> u32 {
        self.0
    }
}

impl fmt::Display for TopK {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<u32> for TopK {
    type Error = VectorsValidationErr;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<usize> for TopK {
    type Error = VectorsValidationErr;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        Self::new(value as u32)
    }
}

impl TryFrom<i32> for TopK {
    type Error = VectorsValidationErr;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        if value < 0 {
            return Err(VectorsValidationErr::InvalidTopK(0));
        }
        Self::new(value as u32)
    }
}

impl From<&TopK> for TopK {
    fn from(value: &TopK) -> Self {
        *value
    }
}

impl From<TopK> for u32 {
    fn from(value: TopK) -> Self {
        value.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_index_name_valid() {
        assert!(IndexName::new("idx").is_ok());
        assert!(IndexName::new("my-index").is_ok());
        assert!(IndexName::new("embeddings.v1").is_ok());
    }

    #[test]
    fn test_index_name_invalid() {
        assert!(IndexName::new("ab").is_err());
        assert!(IndexName::new("a".repeat(64)).is_err());
        assert!(IndexName::new("INDEX").is_err());
    }

    #[test]
    fn test_vector_key_valid() {
        assert!(VectorKey::new("a").is_ok());
        assert!(VectorKey::new("doc-123").is_ok());
        assert!(VectorKey::new("a".repeat(1024)).is_ok());
    }

    #[test]
    fn test_vector_key_invalid() {
        assert!(VectorKey::new("").is_err());
        assert!(VectorKey::new("a".repeat(1025)).is_err());
    }

    #[test]
    fn test_top_k_valid() {
        assert!(TopK::new(1).is_ok());
        assert!(TopK::new(10).is_ok());
        assert!(TopK::new(100).is_ok());
        assert_eq!(TopK::new(42).unwrap().as_u32(), 42);
    }

    #[test]
    fn test_top_k_invalid() {
        assert!(TopK::new(0).is_err());
    }

    #[test]
    fn test_top_k_try_from() {
        assert!(TopK::try_from(5u32).is_ok());
        assert!(TopK::try_from(5usize).is_ok());
        assert!(TopK::try_from(5i32).is_ok());
        assert!(TopK::try_from(0u32).is_err());
        assert!(TopK::try_from(-1i32).is_err());
    }

    #[test]
    fn test_dimension_valid() {
        assert!(Dimension::new(1).is_ok());
        assert!(Dimension::new(1024).is_ok());
        assert!(Dimension::new(4096).is_ok());
        assert_eq!(Dimension::new(512).unwrap().as_u32(), 512);
    }

    #[test]
    fn test_dimension_invalid() {
        assert!(Dimension::new(0).is_err());
        assert!(Dimension::new(4097).is_err());
        assert!(Dimension::new(10000).is_err());
    }

    #[test]
    fn test_dimension_try_from() {
        assert!(Dimension::try_from(1024u32).is_ok());
        assert!(Dimension::try_from(1024usize).is_ok());
        assert!(Dimension::try_from(1024i32).is_ok());
        assert!(Dimension::try_from(0u32).is_err());
        assert!(Dimension::try_from(-1i32).is_err());
        assert!(Dimension::try_from(5000u32).is_err());
    }
}
