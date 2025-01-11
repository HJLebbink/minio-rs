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

//! S3 APIs for downloading objects.

use crate::s3::builders::{DownloadObject, ObjectToDownload};

use super::Client;

impl Client {

    /*
    pub async fn get_object_old(
        &self,
        args: &GetObjectArgs<'_>,
    ) -> Result<reqwest::Response, Error> {
        if args.ssec.is_some() && !self.base_url.https {
            return Err(Error::SseTlsRequired(None));
        }

        let region = self.get_region(args.bucket, args.region).await?;

        let mut headers = Multimap::new();
        if let Some(v) = &args.extra_headers {
            merge(&mut headers, v);
        }
        merge(&mut headers, &args.get_headers());

        let mut query_params = Multimap::new();
        if let Some(v) = &args.extra_query_params {
            merge(&mut query_params, v);
        }
        if let Some(v) = args.version_id {
            query_params.insert(String::from("versionId"), v.to_string());
        }

        self.execute(
            Method::GET,
            &region,
            &mut headers,
            &query_params,
            Some(args.bucket),
            Some(args.object),
            None,
        )
        .await
    }

    pub async fn download_object_old(
        &self,
        args: &DownloadObjectArgsOld<'_>,
    ) -> Result<DownloadObjectResponseOld, Error> {
        let mut resp = self
            .get_object_old(&GetObjectArgs {
                extra_headers: args.extra_headers,
                extra_query_params: args.extra_query_params,
                region: args.region,
                bucket: args.bucket,
                object: args.object,
                version_id: args.version_id,
                ssec: args.ssec,
                offset: None,
                length: None,
                match_etag: None,
                not_match_etag: None,
                modified_since: None,
                unmodified_since: None,
            })
            .await?;
        let path = Path::new(&args.filename);
        if let Some(parent_dir) = path.parent() {
            if !parent_dir.exists() {
                fs::create_dir_all(parent_dir).await?;
            }
        }
        let mut file = match args.overwrite {
            true => File::create(args.filename)?,
            false => File::options()
                .write(true)
                .truncate(true)
                .create_new(true)
                .open(args.filename)?,
        };
        while let Some(v) = resp.chunk().await? {
            file.write_all(&v)?;
        }
        file.sync_all()?;

        Ok(DownloadObjectResponseOld {
            headers: resp.headers().clone(),
            region: args.region.map_or(String::new(), String::from),
            bucket_name: args.bucket.to_string(),
            object_name: args.object.to_string(),
            version_id: args.version_id.as_ref().map(|v| v.to_string()),
        })
    }
    */

    /// Create a DownloadObject request builder.
    pub fn download_object(
        &self,
        bucket: &str,
        object: impl Into<ObjectToDownload>,
        filename: &str,
    ) -> DownloadObject {
        DownloadObject::new(bucket, object, filename).client(self)
    }
}
