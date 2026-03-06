/**
 * Request interceptor that wraps JSON POST/PUT/PATCH bodies in a SignedMessage
 * envelope using the node's Ed25519 private key.
 *
 * When no private key is available, requests pass through unsigned (the server
 * with FOLD_REQUIRE_SIGNATURES=false will accept them).
 */

import { createSignedMessage } from "../../utils/cryptoUtils";
import { store } from "../../store/store";
import type { RequestConfig } from "../core/types";

/** Write methods that should be signed */
const SIGN_METHODS = new Set(["POST", "PUT", "PATCH"]);

/** Paths that are explicitly exempt from signing (read-only or bootstrap) */
const EXEMPT_PATHS = [
  "/api/query",
  "/api/system/auto-identity",
  "/api/system/private-key",
  "/api/system/public-key",
  "/api/system/status",
  "/api/system/database-status",
  "/api/system/list-directory",
  "/api/system/complete-path",
  "/api/security/",
  "/api/openapi.json",
];

function isExemptPath(url: string): boolean {
  for (const prefix of EXEMPT_PATHS) {
    if (url.includes(prefix)) {
      return true;
    }
  }
  return false;
}

/**
 * Creates a request interceptor that signs write requests.
 */
export function createSignatureInterceptor(): (
  config: RequestConfig,
) => Promise<RequestConfig> {
  return async (config: RequestConfig): Promise<RequestConfig> => {
    // Only sign write methods
    if (!SIGN_METHODS.has(config.method)) {
      return config;
    }

    // Skip exempt paths
    if (isExemptPath(config.url)) {
      return config;
    }

    // Skip if no body to sign
    if (!config.body) {
      return config;
    }

    // Skip multipart/form-data (file uploads)
    if (config.body instanceof FormData) {
      return config;
    }

    // Get private key from Redux store
    const state = store.getState();
    const privateKey = state.auth.privateKey;

    // If no private key available, pass through unsigned.
    // The backend will reject protected endpoints that require signatures,
    // but non-protected endpoints will still work.
    if (!privateKey) {
      console.warn(
        `[SignatureInterceptor] No private key available for ${config.method} ${config.url}. Request will be sent unsigned.`,
      );
      return config;
    }

    // The interceptor runs before serializeBody(), so config.body is the raw
    // object passed to post()/put()/patch(). Sign it and replace with the
    // SignedMessage envelope object (serializeBody will JSON.stringify it later).
    const payload = config.body;

    // Create signed message envelope
    const signedMessage = await createSignedMessage(payload, privateKey);

    // Replace body with the signed envelope object
    return {
      ...config,
      body: signedMessage,
    };
  };
}
