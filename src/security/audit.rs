//! Security audit logging and performance monitoring
//!
//! This module provides comprehensive audit logging and performance metrics
//! for all security operations including authentication, encryption, and key management.

use crate::security::{PublicKeyInfo, SignedMessage, VerificationResult};
use crate::logging::features::LogFeature;
use crate::log_feature;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

/// Security audit event types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type")]
pub enum SecurityEvent {
    /// Key registration event
    KeyRegistration {
        key_id: String,
        owner_id: String,
        permissions: Vec<String>,
        success: bool,
        error: Option<String>,
    },
    /// Key removal event
    KeyRemoval {
        key_id: String,
        success: bool,
        error: Option<String>,
    },
    /// Message signature verification event
    SignatureVerification {
        key_id: String,
        owner_id: Option<String>,
        success: bool,
        timestamp_valid: bool,
        permissions_checked: Option<Vec<String>>,
        error: Option<String>,
    },
    /// Encryption operation event
    EncryptionOperation {
        operation: EncryptionOp,
        data_size: usize,
        success: bool,
        error: Option<String>,
    },
    /// Authentication attempt event
    AuthenticationAttempt {
        endpoint: String,
        key_id: Option<String>,
        owner_id: Option<String>,
        success: bool,
        required_permissions: Option<Vec<String>>,
        error: Option<String>,
    },
    /// Security configuration change
    ConfigurationChange {
        setting: String,
        old_value: Option<String>,
        new_value: String,
        admin_id: Option<String>,
    },
}

/// Encryption operation types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EncryptionOp {
    Encrypt,
    Decrypt,
}

/// Performance metrics for security operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityMetrics {
    /// Operation timestamp
    pub timestamp: u64,
    /// Duration of the operation
    pub duration_ms: u64,
    /// Operation type
    pub operation: String,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// Audit log entry combining event and performance data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogEntry {
    /// Unique identifier for this log entry
    pub id: String,
    /// Timestamp when the event occurred
    pub timestamp: u64,
    /// IP address or client identifier
    pub client_id: Option<String>,
    /// Security event details
    pub event: SecurityEvent,
    /// Performance metrics for the operation
    pub metrics: Option<SecurityMetrics>,
    /// Request correlation ID
    pub correlation_id: Option<String>,
}

/// Security audit logger
pub struct SecurityAuditLogger {
    /// In-memory audit log storage
    audit_logs: Arc<RwLock<Vec<AuditLogEntry>>>,
    /// Performance metrics storage
    metrics: Arc<RwLock<Vec<SecurityMetrics>>>,
    /// Maximum number of logs to keep in memory
    max_logs: usize,
}

impl SecurityAuditLogger {
    /// Create a new security audit logger
    pub fn new(max_logs: usize) -> Self {
        Self {
            audit_logs: Arc::new(RwLock::new(Vec::new())),
            metrics: Arc::new(RwLock::new(Vec::new())),
            max_logs,
        }
    }

    /// Log a security event with optional performance timing
    pub async fn log_event(
        &self,
        event: SecurityEvent,
        client_id: Option<String>,
        correlation_id: Option<String>,
        duration: Option<Duration>,
    ) {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let id = format!("{}-{}", timestamp, uuid::Uuid::new_v4());

        // Create performance metrics if duration is provided
        let metrics = duration.map(|dur| SecurityMetrics {
            timestamp,
            duration_ms: dur.as_millis() as u64,
            operation: self.event_operation_name(&event),
            metadata: self.extract_metadata(&event),
        });

        let entry = AuditLogEntry {
            id,
            timestamp,
            client_id,
            event: event.clone(),
            metrics: metrics.clone(),
            correlation_id,
        };

        // Store audit log
        {
            let mut logs = self.audit_logs.write().await;
            logs.push(entry);
            
            // Trim logs if exceeding max size
            if logs.len() > self.max_logs {
                logs.drain(0..logs.len() - self.max_logs);
            }
        }

        // Store metrics separately for performance analysis
        if let Some(metric) = metrics {
            let mut metrics_storage = self.metrics.write().await;
            metrics_storage.push(metric);
            
            // Trim metrics if exceeding max size
            if metrics_storage.len() > self.max_logs {
                metrics_storage.drain(0..metrics_storage.len() - self.max_logs);
            }
        }

        // Log to standard logger as well
        match &event {
            SecurityEvent::KeyRegistration { key_id, owner_id, success, .. } => {
                if *success {
                    log_feature!(
                        LogFeature::Permissions,
                        info,
                        "SECURITY_AUDIT: Key registered - key_id={}, owner_id={}, client={:?}",
                        key_id, owner_id, client_id
                    );
                } else {
                    log_feature!(
                        LogFeature::Permissions,
                        warn,
                        "SECURITY_AUDIT: Key registration failed - key_id={}, owner_id={}, client={:?}",
                        key_id, owner_id, client_id
                    );
                }
            }
            SecurityEvent::SignatureVerification { key_id, success, .. } => {
                if *success {
                    log_feature!(
                        LogFeature::Permissions,
                        info,
                        "SECURITY_AUDIT: Signature verified - key_id={}, client={:?}",
                        key_id, client_id
                    );
                } else {
                    log_feature!(
                        LogFeature::Permissions,
                        warn,
                        "SECURITY_AUDIT: Signature verification failed - key_id={}, client={:?}",
                        key_id, client_id
                    );
                }
            }
            SecurityEvent::AuthenticationAttempt { endpoint, success, .. } => {
                if *success {
                    log_feature!(
                        LogFeature::Permissions,
                        info,
                        "SECURITY_AUDIT: Authentication successful - endpoint={}, client={:?}",
                        endpoint, client_id
                    );
                } else {
                    log_feature!(
                        LogFeature::Permissions,
                        warn,
                        "SECURITY_AUDIT: Authentication failed - endpoint={}, client={:?}",
                        endpoint, client_id
                    );
                }
            }
            _ => {
                log_feature!(
                    LogFeature::Permissions,
                    info,
                    "SECURITY_AUDIT: Event logged - type={}, client={:?}",
                    self.event_operation_name(&event), client_id
                );
            }
        }
    }

    /// Get recent audit logs
    pub async fn get_recent_logs(&self, limit: Option<usize>) -> Vec<AuditLogEntry> {
        let logs = self.audit_logs.read().await;
        let limit = limit.unwrap_or(100).min(logs.len());
        logs.iter().rev().take(limit).cloned().collect()
    }

    /// Get performance metrics for a specific operation type
    pub async fn get_metrics(&self, operation: Option<String>, limit: Option<usize>) -> Vec<SecurityMetrics> {
        let metrics = self.metrics.read().await;
        let filtered: Vec<SecurityMetrics> = if let Some(op) = operation {
            metrics.iter().filter(|m| m.operation == op).cloned().collect()
        } else {
            metrics.iter().cloned().collect()
        };

        let limit = limit.unwrap_or(100).min(filtered.len());
        filtered.iter().rev().take(limit).cloned().collect()
    }

    /// Get performance statistics for an operation
    pub async fn get_performance_stats(&self, operation: String) -> Option<PerformanceStats> {
        let metrics = self.metrics.read().await;
        let operation_metrics: Vec<u64> = metrics
            .iter()
            .filter(|m| m.operation == operation)
            .map(|m| m.duration_ms)
            .collect();

        if operation_metrics.is_empty() {
            return None;
        }

        let mut sorted = operation_metrics.clone();
        sorted.sort_unstable();

        let count = sorted.len();
        let sum: u64 = sorted.iter().sum();
        let avg = sum / count as u64;
        let min = *sorted.first().unwrap();
        let max = *sorted.last().unwrap();
        let p50 = sorted[count / 2];
        let p95 = sorted[(count as f64 * 0.95) as usize];
        let p99 = sorted[(count as f64 * 0.99) as usize];

        Some(PerformanceStats {
            operation,
            count,
            avg_ms: avg,
            min_ms: min,
            max_ms: max,
            p50_ms: p50,
            p95_ms: p95,
            p99_ms: p99,
        })
    }

    /// Extract operation name from security event
    fn event_operation_name(&self, event: &SecurityEvent) -> String {
        match event {
            SecurityEvent::KeyRegistration { .. } => "key_registration".to_string(),
            SecurityEvent::KeyRemoval { .. } => "key_removal".to_string(),
            SecurityEvent::SignatureVerification { .. } => "signature_verification".to_string(),
            SecurityEvent::EncryptionOperation { operation, .. } => {
                format!("encryption_{:?}", operation).to_lowercase()
            }
            SecurityEvent::AuthenticationAttempt { .. } => "authentication_attempt".to_string(),
            SecurityEvent::ConfigurationChange { .. } => "configuration_change".to_string(),
        }
    }

    /// Extract metadata from security event
    fn extract_metadata(&self, event: &SecurityEvent) -> HashMap<String, String> {
        let mut metadata = HashMap::new();

        match event {
            SecurityEvent::KeyRegistration { permissions, .. } => {
                metadata.insert("permissions_count".to_string(), permissions.len().to_string());
            }
            SecurityEvent::EncryptionOperation { data_size, .. } => {
                metadata.insert("data_size_bytes".to_string(), data_size.to_string());
            }
            SecurityEvent::SignatureVerification { permissions_checked, .. } => {
                if let Some(perms) = permissions_checked {
                    metadata.insert("permissions_checked".to_string(), perms.join(","));
                }
            }
            _ => {}
        }

        metadata
    }
}

/// Performance statistics for security operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceStats {
    pub operation: String,
    pub count: usize,
    pub avg_ms: u64,
    pub min_ms: u64,
    pub max_ms: u64,
    pub p50_ms: u64,
    pub p95_ms: u64,
    pub p99_ms: u64,
}

/// Timing helper for measuring operation performance
pub struct SecurityTimer {
    start: Instant,
    operation: String,
}

impl SecurityTimer {
    /// Start timing a security operation
    pub fn start(operation: String) -> Self {
        Self {
            start: Instant::now(),
            operation,
        }
    }

    /// Stop timing and return the duration
    pub fn stop(self) -> (String, Duration) {
        (self.operation, self.start.elapsed())
    }
}

/// Convenience macro for timing security operations
#[macro_export]
macro_rules! time_security_op {
    ($operation:expr, $code:block) => {{
        let timer = SecurityTimer::start($operation.to_string());
        let result = $code;
        let (op, duration) = timer.stop();
        (result, Some(duration))
    }};
}

/// Global security audit logger instance
static SECURITY_AUDIT_LOGGER: once_cell::sync::OnceCell<SecurityAuditLogger> = once_cell::sync::OnceCell::new();

/// Initialize the global security audit logger
pub fn init_security_audit_logger(max_logs: usize) {
    SECURITY_AUDIT_LOGGER.set(SecurityAuditLogger::new(max_logs)).ok();
}

/// Get the global security audit logger
pub fn get_security_audit_logger() -> Option<&'static SecurityAuditLogger> {
    SECURITY_AUDIT_LOGGER.get()
}

/// Convenience function to log a security event
pub async fn log_security_event(
    event: SecurityEvent,
    client_id: Option<String>,
    correlation_id: Option<String>,
    duration: Option<Duration>,
) {
    if let Some(logger) = get_security_audit_logger() {
        logger.log_event(event, client_id, correlation_id, duration).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_security_audit_logger() {
        let logger = SecurityAuditLogger::new(100);

        // Test key registration event
        let event = SecurityEvent::KeyRegistration {
            key_id: "test_key_123".to_string(),
            owner_id: "test_user".to_string(),
            permissions: vec!["read".to_string(), "write".to_string()],
            success: true,
            error: None,
        };

        logger.log_event(event, Some("client_123".to_string()), Some("corr_456".to_string()), Some(Duration::from_millis(25))).await;

        // Test signature verification event
        let event = SecurityEvent::SignatureVerification {
            key_id: "test_key_123".to_string(),
            owner_id: Some("test_user".to_string()),
            success: true,
            timestamp_valid: true,
            permissions_checked: Some(vec!["read".to_string()]),
            error: None,
        };

        logger.log_event(event, Some("client_123".to_string()), None, Some(Duration::from_millis(15))).await;

        // Get recent logs
        let logs = logger.get_recent_logs(Some(10)).await;
        assert_eq!(logs.len(), 2);

        // Get metrics
        let metrics = logger.get_metrics(Some("signature_verification".to_string()), None).await;
        assert_eq!(metrics.len(), 1);
        assert_eq!(metrics[0].duration_ms, 15);

        // Get performance stats
        let stats = logger.get_performance_stats("signature_verification".to_string()).await;
        assert!(stats.is_some());
        let stats = stats.unwrap();
        assert_eq!(stats.count, 1);
        assert_eq!(stats.avg_ms, 15);
    }

    #[tokio::test]
    async fn test_security_timer() {
        let timer = SecurityTimer::start("test_operation".to_string());
        sleep(Duration::from_millis(10)).await;
        let (operation, duration) = timer.stop();

        assert_eq!(operation, "test_operation");
        assert!(duration.as_millis() >= 10);
    }
}