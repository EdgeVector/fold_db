# Declarative Transforms System

## Overview

The Declarative Transforms System allows you to use declarative schema definitions directly as transforms. Instead of writing procedural transform logic, you define the desired data structure and relationships declaratively, and the system automatically generates the appropriate data when source schemas change.

**Key Concept**: Your declarative schema format (like the `blogs_by_author` example) becomes the transform, just declarative specifications of what data should be generated and how.

## Table of Contents

1. [Architecture Overview](#architecture-overview)
2. [Core Components](#core-components)
3. [Integration Points](#integration-points)
4. [Usage Examples](#usage-examples)
5. [How Declarative Transforms Work](#how-declarative-transforms-work)
6. [Schema Parser and Transform Parser Integration](#schema-parser-and-transform-parser-integration)
7. [Testing Strategy](#testing-strategy)
8. [Performance Considerations](#performance-considerations)

## Architecture Overview

### Current Transform System
```
Input Data → Transform DSL → AST → Interpreter → Output Value
```

### Declarative Transform System
```
Source Schema Changes → Declarative Schema Definition → Schema Parser → Data Generator → Target Schema Field Updates
```

### Key Benefits
- **Declarative Definition**: Define data relationships without writing procedural code
- **Automatic Data Generation**: Data automatically generated based on declarative specifications
- **Reusable Patterns**: Common data structure patterns can be shared across schemas
- **Consistent Architecture**: Follows existing schema definition patterns



## Core Components

### 1. Declarative Transform Structure

```rust
// src/schema/types/json_schema.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonTransform {
    /// Explicit list of input fields in `Schema.field` format
    #[serde(default)]
    pub inputs: Vec<String>,

    /// Output field for this transform in `Schema.field` format
    pub output: String,

    /// Transform kind: either procedural DSL logic or a declarative schema
    #[serde(flatten)]
    pub kind: TransformKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TransformKind {
    Procedural { logic: String },
    Declarative { schema: DeclarativeSchemaDefinition },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeclarativeSchemaDefinition {
    /// Schema name (same as transform name)
    pub name: String,
    /// Schema type ("Single" | "HashRange")
    pub schema_type: String,
    /// Key configuration (required when schema_type == "HashRange")
    pub key: Option<KeyConfig>,
    /// Field definitions with their mapping expressions
    pub fields: std::collections::HashMap<String, FieldDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyConfig {
    /// Hash field expression for the key
    pub hash_field: String,
    /// Range field expression for the key
    pub range_field: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldDefinition {
    /// Atom UUID field expression (for reference fields)
    pub atom_uuid: Option<String>,
    /// Field type (inferred from context)
    pub field_type: Option<String>,
}
```

## Core Components (continued)

### 2. Declarative Transform Manager

The Declarative Transform Manager is responsible for:
- Parsing declarative JSON schema definitions
- Automatically generating the underlying procedural transforms
- Managing the execution of declarative transforms
- Monitoring source schema changes and triggering updates

### Key Responsibilities

1. **Parse Declarative Definitions**: Convert your JSON schema format into executable transforms
2. **Compile to Execution Plan (IR)**: Build an internal plan from the declarative spec; no user-visible logic strings
3. **Monitor Changes**: Watch for changes in source schemas and automatically update derived data
4. **Manage Execution**: Handle the lifecycle of declarative transforms from registration to execution

## Integration Points

### 1. Schema Manager Integration

The Schema Manager integrates with declarative transforms by:
- **Parsing and registering** transform definitions (using TransformKind enum)
- **Setting up field-to-transform mappings** for automatic triggering
- **Storing transforms** in the same registry as procedural transforms
- **Creating monitoring relationships** between source fields and transforms

### 2. Transform Manager Integration

The Transform Manager handles transforms (procedural and declarative) by:
- **Storing and retrieving** transform definitions (TransformKind enum)
- **Managing field-to-transform mappings** for automatic discovery
- **Providing transform existence checks** for the execution system
- **Supporting the same registration patterns** as procedural transforms

### 3. Transform Queue Integration

The Transform Queue system integrates with declarative transforms by:
- **Accepting declarative transforms** into the same queue as procedural transforms
- **Using identical QueueItem structures** for both transform types
- **Providing the same queuing, persistence, and deduplication** features
- **Enabling seamless orchestration** of mixed transform types

### 4. Transform Orchestrator Integration

The Transform Orchestrator integrates with declarative transforms by:
- **Automatically queuing declarative transforms** when source fields change
- **Processing declarative transforms** through the same execution flow
- **Managing declarative transform lifecycle** (queuing → execution → completion)
- **Providing unified monitoring and observability** for all transform types

## Usage Examples

### Example 1: Blog Post Word Indexing Declarative Transform

```json
{
  "name": "blogs_by_word",
  "schema_type": "HashRange",
  "key": {
      "hash_field": "blogpost.map().content.split_by_word().map()",
      "range_field": "blogpost.map().publish_date"
  },
  "fields": {
    "blog": { "atom_uuid": "blogpost.map().$atom_uuid" },
    "author": { "atom_uuid": "blogpost.map().author.$atom_uuid" }
  }
}
```

**What this does**: This declarative transform automatically creates a word-based index of blog posts. It splits blog content by words, maps each word to publish dates, and maintains references to both the blog content and author. This enables efficient full-text search and word-based queries.

### Example 2: User Activity Summary Declarative Transform

```json
{
  "name": "user_activity_summary",
  "schema_type": "HashRange",
  "key": {
    "hash_field": "user_activity.map().user_id",
    "range_field": "user_activity.map().timestamp"
  },
  "fields": {
    "activity_summary": { "atom_uuid": "user_activity.map().$atom_uuid" },
    "usage_patterns":   { "atom_uuid": "user_activity.map().usage_patterns.$atom_uuid" }
  }
}
```

**What this does**: This declarative transform automatically creates a user activity summary index organized by user ID and timestamp. It maintains references to activity summaries and usage patterns, enabling efficient user behavior analysis and timeline queries.

### Example 3: Product Category Indexing

```json
{
  "name": "products_by_category",
  "schema_type": "HashRange",
  "key": {
    "hash_field": "product.map().category",
    "range_field": "product.map().price"
  },
  "fields": {
    "product_info": { "atom_uuid": "product.map().$atom_uuid" },
    "brand":        { "atom_uuid": "product.map().brand.$atom_uuid" }
  }
}
```

**What this does**: Creates a category-based product index sorted by price, enabling efficient category browsing and price-based filtering.

### Example 4: User Activity Timeline

```json
{
  "name": "user_activity_timeline",
  "schema_type": "HashRange",
  "key": {
    "hash_field": "user_activity.map().user_id",
    "range_field": "user_activity.map().timestamp"
  },
  "fields": {
    "activity":      { "atom_uuid": "user_activity.map().$atom_uuid" },
    "activity_type": { "atom_uuid": "user_activity.map().activity_type.$atom_uuid" }
  }
}
```

**What this does**: Creates a user activity timeline indexed by user ID and timestamp, enabling efficient user activity queries and time-based analysis.

## How Declarative Transforms Work

### 1. **Registration and Queuing**
When you register a declarative transform, the system:

#### **Transform Registration**
- Parses your declarative schema definition
- Automatically infers field types from the mapping expressions
- Sets up monitoring for all referenced source schemas
- Creates the target schema with the specified structure
- **Registers the transform with the TransformManager** for execution
- **Creates field-to-transform mappings** for automatic triggering

#### **Queue Integration**
- Declarative transforms are stored in the same transform registry as procedural transforms
- Each declarative transform gets a unique transform ID (e.g., `"blogs_by_word.declarative"`)
- The transform is automatically added to the **TransformOrchestrator's execution queue** when source data changes
- **Same queuing system** handles both procedural and declarative transforms seamlessly

### 2. **Automatic Execution via Queue**
The transform automatically runs through the existing queue system when:
- Any referenced source schema data changes
- Source field values are updated
- New data is added to source schemas
- **The system automatically adds the transform to the execution queue**
- **TransformOrchestrator processes the queued transform** using the same execution flow

### 3. **Data Generation**
Based on your declarative specification:
- **key field**: Creates the primary index structure (HashRange by default)
- **Other fields**: Creates reference fields with atom UUIDs
- **Mapping expressions**: Automatically resolved to source data

### 4. **Complete Flow for blogs_by_word**
```
1. User creates a new blog post in 'blogpost' schema
2. System detects change in 'blogpost' content
3. Field change triggers field-to-transform mapping lookup
4. Declarative transform 'blogs_by_word.declarative' is found
5. Transform is automatically added to TransformOrchestrator queue
6. TransformOrchestrator processes the queued transform
7. System splits blog content by words using 'split_by_word.map()'
8. For each word, creates index entry: word → publish_date → blog_ref + author_ref
9. New index enables efficient word-based search queries
```

### 5. **Benefits of Declarative Approach**
- **No user-written procedural code**: You describe the structure; the runtime compiles an internal plan
- **Automatic maintenance**: Indexes stay up-to-date automatically
- **Performance**: Optimized data structures created automatically
- **Consistency**: Same pattern works across different schemas
- **Intuitive**: Schema definition format is the transform definition
- **Seamless integration**: Uses existing transform queue and orchestration system

## Queue Integration and Execution Flow

### How Declarative Transforms Integrate with the Existing Queue System

Declarative transforms seamlessly integrate with DataFold's existing transform queue and orchestration system. Here's how they fit into the established architecture:

#### **1. Transform Registration Process**

```rust
// In src/schema/transform.rs - register_schema_transforms()
pub(crate) fn register_schema_transforms(&self, schema: &Schema) -> Result<(), SchemaError> {
    // ... existing procedural transform registration ...
    
    // NEW: Handle declarative transforms
    if let Some(declarative_schema) = &schema.declarative_schema {
        let transform_id = format!("{}.{}", schema.name, "declarative");
        
        // Store declarative transform definition in the same registry
        let transform = Transform::from_kind(TransformKind::Declarative { schema: declarative_schema.clone() });
        self.db_ops.store_transform(&transform_id, &transform)?;
        
        // Create field mappings for automatic triggering
        for input_field in &transform.get_inputs() {
            self.store_field_to_transform_mapping(input_field, &transform_id)?;
        }
        
        info!("✅ Declarative transform '{}' registered and ready for queuing", transform_id);
    }
    
    Ok(())
}
```

#### **2. Queue Integration**

Declarative transforms use the exact same queuing system as procedural transforms:

```rust
// In src/fold_db_core/orchestration/queue_manager.rs
pub struct QueueItem {
    pub id: String,           // "blogs_by_word.declarative"
    pub mutation_hash: String, // Mutation identifier
}

// Both procedural and declarative transforms use the same QueueItem structure
// No changes needed to the queue system itself
```

#### **3. Execution Flow Integration**

The execution coordinator automatically handles both types:

```rust
// In src/fold_db_core/orchestration/execution_coordinator.rs
impl ExecutionCoordinator {
    pub fn execute_transform(&self, item: &QueueItem, already_processed: bool) -> Result<JsonValue, SchemaError> {
        let transform_id = &item.id;
        let transform = self.manager.get_transform(transform_id)?;

        match &transform.kind {
            TransformKind::Declarative { schema } => {
                let plan = self.compiler.compile(schema)?; // compile to IR/ExecPlan
                self.execute_plan(transform_id, &plan)
            }
            TransformKind::Procedural { logic } => {
                self.execute_procedural(transform_id, logic)
            }
        }
    }
}
```

#### **4. Field Monitoring and Auto-Queuing**

The system automatically monitors source fields and queues transforms:

```rust
// In src/fold_db_core/orchestration/event_monitor.rs
impl EventMonitor {
    fn handle_field_value_set(&self, schema_name: &str, field_name: &str, mutation_hash: &str) {
        // Look up transforms that depend on this field
        let transform_ids = self.manager.get_transforms_for_field(schema_name, field_name)?;
        
        // Add ALL transforms (procedural and declarative) to the queue
        for transform_id in transform_ids {
            self.orchestrator.add_transform(&transform_id, mutation_hash)?;
        }
    }
}
```

### **Key Benefits of Queue Integration**

1. **Unified Management**: Both transform types use the same queue, orchestration, and execution systems
2. **Automatic Triggering**: Declarative transforms are automatically queued when source data changes
3. **Consistent Lifecycle**: Same monitoring, queuing, execution, and result handling for all transforms
4. **Scalability**: Leverages existing queue management, persistence, and error handling
5. **Monitoring**: Same observability and debugging tools work for both transform types

## Schema Parser and Transform Parser Integration

### How `key` is Interpreted

The `key` field is a **special field name** that the schema parser recognizes to automatically configure the schema type and indexing structure. For HashRange schemas, this field serves as both the hash key and range key.

#### 1. **Schema Parser Recognition**

```rust
// src/schema/parser.rs
impl SchemaParser {
    fn parse_declarative_transform(&mut self, json: &JsonValue) -> Result<DeclarativeSchemaDefinition, ParseError> {
        let name = json["name"].as_str().ok_or(ParseError::MissingField("name"))?.to_owned();
        let schema_type = json["schema_type"].as_str().ok_or(ParseError::MissingField("schema_type"))?.to_owned();

        let key = if schema_type == "HashRange" {
            let key_value = json.get("key").ok_or(ParseError::MissingField("key"))?;
            Some(self.parse_key_config(key_value)?)
        } else { None };

        let fields_obj = json.get("fields").and_then(|v| v.as_object()).ok_or(ParseError::MissingField("fields"))?;
        let fields = self.parse_fields(fields_obj)?;

        Ok(DeclarativeSchemaDefinition { name, schema_type, key, fields })
    }

    fn parse_key_config(&mut self, key_value: &Value) -> Result<KeyConfig, ParseError> {
        let key_obj = key_value.as_object().ok_or(ParseError::InvalidField("key must be an object"))?;
        let hash_field = key_obj.get("hash_field").and_then(|v| v.as_str()).ok_or(ParseError::MissingField("hash_field"))?;
        let range_field = key_obj.get("range_field").and_then(|v| v.as_str()).ok_or(ParseError::MissingField("range_field"))?;
        Ok(KeyConfig { hash_field: hash_field.to_owned(), range_field: range_field.to_owned() })
    }
}
```

#### 2. **Range Key Configuration Structure**

```rust
#[derive(Debug, Clone)]
pub struct KeyConfig {
    pub hash_field: String,
    pub range_field: String,
}

#[derive(Debug, Clone)]
pub struct DeclarativeSchemaDefinition {
    pub name: String,
    pub schema_type: String, // "Single" | "HashRange"
    pub key: Option<KeyConfig>,
    pub fields: HashMap<String, FieldDefinition>,
}
```

#### 3. **Schema Type Inference**

The parser automatically infers the schema type based on the presence of `key`:

```rust
impl SchemaParser {
    fn create_schema_structure(&self, d: &DeclarativeSchemaDefinition) -> Schema {
        match d.schema_type.as_str() {
            "HashRange" => {
                let _k = d.key.as_ref().expect("key required for HashRange");
                Schema::new_hash_range(d.name.clone(), "key".to_string(), "key".to_string())
            }
            "Single" => Schema::new_single(d.name.clone()),
            other => panic!("Unknown schema_type: {}", other),
        }
    }
}
```

#### 4. **Field Type Inference**

The parser automatically determines field types based on the `key` configuration:

```rust
impl SchemaParser {
    fn infer_field_types(&self, d: &DeclarativeSchemaDefinition) -> HashMap<String, FieldVariant> {
        let mut out = HashMap::new();
        if let Some(k) = &d.key {
            out.insert(
                "key".to_string(),
                FieldVariant::HashRange(
                    HashRangeField::new(
                        self.default_permissions(),
                        self.default_payment_config(),
                        HashMap::new(),
                        k.hash_field.clone(),
                        k.range_field.clone(),
                        None, // no implicit atom uuid binding
                    ),
                ),
            );
        }
        for (name, def) in &d.fields {
            match &def.atom_uuid {
                Some(_expr) => out.insert(
                    name.clone(),
                    FieldVariant::Single(SingleField::new(
                        self.default_permissions(),
                        self.default_payment_config(),
                        HashMap::new(),
                    )),
                ),
                None => out.insert(
                    name.clone(),
                    FieldVariant::Single(SingleField::new(
                        self.default_permissions(),
                        self.default_payment_config(),
                        HashMap::new(),
                    )),
                ),
            };
        }
        out
    }
}
```

**Note**: The field type inference code above is conceptual and shows how the parser would work. The actual implementation would use the existing DataFold field types and structures.
```

#### 5. **Transform Parser Integration**

The schema parser then passes the structured data to the transform parser:

```rust
impl TransformParser {
    fn parse_declarative_transform(&mut self, d: &DeclarativeSchemaDefinition) -> Result<Transform, ParseError> {
        let output = format!("{}.key", d.name);
        let plan = self.compiler.compile(d)?; // produce IR/ExecPlan
        Ok(Transform::from_kind_and_output(TransformKind::Declarative { schema: d.clone() }, output, plan))
    }
}
```

### Complete Flow Example

For your `blogs_by_word` example:

```json
{
  "name": "blogs_by_word",
  "schema_type": "HashRange",
  "key": {
      "hash_field": "blogpost.map().content.split_by_word().map()",
      "range_field": "blogpost.map().publish_date"
  },
  "fields": {
    "blog": { "atom_uuid": "blogpost.map().$atom_uuid" },
    "author": { "atom_uuid": "blogpost.map().author.$atom_uuid" }
  }
}
```

#### 1. **Schema Parser** recognizes `key` and creates:
- Schema type: `HashRange` with `key` as both hash and range key field
- Field type: `HashRange` for the `key` field
- Reference fields: `blog` and `author` as Single fields

#### 2. **Transform Parser** receives structured data and generates:
- Transform logic: Automatically generated procedural code
- Output field: `blogs_by_word.key`
- Input dependencies: Automatically extracted from mapping expressions

#### 3. **Result**: A working transform that:
- Monitors `blogpost` schema changes
- Automatically generates HashRange index entries
- Maintains the word-based index structure

**The `key` field essentially tells the system "this field is special - it defines the primary index structure" and the parser automatically handles all the configuration details!**

### Benefits of the Improved Structure

Moving `key` to the top level provides several advantages:

1. **Clearer Separation**: The `key` configuration is clearly separated from regular fields
2. **Better Readability**: It's immediately obvious which fields are for indexing vs. references
3. **Easier Parsing**: The parser can handle `key` independently of field parsing
4. **More Intuitive**: Follows the natural schema structure where range keys are schema-level properties
5. **Consistent with Schema Definition**: Matches how HashRange schemas are typically defined

### Structure Comparison

**Before (nested in fields):**
```json
{
  "fields": {
    "key": { ... },
    "blog": { ... }
  }
}
```

**After (top-level):**
```json
{
  "key": { ... },
  "fields": {
    "blog": { ... }
  }
}
```

This cleaner structure makes it easier to understand the intent and easier for the parser to process!

## Testing Strategy

The testing strategy for declarative transforms focuses on:

### 1. **Unit Tests**
- Testing declarative JSON parsing
- Validating field type inference
- Testing key configuration parsing
- Verifying schema structure generation
- **Testing transform registration** with the existing registry
- **Validating field-to-transform mapping** creation

### 2. **Integration Tests**
- End-to-end declarative transform execution
- Testing automatic data generation
- Verifying schema change monitoring
- Testing transform lifecycle management
- **Testing queue integration** with existing TransformOrchestrator
- **Validating automatic queuing** when source fields change
- **Testing mixed transform execution** (procedural + declarative in same queue)

### 3. **Queue System Tests**
- **Testing declarative transform queuing** through the existing queue system
- **Validating QueueItem compatibility** for declarative transforms
- **Testing execution flow integration** with ExecutionCoordinator
- **Verifying persistence and state management** for declarative transforms

### 4. **Performance Tests**
- Measuring declarative transform parsing speed
- Testing data generation performance
- Validating caching effectiveness
- Monitoring memory usage during execution
- **Testing queue performance** with mixed transform types
- **Measuring orchestration overhead** for declarative transforms

## Performance Considerations

### 1. **Caching Strategy**
- Cache parsed declarative definitions to avoid re-parsing
- Cache generated procedural logic for reuse
- Invalidate caches when source schemas change

### 2. **Lazy Evaluation**
- Parse declarative definitions only when needed
- Generate procedural logic on-demand
- Cache results for subsequent executions

### 3. **Incremental Updates**
- Only regenerate data for changed source fields
- Batch multiple source changes together
- Use efficient change detection mechanisms

## Conclusion

The Declarative Transforms System provides a powerful way to automatically generate and maintain data structures based on your declarative schema format. Your schema definition **is the transform** - no procedural code needed, just declarative specifications of what data should be generated and how.

### Key Benefits

- **Pure Declarative Approach**: Your JSON schema format directly becomes the transform definition
- **Automatic Data Generation**: Data structures are created and maintained automatically based on source schema changes
- **No Procedural Code**: The system automatically generates the underlying procedural logic needed
- **Intuitive Design**: Uses the same schema definition patterns you're already familiar with
- **Performance Optimization**: Optimized indexes and data structures are created automatically
- **Seamless Integration**: Leverages existing transform queue and orchestration infrastructure
- **Unified Management**: Both procedural and declarative transforms use the same execution system

### How It Works

1. **Define**: Write your schema structure in declarative JSON format
2. **Register**: The system parses your declarative definition and registers it with the existing transform registry
3. **Queue**: When source data changes, the system automatically adds your declarative transform to the execution queue
4. **Execute**: The TransformOrchestrator processes the queued transform using the same execution flow as procedural transforms
5. **Maintain**: Your indexes, computed fields, and derived data stay up-to-date automatically

### Queue Integration Benefits

- **Automatic Queuing**: Declarative transforms are automatically queued when source fields change
- **Unified Execution**: Same queue, orchestration, and execution system for all transform types
- **Consistent Monitoring**: Same observability, error handling, and lifecycle management
- **Scalable Architecture**: Leverages proven queue management, persistence, and orchestration
- **Mixed Transform Support**: Procedural and declarative transforms can coexist seamlessly

This system enables DataFold to automatically create and maintain indexes, computed fields, and derived data structures based on your declarative schema specifications. Your `blogs_by_word` format automatically becomes a working transform that maintains word-based indexes, your `products_by_category` format becomes a category-based product index, and so on.

**The beauty is in the simplicity**: Define your schema structure declaratively, and it automatically becomes a transform that maintains that structure as your data evolves. No user-written procedural DSL, no complex logic - just clean, intuitive schema definitions that work as transforms, seamlessly integrated with the existing queue and orchestration system.

### **1.4 Example: blogs_by_word Input Resolution**

```rust
// For the expression "blogpost.map().content.split_by_word().map()"
let operations = vec![
    IteratorOperation::SelectSchema("blogpost".to_string()),
    IteratorOperation::Map,
    IteratorOperation::SelectField("content".to_string()),
    IteratorOperation::SplitByWord,
    IteratorOperation::Map,
];

// This creates:
// 1. Select blogpost schema data
// 2. Map over all blogpost entries
// 3. Extract content field from each entry
// 4. Split content by words (creates arrays)
// 5. Map over the split results (fan-out applied)

// Input: {"blogpost": {"entry1": {"content": "Hello world", "publish_date": "2025-01-27"}}}
// After SelectSchema: {"entry1": {"content": "Hello world", "publish_date": "2025-01-27"}}
// After Map: {"entry1": {"content": "Hello world", "publish_date": "2025-01-27"}}
// After SelectField: {"entry1": "Hello world"}
// After SplitByWord: {"entry1": ["Hello", "world"]}
// After Map + Fan-out: [{"entry1": "Hello"}, {"entry1": "world"}]
```

### 2. **Data Generation Using Iterator Stack**

#### **2.1 Declarative Data Generator with Iterator Stack**

The data generator now leverages the iterator stack for powerful, composable data processing:

```rust
pub struct DeclarativeDataGenerator {
    input_resolver: DeclarativeInputResolver,
    iterator_stack: IteratorStack,
}

impl DeclarativeDataGenerator {
    pub fn new() -> Self {
        Self {
            input_resolver: DeclarativeInputResolver::new(),
            iterator_stack: IteratorStack::new(),
        }
    }
    
    pub fn generate_from_schema(
        &self,
        schema: &DeclarativeSchemaDefinition,
        source_data: HashMap<String, JsonValue>,
    ) -> Result<JsonValue, ExecutionError> {
        // Resolve all inputs using iterator stack
        let resolved_inputs = self.input_resolver.resolve_inputs(schema, &source_data)?;
        
        // Generate data based on schema type
        match schema.schema_type.as_str() {
            "HashRange" => self.generate_hash_range_data(schema, &resolved_inputs),
            "Single" => self.generate_single_data(schema, &resolved_inputs),
            _ => Err(ExecutionError::UnsupportedSchemaType(schema.schema_type.clone())),
        }
    }
    
    fn generate_hash_range_data(
        &self,
        schema: &DeclarativeSchemaDefinition,
        inputs: &ResolvedInputs,
    ) -> Result<JsonValue, ExecutionError> {
        let key_config = schema.key.as_ref()
            .ok_or(ExecutionError::MissingKeyConfiguration)?;
        
        // Get hash and range values
        let hash_data = inputs.data.get("hash_field")
            .ok_or(ExecutionError::InputNotFound("hash_field".to_string()))?;
        let range_data = inputs.data.get("range_field")
            .ok_or(ExecutionError::InputNotFound("range_field".to_string()))?;
        
        // Handle fan-out for hash field (e.g., split words)
        let hash_values = self.extract_values_for_fan_out(hash_data)?;
        let range_values = self.extract_values_for_fan_out(range_data)?;
        
        // Generate entries for each hash/range combination
        let mut entries = Vec::new();
        let max_length = hash_values.len().max(range_values.len());
        
        for i in 0..max_length {
            let hash_value = hash_values.get(i).unwrap_or(&JsonValue::Null);
            let range_value = range_values.get(i).unwrap_or(&JsonValue::Null);
            
            let mut entry = HashMap::new();
            entry.insert("hash_key".to_string(), hash_value.clone());
            entry.insert("range_key".to_string(), range_value.clone());
            
            // Add reference fields
            for (field_name, field_def) in &schema.fields {
                if let Some(atom_uuid_expr) = &field_def.atom_uuid {
                    if let Some(field_data) = inputs.data.get(field_name) {
                        let field_values = self.extract_values_for_fan_out(field_data)?;
                        let field_value = field_values.get(i).unwrap_or(&JsonValue::Null);
                        entry.insert(field_name.clone(), field_value.clone());
                    }
                }
            }
            
            entries.push(JsonValue::Object(entry));
        }
        
        Ok(JsonValue::Array(entries))
    }
    
    fn generate_single_data(
        &self,
        schema: &DeclarativeSchemaDefinition,
        inputs: &ResolvedInputs,
    ) -> Result<JsonValue, ExecutionError> {
        let mut entry = HashMap::new();
        
        // Add all field values
        for (field_name, field_def) in &schema.fields {
            if let Some(atom_uuid_expr) = &field_def.atom_uuid {
                if let Some(field_data) = inputs.data.get(field_name) {
                    entry.insert(field_name.clone(), field_data.clone());
                }
            }
        }
        
        Ok(JsonValue::Object(entry))
    }
    
    fn extract_values_for_fan_out(&self, data: &JsonValue) -> Result<Vec<JsonValue>, ExecutionError> {
        match data {
            JsonValue::Array(arr) => Ok(arr.clone()),
            JsonValue::String(s) => Ok(vec![JsonValue::String(s.clone())]),
            JsonValue::Number(n) => Ok(vec![JsonValue::Number(n.clone())]),
            JsonValue::Bool(b) => Ok(vec![JsonValue::Bool(*b)]),
            JsonValue::Object(obj) => {
                // For objects, create a single entry
                Ok(vec![JsonValue::Object(obj.clone())])
            }
            JsonValue::Null => Ok(vec![JsonValue::Null]),
        }
    }
}
```

### 3. **Complete Execution Flow with Iterator Stack**

#### **3.1 Execution Coordinator Integration**

```rust
impl ExecutionCoordinator {
    pub fn execute_declarative_transform(
        &self,
        transform_id: &str,
        schema: &DeclarativeSchemaDefinition,
        source_data: HashMap<String, JsonValue>,
    ) -> Result<JsonValue, ExecutionError> {
        info!("🔄 Executing declarative transform: {}", schema.name);
        
        // Generate data using iterator stack-based generator
        let data_generator = DeclarativeDataGenerator::new();
        let result = data_generator.generate_from_schema(schema, source_data)?;
        
        // Store the result to target schema
        self.store_declarative_result(transform_id, schema, &result)?;
        
        info!("✅ Declarative transform '{}' executed successfully", transform_id);
        Ok(result)
    }
    
    fn store_declarative_result(
        &self,
        transform_id: &str,
        schema: &DeclarativeSchemaDefinition,
        result: &JsonValue,
    ) -> Result<(), ExecutionError> {
        match schema.schema_type.as_str() {
            "HashRange" => {
                if let Some(key_config) = &schema.key {
                    self.store_hash_range_entries(schema.name.as_str(), result)?;
                }
            }
            "Single" => {
                self.store_single_entry(schema.name.as_str(), result)?;
            }
            _ => return Err(ExecutionError::UnsupportedSchemaType(schema.schema_type.clone())),
        }
        
        Ok(())
    }
    
    fn store_hash_range_entries(
        &self,
        schema_name: &str,
        result: &JsonValue,
    ) -> Result<(), ExecutionError> {
        if let JsonValue::Array(entries) = result {
            for entry in entries {
                if let JsonValue::Object(entry_obj) = entry {
                    self.store_hash_range_entry(schema_name, entry_obj)?;
                }
            }
        }
        Ok(())
    }
}
```

#### **3.2 Example Execution for blogs_by_word with Iterator Stack**

```rust
// When blogpost.content changes, the system:
// 1. Detects change and queues declarative transform
// 2. Resolves inputs using iterator stack:

let source_data = HashMap::from([
    ("blogpost".to_string(), JsonValue::Object(HashMap::from([
        ("entry1".to_string(), JsonValue::Object(HashMap::from([
            ("content".to_string(), JsonValue::String("Hello world example".to_string())),
            ("publish_date".to_string(), JsonValue::String("2025-01-27".to_string())),
            ("$atom_uuid".to_string(), JsonValue::String("uuid-123".to_string())),
            ("author".to_string(), JsonValue::Object(HashMap::from([
                ("$atom_uuid".to_string(), JsonValue::String("uuid-456".to_string())),
            ]))),
        ]))),
    ]))),
]);

// 3. Execute iterator stack operations for hash_field: "blogpost.map().content.split_by_word().map()"
let hash_operations = vec![
    IteratorOperation::SelectSchema("blogpost".to_string()),
    IteratorOperation::Map,
    IteratorOperation::SelectField("content".to_string()),
    IteratorOperation::SplitByWord,
    IteratorOperation::Map,
];

// Result: ["Hello", "world", "example"] (fan-out applied)

// 4. Execute iterator stack operations for range_field: "blogpost.map().publish_date"
let range_operations = vec![
    IteratorOperation::SelectSchema("blogpost".to_string()),
    IteratorOperation::Map,
    IteratorOperation::SelectField("publish_date".to_string()),
];

// Result: "2025-01-27"

// 5. Generate HashRange entries using iterator stack results:
let entries = vec![
    JsonValue::Object(HashMap::from([
        ("hash_key".to_string(), JsonValue::String("Hello".to_string())),
        ("range_key".to_string(), JsonValue::String("2025-01-27".to_string())),
        ("blog".to_string(), JsonValue::String("uuid-123".to_string())),
        ("author".to_string(), JsonValue::String("uuid-456".to_string())),
    ])),
    JsonValue::Object(HashMap::from([
        ("hash_key".to_string(), JsonValue::String("world".to_string())),
        ("range_key".to_string(), JsonValue::String("2025-01-27".to_string())),
        ("blog".to_string(), JsonValue::String("uuid-123".to_string())),
        ("author".to_string(), JsonValue::String("uuid-456".to_string())),
    ])),
    JsonValue::Object(HashMap::from([
        ("hash_key".to_string(), JsonValue::String("example".to_string())),
        ("range_key".to_string(), JsonValue::String("2025-01-27".to_string())),
        ("blog".to_string(), JsonValue::String("uuid-123".to_string())),
        ("author".to_string(), JsonValue::String("uuid-456".to_string())),
    ])),
];

// 6. Store to blogs_by_word schema
store_hash_range_entries("blogs_by_word", &JsonValue::Array(entries))?;
```

### 4. **Performance Optimizations with Iterator Stack**

#### **4.1 Lazy Evaluation and Streaming**

The iterator stack enables lazy evaluation and streaming for large datasets:

```rust
impl IteratorStack {
    pub fn execute_operations_streaming(
        &self,
        operations: &[IteratorOperation],
        source_data: &HashMap<String, JsonValue>,
    ) -> impl Iterator<Item = Result<JsonValue, ExecutionError>> {
        // Execute operations one at a time, yielding results as they're computed
        // This enables processing of large datasets without loading everything into memory
        
        let mut current_data = source_data.clone();
        
        operations.iter().map(move |operation| {
            current_data = self.apply_operation(&current_data, operation)?;
            Ok(JsonValue::Object(current_data.clone()))
        })
    }
}
```

#### **4.2 Caching Iterator Results**

```rust
impl DeclarativeDataGenerator {
    fn get_cached_iterator_result(&self, cache_key: &str) -> Option<JsonValue> {
        // Check cache for previously computed iterator results
        self.cache.get(cache_key).cloned()
    }
    
    fn cache_iterator_result(&mut self, cache_key: String, result: JsonValue) {
        // Cache computed iterator results for reuse
        self.cache.insert(cache_key, result);
    }
}
```

#### **4.3 Incremental Updates with Iterator Stack**

```rust
impl IncrementalUpdater {
    pub fn update_only_changed_fields_with_iterator(
        &self,
        schema: &DeclarativeSchemaDefinition,
        changed_fields: &HashSet<String>,
        source_data: &HashMap<String, JsonValue>,
    ) -> Result<(), ExecutionError> {
        // Only regenerate data for fields that actually changed
        for field in changed_fields {
            if self.field_affects_declarative_transform(field, schema) {
                // Use iterator stack to efficiently process only changed data
                let field_operations = self.get_field_iterator_operations(field, schema)?;
                let updated_data = self.iterator_stack.execute_operations(&field_operations, source_data)?;
                
                self.regenerate_affected_data_with_iterator(schema, field, &updated_data)?;
            }
        }
        Ok(())
    }
}
```

This implementation leverages the existing **Schema Indexing Iterator Stack and Fan-out Model** to provide a powerful, consistent, and well-tested foundation for declarative transform execution. The iterator stack approach offers several advantages:

1. **Consistency**: Uses the same data processing patterns already established in DataFold
2. **Power**: Provides composable operations like Map, Filter, Sort, Split, etc.
3. **Performance**: Enables lazy evaluation, streaming, and efficient data processing
4. **Extensibility**: Easy to add new iterator operations as needed
5. **Testing**: Leverages existing iterator stack tests and validation
6. **Maintainability**: Reuses proven, well-tested infrastructure
