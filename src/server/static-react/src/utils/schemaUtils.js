/**
 * Shared schema utilities used across multiple components.
 *
 * Consolidates duplicated logic for:
 * - Schema display names
 * - Field name extraction
 * - Set toggling
 * - Error message conversion
 * - Query-by-key record fetching
 */

/**
 * Get the human-readable display name for a schema.
 * Prefers descriptive_name, falls back to the identity-hash name.
 */
export function getSchemaDisplayName(schema) {
  return schema?.descriptive_name || schema?.name || ''
}

/**
 * Build a schema options array for SelectField dropdowns.
 * @param {Object[]} schemas - Array of schema objects
 * @returns {{ value: string, label: string }[]}
 */
export function buildSchemaOptions(schemas) {
  return (schemas || []).map(schema => ({
    value: schema.name,
    label: getSchemaDisplayName(schema),
  }))
}

/**
 * Extract field names from a schema object.
 * Handles both array-of-strings and object-with-field-keys formats.
 */
export function getFieldNames(schemaObj) {
  if (!schemaObj) return []
  const f = schemaObj.fields || schemaObj.transform_fields
  if (Array.isArray(f)) return f.slice()
  if (f && typeof f === 'object') return Object.keys(f)
  return []
}

/**
 * Toggle a value in a Set (immutable — returns a new Set).
 */
export function toggleSetItem(set, item) {
  const next = new Set(set)
  if (next.has(item)) next.delete(item)
  else next.add(item)
  return next
}

/**
 * Convert an unknown error value to a string message.
 */
export function toErrorMessage(error) {
  if (error instanceof Error) return error.message
  return String(error)
}

/**
 * Truncate a hash string for display.
 * @param {string} name - The hash/name string
 * @param {number} [threshold=16] - Show full string if shorter than this
 * @param {number} [keep=12] - Number of leading characters to keep
 * @returns {string}
 */
export function truncateHash(name, threshold = 16, keep = 12) {
  if (typeof name !== 'string') return name
  if (name.length > threshold) return name.slice(0, keep) + '...'
  return name
}
