# MinIO AIStor Bug Report: Warehouse Registry File Locking Under Concurrent Load

## Summary

MinIO AIStor S3 Tables implementation returns HTTP 500 Internal Server Error when handling concurrent Iceberg REST API requests due to file locking conflicts on the warehouse registry file.

## Severity

**High** - Affects production workloads with concurrent table operations.

---

## Prerequisites

### Software Requirements

- MinIO AIStor server (Enterprise edition with S3 Tables support)
- Rust toolchain (for running the test tool)
- Git
- curl (for manual testing)

### MinIO Server Setup

1. **Clone and build MinIO AIStor** (if not already available):
   ```bash
   # MinIO AIStor source should be at C:\source\minio\eos
   # or your preferred location
   ```

2. **Create a fresh data directory**:
   ```bash
   # Windows
   rmdir /s /q C:\minio-test-data
   mkdir C:\minio-test-data

   # Linux/macOS
   rm -rf /tmp/minio-test-data
   mkdir -p /tmp/minio-test-data
   ```

3. **Start MinIO AIStor server**:
   ```bash
   # Windows (PowerShell)
   cd C:\source\minio\eos
   $env:MINIO_ROOT_USER="minioadmin"
   $env:MINIO_ROOT_PASSWORD="minioadmin"
   .\minio.exe server C:\minio-test-data --console-address ":9001"

   # Windows (Git Bash / MSYS2)
   cd /c/source/minio/eos
   MINIO_ROOT_USER=minioadmin MINIO_ROOT_PASSWORD=minioadmin ./minio.exe server /c/minio-test-data --console-address ":9001"

   # Linux/macOS
   cd /path/to/minio
   MINIO_ROOT_USER=minioadmin MINIO_ROOT_PASSWORD=minioadmin ./minio server /tmp/minio-test-data --console-address ":9001"
   ```

4. **Verify server is running**:
   ```bash
   curl -s http://localhost:9000/minio/health/live
   # Should return nothing (HTTP 200)
   ```

---

## Reproduction Method 1: Using the Iceberg Spec Test Tool (Recommended)

This method provides consistent, automated reproduction.

### Step 1: Clone the MinIO Rust SDK

```bash
git clone https://github.com/minio/minio-rs.git
cd minio-rs
git checkout feature/s3tables
```

### Step 2: Build the test tool

```bash
cargo build --example iceberg_spec_test
```

### Step 3: Run with concurrency=5 (triggers the bug)

```bash
cargo run --example iceberg_spec_test -- --concurrency 5
```

**Expected output showing failures:**
```
============================================================
Iceberg REST API Spec Compliance Test Results
============================================================
Date: 2026-01-14 21:17:47 UTC
Total tests: 92
Passed: 41 (44.6%)
Failed: 9
Skipped: 42

Failures (first 10):
  - namespace_exists 1/1 none: SPEC_VIOLATION    <-- THIS IS THE BUG
  - rename_table 1/1 none: UNEXPECTED_ERROR      <-- THIS IS THE BUG
  ...
```

### Step 4: Run with concurrency=1 (bug does not occur)

```bash
cargo run --example iceberg_spec_test -- --concurrency 1
```

**Expected output showing success:**
```
============================================================
Iceberg REST API Spec Compliance Test Results
============================================================
Date: 2026-01-14 21:23:30 UTC
Total tests: 92
Passed: 43 (46.7%)
Failed: 7
Skipped: 42
```

### Step 5: Compare results

| Test | Concurrency=5 | Concurrency=1 |
|------|---------------|---------------|
| `namespace_exists` | **FAIL (500)** | PASS (204) |
| `rename_table` | **FAIL (setup error)** | PASS |

---

## Reproduction Method 2: Manual curl Commands

This method reproduces the bug using only curl commands.

### Step 1: Create a test warehouse

```bash
curl -X POST "http://localhost:9000/_iceberg/v1/warehouses" \
  --aws-sigv4 "aws:amz:us-east-1:s3" \
  -u "minioadmin:minioadmin" \
  -H "Content-Type: application/json" \
  -d '{"name":"bug-test-warehouse"}'
```

**Expected response (HTTP 200):**
```json
{"name":"bug-test-warehouse","status":"active",...}
```

### Step 2: Create a test namespace

```bash
curl -X POST "http://localhost:9000/_iceberg/v1/warehouses/bug-test-warehouse/namespaces" \
  --aws-sigv4 "aws:amz:us-east-1:s3" \
  -u "minioadmin:minioadmin" \
  -H "Content-Type: application/json" \
  -d '{"namespace":["bug-test-namespace"]}'
```

**Expected response (HTTP 200):**
```json
{"namespace":["bug-test-namespace"],"properties":{}}
```

### Step 3: Verify single request works

```bash
curl -X HEAD "http://localhost:9000/_iceberg/v1/warehouses/bug-test-warehouse/namespaces/bug-test-namespace" \
  --aws-sigv4 "aws:amz:us-east-1:s3" \
  -u "minioadmin:minioadmin" \
  -w "HTTP Status: %{http_code}\n" \
  -o /dev/null
```

**Expected output:**
```
HTTP Status: 204
```

### Step 4: Run concurrent requests (triggers the bug)

**Linux/macOS/Git Bash:**
```bash
#!/bin/bash
# Save as: reproduce-bug.sh

echo "Running 20 concurrent HEAD requests..."
echo "Watch for HTTP 500 responses (should be 204)"
echo ""

for i in {1..20}; do
  (
    result=$(curl -s -X HEAD \
      "http://localhost:9000/_iceberg/v1/warehouses/bug-test-warehouse/namespaces/bug-test-namespace" \
      --aws-sigv4 "aws:amz:us-east-1:s3" \
      -u "minioadmin:minioadmin" \
      -w "%{http_code}" \
      -o /dev/null)
    echo "Request $i: HTTP $result"
  ) &
done

wait
echo ""
echo "Done. Any HTTP 500 responses indicate the bug."
```

Run it:
```bash
chmod +x reproduce-bug.sh
./reproduce-bug.sh
```

**Expected buggy output (some requests fail):**
```
Running 20 concurrent HEAD requests...
Watch for HTTP 500 responses (should be 204)

Request 1: HTTP 204
Request 2: HTTP 204
Request 3: HTTP 500    <-- BUG
Request 4: HTTP 204
Request 5: HTTP 500    <-- BUG
Request 6: HTTP 204
...
```

**PowerShell version:**
```powershell
# Save as: reproduce-bug.ps1

Write-Host "Running 20 concurrent HEAD requests..."
Write-Host "Watch for HTTP 500 responses (should be 204)"
Write-Host ""

$jobs = 1..20 | ForEach-Object {
    $i = $_
    Start-Job -ScriptBlock {
        param($num)
        $result = curl -s -X HEAD `
            "http://localhost:9000/_iceberg/v1/warehouses/bug-test-warehouse/namespaces/bug-test-namespace" `
            --aws-sigv4 "aws:amz:us-east-1:s3" `
            -u "minioadmin:minioadmin" `
            -w "%{http_code}" `
            -o NUL
        "Request $num : HTTP $result"
    } -ArgumentList $i
}

$jobs | Wait-Job | Receive-Job
$jobs | Remove-Job

Write-Host ""
Write-Host "Done. Any HTTP 500 responses indicate the bug."
```

Run it:
```powershell
.\reproduce-bug.ps1
```

### Step 5: Check MinIO server logs

While the concurrent requests are running, the MinIO server console will show errors like:

```
API: NamespaceExists(bucket=bucket, object=bug-test-warehouse)
Time: 21:17:46 UTC 01/14/2026
DeploymentID: 7b186dfd-e7ed-4df6-b9df-09f0fbdb8c9a
RequestID: 188AB597B3212A98
RemoteHost: [::1]
Error: failed to read warehouse registry: The process cannot access the file because it is being used by another process. (*fmt.wrapError)
       s3tables:namespace="bug-test-namespace"
       6: C:\source\minio\eos\cmd\logging.go:168:cmd.internalLogIf()
       5: C:\source\minio\eos\cmd\api-errors.go:2951:cmd.toAPIError()
       4: C:\source\minio\eos\cmd\tables-api-errors.go:412:cmd.toTablesAPIError()
       3: C:\source\minio\eos\cmd\tables-api-handlers.go:464:cmd.tablesAPIHandlers.NamespaceExists()
```

---

## Reproduction Method 3: Minimal Rust Program

Create a file `reproduce_bug.rs`:

```rust
//! Minimal reproduction of MinIO warehouse registry file locking bug
//!
//! Usage:
//!   cargo run --example reproduce_bug

use std::time::Instant;
use tokio::task::JoinSet;

const ENDPOINT: &str = "http://localhost:9000";
const ACCESS_KEY: &str = "minioadmin";
const SECRET_KEY: &str = "minioadmin";
const WAREHOUSE: &str = "locking-bug-test";
const NAMESPACE: &str = "test-ns";
const CONCURRENT_REQUESTS: usize = 20;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("MinIO Warehouse Registry File Locking Bug Reproduction");
    println!("=======================================================\n");

    // Create HTTP client
    let client = reqwest::Client::new();

    // Step 1: Create warehouse
    println!("Step 1: Creating warehouse '{}'...", WAREHOUSE);
    let resp = client
        .post(format!("{}/_iceberg/v1/warehouses", ENDPOINT))
        .basic_auth(ACCESS_KEY, Some(SECRET_KEY))
        .header("Content-Type", "application/json")
        .body(format!(r#"{{"name":"{}"}}"#, WAREHOUSE))
        .send()
        .await?;
    println!("  Response: {} {}\n", resp.status().as_u16(),
             if resp.status().is_success() { "OK" } else { "FAILED" });

    // Step 2: Create namespace
    println!("Step 2: Creating namespace '{}'...", NAMESPACE);
    let resp = client
        .post(format!("{}/_iceberg/v1/warehouses/{}/namespaces", ENDPOINT, WAREHOUSE))
        .basic_auth(ACCESS_KEY, Some(SECRET_KEY))
        .header("Content-Type", "application/json")
        .body(format!(r#"{{"namespace":["{}"]}}"#, NAMESPACE))
        .send()
        .await?;
    println!("  Response: {} {}\n", resp.status().as_u16(),
             if resp.status().is_success() { "OK" } else { "FAILED" });

    // Step 3: Single request (should work)
    println!("Step 3: Single HEAD request (should return 204)...");
    let resp = client
        .head(format!("{}/_iceberg/v1/warehouses/{}/namespaces/{}",
                      ENDPOINT, WAREHOUSE, NAMESPACE))
        .basic_auth(ACCESS_KEY, Some(SECRET_KEY))
        .send()
        .await?;
    println!("  Response: {}\n", resp.status().as_u16());

    // Step 4: Concurrent requests (triggers bug)
    println!("Step 4: {} concurrent HEAD requests (triggers bug)...", CONCURRENT_REQUESTS);
    println!("  Expected: All should return 204");
    println!("  Actual: Some will return 500 due to file locking\n");

    let start = Instant::now();
    let mut tasks = JoinSet::new();

    for i in 0..CONCURRENT_REQUESTS {
        let client = client.clone();
        let url = format!("{}/_iceberg/v1/warehouses/{}/namespaces/{}",
                         ENDPOINT, WAREHOUSE, NAMESPACE);
        tasks.spawn(async move {
            let resp = client
                .head(&url)
                .basic_auth(ACCESS_KEY, Some(SECRET_KEY))
                .send()
                .await;
            (i, resp.map(|r| r.status().as_u16()).unwrap_or(0))
        });
    }

    let mut results: Vec<(usize, u16)> = Vec::new();
    while let Some(result) = tasks.join_next().await {
        if let Ok((i, status)) = result {
            results.push((i, status));
        }
    }

    results.sort_by_key(|r| r.0);

    let mut success = 0;
    let mut failures = 0;

    for (i, status) in &results {
        let marker = if *status == 204 {
            success += 1;
            "OK"
        } else {
            failures += 1;
            "BUG!"
        };
        println!("  Request {:2}: HTTP {} {}", i, status, marker);
    }

    let duration = start.elapsed();

    println!("\n=======================================================");
    println!("Results:");
    println!("  Total requests:  {}", CONCURRENT_REQUESTS);
    println!("  Successful (204): {}", success);
    println!("  Failed (500):     {} {}", failures,
             if failures > 0 { "<-- BUG REPRODUCED" } else { "" });
    println!("  Duration:         {:?}", duration);
    println!("=======================================================");

    if failures > 0 {
        println!("\nBUG CONFIRMED: {} requests failed due to file locking.", failures);
        println!("Check MinIO server logs for 'failed to read warehouse registry' errors.");
    } else {
        println!("\nBug not reproduced in this run. Try increasing CONCURRENT_REQUESTS.");
    }

    Ok(())
}
```

Add to `Cargo.toml`:
```toml
[[example]]
name = "reproduce_bug"
path = "examples/reproduce_bug.rs"

[dev-dependencies]
reqwest = { version = "0.11", features = ["json"] }
tokio = { version = "1", features = ["full"] }
```

Run:
```bash
cargo run --example reproduce_bug
```

**Expected output:**
```
MinIO Warehouse Registry File Locking Bug Reproduction
=======================================================

Step 1: Creating warehouse 'locking-bug-test'...
  Response: 200 OK

Step 2: Creating namespace 'test-ns'...
  Response: 200 OK

Step 3: Single HEAD request (should return 204)...
  Response: 204

Step 4: 20 concurrent HEAD requests (triggers bug)...
  Expected: All should return 204
  Actual: Some will return 500 due to file locking

  Request  0: HTTP 204 OK
  Request  1: HTTP 204 OK
  Request  2: HTTP 500 BUG!
  Request  3: HTTP 204 OK
  Request  4: HTTP 500 BUG!
  ...

=======================================================
Results:
  Total requests:  20
  Successful (204): 16
  Failed (500):     4 <-- BUG REPRODUCED
  Duration:         45.2ms
=======================================================

BUG CONFIRMED: 4 requests failed due to file locking.
Check MinIO server logs for 'failed to read warehouse registry' errors.
```

---

## Expected vs Actual Behavior

### Expected Behavior

All concurrent HEAD requests to `/_iceberg/v1/warehouses/{warehouse}/namespaces/{namespace}` should return:
- **HTTP 204 No Content** - if the namespace exists
- **HTTP 404 Not Found** - if the namespace does not exist

### Actual Behavior

Under concurrent load, some requests return:
- **HTTP 500 Internal Server Error** - with empty body

The MinIO server logs show:
```
Error: failed to read warehouse registry: The process cannot access the file because it is being used by another process.
```

---

## Root Cause Analysis

Based on the server logs and stack traces:

1. **Location**: `cmd/tables-api-handlers.go:464` in `NamespaceExists()` handler
2. **Issue**: The warehouse registry file is opened with exclusive locking
3. **Effect**: Concurrent read requests cannot acquire the lock simultaneously
4. **Result**: Lock acquisition failure is converted to HTTP 500 error

### Relevant MinIO Source Files

```
cmd/tables-api-handlers.go:464   - NamespaceExists handler
cmd/tables-api-handlers.go:560   - CreateTable handler
cmd/tables-api-handlers.go:956   - ListTables handler
cmd/tables-api-errors.go:412     - Error conversion to API response
```

---

## Workaround

Serialize all S3 Tables API requests (not practical for production):

```bash
# Use --concurrency 1 with the test tool
cargo run --example iceberg_spec_test -- --concurrency 1
```

---

## Suggested Fixes

1. **Use shared read locks**: Allow multiple concurrent readers on the warehouse registry file.

2. **Implement retry with backoff**: Retry lock acquisition with exponential backoff before failing.

3. **In-memory caching**: Cache registry contents in memory to reduce file I/O contention.

4. **Lock-free reads**: Use copy-on-write or MVCC pattern for registry access.

---

## Environment Details

| Component | Version/Details |
|-----------|-----------------|
| MinIO | AIStor Enterprise (January 2026 build) |
| OS | Windows 11 / Windows Server |
| Storage | Local NTFS filesystem |
| API | Iceberg REST Catalog v1 |
| Test Tool | minio-rs iceberg_spec_test |

---

## References

- Iceberg REST Catalog API Spec: https://github.com/apache/iceberg/blob/main/open-api/rest-catalog-open-api.yaml
- MinIO S3 Tables Documentation: https://docs.min.io/enterprise/aistor-object-store/
