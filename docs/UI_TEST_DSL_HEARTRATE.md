# UI Test: DSL Transformation (Average Heart Rate)

This document describes how to verify the DSL refactoring and the new `average()` reducer function using the FoldDB UI.

## Scenario

We are ingesting daily health summary records. Each record contains a list of raw heart rate samples (BPM) collected throughout the day. We want the database to automatically calculate the **average heart rate** for that day and store it as a top-level field.

## 1. Prepare Sample Data

Save the following JSON as `heart_rate_samples.json` or copy it to your clipboard.

```json
[
  {
    "user_id": "user_123",
    "date": "2023-10-27",
    "raw_readings": [60, 65, 70, 75, 80, 85, 90, 85, 80, 75, 70, 65],
    "notes": "Morning jog included"
  },
  {
    "user_id": "user_456",
    "date": "2023-10-27",
    "raw_readings": [55, 58, 60, 62, 60, 58],
    "notes": "Sedentary day"
  }
]
```

## 2. Configure Schema (DSL Instructions)

In the FoldDB UI **Ingestion** page (or using the Schema API), use the following Schema definition. Note the `transform_fields` section using the new DSL syntax.

```json
{
  "name": "DailyHealthSummary",
  "descriptive_name": "Daily Health Metrics and Averages",
  "key": {
    "range_field": "user_id"
  },
  "fields": [
    "user_id",
    "date",
    "raw_readings",
    "notes",
    "avg_bpm",
    "max_bpm",
    "min_bpm",
    "reading_count"
  ],
  "transform_fields": {
    "avg_bpm": "DailyHealthSummary.raw_readings.split_array().average()",
    "max_bpm": "DailyHealthSummary.raw_readings.split_array().max()",
    "min_bpm": "DailyHealthSummary.raw_readings.split_array().min()",
    "reading_count": "DailyHealthSummary.raw_readings.split_array().count()"
  },
  "field_topologies": {
    "user_id": {
      "root": {
        "type": "Primitive",
        "value": "String",
        "classifications": ["word"]
      }
    },
    "date": {
      "root": {
        "type": "Primitive",
        "value": "String",
        "classifications": ["word"]
      }
    },
    "raw_readings": {
      "root": {
        "type": "Array",
        "value": {
          "type": "Primitive",
          "value": "Number",
          "classifications": []
        }
      }
    },
    "notes": {
      "root": {
        "type": "Primitive",
        "value": "String",
        "classifications": ["word"]
      }
    },
    "avg_bpm": {
      "root": {
        "type": "Primitive",
        "value": "Number",
        "classifications": ["word"]
      }
    },
    "max_bpm": {
      "root": {
        "type": "Primitive",
        "value": "Number",
        "classifications": ["word"]
      }
    },
    "min_bpm": {
      "root": {
        "type": "Primitive",
        "value": "Number",
        "classifications": ["word"]
      }
    },
    "reading_count": {
      "root": {
        "type": "Primitive",
        "value": "Number",
        "classifications": ["word"]
      }
    }
  }
}
```

### DSL Explanation

- `DailyHealthSummary.raw_readings` : Accesses the array of numbers for each row (Implicit 1:1).
- `.split_array()` : Iterates over the array elements (1:N expansion).
- `.average()` : Reducer that calculates the mean of the numbers (N:1 reduction).

This chain results in a single numeric value for each row, perfectly populating the `avg_bpm` field.

## 3. Verify Results

After ingestion:

1.  Navigate to the **Search** or **Data Browser** page.
2.  Search for `DailyHealthSummary`.
3.  Verify the `avg_bpm` field is populated correctly:
    - **user_123**: Average of [60..65] (approx **75**)
    - **user_456**: Average of [55..58] (approx **58.83**)

## 4. Troubleshooting

If the `avg_bpm` field is empty, check the `transform_errors` log or ensure the DSL parser has been updated to support `average()` and implicit cardinality.
