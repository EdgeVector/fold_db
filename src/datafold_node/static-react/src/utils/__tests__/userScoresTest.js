import { describe, it, expect } from 'vitest'
import {
  isRangeSchema,
  getRangeKey,
  getNonRangeKeyFields
} from '../rangeSchemaUtils'

// This is the actual UserScores schema structure from the file
const realUserScoresSchema = {
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
      "payment_config": {
        "base_multiplier": 1.0,
        "trust_distance_scaling": "None",
        "min_payment": null
      },
      "molecule_uuid": null,
      "field_type": "Range",
      "field_mappers": {},
      "transform": null,
      "writable": true
    },
    "game_scores": {
      "permission_policy": {
        "read_policy": { "Distance": 0 },
        "write_policy": { "Distance": 1 }
      },
      "payment_config": {
        "base_multiplier": 1.2,
        "trust_distance_scaling": {
          "Linear": {
            "slope": 0.5,
            "intercept": 0.0,
            "min_factor": 1.0
          }
        },
        "min_payment": 2
      },
      "molecule_uuid": null,
      "field_type": "Range",
      "field_mappers": {},
      "transform": null,
      "writable": true
    },
    "achievements": {
      "permission_policy": {
        "read_policy": { "Distance": 1 },
        "write_policy": { "Distance": 2 }
      },
      "payment_config": {
        "base_multiplier": 1.5,
        "trust_distance_scaling": {
          "Linear": {
            "slope": 1.0,
            "intercept": 0.0,
            "min_factor": 1.0
          }
        },
        "min_payment": 5
      },
      "molecule_uuid": null,
      "field_type": "Range",
      "field_mappers": {},
      "transform": null,
      "writable": true
    },
    "player_statistics": {
      "permission_policy": {
        "read_policy": { "Distance": 0 },
        "write_policy": { "Distance": 1 }
      },
      "payment_config": {
        "base_multiplier": 2.0,
        "trust_distance_scaling": {
          "Exponential": {
            "base": 1.5,
            "scale": 1.0,
            "min_factor": 1.0
          }
        },
        "min_payment": 10
      },
      "molecule_uuid": null,
      "field_type": "Range",
      "field_mappers": {},
      "transform": null,
      "writable": true
    },
    "ranking_data": {
      "permission_policy": {
        "read_policy": { "Distance": 1 },
        "write_policy": { "Distance": 3 }
      },
      "payment_config": {
        "base_multiplier": 3.0,
        "trust_distance_scaling": {
          "Exponential": {
            "base": 2.0,
            "scale": 1.0,
            "min_factor": 1.0
          }
        },
        "min_payment": 25
      },
      "molecule_uuid": null,
      "field_type": "Range",
      "field_mappers": {},
      "transform": {
        "inputs": [],
        "logic": "return calculate_rank_from_scores(game_scores)",
        "output": "UserScores.ranking_data"
      },
      "writable": false
    }
  },
  "payment_config": {
    "base_multiplier": 1.8,
    "min_payment_threshold": 3
  }
}

describe('UserScores Real Schema Test', () => {
  it('should correctly identify UserScores as a range schema', () => {
    expect(isRangeSchema(realUserScoresSchema)).toBe(true)
  })

  it('should extract the correct range key', () => {
    expect(getRangeKey(realUserScoresSchema)).toBe('user_id')
  })

  it('should get non-range-key fields correctly', () => {
    const nonRangeKeyFields = getNonRangeKeyFields(realUserScoresSchema)
    expect(nonRangeKeyFields).not.toHaveProperty('user_id')
    expect(nonRangeKeyFields).toHaveProperty('game_scores')
    expect(nonRangeKeyFields).toHaveProperty('achievements')
    expect(nonRangeKeyFields).toHaveProperty('player_statistics')
    expect(nonRangeKeyFields).toHaveProperty('ranking_data')
  })

  it('should verify all fields are Range type', () => {
    const allFieldsAreRange = Object.entries(realUserScoresSchema.fields)
      .every(([_, field]) => field.field_type === 'Range')
    expect(allFieldsAreRange).toBe(true)
  })
})
