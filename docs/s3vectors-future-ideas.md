# S3 Vectors Future Ideas

This document captures future enhancement ideas for MinIO's S3 Vectors implementation.

## Search Algorithm Abstraction

### Current State

Currently, the search method is hardcoded as HNSW with a brute-force fallback option. Parameters are passed as individual fields.

### Proposed Enhancement

Make search algorithms pluggable at the API level. This would allow:

1. **Multiple algorithm implementations**: HNSW, Vamana, brute-force, and future algorithms
2. **Algorithm-specific parameters**: Each algorithm has its own parameter struct
3. **Runtime algorithm selection**: Choose algorithm per-query or per-index

### API Design Options

#### Option A: Search Method Enum with Algorithm-Specific Params

```rust
enum SearchMethod {
    Hnsw(HnswParams),
    Vamana(VamanaParams),
    BruteForce,
}

struct HnswParams {
    ef_search: Option<u32>,
}

struct VamanaParams {
    search_list_size: u32,
    // ... other Vamana-specific params
}
```

#### Option B: Trait-Based Search Providers

```rust
trait SearchProvider {
    type Params;
    fn search(&self, query: &[f32], params: Self::Params) -> Vec<SearchResult>;
}

impl SearchProvider for HnswIndex { ... }
impl SearchProvider for VamanaIndex { ... }
```

### Related Work

User has Vamana implementation at `c:\source\Private\rust\vecdb` that could be integrated.

---

## HNSW Recall Investigation (Resolved)

### Issue Summary

The MinIO HNSW implementation showed unexpectedly low recall that didn't improve with higher ef_search values.

### Root Cause

The early termination condition in `internal/hnsw/graph.go` was too aggressive. It terminated search based solely on result quality without respecting the ef_search parameter:

```go
// Old buggy code
if result.Len() >= k && current.dist > result.Max().dist {
    break  // Terminated too early, ignoring ef_search
}
```

### Fix Applied

Added a step counter to ensure at least ef_search candidates are explored:

```go
// Fixed code
nstep := 0
for candidates.Len() > 0 {
    current := candidates.Pop()
    nstep++
    if nstep > efSearch && result.Len() >= k && current.dist > result.Max().dist {
        break
    }
    // ...
}
```

### Current Behavior

ef_search now properly controls recall vs latency tradeoff. See `hnsw-parameter-tuning.md` for benchmarks.

---

## Index State API

### Motivation

When vectors are uploaded, the HNSW index may still be building. Currently there's no way to know when indexing is complete.

### Proposed API

```json
GET /GetIndexStatus

{
    "indexName": "my-index",
    "vectorBucketName": "my-bucket"
}

Response:
{
    "status": "INDEXING" | "READY" | "ERROR",
    "vectorsIndexed": 9500,
    "vectorsTotal": 10000,
    "percentComplete": 95.0,
    "estimatedTimeRemaining": "2m30s"
}
```

### Implementation Notes

- Track indexing progress during bulk uploads
- Consider background indexing for large batches
- Provide ETA based on recent indexing rate

---

## Performance Optimizations

### SIMD Distance Computation

The current distance functions can be accelerated with SIMD instructions:

```go
// Current implementation
func cosineDistanceFloat32(a, b []float32) float32 {
    // Scalar loop
}

// SIMD-accelerated (AVX2/AVX-512)
func cosineDistanceSIMD(a, b []float32) float32 {
    // Use vectorized operations
}
```

### Batch Query Support

Allow multiple queries in a single request to reduce round-trip overhead:

```json
{
    "queries": [
        {"queryVector": [...], "topK": 10},
        {"queryVector": [...], "topK": 5}
    ]
}
```

### Memory-Mapped Indexes

For large indexes that don't fit in memory:

- Memory-map the graph structure
- On-demand node loading
- LRU cache for frequently accessed nodes

---

## Additional Index Types

### Flat Index

Simple exhaustive search, useful for:
- Small datasets (<10k vectors)
- Baseline comparison
- When 100% recall is required

### IVF (Inverted File Index)

Good for large datasets with clustering:
- Partition vectors into clusters
- Search only relevant clusters
- Good recall/speed tradeoff

### Product Quantization

For memory-constrained scenarios:
- Compress vectors using PQ
- Trade accuracy for memory
- Support millions of vectors

---

## Filtering Enhancements

### Pre-filtering vs Post-filtering

Current implementation does post-filtering (search, then filter). Pre-filtering can be more efficient:

```json
{
    "preFilter": {
        "category": "electronics",
        "price": {"$lt": 100}
    },
    "postFilter": {
        "inStock": true
    }
}
```

### Hybrid Search

Combine vector similarity with keyword search:

```json
{
    "queryVector": [...],
    "keywords": "wireless bluetooth",
    "hybridWeight": 0.7  // 70% vector, 30% keyword
}
```

---

## Replication and Distribution

### Multi-Node Index Distribution

For large indexes that exceed single-node capacity:

- Shard indexes across nodes
- Distributed search with result merging
- Automatic rebalancing

### Index Replication

For high availability:

- Replicate indexes across nodes
- Automatic failover
- Read scaling

---

## Monitoring and Observability

### Metrics

- Query latency histograms (p50, p95, p99)
- Index build time
- Memory usage per index
- QPS per index

### Tracing

- Trace query execution path
- Identify bottlenecks
- Debug low recall issues

---

## API Compatibility

### AWS S3 Vectors Compatibility

As AWS S3 Vectors evolves, track new features:

- New distance metrics
- New data types
- Filtering improvements

### MinIO Extensions Documentation

Clearly document MinIO-specific extensions:

- HNSW parameters (m, ef_construction, ef_search)
- Brute-force search mode
- Index status API
- Future algorithm support
