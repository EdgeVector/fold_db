# Design: Loading Views from the Global Registry

## Problem

When a user loads a view from the global schema service, the system must:
1. Fetch the view definition (`StoredView`) from the service
2. Fetch its output schema (needed to reconstruct typed `TransformView`)
3. Resolve transitive dependencies — if the view's input queries reference
   other views, those must also be fetched and loaded
4. Register everything locally in the correct order (leaves before parents)

Today, **none of this exists.** You can push views to the service
(`add_view_to_service`) but there's no pull. Schemas have a pull flow
(`load_schemas`, `get_available_schemas`); views need the same.

## Existing Infrastructure

```
ALREADY BUILT (reuse these):

  SchemaServiceClient          FoldNode                 OperationProcessor
  ├─ get_view(name)            ├─ add_view_to_service   ├─ create_view()
  ├─ get_available_views()     ├─ fetch_available_       ├─ approve_view()
  ├─ list_views()              │  schemas()             ├─ block_view()
  ├─ add_view(request)         └─ require_real_          └─ delete_view()
  ├─ get_schema(name)             schema_service()
  └─ get_available_schemas()

  fold_db (local)
  ├─ schema_manager.register_view(TransformView)
  ├─ schema_manager.load_schema_from_json(json)
  └─ schema_manager.get_view(name) / get_schema(name)
```

## Type Gap

```
StoredView (schema service)          TransformView (local DB)
─────────────────────────            ────────────────────────
name                          ═══▶   name
input_queries                 ═══▶   input_queries
wasm_bytes                    ═══▶   wasm_transform
schema_type                   ═══▶   schema_type
output_schema_name            ───┐
                                 │   key_config          ◀── from output Schema
                                 └▶  output_fields       ◀── from output Schema
                                     (HashMap<String, FieldValueType>)
```

Converting `StoredView → TransformView` requires fetching the output schema
from the service to extract `key_config` and typed `output_fields`.

## Proposed Design

### Flow: Load a Single View

```
User: "Load view 'RevenueByRegion'"
         │
         ▼
┌─ load_view_from_service(name) ─────────────────────────────┐
│                                                             │
│  1. Fetch StoredView from service                          │
│     GET /api/view/RevenueByRegion                          │
│         │                                                   │
│         ▼                                                   │
│  2. Fetch output schema                                    │
│     GET /api/schema/{output_schema_name}                   │
│         │                                                   │
│         ▼                                                   │
│  3. Resolve input dependencies (recursive)                 │
│     For each input_query.schema_name:                      │
│       - Already loaded locally? → skip                     │
│       - Is it a schema on the service? → fetch + load      │
│       - Is it a view on the service? → recurse (load it)   │
│       - Not found anywhere? → error                        │
│         │                                                   │
│         ▼                                                   │
│  4. Convert StoredView → TransformView                     │
│     (using fetched output schema for key_config +          │
│      output_fields)                                        │
│         │                                                   │
│         ▼                                                   │
│  5. Register locally                                       │
│     schema_manager.register_view(transform_view)           │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### Dependency Resolution Detail

```
load_view("ViewC")
  │
  ├─ fetch StoredView "ViewC" from service
  ├─ ViewC.input_queries = [Query("ViewB", ["x"]), Query("SchemaA", ["y"])]
  │
  ├─ resolve "ViewB":
  │    ├─ not loaded locally
  │    ├─ not a schema on service
  │    ├─ IS a view on service → load_view("ViewB")  ◀── recurse
  │    │    ├─ fetch StoredView "ViewB"
  │    │    ├─ ViewB.input_queries = [Query("SchemaX", ["z"])]
  │    │    ├─ resolve "SchemaX":
  │    │    │    ├─ not loaded locally
  │    │    │    ├─ IS a schema on service → fetch + load
  │    │    │    └─ done
  │    │    ├─ convert + register ViewB locally
  │    │    └─ done
  │    └─ done
  │
  ├─ resolve "SchemaA":
  │    ├─ not loaded locally
  │    ├─ IS a schema on service → fetch + load
  │    └─ done
  │
  ├─ convert + register ViewC locally
  └─ done
```

### Cycle Prevention

The recursion uses a `loading: HashSet<String>` to detect cycles:

```
load_view("ViewC", loading = {"ViewC"})
  └─ load_view("ViewB", loading = {"ViewC", "ViewB"})
       └─ load_view("ViewC", loading = {"ViewC", "ViewB"})
            └─ "ViewC" already in loading set → ERROR
```

This mirrors the `visited` set in cascade invalidation. The local
`register_view` also has cycle detection, but checking early avoids
wasted network calls.

### Depth Limit

Enforce `MAX_VIEW_LOAD_DEPTH = 16` (matching the proposed chain depth limit).
Each recursive call increments a depth counter. Exceeding it returns an error
with the chain path for debugging.

## Where the Code Goes

```
fold_db_node/src/fold_node/
├── node.rs                    ← add load_view_from_service()
├── schema_client.rs           ← already has get_view(), get_schema() ✓
└── operation_processor/
    └── view_ops.rs            ← add load_view() operation

fold_db_node/src/server/routes/
└── views.rs                   ← add POST /api/views/load/{name} endpoint
```

**4 files touched, 0 new files, 0 new types.** The `StoredView → TransformView`
conversion is a function in `node.rs` or `view_ops.rs`, not a new struct.

## API

### Load a single view (with transitive dependencies)

```
POST /api/views/load/{name}

Response 200:
{
  "loaded_views": ["SchemaX", "ViewB", "ViewC"],
  "loaded_schemas": ["SchemaA"],
  "already_loaded": ["ExistingSchema"]
}

Response 400:
{
  "error": "View 'ViewB' not found on schema service"
}

Response 400:
{
  "error": "View chain depth exceeds limit of 16: ViewC → ViewB → ... → ViewA"
}
```

### Load all available views

```
POST /api/views/load-all

Response 200:
{
  "total": 12,
  "loaded": 10,
  "failed": ["BrokenView", "MissingDepView"]
}
```

This mirrors the existing `POST /api/schemas/load` endpoint.

## Edge Cases

| Case | Behavior |
|------|----------|
| View already loaded locally | Skip, return in `already_loaded` |
| View depends on locally-loaded schema | Skip schema fetch, proceed |
| View depends on unknown name (not schema or view) | Error with clear message |
| Circular dependency detected | Error before any network calls after first detection |
| Output schema not found on service | Error — StoredView is orphaned |
| WASM bytes are large (>10MB) | Let it through — wasmtime handles validation |
| Schema service unreachable | Error from SchemaServiceClient propagates |
| View's source schema was expanded (old name blocked) | Load the current active schema, not the blocked one |

## NOT in Scope

- **Auto-sync / watch for new views** — manual load only, like schemas
- **Conflict resolution** — if local view differs from service, service wins (overwrite)
- **Partial loading** — either all dependencies load or none do (no partial state)
- **View versioning** — no version tracking, just latest from service
- **Permission checks on views** — views inherit source schema permissions
