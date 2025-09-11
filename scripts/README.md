# User Activity HashRange Schema

This directory contains a HashRange schema example and management script for DataFold.

## Files

- `available_schemas/UserActivity.json` - HashRange schema for tracking user activities
- `scripts/manage_user_activity.rs` - Rust script to populate and query user activity data
- `scripts/run_user_activity.sh` - Shell script to run the management script
- `scripts/Cargo.toml` - Cargo configuration for the script

## Schema Overview

The `UserActivity` schema is a HashRange schema that tracks user activities with:

- **Hash Key**: `user_id` - Groups activities by user
- **Range Key**: `timestamp` - Orders activities by time
- **Fields**:
  - `user_id` - The user identifier
  - `action` - The action performed (login, view_page, create_post, etc.)
  - `resource` - The resource accessed (/dashboard, /profile, etc.)
  - `timestamp` - When the activity occurred
  - `metadata` - Additional context (IP address, session info, etc.)

## Usage

### 1. Run the Management Script

```bash
# From the project root directory
./scripts/run_user_activity.sh
```

This will:
- Load and approve the UserActivity schema
- Create sample user activity data for 3 users
- Demonstrate HashRange queries by user_id and action
- Display formatted results

### 2. Manual Usage

You can also run the script directly:

```bash
cd scripts
cargo run --bin manage_user_activity
```

### 3. Query Examples

The script demonstrates several query patterns:

- **Query by user**: Find all activities for a specific user
- **Query by action**: Find all activities of a specific type (e.g., all logins)
- **Time-based queries**: Use range filters to find activities within time ranges

## Sample Data

The script creates sample data for 3 users:

- **user_001**: Web app user with login, page views, post creation, and logout
- **user_002**: Mobile app user with profile updates
- **user_003**: Web app user with search and tutorial viewing

## HashRange Benefits

This schema demonstrates the power of HashRange schemas:

1. **Efficient User Queries**: Quickly find all activities for a specific user
2. **Time Ordering**: Activities are naturally ordered by timestamp
3. **Action Filtering**: Can query by action type across all users
4. **Scalability**: Hash distribution allows for horizontal scaling

## Customization

To customize the schema or data:

1. **Modify the schema**: Edit `available_schemas/UserActivity.json`
2. **Add more fields**: Update the schema and regenerate data
3. **Change sample data**: Modify the `create_sample_activities()` function
4. **Add new query patterns**: Extend the query methods in the script

## Integration with HTTP Server

To use this schema with the HTTP server:

1. Start the HTTP server: `./run_http_server.sh`
2. The schema will be available for mutations and queries via the API
3. Use the same mutation/query patterns as shown in the script
