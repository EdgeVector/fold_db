// Simple inline test
const userScoresSchema = {
  "name": "UserScores",
  "schema_type": {
    "Range": {
      "range_key": "user_id"
    }
  },
  "fields": {
    "user_id": {
      "permission_policy": {
        "read_policy": { "Distance": 0 },
        "write_policy": { "Distance": 2 }
      },
      "field_type": "Range",
      "writable": true
    },
    "game_scores": {
      "permission_policy": {
        "read_policy": { "Distance": 0 },
        "write_policy": { "Distance": 1 }
      },
      "field_type": "Range",
      "writable": true
    },
    "achievements": {
      "permission_policy": {
        "read_policy": { "Distance": 1 },
        "write_policy": { "Distance": 2 }
      },
      "field_type": "Range",
      "writable": true
    },
    "player_statistics": {
      "permission_policy": {
        "read_policy": { "Distance": 0 },
        "write_policy": { "Distance": 1 }
      },
      "field_type": "Range",
      "writable": true
    },
    "ranking_data": {
      "permission_policy": {
        "read_policy": { "Distance": 1 },
        "write_policy": { "Distance": 3 }
      },
      "field_type": "Range",
      "writable": false
    }
  }
};

// Test the current logic
function isRangeSchema(schema) {
  if (!schema) {
    return false
  }
  
  // Check for range_key in the new schema_type structure or old format
  const hasRangeKey = schema.schema_type?.Range?.range_key || schema.range_key
  if (!hasRangeKey) {
    return false
  }
  
  if (!schema.fields) {
    return false
  }
  
  // Check if all fields have field_type: "Range"
  const fieldEntries = Object.entries(schema.fields)
  if (fieldEntries.length === 0) {
    return false
  }
  
  return fieldEntries.every(([_, field]) => field.field_type === 'Range')
}

function getRangeKey(schema) {
  return schema?.schema_type?.Range?.range_key || schema?.range_key || null
}

console.log('=== UserScores Schema Analysis ===');
console.log('Has schema_type.Range.range_key:', !!userScoresSchema.schema_type?.Range?.range_key);
console.log('Range key value:', getRangeKey(userScoresSchema));
console.log('Fields count:', Object.keys(userScoresSchema.fields || {}).length);

console.log('\nField types:');
Object.entries(userScoresSchema.fields).forEach(([name, field]) => {
  console.log(`  ${name}: ${field.field_type}`);
});

console.log('\nAll fields are Range type:', Object.entries(userScoresSchema.fields).every(([_, field]) => field.field_type === 'Range'));
console.log('isRangeSchema result:', isRangeSchema(userScoresSchema));