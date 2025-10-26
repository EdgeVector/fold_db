const fs = require('fs');
const path = require('path');

// Load the actual UserScores schema
const userScoresPath = path.join(__dirname, '../../../tests/schemas_for_testing/UserScores.json');
const userScoresSchema = JSON.parse(fs.readFileSync(userScoresPath, 'utf8'));

// Import the detection functions
const { isRangeSchema, getRangeKey } = require('./src/utils/rangeSchemaUtils.js');

console.log('UserScores Schema Analysis:');
console.log('=========================');
console.log('Schema name:', userScoresSchema.name);
console.log('Has schema_type:', !!userScoresSchema.schema_type);
console.log('Has Range in schema_type:', !!userScoresSchema.schema_type?.Range);
console.log('Range key value:', getRangeKey(userScoresSchema));
console.log('Fields count:', Object.keys(userScoresSchema.fields || {}).length);

console.log('\nField types:');
Object.entries(userScoresSchema.fields).forEach(([name, field]) => {
  console.log(`  ${name}: ${field.field_type}`);
});

console.log('\nAll fields are Range type:', Object.entries(userScoresSchema.fields).every(([_, field]) => field.field_type === 'Range'));
console.log('isRangeSchema result:', isRangeSchema(userScoresSchema));

// Test with minimal structure
const minimalTest = {
  name: 'UserScores',
  schema_type: { Range: { range_key: 'user_id' } },
  fields: {
    user_id: { field_type: 'Range' },
    game_scores: { field_type: 'Range' }
  }
};

console.log('\nMinimal test result:', isRangeSchema(minimalTest));