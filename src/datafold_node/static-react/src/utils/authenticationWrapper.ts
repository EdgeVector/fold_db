import { createSignedMessage } from './signing';
import { getAuthContextInstance } from '../auth/useAuth';

/**
 * Validates authentication state and returns the authenticated context.
 *
 * @returns The authenticated context
 * @throws Error if authentication is required but missing
 */
function requireAuth() {
  const authContext = getAuthContextInstance();
  
  if (!authContext?.isAuthenticated || !authContext?.privateKey || !authContext?.publicKeyId) {
    throw new Error('Authentication required: This operation requires valid authentication');
  }
  
  return authContext;
}

/**
 * Authentication wrapper for protected operations.
 * Ensures user is authenticated and signs the request using createSignedMessage().
 *
 * @param requestFunction - The API request function to execute
 * @returns The result of the request function
 * @throws Error if authentication is required but missing
 */
export async function signedRequest<T>(requestFunction: () => Promise<T>): Promise<T> {
  requireAuth();
  
  // Execute the request function with authentication context available
  // The actual signing will be handled by the individual API methods using createSignedMessage()
  return await requestFunction();
}

/**
 * Signs a payload using the current authentication context.
 * This is a helper function for API clients that need to sign their payloads.
 *
 * @param payload - The payload to sign
 * @returns The signed message
 * @throws Error if authentication is required but missing
 */
export async function signPayload(payload: any) {
  const authContext = requireAuth();
  
  return await createSignedMessage(payload, authContext.publicKeyId!, authContext.privateKey!);
}