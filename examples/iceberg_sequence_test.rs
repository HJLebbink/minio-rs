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

//! Iceberg REST API Sequence Test Tool
//!
//! Comprehensive testing tool for the Iceberg REST Catalog API implementation.
//! Tests N-step API operation sequences with all field combinations to find
//! state-dependent bugs and ensure spec compliance.
//!
//! # Test Modes
//!
//! - **Sequential** (default): Operations run one after another (A → B → C)
//! - **Parallel** (`-p`): Operations run concurrently (A || B || C)
//!
//! All tests include field combinations - each method is tested with all
//! combinations of its optional fields (e.g., create_table with/without
//! partition_spec, sort_order, properties).
//!
//! # Usage
//!
//! ```bash
//! # Run 1-step tests (each method with all field combinations)
//! cargo run --release --example iceberg_sequence_test -- -n 1
//!
//! # Run 2-step sequential tests (default)
//! cargo run --release --example iceberg_sequence_test
//!
//! # Run 2-step parallel tests
//! cargo run --release --example iceberg_sequence_test -- -p
//!
//! # Run 3-step tests
//! cargo run --release --example iceberg_sequence_test -- -n 3
//!
//! # Run parallel tests with repeat (for flaky detection)
//! cargo run --release --example iceberg_sequence_test -- -p -r 3
//! ```

#[path = "iceberg_test_common.rs"]
mod common;

use common::{
    DEFAULT_ACCESS_KEY, DEFAULT_ENDPOINT, DEFAULT_OUTPUT_DIR, DEFAULT_SECRET_KEY, Resource,
    ResourceState, TestContext, cleanup, create_client, execute_commit_multi_table_transaction,
    execute_register_table, execute_register_view, extract_status_code, format_result,
    generate_run_prefix, minimal_schema, sample_partition_spec, sample_properties,
    sample_sort_order, setup_state, timestamp,
};
use futures::stream::{self, StreamExt};
use minio::s3tables::builders::{SnapshotMode, TableMaintenanceConfig};
use minio::s3tables::types::{
    EncryptionConfiguration, MaintenanceStatus, MaintenanceType, MetricsConfiguration,
    RecordExpirationConfiguration, ReplicationConfiguration, StorageClass, TablesApi, Tag,
};
use minio::s3tables::utils::{TableName, ViewName, ViewSql};
use std::fs::{self, File};
use std::io::{Write, stdout};
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;
use tokio::sync::Mutex;

fn git_commit_hash() -> String {
    Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

// ============================================================================
// Method Definition
// ============================================================================

/// An API method with its effects on resources.
#[derive(Debug, Clone)]
struct Method {
    /// Method name (e.g., "create_warehouse")
    name: &'static str,
    /// HTTP status code on success
    success_code: u16,
    /// Resources required for this method to succeed
    requires: Vec<Resource>,
    /// Resource created by this method (if any)
    creates: Option<Resource>,
    /// Resource deleted by this method (if any)
    deletes: Option<Resource>,
    /// Optional fields that can be toggled for field combination testing
    optional_fields: &'static [&'static str],
}

impl Method {
    fn new(name: &'static str, success_code: u16) -> Self {
        Self {
            name,
            success_code,
            requires: vec![],
            creates: None,
            deletes: None,
            optional_fields: &[],
        }
    }

    fn requires(mut self, resources: Vec<Resource>) -> Self {
        self.requires = resources;
        self
    }

    fn creates(mut self, resource: Resource) -> Self {
        self.creates = Some(resource);
        self
    }

    fn deletes(mut self, resource: Resource) -> Self {
        self.deletes = Some(resource);
        self
    }

    fn optional_fields(mut self, fields: &'static [&'static str]) -> Self {
        self.optional_fields = fields;
        self
    }
}

/// All testable API methods.
///
/// Returns only methods that are supported by MinIO (filters out unsupported ones).
fn get_methods() -> Vec<Method> {
    use Resource::*;
    let all_methods = vec![
        // ========== Warehouse Core (5) ==========
        Method::new("create_warehouse", 200)
            .creates(Warehouse)
            .optional_fields(&["upgrade_existing"]),
        Method::new("get_warehouse", 200).requires(vec![Warehouse]),
        Method::new("list_warehouses", 200).optional_fields(&["page_size", "page_token"]),
        Method::new("delete_warehouse", 204)
            .requires(vec![Warehouse])
            .deletes(Warehouse),
        Method::new("get_config", 200).requires(vec![Warehouse]),
        // ========== Warehouse Policy/Config (16) ==========
        Method::new("put_warehouse_policy", 204).requires(vec![Warehouse]),
        Method::new("get_warehouse_policy", 200).requires(vec![Warehouse]),
        Method::new("delete_warehouse_policy", 204).requires(vec![Warehouse]),
        Method::new("put_warehouse_encryption", 204).requires(vec![Warehouse]),
        Method::new("get_warehouse_encryption", 200).requires(vec![Warehouse]),
        Method::new("delete_warehouse_encryption", 204).requires(vec![Warehouse]),
        Method::new("put_warehouse_replication", 204).requires(vec![Warehouse]),
        Method::new("get_warehouse_replication", 200).requires(vec![Warehouse]),
        Method::new("delete_warehouse_replication", 204).requires(vec![Warehouse]),
        Method::new("put_warehouse_metrics", 204).requires(vec![Warehouse]),
        Method::new("get_warehouse_metrics", 200).requires(vec![Warehouse]),
        Method::new("delete_warehouse_metrics", 204).requires(vec![Warehouse]),
        Method::new("put_warehouse_maintenance", 204).requires(vec![Warehouse]),
        Method::new("get_warehouse_maintenance", 200).requires(vec![Warehouse]),
        Method::new("put_warehouse_storage_class", 204).requires(vec![Warehouse]),
        Method::new("get_warehouse_storage_class", 200).requires(vec![Warehouse]),
        // ========== Namespace (6) ==========
        Method::new("create_namespace", 200)
            .requires(vec![Warehouse])
            .creates(Namespace)
            .optional_fields(&["properties"]),
        Method::new("get_namespace", 200).requires(vec![Warehouse, Namespace]),
        Method::new("list_namespaces", 200)
            .requires(vec![Warehouse])
            .optional_fields(&["page_size", "page_token"]),
        Method::new("delete_namespace", 204)
            .requires(vec![Warehouse, Namespace])
            .deletes(Namespace),
        Method::new("namespace_exists", 204).requires(vec![Warehouse, Namespace]),
        Method::new("update_namespace_properties", 200).requires(vec![Warehouse, Namespace]),
        // ========== Table Core (12) ==========
        Method::new("create_table", 200)
            .requires(vec![Warehouse, Namespace])
            .creates(Table)
            .optional_fields(&["partition_spec", "sort_order", "properties"]),
        Method::new("load_table", 200)
            .requires(vec![Warehouse, Namespace, Table])
            .optional_fields(&["snapshots"]),
        Method::new("list_tables", 200)
            .requires(vec![Warehouse, Namespace])
            .optional_fields(&["page_size", "page_token"]),
        Method::new("delete_table", 204)
            .requires(vec![Warehouse, Namespace, Table])
            .deletes(Table),
        Method::new("table_exists", 204).requires(vec![Warehouse, Namespace, Table]),
        Method::new("rename_table", 204).requires(vec![Warehouse, Namespace, Table]),
        Method::new("register_table", 200).requires(vec![Warehouse, Namespace]),
        Method::new("commit_table", 200).requires(vec![Warehouse, Namespace, Table]),
        Method::new("commit_multi_table_transaction", 204)
            .requires(vec![Warehouse, Namespace, Table]),
        Method::new("load_table_credentials", 200).requires(vec![Warehouse, Namespace, Table]),
        Method::new("table_metrics", 204).requires(vec![Warehouse, Namespace, Table]),
        // ========== Table Policy/Config (17) ==========
        Method::new("put_table_policy", 204).requires(vec![Warehouse, Namespace, Table]),
        Method::new("get_table_policy", 200).requires(vec![Warehouse, Namespace, Table]),
        Method::new("delete_table_policy", 204).requires(vec![Warehouse, Namespace, Table]),
        Method::new("put_table_encryption", 204).requires(vec![Warehouse, Namespace, Table]),
        Method::new("get_table_encryption", 200).requires(vec![Warehouse, Namespace, Table]),
        Method::new("delete_table_encryption", 204).requires(vec![Warehouse, Namespace, Table]),
        Method::new("put_table_replication", 204).requires(vec![Warehouse, Namespace, Table]),
        Method::new("get_table_replication", 200).requires(vec![Warehouse, Namespace, Table]),
        Method::new("delete_table_replication", 204).requires(vec![Warehouse, Namespace, Table]),
        Method::new("get_table_replication_status", 200)
            .requires(vec![Warehouse, Namespace, Table]),
        Method::new("put_table_maintenance", 204).requires(vec![Warehouse, Namespace, Table]),
        Method::new("get_table_maintenance", 200).requires(vec![Warehouse, Namespace, Table]),
        Method::new("get_table_maintenance_job_status", 200)
            .requires(vec![Warehouse, Namespace, Table]),
        Method::new("get_table_storage_class", 200).requires(vec![Warehouse, Namespace, Table]),
        Method::new("put_table_expiration", 204).requires(vec![Warehouse, Namespace, Table]),
        Method::new("get_table_expiration", 200).requires(vec![Warehouse, Namespace, Table]),
        Method::new("get_table_expiration_job_status", 200)
            .requires(vec![Warehouse, Namespace, Table]),
        // ========== Scan Planning (5) ==========
        Method::new("plan_table_scan", 200).requires(vec![Warehouse, Namespace, Table]),
        Method::new("execute_table_scan", 200).requires(vec![Warehouse, Namespace, Table]),
        Method::new("fetch_scan_tasks", 200).requires(vec![Warehouse, Namespace, Table]),
        Method::new("fetch_planning_result", 200).requires(vec![Warehouse, Namespace, Table]),
        Method::new("cancel_planning", 204).requires(vec![Warehouse, Namespace, Table]),
        // ========== Tagging (3) ==========
        Method::new("tag_resource", 204).requires(vec![Warehouse, Namespace, Table]),
        Method::new("untag_resource", 204).requires(vec![Warehouse, Namespace, Table]),
        Method::new("list_tags_for_resource", 200).requires(vec![Warehouse, Namespace, Table]),
        // ========== View (7) ==========
        Method::new("create_view", 200)
            .requires(vec![Warehouse, Namespace])
            .creates(View)
            .optional_fields(&["properties"]),
        Method::new("load_view", 200).requires(vec![Warehouse, Namespace, View]),
        Method::new("list_views", 200)
            .requires(vec![Warehouse, Namespace])
            .optional_fields(&["page_size", "page_token"]),
        Method::new("drop_view", 204)
            .requires(vec![Warehouse, Namespace, View])
            .deletes(View),
        Method::new("view_exists", 204).requires(vec![Warehouse, Namespace, View]),
        Method::new("rename_view", 204).requires(vec![Warehouse, Namespace, View]),
        Method::new("register_view", 200).requires(vec![Warehouse, Namespace]),
        Method::new("replace_view", 200).requires(vec![Warehouse, Namespace, View]),
    ];

    all_methods
}

// ============================================================================
// Expected Results
// ============================================================================

/// Expected outcome of an API call.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Expected {
    Success(u16),
    NotFound(Resource),
    Conflict,
}

impl std::fmt::Display for Expected {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Expected::Success(code) => write!(f, "{}:OK", code),
            Expected::NotFound(r) => write!(f, "404:NOT_FOUND({})", r),
            Expected::Conflict => write!(f, "409:CONFLICT"),
        }
    }
}

/// Calculate expected result for a method given current state.
fn calculate_expected(method: &Method, state: &ResourceState) -> Expected {
    // Check if all required resources exist
    for required in &method.requires {
        if !state.has(*required) {
            return Expected::NotFound(*required);
        }
    }

    // Check for conflict (creating something that exists)
    if method.creates.is_some_and(|creates| state.has(creates)) {
        return Expected::Conflict;
    }

    // Special cases per Iceberg spec
    if method.name == "delete_warehouse" && state.namespace {
        return Expected::Conflict; // Cannot delete warehouse with namespaces
    }
    if method.name == "delete_namespace" && (state.table || state.view) {
        return Expected::Conflict; // Namespace must be empty
    }

    Expected::Success(method.success_code)
}

/// Apply method effects to state (for sequential execution).
fn apply_effects(state: &mut ResourceState, method: &Method) {
    if let Some(created) = method.creates {
        state.set(created, true);
    }
    if let Some(deleted) = method.deletes {
        state.set(deleted, false);
        // Cascade deletes
        match deleted {
            Resource::Warehouse => {
                state.namespace = false;
                state.table = false;
                state.view = false;
            }
            Resource::Namespace => {
                state.table = false;
                state.view = false;
            }
            _ => {}
        }
    }
}

// ============================================================================
// Test Case
// ============================================================================

/// Field flags for a single step in a test case.
/// Tracks which optional fields should be enabled for that step.
#[derive(Debug, Clone, Default)]
struct FieldFlags {
    /// Field names mapped to whether they are enabled
    enabled: Vec<(&'static str, bool)>,
}

impl FieldFlags {
    /// Create empty field flags (no optional fields).
    fn empty() -> Self {
        Self { enabled: vec![] }
    }

    /// Create field flags from a method and a combination index.
    fn from_method_combination(method: &Method, combo: usize) -> Self {
        let enabled: Vec<_> = method
            .optional_fields
            .iter()
            .enumerate()
            .map(|(i, &f)| (f, (combo >> i) & 1 == 1))
            .collect();
        Self { enabled }
    }

    /// Check if a field is enabled.
    fn is_on(&self, name: &str) -> bool {
        self.enabled.iter().any(|(f, e)| *f == name && *e)
    }

    /// Format as a human-readable string.
    #[allow(dead_code)]
    fn format(&self) -> String {
        if self.enabled.is_empty() {
            "none".to_string()
        } else {
            self.enabled
                .iter()
                .map(|(name, on)| format!("{}={}", name, if *on { "ON" } else { "OFF" }))
                .collect::<Vec<_>>()
                .join(",")
        }
    }
}

/// A sequence test case with N steps.
struct TestCase {
    id: String,
    steps: Vec<Method>,
    /// Field flags for each step (same length as steps)
    field_flags: Vec<FieldFlags>,
    initial_state: ResourceState,
    expected: Vec<Expected>,
}

/// Result of running a test case.
enum TestResult {
    Pass {
        id: String,
        steps: Vec<String>,
        state: String,
        actual: String,
        duration_ms: u64,
    },
    Fail {
        id: String,
        steps: Vec<String>,
        state: String,
        expected: String,
        actual: String,
        curls: Vec<String>,
        /// Full error messages/response bodies for each step that failed
        error_bodies: Vec<String>,
    },
    /// Test passed some runs but failed others (only when --repeat > 1)
    Flaky {
        id: String,
        steps: Vec<String>,
        state: String,
        passed: usize,
        total: usize,
        results: Vec<String>,
        curls: Vec<String>,
        /// Error bodies collected from failed runs
        error_bodies: Vec<String>,
    },
    Skip {
        id: String,
        steps: Vec<String>,
        reason: String,
    },
}

// ============================================================================
// Test Generation
// ============================================================================

/// Generate all test cases for the given configuration.
///
/// All tests include field combinations - each method is tested with all
/// combinations of its optional fields.
fn generate_tests(
    num_steps: usize,
    parallel: bool,
    unsupported: &std::collections::HashSet<&'static str>,
) -> Vec<TestCase> {
    let methods: Vec<_> = get_methods()
        .into_iter()
        .filter(|m| !unsupported.contains(m.name))
        .collect();
    let states = [
        ResourceState::empty(),
        ResourceState::with_warehouse(),
        ResourceState::with_namespace(),
        ResourceState::with_table(),
        ResourceState::with_view(),
    ];

    let mut cases = Vec::new();
    let mut id = 0;

    for state in &states {
        if parallel {
            // Parallel mode: generate method combinations with field combinations
            generate_method_sequences_with_fields(&methods, num_steps, |steps, field_flags| {
                id += 1;
                cases.push(TestCase {
                    id: format!("seq-{:05}", id),
                    steps: steps.to_vec(),
                    field_flags: field_flags.to_vec(),
                    initial_state: state.clone(),
                    expected: vec![Expected::Success(200); num_steps],
                });
            });
        } else {
            // Sequential mode: generate method sequences with field combinations
            generate_method_sequences_with_fields(&methods, num_steps, |steps, field_flags| {
                id += 1;
                let mut sim_state = state.clone();
                let expected: Vec<_> = steps
                    .iter()
                    .map(|step| {
                        let exp = calculate_expected(step, &sim_state);
                        if matches!(exp, Expected::Success(_)) {
                            apply_effects(&mut sim_state, step);
                        }
                        exp
                    })
                    .collect();

                cases.push(TestCase {
                    id: format!("seq-{:05}", id),
                    steps: steps.to_vec(),
                    field_flags: field_flags.to_vec(),
                    initial_state: state.clone(),
                    expected,
                });
            });
        }
    }

    cases
}

/// Generate all method sequences with field combinations.
///
/// For each sequence of methods, generates all combinations of field flags.
fn generate_method_sequences_with_fields<F>(methods: &[Method], num_steps: usize, mut callback: F)
where
    F: FnMut(&[Method], &[FieldFlags]),
{
    // Calculate total method sequences
    let total_sequences = methods.len().pow(num_steps as u32);

    for seq_idx in 0..total_sequences {
        // Decode sequence index into method indices
        let mut steps = Vec::with_capacity(num_steps);
        let mut temp = seq_idx;
        for _ in 0..num_steps {
            steps.push(methods[temp % methods.len()].clone());
            temp /= methods.len();
        }

        // Calculate total field combinations for this sequence
        let field_combo_counts: Vec<usize> = steps
            .iter()
            .map(|m| {
                if m.optional_fields.is_empty() {
                    1
                } else {
                    1 << m.optional_fields.len()
                }
            })
            .collect();

        let total_field_combos: usize = field_combo_counts.iter().product();

        // Generate all field combinations for this sequence
        for field_idx in 0..total_field_combos {
            let mut field_flags = Vec::with_capacity(num_steps);
            let mut temp_field = field_idx;

            for (step_idx, step) in steps.iter().enumerate() {
                let combo = temp_field % field_combo_counts[step_idx];
                temp_field /= field_combo_counts[step_idx];
                field_flags.push(FieldFlags::from_method_combination(step, combo));
            }

            callback(&steps, &field_flags);
        }
    }
}

// ============================================================================
// Method Execution
// ============================================================================

macro_rules! run {
    ($fut:expr, $code:expr) => {
        match $fut.await {
            Ok(_) => Ok($code),
            Err(e) => Err((extract_status_code(&e), e.to_string())),
        }
    };
}

/// Execute a single API method and return the status code.
///
/// The `flags` parameter controls which optional fields are enabled for this call.
async fn execute_method(
    ctx: &TestContext,
    method: &Method,
    flags: &FieldFlags,
) -> Result<u16, (u16, String)> {
    let c = &ctx.client;
    let (wh, ns, tbl) = (ctx.warehouse(), ctx.namespace(), ctx.table());

    match method.name {
        // Warehouse
        "create_warehouse" => {
            let b = c.create_warehouse(wh).unwrap();
            if flags.is_on("upgrade_existing") {
                run!(b.upgrade_existing(true).build().send(), 200)
            } else {
                run!(b.build().send(), 200)
            }
        }
        "get_warehouse" => run!(c.get_warehouse(wh).unwrap().build().send(), 200),
        "list_warehouses" => match (flags.is_on("page_size"), flags.is_on("page_token")) {
            (true, true) => run!(
                c.list_warehouses()
                    .page_size(10.try_into().unwrap())
                    .page_token("test")
                    .build()
                    .send(),
                200
            ),
            (true, false) => run!(
                c.list_warehouses()
                    .page_size(10.try_into().unwrap())
                    .build()
                    .send(),
                200
            ),
            (false, true) => run!(c.list_warehouses().page_token("test").build().send(), 200),
            (false, false) => run!(c.list_warehouses().build().send(), 200),
        },
        "delete_warehouse" => run!(c.delete_warehouse(wh).unwrap().build().send(), 204),
        "get_config" => run!(c.get_config(wh).unwrap().build().send(), 200),

        // Namespace
        "create_namespace" => {
            let b = c.create_namespace(wh, ns).unwrap();
            if flags.is_on("properties") {
                run!(b.properties(sample_properties()).build().send(), 200)
            } else {
                run!(b.build().send(), 200)
            }
        }
        "get_namespace" => run!(c.get_namespace(wh, ns).unwrap().build().send(), 200),
        "list_namespaces" => match (flags.is_on("page_size"), flags.is_on("page_token")) {
            (true, true) => run!(
                c.list_namespaces(wh)
                    .unwrap()
                    .page_size(10.try_into().unwrap())
                    .page_token("test")
                    .build()
                    .send(),
                200
            ),
            (true, false) => run!(
                c.list_namespaces(wh)
                    .unwrap()
                    .page_size(10.try_into().unwrap())
                    .build()
                    .send(),
                200
            ),
            (false, true) => run!(
                c.list_namespaces(wh)
                    .unwrap()
                    .page_token("test")
                    .build()
                    .send(),
                200
            ),
            (false, false) => run!(c.list_namespaces(wh).unwrap().build().send(), 200),
        },
        "delete_namespace" => run!(c.delete_namespace(wh, ns).unwrap().build().send(), 204),
        "namespace_exists" => match c.namespace_exists(wh, ns).unwrap().build().send().await {
            Ok(r) if r.exists() => Ok(204),
            Ok(_) => Err((404, "Namespace does not exist".into())),
            Err(e) => Err((extract_status_code(&e), e.to_string())),
        },
        "update_namespace_properties" => {
            let r = c
                .update_namespace_properties(wh, ns)
                .map_err(|e| (400, e.to_string()))?
                .updates(sample_properties())
                .build()
                .map_err(|e| (400, e.to_string()))?;
            run!(r.send(), 200)
        }

        // Table
        "create_table" => {
            // Type-state builder requires matching all combinations
            match (
                flags.is_on("partition_spec"),
                flags.is_on("sort_order"),
                flags.is_on("properties"),
            ) {
                (true, true, true) => run!(
                    c.create_table(wh, ns, tbl, minimal_schema())
                        .unwrap()
                        .partition_spec(sample_partition_spec())
                        .sort_order(sample_sort_order())
                        .properties(sample_properties())
                        .build()
                        .send(),
                    200
                ),
                (true, true, false) => run!(
                    c.create_table(wh, ns, tbl, minimal_schema())
                        .unwrap()
                        .partition_spec(sample_partition_spec())
                        .sort_order(sample_sort_order())
                        .build()
                        .send(),
                    200
                ),
                (true, false, true) => run!(
                    c.create_table(wh, ns, tbl, minimal_schema())
                        .unwrap()
                        .partition_spec(sample_partition_spec())
                        .properties(sample_properties())
                        .build()
                        .send(),
                    200
                ),
                (true, false, false) => run!(
                    c.create_table(wh, ns, tbl, minimal_schema())
                        .unwrap()
                        .partition_spec(sample_partition_spec())
                        .build()
                        .send(),
                    200
                ),
                (false, true, true) => run!(
                    c.create_table(wh, ns, tbl, minimal_schema())
                        .unwrap()
                        .sort_order(sample_sort_order())
                        .properties(sample_properties())
                        .build()
                        .send(),
                    200
                ),
                (false, true, false) => run!(
                    c.create_table(wh, ns, tbl, minimal_schema())
                        .unwrap()
                        .sort_order(sample_sort_order())
                        .build()
                        .send(),
                    200
                ),
                (false, false, true) => run!(
                    c.create_table(wh, ns, tbl, minimal_schema())
                        .unwrap()
                        .properties(sample_properties())
                        .build()
                        .send(),
                    200
                ),
                (false, false, false) => run!(
                    c.create_table(wh, ns, tbl, minimal_schema())
                        .unwrap()
                        .build()
                        .send(),
                    200
                ),
            }
        }
        "load_table" => {
            let b = c.load_table(wh, ns, tbl).unwrap();
            if flags.is_on("snapshots") {
                run!(b.snapshots(SnapshotMode::All).build().send(), 200)
            } else {
                run!(b.build().send(), 200)
            }
        }
        "list_tables" => match (flags.is_on("page_size"), flags.is_on("page_token")) {
            (true, true) => run!(
                c.list_tables(wh, ns)
                    .unwrap()
                    .page_size(10.try_into().unwrap())
                    .page_token("test")
                    .build()
                    .send(),
                200
            ),
            (true, false) => run!(
                c.list_tables(wh, ns)
                    .unwrap()
                    .page_size(10.try_into().unwrap())
                    .build()
                    .send(),
                200
            ),
            (false, true) => run!(
                c.list_tables(wh, ns)
                    .unwrap()
                    .page_token("test")
                    .build()
                    .send(),
                200
            ),
            (false, false) => run!(c.list_tables(wh, ns).unwrap().build().send(), 200),
        },
        "delete_table" => run!(c.delete_table(wh, ns, tbl).unwrap().build().send(), 204),
        "table_exists" => match c.table_exists(wh, ns, tbl).unwrap().build().send().await {
            Ok(r) if r.exists() => Ok(204),
            Ok(_) => Err((404, "Table does not exist".into())),
            Err(e) => Err((extract_status_code(&e), e.to_string())),
        },
        "rename_table" => {
            let new = TableName::try_from(format!("{}_renamed", ctx.table_name).as_str()).unwrap();
            match c
                .rename_table(wh.clone(), ns.clone(), tbl.clone(), ns.clone(), new.clone())
                .unwrap()
                .build()
                .send()
                .await
            {
                Ok(_) => {
                    match c
                        .rename_table(wh, ns.clone(), new, ns, tbl)
                        .unwrap()
                        .build()
                        .send()
                        .await
                    {
                        Ok(_) => Ok(204),
                        Err(e) => Err((extract_status_code(&e), format!("Rename back: {}", e))),
                    }
                }
                Err(e) => Err((extract_status_code(&e), e.to_string())),
            }
        }
        "register_table" => execute_register_table(ctx).await,
        "commit_table" => run!(c.commit_table(wh, ns, tbl).unwrap().build().send(), 200),
        "commit_multi_table_transaction" => execute_commit_multi_table_transaction(ctx).await,
        "load_table_credentials" => run!(
            c.load_table_credentials(wh, ns, tbl)
                .unwrap()
                .build()
                .send(),
            200
        ),
        "table_metrics" => run!(c.table_metrics(wh, ns, tbl).unwrap().build().send(), 204),

        // Warehouse Policy/Config
        "put_warehouse_policy" => run!(
            c.put_warehouse_policy(wh, r#"{"Version":"2012-10-17","Statement":[]}"#)
                .unwrap()
                .build()
                .send(),
            204
        ),
        "get_warehouse_policy" => run!(c.get_warehouse_policy(wh).unwrap().build().send(), 200),
        "delete_warehouse_policy" => {
            run!(c.delete_warehouse_policy(wh).unwrap().build().send(), 204)
        }
        "put_warehouse_encryption" => run!(
            c.put_warehouse_encryption(wh, EncryptionConfiguration::s3_managed())
                .unwrap()
                .build()
                .send(),
            204
        ),
        "get_warehouse_encryption" => {
            run!(c.get_warehouse_encryption(wh).unwrap().build().send(), 200)
        }
        "delete_warehouse_encryption" => run!(
            c.delete_warehouse_encryption(wh).unwrap().build().send(),
            204
        ),
        "put_warehouse_replication" => run!(
            c.put_warehouse_replication(wh, ReplicationConfiguration::new(vec![]))
                .unwrap()
                .build()
                .send(),
            204
        ),
        "get_warehouse_replication" => {
            run!(c.get_warehouse_replication(wh).unwrap().build().send(), 200)
        }
        "delete_warehouse_replication" => run!(
            c.delete_warehouse_replication(wh).unwrap().build().send(),
            204
        ),
        "put_warehouse_metrics" => run!(
            c.put_warehouse_metrics(wh, MetricsConfiguration::enabled())
                .unwrap()
                .build()
                .send(),
            204
        ),
        "get_warehouse_metrics" => run!(c.get_warehouse_metrics(wh).unwrap().build().send(), 200),
        "delete_warehouse_metrics" => {
            run!(c.delete_warehouse_metrics(wh).unwrap().build().send(), 204)
        }
        "put_warehouse_maintenance" => run!(
            c.put_warehouse_maintenance(wh, MaintenanceStatus::Enabled, None)
                .unwrap()
                .build()
                .send(),
            204
        ),
        "get_warehouse_maintenance" => {
            run!(c.get_warehouse_maintenance(wh).unwrap().build().send(), 200)
        }
        "put_warehouse_storage_class" => run!(
            c.put_warehouse_storage_class(wh, StorageClass::Standard)
                .unwrap()
                .build()
                .send(),
            204
        ),
        "get_warehouse_storage_class" => run!(
            c.get_warehouse_storage_class(wh).unwrap().build().send(),
            200
        ),

        // Table Policy/Config
        "put_table_policy" => run!(
            c.put_table_policy(wh, ns, tbl, r#"{"Version":"2012-10-17","Statement":[]}"#)
                .unwrap()
                .build()
                .send(),
            204
        ),
        "get_table_policy" => run!(c.get_table_policy(wh, ns, tbl).unwrap().build().send(), 200),
        "delete_table_policy" => run!(
            c.delete_table_policy(wh, ns, tbl).unwrap().build().send(),
            204
        ),
        "put_table_encryption" => run!(
            c.put_table_encryption(wh, ns, tbl, EncryptionConfiguration::s3_managed())
                .unwrap()
                .build()
                .send(),
            204
        ),
        "get_table_encryption" => run!(
            c.get_table_encryption(wh, ns, tbl).unwrap().build().send(),
            200
        ),
        "delete_table_encryption" => run!(
            c.delete_table_encryption(wh, ns, tbl)
                .unwrap()
                .build()
                .send(),
            204
        ),
        "put_table_replication" => run!(
            c.put_table_replication(wh, ns, tbl, ReplicationConfiguration::new(vec![]))
                .unwrap()
                .build()
                .send(),
            204
        ),
        "get_table_replication" => run!(
            c.get_table_replication(wh, ns, tbl).unwrap().build().send(),
            200
        ),
        "delete_table_replication" => run!(
            c.delete_table_replication(wh, ns, tbl)
                .unwrap()
                .build()
                .send(),
            204
        ),
        "get_table_replication_status" => run!(
            c.get_table_replication_status(wh, ns, tbl)
                .unwrap()
                .build()
                .send(),
            200
        ),
        "put_table_maintenance" => run!(
            c.put_table_maintenance(wh, ns, tbl, TableMaintenanceConfig::compaction_disabled())
                .unwrap()
                .build()
                .send(),
            204
        ),
        "get_table_maintenance" => run!(
            c.get_table_maintenance(wh, ns, tbl).unwrap().build().send(),
            200
        ),
        "get_table_maintenance_job_status" => run!(
            c.get_table_maintenance_job_status(wh, ns, tbl, MaintenanceType::IcebergCompaction)
                .unwrap()
                .build()
                .send(),
            200
        ),
        "get_table_storage_class" => run!(
            c.get_table_storage_class(wh, ns, tbl)
                .unwrap()
                .build()
                .send(),
            200
        ),
        "put_table_expiration" => run!(
            c.put_table_expiration(
                wh,
                ns,
                tbl,
                RecordExpirationConfiguration::enabled("timestamp")
            )
            .unwrap()
            .build()
            .send(),
            204
        ),
        "get_table_expiration" => run!(
            c.get_table_expiration(wh, ns, tbl).unwrap().build().send(),
            200
        ),
        "get_table_expiration_job_status" => run!(
            c.get_table_expiration_job_status(wh, ns, tbl)
                .unwrap()
                .build()
                .send(),
            200
        ),

        // Scan Planning
        "plan_table_scan" => run!(c.plan_table_scan(wh, ns, tbl).unwrap().build().send(), 200),
        "execute_table_scan" => run!(
            c.execute_table_scan(wh, ns, tbl).unwrap().build().send(),
            200
        ),
        "fetch_scan_tasks" => run!(
            c.fetch_scan_tasks(wh, ns, tbl, serde_json::json!({"plan-id": "test"}))
                .unwrap()
                .build()
                .send(),
            200
        ),
        "fetch_planning_result" => run!(
            c.fetch_planning_result(wh, ns, tbl, "plan-id")
                .unwrap()
                .build()
                .send(),
            200
        ),
        "cancel_planning" => run!(
            c.cancel_planning(wh, ns, tbl, "plan-id")
                .unwrap()
                .build()
                .send(),
            204
        ),

        // Tagging
        "tag_resource" => {
            let arn = format!(
                "arn:aws:s3tables:us-east-1:123456789012:bucket/{}/table/{}/{}",
                ctx.warehouse_name, ctx.namespace_name, ctx.table_name
            );
            run!(
                c.tag_resource(&arn, vec![Tag::new("key", "value")])
                    .build()
                    .send(),
                204
            )
        }
        "untag_resource" => {
            let arn = format!(
                "arn:aws:s3tables:us-east-1:123456789012:bucket/{}/table/{}/{}",
                ctx.warehouse_name, ctx.namespace_name, ctx.table_name
            );
            run!(
                c.untag_resource(&arn, vec!["key".to_string()])
                    .build()
                    .send(),
                204
            )
        }
        "list_tags_for_resource" => {
            let arn = format!(
                "arn:aws:s3tables:us-east-1:123456789012:bucket/{}/table/{}/{}",
                ctx.warehouse_name, ctx.namespace_name, ctx.table_name
            );
            run!(c.list_tags_for_resource(&arn).build().send(), 200)
        }

        // View
        "create_view" => {
            let b = c
                .create_view(
                    wh,
                    ns,
                    ctx.view(),
                    minimal_schema(),
                    ViewSql::new("SELECT * FROM dummy").unwrap(),
                )
                .unwrap();
            if flags.is_on("properties") {
                run!(b.properties(sample_properties()).build().send(), 200)
            } else {
                run!(b.build().send(), 200)
            }
        }
        "load_view" => run!(c.load_view(wh, ns, ctx.view()).unwrap().build().send(), 200),
        "list_views" => match (flags.is_on("page_size"), flags.is_on("page_token")) {
            (true, true) => run!(
                c.list_views(wh, ns)
                    .unwrap()
                    .page_size(10.try_into().unwrap())
                    .page_token("test")
                    .build()
                    .send(),
                200
            ),
            (true, false) => run!(
                c.list_views(wh, ns)
                    .unwrap()
                    .page_size(10.try_into().unwrap())
                    .build()
                    .send(),
                200
            ),
            (false, true) => run!(
                c.list_views(wh, ns)
                    .unwrap()
                    .page_token("test")
                    .build()
                    .send(),
                200
            ),
            (false, false) => run!(c.list_views(wh, ns).unwrap().build().send(), 200),
        },
        "drop_view" => run!(c.drop_view(wh, ns, ctx.view()).unwrap().build().send(), 204),
        "view_exists" => match c
            .view_exists(wh, ns, ctx.view())
            .unwrap()
            .build()
            .send()
            .await
        {
            Ok(r) if r.exists() => Ok(204),
            Ok(_) => Err((404, "View does not exist".into())),
            Err(e) => Err((extract_status_code(&e), e.to_string())),
        },
        "rename_view" => {
            let new = ViewName::try_from(format!("{}_renamed", ctx.view_name).as_str()).unwrap();
            match c
                .rename_view(wh.clone(), ns.clone(), ctx.view(), ns.clone(), new.clone())
                .unwrap()
                .build()
                .send()
                .await
            {
                Ok(_) => {
                    match c
                        .rename_view(wh, ns.clone(), new, ns, ctx.view())
                        .unwrap()
                        .build()
                        .send()
                        .await
                    {
                        Ok(_) => Ok(204),
                        Err(e) => Err((extract_status_code(&e), format!("Rename back: {}", e))),
                    }
                }
                Err(e) => Err((extract_status_code(&e), e.to_string())),
            }
        }
        "register_view" => execute_register_view(ctx).await,
        "replace_view" => run!(
            c.replace_view(wh, ns, ctx.view()).unwrap().build().send(),
            200
        ),

        _ => Err((0, format!("Unknown method: {}", method.name))),
    }
}

/// Generate curl command for a method (for debugging).
fn generate_curl(endpoint: &str, ctx: &TestContext, method: &Method) -> String {
    let base = format!("{}/_iceberg/v1", endpoint);
    let ns = &ctx.namespace_name;
    let auth = r#"--aws-sigv4 "aws:amz:us-east-1:s3tables" -u minioadmin:minioadmin"#;

    match method.name {
        "create_warehouse" => format!(
            r#"curl.exe {auth} -X POST "{base}/warehouses" -H "Content-Type: application/json" -d "{{\`"name\`":\`"{}\`"}}""#,
            ctx.warehouse_name
        ),
        "get_warehouse" => format!(r#"curl.exe {auth} -X GET "{base}/{}""#, ctx.warehouse_name),
        "list_warehouses" => format!(r#"curl.exe {auth} -X GET "{base}/warehouses""#),
        "delete_warehouse" => {
            format!(
                r#"curl.exe {auth} -X DELETE "{base}/{}""#,
                ctx.warehouse_name
            )
        }
        "get_config" => format!(
            r#"curl.exe {auth} -X GET "{base}/{}/config""#,
            ctx.warehouse_name
        ),
        "create_namespace" => format!(
            r#"curl.exe {auth} -X POST "{base}/{}/namespaces" -H "Content-Type: application/json" -d "{{\`"namespace\`":[\`"{}\`"]}}""#,
            ctx.warehouse_name, ns
        ),
        "get_namespace" => {
            format!(
                r#"curl.exe {auth} -X GET "{base}/{}/namespaces/{ns}""#,
                ctx.warehouse_name
            )
        }
        "list_namespaces" => {
            format!(
                r#"curl.exe {auth} -X GET "{base}/{}/namespaces""#,
                ctx.warehouse_name
            )
        }
        "delete_namespace" => format!(
            r#"curl.exe {auth} -X DELETE "{base}/{}/namespaces/{ns}""#,
            ctx.warehouse_name
        ),
        "namespace_exists" => {
            format!(
                r#"curl.exe {auth} -I "{base}/{}/namespaces/{ns}""#,
                ctx.warehouse_name
            )
        }
        "update_namespace_properties" => format!(
            r#"curl.exe {auth} -X POST "{base}/{}/namespaces/{ns}/properties" -H "Content-Type: application/json" -d "{{\`"updates\`":{{}},\`"removals\`":[]}}""#,
            ctx.warehouse_name
        ),
        "create_table" => format!(
            r#"curl.exe {auth} -X POST "{base}/{}/namespaces/{ns}/tables" -H "Content-Type: application/json" -d "{{\`"name\`":\`"{}\`",\`"schema\`":{{\`"type\`":\`"struct\`",\`"fields\`":[]}}}}""#,
            ctx.warehouse_name, ctx.table_name
        ),
        "load_table" => format!(
            r#"curl.exe {auth} -X GET "{base}/{}/namespaces/{ns}/tables/{}""#,
            ctx.warehouse_name, ctx.table_name
        ),
        "list_tables" => format!(
            r#"curl.exe {auth} -X GET "{base}/{}/namespaces/{ns}/tables""#,
            ctx.warehouse_name
        ),
        "delete_table" => format!(
            r#"curl.exe {auth} -X DELETE "{base}/{}/namespaces/{ns}/tables/{}""#,
            ctx.warehouse_name, ctx.table_name
        ),
        "table_exists" => format!(
            r#"curl.exe {auth} -I "{base}/{}/namespaces/{ns}/tables/{}""#,
            ctx.warehouse_name, ctx.table_name
        ),
        "rename_table" => format!(
            r#"curl.exe {auth} -X POST "{base}/{}/tables/rename" -H "Content-Type: application/json" -d "{{}}""#,
            ctx.warehouse_name
        ),
        "create_view" => format!(
            r#"curl.exe {auth} -X POST "{base}/{}/namespaces/{ns}/views" -H "Content-Type: application/json" -d "{{\`"name\`":\`"{}\`"}}""#,
            ctx.warehouse_name, ctx.view_name
        ),
        "load_view" => format!(
            r#"curl.exe {auth} -X GET "{base}/{}/namespaces/{ns}/views/{}""#,
            ctx.warehouse_name, ctx.view_name
        ),
        "list_views" => format!(
            r#"curl.exe {auth} -X GET "{base}/{}/namespaces/{ns}/views""#,
            ctx.warehouse_name
        ),
        "drop_view" => format!(
            r#"curl.exe {auth} -X DELETE "{base}/{}/namespaces/{ns}/views/{}""#,
            ctx.warehouse_name, ctx.view_name
        ),
        "view_exists" => format!(
            r#"curl.exe {auth} -I "{base}/{}/namespaces/{ns}/views/{}""#,
            ctx.warehouse_name, ctx.view_name
        ),
        "rename_view" => format!(
            r#"curl.exe {auth} -X POST "{base}/{}/views/rename" -H "Content-Type: application/json" -d "{{}}""#,
            ctx.warehouse_name
        ),
        "register_view" => format!(
            r#"# Multi-step: create view, get metadata location, drop, then register
curl.exe {auth} -X POST "{}/_iceberg/v0/{}/namespaces/{ns}/views/register" -H "Content-Type: application/json" -d "{{\`"name\`":\`"{}\`",\`"metadata-location\`":\`"<location>\`"}}""#,
            endpoint, ctx.warehouse_name, ctx.view_name
        ),
        "register_table" => format!(
            r#"# Multi-step: create table, get metadata location, delete, then register
curl.exe {auth} -X POST "{base}/{}/namespaces/{ns}/register" -H "Content-Type: application/json" -d "{{\`"name\`":\`"{}\`",\`"metadata-location\`":\`"<location>\`"}}""#,
            ctx.warehouse_name, ctx.table_name
        ),
        "commit_table" => format!(
            r#"curl.exe {auth} -X POST "{base}/{}/namespaces/{ns}/tables/{}/commit" -H "Content-Type: application/json" -d "{{\`"requirements\`":[],\`"updates\`":[]}}""#,
            ctx.warehouse_name, ctx.table_name
        ),
        "commit_multi_table_transaction" => format!(
            r#"curl.exe {auth} -X POST "{base}/{}/transactions/commit" -H "Content-Type: application/json" -d "{{\`"table-changes\`":[]}}""#,
            ctx.warehouse_name
        ),
        "replace_view" => format!(
            r#"curl.exe {auth} -X POST "{base}/{}/namespaces/{ns}/views/{}" -H "Content-Type: application/json" -d "{{}}""#,
            ctx.warehouse_name, ctx.view_name
        ),
        "report_metrics" => format!(
            r#"curl.exe {auth} -X POST "{base}/{}/namespaces/{ns}/tables/{}/metrics" -H "Content-Type: application/json" -d "{{}}""#,
            ctx.warehouse_name, ctx.table_name
        ),
        _ => "# Unknown".to_string(),
    }
}

// ============================================================================
// Test Execution
// ============================================================================

/// Run a test case sequentially (A → B → C).
async fn run_sequential(ctx: &TestContext, test: &TestCase, endpoint: &str) -> TestResult {
    let start = Instant::now();
    let steps: Vec<String> = test.steps.iter().map(|s| s.name.to_string()).collect();
    let curls: Vec<String> = test
        .steps
        .iter()
        .map(|s| generate_curl(endpoint, ctx, s))
        .collect();

    if let Err(e) = setup_state(ctx, &test.initial_state).await {
        cleanup(ctx).await;
        return TestResult::Skip {
            id: test.id.clone(),
            steps,
            reason: format!("Setup failed: {}", e),
        };
    }

    let mut results = Vec::new();
    let mut error_bodies = Vec::new();
    let mut first_fail = None;

    for (i, step) in test.steps.iter().enumerate() {
        let flags = &test.field_flags[i];
        let result = execute_method(ctx, step, flags).await;
        let expected = &test.expected[i];
        let matches = match (&result, expected) {
            (Ok(code), Expected::Success(exp)) => code == exp,
            (Err((code, _)), Expected::NotFound(_)) => *code == 404,
            (Err((code, _)), Expected::Conflict) => *code == 409,
            _ => false,
        };

        if !matches {
            if first_fail.is_none() {
                first_fail = Some(i);
            }
            // Capture the full error body for failed steps
            if let Err((_, msg)) = &result {
                error_bodies.push(format!("Step {} ({}): {}", i + 1, step.name, msg));
            }
        }
        results.push(format_result(&result));
    }

    cleanup(ctx).await;

    if first_fail.is_some() {
        TestResult::Fail {
            id: test.id.clone(),
            steps,
            state: test.initial_state.to_short_string(),
            expected: test
                .expected
                .iter()
                .map(|e| e.to_string())
                .collect::<Vec<_>>()
                .join(" -> "),
            actual: results.join(" -> "),
            curls,
            error_bodies,
        }
    } else {
        TestResult::Pass {
            id: test.id.clone(),
            steps,
            state: test.initial_state.to_short_string(),
            actual: results.join(" -> "),
            duration_ms: start.elapsed().as_millis() as u64,
        }
    }
}

/// Run a test case in parallel (A || B || C).
async fn run_parallel(ctx: &TestContext, test: &TestCase, endpoint: &str) -> TestResult {
    let start = Instant::now();
    let steps: Vec<String> = test.steps.iter().map(|s| s.name.to_string()).collect();
    let curls: Vec<String> = test
        .steps
        .iter()
        .map(|s| generate_curl(endpoint, ctx, s))
        .collect();

    if let Err(e) = setup_state(ctx, &test.initial_state).await {
        cleanup(ctx).await;
        return TestResult::Skip {
            id: test.id.clone(),
            steps,
            reason: format!("Setup failed: {}", e),
        };
    }

    let futures: Vec<_> = test
        .steps
        .iter()
        .zip(test.field_flags.iter())
        .map(|(s, f)| execute_method(ctx, s, f))
        .collect();
    let results = futures::future::join_all(futures).await;

    cleanup(ctx).await;

    let result_strs: Vec<String> = results.iter().map(format_result).collect();

    // In parallel mode, fail only on 500 errors
    let mut error_bodies = Vec::new();
    for (i, result) in results.iter().enumerate() {
        if let Err((500, msg)) = result {
            error_bodies.push(format!("Step {} ({}): {}", i + 1, test.steps[i].name, msg));
            return TestResult::Fail {
                id: test.id.clone(),
                steps,
                state: test.initial_state.to_short_string(),
                expected: "no 500 errors".to_string(),
                actual: result_strs.join(" || "),
                curls,
                error_bodies,
            };
        }
    }

    TestResult::Pass {
        id: test.id.clone(),
        steps,
        state: test.initial_state.to_short_string(),
        actual: result_strs.join(" || "),
        duration_ms: start.elapsed().as_millis() as u64,
    }
}

/// Run a parallel test case with repetition for flaky detection.
///
/// When `repeat` > 1, the test is run multiple times:
/// - All pass: TestResult::Pass
/// - All fail: TestResult::Fail (returns first failure)
/// - Mixed: TestResult::Flaky
async fn run_parallel_with_repeat(
    ctx: &TestContext,
    test: &TestCase,
    endpoint: &str,
    repeat: usize,
) -> TestResult {
    if repeat <= 1 {
        return run_parallel(ctx, test, endpoint).await;
    }

    let steps: Vec<String> = test.steps.iter().map(|s| s.name.to_string()).collect();
    let curls: Vec<String> = test
        .steps
        .iter()
        .map(|s| generate_curl(endpoint, ctx, s))
        .collect();

    let mut pass_count = 0;
    let mut total_duration_ms = 0u64;
    let mut results_strs: Vec<String> = Vec::with_capacity(repeat);
    let mut all_error_bodies: Vec<String> = Vec::new();
    let mut first_failure: Option<TestResult> = None;

    for run_idx in 0..repeat {
        let result = run_parallel(ctx, test, endpoint).await;
        match &result {
            TestResult::Pass {
                actual,
                duration_ms,
                ..
            } => {
                pass_count += 1;
                total_duration_ms += duration_ms;
                results_strs.push(format!("PASS: {}", actual));
            }
            TestResult::Fail {
                actual,
                error_bodies,
                ..
            } => {
                results_strs.push(format!("FAIL: {}", actual));
                // Collect error bodies with run index prefix
                for body in error_bodies {
                    all_error_bodies.push(format!("Run {}: {}", run_idx + 1, body));
                }
                if first_failure.is_none() {
                    first_failure = Some(result);
                }
            }
            TestResult::Skip { .. } => {
                return result;
            }
            TestResult::Flaky { .. } => {
                results_strs.push("FLAKY".to_string());
            }
        }
    }

    if pass_count == repeat {
        TestResult::Pass {
            id: test.id.clone(),
            steps,
            state: test.initial_state.to_short_string(),
            actual: format!("{}/{} passed", pass_count, repeat),
            duration_ms: total_duration_ms,
        }
    } else if pass_count == 0 {
        // All failed - return the first failure
        first_failure.unwrap()
    } else {
        // Mixed - flaky
        TestResult::Flaky {
            id: test.id.clone(),
            steps,
            state: test.initial_state.to_short_string(),
            passed: pass_count,
            total: repeat,
            results: results_strs,
            curls,
            error_bodies: all_error_bodies,
        }
    }
}

// ============================================================================
// Output Formatting
// ============================================================================

fn format_pass(r: &TestResult, sep: &str) -> String {
    let TestResult::Pass {
        id,
        steps,
        state,
        actual,
        duration_ms,
    } = r
    else {
        return String::new();
    };
    format!(
        "PASS | {} | {} | state={} | {} | {}ms",
        id,
        steps.join(sep),
        state,
        actual,
        duration_ms
    )
}

fn format_fail(r: &TestResult, sep: &str) -> String {
    let TestResult::Fail {
        id,
        steps,
        state,
        expected,
        actual,
        curls,
        error_bodies,
    } = r
    else {
        return String::new();
    };
    let curl_lines: Vec<_> = curls
        .iter()
        .enumerate()
        .map(|(i, c)| format!("  Step {}: {}", i + 1, c))
        .collect();

    let mut output = format!(
        "{} | {} | state={}\n  Expected: {}\n  Actual:   {}\n{}\n",
        id,
        steps.join(sep),
        state,
        expected,
        actual,
        curl_lines.join("\n")
    );

    // Add error response bodies if present
    if !error_bodies.is_empty() {
        output.push_str("  Response Bodies:\n");
        for body in error_bodies {
            output.push_str(&format!("    {}\n", body));
        }
    }

    output
}

fn format_skip(r: &TestResult, sep: &str) -> String {
    let TestResult::Skip { id, steps, reason } = r else {
        return String::new();
    };
    format!("SKIP | {} | {} | {}", id, steps.join(sep), reason)
}

fn format_flaky(r: &TestResult, sep: &str) -> String {
    let TestResult::Flaky {
        id,
        steps,
        state,
        passed,
        total,
        results,
        curls,
        error_bodies,
    } = r
    else {
        return String::new();
    };
    let curl_lines: Vec<_> = curls
        .iter()
        .enumerate()
        .map(|(i, c)| format!("  Step {}: {}", i + 1, c))
        .collect();
    let results_str: Vec<_> = results
        .iter()
        .enumerate()
        .map(|(i, r)| format!("  Run {}: {}", i + 1, r))
        .collect();

    let mut output = format!(
        "{} | {} | state={}\n  Passed: {}/{}\n{}\n{}\n",
        id,
        steps.join(sep),
        state,
        passed,
        total,
        results_str.join("\n"),
        curl_lines.join("\n")
    );

    // Add error response bodies if present
    if !error_bodies.is_empty() {
        output.push_str("  Response Bodies:\n");
        for body in error_bodies {
            output.push_str(&format!("    {}\n", body));
        }
    }

    output
}

// ============================================================================
// CLI
// ============================================================================

struct Config {
    endpoint: String,
    access_key: String,
    secret_key: String,
    steps: usize,
    output_dir: String,
    parallel: bool,
    repeat: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            endpoint: DEFAULT_ENDPOINT.to_string(),
            access_key: DEFAULT_ACCESS_KEY.to_string(),
            secret_key: DEFAULT_SECRET_KEY.to_string(),
            steps: 2,
            output_dir: DEFAULT_OUTPUT_DIR.to_string(),
            parallel: false,
            repeat: 1,
        }
    }
}

fn parse_args() -> Config {
    let mut cfg = Config::default();
    let args: Vec<String> = std::env::args().collect();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-e" | "--endpoint" => {
                i += 1;
                if i < args.len() {
                    cfg.endpoint = args[i].clone();
                }
            }
            "--access-key" => {
                i += 1;
                if i < args.len() {
                    cfg.access_key = args[i].clone();
                }
            }
            "--secret-key" => {
                i += 1;
                if i < args.len() {
                    cfg.secret_key = args[i].clone();
                }
            }
            "-n" | "--steps" => {
                i += 1;
                if i < args.len() {
                    cfg.steps = args[i].parse().unwrap_or(2).clamp(1, 5);
                }
            }
            "-p" | "--parallel" => cfg.parallel = true,
            "-r" | "--repeat" => {
                i += 1;
                if i < args.len() {
                    cfg.repeat = args[i].parse().unwrap_or(1).clamp(1, 10);
                }
            }
            "-o" | "--output" => {
                i += 1;
                if i < args.len() {
                    cfg.output_dir = args[i].clone();
                }
            }
            "-h" | "--help" => {
                print_help();
                std::process::exit(0);
            }
            _ => {}
        }
        i += 1;
    }
    cfg
}

fn print_help() {
    println!(
        r#"Iceberg REST API Sequence Test Tool

Tests N-step API operation sequences with all field combinations to find
state-dependent bugs, race conditions, and ensure spec compliance.

USAGE: iceberg_sequence_test [OPTIONS]

OPTIONS:
    -e, --endpoint <URL>   MinIO endpoint (default: http://localhost:9000)
    --access-key <KEY>     Access key (default: minioadmin)
    --secret-key <KEY>     Secret key (default: minioadmin)
    -n, --steps <N>        Operations per test (1-5, default: 2)
    -p, --parallel         Run operations concurrently
    -r, --repeat <N>       Repeat each test N times (1-10, default: 1)
    -o, --output <DIR>     Output directory (default: results)
    -h, --help             Show help

All tests include field combinations - each method is tested with all
combinations of its optional fields.

NOTE: API probe runs automatically before tests to show implementation status.

EXAMPLES:
    cargo run --example iceberg_sequence_test -- -n 1   # Single method tests
    cargo run --example iceberg_sequence_test           # 2-step sequence tests
    cargo run --example iceberg_sequence_test -- -n 3   # 3-step sequence tests
    cargo run --example iceberg_sequence_test -- -p     # Parallel tests
    cargo run --example iceberg_sequence_test -- -p -r 3 # Flaky detection
"#
    );
}

// ============================================================================
// API Probe
// ============================================================================

/// Probe all API methods to check which are implemented by the server.
/// Returns set of unsupported method names.
///
/// For each method, sets up the required resources, calls the method,
/// and checks if the server returns 501 (Not Implemented).
async fn probe_api(cfg: &Config) -> std::collections::HashSet<&'static str> {
    println!("============================================================");
    println!("API Probe - Checking implementation status");
    println!("============================================================");

    let client = create_client(&cfg.endpoint, &cfg.access_key, &cfg.secret_key);
    let run_prefix = generate_run_prefix();
    let methods = get_methods();

    let mut implemented: Vec<&Method> = Vec::new();
    let mut not_implemented = std::collections::HashSet::new();

    for method in &methods {
        // Create fresh context for each method to avoid state pollution
        // TestContext::new handles name sanitization for each resource type
        let ctx = TestContext::new(&format!("p{}", method.name), &run_prefix, client.clone());

        // Set up required resources based on method requirements
        let required_state = compute_required_state(method);
        if let Err(e) = setup_state(&ctx, &required_state).await {
            // Setup failed - skip this method but don't mark as unsupported
            eprintln!(
                "  ! {} - setup failed: {} (skipping)",
                method.name,
                e.lines().next().unwrap_or(&e)
            );
            cleanup(&ctx).await;
            continue;
        }

        // Execute the method
        let flags = FieldFlags::empty();
        let (code, msg) = match execute_method(&ctx, method, &flags).await {
            Ok(c) => (c, String::new()),
            Err((c, m)) => (c, m.to_lowercase()),
        };

        // Clean up before checking result
        cleanup(&ctx).await;

        // Check if method is implemented
        // 501 = Not Implemented (HTTP standard)
        // 400 with "unsupported" or "not implemented" = server explicitly says not supported
        let is_unsupported = code == 501
            || (code == 400
                && (msg.contains("unsupported")
                    || msg.contains("not implemented")
                    || msg.contains("notimplemented")));

        if is_unsupported {
            not_implemented.insert(method.name);
        } else {
            implemented.push(method);
        }
    }

    // Print results
    let print_ctx = TestContext::new("print", &run_prefix, client);

    println!("\nImplemented ({}):", implemented.len());
    println!("------------------------------------------------------------");
    for method in &implemented {
        // Build optional fields info
        let fields_info = if method.optional_fields.is_empty() {
            String::new()
        } else {
            let num_combos = 1 << method.optional_fields.len();
            format!(" [{}] ({})", method.optional_fields.join(", "), num_combos)
        };

        // Get curl command
        let curl = generate_curl(&cfg.endpoint, &print_ctx, method);

        println!("  + {}{} | {}", method.name, fields_info, curl);
    }

    if !not_implemented.is_empty() {
        println!("\nNot Implemented ({}):", not_implemented.len());
        println!("------------------------------------------------------------");
        for name in not_implemented.iter() {
            println!("  - {}", name);
        }
    }
    println!();

    not_implemented
}

/// Compute the required resource state for a method to be testable.
fn compute_required_state(method: &Method) -> ResourceState {
    let mut state = ResourceState::empty();

    for req in &method.requires {
        state.set(*req, true);
        // Also set dependencies
        match req {
            Resource::Namespace => state.warehouse = true,
            Resource::Table => {
                state.warehouse = true;
                state.namespace = true;
            }
            Resource::View => {
                state.warehouse = true;
                state.namespace = true;
            }
            Resource::Warehouse => {}
        }
    }

    // For create methods, we need the parent resources
    if let Some(creates) = method.creates {
        match creates {
            Resource::Warehouse => {} // No parent needed
            Resource::Namespace => state.warehouse = true,
            Resource::Table => {
                state.warehouse = true;
                state.namespace = true;
            }
            Resource::View => {
                state.warehouse = true;
                state.namespace = true;
            }
        }
    }

    state
}

// ============================================================================
// Main
// ============================================================================

#[tokio::main]
async fn main() {
    let cfg = parse_args();

    // Run probe first to detect unsupported methods
    let unsupported = probe_api(&cfg).await;

    let client = create_client(&cfg.endpoint, &cfg.access_key, &cfg.secret_key);
    let run_prefix = generate_run_prefix();
    let start_time = timestamp();

    let (mode, sep, dir_prefix) = if cfg.parallel {
        ("parallel", " || ", "par")
    } else {
        ("sequence", " -> ", "seq")
    };
    let commit_hash = git_commit_hash();

    println!("============================================================");
    println!("Iceberg REST API Sequence Test");
    println!("============================================================");
    println!("Endpoint: {}", cfg.endpoint);
    println!("Mode:     {} ({} ops {})", mode, cfg.steps, sep.trim());
    if cfg.repeat > 1 {
        println!("Repeat:   {}", cfg.repeat);
    }
    println!("Prefix:   {}", run_prefix);
    println!("Commit:   {}", commit_hash);
    println!("============================================================\n");

    let tests = generate_tests(cfg.steps, cfg.parallel, &unsupported);
    println!("Generated {} test cases\n", tests.len());

    if tests.is_empty() {
        println!("No test cases match criteria.");
        return;
    }

    // Create output directory
    let run_dir = format!("{}/{}-{}", cfg.output_dir, dir_prefix, start_time);
    let _ = fs::create_dir_all(&run_dir);

    let pass_log = Arc::new(Mutex::new(
        File::create(format!("{}/pass.txt", run_dir)).unwrap(),
    ));
    let fail_log = Arc::new(Mutex::new(
        File::create(format!("{}/fail.txt", run_dir)).unwrap(),
    ));
    let flaky_log = Arc::new(Mutex::new(
        File::create(format!("{}/flaky.txt", run_dir)).unwrap(),
    ));
    let skip_log = Arc::new(Mutex::new(
        File::create(format!("{}/skip.txt", run_dir)).unwrap(),
    ));
    let log_503 = Arc::new(Mutex::new(
        File::create(format!("{}/503.txt", run_dir)).unwrap(),
    ));

    println!("Output: {}/", run_dir);
    println!();

    let client = Arc::new(client);
    let endpoint = Arc::new(cfg.endpoint.clone());
    let total = tests.len();
    let completed = Arc::new(AtomicUsize::new(0));
    let is_parallel = cfg.parallel;
    let repeat_count = cfg.repeat;

    let results: Vec<TestResult> = stream::iter(tests.into_iter().enumerate())
        .map(|(idx, test)| {
            let client = client.clone();
            let endpoint = endpoint.clone();
            let completed = completed.clone();
            let pass_log = pass_log.clone();
            let fail_log = fail_log.clone();
            let flaky_log = flaky_log.clone();
            let skip_log = skip_log.clone();
            let log_503 = log_503.clone();
            let run_prefix = run_prefix.clone();

            async move {
                let ctx = TestContext::new(&format!("{:05}", idx), &run_prefix, (*client).clone());
                let result = if is_parallel {
                    run_parallel_with_repeat(&ctx, &test, &endpoint, repeat_count).await
                } else {
                    run_sequential(&ctx, &test, &endpoint).await
                };

                let done = completed.fetch_add(1, Ordering::Relaxed) + 1;
                let sep = if is_parallel { " || " } else { " -> " };

                match &result {
                    TestResult::Pass { actual, .. } => {
                        let mut log = pass_log.lock().await;
                        let _ = writeln!(log, "{}", format_pass(&result, sep));
                        if actual.contains("503") {
                            let mut l = log_503.lock().await;
                            let _ = writeln!(l, "{}", format_pass(&result, sep));
                        }
                    }
                    TestResult::Fail {
                        id,
                        steps,
                        expected,
                        actual,
                        ..
                    } => {
                        println!(
                            "\r[FAIL] {} | {} | expected {} | got {}",
                            id,
                            steps.join(sep),
                            expected,
                            actual
                        );
                        let mut log = fail_log.lock().await;
                        let _ = writeln!(log, "{}", format_fail(&result, sep));
                    }
                    TestResult::Flaky {
                        id,
                        steps,
                        passed,
                        total,
                        ..
                    } => {
                        println!(
                            "\r[FLAKY] {} | {} | passed {}/{}",
                            id,
                            steps.join(sep),
                            passed,
                            total
                        );
                        let mut log = flaky_log.lock().await;
                        let _ = writeln!(log, "{}", format_flaky(&result, sep));
                    }
                    TestResult::Skip { .. } => {
                        let mut log = skip_log.lock().await;
                        let _ = writeln!(log, "{}", format_skip(&result, sep));
                    }
                }

                print!("\rProgress: {}/{}", done, total);
                let _ = stdout().flush();
                result
            }
        })
        .buffer_unordered(1)
        .collect()
        .await;

    println!();

    // Summary
    let passed = results
        .iter()
        .filter(|r| matches!(r, TestResult::Pass { .. }))
        .count();
    let failed = results
        .iter()
        .filter(|r| matches!(r, TestResult::Fail { .. }))
        .count();
    let flaky = results
        .iter()
        .filter(|r| matches!(r, TestResult::Flaky { .. }))
        .count();
    let skipped = results
        .iter()
        .filter(|r| matches!(r, TestResult::Skip { .. }))
        .count();
    let total_tests = passed + failed + flaky + skipped;

    let summary = format!(
        r#"============================================================
Iceberg REST API Sequence Test Results
============================================================
Directory: {}
Commit: {}
Mode: {}
Total: {}
Passed: {} ({:.1}%)
Failed: {}
Flaky: {}
Skipped: {}
============================================================
"#,
        run_dir,
        commit_hash,
        mode,
        total_tests,
        passed,
        if total_tests > 0 {
            passed as f64 / total_tests as f64 * 100.0
        } else {
            0.0
        },
        failed,
        flaky,
        skipped,
    );
    println!("{}", summary);

    let mut summary_file = File::create(format!("{}/summary.txt", run_dir)).unwrap();
    let _ = writeln!(summary_file, "{}", summary);

    if failed > 0 {
        println!("Failed:");
        for r in &results {
            if let TestResult::Fail {
                id,
                steps,
                expected,
                actual,
                ..
            } = r
            {
                println!(
                    "  {} | {} | {} vs {}",
                    id,
                    steps.join(sep),
                    expected,
                    actual
                );
            }
        }
    }

    if flaky > 0 {
        println!("\nFlaky:");
        for r in &results {
            if let TestResult::Flaky {
                id,
                steps,
                passed: p,
                total: t,
                ..
            } = r
            {
                println!("  {} | {} | passed {}/{}", id, steps.join(sep), p, t);
            }
        }
    }

    println!(
        "\nComplete: {} passed, {} failed, {} flaky",
        passed, failed, flaky
    );
}
