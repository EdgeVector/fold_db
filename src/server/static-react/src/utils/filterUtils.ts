/**
 * Filter Utilities - Type-Safe HashRangeFilter Creation
 * 
 * This module provides type-safe utilities for creating HashRangeFilter objects
 * that match exactly what the Rust backend expects. All filter creation goes
 * through these utilities to ensure no ambiguity or interpretation issues.
 */

import type { HashRangeFilter } from '@generated/generated';

/**
 * Creates a HashKey filter for hash key matches (returns all range keys for that hash)
 */
export function createHashKeyFilter(hashKey: string): HashRangeFilter {
  return { HashKey: hashKey };
}

/**
 * Creates a RangeKey filter for exact range key matches
 */
export function createRangeKeyFilter(rangeKey: string): HashRangeFilter {
  return { RangeKey: rangeKey };
}

/**
 * Creates a RangePrefix filter for range key prefix matches
 */
export function createRangePrefixFilter(prefix: string): HashRangeFilter {
  return { RangePrefix: prefix };
}

/**
 * Creates a RangeRange filter for range key range matches
 */
export function createRangeRangeFilter(start: string, end: string): HashRangeFilter {
  return { RangeRange: { start, end } };
}

/**
 * Creates a HashRangeKey filter for exact hash and range key matches
 */
export function createHashRangeKeyFilter(hash: string, range: string): HashRangeFilter {
  return { HashRangeKey: { hash, range } };
}

/**
 * Creates a HashRangePrefix filter for range key prefix within a specific hash group
 */
export function createHashRangePrefixFilter(hash: string, prefix: string): HashRangeFilter {
  return { HashRangePrefix: { hash, prefix } };
}

/**
 * Creates a HashRangeRange filter for range key range within a specific hash group
 */
export function createHashRangeRangeFilter(hash: string, start: string, end: string): HashRangeFilter {
  return { HashRangeRange: { hash, start, end } };
}

/**
 * Creates a SampleN filter for limiting results
 */
export function createSampleNFilter(count: number): HashRangeFilter {
  return { SampleN: count };
}

/**
 * Creates a HashRangeKeys filter for multiple hash-range key pairs
 */
export function createHashRangeKeysFilter(keys: Array<[string, string]>): HashRangeFilter {
  return { HashRangeKeys: keys };
}

/**
 * Creates a HashRangePattern filter for range key pattern within a specific hash group
 */
export function createHashRangePatternFilter(hash: string, pattern: string): HashRangeFilter {
  return { HashRangePattern: { hash, pattern } };
}

/**
 * Creates a RangePattern filter for range key pattern across all hash groups
 */
export function createRangePatternFilter(pattern: string): HashRangeFilter {
  return { RangePattern: pattern };
}

/**
 * Creates a HashPattern filter for hash key pattern matching
 */
export function createHashPatternFilter(pattern: string): HashRangeFilter {
  return { HashPattern: pattern };
}

/**
 * Creates a HashRange filter for hash range matching
 */
export function createHashRangeFilter(start: string, end: string): HashRangeFilter {
  return { HashRange: { start, end } };
}

/**
 * Type-safe filter creation from UI input
 */
export interface RangeFilterInput {
  key?: string;
  keyPrefix?: string;
  start?: string;
  end?: string;
}

/**
 * Converts UI range filter input to proper HashRangeFilter
 */
export function createFilterFromRangeInput(input: RangeFilterInput): HashRangeFilter | null {
  if (input.key) {
    return createRangeKeyFilter(input.key);
  } else if (input.keyPrefix) {
    return createRangePrefixFilter(input.keyPrefix);
  } else if (input.start && input.end) {
    return createRangeRangeFilter(input.start, input.end);
  }
  return null;
}

/**
 * Type-safe filter creation from hash range input
 */
export interface HashRangeFilterInput {
  hash?: string;
  range?: string;
  prefix?: string;
  start?: string;
  end?: string;
  pattern?: string;
}

/**
 * Converts UI hash range filter input to proper HashRangeFilter
 */
export function createFilterFromHashRangeInput(input: HashRangeFilterInput): HashRangeFilter | null {
  if (input.hash && input.range) {
    return createHashRangeKeyFilter(input.hash, input.range);
  } else if (input.hash && input.prefix) {
    return createHashRangePrefixFilter(input.hash, input.prefix);
  } else if (input.hash && input.start && input.end) {
    return createHashRangeRangeFilter(input.hash, input.start, input.end);
  } else if (input.hash && input.pattern) {
    return createHashRangePatternFilter(input.hash, input.pattern);
  } else if (input.hash) {
    return createHashKeyFilter(input.hash);
  } else if (input.range) {
    return createRangeKeyFilter(input.range);
  } else if (input.prefix) {
    return createRangePrefixFilter(input.prefix);
  } else if (input.start && input.end) {
    return createRangeRangeFilter(input.start, input.end);
  } else if (input.pattern) {
    return createRangePatternFilter(input.pattern);
  }
  return null;
}
