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

use crate::madmin::madmin_client::MadminClient;
use crate::madmin::response::ListRemoteTargetsResponse;
use crate::madmin::types::{MadminApi, MadminRequest, ToMadminRequest};
use crate::s3::error::Error;
use crate::s3::multimap::{Multimap, MultimapExt};
use crate::s3::utils::check_bucket_name;
use http::Method;

#[derive(Clone, Debug, Default)]
pub struct ListRemoteTargets {
    client: MadminClient,
    extra_headers: Option<Multimap>,
    extra_query_params: Option<Multimap>,
    bucket: String,
    arn_type: String,
}

impl ListRemoteTargets {
    pub fn new(client: MadminClient, bucket: String, arn_type: String) -> Self {
        Self {
            client,
            bucket,
            arn_type,
            ..Default::default()
        }
    }

    pub fn extra_headers(mut self, extra_headers: Option<Multimap>) -> Self {
        self.extra_headers = extra_headers;
        self
    }

    pub fn extra_query_params(mut self, extra_query_params: Option<Multimap>) -> Self {
        self.extra_query_params = extra_query_params;
        self
    }
}

impl MadminApi for ListRemoteTargets {
    type MadminResponse = ListRemoteTargetsResponse;
}

impl ToMadminRequest for ListRemoteTargets {
    fn to_madmin_request(self) -> Result<MadminRequest, Error> {
        check_bucket_name(&self.bucket, true)?;

        let mut query_params: Multimap = self.extra_query_params.unwrap_or_default();
        query_params.add("bucket", self.bucket);
        query_params.add("type", self.arn_type);

        Ok(MadminRequest::new(self.client, Method::GET)
            .query_params(query_params)
            .headers(self.extra_headers.unwrap_or_default()))
    }
}
