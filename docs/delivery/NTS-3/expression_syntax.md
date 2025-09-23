# Expression Syntax and Operator Documentation

The Native Transform System (NTS-3) includes a powerful expression evaluation engine that supports a rich set of operators, functions, and syntax elements. This document provides comprehensive documentation of the expression language.

## Table of Contents

- [Overview](#overview)
- [Literals and Values](#literals-and-values)
- [Variables and Field Access](#variables-and-field-access)
- [Operators](#operators)
- [Function Calls](#function-calls)
- [Operator Precedence](#operator-precedence)
- [Expression Examples](#expression-examples)
- [Error Handling](#error-handling)

## Overview

The expression language supports:
- **Arithmetic operations** with integers and floating-point numbers
- **String operations** including concatenation and manipulation
- **Comparison operations** for all data types
- **Logical operations** with short-circuit evaluation
- **Field access** for nested object and array structures
- **Function calls** to built-in and custom functions
- **Type coercion** for mixed-type operations

## Literals and Values

### Numeric Literals

```rust
// Integer literals
42
-10
0

// Floating-point literals
3.14
-2.5
1.0e10
```

### String Literals

```rust
// Double-quoted strings
"hello"
"hello world"
""

// Escaped characters
"hello \"world\""  // Contains quote
"line 1\nline 2"  // Contains newline
"tab\there"       // Contains tab
```

### Boolean Literals

```rust
true
false
```

### Null Literal

```rust
null
```

## Variables and Field Access

### Variable References

Variables reference fields from the input data:

```rust
name           // References the "name" field
age            // References the "age" field
user.email     // References nested field
scores.0       // References array element
```

### Field Access Syntax

#### Object Field Access

```rust
user.name              // Access "name" field of "user" object
user.profile.email     // Nested object access
settings.theme.color   // Deep nesting
```

#### Array Element Access

```rust
scores.0               // First element (0-based indexing)
scores.1               // Second element
items.10               // Eleventh element
```

#### Mixed Access

```rust
users.0.name           // Name field of first user in array
orders.5.items.0       // First item of sixth order
```

## Operators

### Arithmetic Operators

| Operator | Description | Example | Result |
|----------|-------------|---------|---------|
| `+` | Addition | `2 + 3` | `5` |
| `-` | Subtraction | `5 - 2` | `3` |
| `*` | Multiplication | `3 * 4` | `12` |
| `/` | Division | `8 / 2` | `4.0` |
| `%` | Modulo | `7 % 3` | `1` |
| `^` | Power/Exponentiation | `2 ^ 3` | `8` |

#### Arithmetic Type Rules

- **Integer + Integer** → Integer
- **Number + Number** → Number
- **Integer + Number** → Number
- **String + Any** → String (concatenation)
- **Any + String** → String (concatenation)

### Comparison Operators

| Operator | Description | Example | Result |
|----------|-------------|---------|---------|
| `==` | Equal | `5 == 5` | `true` |
| `!=` | Not equal | `5 != 3` | `true` |
| `<` | Less than | `3 < 5` | `true` |
| `<=` | Less than or equal | `3 <= 5` | `true` |
| `>` | Greater than | `5 > 3` | `true` |
| `>=` | Greater than or equal | `5 >= 5` | `true` |

#### Comparison Type Rules

- **Numbers**: Standard numeric comparison
- **Strings**: Lexicographic comparison
- **Booleans**: `false < true`
- **Mixed types**: Type coercion to string for comparison
- **Null**: `null == null` is `true`, `null == anything_else` is `false`

### Logical Operators

| Operator | Description | Example | Result |
|----------|-------------|---------|---------|
| `&&` | Logical AND | `true && false` | `false` |
| `\|\|` | Logical OR | `true \|\| false` | `true` |
| `!` | Logical NOT | `!true` | `false` |

#### Logical Evaluation Rules

- **Short-circuit evaluation**: Left operand evaluated first
- **Truthy values**: Numbers ≠ 0, non-empty strings, non-empty arrays/objects, `true`
- **Falsy values**: 0, 0.0, empty string, empty array/object, `false`, `null`

### String Concatenation

The `+` operator performs string concatenation when either operand is a string:

```rust
"hello" + "world"           // "helloworld"
"age: " + 25               // "age: 25"
123 + " is a number"       // "123 is a number"
true + " is truthy"        // "true is truthy"
```

## Function Calls

### Basic Function Calls

```rust
uppercase("hello")         // Function call with string argument
length("hello world")      // Function call with expression
sum([1, 2, 3, 4])         // Function call with array literal
```

### Nested Function Calls

```rust
uppercase(substring("hello world", 0, 5))  // "HELLO"
length(to_string(42))                      // 2
sum([length("a"), length("bb"), length("ccc")])  // 6
```

### Function Calls in Expressions

```rust
length("hello") + 10                       // 15
to_string(sum([1, 2, 3])) + " total"       // "6 total"
uppercase("hello") == "HELLO"             // true
```

## Operator Precedence

Operators are evaluated in the following order (highest to lowest precedence):

### Precedence Table

| Precedence | Operators | Description | Associativity |
|------------|-----------|-------------|---------------|
| 1 | `.` | Field access | Left-to-right |
| 2 | `!` `-` | Unary operators | Right-to-left |
| 3 | `^` | Power/Exponentiation | Right-to-left |
| 4 | `*` `/` `%` | Multiplication, Division, Modulo | Left-to-right |
| 5 | `+` `-` | Addition, Subtraction | Left-to-right |
| 6 | `==` `!=` `<` `<=` `>` `>=` | Comparison operators | Left-to-right |
| 7 | `&&` | Logical AND | Left-to-right |
| 8 | `\|\|` | Logical OR | Left-to-right |
| 9 | Function calls | Function invocation | Left-to-right |

### Precedence Examples

#### Field Access (Highest Precedence)

```rust
user.name.length()  // Equivalent to: (user.name).length()
users.0.score + 1   // Field access before addition
```

#### Unary Operators

```rust
!true && false      // Equivalent to: (!true) && false
-5 + 3              // Equivalent to: (-5) + 3
```

#### Power Operations

```rust
2 ^ 3 * 4           // Equivalent to: (2 ^ 3) * 4 = 8 * 4 = 32
2 * 3 ^ 2           // Equivalent to: 2 * (3 ^ 2) = 2 * 9 = 18
```

#### Multiplication/Division

```rust
2 + 3 * 4           // Equivalent to: 2 + (3 * 4) = 14
8 / 2 + 1           // Equivalent to: (8 / 2) + 1 = 5
```

#### Addition/Subtraction

```rust
1 + 2 - 3           // Equivalent to: (1 + 2) - 3 = 0
10 - 5 + 2          // Equivalent to: (10 - 5) + 2 = 7
```

#### Comparisons

```rust
5 > 3 == true       // Equivalent to: (5 > 3) == true
a > b && b > c      // Equivalent to: (a > b) && (b > c)
```

#### Logical Operations

```rust
true || false && false  // Equivalent to: true || (false && false) = true
a && b || c && d        // Equivalent to: (a && b) || (c && d)
```

### Using Parentheses

Parentheses can override the default precedence:

```rust
// Without parentheses (default precedence)
2 + 3 * 4           // 2 + 12 = 14

// With parentheses (explicit precedence)
(2 + 3) * 4         // 5 * 4 = 20
```

## Expression Examples

### Arithmetic Expressions

```rust
// Basic arithmetic
1 + 2 * 3           // 7
(1 + 2) * 3         // 9
10 / 3              // 3.333...
10 % 3              // 1
2 ^ 3               // 8

// Mixed types
10 + 5.5            // 15.5
"prefix" + 123      // "prefix123"
```

### Comparison Expressions

```rust
// Numeric comparisons
age >= 18
score > 80.0
count != 0

// String comparisons
name == "admin"
status != "inactive"

// Boolean comparisons
active == true
is_admin != false
```

### Logical Expressions

```rust
// Simple logical operations
active && age >= 18
is_admin || is_moderator

// Complex logical expressions
(age >= 18 && active) || is_admin
!(age < 18) && has_permission
has_access && (is_owner || is_admin)
```

### String Operations

```rust
// String concatenation
first_name + " " + last_name
"User: " + username

// String functions with expressions
uppercase(first_name + " " + last_name)
length(first_name) + length(last_name)
```

### Field Access Expressions

```rust
// Object field access
user.name
user.profile.email
settings.theme.color

// Array element access
scores.0
items.1.price
users.0.name

// Mixed access
users.0.profile.name
orders.5.items.0.name
```

### Function Call Expressions

```rust
// Basic function calls
length("hello")
sum([1, 2, 3])
uppercase("hello")

// Function calls with field access
length(user.name)
sum(scores)
uppercase(user.first_name + " " + user.last_name)

// Nested function calls
uppercase(substring("hello world", 0, 5))
sum([length("a"), length("bb"), length("ccc")])
```

### Complex Expressions

```rust
// Complex arithmetic
(price * quantity) + (tax * (price * quantity))

// Complex logical
(age >= 18 && active) || (is_admin && !suspended)

// Mixed operations
to_string(score) + " points" + (is_bonus ? " (bonus)" : "")

// Nested field access with operations
(user.age * 2) + (user.profile.level * 10)
```

## Error Handling

### Common Expression Errors

1. **Syntax Errors**
   ```rust
   // Unclosed parentheses
   (a + b * c

   // Missing operators
   a b + c

   // Invalid characters
   a @ b
   ```

2. **Type Errors**
   ```rust
   // Incompatible types
   "hello" - 5

   // Invalid field access
   null.field
   42.name
   ```

3. **Runtime Errors**
   ```rust
   // Division by zero
   10 / 0

   // Array index out of bounds
   scores.10  // if scores has only 3 elements

   // Function errors
   length(null)
   sum("not_a_number")
   ```

4. **Field Access Errors**
   ```rust
   // Non-existent field
   user.nonexistent

   // Invalid array index
   scores.-1
   scores.abc  // non-numeric index

   // Wrong access type
   array.field  // trying to access field on array
   ```

### Error Recovery

1. **Use Safe Operators**
   ```rust
   // Safe division
   total > 0 ? amount / total : 0

   // Safe field access
   user != null ? user.name : "Unknown"
   ```

2. **Validate Array Bounds**
   ```rust
   // Check array length before access
   length(scores) > 0 ? scores.0 : null
   ```

3. **Handle Null Values**
   ```rust
   // Null-safe operations
   user != null && user.active == true
   user != null ? user.name : "default"
   ```

4. **Type Checking**
   ```rust
   // Ensure correct types
   to_number(field) + 10
   to_string(field) + "_suffix"
   ```

## Best Practices

### Performance

1. **Minimize Function Calls**: Use direct operations when possible
   ```rust
   // Better: Direct concatenation
   first_name + " " + last_name

   // Avoid: Multiple function calls
   concat([first_name, " ", last_name])
   ```

2. **Use Parentheses for Clarity**: Make complex expressions readable
   ```rust
   // Clear precedence
   (age >= 18) && (active == true) && (score > 80)

   // Ambiguous precedence
   age >= 18 && active == true && score > 80
   ```

3. **Optimize Field Access**: Cache frequently accessed nested fields
   ```rust
   // Better: Single access
   user.profile.name + " " + user.profile.email

   // Avoid: Multiple traversals
   user.profile.name + " " + user.profile.email
   ```

### Readability

1. **Use Descriptive Expressions**: Make intent clear
   ```rust
   // Clear intent
   age >= 18 && active == true

   // Unclear intent
   age > 17 && active
   ```

2. **Break Complex Expressions**: Use intermediate variables when needed
   ```rust
   // Complex single expression (hard to read)
   (user.age >= 18 && user.active) || (user.is_admin && !user.suspended)

   // Multiple simple expressions (easier to read)
   is_adult = user.age >= 18 && user.active
   is_authorized = user.is_admin && !user.suspended
   is_adult || is_authorized
   ```

3. **Consistent Formatting**: Use consistent spacing and line breaks
   ```rust
   // Consistent style
   (age >= 18) && (active == true)
   user.name + " (" + user.email + ")"

   // Inconsistent style (avoid)
   ( age>=18 ) && (active==true)
   user.name+" ("+user.email+")"
   ```

### Safety

1. **Handle Edge Cases**: Consider null values, empty arrays, etc.
   ```rust
   // Safe operations
   user != null ? user.age : 0
   length(scores) > 0 ? scores.0 : null
   total != 0 ? amount / total : 0
   ```

2. **Validate Assumptions**: Don't assume field existence
   ```rust
   // Safe field access
   user != null && user.profile != null ? user.profile.email : null

   // Unsafe field access (avoid)
   user.profile.email  // Might panic if user or profile is null
   ```

3. **Use Type Conversion Functions**: When mixing types
   ```rust
   // Safe type conversion
   to_number(string_field) + 10
   to_string(number_field) + "_items"

   // Potentially unsafe (avoid)
   string_field + 10  // Might not convert as expected
   ```

This comprehensive expression syntax documentation provides all the tools needed to create powerful, type-safe expressions in the Native Transform System.