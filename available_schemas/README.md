# Available Schemas Directory

This directory contains pre-defined schema templates that can be loaded into SchemaCore. These schemas demonstrate various field types, permission policies, payment configurations, and transform capabilities.

## Schema Files (8 total)

### BlogPost.json
A schema for blog post content with the following fields:
- **title**: Single field with basic permissions
- **content**: Single field with Linear trust distance scaling
- **author**: Single field with restricted write permissions
- **publish_date**: Single field for timestamps
- **tags**: Collection field for multiple tags

### User.json
A user management schema with privacy controls:
- **username**: Public read, restricted write
- **email**: Private field with Exponential payment scaling
- **profile**: Semi-private with Linear scaling
- **created_at**: Read-only timestamp field
- **roles**: Collection field with restricted access

### Product.json
An e-commerce product schema with range fields:
- **name**: Basic product name
- **description**: Single field with Linear scaling
- **price_range**: Range field for price variations
- **category**: Product categorization
- **tags**: Collection for product tags
- **availability_count**: Range field for inventory tracking

### Analytics.json
Analytics data schema with computed fields:
- **event_name**: Event identifier
- **timestamp**: Event timestamp
- **user_id**: User reference with Linear scaling
- **session_duration**: Computed field using transforms
- **page_views**: Collection of page view data
- **conversion_rate**: Computed field with Exponential scaling

### Inventory.json
Inventory management schema:
- **item_id**: Unique item identifier
- **quantity**: Current stock quantity
- **location**: Storage location
- **reorder_level**: Threshold with Linear scaling
- **suppliers**: Collection of supplier information
- **last_updated**: Read-only timestamp

### TransformBase.json
Base schema for transform demonstrations:
- **value1**: Simple numeric value for transform inputs
- **value2**: Simple numeric value for transform inputs

### TransformSchema.json
Schema demonstrating cross-schema transforms:
- **result**: Computed field that adds TransformBase.value1 + TransformBase.value2

### UserScores.json
A range schema for gaming user score tracking and analytics:
- **user_id**: Range key field for user identification
- **game_scores**: Range field for storing individual game scores with Linear payment scaling
- **achievements**: Range field for user achievements with moderate access controls
- **player_statistics**: Range field for detailed player stats with Exponential payment scaling
- **ranking_data**: Computed range field that calculates rankings from game scores

This schema demonstrates the range functionality with a realistic gaming use case, including proper range_key configuration and mixed data format handling.

## Field Types

The schemas demonstrate three field types:
- **Single**: Individual values
- **Collection**: Arrays of values
- **Range**: Value ranges with filtering capabilities

## Payment Configurations

### Trust Distance Scaling Types:
- **None**: No scaling based on trust distance
- **Linear**: Linear scaling with slope, intercept, and min_factor
- **Exponential**: Exponential scaling with base, scale, and min_factor

## Transform Examples

### Intra-Schema Transforms
The Analytics schema includes transform examples:
- **session_duration**: Calculates duration from start/end times
- **conversion_rate**: Computes percentage from conversion data

### Cross-Schema Transforms
The TransformBase and TransformSchema demonstrate cross-schema transforms:
- **TransformBase**: Provides input values (value1, value2)
- **TransformSchema.result**: Computes TransformBase.value1 + TransformBase.value2

This demonstrates how transforms can reference fields from other schemas, enabling complex data relationships and computed fields that span multiple data sources.

## Usage

### Loading Schemas into SchemaCore

```rust
use fold_node::schema::core::SchemaCore;

// Create SchemaCore instance
let schema_core = SchemaCore::init_default()?;

// Load all available schemas
schema_core.load_available_schemas_from_directory()?;

// List loaded schemas
let schemas = schema_core.list_available_schemas()?;
println!("Loaded schemas: {:?}", schemas);

// Approve a schema for use
schema_core.approve_schema("BlogPost")?;

// Check if schema can be queried
let can_query = schema_core.can_query_schema("BlogPost");
```

### Discovering Available Schemas

```rust
// Discover schemas from available_schemas directory
let discovered = schema_core.discover_available_schemas()?;

// Fetch all available schema names (from both directories)
let all_schemas = schema_core.fetch_available_schemas()?;
```

## Schema States

Schemas can be in one of three states:
- **Available**: Discovered but not yet approved
- **Approved**: Ready for queries and mutations
- **Blocked**: Blocked from queries/mutations but transforms still run

## Integration with SchemaCore

The SchemaCore has been enhanced with the following methods:
- `discover_available_schemas()`: Find schemas in available_schemas directory
- `load_available_schemas_from_directory()`: Load all schemas into SchemaCore
- `fetch_available_schemas()`: Get schema names from both directories
- Enhanced `load_schemas_from_disk()`: Loads schemas from the data directory only and leaves available_schemas untouched

## File Format

All schema files follow the standard Schema JSON format with:
- Schema name and fields
- Permission policies with trust distance settings
- Payment configurations with scaling options
- Optional transforms for computed fields
- Field type specifications (Single, Collection, Range)