export function getSchemaDependencies(schemas) {
  const deps = {}
  schemas.forEach(schema => {
    deps[schema.name] = new Map()
  })

  schemas.forEach(schema => {
    // Declarative schemas: dependencies come from transform_fields
    if (schema.transform_fields && typeof schema.transform_fields === 'object') {
      Object.values(schema.transform_fields).forEach(expression => {
        // Extract schema name from transform expression like "BlogPost.map().content"
        const match = expression.match(/^(\w+)\./)
        if (match && match[1] && match[1] !== schema.name) {
          const sourceSchema = match[1]
          if (!deps[schema.name].has(sourceSchema)) {
            deps[schema.name].set(sourceSchema, new Set())
          }
          deps[schema.name].get(sourceSchema).add('transform')
        }
      })
    }
  })

  return Object.fromEntries(
    Object.entries(deps).map(([schema, map]) => [
      schema,
      Array.from(map.entries()).map(([depSchema, types]) => ({
        schema: depSchema,
        types: Array.from(types)
      }))
    ])
  )
}

export function getDependencyGraph(schemas) {
  const deps = getSchemaDependencies(schemas)
  const nodes = schemas.map(s => s.name)
  const edges = []

  Object.entries(deps).forEach(([target, arr]) => {
    arr.forEach(dep => {
      dep.types.forEach(type => {
        if (nodes.includes(dep.schema) && nodes.includes(target)) {
          edges.push({ source: dep.schema, target, type })
        }
      })
    })
  })

  return { nodes, edges }
}
