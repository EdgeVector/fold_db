# Coverage Reporting Configuration

This document describes the comprehensive code coverage setup for the datafold project, including both frontend (React/Vitest) and backend (Rust) components.

## Overview

The project implements a **minimum 80% code coverage** requirement across all modules with automated enforcement through CI/CD pipelines.

## Coverage Infrastructure

### Frontend Coverage (Vitest + V8)

#### Configuration Files
- [`vitest.config.js`](./vitest.config.js) - Main Vitest configuration with coverage settings
- [`package.json`](./package.json) - Coverage scripts and dependencies
- [`.gitignore`](./.gitignore) - Excludes coverage artifacts from version control

#### Coverage Thresholds
- **Global minimum**: 80% for all metrics (lines, functions, statements, branches)
- **Hooks**: 85% (higher standard for critical utility functions)
- **Utils**: 90% (highest standard for pure utility functions)
- **Store/API**: 85% (business logic requires high coverage)
- **Components**: 80% (standard UI component coverage)

#### Key Features
- **Automatic exclusions**: Test files, build artifacts, configuration files
- **Multiple report formats**: Text, HTML, JSON, LCOV for CI integration
- **Threshold enforcement**: Builds fail if coverage drops below minimum
- **Watermark indicators**: Visual coverage quality indicators (80%-90% range)

### Backend Coverage (Rust + LLVM-COV)

#### Configuration
- [`scripts/generate_coverage.sh`](../../scripts/generate_coverage.sh) - Unified coverage generation
- Uses `cargo-llvm-cov` for comprehensive Rust coverage

### CI/CD Integration

#### GitHub Actions
- [`.github/workflows/coverage.yml`](../../.github/workflows/coverage.yml) - Automated coverage pipeline
- **Triggers**: Push to main/develop, pull requests
- **Reports**: Codecov integration, PR comments, artifact uploads

#### Coverage Pipeline Features
- **Parallel execution**: Frontend and Rust coverage run simultaneously
- **Threshold enforcement**: Builds fail on insufficient coverage
- **Artifact retention**: 30-day coverage report storage
- **PR integration**: Automatic coverage comments on pull requests

## Available Scripts

### Frontend Coverage Scripts

```bash
# Basic coverage generation
npm run test:coverage

# Coverage with file watching
npm run test:coverage:watch

# Coverage with UI dashboard
npm run test:coverage:ui

# Detailed coverage report
npm run test:coverage:report

# Generate JSON summary for CI
npm run test:coverage:threshold

# Open HTML coverage report
npm run coverage:open

# Serve coverage reports locally
npm run coverage:serve

# CI pipeline (lint + coverage + threshold check)
npm run ci:test

# Check coverage thresholds
npm run ci:coverage-check
```

### Combined Coverage Scripts

```bash
# Generate both frontend and backend coverage
./scripts/generate_coverage.sh

# Quick coverage check (frontend only)
cd src/datafold_node/static-react && npm run test:coverage
```

## Coverage Reports

### Report Formats

1. **Console Output**: Real-time coverage metrics during test runs
2. **HTML Reports**: Interactive browser-based coverage exploration
3. **JSON Summary**: Machine-readable coverage data for CI
4. **LCOV**: Industry-standard format for external integrations

### Report Locations

```
Frontend Coverage:
├── coverage/
│   ├── index.html              # Main HTML report
│   ├── lcov.info              # LCOV format for CI
│   ├── coverage-summary.json  # JSON summary
│   └── ...                    # Detailed coverage files

Backend Coverage:
├── target/
│   ├── coverage-html/         # HTML reports
│   ├── coverage.lcov          # LCOV format
│   └── llvm-cov/             # Raw coverage data
```

## Quality Gates

### Coverage Enforcement

The coverage system enforces quality through multiple mechanisms:

1. **Build Failures**: Tests fail if coverage drops below thresholds
2. **PR Blocks**: Pull requests show coverage status and changes
3. **Trend Monitoring**: Codecov tracks coverage trends over time
4. **File-level Analysis**: Individual file coverage requirements

### Coverage Exclusions

#### Frontend Exclusions
```javascript
// Automatically excluded from coverage:
- src/test/**                    // Test files
- **/*.{test,spec}.*            // Test files by pattern
- src/main.jsx                  // Entry point (minimal logic)
- src/assets/**                 // Static assets
- src/styles/**                 // Styling files
- **/*.config.{js,ts}           // Configuration files
```

#### Backend Exclusions
```rust
// Excluded via standard cargo-llvm-cov patterns:
- tests/**                      // Test modules
- examples/**                   // Example code
- target/**                     // Build artifacts
```

## Development Workflow

### Local Development

1. **Run tests with coverage**:
   ```bash
   npm run test:coverage
   ```

2. **Check coverage thresholds**:
   ```bash
   npm run ci:coverage-check
   ```

3. **Open coverage reports**:
   ```bash
   npm run coverage:open
   ```

4. **Generate complete project coverage**:
   ```bash
   ./scripts/generate_coverage.sh
   ```

### Continuous Integration

The CI pipeline automatically:
1. Runs all tests with coverage collection
2. Enforces minimum coverage thresholds
3. Uploads reports to Codecov
4. Comments coverage changes on PRs
5. Stores coverage artifacts

### Coverage Improvement

When coverage falls below thresholds:

1. **Identify gaps**: Use HTML reports to find uncovered lines
2. **Add tests**: Focus on uncovered branches and functions
3. **Review exclusions**: Ensure only appropriate files are excluded
4. **Test edge cases**: Cover error handling and boundary conditions

## Codecov Integration

### Features
- **Coverage trends**: Historical coverage tracking
- **PR integration**: Automated coverage comments
- **Flag-based reporting**: Separate frontend/backend tracking
- **Threshold enforcement**: Configurable quality gates

### Configuration
- [`.codecov.yml`](../../.codecov.yml) - Codecov settings and thresholds

## Troubleshooting

### Common Issues

#### "Coverage threshold not met"
```bash
# Check which files are below threshold
npm run ci:coverage-check

# Generate detailed coverage report
npm run test:coverage:report
```

#### "No coverage data found"
```bash
# Ensure tests are running correctly
npm test

# Verify Vitest configuration
npm run test:coverage:ui
```

#### "CI pipeline failing on coverage"
```bash
# Run the same checks locally
npm run ci:test

# Review coverage changes
git diff HEAD~1 -- coverage/
```

### Performance Optimization

#### Large Test Suites
- Use `npm run test:coverage:watch` for faster feedback
- Consider test parallelization for large codebases
- Optimize test setup and teardown

#### Coverage Generation Speed
- Exclude unnecessary files via configuration
- Use incremental testing during development
- Leverage CI caching for dependencies

## Maintenance

### Regular Tasks
- Review coverage trends monthly
- Update thresholds based on project maturity
- Monitor CI pipeline performance
- Update exclusion patterns as codebase evolves

### Configuration Updates
- Threshold adjustments in `vitest.config.js`
- CI pipeline updates in `.github/workflows/coverage.yml`
- Codecov settings in `.codecov.yml`

## Resources

- [Vitest Coverage Documentation](https://vitest.dev/guide/coverage.html)
- [Codecov Documentation](https://docs.codecov.com/)
- [cargo-llvm-cov Documentation](https://github.com/taiki-e/cargo-llvm-cov)
- [LCOV Format Specification](http://ltp.sourceforge.net/coverage/lcov/genhtml.1.php)