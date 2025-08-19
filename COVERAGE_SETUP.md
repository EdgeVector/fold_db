# Code Coverage Setup - UTC-1-10

This document outlines the comprehensive code coverage infrastructure implemented for the datafold project as part of UTC-1-10.

## Overview

The project now enforces a **minimum 80% code coverage** requirement across all modules with automated CI/CD integration and detailed reporting.

## Infrastructure Components

### 1. Frontend Coverage (React/Vitest)
- **Framework**: Vitest with V8 coverage provider
- **Location**: `src/datafold_node/static-react/`
- **Thresholds**: 80% minimum (higher for hooks/utils: 85-90%)
- **Config**: [`vitest.config.js`](src/datafold_node/static-react/vitest.config.js)

### 2. Backend Coverage (Rust/LLVM-COV)
- **Framework**: cargo-llvm-cov
- **Coverage**: Workspace-wide Rust coverage
- **Output**: HTML reports + LCOV format

### 3. CI/CD Pipeline
- **GitHub Actions**: [`.github/workflows/coverage.yml`](.github/workflows/coverage.yml)
- **Triggers**: Push to main/develop, pull requests
- **Features**: Parallel execution, threshold enforcement, PR comments

### 4. Reporting & Integration
- **Codecov**: Central coverage dashboard with trends
- **Reports**: HTML, JSON, LCOV formats
- **Artifacts**: 30-day retention of coverage reports

## Key Features Implemented

### ✅ Coverage Collection
- **Frontend**: Vitest V8 provider with comprehensive source file inclusion
- **Backend**: cargo-llvm-cov for Rust workspace coverage
- **Unified**: Single script for both frontend and backend coverage

### ✅ Threshold Enforcement
- **Minimum 80%** across lines, functions, statements, branches
- **Differentiated thresholds** for different file types
- **Build failures** when coverage drops below thresholds
- **Detailed reporting** of files below threshold

### ✅ CI/CD Integration
- **Automated coverage** on every push and pull request
- **Parallel execution** of frontend and backend coverage
- **PR integration** with coverage change comments
- **Artifact uploads** for detailed analysis

### ✅ Developer Tools
- **Local coverage generation**: `./scripts/generate_coverage.sh`
- **Threshold checking**: `npm run ci:coverage-check`
- **Multiple report formats**: Console, HTML, JSON, LCOV
- **Coverage serving**: Local HTTP server for reports

### ✅ Quality Gates
- **Pre-commit coverage**: Local verification before commits
- **PR blocking**: Coverage requirements for merge approval
- **Trend monitoring**: Historical coverage tracking via Codecov
- **File-level analysis**: Individual file coverage requirements

## Usage

### Quick Start
```bash
# Generate complete project coverage
./scripts/generate_coverage.sh

# Frontend-only coverage
cd src/datafold_node/static-react
npm run test:coverage

# Check coverage thresholds
npm run ci:coverage-check

# Open coverage reports
npm run coverage:open
```

### Available Commands

#### Frontend Scripts
```bash
npm run test:coverage          # Basic coverage generation
npm run test:coverage:watch    # Coverage with file watching
npm run test:coverage:ui       # Coverage with UI dashboard
npm run test:coverage:report   # Detailed coverage report
npm run ci:test               # Full CI pipeline (lint + coverage)
npm run ci:coverage-check     # Threshold enforcement
```

#### Project Scripts
```bash
./scripts/generate_coverage.sh  # Unified coverage generation
```

## Coverage Targets

### Current Status
- **Frontend**: 63.7% (requires improvement to reach 80%)
- **Backend**: To be measured with new infrastructure

### Target Requirements
- **Global minimum**: 80% across all metrics
- **Components**: 80% (standard UI coverage)
- **Hooks**: 85% (critical utility functions)
- **Utils**: 90% (pure utility functions)
- **Store/API**: 85% (business logic)

## File Exclusions

### Automatically Excluded
- **Test files**: `src/test/**`, `**/*.test.*`, `**/*.spec.*`
- **Build artifacts**: `dist/**`, `build/**`, `target/**`
- **Configuration**: `**/*.config.{js,ts}`, type definitions
- **Static assets**: `src/assets/**`, `src/styles/**`

### Type Files
- TypeScript definition files (`.d.ts`)
- Type-only exports and interfaces

## Reports & Dashboards

### Local Reports
- **HTML Reports**: Interactive browser-based coverage exploration
- **Console Output**: Real-time coverage metrics during test runs
- **JSON Summary**: Machine-readable coverage data

### CI/CD Reports
- **Codecov Dashboard**: Centralized coverage tracking and trends
- **GitHub Artifacts**: Downloadable coverage reports (30-day retention)
- **PR Comments**: Automated coverage change notifications

### Report Locations
```
Frontend: src/datafold_node/static-react/coverage/
├── index.html              # Main HTML report
├── lcov.info              # LCOV format for CI
└── coverage-summary.json  # JSON summary

Backend: target/
├── coverage-html/         # HTML reports
├── coverage.lcov          # LCOV format
└── llvm-cov/             # Raw coverage data
```

## Quality Enforcement

### Build Pipeline
1. **Linting**: Code quality checks
2. **Testing**: Full test suite execution
3. **Coverage**: Comprehensive coverage collection
4. **Threshold**: 80% minimum enforcement
5. **Reporting**: Multiple format generation
6. **Upload**: Codecov and artifact storage

### Failure Modes
- **Below 80%**: Build fails with detailed file-by-file analysis
- **Test failures**: No coverage calculation if tests fail
- **Missing files**: Coverage tracks all source files

## Maintenance

### Regular Tasks
- **Monthly reviews**: Coverage trend analysis
- **Threshold updates**: Adjust based on project maturity
- **Exclusion reviews**: Update patterns as codebase evolves
- **CI optimization**: Performance monitoring and improvements

### Documentation
- **Detailed guide**: [`src/datafold_node/static-react/COVERAGE.md`](src/datafold_node/static-react/COVERAGE.md)
- **Configuration**: All settings documented and explained
- **Troubleshooting**: Common issues and solutions

## Implementation Summary

UTC-1-10 successfully delivers:

### ✅ Complete Coverage Infrastructure
- Comprehensive coverage collection for both frontend and backend
- Automated threshold enforcement with 80% minimum requirement
- Multiple report formats for different use cases

### ✅ CI/CD Integration
- GitHub Actions workflow for automated coverage
- Codecov integration for trend tracking and PR comments
- Artifact storage for detailed analysis

### ✅ Developer Experience
- Simple commands for local coverage generation
- Clear feedback on coverage gaps
- Multiple viewing options (console, HTML, UI)

### ✅ Quality Assurance
- Build failures on insufficient coverage
- File-level threshold tracking
- Historical trend monitoring

The coverage infrastructure is now ready for production use and will enforce the 80% minimum coverage requirement across the entire datafold project.