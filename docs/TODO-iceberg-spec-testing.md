# TODO: Iceberg REST API Spec Compliance Testing Framework

## Overview

This document outlines the implementation tasks for building a comprehensive spec compliance testing framework for MinIO's S3 Tables (Iceberg) implementation.

**Goal:** Test all REST API methods with all combinations of optional fields, and detect spec violations.

---

## Phase 1: Foundation

### 1.1 Project Structure

- [ ] **Create test framework module**
  - Location: `src/s3tables/spec_tests/` or separate crate `minio-iceberg-spec-tests`
  - Decide: in-tree module vs separate crate

- [ ] **Add dependencies to Cargo.toml**
  ```toml
  [dev-dependencies]
  tokio = { version = "1", features = ["full", "test-util"] }
  futures = "0.3"
  toml = "0.8"
  chrono = "0.4"
  ```

- [ ] **Create module structure**
  ```
  spec_tests/
  ├── mod.rs              # Module root
  ├── api_methods.rs      # API method definitions
  ├── support_matrix.rs   # Support matrix types and parser
  ├── test_generator.rs   # Test case generation
  ├── test_runner.rs      # Test execution engine
  ├── fixtures.rs         # Setup/teardown helpers
  ├── results.rs          # Result types and classification
  └── reports.rs          # Output formatting
  ```

### 1.2 API Method Definitions

- [ ] **Define core data structures**
  ```rust
  struct ApiMethod {
      name: &'static str,
      category: Category,
      http_method: http::Method,
      optional_fields: &'static [OptionalField],
      success_code: u16,  // 200 or 204
      resource_effects: ResourceEffects,
  }

  struct OptionalField {
      name: &'static str,
      field_type: FieldType,
  }

  struct ResourceEffects {
      requires: &'static [ResourceRef],
      creates: Option<ResourceRef>,
      deletes: Option<ResourceRef>,
  }
  ```

- [ ] **Enumerate all 70 API methods with metadata**

  **Warehouse Operations (7):**
  - [ ] create_warehouse: optional=[upgrade_existing], success=200, creates=Warehouse
  - [ ] get_warehouse: optional=[], success=200, requires=Warehouse
  - [ ] list_warehouses: optional=[page_size, page_token], success=200
  - [ ] delete_warehouse: optional=[], success=204, deletes=Warehouse
  - [ ] put_warehouse_policy: optional=[], success=204, requires=Warehouse
  - [ ] get_warehouse_policy: optional=[], success=200, requires=Warehouse
  - [ ] delete_warehouse_policy: optional=[], success=204, requires=Warehouse

  **Namespace Operations (6):**
  - [ ] create_namespace: optional=[properties], success=200, creates=Namespace
  - [ ] get_namespace: optional=[], success=200, requires=Namespace
  - [ ] list_namespaces: optional=[parent, page_size, page_token], success=200
  - [ ] delete_namespace: optional=[], success=204, deletes=Namespace
  - [ ] namespace_exists: optional=[], success=204, requires=Namespace
  - [ ] update_namespace_properties: optional=[], success=200, requires=Namespace

  **Table Core Operations (13):**
  - [ ] create_table: optional=[partition_spec, sort_order, properties, location, stage_create], success=200, creates=Table
  - [ ] load_table: optional=[snapshot_id], success=200, requires=Table
  - [ ] list_tables: optional=[page_size, page_token], success=200, requires=Namespace
  - [ ] table_exists: optional=[], success=204, requires=Table
  - [ ] delete_table: optional=[purge_requested], success=204, deletes=Table
  - [ ] rename_table: optional=[], success=204, requires=Table
  - [ ] register_table: optional=[], success=200, requires=Namespace
  - [ ] commit_table: optional=[requirements, updates, idempotency_key], success=200, requires=Table
  - [ ] load_table_credentials: optional=[], success=200, requires=Table
  - [ ] put_table_policy: optional=[], success=204, requires=Table
  - [ ] get_table_policy: optional=[], success=200, requires=Table
  - [ ] delete_table_policy: optional=[], success=204, requires=Table
  - [ ] commit_multi_table_transaction: optional=[idempotency_key], success=204

  **View Operations (7):**
  - [ ] create_view: optional=[properties], success=200, creates=View
  - [ ] load_view: optional=[], success=200, requires=View
  - [ ] list_views: optional=[page_size, page_token], success=200
  - [ ] view_exists: optional=[], success=204, requires=View
  - [ ] rename_view: optional=[], success=204, requires=View
  - [ ] replace_view: optional=[idempotency_key], success=200, requires=View
  - [ ] drop_view: optional=[], success=204, deletes=View

  **Encryption Operations (6):**
  - [ ] put_warehouse_encryption: optional=[], success=204
  - [ ] get_warehouse_encryption: optional=[], success=200
  - [ ] delete_warehouse_encryption: optional=[], success=204
  - [ ] put_table_encryption: optional=[], success=204
  - [ ] get_table_encryption: optional=[], success=200
  - [ ] delete_table_encryption: optional=[], success=204

  **Replication Operations (7):**
  - [ ] put_warehouse_replication: optional=[], success=204
  - [ ] get_warehouse_replication: optional=[], success=200
  - [ ] delete_warehouse_replication: optional=[], success=204
  - [ ] put_table_replication: optional=[], success=204
  - [ ] get_table_replication: optional=[], success=200
  - [ ] get_table_replication_status: optional=[], success=200
  - [ ] delete_table_replication: optional=[], success=204

  **Maintenance Operations (5):**
  - [ ] put_warehouse_maintenance: optional=[], success=204
  - [ ] get_warehouse_maintenance: optional=[], success=200
  - [ ] put_table_maintenance: optional=[], success=204
  - [ ] get_table_maintenance: optional=[], success=200
  - [ ] get_table_maintenance_job_status: optional=[], success=200

  **Storage Class Operations (3):**
  - [ ] put_warehouse_storage_class: optional=[], success=204
  - [ ] get_warehouse_storage_class: optional=[], success=200
  - [ ] get_table_storage_class: optional=[], success=200

  **Metrics Operations (4):**
  - [ ] put_warehouse_metrics: optional=[], success=204
  - [ ] get_warehouse_metrics: optional=[], success=200
  - [ ] delete_warehouse_metrics: optional=[], success=204
  - [ ] table_metrics: optional=[], success=204

  **Expiration Operations (3):**
  - [ ] put_table_expiration: optional=[], success=204
  - [ ] get_table_expiration: optional=[], success=200
  - [ ] get_table_expiration_job_status: optional=[], success=200

  **Tagging Operations (3):**
  - [ ] tag_resource: optional=[], success=204
  - [ ] untag_resource: optional=[], success=204
  - [ ] list_tags_for_resource: optional=[], success=200

  **Scan Planning Operations (5):**
  - [ ] plan_table_scan: optional=[snapshot_id, select, filter, case_sensitive, use_snapshot_schema, start_snapshot_id, end_snapshot_id], success=200
  - [ ] execute_table_scan: optional=[], success=200
  - [ ] fetch_scan_tasks: optional=[], success=200
  - [ ] fetch_planning_result: optional=[], success=200
  - [ ] cancel_planning: optional=[], success=204

  **Config Operations (1):**
  - [ ] get_config: optional=[], success=200

### 1.3 Support Matrix

- [ ] **Define support matrix schema**
  ```rust
  enum SupportStatus {
      Supported,
      Unsupported,
      Partial { unsupported_fields: Vec<String> },
  }

  struct SupportMatrix {
      version: String,
      minio_version: String,
      methods: HashMap<String, MethodSupport>,
  }

  struct MethodSupport {
      status: SupportStatus,
      fields: HashMap<String, SupportStatus>,
      notes: Option<String>,
  }
  ```

- [ ] **Create initial support-matrix.toml**
  - Start with all methods marked "unknown"
  - Will be populated by auto-discovery run

- [ ] **Implement TOML parser**
  - Parse support-matrix.toml
  - Return SupportMatrix struct

- [ ] **Implement lookup functions**
  - `is_method_supported(method_name) -> SupportStatus`
  - `is_field_supported(method_name, field_name) -> SupportStatus`
  - `should_skip_test(test_case) -> bool`

### 1.4 Test Case Generation

- [ ] **Define test case structure**
  ```rust
  struct TestCase {
      id: String,
      method: &'static ApiMethod,
      combination_id: usize,
      fields_enabled: Vec<(&'static str, bool)>,
  }
  ```

- [ ] **Implement combination generator**
  ```rust
  fn generate_combinations(method: &ApiMethod) -> Vec<TestCase> {
      let n = method.optional_fields.len();
      (0..2_usize.pow(n as u32))
          .map(|combo| /* create TestCase */)
          .collect()
  }
  ```

- [ ] **Implement test case builder for each method category**
  - Build actual SDK request from TestCase
  - Apply enabled/disabled fields

- [ ] **Calculate total test count**
  - Sum of 2^n for each method
  - Log total at startup

### 1.5 Fixtures

- [ ] **Implement resource naming**
  ```rust
  struct TestContext {
      test_id: String,
      client: TablesClient,
      resources: HashMap<ResourceRef, String>,
  }

  impl TestContext {
      fn new(test_id: &str, client: TablesClient) -> Self;
      fn warehouse(&self) -> &str;
      fn namespace(&self) -> &str;
      fn table(&self) -> &str;
  }
  ```

- [ ] **Implement fixture setup**
  ```rust
  async fn setup_warehouse(ctx: &TestContext) -> Result<(), Error>;
  async fn setup_namespace(ctx: &TestContext) -> Result<(), Error>;
  async fn setup_table(ctx: &TestContext) -> Result<(), Error>;
  async fn setup_for_method(ctx: &TestContext, method: &ApiMethod) -> Result<(), Error>;
  ```

- [ ] **Implement fixture teardown**
  ```rust
  async fn teardown(ctx: &TestContext) -> Result<(), Error>;
  ```
  - Delete in reverse order: table → namespace → warehouse
  - Ignore errors (resource might not exist)

- [ ] **Implement minimal valid values**
  - `minimal_schema()` - simplest valid Iceberg schema
  - `minimal_partition_spec()` - simplest partition spec
  - `minimal_sort_order()` - simplest sort order
  - `minimal_properties()` - single key-value pair

### 1.6 Test Execution

- [ ] **Implement single test executor**
  ```rust
  async fn execute_test(ctx: &TestContext, test_case: &TestCase) -> TestResult {
      // 1. Setup required resources
      // 2. Build request from test case
      // 3. Send request
      // 4. Classify result
      // 5. Return TestResult
  }
  ```

- [ ] **Implement request builder dispatcher**
  - Match on method name
  - Call appropriate SDK builder
  - Apply enabled optional fields
  - Return generic Result

- [ ] **Implement result classification**
  ```rust
  fn classify_result<R>(
      result: Result<R, Error>,
      expected_code: u16,
      support_status: SupportStatus,
  ) -> TestResult
  ```

- [ ] **Implement concurrent test runner**
  ```rust
  async fn run_all_tests(
      client: TablesClient,
      test_cases: Vec<TestCase>,
      concurrency: usize,
  ) -> Vec<TestResult>
  ```
  - Use semaphore for concurrency control
  - Use unique test_id per test
  - Collect all results

### 1.7 Results and Reporting

- [ ] **Define result types**
  ```rust
  enum TestResult {
      Pass { duration_ms: u64 },
      Fail {
          expected: String,
          actual: String,
          error: Option<Error>,
          classification: FailureClassification,
      },
      Skipped { reason: String },
  }

  enum FailureClassification {
      SpecViolation,
      Regression,
      UnexpectedError,
  }
  ```

- [ ] **Implement pass log formatter**
  ```
  PASS | method_name | combo=X/Y | field1=ON,field2=OFF | 200 OK | 45ms
  ```

- [ ] **Implement fail log formatter**
  ```
  FAIL | method_name | combo=X/Y | fields...
       | Expected: 200 OK
       | Actual: 400 Bad Request
       | Response: {"error": "..."}
       | Classification: SPEC_VIOLATION
  ```

- [ ] **Implement summary report**
  - Total tests, passed, failed, skipped
  - Breakdown by category
  - List of failures
  - Compliance percentage

- [ ] **Implement JSON output**
  - Machine-readable full results
  - For CI integration

- [ ] **Implement file output**
  - `results/pass-log-YYYY-MM-DD-HHMMSS.txt`
  - `results/fail-log-YYYY-MM-DD-HHMMSS.txt`
  - `results/summary-YYYY-MM-DD-HHMMSS.txt`
  - `results/results-YYYY-MM-DD-HHMMSS.json`

---

## Phase 2: Single Method Tests

### 2.1 Implement Method Executors by Category

Each method needs a specific executor that:
1. Takes a TestCase
2. Builds the SDK request with appropriate fields enabled/disabled
3. Sends the request
4. Returns the result

- [ ] **Warehouse executors**
  - [ ] execute_create_warehouse
  - [ ] execute_get_warehouse
  - [ ] execute_list_warehouses
  - [ ] execute_delete_warehouse
  - [ ] execute_put_warehouse_policy
  - [ ] execute_get_warehouse_policy
  - [ ] execute_delete_warehouse_policy

- [ ] **Namespace executors**
  - [ ] execute_create_namespace
  - [ ] execute_get_namespace
  - [ ] execute_list_namespaces
  - [ ] execute_delete_namespace
  - [ ] execute_namespace_exists
  - [ ] execute_update_namespace_properties

- [ ] **Table executors**
  - [ ] execute_create_table
  - [ ] execute_load_table
  - [ ] execute_list_tables
  - [ ] execute_table_exists
  - [ ] execute_delete_table
  - [ ] execute_rename_table
  - [ ] execute_register_table
  - [ ] execute_commit_table
  - [ ] execute_load_table_credentials
  - [ ] execute_put_table_policy
  - [ ] execute_get_table_policy
  - [ ] execute_delete_table_policy
  - [ ] execute_commit_multi_table_transaction

- [ ] **View executors**
  - [ ] execute_create_view
  - [ ] execute_load_view
  - [ ] execute_list_views
  - [ ] execute_view_exists
  - [ ] execute_rename_view
  - [ ] execute_replace_view
  - [ ] execute_drop_view

- [ ] **Encryption executors** (6 methods)
- [ ] **Replication executors** (7 methods)
- [ ] **Maintenance executors** (5 methods)
- [ ] **Storage class executors** (3 methods)
- [ ] **Metrics executors** (4 methods)
- [ ] **Expiration executors** (3 methods)
- [ ] **Tagging executors** (3 methods)
- [ ] **Scan planning executors** (5 methods)
- [ ] **Config executors** (1 method)

### 2.2 Special Handling for commit_table

The `commit_table` method has complex optional fields:
- `requirements`: Vec<TableRequirement> - 8 variants
- `updates`: Vec<TableUpdate> - 15 variants

- [ ] **Define requirement/update combinations to test**
  - Empty requirements + empty updates
  - Each requirement type individually
  - Each update type individually
  - Common combinations (schema changes, snapshot operations)

- [ ] **Implement commit_table test generator**
  - More sophisticated than simple 2^n
  - Test meaningful combinations of requirements and updates

### 2.3 Run Phase 2 Tests

- [ ] **Create main entry point**
  ```rust
  #[tokio::main]
  async fn main() {
      let config = load_config();
      let client = create_client(&config);
      let support_matrix = load_support_matrix();
      let test_cases = generate_all_test_cases();
      let results = run_all_tests(client, test_cases, config.concurrency).await;
      generate_reports(&results);
  }
  ```

- [ ] **Add CLI arguments**
  - `--endpoint` - MinIO server URL
  - `--access-key` / `--secret-key` - credentials
  - `--concurrency` - parallel test count
  - `--filter` - run subset of tests (e.g., `--filter warehouse`)
  - `--output-dir` - results directory
  - `--discover` - auto-discovery mode (ignore support matrix)

- [ ] **Run against MinIO and collect results**
- [ ] **Update support matrix based on results**

---

## Phase 3: Multi-Call Sequences

### 3.1 Sequence Infrastructure

- [ ] **Define sequence types**
  ```rust
  struct Sequence {
      id: String,
      steps: Vec<SequenceStep>,
      expected_results: Vec<ExpectedResult>,
  }

  struct SequenceStep {
      method: &'static ApiMethod,
      resource_bindings: HashMap<ResourceRef, u8>,  // Which resource slot to use
  }

  enum ExpectedResult {
      Success(u16),
      Error(u16),
  }
  ```

- [ ] **Implement resource state tracker**
  ```rust
  struct ResourceState {
      warehouses: HashSet<u8>,   // Which warehouse slots exist
      namespaces: HashSet<u8>,
      tables: HashSet<u8>,
      views: HashSet<u8>,
  }

  impl ResourceState {
      fn apply(&mut self, method: &ApiMethod, step: &SequenceStep);
      fn can_execute(&self, method: &ApiMethod, step: &SequenceStep) -> bool;
  }
  ```

- [ ] **Implement expected result calculator**
  ```rust
  fn expected_result(method: &ApiMethod, state: &ResourceState) -> ExpectedResult {
      if state.can_execute(method) {
          ExpectedResult::Success(method.success_code)
      } else {
          ExpectedResult::Error(/* 404, 409, etc. based on why */)
      }
  }
  ```

### 3.2 Two-Step Sequences

- [ ] **Generate all two-step sequences**
  ```rust
  fn generate_two_step_sequences() -> Vec<Sequence> {
      let mut sequences = Vec::new();
      for m1 in SUPPORTED_METHODS {
          for m2 in SUPPORTED_METHODS {
              sequences.push(create_sequence(vec![m1, m2]));
          }
      }
      sequences
  }
  ```

- [ ] **Implement two-step executor**
  ```rust
  async fn execute_sequence(ctx: &TestContext, seq: &Sequence) -> SequenceResult {
      let mut state = initial_state();
      let mut results = Vec::new();

      for (step, expected) in seq.steps.iter().zip(&seq.expected_results) {
          let result = execute_step(ctx, step, &state).await;
          results.push(classify_step_result(result, expected));
          state.apply(step.method, step);
      }

      SequenceResult { sequence: seq.clone(), step_results: results }
  }
  ```

- [ ] **Run two-step sequences**
- [ ] **Generate sequence-specific reports**

### 3.3 Three-Step Sequences

- [ ] **Generate three-step sequences**
  - 70^3 = 343,000 combinations
  - Consider filtering to reduce count
  - Or: sample randomly (e.g., 10,000 random sequences)

- [ ] **Implement three-step executor**
  - Same as two-step but with one more iteration

- [ ] **Run three-step sequences**
- [ ] **Estimate runtime and adjust concurrency**

---

## Phase 4: Refinement

### 4.1 Auto-Discovery Mode

- [ ] **Implement discovery runner**
  - Ignore support matrix
  - Run all tests
  - Collect results

- [ ] **Implement support matrix generator**
  ```rust
  fn generate_support_matrix(results: &[TestResult]) -> SupportMatrix {
      // Group results by method
      // If all pass → Supported
      // If all fail with same error → Unsupported
      // If mixed → Partial (identify which fields fail)
  }
  ```

- [ ] **Output generated support matrix**
  - Write to `support-matrix-discovered.toml`
  - Diff against existing matrix to show changes

### 4.2 Regression Detection

- [ ] **Implement baseline storage**
  - Save known-good results
  - Compare new runs against baseline

- [ ] **Implement regression reporter**
  - Identify tests that previously passed but now fail
  - Flag as REGRESSION in output

### 4.3 CI Integration

- [ ] **Add JUnit XML output**
  ```rust
  fn generate_junit_xml(results: &[TestResult]) -> String
  ```

- [ ] **Create GitHub Actions workflow**
  ```yaml
  name: Iceberg Spec Compliance
  on: [push, pull_request]
  jobs:
    compliance-test:
      runs-on: ubuntu-latest
      services:
        minio:
          image: minio/minio
          # ...
      steps:
        - uses: actions/checkout@v4
        - name: Run compliance tests
          run: cargo run --bin iceberg-spec-test
        - name: Upload results
          uses: actions/upload-artifact@v4
  ```

- [ ] **Add badge to README**
  - Compliance percentage
  - Last test date

### 4.4 Documentation

- [ ] **Write usage documentation**
  - How to run tests
  - How to interpret results
  - How to update support matrix

- [ ] **Document each failure type**
  - What SPEC_VIOLATION means
  - What REGRESSION means
  - How to report issues to MinIO

---

## Estimated Test Counts

### Phase 1: Single Method Tests

| Category | Methods | Avg Optional Fields | Est. Tests |
|----------|---------|---------------------|------------|
| Warehouse | 7 | 1 | 14 |
| Namespace | 6 | 1 | 12 |
| Table Core | 13 | 2 | 52 |
| View | 7 | 1 | 14 |
| Encryption | 6 | 0 | 6 |
| Replication | 7 | 0 | 7 |
| Maintenance | 5 | 0 | 5 |
| Storage | 3 | 0 | 3 |
| Metrics | 4 | 0 | 4 |
| Expiration | 3 | 0 | 3 |
| Tagging | 3 | 0 | 3 |
| Scan Planning | 5 | 3 | 40 |
| Config | 1 | 0 | 1 |
| **Total** | **70** | | **~164** |

Plus commit_table combinations: ~50-100 additional tests

**Phase 1 Total: ~250-300 tests**

### Phase 2: Two-Step Sequences

- 70 × 70 = 4,900 sequences
- With support filtering: ~2,000-3,000 sequences

### Phase 3: Three-Step Sequences

- 70^3 = 343,000 sequences
- With sampling: ~10,000 sequences

---

## Priority Order

1. **Must have (MVP):**
   - [ ] API method definitions
   - [ ] Test case generation
   - [ ] Single method executors
   - [ ] Basic pass/fail reporting
   - [ ] Support matrix (manual)

2. **Should have:**
   - [ ] Concurrent execution
   - [ ] Detailed failure logging
   - [ ] JSON output
   - [ ] Two-step sequences

3. **Nice to have:**
   - [ ] Auto-discovery mode
   - [ ] Three-step sequences
   - [ ] CI integration
   - [ ] Regression detection
   - [ ] JUnit XML output

---

## Open Decisions

- [ ] **Crate structure:** In-tree module vs separate crate?
- [ ] **Test ID format:** UUID vs sequential vs timestamp-based?
- [ ] **Concurrency default:** 10? 20? Configurable?
- [ ] **commit_table coverage:** All combinations or curated subset?
- [ ] **Three-step strategy:** Full enumeration or random sampling?
