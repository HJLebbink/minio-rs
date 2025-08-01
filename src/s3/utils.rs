// MinIO Rust Library for Amazon S3 Compatible Cloud Storage
// Copyright 2022 MinIO, Inc.
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

//! Various utility and helper functions

use crate::s3::error::Error;
use crate::s3::multimap::Multimap;
use crate::s3::segmented_bytes::SegmentedBytes;
use base64::engine::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use byteorder::{BigEndian, ReadBytesExt};
use chrono::{DateTime, Datelike, NaiveDateTime, ParseError, Utc};
use crc::{CRC_32_ISO_HDLC, Crc};
use hex::ToHex;
use lazy_static::lazy_static;
use md5::compute as md5compute;
use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, percent_decode_str, utf8_percent_encode};
use regex::Regex;
#[cfg(feature = "ring")]
use ring::digest::{Context, SHA256};
#[cfg(not(feature = "ring"))]
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;
pub use urlencoding::decode as urldecode;
pub use urlencoding::encode as urlencode;
use xmltree::Element;

/// Date and time with UTC timezone
pub type UtcTime = DateTime<Utc>;

/// Encodes data using base64 algorithm
pub fn b64encode(input: impl AsRef<[u8]>) -> String {
    BASE64.encode(input)
}

/// Computes CRC32 of given data.
pub fn crc32(data: &[u8]) -> u32 {
    //TODO creating a new Crc object is expensive, we should cache it
    Crc::<u32>::new(&CRC_32_ISO_HDLC).checksum(data)
}

/// Converts data array into 32 bit unsigned int
pub fn uint32(mut data: &[u8]) -> Result<u32, std::io::Error> {
    data.read_u32::<BigEndian>()
}

/// sha256 hash of empty data
pub const EMPTY_SHA256: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

/// Gets hex encoded SHA256 hash of given data
pub fn sha256_hash(data: &[u8]) -> String {
    #[cfg(feature = "ring")]
    {
        ring::digest::digest(&SHA256, data).encode_hex()
    }
    #[cfg(not(feature = "ring"))]
    {
        Sha256::new_with_prefix(data).finalize().encode_hex()
    }
}

pub fn sha256_hash_sb(sb: Arc<SegmentedBytes>) -> String {
    #[cfg(feature = "ring")]
    {
        let mut context = Context::new(&SHA256);
        for data in sb.iter() {
            // Note: &SegmentedBytes.iter yields clones of Bytes, but those clones are cheap
            context.update(data.as_ref());
        }
        context.finish().encode_hex()
    }
    #[cfg(not(feature = "ring"))]
    {
        let mut hasher = Sha256::new();
        for data in sb.iter() {
            // Note: &SegmentedBytes.iter yields clones of Bytes, but those clones are cheap
            hasher.update(data);
        }
        hasher.finalize().encode_hex()
    }
}

#[cfg(test)]
mod tests {
    use crate::s3::utils::SegmentedBytes;
    use crate::s3::utils::sha256_hash_sb;
    use std::sync::Arc;

    #[test]
    fn test_empty_sha256_segmented_bytes() {
        assert_eq!(
            super::EMPTY_SHA256,
            sha256_hash_sb(Arc::new(SegmentedBytes::new()))
        );
    }
}

/// Gets bas64 encoded MD5 hash of given data
pub fn md5sum_hash(data: &[u8]) -> String {
    b64encode(md5compute(data).as_slice())
}

/// Gets current UTC time
pub fn utc_now() -> UtcTime {
    chrono::offset::Utc::now()
}

/// Gets signer date value of given time
pub fn to_signer_date(time: UtcTime) -> String {
    time.format("%Y%m%d").to_string()
}

/// Gets AMZ date value of given time
pub fn to_amz_date(time: UtcTime) -> String {
    time.format("%Y%m%dT%H%M%SZ").to_string()
}

/// Gets HTTP header value of given time
pub fn to_http_header_value(time: UtcTime) -> String {
    format!(
        "{}, {} {} {} GMT",
        time.weekday(),
        time.day(),
        match time.month() {
            1 => "Jan",
            2 => "Feb",
            3 => "Mar",
            4 => "Apr",
            5 => "May",
            6 => "Jun",
            7 => "Jul",
            8 => "Aug",
            9 => "Sep",
            10 => "Oct",
            11 => "Nov",
            12 => "Dec",
            _ => "",
        },
        time.format("%Y %H:%M:%S")
    )
}

/// Gets ISO8601 UTC formatted value of given time
pub fn to_iso8601utc(time: UtcTime) -> String {
    time.format("%Y-%m-%dT%H:%M:%S.%3fZ").to_string()
}

/// Parses ISO8601 UTC formatted value to time
pub fn from_iso8601utc(s: &str) -> Result<UtcTime, ParseError> {
    Ok(DateTime::<Utc>::from_naive_utc_and_offset(
        match NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S.%3fZ") {
            Ok(d) => d,
            _ => NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%SZ")?,
        },
        Utc,
    ))
}

const OBJECT_KEY_ENCODE_SET: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'_')
    .remove(b'.')
    .remove(b'~')
    .remove(b'/');

pub fn urlencode_object_key(key: &str) -> String {
    utf8_percent_encode(key, OBJECT_KEY_ENCODE_SET).collect()
}

pub mod aws_date_format {
    use super::{UtcTime, from_iso8601utc, to_iso8601utc};
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(date: &UtcTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&to_iso8601utc(*date))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<UtcTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        from_iso8601utc(&s).map_err(serde::de::Error::custom)
    }
}

/// Parses HTTP header value to time
pub fn from_http_header_value(s: &str) -> Result<UtcTime, ParseError> {
    Ok(DateTime::<Utc>::from_naive_utc_and_offset(
        NaiveDateTime::parse_from_str(s, "%a, %d %b %Y %H:%M:%S GMT")?,
        Utc,
    ))
}

/// Checks if given hostname is valid or not
pub fn match_hostname(value: &str) -> bool {
    lazy_static! {
        static ref HOSTNAME_REGEX: Regex =
            Regex::new(r"^([a-z_\d-]{1,63}\.)*([a-z_\d-]{1,63})$").unwrap();
    }

    if !HOSTNAME_REGEX.is_match(value.to_lowercase().as_str()) {
        return false;
    }

    for token in value.split('.') {
        if token.starts_with('-')
            || token.starts_with('_')
            || token.ends_with('-')
            || token.ends_with('_')
        {
            return false;
        }
    }

    true
}

/// Checks if given region is valid or not
pub fn match_region(value: &str) -> bool {
    lazy_static! {
        static ref REGION_REGEX: Regex = Regex::new(r"^([a-z_\d-]{1,63})$").unwrap();
    }

    !REGION_REGEX.is_match(value.to_lowercase().as_str())
        || value.starts_with('-')
        || value.starts_with('_')
        || value.ends_with('-')
        || value.ends_with('_')
}

/// Validates given bucket name
pub fn check_bucket_name(bucket_name: impl AsRef<str>, strict: bool) -> Result<(), Error> {
    let bucket_name: &str = bucket_name.as_ref().trim();
    let bucket_name_len = bucket_name.len();
    if bucket_name_len == 0 {
        return Err(Error::InvalidBucketName(
            "bucket name cannot be empty".into(),
        ));
    }
    if bucket_name_len < 3 {
        return Err(Error::InvalidBucketName(format!(
            "bucket name ('{bucket_name}') cannot be less than 3 characters"
        )));
    }
    if bucket_name_len > 63 {
        return Err(Error::InvalidBucketName(format!(
            "Bucket name ('{bucket_name}') cannot be greater than 63 characters"
        )));
    }

    lazy_static! {
    static ref IPV4_REGEX: Regex = Regex::new(r"^((25[0-5]|2[0-4][0-9]|1[0-9][0-9]|[1-9][0-9]|[0-9])\.){3}(25[0-5]|2[0-4][0-9]|1[0-9][0-9]|[1-9][0-9]|[0-9])$").unwrap();
        static ref VALID_BUCKET_NAME_REGEX: Regex =
            Regex::new("^[A-Za-z0-9][A-Za-z0-9\\.\\-_:]{1,61}[A-Za-z0-9]$").unwrap();
        static ref VALID_BUCKET_NAME_STRICT_REGEX: Regex =
            Regex::new("^[a-z0-9][a-z0-9\\.\\-]{1,61}[a-z0-9]$").unwrap();
    }

    if IPV4_REGEX.is_match(bucket_name) {
        return Err(Error::InvalidBucketName(format!(
            "bucket name ('{bucket_name}') cannot be an IP address"
        )));
    }

    if bucket_name.contains("..") || bucket_name.contains(".-") || bucket_name.contains("-.") {
        return Err(Error::InvalidBucketName(format!(
            "bucket name ('{bucket_name}') contains invalid successive characters '..', '.-' or '-.'",
        )));
    }

    if strict {
        if !VALID_BUCKET_NAME_STRICT_REGEX.is_match(bucket_name) {
            return Err(Error::InvalidBucketName(format!(
                "bucket name ('{bucket_name}') does not follow S3 standards strictly, according to {}",
                *VALID_BUCKET_NAME_STRICT_REGEX
            )));
        }
    } else if !VALID_BUCKET_NAME_REGEX.is_match(bucket_name) {
        return Err(Error::InvalidBucketName(format!(
            "bucket name ('{bucket_name}') does not follow S3 standards, according to {}",
            *VALID_BUCKET_NAME_REGEX
        )));
    }

    Ok(())
}

pub fn check_object_name(object_name: impl AsRef<str>) -> Result<(), Error> {
    let object_name: &str = object_name.as_ref();
    let object_name_n_bytes = object_name.len();
    if object_name_n_bytes == 0 {
        return Err(Error::InvalidObjectName(
            "object name cannot be empty".into(),
        ));
    }
    if object_name_n_bytes > 1024 {
        return Err(Error::InvalidObjectName(format!(
            "Object name ('{object_name}') cannot be greater than 1024 bytes"
        )));
    }

    Ok(())
}

/// Gets text value of given XML element for given tag.
pub fn get_text(element: &Element, tag: &str) -> Result<String, Error> {
    Ok(element
        .get_child(tag)
        .ok_or(Error::XmlError(format!("<{tag}> tag not found")))?
        .get_text()
        .ok_or(Error::XmlError(format!("text of <{tag}> tag not found")))?
        .to_string())
}

/// Gets optional text value of given XML element for given tag.
pub fn get_option_text(element: &Element, tag: &str) -> Option<String> {
    if let Some(v) = element.get_child(tag) {
        return Some(v.get_text().unwrap_or_default().to_string());
    }

    None
}

/// Trim leading and trailing quotes from a string. It consumes the
pub fn trim_quotes(mut s: String) -> String {
    if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
        s.drain(0..1); // remove the leading quote
        s.pop(); // remove the trailing quote
    }
    s
}

/// Gets default text value of given XML element for given tag.
pub fn get_default_text(element: &Element, tag: &str) -> String {
    element.get_child(tag).map_or(String::new(), |v| {
        v.get_text().unwrap_or_default().to_string()
    })
}

/// Copies source byte slice into destination byte slice
pub fn copy_slice(dst: &mut [u8], src: &[u8]) -> usize {
    let mut c = 0;
    for (d, s) in dst.iter_mut().zip(src.iter()) {
        *d = *s;
        c += 1;
    }
    c
}

// Characters to escape in query strings. Based on RFC 3986 and the golang
// net/url implementation used in the MinIO server.
//
// https://tools.ietf.org/html/rfc3986
//
// 1. All non-ascii characters are escaped always.
// 2. All reserved characters are escaped.
// 3. Any other characters are not escaped.
//
// Unreserved characters in addition to alphanumeric characters are: '-', '_',
// '.', '~' (§2.3 Unreserved characters (mark))
//
// Reserved characters for query strings: '$', '&', '+', ',', '/', ':', ';',
// '=', '?', '@' (§3.4)
//
// NON_ALPHANUMERIC already escapes everything non-alphanumeric (it includes all
// the reserved characters). So we only remove the unreserved characters from
// this set.
const QUERY_ESCAPE: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'_')
    .remove(b'.')
    .remove(b'~');

fn unescape(s: &str) -> Result<String, Error> {
    percent_decode_str(s)
        .decode_utf8()
        .map_err(|e| Error::TagDecodingError(s.to_string(), e.to_string()))
        .map(|s| s.to_string())
}

fn escape(s: &str) -> String {
    utf8_percent_encode(s, QUERY_ESCAPE).collect()
}

// TODO: use this while adding API to set tags.
//
// Handles escaping same as MinIO server - needed for ensuring compatibility.
pub fn encode_tags(h: &HashMap<String, String>) -> String {
    let mut tags = Vec::new();
    for (k, v) in h {
        tags.push(format!("{}={}", escape(k), escape(v)));
    }
    tags.join("&")
}

pub fn parse_tags(s: &str) -> Result<HashMap<String, String>, Error> {
    let mut tags = HashMap::new();
    for tag in s.split('&') {
        let mut kv = tag.split('=');
        let k = match kv.next() {
            Some(v) => unescape(v)?,
            None => {
                return Err(Error::TagDecodingError(
                    s.into(),
                    "tag key was empty".into(),
                ));
            }
        };
        let v = match kv.next() {
            Some(v) => unescape(v)?,
            None => "".to_owned(),
        };
        if kv.next().is_some() {
            return Err(Error::TagDecodingError(
                s.into(),
                "tag had too many values for a key".into(),
            ));
        }
        tags.insert(k, v);
    }
    Ok(tags)
}

#[must_use]
/// Returns the consumed data and inserts a key into it with an empty value.
pub fn insert(data: Option<Multimap>, key: impl Into<String>) -> Multimap {
    let mut result: Multimap = data.unwrap_or_default();
    result.insert(key.into(), String::new());
    result
}

pub mod xml {
    use std::collections::HashMap;

    use crate::s3::error::Error;

    #[derive(Debug, Clone)]
    struct XmlElementIndex {
        children: HashMap<String, Vec<usize>>,
    }

    impl XmlElementIndex {
        fn get_first(&self, tag: &str) -> Option<usize> {
            let tag: String = tag.to_string();
            let is = self.children.get(&tag)?;
            is.first().copied()
        }

        fn get(&self, tag: &str) -> Option<&Vec<usize>> {
            let tag: String = tag.to_string();
            self.children.get(&tag)
        }
    }

    impl From<&xmltree::Element> for XmlElementIndex {
        fn from(value: &xmltree::Element) -> Self {
            let mut children = HashMap::new();
            for (i, e) in value
                .children
                .iter()
                .enumerate()
                .filter_map(|(i, v)| v.as_element().map(|e| (i, e)))
            {
                children
                    .entry(e.name.clone())
                    .or_insert_with(Vec::new)
                    .push(i);
            }
            Self { children }
        }
    }

    #[derive(Debug, Clone)]
    pub struct Element<'a> {
        inner: &'a xmltree::Element,
        child_element_index: XmlElementIndex,
    }

    impl<'a> From<&'a xmltree::Element> for Element<'a> {
        fn from(value: &'a xmltree::Element) -> Self {
            let element_index = XmlElementIndex::from(value);
            Self {
                inner: value,
                child_element_index: element_index,
            }
        }
    }

    impl Element<'_> {
        pub fn name(&self) -> &str {
            &self.inner.name
        }

        pub fn get_child_text(&self, tag: &str) -> Option<String> {
            let index = self.child_element_index.get_first(tag)?;
            self.inner.children[index]
                .as_element()?
                .get_text()
                .map(|v| v.to_string())
        }

        pub fn get_child_text_or_error(&self, tag: &str) -> Result<String, Error> {
            let i = self
                .child_element_index
                .get_first(tag)
                .ok_or(Error::XmlError(format!("<{tag}> tag not found")))?;
            self.inner.children[i]
                .as_element()
                .unwrap()
                .get_text()
                .map(|x| x.to_string())
                .ok_or(Error::XmlError(format!("text of <{tag}> tag not found")))
        }

        // Returns all children with given tag along with their index.
        pub fn get_matching_children(&self, tag: &str) -> Vec<(usize, Element)> {
            self.child_element_index
                .get(tag)
                .unwrap_or(&vec![])
                .iter()
                .map(|i| (*i, self.inner.children[*i].as_element().unwrap().into()))
                .collect()
        }

        pub fn get_child(&self, tag: &str) -> Option<Element> {
            let index = self.child_element_index.get_first(tag)?;
            Some(self.inner.children[index].as_element()?.into())
        }

        pub fn get_xmltree_children(&self) -> Vec<&xmltree::Element> {
            self.inner
                .children
                .iter()
                .filter_map(|v| v.as_element())
                .collect()
        }
    }

    // Helper type that implements merge sort in the iterator.
    pub struct MergeXmlElements<'a> {
        v1: &'a Vec<(usize, Element<'a>)>,
        v2: &'a Vec<(usize, Element<'a>)>,
        i1: usize,
        i2: usize,
    }

    impl<'a> MergeXmlElements<'a> {
        pub fn new(v1: &'a Vec<(usize, Element<'a>)>, v2: &'a Vec<(usize, Element<'a>)>) -> Self {
            Self {
                v1,
                v2,
                i1: 0,
                i2: 0,
            }
        }
    }

    impl<'a> Iterator for MergeXmlElements<'a> {
        type Item = &'a Element<'a>;

        fn next(&mut self) -> Option<Self::Item> {
            let c1 = self.v1.get(self.i1);
            let c2 = self.v2.get(self.i2);
            match (c1, c2) {
                (Some(val1), Some(val2)) => {
                    if val1.0 < val2.0 {
                        self.i1 += 1;
                        Some(&val1.1)
                    } else {
                        self.i2 += 1;
                        Some(&val2.1)
                    }
                }
                (Some(val1), None) => {
                    self.i1 += 1;
                    Some(&val1.1)
                }
                (None, Some(val2)) => {
                    self.i2 += 1;
                    Some(&val2.1)
                }
                (None, None) => None,
            }
        }
    }
}
