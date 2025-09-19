# Payment Requirements Integration

## Overview
This document outlines the integration of per-query payment requirements into FoldDB's schema system using Bitcoin Lightning Network payments. The payment system will be implemented at both schema and field levels, with trust distance scaling factors.

## Core Components

### 1. Payment Structure
```rust
pub struct GlobalPaymentConfig {
    pub system_base_rate: u64,  // Minimum system-wide base rate in sats
    pub payment_timeout: Duration,  // Global payment timeout
    pub max_invoice_retries: u32,  // Maximum invoice regeneration attempts
    pub hold_invoice_timeout: Duration,  // Timeout for hold invoices
}

pub struct SchemaPaymentConfig {
    pub base_multiplier: f64,  // Multiplier applied to market base rate (in sats)
    pub min_payment_threshold: u64,  // Minimum payment required in sats
}

pub struct MarketRate {
    pub base_rate: u64,  // Current market base rate in sats
    pub last_updated: DateTime<Utc>,
}

pub struct LightningPaymentRequest {
    pub amount: u64,  // Amount in sats
    pub invoice: String,  // Lightning invoice
    pub expiry: DateTime<Utc>,
    pub payment_hash: String,  // For tracking payment status
    pub hold_invoice: bool,  // Whether this is a hold invoice
}

pub struct PaymentState {
    pub invoice_id: String,
    pub status: PaymentStatus,
    pub created_at: DateTime<Utc>,
    pub last_checked: DateTime<Utc>,
    pub retry_count: u32,
}

pub enum PaymentStatus {
    Pending,
    Settled,
    Expired,
    Failed,
    PartiallyPaid(u64),  // Amount paid so far
    Cancelled,
}
```

### 2. Field Payment Structure
```rust
pub struct FieldPaymentConfig {
    pub base_multiplier: f64,  // Multiplier applied to schema base payment
    pub trust_distance_scaling: TrustDistanceScaling,
    pub min_payment: Option<u64>,  // Minimum payment for this field in sats
}

pub enum TrustDistanceScaling {
    Linear {
        slope: f64,
        intercept: f64,
        min_factor: f64,  // Minimum scaling factor (>= 1.0)
    },
    Exponential {
        base: f64,
        scale: f64,
        min_factor: f64,  // Minimum scaling factor (>= 1.0)
    },
    None
}
```

## Integration Points

### 1. Schema Modifications
```rust
pub struct Schema {
    pub name: String,
    pub fields: HashMap<String, SchemaField>,
    pub transforms: Vec<String>,
    pub payment_config: SchemaPaymentConfig,
}
```

### 2. SchemaField Modifications
```rust
pub struct SchemaField {
    pub permission_policy: PermissionsPolicy,
    pub molecule_uuid: String,
    pub payment_config: FieldPaymentConfig,
}
```

## Payment Calculation

### 1. Base Formula
```
Final payment = max(
    Global system base rate,
    sum(
        for each field:
            Market base rate 
            * Schema multiplier 
            * Field multiplier 
            * max(Trust distance scaling factor, 1.0)
    )
)
```

### 2. Trust Distance Scaling
Two scaling options with safety bounds:

1. Linear Scaling:
   ```
   scale_factor = max(slope * trust_distance + intercept, min_factor)
   ```

2. Exponential Scaling:
   ```
   scale_factor = max(base^(scale * trust_distance), min_factor)
   ```

### 3. Example Implementation
```rust
fn calculate_field_payment(
    global_config: &GlobalPaymentConfig,
    market_rate: &MarketRate,
    schema_payment: &SchemaPaymentConfig,
    field_payment: &FieldPaymentConfig,
    trust_distance: f64
) -> u64 {
    let base = market_rate.base_rate;
    let schema_multiplier = schema_payment.base_multiplier;
    let field_multiplier = field_payment.base_multiplier;
    
    let scale_factor = match &field_payment.trust_distance_scaling {
        TrustDistanceScaling::Linear { slope, intercept, min_factor } => {
            (slope * trust_distance + intercept).max(*min_factor)
        },
        TrustDistanceScaling::Exponential { base, scale, min_factor } => {
            base.powf(scale * trust_distance).max(*min_factor)
        },
        TrustDistanceScaling::None => 1.0
    };
    
    // Convert to u64 after calculation to maintain precision
    let payment = (base as f64 * schema_multiplier * field_multiplier * scale_factor).round() as u64;
    
    // Apply minimum thresholds
    let field_min = field_payment.min_payment.unwrap_or(0);
    let schema_min = schema_payment.min_payment_threshold;
    let system_min = global_config.system_base_rate;
    
    payment.max(field_min).max(schema_min).max(system_min)
}

#[derive(Debug)]
pub struct PaymentManager {
    invoice_states: Arc<RwLock<HashMap<String, PaymentState>>>,
    lightning_client: Arc<LightningClient>,
}

impl PaymentManager {
    async fn generate_invoice(
        &self,
        amount: u64,
        memo: String,
        hold_invoice: bool
    ) -> Result<LightningPaymentRequest, Error> {
        let timeout = if hold_invoice {
            self.config.hold_invoice_timeout
        } else {
            self.config.payment_timeout
        };
        
        let invoice = self.lightning_client.create_invoice(
            amount,
            memo,
            timeout,
            hold_invoice
        ).await?;
        
        // Track invoice state
        let state = PaymentState {
            invoice_id: invoice.payment_hash.clone(),
            status: PaymentStatus::Pending,
            created_at: Utc::now(),
            last_checked: Utc::now(),
            retry_count: 0,
        };
        
        self.invoice_states.write().await.insert(
            invoice.payment_hash.clone(),
            state
        );
        
        Ok(invoice)
    }
    
    async fn verify_payment(&self, invoice: &str) -> Result<bool, Error> {
        let mut states = self.invoice_states.write().await;
        let state = states.get_mut(invoice).ok_or(Error::InvalidInvoice)?;
        
        // Update last checked timestamp
        state.last_checked = Utc::now();
        
        // Check if expired
        if Utc::now() > state.created_at + self.config.payment_timeout {
            state.status = PaymentStatus::Expired;
            return Ok(false);
        }
        
        // Verify with Lightning node
        match self.lightning_client.check_payment(invoice).await? {
            PaymentStatus::Settled => {
                state.status = PaymentStatus::Settled;
                Ok(true)
            },
            PaymentStatus::PartiallyPaid(amount) => {
                state.status = PaymentStatus::PartiallyPaid(amount);
                Ok(false)
            },
            status => {
                state.status = status;
                Ok(false)
            }
        }
    }
}
```

## Implementation Steps

1. Create new module structure:
   ```
   src/fees/
   ├── mod.rs
   ├── payment_config.rs
   ├── payment_calculator.rs
   ├── payment_manager.rs
   ├── lightning/
   │   ├── mod.rs
   │   ├── client.rs
   │   └── types.rs
   └── types.rs
   ```

2. Implement robust Lightning node integration:
   - Secure connection management
   - Automatic reconnection handling
   - Node health monitoring
   - Backup node failover
   - Payment state persistence

3. Implement payment state management:
   - Thread-safe invoice tracking
   - Payment status monitoring
   - Timeout handling
   - Partial payment tracking
   - Hold invoice management

4. Add payment validation pipeline:
   - Calculate total required payment
   - Generate appropriate invoice type
   - Monitor payment status with timeouts
   - Handle payment verification atomically
   - Support payment retries and invoice regeneration

5. Implement security measures:
   - Payment race condition prevention
   - Atomic payment verification
   - Invoice expiry enforcement
   - Payment state consistency checks
   - Node backup procedures

## Usage Example

```rust
let global_config = GlobalPaymentConfig {
    system_base_rate: 50,  // Minimum 50 sats for any query
    payment_timeout: Duration::from_secs(3600),
    max_invoice_retries: 3,
    hold_invoice_timeout: Duration::from_secs(7200),
};

let market_rate = MarketRate {
    base_rate: 100,  // 100 sats base rate
    last_updated: Utc::now(),
};

let schema_payment = SchemaPaymentConfig {
    base_multiplier: 1.5,  // Schema costs 1.5x market rate
    min_payment_threshold: 10,
};

let field_payment = FieldPaymentConfig {
    base_multiplier: 2.0,  // Field costs 2x schema rate
    trust_distance_scaling: TrustDistanceScaling::Exponential {
        base: 2.0,
        scale: 0.5,
        min_factor: 1.0,
    },
    min_payment: Some(50),
};

// Calculate payment with safety bounds
let payment = calculate_field_payment(
    &global_config,
    &market_rate,
    &schema_payment,
    &field_payment,
    3.0
);

// Generate hold invoice for complex query
let payment_request = payment_manager
    .generate_invoice(payment, "Complex query".to_string(), true)
    .await?;

// Monitor payment with timeout and retries
let mut retries = 0;
while retries < global_config.max_invoice_retries {
    match payment_manager.verify_payment(&payment_request.invoice).await? {
        PaymentStatus::Settled => {
            // Process query
            break;
        },
        PaymentStatus::Expired => {
            // Generate new invoice
            payment_request = payment_manager
                .generate_invoice(payment, "Retry payment".to_string(), true)
                .await?;
            retries += 1;
        },
        PaymentStatus::PartiallyPaid(amount) => {
            // Handle partial payment
            let remaining = payment - amount;
            // Generate invoice for remaining amount
        },
        _ => {
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }
}
```

## Testing Strategy

1. Unit tests:
   - Payment calculations with bounds checking
   - Trust distance scaling safety
   - Payment state transitions
   - Timeout handling
   - Retry logic

2. Integration tests:
   - Lightning Network payments
   - Hold invoice flows
   - Payment verification atomicity
   - Race condition handling
   - Node failover

3. Security tests:
   - Payment race conditions
   - Concurrent payment verification
   - Invoice expiry enforcement
   - Payment state consistency
   - Node backup/recovery

## Security Considerations

1. Lightning Network:
   - Secure node configuration
   - Private channel management
   - Regular security audits
   - Backup procedures
   - Key management

2. Payment Processing:
   - Atomic payment verification
   - Race condition prevention
   - State consistency checks
   - Timeout enforcement
   - Partial payment handling

3. System Security:
   - Node redundancy
   - Backup procedures
   - Payment state persistence
   - Error recovery
   - Monitoring and alerts

## Future Considerations

1. Advanced Payment Features:
   - Payment channel optimization
   - Multi-path payments
   - Payment batching
   - Dynamic fee adjustment
   - Payment streaming

2. Performance Optimization:
   - Payment caching
   - State management optimization
   - Concurrent payment processing
   - Invoice aggregation
   - Channel rebalancing

3. Reliability:
   - Node redundancy
   - Automatic failover
   - Payment retry strategies
   - State recovery
   - Error handling improvements
