#!/usr/bin/env node

/**
 * Coverage threshold checker for CI/CD pipeline
 * Reads coverage summary and enforces minimum thresholds
 */

import fs from 'fs';

const COVERAGE_FILE = './coverage/coverage-summary.json';
const MIN_COVERAGE = 80;

// Color codes for console output
const colors = {
  red: '\x1b[31m',
  green: '\x1b[32m',
  yellow: '\x1b[33m',
  blue: '\x1b[34m',
  reset: '\x1b[0m',
  bold: '\x1b[1m'
};

function log(message, color = colors.reset) {
  console.log(`${color}${message}${colors.reset}`);
}

function checkCoverageFile() {
  if (!fs.existsSync(COVERAGE_FILE)) {
    log(`❌ Coverage file not found: ${COVERAGE_FILE}`, colors.red);
    log('Run "npm run test:coverage:threshold" first to generate coverage data.', colors.yellow);
    process.exit(1);
  }
}

function readCoverageData() {
  try {
    const coverageData = JSON.parse(fs.readFileSync(COVERAGE_FILE, 'utf8'));
    return coverageData;
  } catch (error) {
    log(`❌ Error reading coverage file: ${error.message}`, colors.red);
    process.exit(1);
  }
}

function formatPercentage(value) {
  return `${value.toFixed(2)}%`;
}

function checkThreshold(actual, threshold, name) {
  const status = actual >= threshold;
  const symbol = status ? '✅' : '❌';
  const color = status ? colors.green : colors.red;
  
  log(`${symbol} ${name}: ${formatPercentage(actual)} (min: ${formatPercentage(threshold)})`, color);
  return status;
}

function analyzeCoverage(coverageData) {
  log(`\n${colors.bold}📊 Coverage Analysis${colors.reset}`);
  log('═'.repeat(50));

  const total = coverageData.total;
  const metrics = ['lines', 'functions', 'statements', 'branches'];
  
  let allPassed = true;
  
  // Check global coverage
  log(`\n${colors.blue}Global Coverage:${colors.reset}`);
  for (const metric of metrics) {
    const pct = total[metric].pct;
    const passed = checkThreshold(pct, MIN_COVERAGE, metric.charAt(0).toUpperCase() + metric.slice(1));
    if (!passed) allPassed = false;
  }

  // Detailed file-by-file analysis if any files are below threshold
  const problematicFiles = [];
  
  for (const [filePath, fileData] of Object.entries(coverageData)) {
    if (filePath === 'total') continue;
    
    const hasLowCoverage = metrics.some(metric => fileData[metric].pct < MIN_COVERAGE);
    if (hasLowCoverage) {
      problematicFiles.push({ path: filePath, data: fileData });
    }
  }

  if (problematicFiles.length > 0) {
    log(`\n${colors.yellow}Files Below Threshold:${colors.reset}`);
    log('-'.repeat(50));
    
    for (const file of problematicFiles) {
      log(`\n📄 ${file.path}`, colors.yellow);
      for (const metric of metrics) {
        const pct = file.data[metric].pct;
        if (pct < MIN_COVERAGE) {
          checkThreshold(pct, MIN_COVERAGE, `  ${metric}`);
        }
      }
    }
  }

  // Coverage summary
  log(`\n${colors.bold}Summary:${colors.reset}`);
  log('═'.repeat(50));
  
  const totalFiles = Object.keys(coverageData).length - 1; // Exclude 'total'
  const filesBelowThreshold = problematicFiles.length;
  const filesPassingThreshold = totalFiles - filesBelowThreshold;
  
  log(`📁 Total files: ${totalFiles}`);
  log(`✅ Files passing (≥${MIN_COVERAGE}%): ${filesPassingThreshold}`, colors.green);
  log(`❌ Files below threshold: ${filesBelowThreshold}`, filesBelowThreshold > 0 ? colors.red : colors.green);
  
  // Overall result
  if (allPassed) {
    log(`\n🎉 All coverage thresholds met! Minimum ${MIN_COVERAGE}% achieved.`, colors.green);
    return true;
  } else {
    log(`\n💥 Coverage thresholds not met. Minimum ${MIN_COVERAGE}% required.`, colors.red);
    log('\nTo improve coverage:', colors.yellow);
    log('1. Add more unit tests for uncovered lines', colors.yellow);
    log('2. Test edge cases and error handling', colors.yellow);
    log('3. Remove dead code or mark as coverage exclusions', colors.yellow);
    log('4. Review and test complex branching logic', colors.yellow);
    return false;
  }
}

function main() {
  log(`${colors.bold}🧪 Coverage Threshold Checker${colors.reset}`);
  log(`Minimum required coverage: ${MIN_COVERAGE}%\n`);

  checkCoverageFile();
  const coverageData = readCoverageData();
  const passed = analyzeCoverage(coverageData);

  process.exit(passed ? 0 : 1);
}

main();