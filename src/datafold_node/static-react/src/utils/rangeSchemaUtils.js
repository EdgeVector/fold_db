/**
 * Utility functions for handling range schemas
 */

/**
 * Detects if a schema is a range schema
 * Range schemas have:
 * 1. A range_key field defined in the schema
 * 2. All fields have field_type: "Range"
 */
export function isRangeSchema(schema) {
  // Enhanced range schema detection with better validation
  if (!schema || typeof schema !== 'object') {
    return false
  }
  
  // Check for range_key in the new schema_type structure or old format
  const hasRangeKey = schema.schema_type?.Range?.range_key || schema.range_key
  if (!hasRangeKey || typeof hasRangeKey !== 'string') {
    return false
  }
  
  if (!schema.fields || typeof schema.fields !== 'object') {
    return false
  }
  
  // Check if all fields have field_type: "Range"
  const fieldEntries = Object.entries(schema.fields)
  if (fieldEntries.length === 0) {
    return false
  }
  
  // More robust field type checking
  const allFieldsAreRange = fieldEntries.every(([fieldName, field]) => {
    if (!field || typeof field !== 'object') {
      console.warn(`Field ${fieldName} is not a valid field object in schema ${schema.name}`)
      return false
    }
    
    if (field.field_type !== 'Range') {
      console.warn(`Field ${fieldName} has field_type "${field.field_type}", expected "Range" in schema ${schema.name}`)
      return false
    }
    
    return true
  })
  
  return allFieldsAreRange
}

/**
 * Gets the range key field name for a range schema
 */
export function getRangeKey(schema) {
  // Check new schema_type structure first, then fall back to old format
  return schema?.schema_type?.Range?.range_key || schema?.range_key || null
}

/**
 * Gets all non-range-key fields for a range schema
 */
export function getNonRangeKeyFields(schema) {
  if (!isRangeSchema(schema)) {
    return {}
  }
  
  const rangeKey = getRangeKey(schema)
  const fields = { ...schema.fields }
  
  // Remove the range key field from the list
  if (rangeKey && fields[rangeKey]) {
    delete fields[rangeKey]
  }
  
  return fields
}

/**
 * Formats a range schema query with proper range_filter
 */
export function formatRangeSchemaQuery(schema, fields, rangeFilterValue) {
  const query = {
    type: 'query',
    schema: schema.name,
    fields: fields
  }
  
  if (rangeFilterValue && rangeFilterValue.trim()) {
    query.range_filter = { Key: rangeFilterValue.trim() }
  }
  
  return query
}


/**
 * Validates range_key for range schema mutations
 */
export function validateRangeKeyForMutation(rangeKeyValue, isRequired = true) {
  // First check for whitespace-only strings specifically
  if (rangeKeyValue && typeof rangeKeyValue === 'string' && rangeKeyValue.length > 0 && rangeKeyValue.trim().length === 0) {
    return 'Range key cannot be empty'
  }
  
  // Then check for required but missing/empty
  if (isRequired && (!rangeKeyValue || !rangeKeyValue.trim())) {
    return 'Range key is required for range schema mutations'
  }
  
  return null
}

/**
 * Enhanced range schema mutation formatter with better validation
 * Range schemas require non-range_key fields to be JSON objects
 */
export function formatEnhancedRangeSchemaMutation(schema, mutationType, rangeKeyValue, fieldData) {
  const mutation = {
    type: 'mutation',
    schema: schema.name,
    mutation_type: mutationType.toLowerCase()
  }
  
  // Get the actual range key field name from the schema
  const rangeKeyFieldName = getRangeKey(schema)
  
  if (mutationType === 'Delete') {
    mutation.data = {}
    // For delete operations, use the actual range key field name
    if (rangeKeyValue && rangeKeyValue.trim() && rangeKeyFieldName) {
      mutation.data[rangeKeyFieldName] = rangeKeyValue.trim()
    }
  } else {
    const data = {}
    
    // Add range key using the actual field name from schema (as primitive value)
    if (rangeKeyValue && rangeKeyValue.trim() && rangeKeyFieldName) {
      data[rangeKeyFieldName] = rangeKeyValue.trim()
    }
    
    // Format non-range_key fields as JSON objects for range schemas
    // The backend expects non-range_key fields to be objects so it can inject the range_key
    Object.entries(fieldData).forEach(([fieldName, fieldValue]) => {
      if (fieldName !== rangeKeyFieldName) {
        // Convert simple values to JSON objects with a 'value' key
        if (typeof fieldValue === 'string' || typeof fieldValue === 'number' || typeof fieldValue === 'boolean') {
          data[fieldName] = { value: fieldValue }
        } else if (typeof fieldValue === 'object' && fieldValue !== null) {
          // If already an object, use as-is
          data[fieldName] = fieldValue
        } else {
          // For other types, wrap in an object
          data[fieldName] = { value: fieldValue }
        }
      }
    })
    
    mutation.data = data
  }
  
  return mutation
}

/**
 * Gets range schema display information
 */
export function getRangeSchemaInfo(schema) {
  if (!isRangeSchema(schema)) {
    return null
  }
  
  return {
    isRangeSchema: true,
    rangeKey: getRangeKey(schema),
    rangeFields: Object.entries(schema.fields || {}).filter(([_, field]) => field.field_type === 'Range'),
    nonRangeKeyFields: getNonRangeKeyFields(schema),
    totalFields: Object.keys(schema.fields || {}).length
  }
}