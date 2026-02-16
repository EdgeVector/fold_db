import { describe, it, expect } from 'vitest'
import {
  isRangeSchema,
  getRangeKey,
  getNonRangeKeyFields,
  formatRangeQuery,
  validateRangeKey,
  getRangeSchemaInfo
} from '../rangeSchemaHelpers'

describe('rangeSchemaUtils', () => {
  describe('isRangeSchema', () => {
    it('should return false for null or undefined schema', () => {
      expect(isRangeSchema(null)).toBe(false)
      expect(isRangeSchema(undefined)).toBe(false)
    })

    it('should return true for schema with Range schema_type even without fields', () => {
      const schema = {
        name: 'TestSchema',
        schema_type: 'Range',
        key: { range_field: 'test_id' }
      }
      expect(isRangeSchema(schema)).toBe(true) // Backend schema_type is authoritative
    })

    it('should return true for schema with Range schema_type even with empty fields', () => {
      const schema = {
        name: 'TestSchema',
        schema_type: 'Range',
        key: { range_field: 'test_id' },
        fields: {}
      }
      expect(isRangeSchema(schema)).toBe(true) // Backend schema_type is authoritative
    })

    it('should return false for schema without range_key', () => {
      const schema = {
        name: 'TestSchema',
        fields: {
          field1: { field_type: 'Range' },
          field2: { field_type: 'Range' }
        }
      }
      expect(isRangeSchema(schema)).toBe(false)
    })

    it('should return true when schema_type is Range (backend is authoritative)', () => {
      const schema = {
        name: 'TestSchema',
        schema_type: 'Range',
        key: { range_field: 'test_id' },
        fields: {
          test_id: { field_type: 'Range' },
          field1: { field_type: 'Range' },
          field2: { field_type: 'Single' } // Mixed types OK - backend sets schema_type
        }
      }
      expect(isRangeSchema(schema)).toBe(true) // Backend is authoritative
    })

    it('should return true for valid range schema with new format', () => {
      const schema = {
        name: 'UserScores',
        schema_type: 'Range',
        key: { range_field: 'user_id' },
        fields: {
          user_id: { field_type: 'Range' },
          game_scores: { field_type: 'Range' },
          achievements: { field_type: 'Range' }
        }
      }
      expect(isRangeSchema(schema)).toBe(true)
    })

    it('should return false for old format without schema_type (backend is authoritative)', () => {
      const schema = {
        name: 'UserScores',
        range_key: 'user_id',
        fields: {
          user_id: { field_type: 'Range' },
          game_scores: { field_type: 'Range' },
          achievements: { field_type: 'Range' }
        }
      }
      // Without schema_type from backend, we can't determine if it's Range
      expect(isRangeSchema(schema)).toBe(false)
    })

    it('should prioritize new format over old format when both are present', () => {
      const schema = {
        name: 'UserScores',
        range_key: 'old_key', // Old format
        schema_type: 'Range',
        key: { range_field: 'new_key' }, // New format
        fields: {
          new_key: { field_type: 'Range' },
          data: { field_type: 'Range' }
        }
      }
      expect(isRangeSchema(schema)).toBe(true)
    })
  })

  describe('getRangeKey', () => {
    it('should return null for null or undefined schema', () => {
      expect(getRangeKey(null)).toBe(null)
      expect(getRangeKey(undefined)).toBe(null)
    })

    it('should return range_key from new format', () => {
      const schema = {
        name: 'TestSchema',
        schema_type: 'Range',
        key: { range_field: 'test_id' }
      }
      expect(getRangeKey(schema)).toBe('test_id')
    })

    it('should return null for old format (no longer supported)', () => {
      const schema = {
        name: 'TestSchema',
        range_key: 'test_id'
      }
      // Old top-level range_key format is no longer supported
      expect(getRangeKey(schema)).toBe(null)
    })

    it('should prioritize new format over old format', () => {
      const schema = {
        name: 'TestSchema',
        range_key: 'old_key',
        schema_type: 'Range',
        key: { range_field: 'new_key' }
      }
      expect(getRangeKey(schema)).toBe('new_key')
    })

    it('should return null when no range_key is found', () => {
      const schema = {
        name: 'TestSchema',
        schema_type: { Single: {} }
      }
      expect(getRangeKey(schema)).toBe(null)
    })
  })

  describe('getNonRangeKeyFields', () => {
    it('should return empty object for non-range schema', () => {
      const schema = {
        name: 'TestSchema',
        fields: ['field1', 'field2']
      }
      expect(getNonRangeKeyFields(schema)).toEqual({})
    })

    it('should return all fields except range_key for range schema', () => {
      const schema = {
        name: 'UserScores',
        schema_type: 'Range',
        key: { range_field: 'user_id' },
        fields: ['user_id', 'game_scores', 'achievements']
      }
      const result = getNonRangeKeyFields(schema)
      expect(result).toEqual({
        game_scores: {},
        achievements: {}
      })
      expect(result).not.toHaveProperty('user_id')
    })

    it('should handle case where range_key field does not exist in fields', () => {
      const schema = {
        name: 'UserScores',
        schema_type: 'Range',
        key: { range_field: 'missing_key' },
        fields: ['user_id', 'game_scores']
      }
      const result = getNonRangeKeyFields(schema)
      expect(result).toEqual({
        user_id: {},
        game_scores: {}
      })
    })
  })

  describe('formatRangeQuery', () => {
    const schema = {
      name: 'UserScores',
      schema_type: 'Range',
        key: { range_field: 'user_id' }
    }

    it('should format basic query without range filter', () => {
      const fields = ['game_scores', 'achievements']
      const result = formatRangeQuery(schema, fields, '')
      
      expect(result).toEqual({
        type: 'query',
        schema: 'UserScores',
        fields: ['game_scores', 'achievements']
      })
    })

    it('should format query with range filter', () => {
      const fields = ['game_scores', 'achievements']
      const rangeFilterValue = 'user123'
      const result = formatRangeQuery(schema, fields, rangeFilterValue)
      
      expect(result).toEqual({
        type: 'query',
        schema: 'UserScores',
        fields: ['game_scores', 'achievements'],
        filter: { RangeKey: 'user123' }
      })
    })

    it('should trim whitespace from range filter value', () => {
      const fields = ['game_scores']
      const rangeFilterValue = '  user123  '
      const result = formatRangeQuery(schema, fields, rangeFilterValue)

      expect(result.filter).toEqual({ RangeKey: 'user123' })
    })

    it('should not include range_filter for empty string', () => {
      const fields = ['game_scores']
      const result = formatRangeQuery(schema, fields, '')
      
      expect(result).not.toHaveProperty('range_filter')
    })

    it('should not include range_filter for whitespace-only string', () => {
      const fields = ['game_scores']
      const result = formatRangeQuery(schema, fields, '   ')
      
      expect(result).not.toHaveProperty('range_filter')
    })
  })


  describe('validateRangeKey', () => {
    it('should return null for valid string when required', () => {
      expect(validateRangeKey('user123', true)).toBe(null)
    })

    it('should return error for empty string when required', () => {
      expect(validateRangeKey('', true)).toBe('Range key is required')
      expect(validateRangeKey(null, true)).toBe('Range key is required')
      expect(validateRangeKey(undefined, true)).toBe('Range key is required')
    })

    it('should return null for empty string when not required', () => {
      expect(validateRangeKey('', false)).toBe(null)
      expect(validateRangeKey(null, false)).toBe(null)
      expect(validateRangeKey(undefined, false)).toBe(null)
    })

    it('should allow whitespace strings (backend validates)', () => {
      // Backend is authoritative - it will validate whitespace
      expect(validateRangeKey('   ', true)).toBe(null)
    })
  })

  describe('getRangeSchemaInfo', () => {
    it('should return null for non-range schema', () => {
      const schema = {
        name: 'TestSchema',
        fields: {
          field1: { field_type: 'Single' }
        }
      }
      expect(getRangeSchemaInfo(schema)).toBe(null)
    })

    it('should return comprehensive info for range schema', () => {
      const schema = {
        name: 'UserScores',
        schema_type: 'Range',
        key: { range_field: 'user_id' },
        fields: ['user_id', 'game_scores', 'achievements']
      }
      
      const result = getRangeSchemaInfo(schema)
      
      expect(result).toEqual({
        isRangeSchema: true,
        rangeKey: 'user_id',
        rangeFields: [],  // getRangeFields no longer applies to declarative schemas
        nonRangeKeyFields: {
          game_scores: {},
          achievements: {}
        },
        totalFields: 3
      })
    })

    it('should handle schema with mixed field types', () => {
      const schema = {
        name: 'MixedSchema',
        range_key: 'key_field',
        fields: {
          key_field: { field_type: 'Range' },
          range_field: { field_type: 'Range' },
          single_field: { field_type: 'Single' }
        }
      }
      
      // This should return null because not all fields are Range type
      expect(getRangeSchemaInfo(schema)).toBe(null)
    })
  })

  describe('Edge Cases and Integration', () => {
    it('should handle schema with malformed schema_type', () => {
      const schema = {
        name: 'MalformedSchema',
        schema_type: { InvalidType: { some_key: 'value' } },
        fields: {
          field1: { field_type: 'Range' }
        }
      }
      
      expect(isRangeSchema(schema)).toBe(false)
      expect(getRangeKey(schema)).toBe(null)
    })

    it('should handle schema with both valid new and old formats', () => {
      const schema = {
        name: 'HybridSchema',
        range_key: 'old_range_key',
        schema_type: 'Range',
        key: { range_field: 'new_range_key' },
        fields: {
          new_range_key: { field_type: 'Range' },
          data_field: { field_type: 'Range' }
        }
      }
      
      expect(isRangeSchema(schema)).toBe(true)
      expect(getRangeKey(schema)).toBe('new_range_key') // Should prefer new format
    })

    it('should work with real declarative schema structure', () => {
      const blogPostSchema = {
        name: 'BlogPost',
        schema_type: 'Range',
        key: { range_field: 'publish_date' },
        fields: ['author', 'content', 'publish_date', 'tags', 'title']
      }
      
      expect(isRangeSchema(blogPostSchema)).toBe(true)
      expect(getRangeKey(blogPostSchema)).toBe('publish_date')
      
      const nonRangeKeyFields = getNonRangeKeyFields(blogPostSchema)
      expect(nonRangeKeyFields).toHaveProperty('author')
      expect(nonRangeKeyFields).toHaveProperty('content')
      expect(nonRangeKeyFields).not.toHaveProperty('publish_date')
      
      const query = formatRangeQuery(blogPostSchema, ['title', 'author'], '2024-01-01')
      expect(query).toEqual({
        type: 'query',
        schema: 'BlogPost',
        fields: ['title', 'author'],
        filter: { RangeKey: '2024-01-01' }
      })
    })
  })
})
