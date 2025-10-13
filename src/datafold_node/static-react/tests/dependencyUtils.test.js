import { describe, it, expect } from 'vitest'
import { getSchemaDependencies } from '../src/utils/dependencyUtils.js'

describe('dependencyUtils', () => {
  describe('getSchemaDependencies', () => {
    it('computes dependencies with types', () => {
      const schemas = [
        { name: 'BlogPost', fields: ['title', 'content', 'author'] },
        { name: 'BlogPostWordIndex', transform_fields: {
          word: 'BlogPost.map().content.split_by_word().map()',
          title: 'BlogPost.map().title'
        }},
        { name: 'BlogPostAuthorIndex', transform_fields: {
          author: 'BlogPost.map().author'
        }}
      ]
      const deps = getSchemaDependencies(schemas)
      
      expect(deps.BlogPost).toEqual([])
      expect(deps.BlogPostWordIndex).toEqual([
        { schema: 'BlogPost', types: ['transform'] }
      ])
      expect(deps.BlogPostAuthorIndex).toEqual([
        { schema: 'BlogPost', types: ['transform'] }
      ])
    })

    it('handles empty schemas array', () => {
      const deps = getSchemaDependencies([])
      expect(deps).toEqual({})
    })

    it('handles schemas with no dependencies', () => {
      const schemas = [
        { name: 'User', fields: ['id', 'name', 'email'] },
        { name: 'Product', fields: ['id', 'name', 'price'] }
      ]
      const deps = getSchemaDependencies(schemas)
      
      expect(deps.User).toEqual([])
      expect(deps.Product).toEqual([])
    })

    it('handles complex dependency chains', () => {
      const schemas = [
        { name: 'User', fields: ['id', 'name'] },
        { name: 'UserIndex', transform_fields: { name: 'User.map().name' } },
        { name: 'UserStats', transform_fields: { 
          user_name: 'UserIndex.map().name',
          user_id: 'User.map().id'
        }}
      ]
      const deps = getSchemaDependencies(schemas)
      
      expect(deps.User).toEqual([])
      expect(deps.UserIndex).toEqual([{ schema: 'User', types: ['transform'] }])
      expect(deps.UserStats.sort((a, b) => a.schema.localeCompare(b.schema))).toEqual([
        { schema: 'User', types: ['transform'] },
        { schema: 'UserIndex', types: ['transform'] }
      ])
    })

    it('handles multiple transform fields from same source', () => {
      const schemas = [
        { name: 'BlogPost', fields: ['title', 'content', 'author'] },
        { name: 'BlogPostIndex', transform_fields: {
          word: 'BlogPost.map().content.split_by_word().map()',
          author: 'BlogPost.map().author',
          title: 'BlogPost.map().title'
        }}
      ]
      const deps = getSchemaDependencies(schemas)
      
      expect(deps.BlogPost).toEqual([])
      expect(deps.BlogPostIndex).toEqual([{ schema: 'BlogPost', types: ['transform'] }])
    })

    it('handles schemas with no transform_fields', () => {
      const schemas = [
        { name: 'User', fields: ['id', 'name'] },
        { name: 'Product', fields: ['id', 'price'] }
      ]
      const deps = getSchemaDependencies(schemas)
      
      expect(deps.User).toEqual([])
      expect(deps.Product).toEqual([])
    })
  })
})
