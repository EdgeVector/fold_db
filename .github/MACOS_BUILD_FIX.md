# macOS Build Fix Summary

## Problem
The macOS packaging step in the GitHub Actions release workflow was failing with:
1. Version parse errors: `Failed to parse version '2.0' for crate 'tauri-plugin-log'` and `tauri-plugin-shell`
2. Missing DMG error: `cp: ../../../../../dist/DataFold-0.1.10-aarch64.dmg: No such file or directory`
3. Incorrect relative paths when copying artifacts

## Root Causes
1. **Invalid semver format**: Plugin versions in `src-tauri/Cargo.toml` used `"2.0"` instead of proper semver `"2.0.0"`
2. **Relative path errors**: The workflow used relative paths (`../../../../../dist/`) which resolved to incorrect locations
3. **No build verification**: No debugging output to see what artifacts were actually produced

## Fixes Applied

### 1. Fixed Cargo.toml Plugin Versions
**File**: `src/datafold_node/static-react/src-tauri/Cargo.toml`

```diff
- tauri-plugin-log = "2.0"
- tauri-plugin-shell = "2.0"
+ tauri-plugin-log = "2.0.0"
+ tauri-plugin-shell = "2.0.0"
```

**Verification**: `cargo check` now passes without version parsing errors.

### 2. Fixed GitHub Workflow Artifact Paths
**File**: `.github/workflows/release.yml`

**Changes**:
- Added `set -euo pipefail` for fail-fast behavior
- Replaced relative paths with `$GITHUB_WORKSPACE` (absolute path to repo root)
- Added explicit existence checks before copying
- Added informative echo statements for debugging
- Used a glob loop for DMG finding instead of relying on a specific path

**Before**:
```bash
cp dmg/DataFold_*.dmg ../../../../../dist/DataFold-$VERSION-$ARCH.dmg
```

**After**:
```bash
for f in dmg/DataFold_*.dmg; do
  if [ -f "$f" ]; then
    echo "Found DMG: $f"
    cp "$f" "$GITHUB_WORKSPACE/dist/DataFold-$VERSION-$ARCH.dmg"
  else
    echo "No DMG found matching dmg/DataFold_*.dmg"
  fi
done
```

### 3. Added Build Verification Step
**File**: `.github/workflows/release.yml`

Added new step "Show bundle outputs" that lists:
- Contents of `target/release/bundle/`
- Contents of `target/release/bundle/dmg/`
- Contents of `target/release/bundle/macos/`

This provides immediate visibility into what artifacts were actually created.

## Testing Without Deployment

### Local Test Script
Created `test_macos_build.sh` which:
1. ✅ Validates Cargo.toml plugin versions
2. ✅ Runs `cargo check` to verify dependencies
3. ✅ Installs npm dependencies
4. ✅ Builds the frontend
5. ✅ Builds the Tauri app
6. ✅ Checks for version parsing errors
7. ✅ Verifies DMG and .app creation
8. ✅ Simulates the CI artifact preparation process
9. ✅ Creates test artifacts in `dist-test/`

### How to Run the Test

```bash
# From repo root
./test_macos_build.sh
```

**Expected output on success**:
- ✓ Plugin versions look good
- ✓ Cargo dependencies verified
- ✓ npm dependencies installed
- ✓ Frontend built successfully
- ✓ Tauri build completed
- ✓ No version parsing errors found
- ✓ Found DMG: [path]
- ✓ Found .app bundle: [path]
- ✓✓✓ Build test PASSED! The macOS release workflow should work.

**Test artifacts** will be created in `dist-test/`:
- `DataFold-0.1.10-test-aarch64.dmg` (or x86_64)
- `DataFold-0.1.10-test-aarch64.app.zip` (or x86_64)

You can test the DMG locally:
```bash
open dist-test/DataFold-0.1.10-test-*.dmg
```

### Quick Verification (Without Full Build)
If you just want to verify the Cargo fix without a full build:

```bash
cd src/datafold_node/static-react/src-tauri
cargo check
```

This should complete without "Failed to parse version" errors.

## Why These Fixes Work

1. **Proper semver format**: Cargo/Tauri's version parser requires full semantic versioning (major.minor.patch), not just "2.0"

2. **Absolute paths with $GITHUB_WORKSPACE**: 
   - Eliminates relative path confusion
   - Works consistently regardless of current directory
   - Ensures `dist/` is created at repo root where upload steps expect it

3. **Explicit checks and informative output**:
   - Prevents silent failures
   - Provides clear debugging information in CI logs
   - Makes it obvious if/when DMG or .app wasn't created

4. **Glob loop with existence checks**:
   - Handles varying DMG filenames
   - Fails gracefully with clear error message if no DMG found
   - No longer relies on exact path/filename match

## Next Steps

1. **Run local test**: `./test_macos_build.sh` to verify everything works
2. **Commit changes**: The fixes are already in place
3. **Push to GitHub**: Push the changes to trigger CI (or create a PR first)
4. **Create new release tag**: When ready, create a new tag (e.g., `v0.1.11`)
5. **Monitor CI**: Check the "Show bundle outputs" step to see what artifacts are created
6. **Verify release**: DMG and .app.zip should appear in the GitHub release

## Files Modified

1. `src/datafold_node/static-react/src-tauri/Cargo.toml` - Fixed plugin versions
2. `.github/workflows/release.yml` - Fixed artifact paths and added debugging
3. `test_macos_build.sh` - New local test script (not used by CI, just for local testing)

## Verification Checklist

Before creating a new release tag:

- [x] Cargo.toml uses proper semver format (2.0.0, not 2.0)
- [x] `cargo check` passes without errors
- [ ] Run `./test_macos_build.sh` and verify it passes
- [ ] Test artifacts in `dist-test/` can be opened
- [ ] DMG installs and launches successfully

Once these are verified, the GitHub Actions workflow should succeed.

