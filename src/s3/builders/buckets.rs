use http::Method;

use crate::s3::{
    client::Client,
    error::Error,
    response::{GetBucketEncryptionResponse, GetBucketVersioningResponse, ListBucketsResponse},
    types::{S3Api, S3Request, ToS3Request},
    utils::{check_bucket_name, merge, Multimap},
};

/// Argument builder for
/// [list_buckets()](crate::s3::client::Client::list_buckets) API.
#[derive(Clone, Debug, Default)]
pub struct ListBuckets {
    client: Option<Client>,

    extra_headers: Option<Multimap>,
    extra_query_params: Option<Multimap>,
}

impl S3Api for ListBuckets {
    type S3Response = ListBucketsResponse;
}

impl ToS3Request for ListBuckets {
    fn to_s3request(&self) -> Result<S3Request, Error> {
        let mut headers = Multimap::new();
        if let Some(v) = &self.extra_headers {
            headers = v.clone();
        }
        let mut query_params = Multimap::new();
        if let Some(v) = &self.extra_query_params {
            query_params = v.clone();
        }

        let req = S3Request::new(
            self.client.as_ref().ok_or(Error::NoClientProvided)?,
            Method::GET,
        )
        .query_params(query_params)
        .headers(headers);
        Ok(req)
    }
}

impl ListBuckets {
    pub fn new() -> Self {
        Default::default()
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
}

#[derive(Clone, Debug, Default)]
pub struct BucketCommon1 {
    client: Option<Client>,
    extra_headers: Option<Multimap>,
    extra_query_params: Option<Multimap>,
    region: Option<String>,
    bucket: String,
}

impl BucketCommon1 {
    pub fn new(bucket: &str) -> Result<BucketCommon1, Error> {
        check_bucket_name(bucket, true)?;
        Ok(BucketCommon1 {
            bucket: bucket.to_owned(),
            ..Default::default()
        })
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

/// Argument builder for
/// [get_bucket_versioning()](crate::s3::client::Client::get_bucket_versioning)
/// API
pub type GetBucketVersioning = BucketCommon1;

impl S3Api for GetBucketVersioning {
    type S3Response = GetBucketVersioningResponse;
}

impl ToS3Request for GetBucketVersioning {
    fn to_s3request(&self) -> Result<S3Request, Error> {
        let mut headers = Multimap::new();
        if let Some(v) = &self.extra_headers {
            merge(&mut headers, v);
        }

        let mut query_params = Multimap::new();
        if let Some(v) = &self.extra_query_params {
            merge(&mut query_params, v);
        }
        query_params.insert(String::from("versioning"), String::new());

        let req = S3Request::new(
            self.client.as_ref().ok_or(Error::NoClientProvided)?,
            Method::GET,
        )
        .region(self.region.as_deref())
        .bucket(Some(&self.bucket))
        .query_params(query_params)
        .headers(headers);
        Ok(req)
    }
}

#[derive(Clone, Debug, Default)]
pub struct BucketCommon2 {
    client: Option<Client>,
    extra_headers: Option<Multimap>,
    extra_query_params: Option<Multimap>,
    region: Option<String>,
    bucket: String,
}

impl BucketCommon2 {
    pub fn new(bucket: &str) -> Result<BucketCommon2, Error> {
        check_bucket_name(bucket, true)?;
        Ok(BucketCommon2 {
            bucket: bucket.to_owned(),
            ..Default::default()
        })
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

/// Argument builder for
/// [get_bucket_encryption()](crate::s3::client::Client::get_bucket_encryption)
/// API
pub type GetBucketEncryption = BucketCommon2;

impl S3Api for GetBucketEncryption {
    type S3Response = GetBucketEncryptionResponse;
}

impl ToS3Request for GetBucketEncryption {
    fn to_s3request(&self) -> Result<S3Request, Error> {
        let mut headers = Multimap::new();
        if let Some(v) = &self.extra_headers {
            merge(&mut headers, v);
        }

        let mut query_params = Multimap::new();
        if let Some(v) = &self.extra_query_params {
            merge(&mut query_params, v);
        }
        query_params.insert(String::from("encryption"), String::new());

        let req = S3Request::new(
            self.client.as_ref().ok_or(Error::NoClientProvided)?,
            Method::GET,
        )
        .region(self.region.as_deref())
        .bucket(Some(&self.bucket))
        .query_params(query_params)
        .headers(headers);
        Ok(req)
    }
}
