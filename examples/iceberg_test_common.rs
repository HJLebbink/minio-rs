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

//! Shared utilities for Iceberg REST API testing.
//!
//! This module provides common types and functions used by both the spec compliance
//! test (`iceberg_spec_test`) and sequence test (`iceberg_sequence_test`).

#![allow(dead_code)]

use minio::s3::MinioClient;
use minio::s3::creds::StaticProvider;
use minio::s3::http::BaseUrl;
use minio::s3tables::TablesClient;
use minio::s3tables::iceberg::{
    Field, FieldType, NullOrder, PartitionField, PartitionSpec, PrimitiveType, Schema, SchemaType,
    SortDirection, SortField, SortOrder, Transform,
};
use minio::s3tables::utils::{Namespace, TableName, ViewName, WarehouseName};
use std::collections::HashMap;

// ============================================================================
// Resource Types
// ============================================================================

/// Iceberg resource types for tracking state and dependencies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Resource {
    Warehouse,
    Namespace,
    Table,
    View,
}

impl std::fmt::Display for Resource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Resource::Warehouse => write!(f, "Warehouse"),
            Resource::Namespace => write!(f, "Namespace"),
            Resource::Table => write!(f, "Table"),
            Resource::View => write!(f, "View"),
        }
    }
}

// ============================================================================
// Resource State
// ============================================================================

/// Tracks which resources exist in the test environment.
///
/// Used to determine expected API responses and manage test setup/teardown.
#[derive(Debug, Clone, Default)]
pub struct ResourceState {
    pub warehouse: bool,
    pub namespace: bool,
    pub table: bool,
    pub view: bool,
}

impl ResourceState {
    /// Empty state - no resources exist.
    pub fn empty() -> Self {
        Self::default()
    }

    /// State with only warehouse.
    pub fn with_warehouse() -> Self {
        Self {
            warehouse: true,
            ..Default::default()
        }
    }

    /// State with warehouse and namespace.
    pub fn with_namespace() -> Self {
        Self {
            warehouse: true,
            namespace: true,
            ..Default::default()
        }
    }

    /// State with warehouse, namespace, and table.
    pub fn with_table() -> Self {
        Self {
            warehouse: true,
            namespace: true,
            table: true,
            ..Default::default()
        }
    }

    /// State with warehouse, namespace, and view.
    pub fn with_view() -> Self {
        Self {
            warehouse: true,
            namespace: true,
            view: true,
            ..Default::default()
        }
    }

    /// Check if a resource exists.
    pub fn has(&self, resource: Resource) -> bool {
        match resource {
            Resource::Warehouse => self.warehouse,
            Resource::Namespace => self.namespace,
            Resource::Table => self.table,
            Resource::View => self.view,
        }
    }

    /// Set resource existence.
    pub fn set(&mut self, resource: Resource, exists: bool) {
        match resource {
            Resource::Warehouse => self.warehouse = exists,
            Resource::Namespace => self.namespace = exists,
            Resource::Table => self.table = exists,
            Resource::View => self.view = exists,
        }
    }

    /// Short string representation (e.g., "W,N,T" or "empty").
    pub fn to_short_string(&self) -> String {
        let mut parts = Vec::new();
        if self.warehouse {
            parts.push("W");
        }
        if self.namespace {
            parts.push("N");
        }
        if self.table {
            parts.push("T");
        }
        if self.view {
            parts.push("V");
        }
        if parts.is_empty() {
            "empty".to_string()
        } else {
            parts.join(",")
        }
    }
}

// ============================================================================
// Test Context
// ============================================================================

/// Execution context for a single test case.
///
/// Holds the client and unique resource names to ensure test isolation.
pub struct TestContext {
    pub client: TablesClient,
    pub warehouse_name: String,
    pub namespace_name: String,
    pub table_name: String,
    pub view_name: String,
}

impl TestContext {
    /// Create a new test context with unique resource names.
    ///
    /// # Arguments
    /// * `test_id` - Unique identifier for this test (used in resource names)
    /// * `run_prefix` - Prefix for this test run (ensures isolation between runs)
    /// * `client` - The TablesClient to use for API calls
    ///
    /// Resource name rules:
    /// - Warehouse: lowercase letters, numbers, hyphens only
    /// - Namespace: letters, numbers, underscores only
    /// - Table/View: letters, numbers, underscores only
    pub fn new(test_id: &str, run_prefix: &str, client: TablesClient) -> Self {
        // Warehouse: use hyphens (no underscores allowed)
        let wh_id = test_id.replace('_', "-").to_lowercase();
        let wh_prefix = run_prefix.replace('_', "-").to_lowercase();

        // Namespace/Table/View: use underscores (no hyphens allowed)
        let ns_id = test_id.replace('-', "_");
        let ns_prefix = run_prefix.replace('-', "_");

        Self {
            client,
            warehouse_name: format!("wh-{}-{}", wh_prefix, wh_id),
            namespace_name: format!("ns_{}_{}", ns_prefix, ns_id),
            table_name: format!("tbl_{}_{}", ns_prefix, ns_id),
            view_name: format!("vw_{}_{}", ns_prefix, ns_id),
        }
    }

    pub fn warehouse(&self) -> WarehouseName {
        WarehouseName::try_from(self.warehouse_name.as_str())
            .unwrap_or_else(|e| panic!("Invalid warehouse name '{}': {:?}", self.warehouse_name, e))
    }

    pub fn namespace(&self) -> Namespace {
        Namespace::new(vec![self.namespace_name.clone()])
            .unwrap_or_else(|e| panic!("Invalid namespace name '{}': {:?}", self.namespace_name, e))
    }

    pub fn table(&self) -> TableName {
        TableName::try_from(self.table_name.as_str())
            .unwrap_or_else(|e| panic!("Invalid table name '{}': {:?}", self.table_name, e))
    }

    pub fn view(&self) -> ViewName {
        ViewName::try_from(self.view_name.as_str())
            .unwrap_or_else(|e| panic!("Invalid view name '{}': {:?}", self.view_name, e))
    }
}

// ============================================================================
// Client Creation
// ============================================================================

/// Create a TablesClient from endpoint and credentials.
pub fn create_client(endpoint: &str, access_key: &str, secret_key: &str) -> TablesClient {
    let base_url: BaseUrl = endpoint.parse().expect("Invalid endpoint URL");
    let provider = StaticProvider::new(access_key, secret_key, None);
    let minio_client =
        MinioClient::new(base_url, Some(provider), None, None).expect("Failed to create client");
    TablesClient::new(minio_client)
}

// ============================================================================
// Schema Helpers
// ============================================================================

/// Create a minimal valid Iceberg schema with a single `id` field.
pub fn minimal_schema() -> Schema {
    Schema {
        schema_type: SchemaType::Struct,
        schema_id: Some(0),
        identifier_field_ids: None,
        fields: vec![Field {
            id: 1,
            name: "id".to_string(),
            field_type: FieldType::Primitive(PrimitiveType::Long),
            required: true,
            doc: None,
            initial_default: None,
            write_default: None,
        }],
    }
}

/// Create a sample partition spec (bucket on id field).
pub fn sample_partition_spec() -> PartitionSpec {
    PartitionSpec {
        spec_id: 0,
        fields: vec![PartitionField {
            source_id: 1,
            field_id: 1000,
            name: "id_bucket".to_string(),
            transform: Transform::Bucket { n: 16 },
        }],
    }
}

/// Create a sample sort order (ascending by id).
///
/// Note: order_id must be non-zero when specifying sort fields.
/// Per the Iceberg spec, order_id 0 is reserved for "unsorted" (empty fields).
pub fn sample_sort_order() -> SortOrder {
    SortOrder {
        order_id: 1,
        fields: vec![SortField {
            source_id: 1,
            transform: Transform::Identity,
            direction: SortDirection::Asc,
            null_order: NullOrder::NullsLast,
        }],
    }
}

/// Create sample properties for testing.
pub fn sample_properties() -> HashMap<String, String> {
    let mut props = HashMap::new();
    props.insert("test.property".to_string(), "test-value".to_string());
    props
}

// ============================================================================
// Error Handling
// ============================================================================

/// Extract HTTP status code from an SDK error.
///
/// Returns the actual HTTP status code when available from the server response,
/// or infers it from semantic error types.
pub fn extract_status_code(error: &minio::s3::error::Error) -> u16 {
    use minio::s3::error::{Error, S3ServerError};

    match error {
        // S3ServerError::HttpError preserves the actual HTTP status code
        Error::S3Server(S3ServerError::HttpError(code, _)) => *code,

        // TablesError has a status_code() method that handles all variants
        Error::TablesError(tables_err) => tables_err.status_code(),

        // Fallback for other error types
        _ => 0,
    }
}

/// Extract entity type from an error message.
pub fn extract_entity_from_message(msg: &str) -> Option<&'static str> {
    let msg_lower = msg.to_lowercase();
    if msg_lower.contains("warehouse") {
        Some("Warehouse")
    } else if msg_lower.contains("namespace") {
        Some("Namespace")
    } else if msg_lower.contains("table") {
        Some("Table")
    } else if msg_lower.contains("view") {
        Some("View")
    } else {
        None
    }
}

/// Extract detail from a 400 Bad Request error.
pub fn extract_400_detail(msg: &str) -> Option<String> {
    let msg_lower = msg.to_lowercase();

    if msg_lower.contains("invalid") {
        if msg_lower.contains("partition") {
            Some("invalid partition".to_string())
        } else if msg_lower.contains("sort") {
            Some("invalid sort order".to_string())
        } else if msg_lower.contains("schema") {
            Some("invalid schema".to_string())
        } else if msg_lower.contains("json") {
            Some("invalid JSON".to_string())
        } else {
            Some("invalid request".to_string())
        }
    } else if msg_lower.contains("missing") {
        Some("missing field".to_string())
    } else if msg_lower.contains("malformed") {
        Some("malformed request".to_string())
    } else if msg_lower.contains("unsupported") {
        Some("unsupported".to_string())
    } else if msg_lower.contains("required") {
        Some("required field".to_string())
    } else {
        let clean_msg = msg
            .lines()
            .next()
            .unwrap_or(msg)
            .trim()
            .chars()
            .take(40)
            .collect::<String>();
        if clean_msg.is_empty() {
            None
        } else {
            Some(clean_msg)
        }
    }
}

// ============================================================================
// Result Formatting
// ============================================================================

/// Format an API result as a human-readable string.
///
/// Examples: "200:OK", "404:NOT_FOUND(Warehouse)", "409:CONFLICT", "500:INTERNAL_ERROR"
pub fn format_result(result: &Result<u16, (u16, String)>) -> String {
    match result {
        Ok(code) => format!("{}:OK", code),
        Err((code, msg)) => {
            if *code == 500 {
                "500:INTERNAL_ERROR".to_string()
            } else if *code == 404 || msg.contains("does not exist") || msg.contains("NoSuch") {
                if let Some(entity) = extract_entity_from_message(msg) {
                    format!("{}:NOT_FOUND({})", code, entity)
                } else {
                    format!("{}:NOT_FOUND", code)
                }
            } else if *code == 409
                || msg.contains("already exists")
                || msg.contains("AlreadyExists")
            {
                format!("{}:CONFLICT", code)
            } else if *code == 400 {
                if let Some(detail) = extract_400_detail(msg) {
                    format!("400:BAD_REQUEST({})", detail)
                } else {
                    "400:BAD_REQUEST".to_string()
                }
            } else {
                format!("{}:ERROR", code)
            }
        }
    }
}

// ============================================================================
// Setup and Cleanup
// ============================================================================

use minio::s3tables::builders::{TableChange, TableIdentifier, TableRequirement};
use minio::s3tables::response_traits::HasTableResult;
use minio::s3tables::types::TablesApi;
use minio::s3tables::utils::ViewSql;

/// Set up resources according to the desired state.
pub async fn setup_state(ctx: &TestContext, state: &ResourceState) -> Result<(), String> {
    if state.warehouse {
        ctx.client
            .create_warehouse(ctx.warehouse())
            .unwrap()
            .build()
            .send()
            .await
            .map_err(|e| format!("Failed to create warehouse: {}", e))?;
    }

    if state.namespace {
        ctx.client
            .create_namespace(ctx.warehouse(), ctx.namespace())
            .unwrap()
            .build()
            .send()
            .await
            .map_err(|e| format!("Failed to create namespace: {}", e))?;
    }

    if state.table {
        ctx.client
            .create_table(
                ctx.warehouse(),
                ctx.namespace(),
                ctx.table(),
                minimal_schema(),
            )
            .unwrap()
            .build()
            .send()
            .await
            .map_err(|e| format!("Failed to create table: {}", e))?;
    }

    if state.view {
        let sql = ViewSql::new("SELECT * FROM dummy").unwrap();
        ctx.client
            .create_view(
                ctx.warehouse(),
                ctx.namespace(),
                ctx.view(),
                minimal_schema(),
                sql,
            )
            .unwrap()
            .build()
            .send()
            .await
            .map_err(|e| format!("Failed to create view: {}", e))?;
    }

    Ok(())
}

/// Clean up all test resources (ignores errors).
pub async fn cleanup(ctx: &TestContext) {
    // Clean in reverse dependency order
    if let Ok(b) = ctx
        .client
        .drop_view(ctx.warehouse(), ctx.namespace(), ctx.view())
    {
        let _ = b.build().send().await;
    }
    if let Ok(b) = ctx
        .client
        .delete_table(ctx.warehouse(), ctx.namespace(), ctx.table())
    {
        let _ = b.build().send().await;
    }
    if let Ok(b) = ctx
        .client
        .delete_namespace(ctx.warehouse(), ctx.namespace())
    {
        let _ = b.build().send().await;
    }
    if let Ok(b) = ctx.client.delete_warehouse(ctx.warehouse()) {
        let _ = b.build().send().await;
    }
}

// ============================================================================
// Complex Method Helpers
// ============================================================================

/// Execute register_table operation.
///
/// This is complex because it requires:
/// 1. Creating a source table
/// 2. Loading it to get metadata location
/// 3. Deleting the source from catalog
/// 4. Registering using the metadata location
/// 5. Cleaning up the registered table (to allow repeated calls and namespace deletion)
pub async fn execute_register_table(ctx: &TestContext) -> Result<u16, (u16, String)> {
    // Use unique names to allow multiple calls in the same test context
    let suffix = format!("{:04x}", rand::random::<u16>());
    let src_name: TableName = format!("{}_src_{}", ctx.table_name, suffix)
        .as_str()
        .try_into()
        .unwrap();
    let reg_name: TableName = format!("{}_reg_{}", ctx.table_name, suffix)
        .as_str()
        .try_into()
        .unwrap();

    // Create source table
    if let Err(e) = ctx
        .client
        .create_table(
            ctx.warehouse(),
            ctx.namespace(),
            src_name.clone(),
            minimal_schema(),
        )
        .unwrap()
        .build()
        .send()
        .await
    {
        return Err((extract_status_code(&e), format!("Create source: {}", e)));
    }

    // Load to get metadata location
    let resp = match ctx
        .client
        .load_table(ctx.warehouse(), ctx.namespace(), src_name.clone())
        .unwrap()
        .build()
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => return Err((extract_status_code(&e), format!("Load source: {}", e))),
    };

    let loc = match resp.table_result() {
        Ok(r) => match &r.metadata_location {
            Some(loc) => loc.clone(),
            None => return Err((0, "No metadata location".to_string())),
        },
        Err(e) => return Err((0, format!("Parse result: {}", e))),
    };

    // Delete source table without purge (SDK default: purgeRequested=false) to free the
    // UUID pointer while preserving the metadata files on storage. This follows the standard
    // Iceberg workflow: delete (without purge) then register.
    // See: https://github.com/apache/iceberg/issues/11023
    // See: https://github.com/apache/iceberg/pull/11317
    if let Err(e) = ctx
        .client
        .delete_table(ctx.warehouse(), ctx.namespace(), src_name)
        .unwrap()
        .build()
        .send()
        .await
    {
        return Err((extract_status_code(&e), format!("Delete source: {}", e)));
    }

    // Register a new table using the preserved metadata location
    let result = match ctx
        .client
        .register_table(ctx.warehouse(), ctx.namespace(), reg_name.clone(), loc)
        .unwrap()
        .build()
        .send()
        .await
    {
        Ok(_) => Ok(200),
        Err(e) => Err((extract_status_code(&e), e.to_string())),
    };

    // Clean up registered table
    if result.is_ok() {
        let _ = ctx
            .client
            .delete_table(ctx.warehouse(), ctx.namespace(), reg_name)
            .unwrap()
            .build()
            .send()
            .await;
    }

    result
}

/// Execute register_view operation.
///
/// This is a multi-step operation that requires:
/// 1. Creating a source view to get metadata
/// 2. Loading the view to get its metadata location
/// 3. Registering the view with a new name using the metadata location
/// 4. Cleaning up both source and registered views
pub async fn execute_register_view(ctx: &TestContext) -> Result<u16, (u16, String)> {
    use minio::s3tables::response_traits::HasCachedViewResult;

    // Use unique names to allow multiple calls in the same test context
    let suffix = format!("{:04x}", rand::random::<u16>());
    let src_name: ViewName = format!("{}_src_{}", ctx.view_name, suffix)
        .as_str()
        .try_into()
        .unwrap();
    let reg_name: ViewName = format!("{}_reg_{}", ctx.view_name, suffix)
        .as_str()
        .try_into()
        .unwrap();

    // Create source view
    let sql = minio::s3tables::utils::ViewSql::new("SELECT 1 AS id").unwrap();
    if let Err(e) = ctx
        .client
        .create_view(
            ctx.warehouse(),
            ctx.namespace(),
            src_name.clone(),
            minimal_schema(),
            sql,
        )
        .unwrap()
        .build()
        .send()
        .await
    {
        return Err((
            extract_status_code(&e),
            format!("Create source view: {}", e),
        ));
    }

    // Load to get metadata location
    let resp = match ctx
        .client
        .load_view(ctx.warehouse(), ctx.namespace(), src_name.clone())
        .unwrap()
        .build()
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => return Err((extract_status_code(&e), format!("Load source view: {}", e))),
    };

    let loc = match resp.view_metadata_location() {
        Ok(loc) => loc.to_string(),
        Err(e) => return Err((0, format!("Get metadata location: {}", e))),
    };

    // NOTE: The standard Iceberg workflow is drop (without purge) then register. However,
    // MinIO's drop_view always purges the underlying metadata files, deviating from the
    // Iceberg REST spec where drop should only remove the catalog entry. This means
    // drop_view destroys the metadata files, causing register_view to fail with
    // "metadata file not found". We work around this by registering while the source view
    // still exists, ensuring the metadata files remain on storage.
    let result = match ctx
        .client
        .register_view(ctx.warehouse(), ctx.namespace(), reg_name.clone(), loc)
        .unwrap()
        .build()
        .send()
        .await
    {
        Ok(_) => Ok(200),
        Err(e) => Err((extract_status_code(&e), e.to_string())),
    };

    // Clean up both views to leave namespace in clean state
    let _ = ctx
        .client
        .drop_view(ctx.warehouse(), ctx.namespace(), src_name)
        .unwrap()
        .build()
        .send()
        .await;
    if result.is_ok() {
        let _ = ctx
            .client
            .drop_view(ctx.warehouse(), ctx.namespace(), reg_name)
            .unwrap()
            .build()
            .send()
            .await;
    }

    result
}

/// Execute commit_multi_table_transaction operation.
///
/// This requires loading the table first to get its UUID for the assertion.
pub async fn execute_commit_multi_table_transaction(
    ctx: &TestContext,
) -> Result<u16, (u16, String)> {
    let resp = match ctx
        .client
        .load_table(ctx.warehouse(), ctx.namespace(), ctx.table())
        .unwrap()
        .build()
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => return Err((extract_status_code(&e), format!("Load table: {}", e))),
    };

    let uuid = match resp.table_result() {
        Ok(r) => r.metadata.table_uuid.clone(),
        Err(e) => return Err((0, format!("Parse result: {}", e))),
    };

    let change = TableChange {
        identifier: TableIdentifier {
            namespace: ctx.namespace(),
            name: ctx.table(),
        },
        requirements: vec![TableRequirement::AssertTableUuid { uuid }],
        updates: vec![],
    };

    match ctx
        .client
        .commit_multi_table_transaction(ctx.warehouse(), vec![change])
        .unwrap()
        .build()
        .send()
        .await
    {
        Ok(_) => Ok(204),
        Err(e) => Err((extract_status_code(&e), e.to_string())),
    }
}

// ============================================================================
// CLI Helpers
// ============================================================================

/// Default configuration values.
pub const DEFAULT_ENDPOINT: &str = "http://localhost:9000";
pub const DEFAULT_ACCESS_KEY: &str = "minioadmin";
pub const DEFAULT_SECRET_KEY: &str = "minioadmin";
pub const DEFAULT_OUTPUT_DIR: &str = "results";

/// Generate a unique run prefix for test isolation.
pub fn generate_run_prefix() -> String {
    format!("{:06x}", rand::random::<u32>() & 0xFFFFFF)
}

/// Get current timestamp as a string.
pub fn timestamp() -> String {
    chrono::Utc::now().format("%Y-%m-%d-%H%M%S").to_string()
}

// Required main function for Cargo's example compilation.
// This module is meant to be included by other examples via #[path = ...], not run directly.
fn main() {
    eprintln!("This is a shared module, not a standalone example.");
    eprintln!("Run iceberg_sequence_test instead.");
}
