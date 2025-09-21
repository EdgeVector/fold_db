use crate::log_feature;
use crate::logging::features::LogFeature;
use crate::permissions::types::policy::{PermissionsPolicy, TrustDistance};

/// Manages and enforces access control policies in the database.
///
/// The PermissionManager implements a hybrid access control system that combines:
/// - Trust-based access control using trust distances
/// - Explicit permission grants through public keys
///
/// This dual approach provides flexibility in access control:
/// - Trust distances enable relationship-based access control
/// - Explicit permissions allow fine-grained access management
/// - Both mechanisms can work independently or in combination
#[derive(Default, Clone)]
pub struct PermissionManager {}

impl PermissionManager {
    /// Creates a new PermissionManager instance.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Checks if a public key has read permission based on policy and trust distance.
    ///
    /// Permission is granted if either:
    /// 1. The trust distance is within the policy's required distance
    /// 2. The public key has explicit read permission in the policy
    ///
    /// The check follows this sequence:
    /// 1. First checks trust distance requirements
    /// 2. If trust check fails, falls back to explicit permissions
    /// 3. Access is granted if either check passes
    ///
    /// # Arguments
    ///
    /// * `pub_key` - Public key requesting access
    /// * `permissions_policy` - Policy defining access requirements
    /// * `trust_distance` - Current trust distance from the requesting key
    ///
    /// # Returns
    ///
    /// true if access should be granted, false otherwise
    #[must_use]
    pub fn has_read_permission(
        &self,
        pub_key: &str,
        permissions_policy: &PermissionsPolicy,
        trust_distance: u32,
    ) -> bool {
        // Check trust distance first
        let trust_allowed = match permissions_policy.read_policy {
            TrustDistance::NoRequirement => {
                log_feature!(
                    LogFeature::Permissions,
                    info,
                    "READ PERMISSION: pub_key={} - NoRequirement policy, allowing access",
                    pub_key
                );
                true
            }
            TrustDistance::Distance(required_distance) => {
                let allowed = trust_distance <= required_distance;
                log_feature!(
                    LogFeature::Permissions,
                    info,
                    "READ PERMISSION: pub_key={} - Distance check: {} <= {} = {}",
                    pub_key,
                    trust_distance,
                    required_distance,
                    allowed
                );
                allowed
            }
        };

        // If trust distance check passes, allow access
        if trust_allowed {
            log_feature!(
                LogFeature::Permissions,
                info,
                "READ PERMISSION: pub_key={} - Trust distance check PASSED",
                pub_key
            );
            return true;
        }

        // If trust distance check fails, check explicit permissions
        log_feature!(
            LogFeature::Permissions,
            warn,
            "READ PERMISSION: pub_key={} - Trust distance check FAILED, checking explicit permissions",
            pub_key
        );
        permissions_policy.explicit_read_policy.as_ref().map_or_else(
            || {
                log_feature!(
                    LogFeature::Permissions,
                    warn,
                    "READ PERMISSION: pub_key={} - No explicit permissions configured, ACCESS DENIED",
                    pub_key
                );
                false
            },
            |explicit_policy| {
                let allowed = explicit_policy.counts_by_pub_key.contains_key(pub_key);
                if allowed {
                    log_feature!(
                        LogFeature::Permissions,
                        info,
                        "READ PERMISSION: pub_key={} - Explicit permission found, ACCESS GRANTED",
                        pub_key
                    );
                } else {
                    log_feature!(
                        LogFeature::Permissions,
                        warn,
                        "READ PERMISSION: pub_key={} - No explicit permission found, ACCESS DENIED",
                        pub_key
                    );
                }
                allowed
            }
        )
    }

    /// Checks if a public key has write permission based on policy and trust distance.
    ///
    /// Permission is granted if either:
    /// 1. The trust distance is within the policy's required distance
    /// 2. The public key has explicit write permission in the policy
    ///
    /// The check follows this sequence:
    /// 1. First checks trust distance requirements
    /// 2. If trust check fails, falls back to explicit permissions
    /// 3. Access is granted if either check passes
    ///
    /// Write permissions typically have stricter requirements than read permissions,
    /// reflected in the policy's write_policy settings.
    ///
    /// # Arguments
    ///
    /// * `pub_key` - Public key requesting access
    /// * `permissions_policy` - Policy defining access requirements
    /// * `trust_distance` - Current trust distance from the requesting key
    ///
    /// # Returns
    ///
    /// true if access should be granted, false otherwise
    #[must_use]
    #[allow(clippy::let_and_return)]
    pub fn has_write_permission(
        &self,
        pub_key: &str,
        permissions_policy: &PermissionsPolicy,
        trust_distance: u32,
    ) -> bool {
        // Check trust distance first
        let trust_allowed = match permissions_policy.write_policy {
            TrustDistance::NoRequirement => true,
            TrustDistance::Distance(required_distance) => {
                // Calculate result and print it before returning
                let result = trust_distance <= required_distance;
                log_feature!(
                    LogFeature::Permissions,
                    info,
                    "Trust distance check for {pub_key}: {trust_distance} <= {required_distance} = {result}"
                );
                result
            }
        };

        // If trust distance check passes, allow access
        if trust_allowed {
            return true;
        }

        // If trust distance check fails, check explicit permissions
        permissions_policy
            .explicit_write_policy
            .as_ref()
            .map_or_else(
                || {
                    log_feature!(
                        LogFeature::Permissions,
                        warn,
                        "Trust distance failed and no explicit permissions for {pub_key}"
                    );
                    false
                },
                |explicit_policy| {
                    let allowed = explicit_policy.counts_by_pub_key.contains_key(pub_key);
                    log_feature!(
                    LogFeature::Permissions,
                    warn,
                    "Trust distance failed checking explicit permission for {pub_key}: {allowed}"
                );
                    allowed
                },
            )
    }
}
