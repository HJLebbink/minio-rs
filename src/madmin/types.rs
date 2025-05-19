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

pub mod bucket_target;

use std::mem;
use crate::madmin::madmin_client::MadminClient;
use crate::s3::error::{Error, ErrorCode};
use crate::s3::multimap::{Multimap, MultimapExt};
use crate::s3::segmented_bytes::SegmentedBytes;
use async_trait::async_trait;
use bytes::Bytes;
use futures_util::Stream;
use http::{HeaderMap, Method};
use reqwest::Body;
use crate::s3::signer::sign_v4_s3;
use crate::s3::utils::{sha256_hash_sb, to_amz_date, utc_now, EMPTY_SHA256};

#[derive(Clone, Default, Debug)]
/// Generic MinIO Admin Request
pub struct MadminRequest {
    pub(crate) client: MadminClient,

    method: Method,
    pub(crate) bucket: Option<String>,
    pub(crate) query_params: Multimap,
    headers: Multimap,
    body: Option<SegmentedBytes>,
}

impl MadminRequest {
    pub fn new(client: MadminClient, method: Method) -> Self {
        Self {
            client,
            method,
            ..Default::default()
        }
    }

    pub fn bucket(mut self, bucket: Option<String>) -> Self {
        self.bucket = bucket;
        self
    }

    pub fn query_params(mut self, query_params: Multimap) -> Self {
        self.query_params = query_params;
        self
    }

    pub fn headers(mut self, headers: Multimap) -> Self {
        self.headers = headers;
        self
    }

    pub fn body(mut self, body: Option<SegmentedBytes>) -> Self {
        self.body = body;
        self
    }

    pub async fn execute(&self) -> Result<reqwest::Response, Error> {
        
        let method = self.method.clone();
        let bucket = self.bucket.clone();
        let mut headers = self.headers.clone();
        let body  = self.body.clone();

        let url = self.client.shared.base_url.build_url(
            &self.method,
            &self.query_params);


        {
            headers.add("Host", url.host_header_value());

            let sha256: String = match method {
                Method::PUT | Method::POST => {
                    if !headers.contains_key("Content-Type") {
                        headers.add("Content-Type", "application/octet-stream");
                    }
                    let len: usize = body.as_ref().map_or(0, |b| b.len());
                    headers.add("Content-Length", len.to_string());

                    match body {
                        None => EMPTY_SHA256.into(),
                        Some(v) => sha256_hash_sb(v),
                    }
                }
                _ => EMPTY_SHA256.into(),
            };
            headers.add("x-amz-content-sha256", sha256.clone());

            let date = utc_now();
            headers.add("x-amz-date", to_amz_date(date));

            if let Some(p) = &self.shared.provider {
                let creds = p.fetch();
                if creds.session_token.is_some() {
                    headers.add("X-Amz-Security-Token", creds.session_token.unwrap());
                }
                sign_v4_s3(
                    method,
                    &url.path,
                    region,
                    headers,
                    query_params,
                    &creds.access_key,
                    &creds.secret_key,
                    &sha256,
                    date,
                );
            }
        }

        let mut req = self.http_client.request(method.clone(), url.to_string());

        for (key, values) in headers.iter_all() {
            for value in values {
                req = req.header(key, value);
            }
        }

        if false {
            let mut header_strings: Vec<String> = headers
                .iter_all()
                .map(|(k, v)| format!("{}: {}", k, v.join(",")))
                .collect();

            // Sort headers alphabetically by name
            header_strings.sort();

            let body_str: String =
                String::from_utf8(body.unwrap_or(&SegmentedBytes::new()).to_bytes().to_vec())?;

            println!(
                "S3 request: {} url={:?}; headers={:?}; body={}\n",
                method,
                url.path,
                header_strings.join("; "),
                body_str
            );
        }

        if (method == Method::PUT) || (method == Method::POST) {
            //TODO: why-oh-why first collect into a vector and then iterate to a stream?
            let bytes_vec: Vec<Bytes> = match body {
                Some(v) => v.into_iter().collect(),
                None => Vec::new(),
            };
            let stream = futures_util::stream::iter(
                bytes_vec
                    .into_iter()
                    .map(|b| -> Result<_, std::io::Error> { Ok(b) }),
            );
            req = req.body(Body::wrap_stream(stream));
        }

        let resp: reqwest::Response = req.send().await?;

        if resp.status().is_success() {
            return Ok(resp);
        }

        let mut resp = resp;
        let status_code = resp.status().as_u16();
        let headers: HeaderMap = mem::take(resp.headers_mut());
        let body: Bytes = resp.bytes().await?;

        let e: Error = self.shared.get_error_response(
            body,
            status_code,
            headers,
            method,
            &url.path,
            bucket_name,
            object_name,
            retry,
        );

        if let Error::S3Error(ref err) = e {
            if (err.code == ErrorCode::NoSuchBucket) || (err.code == ErrorCode::RetryHead) {
                if let Some(v) = bucket_name {
                    self.shared.region_map.remove(v);
                }
            }
        };

        Err(e)
    }
}

pub trait ToMadminRequest: Sized {
    fn to_madmin_request(self) -> Result<MadminRequest, Error>;
}

#[async_trait]
pub trait FromMadminResponse: Sized {
    async fn from_madmin_response(
        req: MadminRequest,
        resp: Result<reqwest::Response, Error>,
    ) -> Result<Self, Error>;
}

#[async_trait]
pub trait MadminApi: ToMadminRequest {
    type MadminResponse: FromMadminResponse;

    async fn send(self) -> Result<Self::MadminResponse, Error> {
        let req: MadminRequest = self.to_madmin_request()?;
        let resp: Result<reqwest::Response, Error> = req.execute().await;
        Self::MadminResponse::from_madmin_response(req, resp).await
    }
}

#[async_trait]
pub trait ToStream: Sized {
    type Item;
    async fn to_stream(self) -> Box<dyn Stream<Item = Result<Self::Item, Error>> + Unpin + Send>;
}
