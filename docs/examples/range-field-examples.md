# Range Field Examples

This directory contains comprehensive examples demonstrating the use of Range fields in DataFold schemas, including schema definition, data mutations, and various query patterns with range filters.

## Overview

Range fields are a powerful feature in DataFold that allow you to store key-value pairs within a single field, enabling efficient querying and filtering based on keys, key patterns, value matching, and range operations.

## Files

### 1. Schema Definition
- **`product_catalog_schema.json`** - Defines a ProductCatalog schema with two Range fields:
  - `inventory_by_location`: Stores inventory counts by warehouse/store location
  - `attributes`: Stores product specifications and metadata

### 2. Sample Data
- **`product_catalog_mutations.json`** - Contains mutations to create sample products with range field data

### 3. Query Examples
- **`product_catalog_queries.json`** - Comprehensive set of queries demonstrating all range filter types

## Range Field Structure

Range fields store data as key-value pairs where both keys and values are strings. In the schema definition, a Range field is specified with:

```json
{
    "field_type": "Range",
    "permission_policy": { /* ... */ },
    "payment_config": { /* ... */ },
    "field_mappers": {}
}
```

## Range Filter Types

The following filter types are supported for Range fields:

### 1. Key Filter
Matches an exact key:
```json
{
    "range_filter": {
        "Key": "warehouse:north"
    }
}
```

### 2. KeyPrefix Filter
Matches all keys starting with a prefix:
```json
{
    "range_filter": {
        "KeyPrefix": "warehouse:"
    }
}
```

### 3. KeyRange Filter
Matches keys within a lexicographic range (inclusive start, exclusive end):
```json
{
    "range_filter": {
        "KeyRange": {
            "start": "warehouse:east",
            "end": "warehouse:south"
        }
    }
}
```

### 4. Keys Filter
Matches multiple specific keys:
```json
{
    "range_filter": {
        "Keys": [
            "warehouse:north",
            "store:downtown",
            "store:mall"
        ]
    }
}
```

### 5. KeyPattern Filter
Matches keys using glob-style patterns (* and ? wildcards):
```json
{
    "range_filter": {
        "KeyPattern": "store:*"
    }
}
```

### 6. Value Filter
Matches entries by their value:
```json
{
    "range_filter": {
        "Value": "TechCorp"
    }
}
```

## Example Use Cases

### Inventory Management
The `inventory_by_location` field demonstrates how to:
- Track inventory across multiple locations
- Query specific warehouses or stores
- Filter by location patterns (all warehouses vs. all stores)
- Get inventory for multiple locations at once

### Product Attributes
The `attributes` field shows how to:
- Store flexible product specifications
- Query by specific attributes (brand, model, etc.)
- Search for attributes using patterns
- Find products with specific attribute values

## Query Examples Breakdown

1. **Basic Query**: Retrieve all products with standard fields
2. **Specific Location**: Get inventory for one warehouse
3. **Location Prefix**: Get all warehouse inventory
4. **Location Range**: Get inventory for warehouses in alphabetical range
5. **Multiple Locations**: Get inventory for specific locations
6. **Store Pattern**: Get inventory for all stores using wildcards
7. **Specific Attribute**: Get brand information for all products
8. **Attribute Value**: Find products by specific brand
9. **Technical Specs**: Find CPU specifications using pattern matching
10. **Warranty Info**: Get warranty information for all products
11. **Multiple Attributes**: Get several attributes at once
12. **Low Inventory**: Check outlet store inventory
13. **Connectivity**: Find connectivity-related attributes
14. **Display Info**: Get display-related specifications
15. **Warranty Filter**: Find products with 2-year warranty

## Usage Instructions

### 1. Create the Schema
```bash
curl -X POST http://localhost:8080/schema \
  -H "Content-Type: application/json" \
  -d @product_catalog_schema.json
```

### 2. Add Sample Data
```bash
# Add each product from the mutations file
curl -X POST http://localhost:8080/mutation \
  -H "Content-Type: application/json" \
  -d @product_catalog_mutations.json
```

### 3. Run Queries
```bash
# Example: Query inventory for all warehouses
curl -X POST http://localhost:8080/query \
  -H "Content-Type: application/json" \
  -d '{
    "type": "query",
    "schema": "ProductCatalog",
    "fields": ["name", "inventory_by_location"],
    "filter": {
      "field": "inventory_by_location",
      "range_filter": {
        "KeyPrefix": "warehouse:"
      }
    }
  }'
```

## Performance Considerations

- Range filters are optimized for key-based operations
- KeyPrefix and KeyPattern filters scan all keys but are still efficient
- Value filters require scanning all values and may be slower on large datasets
- Consider using specific key filters when possible for best performance

## Advanced Patterns

### Hierarchical Keys
Use structured keys for hierarchical data:
```
"location:region:country:city"
"product:category:subcategory:item"
```

### Timestamped Data
Include timestamps in keys for time-series data:
```
"metric:2024-01-15T10:30:00Z"
"event:2024-01-15:user_login"
```

### Multi-dimensional Attributes
Structure attribute keys for complex queries:
```
"spec:hardware:cpu"
"spec:hardware:memory"
"spec:software:os"
```

## Error Handling

Range filters will return empty results if:
- The specified field is not a Range field
- No keys match the filter criteria
- The field has no data

Invalid filter formats will return an error with details about the expected structure.