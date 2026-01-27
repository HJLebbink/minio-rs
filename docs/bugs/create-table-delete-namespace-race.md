# Race Condition: CreateTable vs DeleteNamespace

## Summary

A race condition exists between `CreateTable` and `DeleteNamespace` operations that causes a 500 Internal Server Error instead of the expected 404 Not Found when a namespace is deleted while a table creation is in progress.

**Test Failure:**

```
seq-01541 | delete_namespace || create_table | state=W,N
  Expected: no 500 errors
  Actual:   204:OK || 500:INTERNAL_ERROR
```

## Root Cause

### Lock Analysis

`CreateTable` and `DeleteNamespace` use different lock paths, allowing them to run concurrently:

| Operation         | Locks Acquired                                                                                                                                          |
| ----------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `CreateTable`     | 1. Warehouse **read** lock<br>2. `catalog/<wh>/namespaces/<ns>.lock` (table registry lock)                                                              |
| `DeleteNamespace` | 1. Warehouse **read** lock<br>2. `catalog/<wh>/namespaces-registry.json` (namespace registry lock)<br>3. `catalog/<wh>/<ns>/` (namespace location lock) |

Since both use warehouse **read** locks and acquire different registry locks, they don't block each other.

### Race Timeline

1. **CreateTable** checks `NamespaceExists()` at `tables-object-layer-impl.go:1103` - returns `true`
2. **CreateTable** performs expensive I/O (creates UUID, metadata, pointer) at lines 1116-1171 - **NO registry lock held**
3. **DeleteNamespace** runs concurrently:
   - Verifies namespace is empty
   - Acquires namespace lock
   - **Deletes table registry shard directory** at line 730: `shardDir := getTableRegistryShardDir(warehouse, namespace)`
   - Removes namespace from registry
4. **CreateTable** acquires table registry lock at line 1178 (succeeds - different lock path)
5. **CreateTable** calls `tabRegistry.GetTable()` at line 1210
6. **The shard files no longer exist** - returns `errTablesShardMissing`
7. Error wrapped as `TablesTableCreateFailedError{Cause: errTablesShardMissing}` at line 1214
8. `toTablesAPIError` unwraps but **`errTablesShardMissing` is not mapped** - falls through to `toAPIError` - **500 Internal Server Error**

## Code Analysis

### CreateTable (tables-object-layer-impl.go:1087-1286)

```go
func (i TablesObjectLayer) CreateTable(ctx context.Context, opts TableOptions) (result *LoadTableResult, err error) {
    // Line 1097: Acquire warehouse READ lock
    whRelease, err := i.lockWarehouseForRead(ctx, opts.Warehouse, true)

    // Line 1103: Check namespace exists - NO LOCK on namespace itself
    exists, err := i.NamespaceExists(ctx, opts.Warehouse, opts.Namespace)
    if !exists {
        return nil, TablesTableCreateFailedError{Cause: TablesNamespaceNotFoundError{...}}
    }

    // Lines 1116-1171: Expensive I/O operations - NO registry lock held
    // - Generate table UUID
    // - Create metadata
    // - Write metadata pointer

    // Line 1176-1178: Acquire table registry lock (different from namespace registry lock)
    tabRegistryPath := getTableRegistryLockPath(opts.Warehouse, opts.Namespace)
    registryLock := NewNSLock(minioMetaBucket, tabRegistryPath)

    // Line 1210: Try to read registry - FAILS if namespace was deleted
    oldTable, err := tabRegistry.GetTable(ctx, newEntry.Name)
    if err != nil {
        // ... cleanup ...
        return nil, TablesTableCreateFailedError{Cause: err}  // err = errTablesShardMissing
    }
}
```

### DeleteNamespace (tables-object-layer-impl.go:636-768)

```go
func (i TablesObjectLayer) DeleteNamespace(ctx context.Context, warehouse string, namespace string) (err error) {
    // Line 645: Acquire warehouse READ lock (same as CreateTable - no conflict)
    whRelease, err := i.lockWarehouseForRead(ctx, warehouse, true)

    // Line 651-657: Acquire namespace REGISTRY lock (different from table registry lock)
    nsRegistryPath := getNamespaceRegistryPath(warehouse)
    lock := NewNSLock(minioMetaBucket, nsRegistryPath)

    // Line 707-712: Acquire namespace LOCATION lock
    nsLock := NewNSLock(minioMetaBucket, getNamespaceLocationInSysBucket(warehouse, namespace))

    // Line 730: DELETE the table registry shard directory
    shardDir := getTableRegistryShardDir(warehouse, namespace)
    _, _ = i.objectLayer.DeleteObject(ctx, minioMetaBucket, shardDir, ObjectOptions{DeletePrefix: true, NoLock: true})

    // Lines 759-765: Remove from namespace registry
    nsRegistry.RemoveNamespace(namespace)
    nsRegistry.Write(ctx, i.objectLayer, false)
}
```

### Error Handling Gap (tables-api-errors.go)

`errTablesShardMissing` is defined but NOT mapped in `toTablesAPIError`:

```go
// Line 49: Definition
errTablesShardMissing = errors.New("Table registry shard file missing")

// Lines 266-435: Sentinel error switch - errTablesShardMissing is MISSING
switch unwrapped {
case errTablesNamespaceNotFound:
    return APIError{Code: "NoSuchNamespaceException", HTTPStatusCode: http.StatusNotFound, ...}
// ... other cases ...
// errTablesShardMissing is NOT handled here
}

// Line 438: Falls through to generic handler
return toAPIError(ctx, err)  // Returns 500
```

### Correct Handling Elsewhere

Other code paths properly handle `errTablesShardMissing`:

```go
// tables-object-layer-impl.go:1845-1846
if errors.Is(regErr, errConfigNotFound) || errors.Is(regErr, errTablesShardMissing) {
    return nil, TablesNamespaceNotFoundError{Namespace: opts.Namespace}
}

// tables-object-layer-impl.go:2700-2701
if errors.Is(err, errTablesShardMissing) {
    return TablesNamespaceNotFoundError{Namespace: namespace}
}

// tables-object-layer-impl.go:3303-3304
if errors.Is(err, errTablesShardMissing) {
    return TablesNamespaceNotFoundError{Namespace: namespace}
}
```

## Lock Path Details

```go
// Table registry lock (used by CreateTable)
func getTableRegistryLockPath(warehouse, namespace string) string {
    return tablesPrefix + warehouse + "/" + namespacesPrefix + "/" + namespace + ".lock"
    // Example: catalog/my-warehouse/namespaces/my-namespace.lock
}

// Namespace registry lock (used by DeleteNamespace)
func getNamespaceRegistryPath(warehouse string) string {
    return tablesPrefix + warehouse + "/" + namespacesRegistryFile
    // Example: catalog/my-warehouse/namespaces-registry.json
}

// Namespace location lock (used by DeleteNamespace)
func getNamespaceLocationInSysBucket(warehouse string, namespace string) string {
    return warehouseNameToLocation(warehouse) + namespace + "/"
    // Example: catalog/my-warehouse/my-namespace/
}
```

## Potential Fixes

### Option 1: Map errTablesShardMissing in Error Handler

Add `errTablesShardMissing` to the sentinel error switch in `toTablesAPIError`:

```go
case errTablesShardMissing:
    return APIError{
        Code:           "NoSuchNamespaceException",
        Description:    "Namespace does not exist",
        HTTPStatusCode: http.StatusNotFound,
    }
```

### Option 2: Handle in CreateTable

Handle `errTablesShardMissing` explicitly in `CreateTable` at line 1211:

```go
oldTable, err := tabRegistry.GetTable(ctx, newEntry.Name)
if err != nil {
    if errors.Is(err, errTablesShardMissing) {
        return nil, TablesTableCreateFailedError{
            Cause: TablesNamespaceNotFoundError{Namespace: opts.Namespace},
        }
    }
    // ... existing cleanup and error handling ...
}
```

### Option 3: Proper Locking

Have `CreateTable` acquire a namespace read lock to prevent deletion during table creation:

```go
// Before expensive I/O operations
nsRelease, err := i.lockNamespaceForRead(ctx, opts.Warehouse, opts.Namespace, true)
if err != nil {
    return nil, TablesTableCreateFailedError{Cause: err}
}
defer nsRelease()
```

## Files Involved

- `cmd/tables-object-layer-impl.go` - CreateTable (lines 1087-1286), DeleteNamespace (lines 636-768)
- `cmd/tables-api-errors.go` - Error definitions and mapping
- `cmd/tables-api-utils.go` - Lock path functions, TableRegistry operations
