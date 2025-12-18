# MinIO S3 Vectors Architecture

## Client-Server Model

### Rust SDK (minio-rs-vectors)
- **Role**: Client SDK
- **Purpose**: Provides Rust API to interact with MinIO S3 Vectors
- **Testing**: Examples in `examples/s3vectors/` test the full system by calling the server
- **Does NOT contain**: Actual algorithm implementations (HNSW, Vamana)

### Go Server (eos-vectors)
- **Role**: MinIO AIStor server
- **Purpose**: Implements the actual vector search algorithms
- **Location**: `C:\Source\minio\eos-vectors`
- **Contains**:
  - `internal/hnsw/` - HNSW algorithm implementation
  - `internal/vamana/` - Vamana algorithm implementation
  - `cmd/vectors-api-handlers.go` - API endpoints

## How Testing Works

When running `cargo run --example s3vectors_recall`:
1. Rust client creates vectors and sends them to the server
2. Go server stores vectors and builds index (HNSW or Vamana)
3. Rust client sends query requests
4. Go server executes the algorithm and returns results
5. Rust client compares results to calculate recall

## Algorithm Selection

The algorithm is specified when creating an index:
- `algorithm: "hnsw"` - Uses HNSW implementation in `internal/hnsw/`
- `algorithm: "vamana"` - Uses Vamana implementation in `internal/vamana/`
- `algorithm: "bruteforce"` - Uses brute-force search

## Development Workflow

1. **Modify algorithm**: Edit Go code in `eos-vectors/internal/hnsw/` or `eos-vectors/internal/vamana/`
2. **Rebuild server**: Run `make install` in `eos-vectors/`
3. **Test changes**: Run examples from `minio-rs-vectors/examples/s3vectors/`
