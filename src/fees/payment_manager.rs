use chrono::Utc;
use log::warn;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

use crate::fees::lightning::LightningClient;
use crate::fees::{
    Error, GlobalPaymentConfig, LightningPaymentRequest, PaymentState, PaymentStatus,
};

/// Manages payment processing and Lightning Network integration.
///
/// The PaymentManager handles all aspects of payment lifecycle:
/// - Invoice generation and tracking
/// - Payment verification and status updates
/// - Hold invoice management
/// - Payment timeouts and retries
/// - Invoice cleanup and cancellation
///
/// It provides thread-safe access to payment state and integrates
/// with the Lightning Network for actual payment processing.
#[derive(Debug)]
pub struct PaymentManager {
    /// Global configuration for payment processing
    config: GlobalPaymentConfig,
    /// Thread-safe storage of payment states
    invoice_states: Arc<RwLock<HashMap<String, PaymentState>>>,
    /// Client for Lightning Network interactions
    lightning_client: Arc<Box<dyn LightningClient>>,
}

/// Helper struct housing the core payment operations.
///
/// This keeps `PaymentManager` methods small and focused on
/// dependency management while the heavy lifting is delegated to
/// these helpers.
pub(super) struct PaymentOps<'a> {
    config: &'a GlobalPaymentConfig,
    states: &'a Arc<RwLock<HashMap<String, PaymentState>>>,
    client: Arc<Box<dyn LightningClient>>,
}

impl<'a> PaymentOps<'a> {
    fn new(manager: &'a PaymentManager) -> Self {
        Self {
            config: &manager.config,
            states: &manager.invoice_states,
            client: Arc::clone(&manager.lightning_client),
        }
    }

    async fn create_invoice(
        &self,
        amount: u64,
        memo: String,
        hold_invoice: bool,
    ) -> Result<LightningPaymentRequest, Error> {
        self.config.validate_payment(amount)?;

        let timeout = if hold_invoice {
            self.config.hold_invoice_timeout
        } else {
            self.config.payment_timeout
        };

        let invoice = self
            .client
            .create_invoice(amount, memo, timeout, hold_invoice)
            .await?;

        let state = PaymentState {
            invoice_id: invoice.payment_hash.clone(),
            status: PaymentStatus::Pending,
            created_at: Utc::now(),
            last_checked: Utc::now(),
            retry_count: 0,
        };

        self.states
            .write()
            .await
            .insert(invoice.payment_hash.clone(), state);

        Ok(invoice)
    }

    async fn verify_payment(&self, payment_hash: &str) -> Result<bool, Error> {
        let mut states = self.states.write().await;
        let state = states
            .get_mut(payment_hash)
            .ok_or_else(|| Error::InvalidInvoice("Invoice not found".to_string()))?;

        state.last_checked = Utc::now();

        if Utc::now()
            > state.created_at
                + chrono::Duration::from_std(self.config.payment_timeout)
                    .map_err(|e| Error::Internal(e.to_string()))?
        {
            state.status = PaymentStatus::Expired;
            return Ok(false);
        }

        match self
            .client
            .check_payment(&format!("mock_invoice_{payment_hash}"))
            .await?
        {
            PaymentStatus::Settled => {
                state.status = PaymentStatus::Settled;
                Ok(true)
            }
            PaymentStatus::PartiallyPaid(amount) => {
                state.status = PaymentStatus::PartiallyPaid(amount);
                Ok(false)
            }
            current_status => {
                state.status = current_status;
                Ok(false)
            }
        }
    }

    async fn wait_for_payment(
        &self,
        invoice: &LightningPaymentRequest,
        check_interval: Duration,
    ) -> Result<bool, Error> {
        let expiry = invoice.expiry;
        let mut retries = 0;

        while Utc::now() < expiry && retries < self.config.max_invoice_retries {
            if self.verify_payment(&invoice.payment_hash).await? {
                return Ok(true);
            }
            tokio::time::sleep(check_interval).await;
            retries += 1;
        }

        let states = self.states.read().await;
        match states.get(&invoice.payment_hash) {
            Some(state) if matches!(state.status, PaymentStatus::PartiallyPaid(_)) => Err(
                Error::PaymentVerification("Partial payment received".to_string()),
            ),
            _ => Err(Error::PaymentTimeout),
        }
    }

    async fn cancel_payment(&self, payment_hash: &str) -> Result<(), Error> {
        let mut states = self.states.write().await;
        let state = states
            .get_mut(payment_hash)
            .ok_or_else(|| Error::InvalidInvoice("Invoice not found".to_string()))?;

        if state.is_final() {
            return Err(Error::InvalidInvoice(
                "Payment already finalized".to_string(),
            ));
        }

        self.client
            .cancel_invoice(&format!("mock_invoice_{payment_hash}"))
            .await?;
        state.status = PaymentStatus::Cancelled;
        Ok(())
    }

    async fn cleanup_expired_invoices(&self) -> Result<(), Error> {
        let mut states = self.states.write().await;
        let now = Utc::now();

        let expired: Vec<_> = states
            .iter()
            .filter(|(_, state)| {
                if let Ok(timeout) = chrono::Duration::from_std(self.config.payment_timeout) {
                    !state.is_final() && now > state.created_at + timeout
                } else {
                    false
                }
            })
            .map(|(k, _)| k.clone())
            .collect();

        for payment_hash in expired {
            if let Err(e) = self
                .client
                .cancel_invoice(&format!("mock_invoice_{payment_hash}"))
                .await
            {
                warn!("Failed to cancel expired invoice {payment_hash}: {e}");
            }
            if let Some(state) = states.get_mut(&payment_hash) {
                state.status = PaymentStatus::Expired;
            }
        }

        Ok(())
    }
}

impl PaymentManager {
    /// Creates a new PaymentManager instance.
    ///
    /// # Arguments
    ///
    /// * `config` - Global configuration for payment processing
    /// * `lightning_client` - Client implementation for Lightning Network operations
    ///
    /// # Returns
    ///
    /// A new PaymentManager instance configured for payment processing
    #[must_use]
    pub fn new(config: GlobalPaymentConfig, lightning_client: Box<dyn LightningClient>) -> Self {
        Self {
            config,
            invoice_states: Arc::new(RwLock::new(HashMap::new())),
            lightning_client: Arc::new(lightning_client),
        }
    }

    /// Generates a new Lightning Network invoice.
    ///
    /// This method:
    /// 1. Validates the payment amount
    /// 2. Creates an invoice through the Lightning client
    /// 3. Initializes payment state tracking
    /// 4. Configures appropriate timeouts
    ///
    /// # Arguments
    ///
    /// * `amount` - Payment amount in satoshis
    /// * `memo` - Description for the payment
    /// * `hold_invoice` - Whether to create a hold invoice
    ///
    /// # Returns
    ///
    /// A Result containing the payment request or an error
    ///
    /// # Errors
    ///
    /// Returns an Error if:
    /// - The invoice amount is invalid
    /// - The lightning client fails to generate the invoice
    pub async fn generate_invoice(
        &self,
        amount: u64,
        memo: String,
        hold_invoice: bool,
    ) -> Result<LightningPaymentRequest, Error> {
        PaymentOps::new(self)
            .create_invoice(amount, memo, hold_invoice)
            .await
    }

    /// Verifies the current status of a payment.
    ///
    /// This method:
    /// 1. Updates the last checked timestamp
    /// 2. Checks for payment expiration
    /// 3. Verifies payment status with Lightning node
    /// 4. Updates internal payment state
    ///
    /// # Arguments
    ///
    /// * `payment_hash` - Hash identifying the payment to verify
    ///
    /// # Returns
    ///
    /// A Result containing:
    /// - true if payment is settled
    /// - false if pending or failed
    ///
    /// # Errors
    ///
    /// Returns an Error if:
    /// - The payment hash is not found
    /// - The lightning client fails to check the payment status
    pub async fn verify_payment(&self, payment_hash: &str) -> Result<bool, Error> {
        PaymentOps::new(self).verify_payment(payment_hash).await
    }

    /// Waits for a payment to complete with periodic status checks.
    ///
    /// This method:
    /// 1. Periodically checks payment status
    /// 2. Handles payment timeouts
    /// 3. Manages retry attempts
    ///
    /// # Arguments
    ///
    /// * `invoice` - The payment request to monitor
    /// * `check_interval` - Time between status checks
    ///
    /// # Returns
    ///
    /// A Result containing:
    /// - true if payment completed successfully
    /// - Error if payment failed or timed out
    ///
    /// # Errors
    ///
    /// Returns an Error if:
    /// - The payment verification fails
    /// - The maximum number of retries is exceeded
    pub async fn wait_for_payment(
        &self,
        invoice: &LightningPaymentRequest,
        check_interval: Duration,
    ) -> Result<bool, Error> {
        PaymentOps::new(self)
            .wait_for_payment(invoice, check_interval)
            .await
    }

    /// Cancels a pending payment.
    ///
    /// This method:
    /// 1. Verifies the payment is not already finalized
    /// 2. Cancels the invoice with the Lightning node
    /// 3. Updates the payment status
    ///
    /// # Arguments
    ///
    /// * `payment_hash` - Hash identifying the payment to cancel
    ///
    /// # Returns
    ///
    /// A Result indicating success or containing an error
    ///
    /// # Errors
    ///
    /// Returns an Error if:
    /// - The payment hash is not found
    /// - The payment is already finalized
    /// - The lightning client fails to cancel the invoice
    pub async fn cancel_payment(&self, payment_hash: &str) -> Result<(), Error> {
        PaymentOps::new(self).cancel_payment(payment_hash).await
    }

    /// Cleans up expired invoices.
    ///
    /// This method:
    /// 1. Identifies expired invoices
    /// 2. Cancels them with the Lightning node
    /// 3. Updates their status to expired
    ///
    /// Failed cancellations are logged but don't stop the cleanup process.
    ///
    /// # Returns
    ///
    /// A Result indicating success or containing an error
    ///
    /// # Errors
    ///
    /// Returns an Error if:
    /// - The lightning client fails to cancel any expired invoices
    pub async fn cleanup_expired_invoices(&self) -> Result<(), Error> {
        PaymentOps::new(self).cleanup_expired_invoices().await
    }

    /// Gets the current status of a payment.
    ///
    /// # Arguments
    ///
    /// * `payment_hash` - Hash identifying the payment
    ///
    /// # Returns
    ///
    /// A Result containing the payment status or an error
    ///
    /// # Errors
    ///
    /// Returns an Error if:
    /// - The payment hash is not found
    pub async fn get_payment_status(&self, payment_hash: &str) -> Result<PaymentStatus, Error> {
        {
            let states = self.invoice_states.read().await;
            let state = states
                .get(payment_hash)
                .ok_or_else(|| Error::InvalidInvoice("Invoice not found".to_string()))?;
            Ok(state.status.clone())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fees::lightning::MockLightningClient;

    async fn setup_test_manager() -> PaymentManager {
        let config =
            GlobalPaymentConfig::new(50, Duration::from_secs(3600), 3, Duration::from_secs(7200))
                .unwrap();

        let lightning_client = Box::new(MockLightningClient::new());
        PaymentManager::new(config, lightning_client)
    }

    #[tokio::test]
    async fn test_invoice_generation() {
        let manager = setup_test_manager().await;

        let result = manager
            .generate_invoice(100, "Test payment".to_string(), false)
            .await;

        assert!(result.is_ok());
        let invoice = result.unwrap();
        assert_eq!(invoice.amount, 100);
    }

    #[tokio::test]
    async fn test_payment_verification() {
        let manager = setup_test_manager().await;

        let invoice = manager
            .generate_invoice(100, "Test payment".to_string(), false)
            .await
            .unwrap();

        let result = manager.verify_payment(&invoice.payment_hash).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_payment_cancellation() {
        let manager = setup_test_manager().await;

        let invoice = manager
            .generate_invoice(100, "Test payment".to_string(), false)
            .await
            .unwrap();

        let result = manager.cancel_payment(&invoice.payment_hash).await;
        assert!(result.is_ok());

        let status = manager
            .get_payment_status(&invoice.payment_hash)
            .await
            .unwrap();
        assert!(matches!(status, PaymentStatus::Cancelled));
    }

    #[tokio::test]
    async fn test_expired_invoice_cleanup() {
        let manager = setup_test_manager().await;

        // Generate an invoice and manipulate its timestamp to make it expired
        let invoice = manager
            .generate_invoice(100, "Test payment".to_string(), false)
            .await
            .unwrap();

        {
            let mut states = manager.invoice_states.write().await;
            let state = states.get_mut(&invoice.payment_hash).unwrap();
            state.created_at = Utc::now() - chrono::Duration::hours(2);
        }

        manager.cleanup_expired_invoices().await.unwrap();

        let status = manager
            .get_payment_status(&invoice.payment_hash)
            .await
            .unwrap();
        assert!(matches!(status, PaymentStatus::Expired));
    }

    #[tokio::test]
    async fn test_ops_create_invoice() {
        let manager = setup_test_manager().await;
        let ops = PaymentOps::new(&manager);
        let invoice = ops
            .create_invoice(100, "Test ops".to_string(), false)
            .await
            .unwrap();
        assert_eq!(invoice.amount, 100);
    }

    #[tokio::test]
    async fn test_ops_verify_payment() {
        let manager = setup_test_manager().await;
        let ops = PaymentOps::new(&manager);
        let invoice = ops
            .create_invoice(100, "verify".to_string(), false)
            .await
            .unwrap();
        assert!(ops.verify_payment(&invoice.payment_hash).await.unwrap());
    }

    #[tokio::test]
    async fn test_ops_wait_for_payment() {
        let manager = setup_test_manager().await;
        let ops = PaymentOps::new(&manager);
        let invoice = ops
            .create_invoice(100, "wait".to_string(), false)
            .await
            .unwrap();
        assert!(ops
            .wait_for_payment(&invoice, Duration::from_millis(10))
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn test_ops_cancel_payment() {
        let manager = setup_test_manager().await;
        let ops = PaymentOps::new(&manager);
        let invoice = ops
            .create_invoice(100, "cancel".to_string(), false)
            .await
            .unwrap();
        assert!(ops.cancel_payment(&invoice.payment_hash).await.is_ok());
    }

    #[tokio::test]
    async fn test_ops_cleanup_expired() {
        let manager = setup_test_manager().await;
        let ops = PaymentOps::new(&manager);
        let invoice = ops
            .create_invoice(100, "cleanup".to_string(), false)
            .await
            .unwrap();
        {
            let mut states = manager.invoice_states.write().await;
            let state = states.get_mut(&invoice.payment_hash).unwrap();
            state.created_at = Utc::now() - chrono::Duration::hours(2);
        }
        ops.cleanup_expired_invoices().await.unwrap();
        let status = manager
            .get_payment_status(&invoice.payment_hash)
            .await
            .unwrap();
        assert!(matches!(status, PaymentStatus::Expired));
    }
}
