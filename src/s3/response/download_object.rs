// MinIO Rust Library for Amazon S3 Compatible Cloud Storage
// Copyright 2022-2025 MinIO, Inc.
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

//! Responses for DownloadObject APIs.

use crate::s3::error::Error;
use crate::s3::types::{FromS3Response, S3Request};
use crate::s3::utils::{get_default_text, get_option_text, get_text};
use async_trait::async_trait;
use bytes::Buf;
use http::HeaderMap;
use xmltree::Element;

#[derive(Debug, Clone)]
/// Response of [download_object()](crate::s3::client::Client::download_object) API
pub struct DownloadObjectResponse {
    pub headers: HeaderMap,
    pub region: String,
    pub bucket_name: String,
    pub object_name: String,
    /// If a delete marker was requested, this field will contain the version_id
    /// of the delete marker. Value of the `x-amz-version-id` header.
    pub version_id: Option<String>,
}

#[async_trait]
impl FromS3Response for DownloadObjectResponse {
    async fn from_s3response<'a>(
        req: S3Request<'a>,
        resp: reqwest::Response,
    ) -> Result<Self, Error> {
        let headers = resp.headers().clone();
        let version_id = headers
            .get("x-amz-version-id")
            .map(|v| v.to_str().unwrap().to_string());

        Ok(DownloadObjectResponse {
            headers,
            region: req.region.unwrap_or("").to_string(),
            bucket_name: req.bucket.unwrap().to_string(),
            object_name: req.object.unwrap().to_string(),
            version_id,
        })
    }
}

/// Error info returned by the S3 API when an object could not be downloaded.
#[derive(Clone, Debug)]
pub struct DownloadError {
    pub code: String,
    pub message: String,
    pub object_name: String,
    pub version_id: Option<String>,
}

/// Information about an object that was downloaded.
#[derive(Clone, Debug)]
pub struct DownloadedObject {
    pub name: String,
    pub version_id: Option<String>,
    pub delete_marker: bool,
    pub delete_marker_version_id: Option<String>,
}

/// Response of
/// [download_objects()](crate::s3::client_core::ClientCore::download_objects)
/// S3 API.
#[derive(Clone, Debug)]
pub struct DownloadObjectsResponse {
    pub headers: HeaderMap,
    pub result: Vec<DownloadResult>,
}

/// Result of downloading an object.
#[derive(Clone, Debug)]
pub enum DownloadResult {
    Downloaded(DownloadedObject),
    Error(DownloadError),
}

impl From<DownloadResult> for Result<DownloadedObject, DownloadError> {
    fn from(result: DownloadResult) -> Self {
        match result {
            DownloadResult::Downloaded(obj) => Ok(obj),
            DownloadResult::Error(err) => Err(err),
        }
    }
}

impl DownloadResult {
    pub fn is_deleted(&self) -> bool {
        matches!(self, DownloadResult::Downloaded(_))
    }

    pub fn is_error(&self) -> bool {
        matches!(self, DownloadResult::Error(_))
    }
}

#[async_trait]
impl FromS3Response for DownloadObjectsResponse {
    async fn from_s3response<'a>(
        _req: S3Request<'a>,
        resp: reqwest::Response,
    ) -> Result<Self, Error> {
        let headers = resp.headers().clone();

        let body = resp.bytes().await?;

        let root = Element::parse(body.reader())?;
        let result = root
            .children
            .iter()
            .map(|elem| elem.as_element().unwrap())
            .map(|elem| {
                if elem.name == "Deleted" {
                    Ok(DownloadResult::Downloaded(DownloadedObject {
                        name: get_text(elem, "Key")?,
                        version_id: get_option_text(elem, "VersionId"),
                        delete_marker: get_default_text(elem, "DeleteMarker").to_lowercase()
                            == "true",
                        delete_marker_version_id: get_option_text(elem, "DeleteMarkerVersionId"),
                    }))
                } else {
                    assert_eq!(elem.name, "Error");
                    Ok(DownloadResult::Error(DownloadError {
                        code: get_text(elem, "Code")?,
                        message: get_text(elem, "Message")?,
                        object_name: get_text(elem, "Key")?,
                        version_id: get_option_text(elem, "VersionId"),
                    }))
                }
            })
            .collect::<Result<Vec<DownloadResult>, Error>>()?;

        Ok(DownloadObjectsResponse { headers, result })
    }
}
