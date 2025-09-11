/**
 * Utility functions for handling HashRange schemas
 */

/**
 * Detects if a schema is a HashRange schema
 * HashRange schemas have:
 * 1. schema_type: "HashRange"
 * 2. A key field with hash_field and range_field defined
 */
export function isHashRangeSchema(schema) {
  // Enhanced HashRange schema detection with better validation
  if (!schema || typeof schema !== 'object') {
    return false
  }
  
  // Check for HashRange schema type
  if (schema.schema_type !== 'HashRange') {
    return false
  }
  
  // Check for key field with hash_field and range_field
  if (!schema.key || typeof schema.key !== 'object') {
    return false
  }
  
  const hasHashField = schema.key.hash_field && typeof schema.key.hash_field === 'string'
  const hasRangeField = schema.key.range_field && typeof schema.key.range_field === 'string'
  
  if (!hasHashField || !hasRangeField) {
    return false
  }
  
  return true
}

/**
 * Gets the hash field name for a HashRange schema
 */
export function getHashField(schema) {
  if (!isHashRangeSchema(schema)) {
    return null
  }
  
  return schema.key?.hash_field || null
}

/**
 * Gets the range field name for a HashRange schema
 */
export function getRangeField(schema) {
  if (!isHashRangeSchema(schema)) {
    return null
  }
  
  return schema.key?.range_field || null
}

/**
 * Gets HashRange schema display information
 */
export function getHashRangeSchemaInfo(schema) {
  if (!isHashRangeSchema(schema)) {
    return null
  }
  
  return {
    isHashRangeSchema: true,
    hashField: getHashField(schema),
    rangeField: getRangeField(schema),
    totalFields: Object.keys(schema.fields || {}).length
  }
}

/**
 * Validates hash_key and range_key for HashRange schema mutations
 */
export function validateHashRangeKeysForMutation(hashKeyValue, rangeKeyValue, isRequired = true) {
  const errors = []
  
  // Check hash key
  if (isRequired && (!hashKeyValue || !hashKeyValue.trim())) {
    errors.push('Hash key is required for HashRange schema mutations')
  }
  
  // Check range key
  if (isRequired && (!rangeKeyValue || !rangeKeyValue.trim())) {
    errors.push('Range key is required for HashRange schema mutations')
  }
  
  return errors.length > 0 ? errors.join(', ') : null
}

/**
 * Formats a HashRange schema mutation with proper hash_key and range_key
 */
export function formatHashRangeSchemaMutation(schema, mutationType, hashKeyValue, rangeKeyValue, fieldData) {
  const mutation = {
    type: 'mutation',
    schema: schema.name,
    mutation_type: mutationType.toLowerCase()
  }
  
  if (mutationType === 'Delete') {
    mutation.data = {}
    // For delete operations, use hash_key and range_key
    if (hashKeyValue && hashKeyValue.trim()) {
      mutation.data.hash_key = hashKeyValue.trim()
    }
    if (rangeKeyValue && rangeKeyValue.trim()) {
      mutation.data.range_key = rangeKeyValue.trim()
    }
  } else {
    const data = {}
    
    // Add hash_key and range_key
    if (hashKeyValue && hashKeyValue.trim()) {
      data.hash_key = hashKeyValue.trim()
    }
    if (rangeKeyValue && rangeKeyValue.trim()) {
      data.range_key = rangeKeyValue.trim()
    }
    
    // Add other field data
    Object.entries(fieldData).forEach(([fieldName, fieldValue]) => {
      if (fieldName !== 'hash_key' && fieldName !== 'range_key') {
        data[fieldName] = fieldValue
      }
    })
    
    mutation.data = data
  }
  
  return mutation
}
