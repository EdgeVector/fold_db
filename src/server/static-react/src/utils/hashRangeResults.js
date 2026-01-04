/**
 * Utilities for detecting and working with hash->range->fields shaped results.
 *
 * Expected shape:
 * {
 *   [hashKey]: {
 *     [rangeKey]: { fieldName: value, ... }
 *   },
 *   ...
 * }
 */

function isPlainObject(value) {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}

/**
 * Detect whether the provided data matches the hash->range->fields shape.
 * Performs a shallow, sample-based validation to keep it fast for large datasets.
 * @param {unknown} maybeData - Either entire results object or results.data
 * @returns {boolean}
 */
export function isHashRangeFieldsShape(maybeData) {
  const data = extractData(maybeData);
  if (!isPlainObject(data)) return false;

  // Sample up to 3 hash entries
  const hashKeys = Object.keys(data);
  if (hashKeys.length === 0) return false;

  for (let i = 0; i < Math.min(3, hashKeys.length); i++) {
    const hashVal = data[hashKeys[i]];
    if (!isPlainObject(hashVal)) return false;

    const rangeKeys = Object.keys(hashVal);
    if (rangeKeys.length === 0) {
      // Allow empty hash buckets
      continue;
    }

    for (let j = 0; j < Math.min(3, rangeKeys.length); j++) {
      const fieldsVal = hashVal[rangeKeys[j]];
      if (!isPlainObject(fieldsVal)) return false;
      // Spot check field map keys exist
      const fieldNames = Object.keys(fieldsVal);
      if (fieldNames.length === 0) {
        // Allow empty field maps
        continue;
      }
      // No further deep type checks to keep it permissive
    }
  }

  return true;
}

/**
 * Extracts the underlying data object whether caller passed results or results.data
 * @param {unknown} maybeData
 * @returns {any}
 */
export function extractData(maybeData) {
  if (maybeData && isPlainObject(maybeData) && Object.prototype.hasOwnProperty.call(maybeData, 'data')) {
    // @ts-ignore
    return maybeData.data;
  }
  return maybeData;
}

/**
 * Counts the total number of hashes and ranges across the dataset.
 * @param {object} data
 */
export function summarizeCounts(maybeData) {
  const data = extractData(maybeData) || {};
  if (!isPlainObject(data)) return { hashes: 0, ranges: 0 };
  const hashes = Object.keys(data).length;
  let ranges = 0;
  for (const hashKey of Object.keys(data)) {
    const rangeObj = data[hashKey];
    if (isPlainObject(rangeObj)) {
      ranges += Object.keys(rangeObj).length;
    }
  }
  return { hashes, ranges };
}

/**
 * Returns a sorted list of hash keys for stable rendering.
 */
export function getSortedHashKeys(maybeData) {
  const data = extractData(maybeData) || {};
  if (!isPlainObject(data)) return [];
  return Object.keys(data).sort(naturalComparator);
}

/**
 * Returns a sorted list of range keys under a given hash key.
 */
export function getSortedRangeKeys(maybeData, hashKey) {
  const data = extractData(maybeData) || {};
  const ranges = isPlainObject(data) && isPlainObject(data[hashKey]) ? data[hashKey] : {};
  return Object.keys(ranges).sort(naturalComparator);
}

/**
 * Natural-ish comparator for stable ordering of mixed numeric/string keys.
 */
function naturalComparator(a, b) {
  const an = toNumberOrNaN(a);
  const bn = toNumberOrNaN(b);
  if (!Number.isNaN(an) && !Number.isNaN(bn)) {
    return an - bn;
  }
  return String(a).localeCompare(String(b));
}

function toNumberOrNaN(v) {
  const n = Number(v);
  return Number.isFinite(n) ? n : Number.NaN;
}

/**
 * Safely get fields object at given hash and range.
 */
export function getFieldsAt(maybeData, hashKey, rangeKey) {
  const data = extractData(maybeData) || {};
  if (!isPlainObject(data)) return null;
  const rangeObj = data[hashKey];
  if (!isPlainObject(rangeObj)) return null;
  const fields = rangeObj[rangeKey];
  return isPlainObject(fields) ? fields : null;
}

/**
 * Slice helpers for lazy rendering.
 */
export function sliceKeys(keys, start, count) {
  return keys.slice(start, Math.min(start + count, keys.length));
}

export default {
  isHashRangeFieldsShape,
  extractData,
  summarizeCounts,
  getSortedHashKeys,
  getSortedRangeKeys,
  getFieldsAt,
  sliceKeys
};


