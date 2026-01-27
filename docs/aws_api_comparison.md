# AWS S3 Tables API vs MinIO Rust SDK Comparison

This document provides a line-by-line comparison between the AWS S3 Tables Control Plane API and the MinIO Rust SDK S3 Tables implementation.

## Important Note

There are **two different APIs** for S3 Tables:

1. **AWS S3 Tables Control Plane API** - Native AWS API (49 operations)
   - Used by `awslabs/s3-tables-catalog` via AWS SDK
   - Endpoint: `https://s3tables.<region>.amazonaws.com`

2. **Iceberg REST Catalog API** - Standard Apache Iceberg REST API
   - Used by MinIO and compatible with any Iceberg REST client
   - Endpoint: `/_iceberg/v1/` (MinIO) or `/v1/` (standard)

The MinIO Rust SDK implements the **Iceberg REST Catalog API** but also includes AWS S3 Tables-specific extensions for features like encryption, maintenance, replication, etc.

---

## Warehouse / Table Bucket Operations

| AWS S3 Tables API | MinIO Rust SDK | Status | Notes |
|-------------------|----------------|--------|-------|
| `CreateTableBucket` | `create_warehouse` | ✅ Implemented | AWS: table bucket, MinIO: warehouse |
| `DeleteTableBucket` | `delete_warehouse` | ✅ Implemented | |
| `GetTableBucket` | `get_warehouse` | ✅ Implemented | |
| `ListTableBuckets` | `list_warehouses` | ✅ Implemented | |

### Code Comparison: CreateTableBucket / create_warehouse

**AWS Java SDK (awslabs/s3-tables-catalog):**
```java
// Not directly exposed - uses AWS SDK
CreateTableBucketRequest request = CreateTableBucketRequest.builder()
    .name(bucketName)
    .build();
s3TablesClient.createTableBucket(request);
```

**MinIO Rust SDK:**
```rust
// src/s3tables/builders/create_warehouse.rs
let resp = tables
    .create_warehouse(warehouse_name)
    .unwrap()
    .build()
    .send()
    .await?;
```

**HTTP Request (MinIO):**
```
POST /_iceberg/v1/warehouses
Content-Type: application/json

{
  "name": "warehouse-name"
}
```

---

## Namespace Operations

| AWS S3 Tables API | MinIO Rust SDK | Status | Notes |
|-------------------|----------------|--------|-------|
| `CreateNamespace` | `create_namespace` | ✅ Implemented | |
| `DeleteNamespace` | `delete_namespace` | ✅ Implemented | |
| `GetNamespace` | `get_namespace` | ✅ Implemented | |
| `ListNamespaces` | `list_namespaces` | ✅ Implemented | |
| - | `namespace_exists` | ✅ Extra | HEAD request |
| - | `update_namespace_properties` | ✅ Extra | Iceberg REST API |

### Code Comparison: CreateNamespace

**AWS Java SDK:**
```java
// S3TablesCatalog.java line ~180
public void createNamespace(Namespace namespace, Map<String, String> metadata) {
    Preconditions.checkArgument(
        namespace.levels().length == 1,
        "S3 Tables only supports single-level namespaces");

    CreateNamespaceRequest request = CreateNamespaceRequest.builder()
        .tableBucketARN(tableBucketArn)
        .namespace(namespace.levels())
        .build();

    try {
        tablesClient.createNamespace(request);
    } catch (ConflictException e) {
        throw new AlreadyExistsException("Namespace already exists: %s", namespace);
    }
}
```

**MinIO Rust SDK:**
```rust
// src/s3tables/builders/create_namespace.rs
impl ToTablesRequest for CreateNamespace {
    fn to_tables_request(&self) -> Result<TablesRequest, Error> {
        let body = CreateNamespaceRequest {
            namespace: self.namespace.as_slice().to_vec(),
            properties: self.properties.clone(),
        };

        Ok(TablesRequest {
            method: Method::POST,
            path: format!("/{}/namespaces", self.warehouse_name.as_str()),
            body: Some(serde_json::to_vec(&body)?),
            ..Default::default()
        })
    }
}
```

**HTTP Request (MinIO):**
```
POST /_iceberg/v1/{warehouse}/namespaces
Content-Type: application/json

{
  "namespace": ["namespace_name"],
  "properties": {"owner": "team-a"}
}
```

---

## Table Operations

| AWS S3 Tables API | MinIO Rust SDK | Status | Notes |
|-------------------|----------------|--------|-------|
| `CreateTable` | `create_table` | ✅ Implemented | |
| `DeleteTable` | `delete_table` | ✅ Implemented | Supports `purge` param |
| `GetTable` | `load_table` | ✅ Implemented | |
| `ListTables` | `list_tables` | ✅ Implemented | |
| `RenameTable` | `rename_table` | ✅ Implemented | |
| `UpdateTableMetadataLocation` | `commit_table` | ✅ Implemented | Iceberg commit with requirements |
| `GetTableMetadataLocation` | (in `load_table`) | ✅ Implemented | Returned in metadata |
| - | `table_exists` | ✅ Extra | HEAD request |
| - | `register_table` | ✅ Extra | Register existing metadata |

### Code Comparison: CreateTable

**AWS Java SDK:**
```java
// Uses BaseMetastoreCatalog.createTable() inherited method
// Internally calls S3TablesTableOperations for metadata management

// S3TablesCatalog.java - loadTable returns operations
protected TableOperations newTableOps(TableIdentifier tableIdentifier) {
    return new S3TablesCatalogOperations(
        tablesClient,
        tableBucketArn,
        tableIdentifier,
        fileIOTracker,
        catalogOptions);
}
```

**MinIO Rust SDK:**
```rust
// src/s3tables/builders/create_table.rs
impl ToTablesRequest for CreateTable {
    fn to_tables_request(&self) -> Result<TablesRequest, Error> {
        let body = CreateTableRequest {
            name: self.table_name.as_str().to_string(),
            schema: self.schema.clone(),
            partition_spec: self.partition_spec.clone(),
            write_order: self.sort_order.clone(),
            properties: self.properties.clone(),
            location: self.location.clone(),
        };

        Ok(TablesRequest {
            method: Method::POST,
            path: format!("/{}/namespaces/{}/tables",
                self.warehouse_name.as_str(),
                encode_namespace(&self.namespace)),
            body: Some(serde_json::to_vec(&body)?),
            ..Default::default()
        })
    }
}
```

**HTTP Request (MinIO):**
```
POST /_iceberg/v1/{warehouse}/namespaces/{namespace}/tables
Content-Type: application/json

{
  "name": "table_name",
  "schema": {
    "type": "struct",
    "fields": [
      {"id": 1, "name": "id", "required": true, "type": "long"}
    ]
  },
  "partition-spec": {...},
  "write-order": {...},
  "properties": {}
}
```

### Code Comparison: UpdateTableMetadataLocation vs commit_table

**AWS Java SDK:**
```java
// S3TablesCatalogOperations.java - doCommit()
protected void doCommit(TableMetadata base, TableMetadata metadata) {
    String newMetadataLocation = writeNewMetadataIfRequired(metadata);

    GetTableMetadataLocationResponse response =
        tablesClient.getTableMetadataLocation(
            GetTableMetadataLocationRequest.builder()
                .tableBucketARN(tableBucketArn)
                .namespace(namespace)
                .name(tableName)
                .build());

    checkMetadataLocation(response, base);

    tablesClient.updateTableMetadataLocation(
        UpdateTableMetadataLocationRequest.builder()
            .tableBucketARN(tableBucketArn)
            .namespace(namespace)
            .name(tableName)
            .versionToken(response.versionToken())
            .metadataLocation(newMetadataLocation)
            .build());
}
```

**MinIO Rust SDK:**
```rust
// src/s3tables/builders/commit_table.rs
impl ToTablesRequest for CommitTable {
    fn to_tables_request(&self) -> Result<TablesRequest, Error> {
        let body = CommitTableRequest {
            identifier: TableIdentifier {
                namespace: self.namespace.as_slice().to_vec(),
                name: self.table_name.as_str().to_string(),
            },
            requirements: self.requirements.clone(),
            updates: self.updates.clone(),
        };

        Ok(TablesRequest {
            method: Method::POST,
            path: format!("/{}/namespaces/{}/tables/{}",
                self.warehouse_name.as_str(),
                encode_namespace(&self.namespace),
                self.table_name.as_str()),
            body: Some(serde_json::to_vec(&body)?),
            ..Default::default()
        })
    }
}
```

**HTTP Request (MinIO):**
```
POST /_iceberg/v1/{warehouse}/namespaces/{namespace}/tables/{table}
Content-Type: application/json

{
  "identifier": {"namespace": ["ns"], "name": "table"},
  "requirements": [
    {"type": "assert-table-uuid", "uuid": "..."},
    {"type": "assert-ref-snapshot-id", "ref": "main", "snapshot-id": 123}
  ],
  "updates": [
    {"action": "add-snapshot", "snapshot": {...}},
    {"action": "set-snapshot-ref", "ref-name": "main", ...}
  ]
}
```

---

## Encryption Operations

| AWS S3 Tables API | MinIO Rust SDK | Status | Notes |
|-------------------|----------------|--------|-------|
| `GetTableBucketEncryption` | `get_warehouse_encryption` | ✅ Implemented | |
| `PutTableBucketEncryption` | `put_warehouse_encryption` | ✅ Implemented | |
| `DeleteTableBucketEncryption` | `delete_warehouse_encryption` | ✅ Implemented | |
| `GetTableEncryption` | `get_table_encryption` | ✅ Implemented | |
| - | `put_table_encryption` | ✅ Extra | |
| - | `delete_table_encryption` | ✅ Extra | |

---

## Maintenance Operations

| AWS S3 Tables API | MinIO Rust SDK | Status | Notes |
|-------------------|----------------|--------|-------|
| `GetTableBucketMaintenanceConfiguration` | `get_warehouse_maintenance` | ✅ Implemented | |
| `PutTableBucketMaintenanceConfiguration` | `put_warehouse_maintenance` | ✅ Implemented | |
| `GetTableMaintenanceConfiguration` | `get_table_maintenance` | ✅ Implemented | |
| `PutTableMaintenanceConfiguration` | `put_table_maintenance` | ✅ Implemented | |
| `GetTableMaintenanceJobStatus` | `get_table_maintenance_job_status` | ✅ Implemented | |

---

## Policy Operations

| AWS S3 Tables API | MinIO Rust SDK | Status | Notes |
|-------------------|----------------|--------|-------|
| `GetTableBucketPolicy` | `get_warehouse_policy` | ✅ Implemented | |
| `PutTableBucketPolicy` | `put_warehouse_policy` | ✅ Implemented | |
| `DeleteTableBucketPolicy` | `delete_warehouse_policy` | ✅ Implemented | |
| `GetTablePolicy` | `get_table_policy` | ✅ Implemented | |
| `PutTablePolicy` | `put_table_policy` | ✅ Implemented | |
| `DeleteTablePolicy` | `delete_table_policy` | ✅ Implemented | |

---

## Replication Operations

| AWS S3 Tables API | MinIO Rust SDK | Status | Notes |
|-------------------|----------------|--------|-------|
| `GetTableBucketReplication` | `get_warehouse_replication` | ✅ Implemented | |
| `PutTableBucketReplication` | `put_warehouse_replication` | ✅ Implemented | |
| `DeleteTableBucketReplication` | `delete_warehouse_replication` | ✅ Implemented | |
| `GetTableReplication` | `get_table_replication` | ✅ Implemented | |
| `PutTableReplication` | `put_table_replication` | ✅ Implemented | |
| `DeleteTableReplication` | `delete_table_replication` | ✅ Implemented | |
| `GetTableReplicationStatus` | `get_table_replication_status` | ✅ Implemented | |

---

## Storage Class Operations

| AWS S3 Tables API | MinIO Rust SDK | Status | Notes |
|-------------------|----------------|--------|-------|
| `GetTableBucketStorageClass` | `get_warehouse_storage_class` | ✅ Implemented | |
| `PutTableBucketStorageClass` | `put_warehouse_storage_class` | ✅ Implemented | |
| `GetTableStorageClass` | `get_table_storage_class` | ✅ Implemented | |

---

## Metrics Operations

| AWS S3 Tables API | MinIO Rust SDK | Status | Notes |
|-------------------|----------------|--------|-------|
| `GetTableBucketMetricsConfiguration` | `get_warehouse_metrics` | ✅ Implemented | |
| `PutTableBucketMetricsConfiguration` | `put_warehouse_metrics` | ✅ Implemented | |
| `DeleteTableBucketMetricsConfiguration` | `delete_warehouse_metrics` | ✅ Implemented | |
| - | `table_metrics` | ✅ Extra | Report table metrics |

---

## Record Expiration Operations

| AWS S3 Tables API | MinIO Rust SDK | Status | Notes |
|-------------------|----------------|--------|-------|
| `GetTableRecordExpirationConfiguration` | `get_table_expiration` | ✅ Implemented | |
| `PutTableRecordExpirationConfiguration` | `put_table_expiration` | ✅ Implemented | |
| `GetTableRecordExpirationJobStatus` | `get_table_expiration_job_status` | ✅ Implemented | |

---

## Tagging Operations

| AWS S3 Tables API | MinIO Rust SDK | Status | Notes |
|-------------------|----------------|--------|-------|
| `ListTagsForResource` | `list_tags_for_resource` | ✅ Implemented | |
| `TagResource` | `tag_resource` | ✅ Implemented | |
| `UntagResource` | `untag_resource` | ✅ Implemented | |

---

## View Operations (Iceberg REST API Only)

| AWS S3 Tables API | MinIO Rust SDK | Status | Notes |
|-------------------|----------------|--------|-------|
| - | `create_view` | ✅ Extra | Iceberg REST API |
| - | `load_view` | ✅ Extra | Iceberg REST API |
| - | `list_views` | ✅ Extra | Iceberg REST API |
| - | `drop_view` | ✅ Extra | Iceberg REST API |
| - | `rename_view` | ✅ Extra | Iceberg REST API |
| - | `replace_view` | ✅ Extra | Iceberg REST API |
| - | `view_exists` | ✅ Extra | Iceberg REST API |

---

## Scan Planning Operations (Iceberg REST API Only)

| AWS S3 Tables API | MinIO Rust SDK | Status | Notes |
|-------------------|----------------|--------|-------|
| - | `plan_table_scan` | ✅ Extra | Server-side scan planning |
| - | `fetch_planning_result` | ✅ Extra | Get planning result |
| - | `fetch_scan_tasks` | ✅ Extra | Get scan tasks |
| - | `execute_table_scan` | ✅ Extra | Execute scan |
| - | `cancel_planning` | ✅ Extra | Cancel planning |

---

## Configuration Operations (Iceberg REST API Only)

| AWS S3 Tables API | MinIO Rust SDK | Status | Notes |
|-------------------|----------------|--------|-------|
| - | `get_config` | ✅ Extra | Get catalog config |

---

## Transaction Operations (Iceberg REST API Only)

| AWS S3 Tables API | MinIO Rust SDK | Status | Notes |
|-------------------|----------------|--------|-------|
| - | `commit_multi_table_transaction` | ✅ Extra | Atomic multi-table |

---

## Credentials Operations (Iceberg REST API Only)

| AWS S3 Tables API | MinIO Rust SDK | Status | Notes |
|-------------------|----------------|--------|-------|
| - | `load_table_credentials` | ✅ Extra | Get table credentials |

---

## Summary

### AWS S3 Tables API Coverage

| Category | AWS Operations | MinIO Implemented | Coverage |
|----------|---------------|-------------------|----------|
| Warehouse/Bucket | 4 | 4 | 100% |
| Namespace | 4 | 6 | 150% (extras) |
| Table | 7 | 10 | 143% (extras) |
| Encryption | 4 | 6 | 150% (extras) |
| Maintenance | 5 | 5 | 100% |
| Policy | 6 | 6 | 100% |
| Replication | 7 | 7 | 100% |
| Storage Class | 3 | 3 | 100% |
| Metrics | 3 | 4 | 133% (extras) |
| Expiration | 3 | 3 | 100% |
| Tagging | 3 | 3 | 100% |
| **Total AWS** | **49** | **57** | **116%** |

### Additional Iceberg REST API Operations

| Category | Operations |
|----------|------------|
| Views | 7 |
| Scan Planning | 5 |
| Configuration | 1 |
| Transactions | 1 |
| Credentials | 1 |
| **Total Extra** | **15** |

### Grand Total

- **AWS S3 Tables API**: 49 operations - **All implemented**
- **Iceberg REST Extras**: 15 operations - **All implemented**
- **Total MinIO SDK**: **72 operations**

---

## Key Differences

### 1. API Style

**AWS S3 Tables (Control Plane):**
- Uses AWS SDK with typed request/response objects
- Authentication via AWS SigV4
- Endpoint: `https://s3tables.<region>.amazonaws.com`

**MinIO Rust SDK (Iceberg REST):**
- Uses HTTP REST with JSON payloads
- Authentication via SigV4 or Bearer token
- Endpoint: `/_iceberg/v1/` (configurable)

### 2. Table Updates

**AWS:**
- Uses `UpdateTableMetadataLocation` with version token
- Simple location update

**MinIO/Iceberg REST:**
- Uses `commit_table` with requirements and updates
- Supports optimistic concurrency with multiple assertions
- Rich update actions (add-schema, add-snapshot, set-properties, etc.)

### 3. Terminology

| AWS S3 Tables | MinIO/Iceberg |
|---------------|---------------|
| Table Bucket | Warehouse |
| Table Bucket ARN | Warehouse Name |
| Metadata Location | Metadata Location |
| Version Token | Requirements |

### 4. Views

AWS S3 Tables Control Plane API does not include view operations. The MinIO SDK supports views via the Iceberg REST API.

---

## Sources

- [AWS S3 Tables API Reference](https://docs.aws.amazon.com/AmazonS3/latest/API/API_Operations_Amazon_S3_Tables.html)
- [awslabs/s3-tables-catalog](https://github.com/awslabs/s3-tables-catalog)
- [Apache Iceberg REST Catalog Spec](https://iceberg.apache.org/rest-catalog-spec/)
- [OpenAPI Specification](https://github.com/apache/iceberg/blob/main/open-api/rest-catalog-open-api.yaml)
