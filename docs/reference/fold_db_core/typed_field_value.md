# Typed Field Value Wrappers

The type-safe field value wrappers introduced in **TSW-1** provide a reliable
bridge between the flexible JSON payloads exchanged across FoldDB and the
strongly typed expectations of the Rust services that consume them.

## Core Types

- `TypedFieldValue` wraps a `serde_json::Value` and exposes typed accessors.
- `FieldType` classifies JSON data as `string`, `integer`, `float`, `boolean`,
  `array`, `object`, etc., enabling precise validation and error reporting.

The wrapper retains the original JSON payload so existing serialization remains
unchanged while enabling consumers to opt-in to strict type checks whenever they
need them.

## Accessor Methods

`TypedFieldValue` surfaces ergonomic accessors that validate the payload before
returning data:

- `as_string()`, `as_bool()`, `as_number()`, `as_i64()`, `as_u64()`, `as_f64()`
- `as_array()` / `into_typed_array()`
- `as_object()` / `into_typed_object()` (returns a `BTreeMap<String, TypedFieldValue>`)

Each method returns a `SchemaError::InvalidData` with detailed context if the
payload does not match the requested type.

## Validation Helpers

Two helper methods make it easy to ensure an incoming payload matches a desired
shape:

- `ensure_type(expected)` – Validates against a `FieldType` and returns `Ok(())`
  on success.
- `ensure_type_with_context(expected, context)` – Same validation but includes a
  fully qualified field name (e.g. `"User.email"`) in the error message.

## Message Bus Integration

`FieldValueSetRequest` and `FieldUpdateRequest` now expose helper constructors
and accessors that leverage the typed wrappers:

- `typed_value()` – Returns a `TypedFieldValue` clone of the payload.
- `typed_value_with_validation(expected)` – Extracts the typed value and
  validates it against an expected `FieldType`, automatically including the
  request's `field_name` in any error message.
- `from_typed_value(...)` – Creates a new request while automatically converting
  the typed payload back into JSON for transport.

These helpers ensure API boundary code can validate incoming mutations without
re-implementing conversion logic or duplicating error handling.

## Usage Example

```rust
use fold_db::field_value::{TypedFieldValue, ValueFieldType};
use fold_db::fold_db_core::infrastructure::message_bus::request_events::FieldValueSetRequest;

let request = FieldValueSetRequest::from_typed_value(
    "corr-123".into(),
    "User".into(),
    "User.email".into(),
    TypedFieldValue::from("user@example.com"),
    "pubkey".into(),
    None,
);

let typed_value = request
    .typed_value_with_validation(ValueFieldType::String)?;
println!("Email: {}", typed_value.as_string()?);
```

The resulting workflow keeps JSON payloads compatible with existing consumers
while ensuring type mismatches are caught immediately with actionable errors.
