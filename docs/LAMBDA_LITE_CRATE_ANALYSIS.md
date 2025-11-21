# Separate Lambda Crate Analysis: `fold_db_lite`

## Current State

The lambda module (`src/lambda/mod.rs`) is 219 lines and depends on:
- `datafold_node::{DataFoldNode, NodeConfig}` - Core node
- `ingestion::{create_progress_tracker, IngestionError, IngestionProgress, ProgressTracker}` - Ingestion types
- `once_cell::sync::OnceCell` - Singleton pattern
- `std::sync::Arc` - Thread-safe reference counting

## Can It Be a Separate Crate?

### Option 1: Separate Crate with Full Dependency ❌

```toml
# fold_db_lite/Cargo.toml
[dependencies]
datafold = "0.1"
once_cell = "1.0"
lambda_runtime = "0.13"
```

**Problems:**
- Still pulls in ALL of datafold (5.3 MB)
- No size savings
- Just adds another crate to maintain
- **Not actually "lite"**

**Verdict:** ❌ Doesn't achieve the goal

### Option 2: Multi-Crate Architecture ✅ (But Major Refactor)

```
fold_db/
├── datafold-core/          # Core database, schemas, atoms
│   └── ~2 MB binary
├── datafold-ingestion/     # Ingestion, file conversion
│   └── Depends on: datafold-core
├── datafold-server/        # HTTP server, API routes  
│   └── Depends on: datafold-core, datafold-ingestion
├── datafold-lambda/        # Lambda helpers
│   └── Depends on: datafold-core (~2.5 MB total)
└── datafold/               # Full featured (re-exports all)
    └── Depends on: all above (~5.3 MB total)
```

**Users could choose:**

```toml
# Lambda-only users (minimal)
[dependencies]
datafold-core = "0.1"
datafold-lambda = "0.1"
# Total: ~2.5 MB

# Full users
[dependencies]
datafold = "0.1"  # Gets everything
# Total: ~5.3 MB
```

**Benefits:**
- ✅ Lambda users get ~50% smaller binary
- ✅ Clear separation of concerns
- ✅ Users pay only for what they use
- ✅ Faster compilation for light users

**Costs:**
- ⚠️ Major refactoring effort
- ⚠️ Need to manage multiple crates
- ⚠️ Versioning complexity
- ⚠️ More CI/CD complexity

### Option 3: Feature Flags (Current Approach) ✅ (Best for Now)

```toml
# Cargo.toml
[features]
default = ["server", "ingestion"]
core = []
lambda = ["lambda_runtime"]
server = ["actix-web"]
ingestion = ["file_to_json"]
```

**Users could choose:**

```toml
# Minimal Lambda
[dependencies]
datafold = { version = "0.1", features = ["lambda"], default-features = false }
# Potential: ~2.5-3 MB

# Full featured
[dependencies]
datafold = { version = "0.1", features = ["lambda"] }
# Current: ~5.3 MB
```

**Benefits:**
- ✅ Single crate to maintain
- ✅ Users can customize features
- ✅ No breaking changes needed
- ✅ Can implement incrementally

**Current Reality:**
- ⚠️ We haven't fully implemented feature gating
- ⚠️ Many dependencies are not optional yet

## Recommendation

### Short Term (Immediate): Keep Current Approach ✅

The lambda module should stay in the main crate because:

1. **It's Already Lightweight**
   - Lambda module itself: ~200 KB
   - Only adds lambda_runtime dependency
   - Clean, simple to use

2. **Dependencies Are Core**
   - Needs `DataFoldNode` (the core type)
   - Needs progress tracking
   - Can't reasonably separate these

3. **Feature Flag Works Well**
   - Users who don't need lambda don't compile it
   - Optional dependency (`lambda_runtime`)
   - Zero overhead if not used

4. **Maintenance Simplicity**
   - One crate = one version
   - Easier to test and release
   - Less cognitive overhead

### Medium Term (3-6 months): Better Feature Gating

Improve feature flags to allow minimal builds:

```toml
# Cargo.toml
[features]
default = ["server", "ingestion", "s3", "dynamodb"]
core = []
lambda = ["lambda_runtime"]
server = ["actix-web", "actix-files", "actix-cors"]
ingestion = ["file_to_json", "server"]
s3 = ["aws-sdk-s3"]
dynamodb = ["aws-sdk-dynamodb"]

[dependencies]
actix-web = { version = "4.3", optional = true }
aws-sdk-s3 = { version = "1.0", optional = true }
aws-sdk-dynamodb = { version = "1.0", optional = true }
file_to_json = { version = "0.1", optional = true }
lambda_runtime = { version = "0.13", optional = true }
```

**Users could build minimal Lambda:**

```toml
[dependencies]
datafold = { 
    version = "0.1", 
    features = ["lambda"],
    default-features = false 
}
```

**Result:** Binary could be ~2-3 MB instead of 5.3 MB

### Long Term (6-12 months): Multi-Crate (If Needed)

Only if the project grows significantly and:
- Multiple teams maintaining different parts
- Clear need for independent versioning
- Community demand for smaller binaries

Split into:
- `datafold-core` - Database core
- `datafold-ingestion` - File ingestion
- `datafold-server` - HTTP server
- `datafold-lambda` - Lambda helpers
- `datafold` - Full featured (re-exports)

## Concrete Action Plan

### Phase 1: Improve Current Structure (Now)
1. ✅ Keep lambda module in main crate
2. ✅ Document it's optional via feature flag
3. ✅ Ensure it stays lightweight (<500 lines)

### Phase 2: Feature Flag Optimization (Next Release)
1. Make actix-web optional (server feature)
2. Make file_to_json optional (ingestion feature)
3. Make AWS SDKs optional (s3/dynamodb features)
4. Document minimal build options

**Example minimal build:**
```toml
datafold = { 
    version = "0.1", 
    features = ["lambda"],
    default-features = false 
}
# Result: ~2.5 MB (50% smaller!)
```

### Phase 3: Separate Crate (If Justified)
Only if:
- Binary size is still a problem after Phase 2
- User feedback demands it
- Multiple maintainers want separation

## Conclusion

**Should we create `fold_db_lite` now?** 

**No.** Here's why:

1. **Current approach is good**
   - Lambda module is already minimal
   - Feature flag provides opt-in
   - Binary size is competitive

2. **Better solution: Feature flags**
   - Make dependencies optional
   - Users choose what they need
   - No multi-crate complexity

3. **Premature separation**
   - Lambda module needs DataFoldNode
   - Would still depend on ~80% of datafold
   - Not actually "lite"

**Better short-term action:**
Improve feature flags so users can build:
```bash
cargo build --release --no-default-features --features lambda
# Target: ~2.5 MB binary
```

This achieves the goal without the complexity of multiple crates!

## Implementation Checklist

To enable true minimal builds:

- [ ] Make actix-web optional (server feature)
- [ ] Make file_to_json optional (ingestion feature)  
- [ ] Make aws-sdk-s3 optional (s3 feature)
- [ ] Make aws-sdk-dynamodb optional (dynamodb feature)
- [ ] Document minimal build in README
- [ ] Add CI job testing minimal build
- [ ] Measure and document size savings
- [ ] Update lambda docs with optimization tips

**Estimated effort:** 2-4 hours
**Estimated savings:** 40-50% binary size for minimal builds

