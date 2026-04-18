use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};
use futures::future;
use minio::s3::builders::ObjectContent;
use minio::s3::client::MinioClient;
use minio::s3::creds::StaticProvider;
use minio::s3::http::BaseUrl;

const BUCKET_NAME: &str = "allan";
const ENDPOINT: &str = "http://localhost:9000";
const ACCESS_KEY: &str = "minioadmin";
const SECRET_KEY: &str = "minioadmin";

#[tokio::main]
async fn main() -> Result<()> {
    let base_url: BaseUrl = ENDPOINT.parse()?;
    let creds = StaticProvider::new(ACCESS_KEY, SECRET_KEY, None);
    let client = MinioClient::new(base_url, Some(creds), None, None)?;

    // Collect files to upload (example: all files in ./data directory)
    let upload_dir = Path::new("./data");
    if !upload_dir.exists() {
        return Err(anyhow!(
            "Upload directory '{}' does not exist. Create it and add files to upload.",
            upload_dir.display()
        ));
    }

    let files: Vec<PathBuf> = std::fs::read_dir(upload_dir)?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            if entry.file_type().ok()?.is_file() {
                Some(entry.path())
            } else {
                None
            }
        })
        .collect();

    println!("Found {} files to upload", files.len());

    let tasks: Vec<_> = files
        .into_iter()
        .map(|path| {
            let client = client.clone();
            let upload_dir = upload_dir.to_path_buf();
            tokio::spawn(async move {  // Fully concurrent - no semaphore limiting
                let rel = path
                    .strip_prefix(&upload_dir)
                    .unwrap()
                    .to_str()
                    .unwrap();
                let object_key = format!("raw/{rel}");
                let content = ObjectContent::from(path.as_path());
                let result = client
                    .put_object_content(BUCKET_NAME, &object_key, content)
                    .content_type(Some("application/octet-stream".to_string()))
                    .build()
                    .send()
                    .await;
                match result {
                    Ok(_) => (path, Ok(object_key)),
                    Err(e) => (path, Err(anyhow!("{e:#}"))),
                }
            })
        })
        .collect();

    let results = future::join_all(tasks).await;

    let mut ok_count = 0usize;
    let mut fail_count = 0usize;
    for result in results {
        match result {
            Ok((path, Ok(key))) => {
                println!("  OK  {key} <- {}", path.display());
                ok_count += 1;
            }
            Ok((path, Err(e))) => {
                eprintln!("  FAIL {e} {}", path.display());
                fail_count += 1;
            }
            Err(e) => {
                eprintln!("  FAIL (task panic): {e}");
                fail_count += 1;
            }
        }
    }

    println!("\nDone: {ok_count} succeeded, {fail_count} failed");
    Ok(())
}
