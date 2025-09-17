# PBI-AUTH-001: Authenticated Request Signing and Tab Unlocking

[View in Backlog](../backlog.md#user-content-PBI-AUTH-001)

## ⚠️ CURRENT STATUS: DISABLED FOR DEVELOPMENT

**Note**: This PBI is currently **NOT IMPLEMENTED** in the system. All endpoints operate in development mode with authentication disabled. All requests automatically use "web-ui" identity without requiring signatures or authentication.

## Overview

This PBI implements a comprehensive authenticated request signing system that requires users to provide authentication (passphrase/PIN) to unlock their client-side private key for signing API requests. The system builds upon existing Ed25519 infrastructure to provide secure, user-controlled authentication with automatic tab unlocking capabilities and re-authentication flows.

## Problem Statement

While Datafold has existing Ed25519 cryptographic infrastructure for request signing, there is no user authentication layer to control access to private keys. Current implementation allows unrestricted access to private keys stored in browser memory, creating security risks:

- Private keys are accessible without user authentication
- No mechanism to "lock" tabs and require re-authentication
- Missing user-controlled security layer for sensitive operations
- No automatic session timeout or re-authentication flows

## User Stories

**Primary User Story:**
As a security-conscious user, I want to authenticate with a passphrase/PIN before my private key can be used to sign requests, so that unauthorized access to my browser session cannot perform sensitive operations.

**Detailed User Stories:**
- As a user, I want to enter a passphrase/PIN to unlock my private key for signing operations
- As a user, I want my tab to automatically lock after inactivity and require re-authentication
- As a user, I want to see clear visual feedback when my session is locked vs unlocked
- As a user, I want to be prompted for re-authentication when my session expires
- As a system administrator, I want to configure session timeout policies
- As a developer, I want authenticated request signing to integrate seamlessly with existing API clients

## Technical Approach

### Simplified Integration Strategy
This approach leverages existing robust authentication infrastructure rather than rebuilding from scratch:

- **Global Authentication State**: Lift existing [`useKeyAuthentication`](../../../src/datafold_node/static-react/src/hooks/useKeyAuthentication.ts) to application-wide context
- **Request Signing Integration**: Wire existing [`createSignedMessage()`](../../../src/datafold_node/static-react/src/utils/signing.ts:28) into API clients
- **Session Management**: Add timeout handling and memory-only session state to existing auth flow
- **UI Integration**: Conditional rendering based on authentication state using existing components

### Existing Infrastructure (Leveraged)
- ✅ **Authentication Logic**: [`useKeyAuthentication`](../../../src/datafold_node/static-react/src/hooks/useKeyAuthentication.ts) - Handles private key validation and auth state
- ✅ **Request Signing**: [`createSignedMessage()`](../../../src/datafold_node/static-react/src/utils/signing.ts:28) - Production-ready Ed25519 signing
- ✅ **API Foundation**: [`schemaClient.ts`](../../../src/datafold_node/static-react/src/api/schemaClient.ts) - Ready for signing integration
- ✅ **Key Management UI**: [`KeyManagementTab.jsx`](../../../src/datafold_node/static-react/src/components/tabs/KeyManagementTab.jsx) - Complete key lifecycle UI
- ✅ **Cryptographic Utils**: Full Ed25519 implementation with comprehensive testing
- ✅ **Security Backend**: Verification infrastructure already in place

### Integration Scope (Minimal Changes Required)
- **Context Wrapper**: Wrap existing authentication hook in React context
- **API Client Updates**: Add signing middleware to existing `get()` and `post()` functions
- **Memory-Only Session State**: Add timeout tracking and session management without persistent storage
- **UI Conditional Logic**: Show/hide tabs based on authentication state

## UX/UI Considerations

- **Security-First Design**: Clear authentication prompts with security messaging
- **Session Visibility**: Visual indicators for locked/unlocked state
- **Smooth Re-authentication**: Non-disruptive prompts for session renewal
- **Accessibility**: WCAG 2.1 AA compliance for authentication flows
- **Error Handling**: Clear feedback for authentication failures and session issues
- **Auto-lock Notifications**: Proactive notifications before automatic locking

## Acceptance Criteria

1. **Authentication Layer**: Users must authenticate with passphrase/PIN to unlock private keys
2. **Request Signing Integration**: All API requests automatically signed when authenticated
3. **Tab Locking**: Automatic session locking after configurable inactivity period
4. **Re-authentication Flow**: Smooth re-authentication when sessions expire or are locked
5. **Security Controls**: Private keys remain in memory only, never persisted to storage
6. **Visual Feedback**: Clear UI indicators for authentication state and session status
7. **API Integration**: Seamless integration with existing API clients
8. **Error Recovery**: Graceful handling of authentication failures and network issues
9. **Session Management**: Configurable timeout policies and manual lock/unlock
10. **Testing**: Comprehensive unit, integration, and E2E tests covering all authentication flows

## Dependencies

### External Dependencies
- Existing browser cryptography libraries (`@noble/ed25519`)
- React 18+ with Context API
- TypeScript for type safety

### Internal Dependencies
- Existing Ed25519 signing infrastructure
- Current API client implementations
- React component ecosystem
- Security verification routes

## Open Questions

1. **Passphrase vs PIN**: Should we support both passphrases and numeric PINs for unlocking?
2. **Memory-Only Security**: How to balance security with user experience for session management?
3. **Biometric Integration**: Future support for WebAuthn/biometric authentication?
4. **Multi-tab Coordination**: How should memory-only authentication state coordinate across tabs?
5. **Timeout Configuration**: Should timeout policies be user-configurable or admin-only?

## Related Tasks

See [Tasks for PBI AUTH-001](./tasks.md) for detailed implementation tasks.