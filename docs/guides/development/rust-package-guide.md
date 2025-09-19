# Making DataFold Available as a Rust Package

This guide shows how to prepare and publish the DataFold project as a Rust package on [crates.io](https://crates.io).

## 1. Package Configuration (Complete ✅)

The `Cargo.toml` has been updated with the required metadata:

```toml
[package]
name = "datafold"
version = "0.1.0"
edition = "2021"
authors = ["Your Name <your.email@example.com>"]
description = "A distributed data platform with schema-based storage and AI-powered ingestion"
documentation = "https://docs.rs/datafold"
homepage = "https://github.com/yourusername/datafold"
repository = "https://github.com/yourusername/datafold"
readme = "README.md"
license = "MIT OR Apache-2.0"
keywords = ["database", "distributed", "schema", "ingestion", "ai"]
categories = ["database", "data-structures", "network-programming"]
```

**You need to update:**
- `authors` - Replace with your actual name and email
- `homepage` and `repository` - Replace with your actual GitHub URLs

## 2. Required Files

### License Files
You need to add license files to your project root:

**Option A: MIT License**
```bash
# Create MIT license file
curl -s https://raw.githubusercontent.com/github/choosealicense.com/gh-pages/_licenses/mit.txt > LICENSE-MIT
```

**Option B: Apache 2.0 License**  
```bash
# Create Apache license file
curl -s https://raw.githubusercontent.com/github/choosealicense.com/gh-pages/_licenses/apache-2.0.txt > LICENSE-APACHE
```

**Recommended: Both (as specified in Cargo.toml)**
```bash
# Add both licenses
curl -s https://raw.githubusercontent.com/github/choosealicense.com/gh-pages/_licenses/mit.txt > LICENSE-MIT
curl -s https://raw.githubusercontent.com/github/choosealicense.com/gh-pages/_licenses/apache-2.0.txt > LICENSE-APACHE
```

### Documentation
Ensure your `README.md` properly describes:
- What DataFold does
- Installation instructions
- Basic usage examples
- API documentation links

## 3. Pre-Publication Checklist

### Test Your Package
```bash
# Check that your package builds
cargo check

# Run tests
cargo test

# Build documentation locally
cargo doc --open

# Check for issues before publishing
cargo publish --dry-run
```

### Verify Binary Targets
The package includes these binaries:
- `datafold_cli` - Command-line interface
- `datafold_http_server` - HTTP server
- `datafold_node` - Node server

Test that they build:
```bash
cargo build --bin datafold_cli
cargo build --bin datafold_http_server  
cargo build --bin datafold_node
```

## 4. Publishing Process

### One-Time Setup
```bash
# Install cargo account tools
cargo install cargo-edit

# Login to crates.io (you'll need a crates.io account)
cargo login
```

### Publish
```bash
# Final check
cargo publish --dry-run

# Publish to crates.io
cargo publish
```

## 5. Using the Published Package

Once published, users can:

### Install the Library
```toml
# In their Cargo.toml
[dependencies]
datafold = "0.1.0"
```

### Install the Binaries
```bash
# Install all binaries
cargo install datafold

# Or install specific binaries
cargo install datafold --bin datafold_cli
cargo install datafold --bin datafold_http_server
cargo install datafold --bin datafold_node
```

### Use in Code
```rust
use datafold::{DataFoldNode, IngestionCore, Schema};

// Your code here
```

## 6. Best Practices

### Versioning
- Follow [Semantic Versioning](https://semver.org/)
- Start with `0.1.0` for initial release
- Update version in `Cargo.toml` for each release

### Documentation
- Add `//!` doc comments to your `lib.rs`
- Add `///` doc comments to public functions
- Include examples in doc comments
- Documentation will auto-generate at `https://docs.rs/datafold`

### Feature Flags
Your package already has good feature flags:
```toml
[features]
default = ["mock"]
test-utils = []
simulate-peers = []
mock = []
```

### Testing
- Ensure `cargo test` passes
- Consider adding integration tests
- Test on different platforms if possible

## 7. Maintenance

### Updates
```bash
# Bump version and publish update
cargo edit set-version 0.1.1
cargo publish
```

### Yanking Bad Releases
```bash
# If you need to remove a version
cargo yank --version 0.1.0
```

## 8. Next Steps

1. **Update author information** in `Cargo.toml`
2. **Add license files** (LICENSE-MIT and LICENSE-APACHE)  
3. **Test the package** with `cargo publish --dry-run`
4. **Create crates.io account** and login with `cargo login`
5. **Publish** with `cargo publish`

Your DataFold package will then be available for the Rust community to discover and use!