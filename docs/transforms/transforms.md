# Transform System and DSL

Fold DB's transform system provides programmable computation capabilities that execute automatically when field values change, enabling real-time data processing and derived field calculations.

## Table of Contents

1. [Transform Fundamentals](#transform-fundamentals)
2. [Transform DSL](#transform-dsl)
3. [Built-in Functions](#built-in-functions)
4. [Event-Driven Execution](#event-driven-execution)
5. [Transform Registration](#transform-registration)
6. [Dependency Management](#dependency-management)
7. [Performance Optimization](#performance-optimization)
8. [Examples](#examples)
9. [Best Practices](#best-practices)

## Transform Fundamentals

### What are Transforms?

Transforms are programmable functions that:
- Execute automatically when input field values change
- Compute derived values from existing data
- Support complex business logic and calculations
- Maintain consistency across related fields
- Enable real-time data processing

### Transform Definition

```json
{
  "transform": {
    "inputs": ["field1", "field2"],
    "logic": "if field1 > field2 { return field1 - field2 } else { return 0 }",
    "output": "SchemaName.output_field"
  }
}
```

### Transform Types

**Field Transforms:**
Defined within schema field definitions, automatically executed when dependencies change.

```json
{
  "name": "UserStatus",
  "fields": {
    "age": {"field_type": "Single"},
    "status": {
      "field_type": "Single",
      "transform": {
        "inputs": ["age"],
        "logic": "if age >= 18 { return \"adult\" } else { return \"minor\" }",
        "output": "UserStatus.status"
      },
      "writable": false
    }
  }
}
```

**Standalone Transforms:**
Registered independently and can operate across multiple schemas.

```json
{
  "name": "conversion_rate_calculator",
  "inputs": ["Analytics.conversions", "Analytics.total_visits"],
  "logic": "return (conversions / total_visits) * 100",
  "output": "Analytics.conversion_rate"
}
```

## Transform DSL

### Syntax Overview

The transform DSL (Domain-Specific Language) supports:
- Variables and expressions
- Conditional statements
- Mathematical operations
- String manipulation
- Built-in functions
- Type conversions

### Basic Syntax

**Variables:**
```javascript
// Access input fields directly by name
username
age
profile_data
```

**Mathematical Operations:**
```javascript
// Basic arithmetic
result = value1 + value2
difference = total - count
percentage = (part / whole) * 100
```

**Conditional Logic:**
```javascript
// If-else statements
if condition {
    return value1
} else {
    return value2
}

// Ternary operator
result = condition ? value1 : value2
```

**String Operations:**
```javascript
// String concatenation
full_name = first_name + " " + last_name

// String functions
uppercase_name = upper(username)
email_domain = substring(email, indexOf(email, "@") + 1)
```

### Data Types

**Supported Types:**
- **String**: Text values
- **Number**: Integers and floating-point numbers
- **Boolean**: true/false values
- **Array**: Collections of values
- **Object**: Key-value pairs

**Type Conversion:**
```javascript
// Explicit type conversion
age_string = toString(age)
price_number = toNumber(price_text)
is_active = toBoolean(status)
```

### Control Flow

**If-Else Statements:**
```javascript
if age >= 65 {
    return "senior"
} else if age >= 18 {
    return "adult"
} else {
    return "minor"
}
```

**Switch-Like Logic:**
```javascript
// Using nested conditionals
if status == "active" {
    return "green"
} else if status == "pending" {
    return "yellow"
} else {
    return "red"
}
```

**Loops and Iteration:**
```javascript
// Array processing
total = 0
for value in values {
    total = total + value
}
return total

// Map operations
doubled_values = map(values, x => x * 2)
```

### Error Handling

**Safe Operations:**
```javascript
// Null-safe access
safe_length = length(optional_string) ?? 0

// Default values
display_name = username ?? "Anonymous"

// Try-catch equivalent
result = tryParseNumber(input_string) ?? 0
```

## Built-in Functions

### Mathematical Functions

**Basic Math:**
```javascript
abs(-5)           // 5
min(a, b)         // minimum of two values
max(a, b)         // maximum of two values
round(3.7)        // 4
floor(3.7)        // 3
ceil(3.2)         // 4
sqrt(16)          // 4
pow(2, 3)         // 8
```

**Statistical Functions:**
```javascript
sum([1, 2, 3, 4])         // 10
avg([1, 2, 3, 4])         // 2.5
median([1, 2, 3, 4, 5])   // 3
stddev([1, 2, 3, 4, 5])   // standard deviation
```

### String Functions

**String Manipulation:**
```javascript
length("hello")                    // 5
upper("hello")                     // "HELLO"
lower("HELLO")                     // "hello"
trim("  hello  ")                  // "hello"
substring("hello", 1, 3)           // "el"
indexOf("hello", "l")              // 2
replace("hello", "l", "x")         // "hexxo"
split("a,b,c", ",")               // ["a", "b", "c"]
join(["a", "b", "c"], ",")        // "a,b,c"
```

**Pattern Matching:**
```javascript
matches(email, "^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}$")
startsWith(text, "prefix")
endsWith(text, "suffix")
contains(text, "substring")
```

### Array Functions

**Array Operations:**
```javascript
length([1, 2, 3])                 // 3
first([1, 2, 3])                  // 1
last([1, 2, 3])                   // 3
contains([1, 2, 3], 2)            // true
indexOf([1, 2, 3], 2)             // 1
slice([1, 2, 3, 4], 1, 3)        // [2, 3]
concat([1, 2], [3, 4])            // [1, 2, 3, 4]
unique([1, 2, 2, 3])              // [1, 2, 3]
sort([3, 1, 2])                   // [1, 2, 3]
reverse([1, 2, 3])                // [3, 2, 1]
```

**Higher-Order Functions:**
```javascript
map([1, 2, 3], x => x * 2)        // [2, 4, 6]
filter([1, 2, 3, 4], x => x > 2)  // [3, 4]
reduce([1, 2, 3], (acc, x) => acc + x, 0)  // 6
any([1, 2, 3], x => x > 2)        // true
all([1, 2, 3], x => x > 0)        // true
```

### Date/Time Functions

**Date Operations:**
```javascript
now()                             // current timestamp
date("2024-01-15")               // parse date string
formatDate(timestamp, "YYYY-MM-DD")
year(date)                        // extract year
month(date)                       // extract month
day(date)                         // extract day
addDays(date, 7)                  // add 7 days
diffDays(date1, date2)            // difference in days
```

### Type Functions

**Type Checking:**
```javascript
isString(value)
isNumber(value)
isBoolean(value)
isArray(value)
isObject(value)
isNull(value)
```

**Type Conversion:**
```javascript
toString(123)                     // "123"
toNumber("123")                   // 123
toBoolean("true")                 // true
toArray("a,b,c", ",")            // ["a", "b", "c"]
```

### Range Field Functions

**Range Operations:**
```javascript
// Access range field values
getValue(range_field, "key")
getKeys(range_field)
getValues(range_field)
hasKey(range_field, "key")

// Range calculations
sumValues(range_field)
avgValues(range_field)
maxValue(range_field)
minValue(range_field)

// Key operations
getKeysWithPrefix(range_field, "prefix:")
getKeysInRange(range_field, "start", "end")
getKeysByPattern(range_field, "pattern*")
```

## Event-Driven Execution

### Automatic Execution

Transforms execute automatically when their input fields change:

```json
{
  "name": "Analytics",
  "fields": {
    "session_start": {"field_type": "Single"},
    "session_end": {"field_type": "Single"},
    "session_duration": {
      "field_type": "Single",
      "transform": {
        "inputs": ["session_start", "session_end"],
        "logic": "return session_end - session_start",
        "output": "Analytics.session_duration"
      },
      "writable": false
    }
  }
}
```

**Trigger Example:**
```bash
# Update session_end - this triggers the transform
curl -X POST http://localhost:9001/api/execute \
  -H "Content-Type: application/json" \
  -d '{
    "operation": "{\"type\":\"mutation\",\"schema\":\"Analytics\",\"operation\":\"update\",\"filter\":{\"id\":\"session123\"},\"data\":{\"session_end\":1609459200}}"
  }'

# session_duration is automatically calculated and updated
```

### Execution Context

**Available Variables:**
- Input field values by name
- Previous field values (prefixed with `prev_`)
- Metadata variables (`timestamp`, `user_id`, etc.)

```javascript
// Current and previous values
current_value = field_name
previous_value = prev_field_name

// Detect changes
if current_value != previous_value {
    return "field changed from " + previous_value + " to " + current_value
}
```

### Execution Ordering

**Dependency Resolution:**
Transforms execute in dependency order to ensure consistency.

```json
{
  "name": "UserMetrics",
  "fields": {
    "points": {"field_type": "Single"},
    "level": {
      "transform": {
        "inputs": ["points"],
        "logic": "return floor(points / 1000) + 1",
        "output": "UserMetrics.level"
      }
    },
    "tier": {
      "transform": {
        "inputs": ["level"],
        "logic": "if level >= 10 { return \"gold\" } else if level >= 5 { return \"silver\" } else { return \"bronze\" }",
        "output": "UserMetrics.tier"
      }
    }
  }
}
```

**Execution Order:**
1. `points` field updated
2. `level` transform executes (depends on `points`)
3. `tier` transform executes (depends on `level`)

### Error Handling

**Transform Failures:**
- Failed transforms log errors but don't prevent data updates
- Previous computed values are preserved on failure
- Error details available via monitoring APIs

**Error Recovery:**
```javascript
// Safe operations with fallbacks
result = tryOperation(risky_calculation) ?? default_value

// Validation before computation
if isValid(input_data) {
    return computeResult(input_data)
} else {
    return "invalid_input"
}
```

## Transform Registration

### Via Schema Definition

**Field-Level Transform:**
```json
{
  "name": "UserProfile",
  "fields": {
    "first_name": {"field_type": "Single"},
    "last_name": {"field_type": "Single"},
    "full_name": {
      "field_type": "Single",
      "transform": {
        "inputs": ["first_name", "last_name"],
        "logic": "return first_name + \" \" + last_name",
        "output": "UserProfile.full_name"
      },
      "writable": false
    }
  }
}
```

### Via HTTP API

**Register Standalone Transform:**
```bash
curl -X POST http://localhost:9001/api/transform/register \
  -H "Content-Type: application/json" \
  -d '{
    "name": "user_status_calculator",
    "inputs": ["UserProfile.age", "UserProfile.account_type"],
    "logic": "if account_type == \"premium\" { return \"premium_user\" } else if age >= 18 { return \"adult_user\" } else { return \"minor_user\" }",
    "output": "UserProfile.computed_status"
  }'
```

### Via CLI

**Register Transform:**
```bash
datafold_cli register-transform transform_definition.json
```

**Transform Definition File:**
```json
{
  "name": "conversion_calculator",
  "description": "Calculates conversion rate from events",
  "inputs": [
    "Analytics.conversions",
    "Analytics.total_events"
  ],
  "logic": "if total_events > 0 { return (conversions / total_events) * 100 } else { return 0 }",
  "output": "Analytics.conversion_rate",
  "enabled": true
}
```

### Transform Management

**List Transforms:**
```bash
curl http://localhost:9001/api/transforms
```

**Get Transform Details:**
```bash
curl http://localhost:9001/api/transform/user_status_calculator
```

**Update Transform:**
```bash
curl -X PUT http://localhost:9001/api/transform/user_status_calculator \
  -H "Content-Type: application/json" \
  -d '{
    "logic": "updated transform logic here",
    "enabled": true
  }'
```

**Delete Transform:**
```bash
curl -X DELETE http://localhost:9001/api/transform/user_status_calculator
```

## Dependency Management

### Dependency Graph

**Automatic Detection:**
The system automatically builds dependency graphs from transform definitions.

```json
{
  "transforms": [
    {
      "name": "base_calculation",
      "inputs": ["raw_data"],
      "output": "intermediate_result"
    },
    {
      "name": "derived_calculation", 
      "inputs": ["intermediate_result"],
      "output": "final_result"
    }
  ]
}
```

**Dependency Visualization:**
```
raw_data → base_calculation → intermediate_result → derived_calculation → final_result
```

### Circular Dependency Detection

**Validation:**
The system prevents circular dependencies during transform registration.

```json
{
  "error": {
    "code": "CIRCULAR_DEPENDENCY",
    "message": "Transform creates circular dependency",
    "cycle": ["field_a", "field_b", "field_c", "field_a"]
  }
}
```

### Cross-Schema Dependencies

**Dependencies Across Schemas:**
```json
{
  "name": "analytics_summary",
  "inputs": [
    "UserProfile.user_count",
    "EventAnalytics.total_events",
    "Sales.total_revenue"
  ],
  "logic": "return { \"users\": user_count, \"events\": total_events, \"revenue\": total_revenue }",
  "output": "Summary.dashboard_data"
}
```

## Performance Optimization

### Execution Optimization

**Lazy Evaluation:**
Transforms only execute when their outputs are requested or dependencies change.

**Parallel Execution:**
Independent transforms execute in parallel when possible.

**Caching:**
Transform results are cached and invalidated only when dependencies change.

### Performance Configuration

**Transform Settings:**
```json
{
  "performance": {
    "max_execution_time": 10000,
    "enable_caching": true,
    "cache_ttl": 3600,
    "parallel_execution": true,
    "max_parallel_transforms": 4
  }
}
```

**Per-Transform Settings:**
```json
{
  "name": "expensive_calculation",
  "inputs": ["large_dataset"],
  "logic": "complex calculation logic",
  "output": "result",
  "performance": {
    "cache_duration": 7200,
    "execution_timeout": 30000,
    "priority": "high"
  }
}
```

### Monitoring Transform Performance

**Performance Metrics:**
```bash
curl http://localhost:9001/api/transforms/metrics
```

**Response:**
```json
{
  "transforms": [
    {
      "name": "user_status_calculator",
      "executions": 1250,
      "avg_execution_time_ms": 5.2,
      "cache_hit_rate": 0.85,
      "error_rate": 0.001
    }
  ]
}
```

## Examples

### User Status Calculation

**Schema Definition:**
```json
{
  "name": "UserAccount",
  "fields": {
    "age": {"field_type": "Single"},
    "account_type": {"field_type": "Single"},
    "is_verified": {"field_type": "Single"},
    "account_status": {
      "field_type": "Single",
      "transform": {
        "inputs": ["age", "account_type", "is_verified"],
        "logic": "if !is_verified { return \"unverified\" } else if account_type == \"premium\" { return \"premium_verified\" } else if age >= 18 { return \"standard_adult\" } else { return \"standard_minor\" }",
        "output": "UserAccount.account_status"
      },
      "writable": false
    }
  }
}
```

### Analytics Calculations

**Schema Definition:**
```json
{
  "name": "SiteAnalytics",
  "fields": {
    "page_views": {"field_type": "Single"},
    "unique_visitors": {"field_type": "Single"},
    "conversions": {"field_type": "Single"},
    "conversion_rate": {
      "field_type": "Single",
      "transform": {
        "inputs": ["conversions", "unique_visitors"],
        "logic": "if unique_visitors > 0 { return round((conversions / unique_visitors) * 100, 2) } else { return 0 }",
        "output": "SiteAnalytics.conversion_rate"
      }
    },
    "pages_per_visitor": {
      "field_type": "Single",
      "transform": {
        "inputs": ["page_views", "unique_visitors"],
        "logic": "if unique_visitors > 0 { return round(page_views / unique_visitors, 2) } else { return 0 }",
        "output": "SiteAnalytics.pages_per_visitor"
      }
    }
  }
}
```

### Financial Calculations

**Complex Transform:**
```json
{
  "name": "investment_calculator",
  "inputs": [
    "Portfolio.principal",
    "Portfolio.interest_rate",
    "Portfolio.time_period",
    "Portfolio.compound_frequency"
  ],
  "logic": "principal * pow((1 + (interest_rate / compound_frequency)), (compound_frequency * time_period))",
  "output": "Portfolio.future_value"
}
```

### Inventory Management

**Stock Level Calculation:**
```json
{
  "name": "InventoryStatus",
  "fields": {
    "stock_levels": {"field_type": "Range"},
    "reorder_points": {"field_type": "Range"},
    "stock_status": {
      "field_type": "Range",
      "transform": {
        "inputs": ["stock_levels", "reorder_points"],
        "logic": "result = {}; for location in getKeys(stock_levels) { current_stock = getValue(stock_levels, location); reorder_point = getValue(reorder_points, location) ?? 0; if current_stock <= 0 { result[location] = \"out_of_stock\" } else if current_stock <= reorder_point { result[location] = \"low_stock\" } else { result[location] = \"in_stock\" } }; return result",
        "output": "InventoryStatus.stock_status"
      }
    }
  }
}
```

### Time-Series Aggregation

**Daily Summary Transform:**
```json
{
  "name": "daily_aggregator",
  "inputs": ["Metrics.hourly_data"],
  "logic": "daily_totals = {}; for key in getKeys(hourly_data) { if startsWith(key, \"2024-01-15:\") { date = substring(key, 0, 10); if !hasKey(daily_totals, date) { daily_totals[date] = 0 }; daily_totals[date] = daily_totals[date] + toNumber(getValue(hourly_data, key)) } }; return daily_totals",
  "output": "Metrics.daily_data"
}
```

## Best Practices

### Transform Design

**1. Keep Logic Simple:**
```javascript
// Good: Simple, readable logic
if age >= 18 { return "adult" } else { return "minor" }

// Avoid: Complex nested logic in transforms
// Better to break into multiple simpler transforms
```

**2. Handle Edge Cases:**
```javascript
// Check for null/undefined values
if username != null && length(username) > 0 {
    return upper(username)
} else {
    return "UNKNOWN"
}

// Avoid division by zero
if total_visits > 0 {
    return conversions / total_visits
} else {
    return 0
}
```

**3. Use Descriptive Names:**
```json
{
  "name": "calculate_user_tier_from_points",  // Good: descriptive
  "name": "calc_tier",                       // Bad: unclear
  "name": "transform_001"                    // Bad: meaningless
}
```

### Performance Considerations

**1. Minimize Dependencies:**
```json
{
  "inputs": ["required_field"],           // Good: only necessary inputs
  "inputs": ["field1", "field2", "field3", "field4"]  // Consider if all are needed
}
```

**2. Cache Expensive Operations:**
```json
{
  "performance": {
    "cache_duration": 3600,  // Cache for 1 hour
    "priority": "low"        // For non-critical calculations
  }
}
```

**3. Optimize Range Operations:**
```javascript
// Good: Efficient range operations
sumValues(metrics_range)

// Less efficient: Manual iteration
total = 0
for key in getKeys(metrics_range) {
    total = total + toNumber(getValue(metrics_range, key))
}
```

### Error Handling

**1. Provide Fallbacks:**
```javascript
// Safe operations with defaults
result = computeValue(input) ?? default_value
percentage = (total > 0) ? (part / total * 100) : 0
```

**2. Validate Inputs:**
```javascript
if isNumber(age) && age >= 0 && age <= 150 {
    return calculateCategory(age)
} else {
    return "invalid_age"
}
```

**3. Log Important Information:**
```javascript
// Include context in error cases
if invalid_input {
    log("Invalid input received: " + toString(input))
    return "error"
}
```

### Security Considerations

**1. Avoid External Dependencies:**
- Transforms run in a sandboxed environment
- No access to external APIs or file system
- Only built-in functions are available

**2. Validate Data Sources:**
```javascript
// Ensure data comes from trusted fields
if hasValidSource(input_field) {
    return processData(input_field)
} else {
    return "untrusted_source"
}
```

**3. Limit Resource Usage:**
```json
{
  "performance": {
    "execution_timeout": 5000,    // 5 second limit
    "memory_limit": "50MB",       // Memory constraints
    "max_iterations": 10000       // Loop limits
  }
}
```

---

**Next**: See [Network Operations](./network-operations.md) for distributed functionality documentation.