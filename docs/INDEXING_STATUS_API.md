# Indexing Status API

## Overview

The Indexing Status API provides real-time information about background indexing operations. This allows the UI to display indexing progress and status to users.

## API Endpoint

### GET `/api/indexing/status`

Returns the current status of the background indexing system.

**Response:**

```json
{
  "state": "Idle" | "Indexing",
  "operations_in_progress": 0,
  "total_operations_processed": 1250,
  "operations_queued": 0,
  "last_operation_time": 1729885200,
  "avg_processing_time_ms": 2.5,
  "current_batch_size": null,
  "current_batch_start_time": null
}
```

**Fields:**

- `state`: Current state of the indexing system
  - `"Idle"`: No indexing operations in progress
  - `"Indexing"`: Currently processing index operations

- `operations_in_progress`: Number of operations currently being processed

- `total_operations_processed`: Total number of index operations processed since server startup

- `operations_queued`: Number of operations waiting to be processed (currently always 0 with fire-and-forget pattern)

- `last_operation_time`: Unix timestamp (seconds) of the last completed indexing operation

- `avg_processing_time_ms`: Exponential moving average of processing time per operation in milliseconds

- `current_batch_size`: Size of the batch currently being processed (null if idle)

- `current_batch_start_time`: Unix timestamp (seconds) when the current batch started (null if idle)

## Usage Examples

### JavaScript/TypeScript

```typescript
async function getIndexingStatus() {
  const response = await fetch('/api/indexing/status');
  const status = await response.json();
  
  if (status.state === 'Indexing') {
    console.log(`Indexing ${status.operations_in_progress} operations...`);
  } else {
    console.log('Indexing is idle');
  }
  
  console.log(`Total processed: ${status.total_operations_processed}`);
  console.log(`Avg time: ${status.avg_processing_time_ms}ms per operation`);
}
```

### React Component Example

```tsx
import { useState, useEffect } from 'react';

interface IndexingStatus {
  state: 'Idle' | 'Indexing';
  operations_in_progress: number;
  total_operations_processed: number;
  avg_processing_time_ms: number;
}

function IndexingStatusIndicator() {
  const [status, setStatus] = useState<IndexingStatus | null>(null);
  
  useEffect(() => {
    const fetchStatus = async () => {
      const response = await fetch('/api/indexing/status');
      const data = await response.json();
      setStatus(data);
    };
    
    // Poll every 500ms
    const interval = setInterval(fetchStatus, 500);
    fetchStatus(); // Initial fetch
    
    return () => clearInterval(interval);
  }, []);
  
  if (!status) return null;
  
  return (
    <div className="indexing-status">
      {status.state === 'Indexing' ? (
        <div className="indexing-active">
          <span className="spinner" />
          Indexing {status.operations_in_progress} operations...
        </div>
      ) : (
        <div className="indexing-idle">
          ✓ All indexed ({status.total_operations_processed} total)
        </div>
      )}
      <div className="stats">
        Avg: {status.avg_processing_time_ms.toFixed(2)}ms per operation
      </div>
    </div>
  );
}
```

### Python Example

```python
import requests
import time

def monitor_indexing():
    while True:
        response = requests.get('http://localhost:9001/api/indexing/status')
        status = response.json()
        
        if status['state'] == 'Indexing':
            print(f"⏳ Indexing {status['operations_in_progress']} operations...")
        else:
            print(f"✓ Idle - {status['total_operations_processed']} total operations processed")
        
        time.sleep(0.5)
```

## UI Integration Tips

### 1. Polling Strategy

For real-time updates, poll the endpoint every 500ms-1000ms:

```typescript
// Poll every 500ms when active, every 5s when idle
const pollInterval = status?.state === 'Indexing' ? 500 : 5000;
```

### 2. Visual Indicators

Show different UI states based on indexing status:

- **Idle**: Green checkmark, "All indexed"
- **Indexing**: Spinner/progress bar, "Indexing N operations..."

### 3. Performance Metrics

Display useful metrics to users:

```typescript
function formatMetrics(status: IndexingStatus) {
  return {
    throughput: `${(1000 / status.avg_processing_time_ms).toFixed(0)} ops/sec`,
    avgTime: `${status.avg_processing_time_ms.toFixed(2)}ms`,
    total: status.total_operations_processed.toLocaleString()
  };
}
```

### 4. Progress Estimation

If you know the total number of operations expected:

```typescript
const progress = status.operations_in_progress > 0
  ? ((totalExpected - status.operations_in_progress) / totalExpected) * 100
  : 100;
```

## Implementation Details

### Status Tracking

The `IndexStatusTracker` maintains real-time status information:

```rust
pub struct IndexingStatus {
    pub state: IndexingState,
    pub operations_in_progress: usize,
    pub total_operations_processed: u64,
    pub operations_queued: usize,
    pub last_operation_time: Option<u64>,
    pub avg_processing_time_ms: f64,
    pub current_batch_size: Option<usize>,
    pub current_batch_start_time: Option<u64>,
}
```

### Exponential Moving Average

The `avg_processing_time_ms` uses an exponential moving average (EMA) with alpha=0.3, giving more weight to recent operations:

```
new_avg = 0.3 * current_batch_avg + 0.7 * previous_avg
```

This provides a smooth, responsive average that adapts to changing performance characteristics.

### Thread Safety

The status tracker uses `Arc<RwLock<IndexingStatus>>` for thread-safe access from both the indexing thread and HTTP request handlers.

## Performance Considerations

- **Lightweight**: Status queries are fast (< 1ms) and don't impact indexing performance
- **No blocking**: Reading status never blocks indexing operations
- **Minimal overhead**: Status updates add < 0.1ms overhead per batch

## Future Enhancements

Potential future additions:

1. **Queue depth tracking**: Track number of pending BatchIndexRequest events
2. **Per-schema metrics**: Break down statistics by schema
3. **Historical data**: Track indexing performance over time
4. **Alerts**: Notify when indexing falls behind or fails

## Example Response States

### Idle State

```json
{
  "state": "Idle",
  "operations_in_progress": 0,
  "total_operations_processed": 5420,
  "operations_queued": 0,
  "last_operation_time": 1729885200,
  "avg_processing_time_ms": 2.3,
  "current_batch_size": null,
  "current_batch_start_time": null
}
```

### Active Indexing

```json
{
  "state": "Indexing",
  "operations_in_progress": 500,
  "total_operations_processed": 5420,
  "operations_queued": 0,
  "last_operation_time": 1729885200,
  "avg_processing_time_ms": 2.3,
  "current_batch_size": 500,
  "current_batch_start_time": 1729885205
}
```

### Fresh Start

```json
{
  "state": "Idle",
  "operations_in_progress": 0,
  "total_operations_processed": 0,
  "operations_queued": 0,
  "last_operation_time": null,
  "avg_processing_time_ms": 0.0,
  "current_batch_size": null,
  "current_batch_start_time": null
}
```

