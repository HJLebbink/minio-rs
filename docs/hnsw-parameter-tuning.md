# HNSW Parameter Tuning Guide for MinIO S3 Vectors

This document summarizes HNSW parameter tuning for MinIO's S3 Vectors implementation.

## Key Parameters

### m (Maximum Connections Per Node)
Controls the graph connectivity. Higher values improve recall but increase memory and index build time.

| m Value | Use Case |
|---------|----------|
| 16-32   | Maximum throughput, lower memory |
| 32-48   | Balanced performance (recommended) |
| 48-64   | High recall requirements |

**Note**: MinIO enforces m <= 100.

### efConstruction (Build-time Search Depth)
Controls index quality during construction. Higher values build better indexes but take longer.

| efConstruction | Use Case |
|----------------|----------|
| 50-100         | Fast indexing |
| 100-200        | Balanced (recommended) |
| 200-400        | High-quality index for demanding recall requirements |

### efSearch (Query-time Search Depth)
Controls search thoroughness at query time. **This is the primary parameter for tuning recall vs latency.**

Higher efSearch values explore more candidates, improving recall but increasing query latency.

## Performance Benchmarks

### 50,000 vectors, 768 dimensions (text embeddings), top_k=10, Cosine distance:

Index parameters: m=48, efConstruction=200

| Method | efSearch | Recall | Latency | QPS |
|--------|----------|--------|---------|-----|
| Brute-force | N/A | 100% | 95.7ms | 10.5 |
| HNSW | 50 | 42.5% | 6.9ms | 144 |
| HNSW | 100 | 54.5% | 15.2ms | 66 |
| HNSW | 200 | 73.5% | 31.3ms | 32 |
| HNSW | 500 | 87.5% | 58.8ms | 17 |
| HNSW | 1000 | 94.0% | 105.9ms | 9.4 |

**Key observations:**
- HNSW at efSearch=50-100 provides 6-14x higher throughput than brute-force
- For >90% recall, use efSearch >= 500
- At very high efSearch values (1000+), HNSW may be slower than brute-force while still not achieving 100% recall
- For workloads requiring >95% recall, consider brute-force search or tuning index parameters

## Recommended Configurations

### High Recall (>95%)
```json
{
    "m": 48,
    "efConstruction": 200
}
```
Query with: `efSearch: 500-1000` or use `bruteForce: true`

### Balanced Performance
```json
{
    "m": 32,
    "efConstruction": 200
}
```
Query with: `efSearch: 100-200`

### Maximum Throughput
```json
{
    "m": 16,
    "efConstruction": 100
}
```
Query with: `efSearch: 50-100`

## Index Readiness

Always wait for the index to be fully built before querying. Use the `GetIndex` API:

```json
{
  "index": {
    "vectorCount": 1000,
    "status": "READY"
  }
}
```

**Polling strategy**:
```rust
loop {
    let response = client.get_index(bucket, index).build().send().await?;
    let info = response.index()?;

    if info.status == IndexStatus::Ready && info.vector_count >= expected_count {
        break;
    }

    tokio::time::sleep(Duration::from_secs(2)).await;
}
```

## Distance Metrics

Both distance metrics are supported:

| Metric | Range | Best For |
|--------|-------|----------|
| Cosine | [0, 2] | Text embeddings, semantic similarity |
| Euclidean | [0, ∞) | Spatial data, image embeddings |

## QueryVectors Parameters

```json
{
    "queryVector": {"float32": [...]},
    "topK": 10,
    "efSearch": 200,
    "bruteForce": false
}
```

- `efSearch`: Override default search depth (1-1000)
- `bruteForce`: Use exact search for 100% recall (slower for large indexes)

## Summary

1. **efSearch controls recall**: Higher values improve recall at the cost of latency
2. **m affects index quality**: Higher values improve recall ceiling but increase memory/build time
3. **Brute-force for 100% recall**: Use when accuracy is critical and latency is acceptable
4. **Wait for index readiness**: Query only when `vectorCount` matches expected and `status` is READY
