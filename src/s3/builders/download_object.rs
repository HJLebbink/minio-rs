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

//! APIs to download objects.

use std::any::Any;
use http::Method;
use crate::s3::builders::GetObject;
use crate::s3::utils::{check_bucket_name, merge, to_http_header_value, Multimap};
use crate::s3::Client;
use crate::s3::error::Error;
use crate::s3::response::GetObjectResponse;
use crate::s3::sse::SseCustomerKey;
use crate::s3::types::{S3Api, S3Request, ToS3Request};

#[derive(Clone, Debug)]
pub struct ObjectToDownload {
    key: String,
    version_id: Option<String>,
}

/// A key can be converted into a ObjectToDownload. The version_id is set to None.
impl From<&str> for ObjectToDownload {
    fn from(key: &str) -> Self {
        ObjectToDownload {
            key: key.to_string(),
            version_id: None,
        }
    }
}

/// A tuple of key and version_id can be converted into a ObjectToDownload.
impl From<(&str, &str)> for ObjectToDownload {
    fn from((key, version_id): (&str, &str)) -> Self {
        ObjectToDownload {
            key: key.to_string(),
            version_id: Some(version_id.to_string()),
        }
    }
}

/// A tuple of key and option version_id can be converted into a ObjectToDownload.
impl From<(&str, Option<&str>)> for ObjectToDownload {
    fn from((key, version_id): (&str, Option<&str>)) -> Self {
        ObjectToDownload {
            key: key.to_string(),
            version_id: version_id.map(|v| v.to_string()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DownloadObject {
    client: Option<Client>,

    extra_headers: Option<Multimap>,
    extra_query_params: Option<Multimap>,
    bucket: String,
    object: ObjectToDownload,
    version_id: Option<String>,
    region: Option<String>,
    ssec: Option<SseCustomerKey>,
}

impl DownloadObject {
    pub fn new(bucket: &str, object: impl Into<ObjectToDownload>, filename: &str) -> Self {
        Self {
            bucket: bucket.to_string(),
            object,
            ..Default::default()
        }
    }

    pub fn client(mut self, client: &Client) -> Self {
        self.client = Some(client.clone());
        self
    }

    pub fn extra_headers(mut self, extra_headers: Option<Multimap>) -> Self {
        self.extra_headers = extra_headers;
        self
    }

    pub fn extra_query_params(mut self, extra_query_params: Option<Multimap>) -> Self {
        self.extra_query_params = extra_query_params;
        self
    }

    pub fn region(mut self, region: Option<String>) -> Self {
        self.region = region;
        self
    }
}

// internal helpers
impl DownloadObject {
    fn get_headers(&self) -> Multimap {
        let mut headers = Multimap::new();

        if let Some(v) = &self.ssec {
            merge(&mut headers, &v.headers());
        }

        headers
    }
}

impl ToS3Request for DownloadObject {
    fn to_s3request(&self) -> Result<S3Request, Error> {
        check_bucket_name(&self.bucket, true)?;

        if self.object.is_empty() {
            return Err(Error::InvalidObjectName(String::from(
                "object name cannot be empty",
            )));
        }
        let client: &Client = self.client.as_ref().ok_or(Error::NoClientProvided)?;

        if self.ssec.is_some() && !client.is_secure() {
            return Err(Error::SseTlsRequired(None));
        }

        let mut headers = Multimap::new();
        if let Some(v) = &self.extra_headers {
            merge(&mut headers, v);
        }
        merge(&mut headers, &self.get_headers());

        let mut query_params = Multimap::new();
        if let Some(v) = &self.extra_query_params {
            merge(&mut query_params, v);
        }
        if let Some(v) = &self.version_id {
            query_params.insert(String::from("versionId"), v.to_string());
        }

        let req = S3Request::new(client, Method::GET)
            .region(self.region.as_deref())
            .bucket(Some(&self.bucket))
            .object(Some(&self.object))
            .query_params(query_params)
            .headers(headers);

        Ok(req)
    }
}

impl S3Api for GetObject {
    type S3Response = DownloadObjectResponse;
}
