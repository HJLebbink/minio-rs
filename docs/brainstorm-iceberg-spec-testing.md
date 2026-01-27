# Brainstorm: Iceberg REST API Spec Compliance Testing

## Goal

Create a bug hunting program that tests MinIO's S3 Tables (Iceberg) implementation against the Iceberg REST catalog specifications.

## Scope

**In Scope:**
- 100% REST API-level tests
- All combinations of optional fields being present or omitted
- All API methods (supported and unsupported)
- Track supported features and filter out "known unsupported" errors

**Out of Scope:**
- Value variations (e.g., different namespace strings, unicode, edge cases) - covered by unit tests
- Business logic validation - covered by unit tests
- Invalid requests (missing required fields) - not needed, we test spec compliance not error handling

## Test Phases

### Phase 1: Single API Method Tests

Test every API method with all combinations of optional fields present/absent.

**Example: `create_namespace`**

```
POST /{warehouse}/namespaces

Required fields: namespace
Optional fields: properties
```

| Test | namespace | properties | Expected |
|------|-----------|------------|----------|
| 1 | present | present | 200 OK |
| 2 | present | absent | 200 OK |

(Tests 3 and 4 with namespace absent are invalid requests, not spec compliance tests)

**Example: `create_table`**

```
POST /{warehouse}/namespaces/{namespace}/tables

Required fields: name, schema
Optional fields: partition-spec, write-order, properties, location, stage-create
```

5 optional fields = 2^5 = **32 combinations** to test.

### Phase 2: Two-Step Sequences

Test any combination of two supported API methods in sequence.

- 70 methods × 70 methods = 4,900 pairs
- Looking for unexpected behavior, crashes, or spec violations

### Phase 3: Three-Step Sequences (Stretch Goal)

- 70^3 = 343,000 combinations
- May need smart pruning or sampling

---

## Multi-Call Sequence Design

### Resource References

Each API call either **creates**, **uses**, or **deletes** resources. We use symbolic references:

```rust
enum ResourceRef {
    Warehouse(u8),    // W0, W1, W2...
    Namespace(u8),    // N0, N1, N2...
    Table(u8),        // T0, T1, T2...
    View(u8),         // V0, V1, V2...
}
```

### Method Signatures (Resource Effects)

```rust
struct MethodSignature {
    name: &'static str,
    requires: &'static [ResourceRef],   // Must exist before call
    creates: Option<ResourceRef>,       // Created by this call
    deletes: Option<ResourceRef>,       // Deleted by this call
}

const METHOD_SIGNATURES: &[MethodSignature] = &[
    MethodSignature {
        name: "create_warehouse",
        requires: &[],
        creates: Some(Warehouse(0)),
        deletes: None,
    },
    MethodSignature {
        name: "create_namespace",
        requires: &[Warehouse(0)],
        creates: Some(Namespace(0)),
        deletes: None,
    },
    MethodSignature {
        name: "create_table",
        requires: &[Warehouse(0), Namespace(0)],
        creates: Some(Table(0)),
        deletes: None,
    },
    MethodSignature {
        name: "load_table",
        requires: &[Warehouse(0), Namespace(0), Table(0)],
        creates: None,
        deletes: None,
    },
    MethodSignature {
        name: "delete_table",
        requires: &[Warehouse(0), Namespace(0), Table(0)],
        creates: None,
        deletes: Some(Table(0)),
    },
    // ... etc
];
```

### Sequence Generation

For a 2-step sequence, we track resource state:

```rust
struct SequenceState {
    warehouse_exists: bool,
    namespace_exists: bool,
    table_exists: bool,
    view_exists: bool,
}

fn generate_two_step_sequences() -> Vec<Sequence> {
    let mut sequences = Vec::new();

    for method1 in &SUPPORTED_METHODS {
        for method2 in &SUPPORTED_METHODS {
            // Start with fresh state (only warehouse exists from setup)
            let initial_state = SequenceState {
                warehouse_exists: true,
                namespace_exists: false,
                table_exists: false,
                view_exists: false,
            };

            // Apply method1 effects
            let state_after_1 = apply_effects(initial_state, method1);

            // Check if method2 is valid given state
            let expected_result = if can_execute(method2, &state_after_1) {
                ExpectedResult::Success
            } else {
                ExpectedResult::Error(expected_error_code(method2, &state_after_1))
            };

            sequences.push(Sequence {
                steps: vec![method1.clone(), method2.clone()],
                expected_result,
            });
        }
    }

    sequences
}
```

### Concrete Naming at Runtime

At test execution, symbolic refs become real names:

```rust
struct TestContext {
    test_id: String,
    resources: HashMap<ResourceRef, String>,
}

impl TestContext {
    fn new(test_id: &str) -> Self {
        let mut resources = HashMap::new();
        // Pre-generate all possible resource names for this test
        resources.insert(Warehouse(0), format!("wh-{}", test_id));
        resources.insert(Namespace(0), format!("ns_{}", test_id));
        resources.insert(Namespace(1), format!("ns2_{}", test_id));
        resources.insert(Table(0), format!("tbl_{}", test_id));
        resources.insert(Table(1), format!("tbl2_{}", test_id));
        Self { test_id: test_id.to_string(), resources }
    }

    fn resolve(&self, r: ResourceRef) -> &str {
        &self.resources[&r]
    }
}
```

### Example Sequences

**Sequence: create_namespace → create_table**
```
Setup: warehouse W0 exists
Step 1: create_namespace(W0, N0) → creates N0
Step 2: create_table(W0, N0, T0) → creates T0
Expected: Both succeed (200 OK)
```

**Sequence: delete_table → load_table**
```
Setup: W0, N0, T0 all exist
Step 1: delete_table(W0, N0, T0) → deletes T0
Step 2: load_table(W0, N0, T0) → T0 doesn't exist
Expected: Step 1 succeeds, Step 2 returns 404
```

**Sequence: create_table → create_table (same name)**
```
Setup: W0, N0 exist
Step 1: create_table(W0, N0, T0) → creates T0
Step 2: create_table(W0, N0, T0) → T0 already exists
Expected: Step 1 succeeds, Step 2 returns 409 Conflict
```

**Sequence: commit_table → commit_table (concurrent modification)**
```
Setup: W0, N0, T0 exist
Step 1: commit_table(W0, N0, T0) with update → modifies T0
Step 2: commit_table(W0, N0, T0) with stale requirement → conflict
Expected: Step 1 succeeds, Step 2 returns 409 Conflict
```

### Three-Step Example

**Sequence: create_table → delete_table → create_table (same name)**
```
Setup: W0, N0 exist
Step 1: create_table(W0, N0, T0) → creates T0
Step 2: delete_table(W0, N0, T0) → deletes T0
Step 3: create_table(W0, N0, T0) → creates T0 again
Expected: All three succeed (200 OK)
```

### Sequence Test Output

```
PASS | seq-2-step | create_namespace → create_table | N0,T0 created | 200,200 | 95ms
PASS | seq-2-step | delete_table → load_table | T0 deleted | 200,404 | 67ms
FAIL | seq-2-step | create_table → create_table | duplicate T0 | 200,500 |
     | Expected: 200,409
     | Actual: 200,500 (server error instead of conflict)
     | Classification: SPEC_VIOLATION
```

## API Methods Inventory

70 total methods across these categories:

| Category | Count | Methods |
|----------|-------|---------|
| Warehouse | 7 | create, get, list, delete, policy (put/get/delete) |
| Namespace | 6 | create, get, list, delete, exists, update_properties |
| Table Core | 13 | create, load, list, exists, delete, rename, register, commit, credentials, policy (put/get/delete), multi-table-transaction |
| View | 7 | create, load, list, exists, rename, replace, drop |
| Encryption | 6 | warehouse (put/get/delete), table (put/get/delete) |
| Replication | 7 | warehouse (put/get/delete), table (put/get/delete/status) |
| Maintenance | 5 | warehouse (put/get), table (put/get/job_status) |
| Storage Class | 3 | warehouse (put/get), table (get) |
| Metrics | 4 | warehouse (put/get/delete), table (post) |
| Expiration | 3 | table (put/get/job_status) |
| Tagging | 3 | tag, untag, list_tags |
| Scan Planning | 5 | plan, execute, fetch_tasks, fetch_result, cancel |
| Config | 1 | get_config |

## Support Matrix

### Purpose

Track MinIO's implementation status so we can:
1. Skip tests for unimplemented features (don't report as failures)
2. Detect regressions (previously working feature now fails)
3. Track progress as MinIO adds support
4. Generate compliance percentage per category

### Support Matrix File (TOML)

```toml
# support-matrix.toml
# Updated: 2025-01-14
# MinIO Version: RELEASE.2025-01-10

[warehouse]
create_warehouse = "supported"
get_warehouse = "supported"
list_warehouses = "supported"
delete_warehouse = "supported"
put_warehouse_policy = "unsupported"      # Not implemented
get_warehouse_policy = "unsupported"
delete_warehouse_policy = "unsupported"

[namespace]
create_namespace = "supported"
get_namespace = "supported"
list_namespaces = "supported"
delete_namespace = "supported"
namespace_exists = "supported"
update_namespace_properties = "supported"

[table]
create_table = "supported"
load_table = "supported"
list_tables = "supported"
table_exists = "supported"
delete_table = "supported"
rename_table = "supported"
register_table = "unsupported"            # Not implemented
commit_table = "partial"                  # See [table.commit_table]
commit_multi_table_transaction = "unsupported"

[table.commit_table]
# Partial support details
requirements = "supported"
updates = "supported"
idempotency_key = "unsupported"           # Header ignored

[table.commit_table.requirements]
AssertCreate = "supported"
AssertTableUuid = "supported"
AssertRefSnapshotId = "supported"
AssertLastAssignedFieldId = "supported"
AssertCurrentSchemaId = "supported"
AssertLastAssignedPartitionId = "supported"
AssertDefaultSpecId = "supported"
AssertDefaultSortOrderId = "unsupported"  # Not checked

[table.commit_table.updates]
UpgradeFormatVersion = "supported"
AddSchema = "supported"
SetCurrentSchema = "supported"
AddPartitionSpec = "supported"
SetDefaultSpec = "supported"
AddSortOrder = "unsupported"
SetDefaultSortOrder = "unsupported"
AddSnapshot = "supported"
SetSnapshotRef = "supported"
RemoveSnapshots = "supported"
RemoveSnapshotRef = "supported"
SetLocation = "unsupported"               # Security restriction
SetProperties = "supported"
RemoveProperties = "supported"

[view]
create_view = "unsupported"
load_view = "unsupported"
list_views = "unsupported"
view_exists = "unsupported"
rename_view = "unsupported"
replace_view = "unsupported"
drop_view = "unsupported"

[scan_planning]
plan_table_scan = "unsupported"
execute_table_scan = "unsupported"
fetch_scan_tasks = "unsupported"
fetch_planning_result = "unsupported"
cancel_planning = "unsupported"

# ... etc for all 70 methods
```

### Test Runner Logic

```rust
fn run_test(test_case: &TestCase, support_matrix: &SupportMatrix) -> TestResult {
    let support_status = support_matrix.get_status(test_case);

    match support_status {
        SupportStatus::Unsupported => {
            // Don't even run the test
            return TestResult::Skipped { reason: "Unsupported" };
        }
        SupportStatus::Partial { unsupported_fields } => {
            // Check if this specific combination uses unsupported fields
            if test_case.uses_any(&unsupported_fields) {
                return TestResult::Skipped { reason: "Uses unsupported field" };
            }
        }
        SupportStatus::Supported => {
            // Run the test
        }
    }

    let result = execute_test(test_case).await;

    match (support_status, &result) {
        (Supported, Ok(_)) => TestResult::Pass,
        (Supported, Err(e)) => TestResult::Fail {
            error: e,
            classification: Classification::Regression
        },
        (Partial, Ok(_)) => TestResult::Pass,
        (Partial, Err(e)) => TestResult::Fail {
            error: e,
            classification: Classification::SpecViolation
        },
        _ => unreachable!()
    }
}
```

### Updating the Support Matrix

Two approaches:

**Manual:** Update TOML file when MinIO releases new features

**Auto-discovery:** Run all tests, update matrix based on results
```rust
fn discover_support(results: &[TestResult]) -> SupportMatrix {
    // If a method consistently returns 501 Not Implemented → unsupported
    // If a method works → supported
    // If some combinations work, others don't → partial
}
```

### Compliance Report with Support Matrix

```
============================================================
Iceberg REST API Compliance Report
============================================================
MinIO Version: RELEASE.2025-01-10

Category          | Supported | Partial | Unsupported | Compliance
------------------|-----------|---------|-------------|------------
Warehouse         | 4/7       | 0/7     | 3/7         | 57%
Namespace         | 6/6       | 0/6     | 0/6         | 100%
Table Core        | 10/13     | 1/13    | 2/13        | 77%
View              | 0/7       | 0/7     | 7/7         | 0%
Scan Planning     | 0/5       | 0/5     | 5/5         | 0%
...

Overall: 45/70 methods supported (64%)

Tests run: 523 (skipped 324 unsupported)
Passed: 520
Failed: 3

Failures (spec violations in supported features):
  - commit_table combo 5/8: empty updates rejected
  - ...
============================================================
```

## Expected Response Codes (from Iceberg OpenAPI Spec)

The Iceberg spec defines expected HTTP response codes per operation. Examples:

### createNamespace
| Code | Meaning |
|------|---------|
| 200 | Namespace created successfully |
| 400 | Malformed request |
| 409 | Namespace already exists |
| 5XX | Server error |

### loadTable
| Code | Meaning |
|------|---------|
| 200 | Table metadata returned |
| 404 | Table does not exist |
| 5XX | Server error |

### commitTable
| Code | Meaning |
|------|---------|
| 200 | Commit successful |
| 400 | Validation failed or unknown updates |
| 404 | Table does not exist |
| 409 | Requirements failed (optimistic concurrency conflict) |
| 5XX | Server error (commit state unknown) |

### deleteTable
| Code | Meaning |
|------|---------|
| 204 | Table dropped successfully |
| 404 | Table does not exist |
| 5XX | Server error |

### Common codes across all endpoints
| Code | Meaning |
|------|---------|
| 401 | Authentication credentials missing/invalid |
| 403 | Permission denied |
| 406 | Operation not supported |
| 419 | Auth token expired |
| 503 | Service unavailable |

### Implication for Testing

The spec tells us **exactly** what to expect:

```rust
fn expected_code_for_sequence(method: &str, resource_state: &State) -> u16 {
    match (method, resource_state) {
        ("load_table", State { table_exists: false, .. }) => 404,
        ("load_table", State { table_exists: true, .. }) => 200,
        ("create_namespace", State { namespace_exists: true, .. }) => 409,
        ("create_namespace", State { namespace_exists: false, .. }) => 200,
        ("delete_table", State { table_exists: false, .. }) => 404,
        ("delete_table", State { table_exists: true, .. }) => 204,
        // ... etc
    }
}
```

We can parse the OpenAPI spec to build this mapping automatically rather than hand-coding it.

## Result Classification

| Result | Meaning |
|--------|---------|
| PASS | Response matches Iceberg spec |
| EXPECTED_UNSUPPORTED | Method/field not implemented (in support matrix) |
| SPEC_VIOLATION | MinIO behavior contradicts Iceberg spec |
| REGRESSION | Previously working, now broken |

## SDK Suitability

The SDK is well-suited for this testing approach:

- **Required fields:** SDK enforces at compile time - this is fine, we don't test invalid requests
- **Optional fields:** SDK uses `Option<T>` with `#[builder(default)]` - we can include or omit any optional field
- **No changes needed:** The SDK already supports all combinations we need to test

### HTTP 204 Handling

Many DELETE operations return HTTP 204 (No Content) with an empty body. The SDK handles this correctly:

1. Response is received and struct is created successfully
2. `body` field contains empty `Bytes`
3. If you call `cached_body()` it will error (can't parse empty JSON)
4. **Success is indicated by the response returning `Ok(DeleteTableResponse)`**

**Operations returning 204:**
- delete_table, delete_namespace, delete_warehouse
- delete_*_policy, delete_*_encryption, delete_*_replication
- put_*_policy, put_*_encryption (some PUT operations)
- rename_table, rename_view
- drop_view
- tag_resource, untag_resource
- commit_multi_table_transaction

**Test framework implication:**

```rust
fn verify_response<R>(result: Result<R, Error>, expected_code: u16) -> TestResult {
    match (result, expected_code) {
        (Ok(_), 200) | (Ok(_), 204) => TestResult::Pass,
        (Ok(_), _) => TestResult::Fail { /* unexpected success */ },
        (Err(e), code) if error_matches_code(&e, code) => TestResult::Pass,
        (Err(e), _) => TestResult::Fail { error: e },
    }
}
```

The SDK returns `Ok(Response)` for both 200 and 204 - we just need to know which to expect per operation.

## Architecture Components

```
┌─────────────────────────────────────────────────────────────┐
│                    Test Framework                           │
├─────────────────────────────────────────────────────────────┤
│  1. API Spec Model                                          │
│     - All endpoints with required/optional fields           │
│     - Expected response codes per combination               │
├─────────────────────────────────────────────────────────────┤
│  2. Support Matrix                                          │
│     - MinIO implementation status per method/field          │
│     - Known bugs with issue references                      │
├─────────────────────────────────────────────────────────────┤
│  3. Test Generator                                          │
│     - Generate 2^n combinations for n optional fields       │
│     - Generate method pairs/triples for sequence tests      │
├─────────────────────────────────────────────────────────────┤
│  4. Test Executor                                           │
│     - Setup fixtures (warehouse, namespace, table)          │
│     - Execute REST API calls                                │
│     - Capture responses                                     │
├─────────────────────────────────────────────────────────────┤
│  5. Result Classifier                                       │
│     - Compare response to spec expectations                 │
│     - Filter known unsupported features                     │
│     - Generate report                                       │
└─────────────────────────────────────────────────────────────┘
```

## API Enumerator Design

### Option A: Declarative Definition

Define each API method with its optional fields in a data structure:

```rust
struct ApiMethod {
    name: &'static str,
    category: &'static str,
    optional_fields: &'static [&'static str],
}

const API_METHODS: &[ApiMethod] = &[
    ApiMethod {
        name: "create_namespace",
        category: "namespace",
        optional_fields: &["properties"],
    },
    ApiMethod {
        name: "create_table",
        category: "table",
        optional_fields: &[
            "partition_spec",
            "sort_order",
            "properties",
            "location",
            "stage_create",
        ],
    },
    ApiMethod {
        name: "commit_table",
        category: "table",
        optional_fields: &[
            "requirements",  // can be empty vec or populated
            "updates",       // can be empty vec or populated
            "idempotency_key",
        ],
    },
    // ... all 70 methods
];
```

### Option B: Macro-based from SDK

Use a proc macro to extract optional fields from the SDK builder types:

```rust
#[derive(TestableApi)]
struct CreateTable { ... }  // Macro reads Option<T> fields
```

**Recommendation:** Option A is simpler and more explicit. We manually enumerate once, and it serves as documentation.

---

## Test Examples

### Example 1: `create_namespace` (1 optional field = 2 tests)

```rust
#[tokio::test]
async fn test_create_namespace_with_properties() {
    let client = setup_client().await;
    let warehouse = create_test_warehouse(&client).await;

    let result = client
        .create_namespace(&warehouse, Namespace::single("test")?)
        .properties(HashMap::from([("key".into(), "value".into())]))  // PRESENT
        .build()
        .send()
        .await;

    assert_matches_spec!(result, 200);
}

#[tokio::test]
async fn test_create_namespace_without_properties() {
    let client = setup_client().await;
    let warehouse = create_test_warehouse(&client).await;

    let result = client
        .create_namespace(&warehouse, Namespace::single("test")?)
        // properties ABSENT - not called
        .build()
        .send()
        .await;

    assert_matches_spec!(result, 200);
}
```

### Example 2: `create_table` (5 optional fields = 32 tests)

Generated test for combination `[partition_spec=ON, sort_order=OFF, properties=ON, location=OFF, stage_create=OFF]`:

```rust
#[tokio::test]
async fn test_create_table_combo_10100() {
    let client = setup_client().await;
    let (warehouse, namespace) = setup_namespace(&client).await;

    let schema = minimal_schema();  // Fixed valid schema for all tests

    let mut builder = client
        .create_table(&warehouse, &namespace, "test_table", schema)?;

    // Combination 10100 = partition_spec ON, sort_order OFF, properties ON, location OFF, stage_create OFF
    builder = builder.partition_spec(minimal_partition_spec());  // ON
    // sort_order: not called (OFF)
    builder = builder.properties(HashMap::from([("k".into(), "v".into())]));  // ON
    // location: not called (OFF)
    // stage_create: not called (OFF)

    let result = builder.build().send().await;

    assert_matches_spec!(result, 200);
}
```

### Example 3: `commit_table` with requirements/updates combinations

```rust
#[tokio::test]
async fn test_commit_table_empty_requirements_empty_updates() {
    let client = setup_client().await;
    let (warehouse, namespace, table) = setup_table(&client).await;

    let result = client
        .commit_table(&warehouse, &namespace, &table)?
        .requirements(vec![])  // Empty
        .updates(vec![])       // Empty
        .build()
        .send()
        .await;

    // Should this succeed or fail? That's what we're testing.
    record_result!(result);
}

#[tokio::test]
async fn test_commit_table_with_uuid_requirement_no_updates() {
    let client = setup_client().await;
    let (warehouse, namespace, table, metadata) = setup_table_with_metadata(&client).await;

    let result = client
        .commit_table(&warehouse, &namespace, &table)?
        .requirements(vec![TableRequirement::AssertTableUuid {
            uuid: metadata.table_uuid.clone()
        }])
        .updates(vec![])
        .build()
        .send()
        .await;

    record_result!(result);
}
```

---

## Test Generation

Rather than hand-writing each test, generate them:

```rust
fn generate_tests_for_method(method: &ApiMethod) -> Vec<TestCase> {
    let n = method.optional_fields.len();
    let combinations = 2_usize.pow(n as u32);

    (0..combinations)
        .map(|combo| {
            let fields_enabled: Vec<bool> = (0..n)
                .map(|i| (combo >> i) & 1 == 1)
                .collect();

            TestCase {
                method: method.name,
                combination_id: combo,
                fields: method.optional_fields
                    .iter()
                    .zip(fields_enabled)
                    .map(|(name, enabled)| (*name, enabled))
                    .collect(),
            }
        })
        .collect()
}
```

---

## Concurrency Strategy

### Sequential Execution (Simple)
- Run one test at a time
- Reuse same warehouse/namespace
- Slow but no interference

### Concurrent Execution (Fast)
- Run tests in parallel
- Each test gets unique resource names:

```rust
async fn setup_isolated_resources(test_id: &str) -> TestFixture {
    let warehouse = format!("test-wh-{}", test_id);
    let namespace = format!("ns_{}", test_id);
    let table = format!("tbl_{}", test_id);
    // ...
}
```

**Considerations:**
- MinIO might have rate limits
- Resource cleanup between tests
- Unique naming prevents interference
- Can use `tokio::spawn` or `futures::join!` for parallelism

### Recommended Approach

```rust
#[tokio::main]
async fn main() {
    let test_cases = generate_all_test_cases();

    // Run with controlled concurrency (e.g., 10 at a time)
    let semaphore = Arc::new(Semaphore::new(10));

    let results: Vec<TestResult> = futures::stream::iter(test_cases)
        .map(|tc| {
            let sem = semaphore.clone();
            async move {
                let _permit = sem.acquire().await;
                run_test_case(tc).await
            }
        })
        .buffer_unordered(10)
        .collect()
        .await;

    generate_report(results);
}
```

## Output and Logging

### Test Results Log

Two output streams:

**1. Pass log (one-liner per test)** - for compliance audit trail
```
PASS | create_namespace | combo=1/2 | properties=ON | 200 OK | 45ms
PASS | create_namespace | combo=2/2 | properties=OFF | 200 OK | 38ms
PASS | create_table | combo=1/32 | partition_spec=OFF,sort_order=OFF,properties=OFF,location=OFF,stage_create=OFF | 200 OK | 52ms
PASS | create_table | combo=2/32 | partition_spec=ON,sort_order=OFF,properties=OFF,location=OFF,stage_create=OFF | 200 OK | 61ms
...
```

**2. Failure log (detailed)** - for debugging
```
FAIL | commit_table | combo=5/8 | requirements=[AssertTableUuid],updates=[]
     | Expected: 200 OK
     | Actual: 400 Bad Request
     | Response: {"error": "updates cannot be empty"}
     | Spec reference: https://iceberg.apache.org/spec/#commit-table
     | Classification: SPEC_VIOLATION (spec allows empty updates)
```

### Summary Report

At end of run:
```
============================================================
Iceberg REST API Compliance Test Results
============================================================
Date: 2025-01-14 15:30:00
MinIO Version: RELEASE.2025-01-10
Total tests: 847
Passed: 812 (95.9%)
Failed: 23 (2.7%)
Skipped (unsupported): 12 (1.4%)

Failed tests:
  - commit_table combo 5/8: SPEC_VIOLATION
  - commit_table combo 7/8: SPEC_VIOLATION
  - plan_table_scan combo 3/16: EXPECTED_UNSUPPORTED
  ...

Full pass log: ./results/pass-log-2025-01-14.txt
Full failure log: ./results/fail-log-2025-01-14.txt
============================================================
```

### Output Files

```
results/
├── pass-log-YYYY-MM-DD.txt      # One-liner per passed test
├── fail-log-YYYY-MM-DD.txt      # Detailed failure info
├── summary-YYYY-MM-DD.txt       # Human-readable summary
└── results-YYYY-MM-DD.json      # Machine-readable full results
```

## Open Questions

1. **Test isolation:** Fresh warehouse per test, or shared resources with unique naming?

2. **Parallelization:** Run tests in parallel or sequential?

3. **Report format:** JUnit XML, custom JSON, or both?

4. **CI integration:** Run on every PR, nightly, or manual trigger?

5. **Sequence test filtering:** For phase 2/3, test ALL pairs or filter to "potentially interesting" combinations?

## Next Steps

- [ ] Enumerate all optional fields per API method
- [ ] Design support matrix file format
- [ ] Address SDK validation gaps
- [ ] Prototype test generator for single-method tests
- [ ] Estimate total test count and runtime
