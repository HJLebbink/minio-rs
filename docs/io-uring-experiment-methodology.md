# io_uring Experiment Methodology

## Purpose

This document provides a step-by-step guide for running experiments to measure the actual performance impact of io_uring on MinIO `append_object` operations.

## Prerequisites

### System Requirements

1. **Operating System**
   - Linux with kernel 5.1+ (for io_uring support)
   - Windows 10+ (for IOCP support via compio)
   - Other Unix systems (will fall back to polling)

2. **Hardware**
   - Storage: Test on your target storage type (HDD/SSD/NVMe)
   - Network: Access to MinIO server (local or remote)
   - Memory: Sufficient for test file sizes

3. **Software**
   - Rust toolchain (cargo, rustc)
   - MinIO server running and accessible
   - Environment variables configured (see below)

### Environment Setup

Set up environment variables for MinIO access:

```bash
# Linux/macOS
export MINIO_ENDPOINT="http://localhost:9000"
export MINIO_ROOT_USER="minioadmin"
export MINIO_ROOT_PASSWORD="minioadmin"
export MINIO_REGION="us-east-1"

# Windows PowerShell
$env:MINIO_ENDPOINT="http://localhost:9000"
$env:MINIO_ROOT_USER="minioadmin"
$env:MINIO_ROOT_PASSWORD="minioadmin"
$env:MINIO_REGION="us-east-1"
```

### Code Preparation

**CRITICAL**: Before running experiments, uncomment the upload code in `examples/file_upload_io_uring.rs`:

1. **Uncomment TestContext initialization** (lines 174-175):
```rust
let ctx = TestContext::new_from_env();
let (bucket_name, _cleanup) = ctx.create_bucket_helper().await;
```

2. **Uncomment the upload call** (line 210):
```rust
dummy_upload(&buf[..n], off, &ctx, &bucket_name, &object_name).await;
```

3. **Update function signature** (line 181) to pass TestContext:
```rust
async fn run(args: Args) {
    let ctx = TestContext::new_from_env();
    let (bucket_name, _cleanup) = ctx.create_bucket_helper().await;
    create_file(&args).await;
    let object_name = rand_object_name();
    // ... rest of function
}
```

4. **Pass context to dummy_upload** (around line 210):
```rust
dummy_upload(buf[..n].to_vec(), off, &ctx, &bucket_name, &object_name).await;
```

## Experiment Design

### Variables to Test

#### Independent Variables (What You Control)

1. **Backend type**: tokio vs uring
2. **File size**: 10 MiB, 100 MiB, 1 GiB, 10 GiB
3. **Chunk size**: 4 MiB, 8 MiB, 16 MiB, 32 MiB, 64 MiB
4. **Concurrency** (uring only): 16, 32, 64, 128
5. **Network condition**: local vs remote MinIO

#### Dependent Variables (What You Measure)

1. **Total time**: End-to-end operation time
2. **Read time**: Time spent reading from disk
3. **Upload time**: Time spent uploading to MinIO
4. **CPU usage**: System CPU utilization during operation
5. **Memory usage**: Peak memory consumption
6. **Throughput**: MB/s achieved

### Control Variables (Keep Constant)

- Same MinIO server for all tests
- Same file content (or same random seed)
- Same system load conditions
- Same network conditions per test group
- Run tests multiple times and average results

## Test Scenarios

### Scenario 1: Baseline Small File

**Purpose**: Establish baseline with minimal file I/O impact

**Configuration**:
```bash
--file-size-mib 10
--chunk-mib 8
```

**Expected Outcome**: Network latency dominates, minimal difference between backends

### Scenario 2: Medium File Local Network

**Purpose**: Test with balanced file I/O and network

**Configuration**:
```bash
--file-size-mib 100
--chunk-mib 8
```

**Expected Outcome**: May show some improvement if storage is fast

### Scenario 3: Large File Local Network

**Purpose**: Test where file I/O becomes significant

**Configuration**:
```bash
--file-size-mib 1024
--chunk-mib 8
```

**Expected Outcome**: io_uring benefits should be most visible here

### Scenario 4: Very Large File

**Purpose**: Test at scale

**Configuration**:
```bash
--file-size-mib 10240
--chunk-mib 16
```

**Expected Outcome**: Sustained performance over time

### Scenario 5: Varying Chunk Sizes

**Purpose**: Find optimal chunk size for each backend

**Test matrix**:
- File size: 1024 MiB (constant)
- Chunk sizes: 4, 8, 16, 32, 64 MiB
- Both backends

**Expected Outcome**: May find different optimal chunk sizes for each backend

### Scenario 6: Varying Concurrency (uring only)

**Purpose**: Find optimal read concurrency for io_uring

**Test matrix**:
- File size: 1024 MiB (constant)
- Chunk size: 8 MiB (constant)
- Concurrency: 8, 16, 32, 64, 128, 256

**Expected Outcome**: Identify diminishing returns point

## Running Experiments

### Step 1: Build the Example

```bash
cargo build --release --example file_upload_io_uring
```

Use `--release` for accurate performance measurements (debug builds are much slower).

### Step 2: Test File Generation

Generate test file once:

```bash
cargo run --release --example file_upload_io_uring -- \
    --backend tokio \
    --file "C:/temp/test_1gb.bin" \
    --file-size-mib 1024 \
    --chunk-mib 8
```

This creates a 1 GiB test file. For subsequent runs, use `--file-size-mib 0` to reuse existing file.

### Step 3: Run Tokio Baseline

```bash
cargo run --release --example file_upload_io_uring -- \
    --backend tokio \
    --file "C:/temp/test_1gb.bin" \
    --file-size-mib 0 \
    --chunk-mib 8
```

**Record**:
- Total time from log output
- Visual observation of CPU usage (Task Manager / htop)
- Note any errors or warnings

### Step 4: Run io_uring Test

```bash
cargo run --release --example file_upload_io_uring -- \
    --backend uring \
    --file "C:/temp/test_1gb.bin" \
    --file-size-mib 0 \
    --chunk-mib 8 \
    --concurrency 64
```

**Record**:
- Total time from log output
- Visual observation of CPU usage
- Note any errors or warnings

### Step 5: Repeat for Statistical Validity

Run each test **at least 3 times** and calculate:
- Mean (average)
- Standard deviation
- Minimum and maximum

Example:
```
Tokio runs: 45.2s, 44.8s, 45.1s
Mean: 45.03s, StdDev: 0.21s

io_uring runs: 38.7s, 39.1s, 38.9s
Mean: 38.9s, StdDev: 0.20s

Improvement: (45.03 - 38.9) / 45.03 = 13.6%
```

## Data Collection

### Metrics to Record

For each test run, create a record with:

```
Test ID: <unique identifier>
Date: <timestamp>
Backend: tokio | uring
File Size: <MiB>
Chunk Size: <MiB>
Concurrency: <N> (uring only)
Network: local | remote (describe)
Storage: HDD | SSD | NVMe (describe)

Results:
  Total Time: <seconds>
  Read Time: <seconds> (from log)
  Upload Time: <seconds> (calculated)
  Throughput: <MB/s>
  CPU Usage: <%> (observed)
  Memory Usage: <MB> (observed)

Notes:
  <Any observations, errors, or anomalies>
```

### Sample Data Collection Template

Create a CSV file: `io_uring_experiment_results.csv`

```csv
test_id,date,backend,file_size_mib,chunk_size_mib,concurrency,network_type,storage_type,total_time_s,read_time_s,upload_time_s,throughput_mbs,cpu_pct,memory_mb,notes
1,2025-10-28,tokio,10,8,N/A,local,NVMe,2.45,0.05,2.40,4.08,15,120,baseline
2,2025-10-28,uring,10,8,64,local,NVMe,2.42,0.02,2.40,4.13,12,130,
3,2025-10-28,tokio,100,8,N/A,local,NVMe,15.2,0.45,14.75,6.58,18,150,
4,2025-10-28,uring,100,8,64,local,NVMe,14.1,0.15,13.95,7.09,14,160,
...
```

## Measuring System Metrics

### CPU Usage (Windows)

**During test**:
1. Open Task Manager (Ctrl+Shift+Esc)
2. Go to Performance tab
3. Monitor CPU usage during test run
4. Note peak and average usage

**Command line** (PowerShell):
```powershell
Get-Counter '\Processor(_Total)\% Processor Time' -SampleInterval 1 -MaxSamples 60
```

### CPU Usage (Linux)

**During test**:
```bash
# Terminal 1: Run test
cargo run --release --example file_upload_io_uring -- ...

# Terminal 2: Monitor CPU
top -d 1
# Or
htop
```

**Command line**:
```bash
# Sample CPU every second for 60 seconds
sar -u 1 60
```

### Memory Usage (Windows)

**PowerShell**:
```powershell
Get-Process cargo | Select-Object WS
```

### Memory Usage (Linux)

```bash
# Watch memory usage of running process
watch -n 1 'ps aux | grep file_upload_io_uring'
```

### Network Latency Measurement

Before running experiments, measure baseline network latency:

**Ping test**:
```bash
# Windows
ping -n 100 <minio-server-ip>

# Linux
ping -c 100 <minio-server-ip>
```

Record:
- Minimum RTT
- Average RTT
- Maximum RTT
- Packet loss

**HTTP latency test** (using curl):
```bash
# Measure MinIO API latency
time curl -X HEAD http://<minio-server>/bucket/object
```

Run 10 times and average the results.

### Storage Performance Measurement

**Sequential read test** (Linux):
```bash
# Create test file
dd if=/dev/zero of=testfile bs=1M count=1024

# Test sequential read
dd if=testfile of=/dev/null bs=8M
```

**Sequential read test** (Windows PowerShell):
```powershell
# Use DiskSpd or similar tool
# Or time a simple read:
Measure-Command { Get-Content -Path testfile -Raw }
```

Record the read throughput (MB/s).

## Analysis

### Calculating Improvement

For each scenario:

```
Improvement (%) = (Tokio_Time - Uring_Time) / Tokio_Time × 100

Example:
Tokio: 45.0s
Uring: 39.0s
Improvement: (45.0 - 39.0) / 45.0 × 100 = 13.3%
```

### Statistical Significance

With multiple runs, calculate if the difference is statistically significant:

```
t = (Mean_Tokio - Mean_Uring) / sqrt((StdDev_Tokio² + StdDev_Uring²) / N)
```

If |t| > 2 (approximately), the difference is likely significant.

### Breakdown Analysis

Calculate time breakdown:

```
Read_Time = (logged read time)
Upload_Time = Total_Time - Read_Time
Network_RTT_Time = (chunk_count × avg_RTT)
Server_Processing = Upload_Time - Network_RTT_Time
```

This helps identify bottlenecks.

## Results Documentation

### Create Results Document

File: `docs/io-uring-experiment-results.md`

Structure:
```markdown
# io_uring Experiment Results

## Test Environment

- Date: <date range>
- System: <OS, kernel version>
- Storage: <type, model>
- Network: <type, latency to MinIO>
- MinIO Version: <version>

## Summary Findings

<High-level conclusions>

## Detailed Results

### Scenario 1: Small Files (10 MiB)

#### Configuration
...

#### Results
| Backend | Mean Time | StdDev | Throughput |
|---------|-----------|--------|------------|
| Tokio   | ...       | ...    | ...        |
| Uring   | ...       | ...    | ...        |

#### Analysis
...

### Scenario 2: ...

## Conclusions

## Recommendations
```

### Visualization

Create graphs showing:
1. **Time vs File Size** (both backends on same graph)
2. **Time vs Chunk Size** (both backends)
3. **Time vs Concurrency** (uring only)
4. **Time Breakdown** (stacked bar: read time vs upload time)

Tools:
- Excel / Google Sheets
- Python matplotlib
- gnuplot
- Any graphing tool

## Troubleshooting

### Issue: Test Fails with Connection Error

**Cause**: MinIO server not accessible

**Solution**:
1. Verify MinIO is running: `curl http://localhost:9000/minio/health/live`
2. Check environment variables are set correctly
3. Test with mc client: `mc alias set test http://localhost:9000 minioadmin minioadmin`

### Issue: File Generation Very Slow

**Cause**: Random data generation is CPU-intensive

**Solution**:
1. Use release build: `cargo build --release`
2. Generate file once, reuse for tests
3. Consider using a pre-existing file

### Issue: Large Variance in Results

**Cause**: System load, background processes, network jitter

**Solution**:
1. Close unnecessary applications
2. Run tests during low-activity periods
3. Increase number of test runs (5-10 instead of 3)
4. Exclude outliers (runs with >2× standard deviations from mean)

### Issue: io_uring Backend Slower Than Tokio

**Possible causes**:
1. Concurrency too high (try lower values: 16, 32)
2. Concurrency too low (try higher values: 128)
3. Storage doesn't benefit from parallel I/O (HDD)
4. Small files where overhead dominates
5. High network latency makes disk I/O insignificant

**Investigation**:
- Check if read time actually improved (should be in logs)
- Test with different concurrency values
- Measure storage parallel I/O capability independently

## Checklist

Before starting experiments:

- [ ] MinIO server running and accessible
- [ ] Environment variables configured
- [ ] Upload code uncommented in example
- [ ] Release build completed
- [ ] Test file paths valid
- [ ] Data collection template prepared
- [ ] System monitoring tools ready
- [ ] Sufficient disk space for test files
- [ ] Network baseline measurements taken
- [ ] Storage baseline measurements taken

During experiments:

- [ ] Run each test at least 3 times
- [ ] Record all metrics consistently
- [ ] Note any anomalies or errors
- [ ] Monitor system resources
- [ ] Maintain same conditions within test groups

After experiments:

- [ ] Calculate statistics (mean, stddev)
- [ ] Calculate improvement percentages
- [ ] Analyze time breakdowns
- [ ] Create visualizations
- [ ] Document conclusions
- [ ] Update theoretical analysis document with real data

## Next Steps After Data Collection

1. **Analyze results against predictions**
   - Compare actual vs theoretical performance
   - Identify surprising results
   - Explain discrepancies

2. **Update documentation**
   - Add real data to `io-uring-experiment.md`
   - Replace "theoretical" with "measured"
   - Provide evidence-based recommendations

3. **Make implementation decisions**
   - Keep io_uring as optional feature?
   - Make it default for certain scenarios?
   - Document when users should enable it

4. **Consider follow-up experiments**
   - Test multipart upload with io_uring
   - Test multiple concurrent uploads
   - Test different storage types
   - Test different network conditions
