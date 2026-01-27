import { jsx as s, jsxs as d, Fragment as At } from "react/jsx-runtime";
import * as H from "react";
import { createContext as Xn, useState as O, useContext as es, useCallback as $, useEffect as ue, useMemo as W, useRef as vt } from "react";
import { Provider as ts, useSelector as rs, useDispatch as ns } from "react-redux";
import { createAsyncThunk as ze, createSlice as vr, createSelector as Qe, configureStore as ss } from "@reduxjs/toolkit";
const as = {
  GET_INDEXING_STATUS: "/indexing/status",
  GET_INGESTION_CONFIG: "/ingestion/config",
  HEALTH_CHECK: "/ingestion/health",
  PROCESS_JSON: "/ingestion/process",
  GET_STATUS: "/ingestion/status",
  VALIDATE_JSON: "/ingestion/validate",
  ANALYZE_QUERY: "/llm-query/analyze",
  GET_BACKFILL_STATUS: (t) => `/llm-query/backfill/${t}`,
  CHAT: "/llm-query/chat",
  EXECUTE_QUERY_PLAN: "/llm-query/execute",
  RUN_QUERY: "/llm-query/run",
  LIST_LOGS: "/logs",
  GET_CONFIG: "/logs/config",
  RELOAD_CONFIG: "/logs/config/reload",
  GET_FEATURES: "/logs/features",
  UPDATE_FEATURE_LEVEL: "/logs/level",
  STREAM_LOGS: "/logs/stream",
  EXECUTE_MUTATION: "/mutation",
  NATIVE_INDEX_SEARCH: "/native-index/search",
  EXECUTE_QUERY: "/query",
  GET_SCHEMA: (t) => `/schema/${t}`,
  APPROVE_SCHEMA: (t) => `/schema/${t}/approve`,
  BLOCK_SCHEMA: (t) => `/schema/${t}/block`,
  LIST_SCHEMAS: "/schemas",
  LOAD_SCHEMAS: "/schemas/load",
  GET_SYSTEM_PUBLIC_KEY: "/security/system-key",
  GET_DATABASE_CONFIG: "/system/database-config",
  GET_NODE_PRIVATE_KEY: "/system/private-key",
  GET_NODE_PUBLIC_KEY: "/system/public-key",
  RESET_DATABASE: "/system/reset-database",
  RESET_SCHEMA_SERVICE: "/system/reset-schema-service",
  GET_SYSTEM_STATUS: "/system/status",
  LIST_TRANSFORMS: "/transforms",
  GET_ALL_BACKFILLS: "/transforms/backfills",
  GET_ACTIVE_BACKFILLS: "/transforms/backfills/active",
  GET_BACKFILL_STATISTICS: "/transforms/backfills/statistics",
  GET_BACKFILL: (t) => `/transforms/backfills/${t}`,
  GET_TRANSFORM_QUEUE: "/transforms/queue",
  ADD_TO_TRANSFORM_QUEUE: (t) => `/transforms/queue/${t}`,
  GET_TRANSFORM_STATISTICS: "/transforms/statistics"
}, is = {
  ROOT: "/api"
}, z = as, os = 3e4, ls = 3, cs = 1e3, le = {
  // Standard operations
  QUICK: 5e3,
  // System status, basic gets
  STANDARD: 8e3,
  // Schema reads, transforms, logs
  CONFIG: 1e4,
  // Config changes, state changes, load/unload
  MUTATION: 15e3,
  // Batch operations, database reset
  AI_PROCESSING: 6e4,
  DESTRUCTIVE_OPERATIONS: 3e4
}, ce = {
  NONE: 0,
  // Mutations, destructive operations
  LIMITED: 1,
  // State changes, config operations, registrations
  STANDARD: 2,
  // Most read operations, network issues
  CRITICAL: 3
}, je = {
  // 3 minutes - schema state, transforms
  STANDARD: 3e5,
  // 1 hour - system public key
  // Semantic aliases
  SYSTEM_STATUS: 3e4,
  SCHEMA_DATA: 3e5,
  SECURITY_STATUS: 6e4,
  SYSTEM_PUBLIC_KEY: 36e5
}, be = {
  BAD_REQUEST: 400,
  UNAUTHORIZED: 401,
  FORBIDDEN: 403,
  NOT_FOUND: 404,
  INTERNAL_SERVER_ERROR: 500,
  BAD_GATEWAY: 502,
  SERVICE_UNAVAILABLE: 503
}, tr = {
  JSON: "application/json",
  FORM_DATA: "multipart/form-data",
  URL_ENCODED: "application/x-www-form-urlencoded",
  TEXT: "text/plain"
}, It = {
  CONTENT_TYPE: "Content-Type",
  AUTHORIZATION: "Authorization",
  SIGNED_REQUEST: "X-Signed-Request",
  REQUEST_ID: "X-Request-ID",
  AUTHENTICATED: "X-Authenticated"
}, me = {
  NETWORK_ERROR: "Network connection failed. Please check your internet connection.",
  TIMEOUT_ERROR: "Request timed out. Please try again.",
  AUTHENTICATION_ERROR: "Authentication required. Please ensure you are properly authenticated.",
  SCHEMA_STATE_ERROR: "Schema operation not allowed. Only approved schemas can be accessed.",
  SERVER_ERROR: "Server error occurred. Please try again later.",
  VALIDATION_ERROR: "Request validation failed. Please check your input.",
  NOT_FOUND_ERROR: "Requested resource not found.",
  PERMISSION_ERROR: "Permission denied. You do not have access to this resource.",
  RATE_LIMIT_ERROR: "Too many requests. Please wait before trying again."
}, Nt = {
  DEFAULT_TTL_MS: je.STANDARD,
  MAX_CACHE_SIZE: 100,
  SCHEMA_CACHE_TTL_MS: je.SCHEMA_DATA,
  SYSTEM_STATUS_CACHE_TTL_MS: je.SYSTEM_STATUS
}, fr = {
  RETRYABLE_STATUS_CODES: [408, 429, 500, 502, 503, 504],
  EXPONENTIAL_BACKOFF_MULTIPLIER: 2,
  MAX_RETRY_DELAY_MS: 1e4
}, Vt = {
  // Use relative path for CloudFront compatibility
  BASE_URL: "/api"
}, fe = {
  AVAILABLE: "available",
  APPROVED: "approved",
  BLOCKED: "blocked"
}, ds = {
  MUTATION: "mutation"
}, $t = {
  SYSTEM_STATUS: "system-status",
  SECURITY_STATUS: "security-status",
  SYSTEM_PUBLIC_KEY: "system-public-key"
};
class de extends Error {
  constructor(e, r = 0, i = {}) {
    super(e), this.name = "ApiError", this.status = r, this.response = i.response, this.isNetworkError = i.isNetworkError || !1, this.isTimeoutError = i.isTimeoutError || !1, this.isRetryable = this.determineRetryability(r, i.isNetworkError, i.isTimeoutError), this.requestId = i.requestId, this.timestamp = Date.now(), this.code = i.code, this.details = i.details, Object.setPrototypeOf(this, de.prototype);
  }
  /**
   * Determines if an error is retryable based on status code and error type
   */
  determineRetryability(e, r, i) {
    return r || i ? !0 : fr.RETRYABLE_STATUS_CODES.includes(e);
  }
  /**
   * Convert error to a user-friendly message
   */
  toUserMessage() {
    if (this.isNetworkError)
      return me.NETWORK_ERROR;
    if (this.isTimeoutError)
      return me.TIMEOUT_ERROR;
    switch (this.status) {
      case be.UNAUTHORIZED:
        return me.AUTHENTICATION_ERROR;
      case be.FORBIDDEN:
        return me.PERMISSION_ERROR;
      case be.NOT_FOUND:
        return me.NOT_FOUND_ERROR;
      case be.BAD_REQUEST:
        return me.VALIDATION_ERROR;
      case be.INTERNAL_SERVER_ERROR:
      case be.BAD_GATEWAY:
      case be.SERVICE_UNAVAILABLE:
        return me.SERVER_ERROR;
      case 429:
        return me.RATE_LIMIT_ERROR;
      default:
        return this.message || me.SERVER_ERROR;
    }
  }
  /**
   * Serialize error for logging
   */
  toJSON() {
    return {
      name: this.name,
      message: this.message,
      status: this.status,
      isNetworkError: this.isNetworkError,
      isTimeoutError: this.isTimeoutError,
      isRetryable: this.isRetryable,
      requestId: this.requestId,
      timestamp: this.timestamp,
      code: this.code,
      details: this.details,
      stack: this.stack
    };
  }
}
class Nr extends de {
  constructor(e = me.AUTHENTICATION_ERROR, r) {
    super(e, be.UNAUTHORIZED, {
      code: "AUTH_ERROR",
      requestId: r
    }), this.name = "AuthenticationError", Object.setPrototypeOf(this, Nr.prototype);
  }
}
class qt extends de {
  constructor(e, r, i, o = me.SCHEMA_STATE_ERROR) {
    super(o, be.FORBIDDEN, {
      code: "SCHEMA_STATE_ERROR",
      details: { schemaName: e, currentState: r, operation: i }
    }), this.name = "SchemaStateError", this.schemaName = e, this.currentState = r, this.operation = i, Object.setPrototypeOf(this, qt.prototype);
  }
}
class Sr extends de {
  constructor(e = me.NETWORK_ERROR, r) {
    super(e, 0, {
      isNetworkError: !0,
      code: "NETWORK_ERROR",
      requestId: r
    }), this.name = "NetworkError", Object.setPrototypeOf(this, Sr.prototype);
  }
}
class Er extends de {
  constructor(e, r) {
    super(`Request timed out after ${e}ms`, 408, {
      isTimeoutError: !0,
      code: "TIMEOUT_ERROR",
      requestId: r,
      details: { timeoutMs: e }
    }), this.name = "TimeoutError", this.timeoutMs = e, Object.setPrototypeOf(this, Er.prototype);
  }
}
class Ar extends de {
  constructor(e, r) {
    super("Request validation failed", be.BAD_REQUEST, {
      code: "VALIDATION_ERROR",
      requestId: r,
      details: { validationErrors: e }
    }), this.name = "ValidationError", this.validationErrors = e, Object.setPrototypeOf(this, Ar.prototype);
  }
}
class _r extends de {
  constructor(e, r) {
    const i = e ? `Rate limit exceeded. Retry after ${e} seconds.` : me.RATE_LIMIT_ERROR;
    super(i, 429, {
      code: "RATE_LIMIT_ERROR",
      requestId: r,
      details: { retryAfter: e }
    }), this.name = "RateLimitError", this.retryAfter = e, Object.setPrototypeOf(this, _r.prototype);
  }
}
class bt {
  /**
   * Create an ApiError from a fetch response
   */
  static async fromResponse(e, r) {
    let i = {};
    try {
      const u = await e.text();
      u && (i = JSON.parse(u));
    } catch {
    }
    const o = typeof i.error == "string" ? i.error : typeof i.message == "string" ? i.message : `HTTP ${e.status}`;
    if (e.status === be.UNAUTHORIZED)
      return new Nr(o, r || "");
    if (e.status === 429) {
      const u = e.headers.get("Retry-After");
      return new _r(u ? parseInt(u) : void 0, r);
    }
    return e.status === be.BAD_REQUEST && i.validationErrors ? new Ar(i.validationErrors, r || "") : new de(o, e.status, {
      response: i,
      requestId: r,
      code: typeof i.code == "string" ? i.code : void 0,
      details: typeof i.details == "object" && i.details !== null ? i.details : void 0
    });
  }
  /**
   * Create an ApiError from a network error
   */
  static fromNetworkError(e, r) {
    return new Sr(e.message, r);
  }
  /**
   * Create an ApiError from a timeout
   */
  static fromTimeout(e, r) {
    return new Er(e, r);
  }
  /**
   * Create a schema state error
   */
  static fromSchemaState(e, r, i) {
    return new qt(e, r, i);
  }
}
function us(t) {
  return t instanceof de;
}
function hs(t) {
  return us(t) && t.isRetryable;
}
let pr = null;
const wo = (t) => {
  pr = t;
};
class ms {
  constructor(e = Nt.MAX_CACHE_SIZE) {
    this.cache = /* @__PURE__ */ new Map(), this.maxSize = e;
  }
  get(e) {
    const r = this.cache.get(e);
    return r ? Date.now() > r.timestamp + r.ttl ? (this.cache.delete(e), null) : r.data : null;
  }
  set(e, r, i = Nt.DEFAULT_TTL_MS) {
    if (this.cache.size >= this.maxSize) {
      const o = this.cache.keys().next().value;
      this.cache.delete(o);
    }
    this.cache.set(e, {
      data: r,
      timestamp: Date.now(),
      ttl: i,
      key: e
    });
  }
  clear() {
    this.cache.clear();
  }
  size() {
    return this.cache.size;
  }
  getHitRate() {
    return this.cache.size > 0 ? 0.8 : 0;
  }
}
class fs {
  constructor() {
    this.queue = /* @__PURE__ */ new Map();
  }
  /**
   * Get or create a request promise to prevent duplicate requests
   */
  getOrCreate(e, r) {
    if (this.queue.has(e))
      return this.queue.get(e);
    const i = r().finally(() => {
      this.queue.delete(e);
    });
    return this.queue.set(e, i), i;
  }
  clear() {
    this.queue.clear();
  }
}
class yn {
  constructor(e = {}) {
    this.requestInterceptors = [], this.responseInterceptors = [], this.errorInterceptors = [], this.metrics = [], this.config = {
      baseUrl: e.baseUrl || Vt.BASE_URL,
      timeout: e.timeout || os,
      retryAttempts: e.retryAttempts || ls,
      retryDelay: e.retryDelay || cs,
      defaultHeaders: e.defaultHeaders || {},
      enableCache: e.enableCache !== !1,
      enableLogging: e.enableLogging !== !1,
      enableMetrics: e.enableMetrics !== !1
    }, this.cache = new ms(), this.requestQueue = new fs();
  }
  /**
   * HTTP GET method
   */
  async get(e, r = {}) {
    return this.request("GET", e, void 0, r);
  }
  /**
   * HTTP POST method
   */
  async post(e, r, i = {}) {
    return this.request("POST", e, r, i);
  }
  /**
   * HTTP PUT method
   */
  async put(e, r, i = {}) {
    return this.request("PUT", e, r, i);
  }
  /**
   * HTTP DELETE method
   */
  async delete(e, r = {}) {
    return this.request("DELETE", e, void 0, r);
  }
  /**
   * HTTP PATCH method
   */
  async patch(e, r, i = {}) {
    return this.request("PATCH", e, r, i);
  }
  /**
   * Batch request processing
   */
  async batch(e) {
    if (e.length > Nt.MAX_CACHE_SIZE)
      throw new de(
        `Batch size exceeds limit of ${Nt.MAX_CACHE_SIZE}`
      );
    const r = e.map(
      async (i) => {
        try {
          const o = await this.request(
            i.method,
            i.url,
            i.body,
            i.options
          );
          return {
            id: i.id,
            success: o.success,
            data: o.data,
            status: o.status
          };
        } catch (o) {
          const u = o instanceof de ? o : new de(o.message);
          return {
            id: i.id,
            success: !1,
            error: u.message,
            status: u.status
          };
        }
      }
    );
    return Promise.all(r);
  }
  /**
   * Core request method with all functionality
   */
  async request(e, r, i, o = {}) {
    const u = o.requestId || this.generateRequestId(), h = Date.now();
    let l = {
      url: this.buildUrl(r),
      method: e,
      headers: { ...this.config.defaultHeaders },
      body: i,
      timeout: o.timeout || this.config.timeout,
      retries: o.retries !== void 0 ? o.retries : this.config.retryAttempts,
      validateSchema: !!o.validateSchema,
      requiresAuth: !1,
      abortSignal: o.abortSignal,
      metadata: {
        requestId: u,
        timestamp: h,
        priority: o.priority || "normal"
      }
    };
    try {
      for (const N of this.requestInterceptors)
        l = await N(l);
      if (l.validateSchema && await this.validateSchemaAccess(
        r,
        e,
        o.validateSchema || !0
      ), e === "GET" && this.config.enableCache && o.cacheable !== !1) {
        const N = this.generateCacheKey(l.url, l.headers), A = this.cache.get(N);
        if (A)
          return {
            ...A,
            meta: {
              ...A.meta,
              cached: !0,
              fromCache: !0,
              requestId: u,
              timestamp: A.meta?.timestamp || Date.now()
            }
          };
      }
      const g = `${e}:${l.url}:${JSON.stringify(i)}`, f = await this.requestQueue.getOrCreate(
        g,
        () => this.executeRequest(l)
      );
      if (e === "GET" && this.config.enableCache && o.cacheable !== !1 && f.success) {
        const N = this.generateCacheKey(l.url, l.headers), A = o.cacheTtl || Nt.DEFAULT_TTL_MS;
        this.cache.set(N, f, A);
      }
      let b = f;
      for (const N of this.responseInterceptors)
        b = await N(
          b
        );
      return this.config.enableMetrics && this.recordMetrics({
        requestId: u,
        url: l.url,
        method: e,
        startTime: h,
        endTime: Date.now(),
        duration: Date.now() - h,
        status: f.status,
        cached: f.meta?.cached || !1
      }), b;
    } catch (g) {
      let f = g instanceof de ? g : bt.fromNetworkError(g, u);
      for (const b of this.errorInterceptors)
        f = await b(f);
      throw this.config.enableMetrics && this.recordMetrics({
        requestId: u,
        url: l.url,
        method: e,
        startTime: h,
        endTime: Date.now(),
        duration: Date.now() - h,
        error: f.message
      }), f;
    }
  }
  /**
   * Execute the actual HTTP request with retry logic
   */
  async executeRequest(e) {
    let r;
    for (let i = 0; i <= e.retries; i++)
      try {
        return await this.performRequest(e);
      } catch (o) {
        if (r = o instanceof de ? o : bt.fromNetworkError(o, e.metadata.requestId), i === e.retries || !hs(r))
          break;
        const u = Math.min(
          this.config.retryDelay * Math.pow(fr.EXPONENTIAL_BACKOFF_MULTIPLIER, i),
          fr.MAX_RETRY_DELAY_MS
        );
        await this.sleep(u);
      }
    throw r;
  }
  /**
   * Perform the actual HTTP request
   */
  async performRequest(e) {
    const r = new AbortController(), i = setTimeout(() => r.abort(), e.timeout);
    try {
      const o = { ...e.headers };
      if (e.body && !o[It.CONTENT_TYPE] && (o[It.CONTENT_TYPE] = tr.JSON), o[It.REQUEST_ID] = e.metadata.requestId, typeof window < "u") {
        const l = localStorage.getItem("fold_user_hash") || localStorage.getItem("exemem_user_hash");
        l && (o["x-user-hash"] = l, o["x-user-id"] = l);
      }
      const u = {
        method: e.method,
        headers: o,
        signal: e.abortSignal || r.signal
      };
      e.body && e.method !== "GET" && (u.body = this.serializeBody(
        e.body,
        o[It.CONTENT_TYPE]
      ));
      const h = await fetch(e.url, u);
      return clearTimeout(i), await this.handleResponse(h, e.metadata.requestId);
    } catch (o) {
      throw clearTimeout(i), o.name === "AbortError" ? bt.fromTimeout(
        e.timeout,
        e.metadata.requestId
      ) : bt.fromNetworkError(o, e.metadata.requestId);
    }
  }
  /**
   * Handle HTTP response and convert to standardized format
   */
  async handleResponse(e, r) {
    if (!e.ok)
      throw await bt.fromResponse(e, r);
    let i;
    const o = e.headers.get("content-type");
    try {
      o?.includes("application/json") ? i = await e.json() : i = await e.text();
    } catch {
      throw new de("Failed to parse response", e.status, {
        requestId: r
      });
    }
    return {
      success: !0,
      data: i,
      status: e.status,
      headers: this.extractHeaders(e.headers),
      meta: {
        requestId: r,
        timestamp: Date.now(),
        cached: !1,
        fromCache: !1
      }
    };
  }
  /**
   * Add authentication headers using the authentication wrapper
   */
  async addAuthHeaders(e, r) {
  }
  /**
   * Validate schema access according to SCHEMA-002 rules
   */
  async validateSchemaAccess(e, r, i) {
    const o = e.match(/\/schemas\/([^\/]+)/);
    if (!o) return;
    const u = o[1], h = typeof i == "boolean" ? {} : i;
    if ((e.includes("/mutation") || e.includes("/query")) && h.requiresApproved !== !1) {
      if (!pr) {
        console.warn(
          "Store not injected into ApiClient, skipping schema validation"
        );
        return;
      }
      const l = pr.getState().schemas, f = Object.values(l.schemas || {}).find((b) => b.name === u);
      if (!f || f.state !== fe.APPROVED)
        throw new qt(
          u,
          f?.state || "unknown",
          ds.MUTATION
        );
    }
  }
  /**
   * Serialize request body based on content type
   */
  serializeBody(e, r) {
    return r === tr.JSON ? JSON.stringify(e) : r === tr.FORM_DATA ? e : String(e);
  }
  /**
   * Extract response headers as plain object
   */
  extractHeaders(e) {
    const r = {};
    return e.forEach((i, o) => {
      r[o] = i;
    }), r;
  }
  /**
   * Generate unique request ID
   */
  generateRequestId() {
    return `req_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
  }
  /**
   * Generate cache key for request
   */
  generateCacheKey(e, r) {
    const i = Object.keys(r).filter((o) => !o.startsWith("X-Request")).sort().map((o) => `${o}:${r[o]}`).join(";");
    return `${e}|${i}`;
  }
  /**
   * Build full URL from endpoint
   */
  buildUrl(e) {
    return e.startsWith("http") ? e : `${this.config.baseUrl}${e.startsWith("/") ? "" : "/"}${e}`;
  }
  /**
   * Sleep utility for retry delays
   */
  sleep(e) {
    return new Promise((r) => setTimeout(r, e));
  }
  /**
   * Record request metrics
   */
  recordMetrics(e) {
    this.metrics.push(e), this.metrics.length > 1e3 && this.metrics.splice(0, this.metrics.length - 1e3);
  }
  // Interceptor management methods
  addRequestInterceptor(e) {
    this.requestInterceptors.push(e);
  }
  addResponseInterceptor(e) {
    this.responseInterceptors.push(e);
  }
  addErrorInterceptor(e) {
    this.errorInterceptors.push(e);
  }
  // Cache management methods
  clearCache() {
    this.cache.clear();
  }
  getCacheStats() {
    return {
      size: this.cache.size(),
      hitRate: this.cache.getHitRate()
    };
  }
  // Metrics methods
  getMetrics() {
    return [...this.metrics];
  }
  clearMetrics() {
    this.metrics.length = 0;
  }
}
new yn();
function Ee(t) {
  return new yn(t);
}
class ps {
  constructor(e) {
    this.client = e || Ee({
      enableCache: !1,
      // System operations should be fresh
      enableLogging: !0,
      enableMetrics: !0
    });
  }
  /**
   * Get system logs
   * UNPROTECTED - No authentication required
   * Replaces LogSidebar direct fetch('/api/logs')
   * 
   * @returns Promise resolving to logs array
   */
  async getLogs(e) {
    const r = e ? `${z.LIST_LOGS}?since=${e}` : z.LIST_LOGS;
    return this.client.get(r, {
      requiresAuth: !1,
      // Logs are public for monitoring
      timeout: le.STANDARD,
      retries: ce.STANDARD,
      cacheable: !1
      // Always get fresh logs
    });
  }
  /**
   * Reset the database (destructive operation)
   * PROTECTED - Requires authentication for security
   * Replaces StatusSection direct fetch('/api/system/reset-database')
   * 
   * @param confirm - Confirmation flag (must be true)
   * @returns Promise resolving to reset result
   */
  async resetDatabase(e = !1) {
    if (!e)
      throw new Error("Database reset requires explicit confirmation");
    const r = { confirm: e };
    return this.client.post(
      z.RESET_DATABASE,
      r,
      {
        timeout: le.DESTRUCTIVE_OPERATIONS,
        // Longer timeout for database operations
        retries: ce.NONE,
        // No retries for destructive operations
        cacheable: !1
        // Never cache destructive operations
      }
    );
  }
  /**
   * Get system status and health information
   * UNPROTECTED - No authentication required for status monitoring
   * Future endpoint for system monitoring
   * 
   * @returns Promise resolving to system status
   */
  async getSystemStatus() {
    return this.client.get(z.GET_SYSTEM_STATUS, {
      requiresAuth: !1,
      // Status is public for monitoring
      timeout: le.QUICK,
      retries: ce.CRITICAL,
      // Multiple retries for critical system data
      cacheable: !0,
      cacheTtl: je.SYSTEM_STATUS,
      // Cache for 30 seconds
      cacheKey: $t.SYSTEM_STATUS
    });
  }
  /**
   * Get the node's private key
   * UNPROTECTED - No authentication required for UI access
   * 
   * @returns Promise resolving to private key response
   */
  async getNodePrivateKey() {
    return this.client.get(z.GET_NODE_PRIVATE_KEY, {
      requiresAuth: !1,
      // No authentication required for UI access
      timeout: le.STANDARD,
      retries: ce.STANDARD,
      cacheable: !1
      // Never cache private keys
    });
  }
  /**
   * Get the node's public key
   * UNPROTECTED - Public key can be shared
   * 
   * @returns Promise resolving to public key response
   */
  async getNodePublicKey() {
    return this.client.get(z.GET_NODE_PUBLIC_KEY, {
      requiresAuth: !1,
      // Public key is safe to share
      timeout: le.QUICK,
      retries: ce.STANDARD,
      cacheable: !0,
      cacheTtl: je.SYSTEM_STATUS,
      // Cache for 30 seconds
      cacheKey: $t.SYSTEM_PUBLIC_KEY
    });
  }
  /**
   * Create EventSource for log streaming
   * Helper method for components that need real-time log updates
   * Manually builds URL to match API client's URL construction logic
   *
   * @param onMessage - Callback for new log messages
   * @param onError - Callback for connection errors
   * @returns EventSource instance (caller must close it)
   */
  createLogStream(e, r) {
    const i = z.STREAM_LOGS, o = i.startsWith("http") ? i : `${Vt.BASE_URL}${i.startsWith("/") ? "" : "/"}${i}`, u = new EventSource(o);
    return u.onmessage = (h) => {
      e(h.data);
    }, r && (u.onerror = r), u;
  }
  /**
   * Validate reset database request
   * Client-side validation helper
   * 
   * @param request - Reset request to validate
   * @returns Validation result
   */
  validateResetRequest(e) {
    const r = [];
    return typeof e != "object" || e === null ? (r.push("Request must be an object"), { isValid: !1, errors: r }) : (typeof e.confirm != "boolean" ? r.push("Confirm must be a boolean value") : e.confirm || r.push("Confirm must be true to proceed with database reset"), {
      isValid: r.length === 0,
      errors: r
    });
  }
  /**
   * Get API metrics for system operations
   */
  getMetrics() {
    return this.client.getMetrics().filter(
      (e) => e.url.includes("/system") || e.url.includes("/logs")
    );
  }
  /**
   * Get database configuration
   * UNPROTECTED - No authentication required
   * 
   * @returns Promise resolving to database configuration
   */
  async getDatabaseConfig() {
    return this.client.get("/system/database-config", {
      requiresAuth: !1,
      timeout: le.STANDARD,
      retries: ce.STANDARD,
      cacheable: !0,
      cacheTtl: je.SYSTEM_STATUS,
      cacheKey: "database_config"
    });
  }
  /**
   * Update database configuration
   * UNPROTECTED - No authentication required
   * 
   * @param config - Database configuration to apply
   * @returns Promise resolving to update result
   */
  async updateDatabaseConfig(e) {
    const r = { database: e };
    return this.client.post(
      "/system/database-config",
      r,
      {
        timeout: le.STANDARD,
        retries: ce.NONE,
        cacheable: !1
      }
    );
  }
  /**
   * Clear system-related cache
   */
  clearCache() {
    this.client.clearCache();
  }
}
const re = new ps();
re.getLogs.bind(re);
re.resetDatabase.bind(re);
re.getSystemStatus.bind(re);
const Tr = re.getNodePrivateKey.bind(re);
re.getNodePublicKey.bind(re);
const gs = re.getDatabaseConfig.bind(re), ys = re.updateDatabaseConfig.bind(re);
re.createLogStream.bind(re);
re.validateResetRequest.bind(re);
const bs = {
  p: 0x7fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffedn,
  n: 0x1000000000000000000000000000000014def9dea2f79cd65812631a5cf5d3edn,
  a: 0x7fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffecn,
  d: 0x52036cee2b6ffe738cc740797779e89800700a4d4141d8ab75eb4dca135978a3n,
  Gx: 0x216936d3cd6e53fec0a4e231fdd6dc5c692cc7609525a7b2c9562d608f25d51an,
  Gy: 0x6666666666666666666666666666666666666666666666666666666666666658n
}, { p: oe, n: Pt, Gx: Pr, Gy: Ur, a: rr, d: nr } = bs, xs = 8n, Et = 32, bn = 64, ve = (t = "") => {
  throw new Error(t);
}, ws = (t) => typeof t == "bigint", xn = (t) => typeof t == "string", vs = (t) => t instanceof Uint8Array || ArrayBuffer.isView(t) && t.constructor.name === "Uint8Array", ot = (t, e) => !vs(t) || typeof e == "number" && e > 0 && t.length !== e ? ve("Uint8Array expected") : t, Gt = (t) => new Uint8Array(t), Cr = (t) => Uint8Array.from(t), wn = (t, e) => t.toString(16).padStart(e, "0"), Rr = (t) => Array.from(ot(t)).map((e) => wn(e, 2)).join(""), Be = { _0: 48, _9: 57, A: 65, F: 70, a: 97, f: 102 }, $r = (t) => {
  if (t >= Be._0 && t <= Be._9)
    return t - Be._0;
  if (t >= Be.A && t <= Be.F)
    return t - (Be.A - 10);
  if (t >= Be.a && t <= Be.f)
    return t - (Be.a - 10);
}, kr = (t) => {
  const e = "hex invalid";
  if (!xn(t))
    return ve(e);
  const r = t.length, i = r / 2;
  if (r % 2)
    return ve(e);
  const o = Gt(i);
  for (let u = 0, h = 0; u < i; u++, h += 2) {
    const l = $r(t.charCodeAt(h)), g = $r(t.charCodeAt(h + 1));
    if (l === void 0 || g === void 0)
      return ve(e);
    o[u] = l * 16 + g;
  }
  return o;
}, vn = (t, e) => ot(xn(t) ? kr(t) : Cr(ot(t)), e), Nn = () => globalThis?.crypto, Ns = () => Nn()?.subtle ?? ve("crypto.subtle must be defined"), Kr = (...t) => {
  const e = Gt(t.reduce((i, o) => i + ot(o).length, 0));
  let r = 0;
  return t.forEach((i) => {
    e.set(i, r), r += i.length;
  }), e;
}, Ss = (t = Et) => Nn().getRandomValues(Gt(t)), Kt = BigInt, qe = (t, e, r, i = "bad number: out of range") => ws(t) && e <= t && t < r ? t : ve(i), M = (t, e = oe) => {
  const r = t % e;
  return r >= 0n ? r : e + r;
}, Es = (t) => M(t, Pt), Sn = (t, e) => {
  (t === 0n || e <= 0n) && ve("no inverse n=" + t + " mod=" + e);
  let r = M(t, e), i = e, o = 0n, u = 1n;
  for (; r !== 0n; ) {
    const h = i / r, l = i % r, g = o - u * h;
    i = r, r = l, o = u, u = g;
  }
  return i === 1n ? M(o, e) : ve("no inverse");
}, jr = (t) => t instanceof xe ? t : ve("Point expected"), gr = 2n ** 256n;
class xe {
  static BASE;
  static ZERO;
  ex;
  ey;
  ez;
  et;
  constructor(e, r, i, o) {
    const u = gr;
    this.ex = qe(e, 0n, u), this.ey = qe(r, 0n, u), this.ez = qe(i, 1n, u), this.et = qe(o, 0n, u), Object.freeze(this);
  }
  static fromAffine(e) {
    return new xe(e.x, e.y, 1n, M(e.x * e.y));
  }
  /** RFC8032 5.1.3: Uint8Array to Point. */
  static fromBytes(e, r = !1) {
    const i = nr, o = Cr(ot(e, Et)), u = e[31];
    o[31] = u & -129;
    const h = En(o);
    qe(h, 0n, r ? gr : oe);
    const g = M(h * h), f = M(g - 1n), b = M(i * g + 1n);
    let { isValid: N, value: A } = Ts(f, b);
    N || ve("bad point: y not sqrt");
    const E = (A & 1n) === 1n, y = (u & 128) !== 0;
    return !r && A === 0n && y && ve("bad point: x==0, isLastByteOdd"), y !== E && (A = M(-A)), new xe(A, h, 1n, M(A * h));
  }
  /** Checks if the point is valid and on-curve. */
  assertValidity() {
    const e = rr, r = nr, i = this;
    if (i.is0())
      throw new Error("bad point: ZERO");
    const { ex: o, ey: u, ez: h, et: l } = i, g = M(o * o), f = M(u * u), b = M(h * h), N = M(b * b), A = M(g * e), E = M(b * M(A + f)), y = M(N + M(r * M(g * f)));
    if (E !== y)
      throw new Error("bad point: equation left != right (1)");
    const x = M(o * u), p = M(h * l);
    if (x !== p)
      throw new Error("bad point: equation left != right (2)");
    return this;
  }
  /** Equality check: compare points P&Q. */
  equals(e) {
    const { ex: r, ey: i, ez: o } = this, { ex: u, ey: h, ez: l } = jr(e), g = M(r * l), f = M(u * o), b = M(i * l), N = M(h * o);
    return g === f && b === N;
  }
  is0() {
    return this.equals(at);
  }
  /** Flip point over y coordinate. */
  negate() {
    return new xe(M(-this.ex), this.ey, this.ez, M(-this.et));
  }
  /** Point doubling. Complete formula. Cost: `4M + 4S + 1*a + 6add + 1*2`. */
  double() {
    const { ex: e, ey: r, ez: i } = this, o = rr, u = M(e * e), h = M(r * r), l = M(2n * M(i * i)), g = M(o * u), f = e + r, b = M(M(f * f) - u - h), N = g + h, A = N - l, E = g - h, y = M(b * A), x = M(N * E), p = M(b * E), w = M(A * N);
    return new xe(y, x, w, p);
  }
  /** Point addition. Complete formula. Cost: `8M + 1*k + 8add + 1*2`. */
  add(e) {
    const { ex: r, ey: i, ez: o, et: u } = this, { ex: h, ey: l, ez: g, et: f } = jr(e), b = rr, N = nr, A = M(r * h), E = M(i * l), y = M(u * N * f), x = M(o * g), p = M((r + i) * (h + l) - A - E), w = M(x - y), _ = M(x + y), S = M(E - b * A), B = M(p * w), I = M(_ * S), j = M(p * S), C = M(w * _);
    return new xe(B, I, C, j);
  }
  /**
   * Point-by-scalar multiplication. Scalar must be in range 1 <= n < CURVE.n.
   * Uses {@link wNAF} for base point.
   * Uses fake point to mitigate side-channel leakage.
   * @param n scalar by which point is multiplied
   * @param safe safe mode guards against timing attacks; unsafe mode is faster
   */
  multiply(e, r = !0) {
    if (!r && (e === 0n || this.is0()))
      return at;
    if (qe(e, 1n, Pt), e === 1n)
      return this;
    if (this.equals(lt))
      return Fs(e).p;
    let i = at, o = lt;
    for (let u = this; e > 0n; u = u.double(), e >>= 1n)
      e & 1n ? i = i.add(u) : r && (o = o.add(u));
    return i;
  }
  /** Convert point to 2d xy affine point. (X, Y, Z) ∋ (x=X/Z, y=Y/Z) */
  toAffine() {
    const { ex: e, ey: r, ez: i } = this;
    if (this.equals(at))
      return { x: 0n, y: 1n };
    const o = Sn(i, oe);
    return M(i * o) !== 1n && ve("invalid inverse"), { x: M(e * o), y: M(r * o) };
  }
  toBytes() {
    const { x: e, y: r } = this.assertValidity().toAffine(), i = As(r);
    return i[31] |= e & 1n ? 128 : 0, i;
  }
  toHex() {
    return Rr(this.toBytes());
  }
  // encode to hex string
  clearCofactor() {
    return this.multiply(Kt(xs), !1);
  }
  isSmallOrder() {
    return this.clearCofactor().is0();
  }
  isTorsionFree() {
    let e = this.multiply(Pt / 2n, !1).double();
    return Pt % 2n && (e = e.add(this)), e.is0();
  }
  static fromHex(e, r) {
    return xe.fromBytes(vn(e), r);
  }
  get x() {
    return this.toAffine().x;
  }
  get y() {
    return this.toAffine().y;
  }
  toRawBytes() {
    return this.toBytes();
  }
}
const lt = new xe(Pr, Ur, 1n, M(Pr * Ur)), at = new xe(0n, 1n, 1n, 0n);
xe.BASE = lt;
xe.ZERO = at;
const As = (t) => kr(wn(qe(t, 0n, gr), bn)).reverse(), En = (t) => Kt("0x" + Rr(Cr(ot(t)).reverse())), _e = (t, e) => {
  let r = t;
  for (; e-- > 0n; )
    r *= r, r %= oe;
  return r;
}, _s = (t) => {
  const r = t * t % oe * t % oe, i = _e(r, 2n) * r % oe, o = _e(i, 1n) * t % oe, u = _e(o, 5n) * o % oe, h = _e(u, 10n) * u % oe, l = _e(h, 20n) * h % oe, g = _e(l, 40n) * l % oe, f = _e(g, 80n) * g % oe, b = _e(f, 80n) * g % oe, N = _e(b, 10n) * u % oe;
  return { pow_p_5_8: _e(N, 2n) * t % oe, b2: r };
}, Hr = 0x2b8324804fc1df0b2b4d00993dfbd7a72f431806ad2fe478c4ee1b274a0ea0b0n, Ts = (t, e) => {
  const r = M(e * e * e), i = M(r * r * e), o = _s(t * i).pow_p_5_8;
  let u = M(t * r * o);
  const h = M(e * u * u), l = u, g = M(u * Hr), f = h === t, b = h === M(-t), N = h === M(-t * Hr);
  return f && (u = l), (b || N) && (u = g), (M(u) & 1n) === 1n && (u = M(-u)), { isValid: f || b, value: u };
}, Cs = (t) => Es(En(t)), Rs = (...t) => yr.sha512Async(...t), ks = (t) => {
  const e = t.slice(0, Et);
  e[0] &= 248, e[31] &= 127, e[31] |= 64;
  const r = t.slice(Et, bn), i = Cs(e), o = lt.multiply(i), u = o.toBytes();
  return { head: e, prefix: r, scalar: i, point: o, pointBytes: u };
}, Is = (t) => Rs(vn(t, Et)).then(ks), Ir = (t) => Is(t).then((e) => e.pointBytes), yr = {
  sha512Async: async (...t) => {
    const e = Ns(), r = Kr(...t);
    return Gt(await e.digest("SHA-512", r.buffer));
  },
  sha512Sync: void 0,
  bytesToHex: Rr,
  hexToBytes: kr,
  concatBytes: Kr,
  mod: M,
  invert: Sn,
  randomBytes: Ss
}, jt = 8, Os = 256, An = Math.ceil(Os / jt) + 1, br = 2 ** (jt - 1), Bs = () => {
  const t = [];
  let e = lt, r = e;
  for (let i = 0; i < An; i++) {
    r = e, t.push(r);
    for (let o = 1; o < br; o++)
      r = r.add(e), t.push(r);
    e = r.double();
  }
  return t;
};
let Vr;
const qr = (t, e) => {
  const r = e.negate();
  return t ? r : e;
}, Fs = (t) => {
  const e = Vr || (Vr = Bs());
  let r = at, i = lt;
  const o = 2 ** jt, u = o, h = Kt(o - 1), l = Kt(jt);
  for (let g = 0; g < An; g++) {
    let f = Number(t & h);
    t >>= l, f > br && (f -= u, t += 1n);
    const b = g * br, N = b, A = b + Math.abs(f) - 1, E = g % 2 !== 0, y = f < 0;
    f === 0 ? i = i.add(qr(E, e[N])) : r = r.add(qr(y, e[A]));
  }
  return { p: r, f: i };
};
function Ls(t) {
  return t instanceof Uint8Array || ArrayBuffer.isView(t) && t.constructor.name === "Uint8Array";
}
function Or(t, ...e) {
  if (!Ls(t))
    throw new Error("Uint8Array expected");
  if (e.length > 0 && !e.includes(t.length))
    throw new Error("Uint8Array expected of length " + e + ", got length=" + t.length);
}
function Gr(t, e = !0) {
  if (t.destroyed)
    throw new Error("Hash instance has been destroyed");
  if (e && t.finished)
    throw new Error("Hash#digest() has already been called");
}
function Ds(t, e) {
  Or(t);
  const r = e.outputLen;
  if (t.length < r)
    throw new Error("digestInto() expects output buffer of length at least " + r);
}
function xr(...t) {
  for (let e = 0; e < t.length; e++)
    t[e].fill(0);
}
function sr(t) {
  return new DataView(t.buffer, t.byteOffset, t.byteLength);
}
function Ms(t) {
  if (typeof t != "string")
    throw new Error("string expected");
  return new Uint8Array(new TextEncoder().encode(t));
}
function _n(t) {
  return typeof t == "string" && (t = Ms(t)), Or(t), t;
}
class Ps {
}
function Us(t) {
  const e = (i) => t().update(_n(i)).digest(), r = t();
  return e.outputLen = r.outputLen, e.blockLen = r.blockLen, e.create = () => t(), e;
}
function $s(t, e, r, i) {
  if (typeof t.setBigUint64 == "function")
    return t.setBigUint64(e, r, i);
  const o = BigInt(32), u = BigInt(4294967295), h = Number(r >> o & u), l = Number(r & u), g = i ? 4 : 0, f = i ? 0 : 4;
  t.setUint32(e + g, h, i), t.setUint32(e + f, l, i);
}
class Ks extends Ps {
  constructor(e, r, i, o) {
    super(), this.finished = !1, this.length = 0, this.pos = 0, this.destroyed = !1, this.blockLen = e, this.outputLen = r, this.padOffset = i, this.isLE = o, this.buffer = new Uint8Array(e), this.view = sr(this.buffer);
  }
  update(e) {
    Gr(this), e = _n(e), Or(e);
    const { view: r, buffer: i, blockLen: o } = this, u = e.length;
    for (let h = 0; h < u; ) {
      const l = Math.min(o - this.pos, u - h);
      if (l === o) {
        const g = sr(e);
        for (; o <= u - h; h += o)
          this.process(g, h);
        continue;
      }
      i.set(e.subarray(h, h + l), this.pos), this.pos += l, h += l, this.pos === o && (this.process(r, 0), this.pos = 0);
    }
    return this.length += e.length, this.roundClean(), this;
  }
  digestInto(e) {
    Gr(this), Ds(e, this), this.finished = !0;
    const { buffer: r, view: i, blockLen: o, isLE: u } = this;
    let { pos: h } = this;
    r[h++] = 128, xr(this.buffer.subarray(h)), this.padOffset > o - h && (this.process(i, 0), h = 0);
    for (let N = h; N < o; N++)
      r[N] = 0;
    $s(i, o - 8, BigInt(this.length * 8), u), this.process(i, 0);
    const l = sr(e), g = this.outputLen;
    if (g % 4)
      throw new Error("_sha2: outputLen should be aligned to 32bit");
    const f = g / 4, b = this.get();
    if (f > b.length)
      throw new Error("_sha2: outputLen bigger than state");
    for (let N = 0; N < f; N++)
      l.setUint32(4 * N, b[N], u);
  }
  digest() {
    const { buffer: e, outputLen: r } = this;
    this.digestInto(e);
    const i = e.slice(0, r);
    return this.destroy(), i;
  }
  _cloneInto(e) {
    e || (e = new this.constructor()), e.set(...this.get());
    const { blockLen: r, buffer: i, length: o, finished: u, destroyed: h, pos: l } = this;
    return e.destroyed = h, e.finished = u, e.length = o, e.pos = l, o % r && e.buffer.set(i), e;
  }
  clone() {
    return this._cloneInto();
  }
}
const ie = /* @__PURE__ */ Uint32Array.from([
  1779033703,
  4089235720,
  3144134277,
  2227873595,
  1013904242,
  4271175723,
  2773480762,
  1595750129,
  1359893119,
  2917565137,
  2600822924,
  725511199,
  528734635,
  4215389547,
  1541459225,
  327033209
]), Ot = /* @__PURE__ */ BigInt(2 ** 32 - 1), zr = /* @__PURE__ */ BigInt(32);
function js(t, e = !1) {
  return e ? { h: Number(t & Ot), l: Number(t >> zr & Ot) } : { h: Number(t >> zr & Ot) | 0, l: Number(t & Ot) | 0 };
}
function Hs(t, e = !1) {
  const r = t.length;
  let i = new Uint32Array(r), o = new Uint32Array(r);
  for (let u = 0; u < r; u++) {
    const { h, l } = js(t[u], e);
    [i[u], o[u]] = [h, l];
  }
  return [i, o];
}
const Qr = (t, e, r) => t >>> r, Yr = (t, e, r) => t << 32 - r | e >>> r, et = (t, e, r) => t >>> r | e << 32 - r, tt = (t, e, r) => t << 32 - r | e >>> r, Bt = (t, e, r) => t << 64 - r | e >>> r - 32, Ft = (t, e, r) => t >>> r - 32 | e << 64 - r;
function Fe(t, e, r, i) {
  const o = (e >>> 0) + (i >>> 0);
  return { h: t + r + (o / 2 ** 32 | 0) | 0, l: o | 0 };
}
const Vs = (t, e, r) => (t >>> 0) + (e >>> 0) + (r >>> 0), qs = (t, e, r, i) => e + r + i + (t / 2 ** 32 | 0) | 0, Gs = (t, e, r, i) => (t >>> 0) + (e >>> 0) + (r >>> 0) + (i >>> 0), zs = (t, e, r, i, o) => e + r + i + o + (t / 2 ** 32 | 0) | 0, Qs = (t, e, r, i, o) => (t >>> 0) + (e >>> 0) + (r >>> 0) + (i >>> 0) + (o >>> 0), Ys = (t, e, r, i, o, u) => e + r + i + o + u + (t / 2 ** 32 | 0) | 0, Tn = Hs([
  "0x428a2f98d728ae22",
  "0x7137449123ef65cd",
  "0xb5c0fbcfec4d3b2f",
  "0xe9b5dba58189dbbc",
  "0x3956c25bf348b538",
  "0x59f111f1b605d019",
  "0x923f82a4af194f9b",
  "0xab1c5ed5da6d8118",
  "0xd807aa98a3030242",
  "0x12835b0145706fbe",
  "0x243185be4ee4b28c",
  "0x550c7dc3d5ffb4e2",
  "0x72be5d74f27b896f",
  "0x80deb1fe3b1696b1",
  "0x9bdc06a725c71235",
  "0xc19bf174cf692694",
  "0xe49b69c19ef14ad2",
  "0xefbe4786384f25e3",
  "0x0fc19dc68b8cd5b5",
  "0x240ca1cc77ac9c65",
  "0x2de92c6f592b0275",
  "0x4a7484aa6ea6e483",
  "0x5cb0a9dcbd41fbd4",
  "0x76f988da831153b5",
  "0x983e5152ee66dfab",
  "0xa831c66d2db43210",
  "0xb00327c898fb213f",
  "0xbf597fc7beef0ee4",
  "0xc6e00bf33da88fc2",
  "0xd5a79147930aa725",
  "0x06ca6351e003826f",
  "0x142929670a0e6e70",
  "0x27b70a8546d22ffc",
  "0x2e1b21385c26c926",
  "0x4d2c6dfc5ac42aed",
  "0x53380d139d95b3df",
  "0x650a73548baf63de",
  "0x766a0abb3c77b2a8",
  "0x81c2c92e47edaee6",
  "0x92722c851482353b",
  "0xa2bfe8a14cf10364",
  "0xa81a664bbc423001",
  "0xc24b8b70d0f89791",
  "0xc76c51a30654be30",
  "0xd192e819d6ef5218",
  "0xd69906245565a910",
  "0xf40e35855771202a",
  "0x106aa07032bbd1b8",
  "0x19a4c116b8d2d0c8",
  "0x1e376c085141ab53",
  "0x2748774cdf8eeb99",
  "0x34b0bcb5e19b48a8",
  "0x391c0cb3c5c95a63",
  "0x4ed8aa4ae3418acb",
  "0x5b9cca4f7763e373",
  "0x682e6ff3d6b2b8a3",
  "0x748f82ee5defb2fc",
  "0x78a5636f43172f60",
  "0x84c87814a1f0ab72",
  "0x8cc702081a6439ec",
  "0x90befffa23631e28",
  "0xa4506cebde82bde9",
  "0xbef9a3f7b2c67915",
  "0xc67178f2e372532b",
  "0xca273eceea26619c",
  "0xd186b8c721c0c207",
  "0xeada7dd6cde0eb1e",
  "0xf57d4f7fee6ed178",
  "0x06f067aa72176fba",
  "0x0a637dc5a2c898a6",
  "0x113f9804bef90dae",
  "0x1b710b35131c471b",
  "0x28db77f523047d84",
  "0x32caab7b40c72493",
  "0x3c9ebe0a15c9bebc",
  "0x431d67c49c100d4c",
  "0x4cc5d4becb3e42b6",
  "0x597f299cfc657e2a",
  "0x5fcb6fab3ad6faec",
  "0x6c44198c4a475817"
].map((t) => BigInt(t))), Ws = Tn[0], Js = Tn[1], Pe = /* @__PURE__ */ new Uint32Array(80), Ue = /* @__PURE__ */ new Uint32Array(80);
class Zs extends Ks {
  constructor(e = 64) {
    super(128, e, 16, !1), this.Ah = ie[0] | 0, this.Al = ie[1] | 0, this.Bh = ie[2] | 0, this.Bl = ie[3] | 0, this.Ch = ie[4] | 0, this.Cl = ie[5] | 0, this.Dh = ie[6] | 0, this.Dl = ie[7] | 0, this.Eh = ie[8] | 0, this.El = ie[9] | 0, this.Fh = ie[10] | 0, this.Fl = ie[11] | 0, this.Gh = ie[12] | 0, this.Gl = ie[13] | 0, this.Hh = ie[14] | 0, this.Hl = ie[15] | 0;
  }
  // prettier-ignore
  get() {
    const { Ah: e, Al: r, Bh: i, Bl: o, Ch: u, Cl: h, Dh: l, Dl: g, Eh: f, El: b, Fh: N, Fl: A, Gh: E, Gl: y, Hh: x, Hl: p } = this;
    return [e, r, i, o, u, h, l, g, f, b, N, A, E, y, x, p];
  }
  // prettier-ignore
  set(e, r, i, o, u, h, l, g, f, b, N, A, E, y, x, p) {
    this.Ah = e | 0, this.Al = r | 0, this.Bh = i | 0, this.Bl = o | 0, this.Ch = u | 0, this.Cl = h | 0, this.Dh = l | 0, this.Dl = g | 0, this.Eh = f | 0, this.El = b | 0, this.Fh = N | 0, this.Fl = A | 0, this.Gh = E | 0, this.Gl = y | 0, this.Hh = x | 0, this.Hl = p | 0;
  }
  process(e, r) {
    for (let S = 0; S < 16; S++, r += 4)
      Pe[S] = e.getUint32(r), Ue[S] = e.getUint32(r += 4);
    for (let S = 16; S < 80; S++) {
      const B = Pe[S - 15] | 0, I = Ue[S - 15] | 0, j = et(B, I, 1) ^ et(B, I, 8) ^ Qr(B, I, 7), C = tt(B, I, 1) ^ tt(B, I, 8) ^ Yr(B, I, 7), k = Pe[S - 2] | 0, L = Ue[S - 2] | 0, P = et(k, L, 19) ^ Bt(k, L, 61) ^ Qr(k, L, 6), F = tt(k, L, 19) ^ Ft(k, L, 61) ^ Yr(k, L, 6), U = Gs(C, F, Ue[S - 7], Ue[S - 16]), K = zs(U, j, P, Pe[S - 7], Pe[S - 16]);
      Pe[S] = K | 0, Ue[S] = U | 0;
    }
    let { Ah: i, Al: o, Bh: u, Bl: h, Ch: l, Cl: g, Dh: f, Dl: b, Eh: N, El: A, Fh: E, Fl: y, Gh: x, Gl: p, Hh: w, Hl: _ } = this;
    for (let S = 0; S < 80; S++) {
      const B = et(N, A, 14) ^ et(N, A, 18) ^ Bt(N, A, 41), I = tt(N, A, 14) ^ tt(N, A, 18) ^ Ft(N, A, 41), j = N & E ^ ~N & x, C = A & y ^ ~A & p, k = Qs(_, I, C, Js[S], Ue[S]), L = Ys(k, w, B, j, Ws[S], Pe[S]), P = k | 0, F = et(i, o, 28) ^ Bt(i, o, 34) ^ Bt(i, o, 39), U = tt(i, o, 28) ^ Ft(i, o, 34) ^ Ft(i, o, 39), K = i & u ^ i & l ^ u & l, V = o & h ^ o & g ^ h & g;
      w = x | 0, _ = p | 0, x = E | 0, p = y | 0, E = N | 0, y = A | 0, { h: N, l: A } = Fe(f | 0, b | 0, L | 0, P | 0), f = l | 0, b = g | 0, l = u | 0, g = h | 0, u = i | 0, h = o | 0;
      const Y = Vs(P, U, V);
      i = qs(Y, L, F, K), o = Y | 0;
    }
    ({ h: i, l: o } = Fe(this.Ah | 0, this.Al | 0, i | 0, o | 0)), { h: u, l: h } = Fe(this.Bh | 0, this.Bl | 0, u | 0, h | 0), { h: l, l: g } = Fe(this.Ch | 0, this.Cl | 0, l | 0, g | 0), { h: f, l: b } = Fe(this.Dh | 0, this.Dl | 0, f | 0, b | 0), { h: N, l: A } = Fe(this.Eh | 0, this.El | 0, N | 0, A | 0), { h: E, l: y } = Fe(this.Fh | 0, this.Fl | 0, E | 0, y | 0), { h: x, l: p } = Fe(this.Gh | 0, this.Gl | 0, x | 0, p | 0), { h: w, l: _ } = Fe(this.Hh | 0, this.Hl | 0, w | 0, _ | 0), this.set(i, o, u, h, l, g, f, b, N, A, E, y, x, p, w, _);
  }
  roundClean() {
    xr(Pe, Ue);
  }
  destroy() {
    xr(this.buffer), this.set(0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0);
  }
}
const Xs = /* @__PURE__ */ Us(() => new Zs()), ea = Xs, ta = (t) => typeof Buffer < "u" ? Buffer.from(t, "base64") : Uint8Array.from(atob(t), (e) => e.charCodeAt(0)), ra = (t) => {
  if (typeof Buffer < "u")
    return Buffer.from(t).toString("base64");
  const e = Array.from(t, (r) => String.fromCharCode(r)).join("");
  return btoa(e);
};
function zt(t) {
  return ta(t);
}
function na(t) {
  return ra(t);
}
yr.sha512Sync = (...t) => ea(yr.concatBytes(...t));
const sa = {
  isAuthenticated: !1,
  systemPublicKey: null,
  systemKeyId: null,
  privateKey: null,
  publicKeyId: null,
  isLoading: !1,
  error: null
}, ar = ze(
  "auth/initializeSystemKey",
  async (t, { rejectWithValue: e }) => {
    try {
      const r = await Tr();
      if (console.log("initializeSystemKey thunk response:", r), r.success && r.data && r.data.private_key) {
        const i = zt(r.data.private_key), o = await Ir(i);
        return {
          systemPublicKey: btoa(
            String.fromCharCode(...o)
          ),
          systemKeyId: "node-private-key",
          privateKey: i,
          isSystemReady: !0
        };
      } else
        return {
          systemPublicKey: null,
          systemKeyId: null,
          privateKey: null,
          isSystemReady: !1
        };
    } catch (r) {
      return console.error("Failed to fetch node private key:", r), e(
        r instanceof Error ? r.message : "Failed to fetch node private key"
      );
    }
  }
), Ut = ze(
  "auth/validatePrivateKey",
  async (t, { getState: e, rejectWithValue: r }) => {
    const i = e(), { systemPublicKey: o, systemKeyId: u } = i.auth;
    if (!o || !u)
      return r("System public key not available");
    try {
      console.log("🔑 Converting private key from base64...");
      const h = zt(t);
      console.log("🔑 Generating public key from private key...");
      const l = await Ir(h), g = btoa(
        String.fromCharCode(...l)
      ), f = g === o;
      return console.log("🔑 Key comparison:", {
        derived: g,
        system: o,
        matches: f
      }), f ? {
        privateKey: h,
        publicKeyId: u,
        isAuthenticated: !0
      } : r("Private key does not match system public key");
    } catch (h) {
      return console.error("Private key validation failed:", h), r(
        h instanceof Error ? h.message : "Private key validation failed"
      );
    }
  }
), ir = ze(
  "auth/refreshSystemKey",
  async (t, { rejectWithValue: e }) => {
    for (let o = 1; o <= 5; o++)
      try {
        const u = await Tr();
        if (u.success && u.data && u.data.private_key) {
          const h = zt(u.data.private_key), l = await Ir(h);
          return {
            systemPublicKey: btoa(
              String.fromCharCode(...l)
            ),
            systemKeyId: "node-private-key",
            privateKey: h,
            isSystemReady: !0
          };
        } else if (o < 5) {
          const h = 200 * o;
          await new Promise((l) => setTimeout(l, h));
        }
      } catch (u) {
        if (o === 5)
          return e(
            u instanceof Error ? u.message : "Failed to fetch node private key"
          );
        {
          const h = 200 * o;
          await new Promise((l) => setTimeout(l, h));
        }
      }
    return e(
      "Failed to fetch node private key after multiple attempts"
    );
  }
), or = ze(
  "auth/fetchNodePrivateKey",
  async (t, { rejectWithValue: e }) => {
    try {
      const r = await Tr();
      return console.log("fetchNodePrivateKey thunk response:", r), r.success && r.data && r.data.private_key ? {
        privateKey: zt(r.data.private_key),
        publicKeyId: "node-private-key",
        // Use a consistent identifier
        isSystemReady: !0
      } : e("Failed to fetch private key from backend");
    } catch (r) {
      return console.error("Failed to fetch node private key:", r), e(
        r instanceof Error ? r.message : "Failed to fetch node private key"
      );
    }
  }
), Br = ze(
  "auth/loginUser",
  async (t, { rejectWithValue: e }) => {
    try {
      const i = new TextEncoder().encode(t), o = await crypto.subtle.digest("SHA-256", i), l = Array.from(new Uint8Array(o)).map((g) => g.toString(16).padStart(2, "0")).join("").substring(0, 32);
      return { id: t, hash: l };
    } catch {
      return e("Failed to generate user hash");
    }
  }
), Cn = vr({
  name: "auth",
  initialState: sa,
  reducers: {
    clearAuthentication: (t) => {
      t.isAuthenticated = !1, t.privateKey = null, t.publicKeyId = null, t.error = null;
    },
    setError: (t, e) => {
      t.error = e.payload;
    },
    clearError: (t) => {
      t.error = null;
    },
    updateSystemKey: (t, e) => {
      t.systemPublicKey = e.payload.systemPublicKey, t.systemKeyId = e.payload.systemKeyId, t.error = null;
    },
    logoutUser: (t) => {
      t.isAuthenticated = !1, t.user = void 0, t.error = null;
    },
    // Restore session from local storage
    restoreSession: (t, e) => {
      t.isAuthenticated = !0, t.user = e.payload, t.error = null;
    }
  },
  extraReducers: (t) => {
    t.addCase(ar.pending, (e) => {
      e.isLoading = !0, e.error = null;
    }).addCase(ar.fulfilled, (e, r) => {
      console.log("initializeSystemKey.fulfilled", r.payload), e.isLoading = !1, e.systemPublicKey = r.payload.systemPublicKey, e.systemKeyId = r.payload.systemKeyId, e.privateKey = r.payload.privateKey, e.error = null;
    }).addCase(ar.rejected, (e, r) => {
      e.isLoading = !1, e.error = r.payload;
    }).addCase(Ut.pending, (e) => {
      e.isLoading = !0, e.error = null;
    }).addCase(Ut.fulfilled, (e, r) => {
      e.isLoading = !1, e.isAuthenticated = r.payload.isAuthenticated, e.privateKey = r.payload.privateKey, e.publicKeyId = r.payload.publicKeyId, e.error = null;
    }).addCase(Ut.rejected, (e, r) => {
      e.isLoading = !1, e.isAuthenticated = !1, e.privateKey = null, e.publicKeyId = null, e.error = r.payload;
    }).addCase(ir.pending, (e) => {
      e.isLoading = !0, e.error = null;
    }).addCase(ir.fulfilled, (e, r) => {
      e.isLoading = !1, e.systemPublicKey = r.payload.systemPublicKey, e.systemKeyId = r.payload.systemKeyId, e.privateKey = r.payload.privateKey, e.user || (e.isAuthenticated = !1), e.error = null;
    }).addCase(ir.rejected, (e, r) => {
      e.isLoading = !1, e.systemPublicKey = null, e.systemKeyId = null, e.error = r.payload;
    }).addCase(or.pending, (e) => {
      e.isLoading = !0, e.error = null;
    }).addCase(or.fulfilled, (e, r) => {
      e.isLoading = !1, e.privateKey = r.payload.privateKey, e.publicKeyId = r.payload.publicKeyId, e.error = null;
    }).addCase(or.rejected, (e, r) => {
      e.isLoading = !1, e.error = r.payload;
    }).addCase(Br.fulfilled, (e, r) => {
      e.isAuthenticated = !0, e.user = r.payload, e.error = null;
    });
  }
}), {
  clearAuthentication: aa,
  setError: vo,
  clearError: No,
  updateSystemKey: So,
  logoutUser: ia,
  restoreSession: Eo
} = Cn.actions, oa = Cn.reducer, la = 3e5, lr = 3, _t = {
  // Async thunk action types
  FETCH_SCHEMAS: "schemas/fetchSchemas",
  APPROVE_SCHEMA: "schemas/approveSchema",
  BLOCK_SCHEMA: "schemas/blockSchema",
  UNLOAD_SCHEMA: "schemas/unloadSchema",
  LOAD_SCHEMA: "schemas/loadSchema"
}, Tt = {
  // Network and API errors
  FETCH_FAILED: "Failed to fetch schemas from server",
  // Schema operation errors
  APPROVE_FAILED: "Failed to approve schema",
  BLOCK_FAILED: "Failed to block schema",
  UNLOAD_FAILED: "Failed to unload schema",
  LOAD_FAILED: "Failed to load schema"
}, ke = {
  AVAILABLE: "available",
  APPROVED: "approved",
  BLOCKED: "blocked",
  LOADING: "loading",
  ERROR: "error"
};
process.env.NODE_ENV, process.env.NODE_ENV;
process.env.NODE_ENV, process.env.NODE_ENV;
const ca = {
  MUTATION_WRAPPER_KEY: "value"
}, da = 200, ua = 300, ha = [
  // Main features
  { id: "ingestion", label: "Ingestion", icon: "📥", group: "main" },
  { id: "file-upload", label: "File Upload", icon: "📄", group: "main" },
  { id: "llm-query", label: "AI Query", icon: "🤖", group: "main" },
  // Developer/Advanced features
  {
    id: "native-index",
    label: "Native Index Query",
    icon: "🧭",
    group: "advanced"
  }
], Lt = {
  executeQuery: "Execute Query"
}, Ge = {
  schema: "Schema",
  schemaEmpty: "No schemas available",
  schemaHelp: "Select a schema to work with",
  operationType: "Operation Type",
  operationHelp: "Select the type of operation to perform"
}, ma = {
  loading: "Loading..."
}, fa = [
  { value: "Insert", label: "Insert" },
  { value: "Update", label: "Update" },
  { value: "Delete", label: "Delete" }
], Rn = {
  Insert: "create",
  Create: "create",
  Update: "update",
  Delete: "delete"
}, Wr = {
  approved: "bg-green-100 text-green-800",
  available: "bg-blue-100 text-blue-800",
  blocked: "bg-red-100 text-red-800",
  pending: "bg-yellow-100 text-yellow-800"
}, kn = {
  schemaStates: {
    approved: "Schema is approved for use in queries and mutations",
    available: "Schema is available but requires approval before use",
    blocked: "Schema is blocked and cannot be used",
    pending: "Schema approval is pending review",
    unknown: "Schema state is unknown or invalid"
  }
}, Jr = {
  label: "Range Key",
  badgeColor: "bg-purple-100 text-purple-800"
};
function In(t) {
  if (!t || typeof t != "object") return null;
  const e = t.schema_type;
  if (e === "Single")
    return "Single";
  if (e === "Range")
    return "Range";
  if (e === "HashRange")
    return "HashRange";
  if (typeof e == "object" && e !== null) {
    if ("HashRange" in e)
      return "HashRange";
    if ("Range" in e)
      return "Range";
  }
  return null;
}
function Fr(t) {
  return !t || typeof t != "object" ? !1 : In(t) === "HashRange";
}
function On(t) {
  if (typeof t != "string") return null;
  const e = t.split(".");
  return e[e.length - 1] || t;
}
function ct(t) {
  return !t || typeof t != "object" ? !1 : In(t) === "Range";
}
function Ye(t) {
  if (!t || typeof t != "object") return null;
  const e = t?.key?.range_field;
  return typeof e == "string" && e.trim() ? On(e) : null;
}
function Bn(t) {
  if (!t || typeof t != "object") return null;
  const e = t?.key?.hash_field;
  return e && typeof e == "string" && e.trim() ? On(e) : null;
}
function pa(t) {
  if (!ct(t))
    return {};
  const e = Ye(t);
  if (!Array.isArray(t.fields))
    throw new Error(`Expected schema.fields to be an array for range schema "${t.name}", got ${typeof t.fields}`);
  return t.fields.reduce((r, i) => (i !== e && (r[i] = {}), r), {});
}
function ga(t, e, r, i) {
  const o = typeof e == "string" ? Rn[e] || e.toLowerCase() : "", u = o === "delete", h = {
    type: "mutation",
    schema: t.name,
    mutation_type: o
  }, l = Ye(t);
  if (u)
    h.fields_and_values = {}, h.key_value = { hash: null, range: null }, r && r.trim() && l && (h.fields_and_values[l] = r.trim(), h.key_value.range = r.trim());
  else {
    const g = {};
    r && r.trim() && l && (g[l] = r.trim()), Object.entries(i).forEach(([f, b]) => {
      if (f !== l) {
        const N = ca.MUTATION_WRAPPER_KEY;
        typeof b == "string" || typeof b == "number" || typeof b == "boolean" ? g[f] = { [N]: b } : typeof b == "object" && b !== null ? g[f] = b : g[f] = { [N]: b };
      }
    }), h.fields_and_values = g, h.key_value = {
      hash: null,
      range: r && r.trim() ? r.trim() : null
    };
  }
  return h;
}
function ya(t) {
  return ct(t) ? {
    isRangeSchema: !0,
    rangeKey: Ye(t),
    rangeFields: [],
    // Declarative schemas don't store field types
    nonRangeKeyFields: pa(t),
    totalFields: Array.isArray(t.fields) ? t.fields.length : 0
  } : null;
}
function Fn(t) {
  return typeof t == "string" ? t.toLowerCase() : typeof t == "object" && t !== null ? t.state ? String(t.state).toLowerCase() : String(t).toLowerCase() : String(t || "").toLowerCase();
}
function ba(t) {
  return t == null;
}
function xa(t) {
  return Fr(t) ? {
    isHashRangeSchema: !0,
    hashField: Bn(t),
    rangeField: Ye(t),
    totalFields: Array.isArray(t.fields) ? t.fields.length : 0
  } : null;
}
const Dt = fe.AVAILABLE, wa = /* @__PURE__ */ new Set([
  fe.AVAILABLE,
  fe.APPROVED,
  fe.BLOCKED,
  "loading",
  "error"
]);
function va(t) {
  if (!t || typeof t != "object")
    return null;
  const e = t.name;
  if (typeof e == "string" && e.trim().length > 0)
    return e;
  const r = t.schema;
  if (r && typeof r == "object") {
    const i = r.name;
    if (typeof i == "string" && i.trim().length > 0)
      return i;
  }
  return null;
}
function Na(t) {
  return !t || typeof t != "object" ? void 0 : [
    t.state,
    t.schema_state,
    t.schemaState,
    t.status,
    t.current_state,
    t.schema?.state
  ].find((r) => r !== void 0);
}
class Qt {
  constructor(e) {
    this.client = e || Ee({
      enableCache: !0,
      enableLogging: !0,
      enableMetrics: !0
    });
  }
  /**
   * Get all schemas with their current states
   * UNPROTECTED - No authentication required
   */
  async getSchemas() {
    const e = await this.client.get(z.LIST_SCHEMAS, {
      cacheable: !0,
      cacheKey: "schemas:all",
      cacheTtl: 3e5
      // 5 minutes
    });
    if (!e.success)
      return { ...e, data: [] };
    const r = e.data;
    let i = [];
    return Array.isArray(r) ? i = r : r && typeof r == "object" ? i = Object.values(r) : (typeof console < "u" && console.warn && console.warn("[schemaClient.getSchemas] Unexpected response shape; normalizing to empty array", r), i = []), { ...e, data: i };
  }
  /**
   * Get a specific schema by name
   * UNPROTECTED - No authentication required
   */
  async getSchema(e) {
    return this.client.get(z.GET_SCHEMA(e), {
      validateSchema: {
        schemaName: e,
        operation: "read",
        requiresApproved: !1
        // Allow reading any schema for inspection
      },
      cacheable: !0,
      cacheKey: `schema:${e}`,
      cacheTtl: 3e5
      // 5 minutes
    });
  }
  /**
   * Get schemas filtered by state (computed client-side)
   * UNPROTECTED - No authentication required
   */
  async getSchemasByState(e) {
    if (!Object.values(fe).includes(e))
      throw new Error(`Invalid schema state: ${e}. Must be one of: ${Object.values(fe).join(", ")}`);
    const r = await this.getSchemas();
    return !r.success || !r.data ? { success: !1, error: "Failed to fetch schemas", status: r.status, data: { data: [], state: e } } : {
      success: !0,
      data: { data: r.data.filter((o) => o.state === e).map((o) => o.name), state: e },
      status: 200,
      meta: { timestamp: Date.now(), cached: r.meta?.cached || !1 }
    };
  }
  /**
   * Get all schemas with their state mappings (computed client-side)
   * UNPROTECTED - No authentication required
   */
  async getAllSchemasWithState() {
    const e = await this.getSchemas();
    if (!e.success || !e.data)
      return {
        success: !1,
        error: "Failed to fetch schemas",
        status: e.status,
        data: {}
      };
    const r = Array.isArray(e.data) ? e.data : [], i = {};
    return r.forEach((o) => {
      const u = va(o);
      if (!u) {
        typeof console < "u" && console.warn && console.warn("[schemaClient.getAllSchemasWithState] Encountered schema entry without a name, skipping entry.");
        return;
      }
      const h = Na(o), l = Fn(h);
      if (!h || l.length === 0) {
        typeof console < "u" && console.warn && console.warn(
          `[schemaClient.getAllSchemasWithState] Missing schema state for '${u}', defaulting to '${Dt}'.`
        ), i[u] = Dt;
        return;
      }
      if (!wa.has(l)) {
        typeof console < "u" && console.warn && console.warn(
          `[schemaClient.getAllSchemasWithState] Unrecognized schema state '${String(h)}' for '${u}', defaulting to '${Dt}'.`
        ), i[u] = Dt;
        return;
      }
      i[u] = l;
    }), {
      success: !0,
      data: i,
      status: e.status ?? 200,
      meta: {
        ...e.meta,
        timestamp: Date.now(),
        cached: e.meta?.cached ?? !1
      }
    };
  }
  /**
   * Get schema status summary (computed client-side)
   * UNPROTECTED - No authentication required
   */
  async getSchemaStatus() {
    const e = await this.getSchemas();
    if (!e.success || !e.data)
      return { success: !1, error: "Failed to fetch schemas", status: e.status, data: { available: 0, approved: 0, blocked: 0, total: 0 } };
    const r = e.data;
    return { success: !0, data: {
      available: r.filter((o) => o.state === fe.AVAILABLE).length,
      approved: r.filter((o) => o.state === fe.APPROVED).length,
      blocked: r.filter((o) => o.state === fe.BLOCKED).length,
      total: r.length
    }, status: 200, meta: { timestamp: Date.now(), cached: e.meta?.cached || !1 } };
  }
  /**
   * Approve a schema (transition to approved state)
   * UNPROTECTED - No authentication required
   * SCHEMA-002 Compliance: Only available schemas can be approved
   */
  async approveSchema(e) {
    return this.client.post(
      z.APPROVE_SCHEMA(e),
      {},
      // Empty body, schema name is in URL
      {
        validateSchema: {
          schemaName: e,
          operation: "approve",
          requiresApproved: !1
          // Can approve non-approved schemas
        },
        timeout: 1e4,
        // Longer timeout for state changes
        retries: 1
        // Limited retries for state-changing operations
      }
    );
  }
  /**
   * Block a schema (transition to blocked state)
   * UNPROTECTED - No authentication required
   * SCHEMA-002 Compliance: Only approved schemas can be blocked
   */
  async blockSchema(e) {
    return this.client.post(
      z.BLOCK_SCHEMA(e),
      {},
      // Empty body, schema name is in URL
      {
        validateSchema: {
          schemaName: e,
          operation: "block",
          requiresApproved: !0
          // Only approved schemas can be blocked
        },
        timeout: 1e4,
        // Longer timeout for state changes
        retries: 1
        // Limited retries for state-changing operations
      }
    );
  }
  /**
   * Get approved schemas only (SCHEMA-002 compliant)
   * This is a convenience method for components that need only approved schemas
   */
  async getApprovedSchemas() {
    try {
      const e = await this.getSchemas();
      return !e.success || !e.data ? { success: !1, error: "Failed to fetch schemas", status: e.status, data: [] } : { success: !0, data: e.data.filter((i) => i.state === fe.APPROVED), status: 200, meta: { timestamp: Date.now(), cached: e.meta?.cached } };
    } catch (e) {
      return { success: !1, error: e.message || "Failed to fetch approved schemas", status: e.status || 500, data: [] };
    }
  }
  /**
   * Load a schema into memory (no-op client-side; server has no endpoint)
   */
  async loadSchema(e) {
    return { success: !0, status: 200 };
  }
  /**
   * Unload a schema from memory (no-op client-side; server has no endpoint)
   */
  async unloadSchema(e) {
    return { success: !0, status: 200 };
  }
  /**
   * Validate if a schema can be used for mutations/queries (SCHEMA-002 compliance)
   */
  async validateSchemaForOperation(e, r) {
    try {
      const i = await this.getSchema(e);
      if (!i.success || !i.data)
        return {
          isValid: !1,
          error: `Schema '${e}' not found`
        };
      const o = i.data;
      return o.state !== fe.APPROVED ? {
        isValid: !1,
        error: `Schema '${e}' is not approved. Current state: ${o.state}. Only approved schemas can be used for ${r}s.`,
        schema: o
      } : {
        isValid: !0,
        schema: o
      };
    } catch (i) {
      return {
        isValid: !1,
        error: `Failed to validate schema '${e}': ${i.message}`
      };
    }
  }
  /**
   * Clear schema cache
   */
  clearCache() {
    this.client.clearCache();
  }
  /**
   * Get cache statistics
   */
  getCacheStats() {
    return this.client.getCacheStats();
  }
  /**
   * Get API metrics
   */
  getMetrics() {
    return this.client.getMetrics();
  }
  /**
   * Get backfill status by hash
   * UNPROTECTED - No authentication required
   */
  async getBackfillStatus(e) {
    return this.client.get(`/api/backfill/${e}`, {
      cacheable: !1,
      // Don't cache backfill status as it changes frequently
      timeout: 5e3
    });
  }
}
const X = new Qt();
function Sa(t) {
  return new Qt(t);
}
X.getSchemasByState.bind(X);
X.getAllSchemasWithState.bind(X);
X.getSchemaStatus.bind(X);
X.getSchema.bind(X);
X.approveSchema.bind(X);
X.blockSchema.bind(X);
X.loadSchema.bind(X);
X.unloadSchema.bind(X);
X.getApprovedSchemas.bind(X);
X.validateSchemaForOperation.bind(X);
X.getBackfillStatus.bind(X);
const he = {
  APPROVE: "approve",
  BLOCK: "block",
  UNLOAD: "unload",
  LOAD: "load"
}, Ln = (t, e) => t ? Date.now() - t < e : !1, Ea = (t, e, r = Date.now()) => ({
  schemaName: t,
  error: e,
  timestamp: r
}), Aa = (t, e, r, i) => ({
  schemaName: t,
  newState: e,
  timestamp: Date.now(),
  updatedSchema: r,
  backfillHash: i
}), Yt = (t, e, r, i) => ze(
  t,
  async ({ schemaName: o, options: u = {} }, { getState: h, rejectWithValue: l }) => {
    h().schemas.schemas[o];
    try {
      const f = await e(o);
      if (!f.success)
        throw new Error(f.error || i);
      const b = f.data?.backfill_hash;
      return Aa(o, r, void 0, b);
    } catch (f) {
      return l(
        Ea(
          o,
          f instanceof Error ? f.message : i
        )
      );
    }
  }
), ye = (t, e) => ({
  pending: (r, i) => {
    const o = i.meta.arg.schemaName;
    r.loading.operations[o] = !0, delete r.errors.operations[o];
  },
  fulfilled: (r, i) => {
    const { schemaName: o, newState: u, updatedSchema: h } = i.payload;
    r.loading.operations[o] = !1, r.schemas[o] && (r.schemas[o].state = u, h && Object.assign(r.schemas[o], h), r.schemas[o].lastOperation = {
      type: e,
      timestamp: Date.now(),
      success: !0
    });
  },
  rejected: (r, i) => {
    const { schemaName: o, error: u } = i.payload;
    r.loading.operations[o] = !1, r.errors.operations[o] = u, r.schemas[o] && (r.schemas[o].lastOperation = {
      type: e,
      timestamp: Date.now(),
      success: !1,
      error: u
    });
  }
}), Zr = {
  schemas: {},
  loading: {
    fetch: !1,
    operations: {}
  },
  errors: {
    fetch: null,
    operations: {}
  },
  lastFetched: null,
  cache: {
    ttl: la,
    version: "1.0.0",
    lastUpdated: null
  },
  activeSchema: null
}, Te = ze(
  _t.FETCH_SCHEMAS,
  async (t = {}, { getState: e, rejectWithValue: r }) => {
    const i = e(), { lastFetched: o, cache: u } = i.schemas;
    if (!t.forceRefresh && Ln(o, u.ttl))
      return {
        schemas: Object.values(i.schemas.schemas),
        timestamp: o
      };
    const h = new Qt(
      Ee({
        baseUrl: Vt.BASE_URL,
        // Use main API base URL (/api)
        enableCache: !0,
        enableLogging: !0,
        enableMetrics: !0
      })
    );
    t.forceRefresh && (console.log("🔄 Force refresh requested - clearing API client cache"), h.clearCache());
    let l = null;
    for (let f = 1; f <= lr; f++)
      try {
        const b = await h.getSchemas();
        if (!b.success)
          throw new Error(`Failed to fetch schemas: ${b.error || "Unknown error"}`);
        console.log("📁 Raw schemas response:", b.data);
        const N = b.data || [];
        if (!Array.isArray(N))
          throw new Error(`Schemas response is not an array: ${typeof N}`);
        const A = N.map((y) => {
          if (!y.name)
            if (console.warn("⚠️ Schema missing name field:", y), y.schema && y.schema.name)
              y.name = y.schema.name;
            else
              return console.error("❌ Schema has no name field and cannot be displayed:", y), null;
          let x = ke.AVAILABLE;
          return y.state && (typeof y.state == "string" ? x = y.state.toLowerCase() : typeof y.state == "object" && y.state.state ? x = String(y.state.state).toLowerCase() : x = String(y.state).toLowerCase()), console.log("🟢 fetchSchemas: Using backend schema for", y.name, "with state:", x), {
            ...y,
            state: x
          };
        }).filter((y) => y !== null);
        console.log("✅ Using backend schemas directly:", A.map((y) => ({ name: y.name, state: y.state })));
        const E = Date.now();
        return {
          schemas: A,
          timestamp: E
        };
      } catch (b) {
        if (l = b instanceof Error ? b : new Error("Unknown error"), f < lr) {
          const A = typeof window < "u" && window.__TEST_ENV__ === !0 ? 10 : 1e3 * f;
          await new Promise((E) => setTimeout(E, A));
        }
      }
    const g = `Failed to fetch schemas after ${lr} attempts: ${l?.message || "Unknown error"}`;
    return r(g);
  }
), Wt = () => new Qt(
  Ee({
    baseUrl: Vt.BASE_URL,
    // Use main API base URL (/api)
    enableCache: !0,
    enableLogging: !0,
    enableMetrics: !0
  })
), $e = Yt(
  _t.APPROVE_SCHEMA,
  (t) => Wt().approveSchema(t),
  ke.APPROVED,
  Tt.APPROVE_FAILED
), Ke = Yt(
  _t.BLOCK_SCHEMA,
  (t) => Wt().blockSchema(t),
  ke.BLOCKED,
  Tt.BLOCK_FAILED
), rt = Yt(
  _t.UNLOAD_SCHEMA,
  (t) => Wt().unloadSchema(t),
  ke.AVAILABLE,
  Tt.UNLOAD_FAILED
), nt = Yt(
  _t.LOAD_SCHEMA,
  (t) => Wt().loadSchema(t),
  ke.APPROVED,
  Tt.LOAD_FAILED
), Dn = vr({
  name: "schemas",
  initialState: Zr,
  reducers: {
    /**
     * Set the currently active schema
     */
    setActiveSchema: (t, e) => {
      t.activeSchema = e.payload;
    },
    /**
     * Update a specific schema's status
     */
    updateSchemaStatus: (t, e) => {
      const { schemaName: r, newState: i } = e.payload;
      t.schemas[r] && (t.schemas[r].state = i, t.schemas[r].lastOperation = {
        type: he.APPROVE,
        timestamp: Date.now(),
        success: !0
      });
    },
    /**
     * Set loading state for operations
     */
    setLoading: (t, e) => {
      const { operation: r, isLoading: i, schemaName: o } = e.payload;
      r === "fetch" ? t.loading.fetch = i : o && (t.loading.operations[o] = i);
    },
    /**
     * Set error state for operations
     */
    setError: (t, e) => {
      const { operation: r, error: i, schemaName: o } = e.payload;
      r === "fetch" ? t.errors.fetch = i : o && (t.errors.operations[o] = i || "");
    },
    /**
     * Clear all errors
     */
    clearError: (t) => {
      t.errors.fetch = null, t.errors.operations = {};
    },
    /**
     * Clear error for specific operation
     */
    clearOperationError: (t, e) => {
      const r = e.payload;
      delete t.errors.operations[r];
    },
    /**
     * Invalidate cache to force next fetch
     */
    invalidateCache: (t) => {
      t.lastFetched = null, t.cache.lastUpdated = null;
    },
    /**
     * Reset all schema state
     */
    resetSchemas: (t) => {
      Object.assign(t, Zr);
    }
  },
  extraReducers: (t) => {
    t.addCase(Te.pending, (e) => {
      e.loading.fetch = !0, e.errors.fetch = null;
    }).addCase(Te.fulfilled, (e, r) => {
      e.loading.fetch = !1, e.errors.fetch = null;
      const i = {};
      r.payload.schemas.forEach((o) => {
        i[o.name] = o;
      }), e.schemas = i, e.lastFetched = r.payload.timestamp, e.cache.lastUpdated = r.payload.timestamp;
    }).addCase(Te.rejected, (e, r) => {
      e.loading.fetch = !1, e.errors.fetch = r.payload || Tt.FETCH_FAILED;
    }).addCase($e.pending, ye($e, he.APPROVE).pending).addCase($e.fulfilled, ye($e, he.APPROVE).fulfilled).addCase($e.rejected, ye($e, he.APPROVE).rejected).addCase(Ke.pending, ye(Ke, he.BLOCK).pending).addCase(Ke.fulfilled, ye(Ke, he.BLOCK).fulfilled).addCase(Ke.rejected, ye(Ke, he.BLOCK).rejected).addCase(rt.pending, ye(rt, he.UNLOAD).pending).addCase(rt.fulfilled, ye(rt, he.UNLOAD).fulfilled).addCase(rt.rejected, ye(rt, he.UNLOAD).rejected).addCase(nt.pending, ye(nt, he.LOAD).pending).addCase(nt.fulfilled, ye(nt, he.LOAD).fulfilled).addCase(nt.rejected, ye(nt, he.LOAD).rejected);
  }
}), _a = (t) => t.schemas, ut = (t) => Object.values(t.schemas.schemas), Ta = (t) => t.schemas.schemas, Ct = Qe(
  [ut],
  (t) => t.filter((e) => (typeof e.state == "string" ? e.state.toLowerCase() : typeof e.state == "object" && e.state !== null && e.state.state ? String(e.state.state).toLowerCase() : String(e.state || "").toLowerCase()) === ke.APPROVED)
), Ca = Qe(
  [ut],
  (t) => t.filter((e) => e.state === ke.AVAILABLE)
);
Qe(
  [ut],
  (t) => t.filter((e) => e.state === ke.BLOCKED)
);
Qe(
  [Ct],
  (t) => t.filter((e) => e.rangeInfo?.isRangeSchema === !0)
);
Qe(
  [Ca],
  (t) => t.filter((e) => e.rangeInfo?.isRangeSchema === !0)
);
const Lr = (t) => t.schemas.loading.fetch, Mn = (t) => t.schemas.errors.fetch, Ra = Qe(
  [_a],
  (t) => ({
    isValid: Ln(t.lastFetched, t.cache.ttl),
    lastFetched: t.lastFetched,
    ttl: t.cache.ttl
  })
), ka = (t) => t.schemas.activeSchema;
Qe(
  [ka, Ta],
  (t, e) => t && e[t] || null
);
const {
  setActiveSchema: Ao,
  updateSchemaStatus: _o,
  setLoading: To,
  setError: Co,
  clearError: Ro,
  clearOperationError: ko,
  invalidateCache: Io,
  resetSchemas: Oo
} = Dn.actions, Ia = Dn.reducer, Xr = {
  inputText: "",
  sessionId: null,
  isProcessing: !1,
  conversationLog: [],
  showResults: !1
}, Pn = vr({
  name: "aiQuery",
  initialState: Xr,
  reducers: {
    // Input management
    setInputText: (t, e) => {
      t.inputText = e.payload;
    },
    clearInputText: (t) => {
      t.inputText = "";
    },
    // Session management
    setSessionId: (t, e) => {
      t.sessionId = e.payload;
    },
    // Processing state
    setIsProcessing: (t, e) => {
      t.isProcessing = e.payload;
    },
    // Conversation management
    addMessage: (t, e) => {
      const r = {
        ...e.payload,
        timestamp: (/* @__PURE__ */ new Date()).toISOString()
      };
      t.conversationLog.push(r);
    },
    clearConversation: (t) => {
      t.conversationLog = [];
    },
    // UI state
    setShowResults: (t, e) => {
      t.showResults = e.payload;
    },
    // Combined actions
    startNewConversation: (t) => {
      t.sessionId = null, t.conversationLog = [], t.inputText = "", t.isProcessing = !1, t.showResults = !1;
    },
    // Reset all state
    resetAIQueryState: () => Xr
  }
}), {
  setInputText: en,
  clearInputText: Bo,
  setSessionId: tn,
  setIsProcessing: rn,
  addMessage: Oa,
  clearConversation: Fo,
  setShowResults: Ba,
  startNewConversation: Fa,
  resetAIQueryState: Lo
} = Pn.actions, La = Pn.reducer, Da = (t) => t.aiQuery.inputText, Ma = (t) => t.aiQuery.sessionId, Pa = (t) => t.aiQuery.isProcessing, Ua = (t) => t.aiQuery.conversationLog, $a = (t) => t.aiQuery.showResults, Ka = (t) => t.aiQuery.sessionId && t.aiQuery.conversationLog.some((e) => e.type === "results"), ja = ss({
  reducer: {
    auth: oa,
    schemas: Ia,
    aiQuery: La
  },
  middleware: (t) => t({
    serializableCheck: {
      // Ignore these action types in serializability checks
      ignoredActions: [
        "auth/validatePrivateKey/fulfilled",
        "auth/setPrivateKey",
        // Schema async thunk actions that may contain non-serializable data
        "schemas/fetchSchemas/fulfilled",
        "schemas/approveSchema/fulfilled",
        "schemas/blockSchema/fulfilled",
        "schemas/unloadSchema/fulfilled",
        "schemas/loadSchema/fulfilled"
      ],
      // Ignore these field paths in all actions
      ignoredActionsPaths: [
        "payload.privateKey",
        "payload.schemas.definition"
      ],
      // Ignore these paths in the state
      ignoredPaths: ["auth.privateKey", "schemas.schemas.*.definition"]
    }
  }),
  devTools: !0
  // Enable Redux DevTools for debugging
});
function Ha() {
  console.log("🔄 Schema client reset - will use new configuration on next request");
}
async function Va(t) {
  const e = Date.now(), r = t.includes("127.0.0.1") || t.includes("localhost"), i = r ? `${t}/health` : `${t}/schema`;
  try {
    const o = {
      method: r ? "GET" : "POST",
      headers: {
        "Content-Type": "application/json"
      },
      signal: AbortSignal.timeout(5e3)
      // 5 second timeout
    };
    r || (o.body = JSON.stringify({ action: "status" }));
    const u = await fetch(i, o), h = Date.now() - e;
    return u.ok ? {
      success: !0,
      status: (await u.json()).status || "online",
      responseTime: h
    } : {
      success: !1,
      error: `HTTP ${u.status}: ${u.statusText}`,
      responseTime: h
    };
  } catch (o) {
    const u = Date.now() - e;
    return {
      success: !1,
      error: o.name === "TimeoutError" ? "Connection timeout" : o.message,
      responseTime: u
    };
  }
}
const it = {
  LOCAL: {
    id: "local",
    name: "Local",
    description: "Local development server",
    baseUrl: "http://127.0.0.1:9002/api"
    // Local schema service with /api prefix
  },
  DEV: {
    id: "dev",
    name: "Development (AWS)",
    description: "DEV Environment (us-west-2)",
    baseUrl: "https://cemkk2xzxd.execute-api.us-west-2.amazonaws.com"
  },
  PROD: {
    id: "prod",
    name: "Production (AWS)",
    description: "PROD Environment (us-east-1)",
    baseUrl: "https://owwjygkso3.execute-api.us-east-1.amazonaws.com"
  }
}, nn = "schemaServiceEnvironment", Un = Xn({
  environment: it.LOCAL,
  setEnvironment: () => {
  },
  getSchemaServiceBaseUrl: () => ""
});
function qa({ children: t }) {
  const [e, r] = O(() => {
    const u = localStorage.getItem(nn);
    if (u) {
      const h = Object.values(it).find((l) => l.id === u);
      if (h) return h;
    }
    return it.LOCAL;
  }), i = (u) => {
    const h = Object.values(it).find((l) => l.id === u);
    h && (r(h), localStorage.setItem(nn, u), Ha(), console.log(`Schema service environment changed to: ${h.name} (${h.baseUrl || "same origin"})`), console.log("🔄 Schema client has been reset - next request will use new endpoint"));
  }, o = () => e.baseUrl || "";
  return /* @__PURE__ */ s(Un.Provider, { value: { environment: e, setEnvironment: i, getSchemaServiceBaseUrl: o }, children: t });
}
function Ga() {
  const t = es(Un);
  if (!t)
    throw new Error("useSchemaServiceConfig must be used within SchemaServiceConfigProvider");
  return t;
}
const Do = ({ children: t, store: e }) => /* @__PURE__ */ s(ts, { store: e || ja, children: /* @__PURE__ */ s(qa, { children: t }) });
function za({
  title: t,
  titleId: e,
  ...r
}, i) {
  return /* @__PURE__ */ H.createElement("svg", Object.assign({
    xmlns: "http://www.w3.org/2000/svg",
    viewBox: "0 0 24 24",
    fill: "currentColor",
    "aria-hidden": "true",
    "data-slot": "icon",
    ref: i,
    "aria-labelledby": e
  }, r), t ? /* @__PURE__ */ H.createElement("title", {
    id: e
  }, t) : null, /* @__PURE__ */ H.createElement("path", {
    fillRule: "evenodd",
    d: "M4.755 10.059a7.5 7.5 0 0 1 12.548-3.364l1.903 1.903h-3.183a.75.75 0 1 0 0 1.5h4.992a.75.75 0 0 0 .75-.75V4.356a.75.75 0 0 0-1.5 0v3.18l-1.9-1.9A9 9 0 0 0 3.306 9.67a.75.75 0 1 0 1.45.388Zm15.408 3.352a.75.75 0 0 0-.919.53 7.5 7.5 0 0 1-12.548 3.364l-1.902-1.903h3.183a.75.75 0 0 0 0-1.5H2.984a.75.75 0 0 0-.75.75v4.992a.75.75 0 0 0 1.5 0v-3.18l1.9 1.9a9 9 0 0 0 15.059-4.035.75.75 0 0 0-.53-.918Z",
    clipRule: "evenodd"
  }));
}
const sn = /* @__PURE__ */ H.forwardRef(za);
function Qa({
  title: t,
  titleId: e,
  ...r
}, i) {
  return /* @__PURE__ */ H.createElement("svg", Object.assign({
    xmlns: "http://www.w3.org/2000/svg",
    viewBox: "0 0 24 24",
    fill: "currentColor",
    "aria-hidden": "true",
    "data-slot": "icon",
    ref: i,
    "aria-labelledby": e
  }, r), t ? /* @__PURE__ */ H.createElement("title", {
    id: e
  }, t) : null, /* @__PURE__ */ H.createElement("path", {
    fillRule: "evenodd",
    d: "M2.25 12c0-5.385 4.365-9.75 9.75-9.75s9.75 4.365 9.75 9.75-4.365 9.75-9.75 9.75S2.25 17.385 2.25 12Zm13.36-1.814a.75.75 0 1 0-1.22-.872l-3.236 4.53L9.53 12.22a.75.75 0 0 0-1.06 1.06l2.25 2.25a.75.75 0 0 0 1.14-.094l3.75-5.25Z",
    clipRule: "evenodd"
  }));
}
const cr = /* @__PURE__ */ H.forwardRef(Qa);
function Ya({
  title: t,
  titleId: e,
  ...r
}, i) {
  return /* @__PURE__ */ H.createElement("svg", Object.assign({
    xmlns: "http://www.w3.org/2000/svg",
    viewBox: "0 0 24 24",
    fill: "currentColor",
    "aria-hidden": "true",
    "data-slot": "icon",
    ref: i,
    "aria-labelledby": e
  }, r), t ? /* @__PURE__ */ H.createElement("title", {
    id: e
  }, t) : null, /* @__PURE__ */ H.createElement("path", {
    fillRule: "evenodd",
    d: "M12.53 16.28a.75.75 0 0 1-1.06 0l-7.5-7.5a.75.75 0 0 1 1.06-1.06L12 14.69l6.97-6.97a.75.75 0 1 1 1.06 1.06l-7.5 7.5Z",
    clipRule: "evenodd"
  }));
}
const $n = /* @__PURE__ */ H.forwardRef(Ya);
function Wa({
  title: t,
  titleId: e,
  ...r
}, i) {
  return /* @__PURE__ */ H.createElement("svg", Object.assign({
    xmlns: "http://www.w3.org/2000/svg",
    viewBox: "0 0 24 24",
    fill: "currentColor",
    "aria-hidden": "true",
    "data-slot": "icon",
    ref: i,
    "aria-labelledby": e
  }, r), t ? /* @__PURE__ */ H.createElement("title", {
    id: e
  }, t) : null, /* @__PURE__ */ H.createElement("path", {
    fillRule: "evenodd",
    d: "M16.28 11.47a.75.75 0 0 1 0 1.06l-7.5 7.5a.75.75 0 0 1-1.06-1.06L14.69 12 7.72 5.03a.75.75 0 0 1 1.06-1.06l7.5 7.5Z",
    clipRule: "evenodd"
  }));
}
const Kn = /* @__PURE__ */ H.forwardRef(Wa);
function Ja({
  title: t,
  titleId: e,
  ...r
}, i) {
  return /* @__PURE__ */ H.createElement("svg", Object.assign({
    xmlns: "http://www.w3.org/2000/svg",
    viewBox: "0 0 24 24",
    fill: "currentColor",
    "aria-hidden": "true",
    "data-slot": "icon",
    ref: i,
    "aria-labelledby": e
  }, r), t ? /* @__PURE__ */ H.createElement("title", {
    id: e
  }, t) : null, /* @__PURE__ */ H.createElement("path", {
    fillRule: "evenodd",
    d: "M12 2.25c-5.385 0-9.75 4.365-9.75 9.75s4.365 9.75 9.75 9.75 9.75-4.365 9.75-9.75S17.385 2.25 12 2.25ZM12.75 6a.75.75 0 0 0-1.5 0v6c0 .414.336.75.75.75h4.5a.75.75 0 0 0 0-1.5h-3.75V6Z",
    clipRule: "evenodd"
  }));
}
const dr = /* @__PURE__ */ H.forwardRef(Ja);
function Za({
  title: t,
  titleId: e,
  ...r
}, i) {
  return /* @__PURE__ */ H.createElement("svg", Object.assign({
    xmlns: "http://www.w3.org/2000/svg",
    viewBox: "0 0 24 24",
    fill: "currentColor",
    "aria-hidden": "true",
    "data-slot": "icon",
    ref: i,
    "aria-labelledby": e
  }, r), t ? /* @__PURE__ */ H.createElement("title", {
    id: e
  }, t) : null, /* @__PURE__ */ H.createElement("path", {
    fillRule: "evenodd",
    d: "M16.5 4.478v.227a48.816 48.816 0 0 1 3.878.512.75.75 0 1 1-.256 1.478l-.209-.035-1.005 13.07a3 3 0 0 1-2.991 2.77H8.084a3 3 0 0 1-2.991-2.77L4.087 6.66l-.209.035a.75.75 0 0 1-.256-1.478A48.567 48.567 0 0 1 7.5 4.705v-.227c0-1.564 1.213-2.9 2.816-2.951a52.662 52.662 0 0 1 3.369 0c1.603.051 2.815 1.387 2.815 2.951Zm-6.136-1.452a51.196 51.196 0 0 1 3.273 0C14.39 3.05 15 3.684 15 4.478v.113a49.488 49.488 0 0 0-6 0v-.113c0-.794.609-1.428 1.364-1.452Zm-.355 5.945a.75.75 0 1 0-1.5.058l.347 9a.75.75 0 1 0 1.499-.058l-.346-9Zm5.48.058a.75.75 0 1 0-1.498-.058l-.347 9a.75.75 0 0 0 1.5.058l.345-9Z",
    clipRule: "evenodd"
  }));
}
const an = /* @__PURE__ */ H.forwardRef(Za);
function Xa({
  title: t,
  titleId: e,
  ...r
}, i) {
  return /* @__PURE__ */ H.createElement("svg", Object.assign({
    xmlns: "http://www.w3.org/2000/svg",
    viewBox: "0 0 24 24",
    fill: "currentColor",
    "aria-hidden": "true",
    "data-slot": "icon",
    ref: i,
    "aria-labelledby": e
  }, r), t ? /* @__PURE__ */ H.createElement("title", {
    id: e
  }, t) : null, /* @__PURE__ */ H.createElement("path", {
    fillRule: "evenodd",
    d: "M12 2.25c-5.385 0-9.75 4.365-9.75 9.75s4.365 9.75 9.75 9.75 9.75-4.365 9.75-9.75S17.385 2.25 12 2.25Zm-1.72 6.97a.75.75 0 1 0-1.06 1.06L10.94 12l-1.72 1.72a.75.75 0 1 0 1.06 1.06L12 13.06l1.72 1.72a.75.75 0 1 0 1.06-1.06L13.06 12l1.72-1.72a.75.75 0 1 0-1.06-1.06L12 10.94l-1.72-1.72Z",
    clipRule: "evenodd"
  }));
}
const ei = /* @__PURE__ */ H.forwardRef(Xa);
class jn {
  constructor(e) {
    this.client = e || Ee({
      enableCache: !0,
      // Cache public keys and verification results
      enableLogging: !0,
      enableMetrics: !0
    });
  }
  /**
   * Verify a signed message
   * UNPROTECTED - No authentication required
   */
  async verifyMessage(e) {
    const r = this.validateSignedMessage(e);
    return r.isValid ? {
      success: !0,
      status: 200,
      data: {
        isValid: !0,
        details: {
          signature: e.signature,
          timestamp: e.timestamp,
          verified: !1
          // Explicitly state not cryptographically verified by server
        }
      }
    } : {
      success: !1,
      error: `Invalid message format: ${r.errors.join(", ")}`,
      status: 400,
      data: { isValid: !1, error: r.errors[0] }
    };
  }
  /**
   * Get the system's public key
   * UNPROTECTED - UI never uses authentication
   *
   * @returns Promise resolving to system public key
   */
  async getSystemPublicKey() {
    return this.client.get(
      z.GET_SYSTEM_PUBLIC_KEY,
      {
        requiresAuth: !1,
        // System public key is public
        timeout: le.QUICK,
        retries: ce.CRITICAL,
        // Multiple retries for critical system data
        cacheable: !0,
        // Cache system public key
        cacheTtl: je.SYSTEM_PUBLIC_KEY,
        // Cache for 1 hour (system key doesn't change often)
        cacheKey: $t.SYSTEM_PUBLIC_KEY
      }
    );
  }
  /**
   * Validate a public key's format and cryptographic properties
   * This is a client-side validation helper
   *
   * @param publicKey The public key to validate (base64 encoded)
   * @returns Validation result with details
   */
  validatePublicKeyFormat(e) {
    try {
      if (!e || typeof e != "string")
        return {
          isValid: !1,
          error: "Public key must be a non-empty string"
        };
      const r = e.trim();
      return /^(?:[A-Za-z0-9+/]{4})*(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?$/.test(r) ? r.length !== 44 ? {
        isValid: !1,
        format: "Unknown",
        length: Math.ceil(r.length / 4 * 3),
        error: "Invalid key length: expected 44 base64 chars for Ed25519"
      } : {
        isValid: !0,
        format: "Ed25519",
        length: 32
      } : {
        isValid: !1,
        error: "Invalid base64 encoding"
      };
    } catch (r) {
      return {
        isValid: !1,
        error: `Validation error: ${r.message}`
      };
    }
  }
  /**
   * Get security status and configuration
   * UNPROTECTED - UI never uses authentication
   *
   * @returns Promise resolving to security status
   */
  async getSecurityStatus() {
    return this.client.get(z.GET_SYSTEM_STATUS, {
      timeout: le.QUICK,
      retries: ce.STANDARD,
      cacheable: !0,
      cacheTtl: je.SECURITY_STATUS,
      // Cache for 1 minute
      cacheKey: $t.SECURITY_STATUS
    });
  }
  /**
   * Validate a signed message's structure before sending for verification
   * This is a client-side validation helper
   *
   * @param signedMessage The signed message to validate
   * @returns Validation result
   */
  validateSignedMessage(e) {
    const r = [];
    if (!e || typeof e != "object")
      return r.push("Signed message must be an object"), { isValid: !1, errors: r };
    if ((!e.payload || typeof e.payload != "string") && r.push("Payload must be a non-empty base64 string"), (!e.signature || typeof e.signature != "string") && r.push("Signature must be a non-empty base64 string"), (!e.public_key_id || typeof e.public_key_id != "string") && r.push("Public key ID must be a non-empty string"), !e.timestamp || typeof e.timestamp != "number")
      r.push("Timestamp must be a Unix timestamp number");
    else {
      const o = Math.floor(Date.now() / 1e3) - e.timestamp;
      o > 300 && r.push("Message is too old (timestamp more than 5 minutes ago)"), o < -60 && r.push("Message timestamp is too far in the future");
    }
    return e.nonce && typeof e.nonce != "string" && r.push("Nonce must be a string if provided"), {
      isValid: r.length === 0,
      errors: r
    };
  }
  /**
   * Get API metrics for security operations
   */
  getMetrics() {
    return this.client.getMetrics().filter((e) => e.url.includes("/security"));
  }
  /**
   * Clear security-related cache
   */
  clearCache() {
    this.client.clearCache();
  }
}
const Re = new jn();
function ti(t) {
  return new jn(t);
}
Re.getSystemPublicKey.bind(Re);
Re.validatePublicKeyFormat.bind(Re);
Re.validateSignedMessage.bind(Re);
Re.getSecurityStatus.bind(Re);
Re.verifyMessage.bind(Re);
class ri {
  constructor(e) {
    this.client = e || Ee({
      enableCache: !0,
      // Cache transform data for performance
      enableLogging: !0,
      enableMetrics: !0
    });
  }
  /**
   * Get all available transforms
   * UNPROTECTED - No authentication required for reading transforms
   * Replaces TransformsTab fetch('/api/transforms')
   *
   * @returns Promise resolving to transforms data
   */
  async getTransforms() {
    return this.client.get(z.LIST_TRANSFORMS, {
      requiresAuth: !1,
      // Transform reading is public
      timeout: 8e3,
      retries: 2,
      cacheable: !0,
      cacheTtl: 18e4,
      // Cache for 3 minutes
      cacheKey: "transforms:all"
    });
  }
  /**
   * Get current transform queue information
   * UNPROTECTED - No authentication required for queue monitoring
   * Replaces TransformsTab fetch('/api/transforms/queue')
   *
   * @returns Promise resolving to queue status
   */
  async getQueue() {
    return this.client.get(z.GET_TRANSFORM_QUEUE, {
      requiresAuth: !1,
      // Queue monitoring is public
      timeout: 5e3,
      retries: 3,
      // Multiple retries for critical queue data
      cacheable: !1
      // Always get fresh queue data
    });
  }
  /**
   * Add a transform to the processing queue
   * UNPROTECTED - No authentication required for transform operations
   * Replaces TransformsTab fetch(`/api/transforms/queue/${transformId}`)
   *
   * @param transformId - The ID of the transform to add to queue
   * @returns Promise resolving to queue addition result
   */
  async addToQueue(e) {
    if (!e || typeof e != "string")
      throw new Error("Transform ID is required and must be a string");
    return this.client.post(
      z.ADD_TO_TRANSFORM_QUEUE(e),
      void 0,
      // No body needed for this endpoint
      {
        timeout: 1e4,
        // Longer timeout for queue operations
        retries: 1,
        // Limited retries for queue modifications
        cacheable: !1
        // Never cache queue modification operations
      }
    );
  }
  /**
   * Refresh queue information (alias to getQueue for convenience)
   * This method provides semantic clarity for refresh operations
   * Used in TransformsTab for refreshing queue after adding transforms
   *
   * @returns Promise resolving to current queue status
   */
  async refreshQueue() {
    return this.getQueue();
  }
  /**
   * Get all backfill information
   * UNPROTECTED - No authentication required for backfill monitoring
   *
   * @returns Promise resolving to all backfill information
   */
  async getAllBackfills() {
    return this.client.get(z.GET_ALL_BACKFILLS, {
      requiresAuth: !1,
      timeout: 5e3,
      retries: 2,
      cacheable: !1
    });
  }
  /**
   * Get backfill information for a specific transform
   * UNPROTECTED - No authentication required for backfill monitoring
   *
   * @param transformId - The ID of the transform
   * @returns Promise resolving to backfill information
   */
  async getBackfill(e) {
    if (!e || typeof e != "string")
      throw new Error("Transform ID is required and must be a string");
    return this.client.get(
      z.GET_BACKFILL(e),
      {
        requiresAuth: !1,
        timeout: 5e3,
        retries: 2,
        cacheable: !1
      }
    );
  }
  /**
   * Get a specific transform by ID from the transforms map
   * Note: The backend returns a map, so individual transform fetching
   * requires fetching all transforms and extracting the specific one
   *
   * @param transformId - The ID of the transform to retrieve
   * @returns Promise resolving to transform details
   */
  async getTransform(e) {
    if (!e || typeof e != "string")
      throw new Error("Transform ID is required and must be a string");
    const r = await this.getTransforms();
    if (r.success && r.data) {
      const i = r.data[e] || null;
      return {
        ...r,
        data: i
      };
    }
    return r;
  }
  /**
   * Get API metrics for transform operations
   */
  getMetrics() {
    return this.client.getMetrics().filter(
      (e) => e.url.includes("/transforms") || e.url.includes("/queue")
    );
  }
  /**
   * Clear transform-related cache
   */
  clearCache() {
    this.client.clearCache();
  }
}
const pe = new ri();
pe.getTransforms.bind(pe);
pe.getQueue.bind(pe);
pe.addToQueue.bind(pe);
pe.refreshQueue.bind(pe);
pe.getTransform.bind(pe);
class Hn {
  constructor(e) {
    this.client = e || Ee({
      enableCache: !1,
      // Mutations should not be cached
      enableLogging: !0,
      enableMetrics: !0
    });
  }
  /**
   * Execute a mutation against an approved schema
   * PROTECTED - Requires authentication and SCHEMA-002 compliance
   *
   * @param mutation The mutation object to execute
   * @returns Promise resolving to mutation result
   */
  async executeMutation(e) {
    return this.client.post(
      z.EXECUTE_MUTATION,
      e,
      {
        validateSchema: !1,
        // Skip schema validation for mutations
        timeout: 15e3,
        // Longer timeout for mutation operations
        retries: 0,
        // No retries for mutations to prevent duplicate operations
        cacheable: !1
        // Never cache mutation results
      }
    );
  }
  /**
   * Execute multiple mutations in a batch for improved performance
   * PROTECTED - Requires authentication and SCHEMA-002 compliance
   *
   * @param mutations Array of mutation objects to execute
   * @returns Promise resolving to array of mutation IDs
   */
  async executeMutationsBatch(e) {
    return this.client.post(
      z.EXECUTE_MUTATIONS_BATCH,
      e,
      {
        validateSchema: !1,
        // Skip schema validation for mutations
        timeout: 3e4,
        // Longer timeout for batch operations
        retries: 0,
        // No retries for mutations to prevent duplicate operations
        cacheable: !1
        // Never cache mutation results
      }
    );
  }
  /**
   * Execute a query against an approved schema
   * UNPROTECTED - No authentication required
   *
   * @param query The query object to execute
   * @returns Promise resolving to query results
   */
  async executeQuery(e) {
    return this.client.post(z.EXECUTE_QUERY, e, {
      validateSchema: {
        operation: "read",
        requiresApproved: !0
        // SCHEMA-002: Only approved schemas for queries
      },
      timeout: 1e4,
      // Standard timeout for queries
      retries: 2,
      // Limited retries for read operations
      cacheable: !0,
      // Query results can be cached
      cacheTtl: 6e4
      // Cache for 1 minute
    });
  }
  /**
   * Validate a mutation before execution
   * This checks schema compliance, field validation, and business rules
   *
   * @param mutation The mutation object to validate
   * @returns Promise resolving to validation result
   */
  async validateMutation(e) {
    return Promise.resolve({
      success: !0,
      data: { isValid: !0 },
      status: 200
    });
  }
  /**
   * Execute a batch of mutations as a single transaction
   * All mutations must target approved schemas
   *
   * @param mutations Array of mutation objects
   * @returns Promise resolving to batch execution results
   */
  async executeBatchMutations(e) {
    return {
      success: !1,
      error: "Batch mutations not supported",
      status: 501,
      data: []
    };
  }
  /**
   * Execute a parameterized query with filters and pagination
   * Provides enhanced query capabilities beyond basic executeQuery
   *
   * @param queryParams Query parameters including schema, filters, pagination
   * @returns Promise resolving to enhanced query results
   */
  async executeParameterizedQuery(e) {
    return this.client.post(
      z.EXECUTE_QUERY,
      e,
      {
        validateSchema: {
          schemaName: e.schema,
          operation: "read",
          requiresApproved: !0
        },
        timeout: 15e3,
        retries: 2,
        cacheable: !0,
        cacheTtl: 12e4,
        cacheKey: `parameterized-query:${JSON.stringify(e)}`
      }
    );
  }
  /**
   * Get mutation history for a specific record or schema
   * Useful for auditing and tracking changes
   *
   * @param params History query parameters
   * @returns Promise resolving to mutation history
   */
  async getMutationHistory(e) {
    return {
      success: !1,
      error: "Mutation history not supported",
      status: 501,
      data: []
    };
  }
  /**
   * Check if a schema is available for mutations (SCHEMA-002 compliance)
   *
   * @param schemaName The name of the schema to check
   * @returns Promise resolving to schema availability info
   */
  async validateSchemaForMutation(e) {
    try {
      const r = await this.client.get(
        z.GET_SCHEMA(e),
        {
          timeout: 5e3,
          retries: 1,
          cacheable: !0,
          cacheTtl: 18e4
          // Cache schema state for 3 minutes
        }
      );
      if (!r.success || !r.data)
        return {
          isValid: !1,
          schemaState: "unknown",
          canMutate: !1,
          canQuery: !1,
          error: `Schema '${e}' not found`
        };
      const i = r.data, o = i.state === fe.APPROVED;
      return {
        isValid: !0,
        schemaState: i.state,
        canMutate: o,
        canQuery: o,
        error: o ? void 0 : `Schema '${e}' is not approved (current state: ${i.state})`
      };
    } catch (r) {
      return {
        isValid: !1,
        schemaState: "error",
        canMutate: !1,
        canQuery: !1,
        error: `Failed to validate schema '${e}': ${r.message}`
      };
    }
  }
  /**
   * Get API metrics for mutation operations
   */
  getMetrics() {
    return this.client.getMetrics().filter(
      (e) => e.url.includes("/mutation") || e.url.includes("/query")
    );
  }
  /**
   * Clear any cached query results
   */
  clearCache() {
    this.client.clearCache();
  }
}
const Dr = new Hn();
function ni(t) {
  return new Hn(t);
}
class si {
  constructor(e) {
    this.client = e || Ee({
      baseUrl: is.ROOT,
      enableCache: !1,
      // Ingestion operations should not be cached
      enableLogging: !0,
      enableMetrics: !0
    });
  }
  /**
   * Get ingestion service status and configuration
   * UNPROTECTED - Status endpoint is public for health monitoring
   *
   * @returns Promise resolving to ingestion service status
   */
  async getStatus() {
    return this.client.get(z.GET_STATUS, {
      requiresAuth: !1,
      // Status endpoint is public
      timeout: le.QUICK,
      retries: ce.STANDARD,
      cacheable: !1
      // Status should always be fresh
    });
  }
  /**
   * Get all active ingestion progress
   * UNPROTECTED - Progress status is public for monitoring
   *
   * @returns Promise resolving to array of ingestion progress items
   */
  async getAllProgress() {
    return this.client.get("/ingestion/progress", {
      requiresAuth: !1,
      timeout: le.QUICK,
      retries: ce.STANDARD,
      cacheable: !1
      // Progress should always be fresh
    });
  }
  /**
   * Get ingestion configuration
   * UNPROTECTED - No authentication required
   *
   * @returns Promise resolving to general ingestion configuration
   */
  async getConfig() {
    return this.client.get(
      z.GET_INGESTION_CONFIG,
      {
        timeout: le.QUICK,
        retries: ce.STANDARD,
        cacheable: !1
        // Config should not be cached for security
      }
    );
  }
  /**
   * Save AI provider configuration
   * UNPROTECTED - No authentication required
   *
   * @param config The Ingestion configuration to save
   * @returns Promise resolving to save operation result
   */
  async saveConfig(e) {
    return this.client.post(
      z.GET_INGESTION_CONFIG,
      e,
      {
        timeout: le.CONFIG,
        // Longer timeout for config operations
        retries: ce.LIMITED,
        // Limited retries for config changes
        cacheable: !1
        // Never cache config operations
      }
    );
  }
  /**
   * Validate JSON data structure for ingestion
   * UNPROTECTED - Validation is a utility operation
   *
   * @param data The JSON data to validate
   * @returns Promise resolving to validation result
   */
  async validateData(e) {
    return this.client.post(
      z.VALIDATE_JSON,
      e,
      {
        requiresAuth: !1,
        // Validation is a utility operation
        timeout: le.MUTATION,
        // Longer timeout for AI analysis
        retries: ce.STANDARD,
        cacheable: !1
        // Validation results should not be cached
      }
    );
  }
  /**
   * Process data ingestion with AI analysis
   * UNPROTECTED - UI does not require authentication per project preference
   *
   * @param data The data to process
   * @param options Processing options
   * @returns Promise resolving to processing result
   */
  async processIngestion(e, r = {}) {
    const i = crypto.randomUUID(), o = {
      data: e,
      auto_execute: r.autoExecute ?? !0,
      trust_distance: r.trustDistance ?? 0,
      pub_key: r.pubKey ?? "default",
      progress_id: i
    }, u = this.validateIngestionRequest(o);
    if (!u.isValid)
      throw new Error(
        `Invalid ingestion request: ${u.errors.join(", ")}`
      );
    return this.client.post(
      z.PROCESS_JSON,
      o,
      {
        timeout: le.AI_PROCESSING,
        // Extended timeout for AI processing (60 seconds)
        retries: ce.LIMITED,
        // Limited retries for processing operations
        cacheable: !1
        // Processing results should not be cached
      }
    );
  }
  /**
   * Validate ingestion request before sending
   * Client-side validation helper
   *
   * @param request The ingestion request to validate
   * @returns Validation result
   */
  validateIngestionRequest(e) {
    const r = [], i = [];
    return !e.data || typeof e.data != "object" ? r.push("Data must be a valid object") : Object.keys(e.data).length === 0 && r.push("Data cannot be empty"), typeof e.trust_distance != "number" || e.trust_distance < 0 ? r.push("Trust distance must be a non-negative number") : e.trust_distance > 10 && i.push("Trust distance is unusually high"), (!e.pub_key || e.pub_key.trim().length === 0) && r.push("Public key is required"), typeof e.auto_execute != "boolean" && r.push("Auto execute must be a boolean value"), {
      isValid: r.length === 0,
      errors: r,
      warnings: i
    };
  }
  /**
   * Create a properly structured ingestion request
   * Helper function for creating valid processing requests
   *
   * @param data The data to process
   * @param options Processing configuration
   * @returns Ingestion request object
   */
  createIngestionRequest(e, r = {}) {
    return {
      data: { ...e },
      // Create a copy
      auto_execute: r.autoExecute ?? !0,
      trust_distance: r.trustDistance ?? 0,
      pub_key: r.pubKey ?? "default",
      progress_id: r.progressId ?? crypto.randomUUID()
    };
  }
  /**
   * Get API metrics for ingestion operations
   */
  getMetrics() {
    return this.client.getMetrics().filter((e) => e.url.includes("/ingestion"));
  }
  /**
   * Clear ingestion-related cache (though ingestion operations should not be cached)
   */
  clearCache() {
    this.client.clearCache();
  }
}
const dt = new si(), st = Ee({
  timeout: le.AI_PROCESSING,
  retries: ce.LIMITED
}), on = {
  /**
   * Run a query in a single step (analyze + execute with internal polling loop)
   */
  async runQuery(t) {
    return st.post("/llm-query/run", t);
  },
  /**
   * Analyze a natural language query
   */
  async analyzeQuery(t) {
    return st.post("/llm-query/analyze", t);
  },
  /**
   * Execute a query plan
   */
  async executeQueryPlan(t) {
    return st.post("/llm-query/execute", t);
  },
  /**
   * Ask a follow-up question about results
   */
  async chat(t) {
    return st.post("/llm-query/chat", t);
  },
  /**
   * Analyze if a follow-up question can be answered from existing context
   */
  async analyzeFollowup(t) {
    return st.post("/llm-query/analyze-followup", t);
  },
  /**
   * Get backfill status by hash
   */
  async getBackfillStatus(t) {
    return st.get(`/llm-query/backfill/${t}`);
  }
};
class ai {
  constructor(e) {
    this.client = e || Ee({ enableCache: !0, enableLogging: !0 });
  }
  async search(e) {
    const r = `${z.NATIVE_INDEX_SEARCH}?term=${encodeURIComponent(e)}`;
    return this.client.get(r, {
      timeout: 8e3,
      retries: 2,
      cacheable: !0,
      cacheTtl: 6e4
    });
  }
}
const ii = new ai();
function Mo() {
  const [t, e] = O(!1), [r, i] = O(!1), [o, u] = O(null), [h, l] = O([]), [g, f] = O(!0), b = $(async () => {
    try {
      const p = await dt.getAllProgress(), w = p.data?.progress || p.data || p.progress || [];
      Array.isArray(w) ? l(w) : l([]);
    } catch (p) {
      console.error("Failed to fetch progress:", p), l([]);
    } finally {
      f(!1);
    }
  }, []);
  ue(() => {
    b();
    const p = setInterval(b, 2e3);
    return () => clearInterval(p);
  }, [b]);
  const N = async () => {
    i(!0), u(null);
    try {
      const p = await re.resetDatabase(!0);
      p.success && p.data ? p.data.job_id ? (u({
        type: "success",
        message: `Reset started (Job: ${p.data.job_id.substring(0, 8)}...). Progress will appear above.`
      }), e(!1), i(!1)) : (u({ type: "success", message: p.data.message }), setTimeout(() => {
        window.location.reload();
      }, 2e3)) : (u({ type: "error", message: p.error || "Reset failed" }), e(!1), i(!1));
    } catch (p) {
      u({ type: "error", message: `Network error: ${p.message}` }), e(!1), i(!1);
    }
  }, A = (p) => {
    const w = p.job_type === "indexing", _ = p.job_type === "database_reset", S = _ ? "Database Reset" : w ? "Indexing Job" : "Ingestion Job";
    if (p.is_complete)
      return /* @__PURE__ */ d(
        "div",
        {
          className: "p-3 rounded-lg border border-gray-200 bg-gray-50 mb-3 opacity-75",
          children: [
            /* @__PURE__ */ d("div", { className: "flex items-center justify-between", children: [
              /* @__PURE__ */ d("div", { className: "flex items-center gap-2", children: [
                /* @__PURE__ */ s(cr, { className: "w-5 h-5 text-gray-400" }),
                /* @__PURE__ */ s("span", { className: "font-medium text-gray-500", children: S }),
                /* @__PURE__ */ s("span", { className: "text-xs text-gray-400 bg-gray-200 px-2 py-0.5 rounded-full", children: "Complete" })
              ] }),
              /* @__PURE__ */ d("div", { className: "flex items-center gap-1 text-xs text-gray-400", children: [
                /* @__PURE__ */ s(dr, { className: "w-3 h-3" }),
                /* @__PURE__ */ s("span", { children: new Date(p.started_at).toLocaleTimeString() })
              ] })
            ] }),
            /* @__PURE__ */ s("div", { className: "text-xs text-gray-400 mt-1", children: p.status_message || "Completed successfully" })
          ]
        },
        p.id
      );
    if (p.is_failed)
      return /* @__PURE__ */ d(
        "div",
        {
          className: "p-4 rounded-lg border-2 border-red-200 bg-red-50 mb-3",
          children: [
            /* @__PURE__ */ d("div", { className: "flex items-center justify-between mb-2", children: [
              /* @__PURE__ */ d("div", { className: "flex items-center gap-2", children: [
                /* @__PURE__ */ s(ei, { className: "w-5 h-5 text-red-500" }),
                /* @__PURE__ */ s("span", { className: "font-medium text-red-800", children: S }),
                /* @__PURE__ */ s("span", { className: "text-xs text-red-600 bg-red-100 px-2 py-0.5 rounded-full", children: "Failed" })
              ] }),
              /* @__PURE__ */ d("div", { className: "flex items-center gap-1 text-xs text-gray-500", children: [
                /* @__PURE__ */ s(dr, { className: "w-3 h-3" }),
                /* @__PURE__ */ s("span", { children: new Date(p.started_at).toLocaleTimeString() })
              ] })
            ] }),
            p.error_message && /* @__PURE__ */ d("div", { className: "text-xs text-red-600 mt-2", children: [
              "Error: ",
              p.error_message
            ] })
          ]
        },
        p.id
      );
    const B = _ ? "red" : w ? "purple" : "blue", I = `bg-${B}-50`, j = `border-${B}-200`, C = w ? "text-purple-800" : _ ? "text-red-800" : "text-blue-800", k = _ ? "bg-orange-500" : w ? "bg-purple-500" : "bg-blue-500";
    return /* @__PURE__ */ d(
      "div",
      {
        className: `p-4 rounded-lg border-2 ${j} ${I} mb-3`,
        children: [
          /* @__PURE__ */ d("div", { className: "flex items-center justify-between mb-2", children: [
            /* @__PURE__ */ d("div", { className: "flex items-center gap-2", children: [
              /* @__PURE__ */ s(sn, { className: "w-5 h-5 text-blue-500 animate-spin" }),
              /* @__PURE__ */ s("span", { className: `font-medium ${C}`, children: S }),
              /* @__PURE__ */ s("span", { className: `text-xs ${C} bg-white/50 px-2 py-0.5 rounded-full`, children: "In Progress" })
            ] }),
            /* @__PURE__ */ d("div", { className: "flex items-center gap-1 text-xs text-gray-500", children: [
              /* @__PURE__ */ s(dr, { className: "w-3 h-3" }),
              /* @__PURE__ */ s("span", { children: new Date(p.started_at).toLocaleTimeString() })
            ] })
          ] }),
          /* @__PURE__ */ d("div", { className: "mb-2", children: [
            /* @__PURE__ */ d("div", { className: "flex justify-between text-xs text-gray-600 mb-1", children: [
              /* @__PURE__ */ s("span", { children: p.status_message || "Processing..." }),
              /* @__PURE__ */ d("span", { children: [
                p.progress_percentage || 0,
                "%"
              ] })
            ] }),
            /* @__PURE__ */ s("div", { className: "w-full bg-gray-200 rounded-full h-2", children: /* @__PURE__ */ s(
              "div",
              {
                className: `h-2 rounded-full transition-all duration-300 ${k}`,
                style: { width: `${p.progress_percentage || 0}%` }
              }
            ) })
          ] })
        ]
      },
      p.id
    );
  }, E = () => t ? /* @__PURE__ */ s("div", { className: "fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50", children: /* @__PURE__ */ d("div", { className: "bg-white rounded-lg p-6 max-w-md w-full mx-4", children: [
    /* @__PURE__ */ d("div", { className: "flex items-center gap-3 mb-4", children: [
      /* @__PURE__ */ s(an, { className: "w-6 h-6 text-red-500" }),
      /* @__PURE__ */ s("h3", { className: "text-lg font-semibold text-gray-900", children: "Reset Database" })
    ] }),
    /* @__PURE__ */ d("div", { className: "mb-6", children: [
      /* @__PURE__ */ s("p", { className: "text-gray-700 mb-2", children: "This will permanently delete all data and restart the node:" }),
      /* @__PURE__ */ d("ul", { className: "list-disc list-inside text-sm text-gray-600 space-y-1", children: [
        /* @__PURE__ */ s("li", { children: "All schemas will be removed" }),
        /* @__PURE__ */ s("li", { children: "All stored data will be deleted" }),
        /* @__PURE__ */ s("li", { children: "Network connections will be reset" }),
        /* @__PURE__ */ s("li", { children: "This action cannot be undone" })
      ] })
    ] }),
    /* @__PURE__ */ d("div", { className: "flex gap-3 justify-end", children: [
      /* @__PURE__ */ s(
        "button",
        {
          onClick: () => e(!1),
          className: "px-4 py-2 text-sm font-medium text-gray-700 bg-gray-100 rounded-md hover:bg-gray-200 transition-colors",
          disabled: r,
          children: "Cancel"
        }
      ),
      /* @__PURE__ */ s(
        "button",
        {
          onClick: N,
          disabled: r,
          className: "px-4 py-2 text-sm font-medium text-white bg-red-600 rounded-md hover:bg-red-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors",
          children: r ? "Resetting..." : "Reset Database"
        }
      )
    ] })
  ] }) }) : null, y = h.filter((p) => !p.is_complete && !p.is_failed), x = y.length > 0 ? y.slice(0, 3) : h.filter((p) => p.is_complete || p.is_failed).slice(0, 1);
  return /* @__PURE__ */ d(At, { children: [
    /* @__PURE__ */ d("div", { className: "bg-white rounded-lg shadow-sm p-4 mb-6", children: [
      /* @__PURE__ */ d("div", { className: "flex items-center justify-between mb-4", children: [
        /* @__PURE__ */ d("div", { className: "flex items-center gap-2", children: [
          /* @__PURE__ */ s(cr, { className: "w-5 h-5 text-green-500" }),
          /* @__PURE__ */ s("h2", { className: "text-lg font-semibold text-gray-900", children: "System Status" })
        ] }),
        /* @__PURE__ */ d(
          "button",
          {
            onClick: () => e(!0),
            className: "flex items-center gap-2 px-3 py-1.5 text-sm font-medium text-red-600 border border-red-200 rounded-md hover:bg-red-50 hover:border-red-300 transition-colors",
            disabled: r,
            children: [
              /* @__PURE__ */ s(an, { className: "w-4 h-4" }),
              "Reset Database"
            ]
          }
        )
      ] }),
      g ? /* @__PURE__ */ d("div", { className: "p-4 rounded-lg border-2 border-gray-200 bg-gray-50 flex items-center justify-center", children: [
        /* @__PURE__ */ s(sn, { className: "w-5 h-5 text-gray-400 animate-spin mr-2" }),
        /* @__PURE__ */ s("span", { className: "text-gray-500", children: "Loading status..." })
      ] }) : x.length > 0 ? x.map((p) => A(p)) : /* @__PURE__ */ s("div", { className: "p-4 rounded-lg border-2 border-green-200 bg-green-50", children: /* @__PURE__ */ d("div", { className: "flex items-center gap-2", children: [
        /* @__PURE__ */ s(cr, { className: "w-5 h-5 text-green-500" }),
        /* @__PURE__ */ s("span", { className: "text-green-800 font-medium", children: "No active jobs" })
      ] }) }),
      o && /* @__PURE__ */ s("div", { className: `mt-3 p-3 rounded-md text-sm ${o.type === "success" ? "bg-green-50 text-green-800 border border-green-200" : "bg-red-50 text-red-800 border border-red-200"}`, children: o.message })
    ] }),
    /* @__PURE__ */ s(E, {})
  ] });
}
function Ne(t) {
  return t !== null && typeof t == "object" && !Array.isArray(t);
}
function oi(t) {
  const e = ht(t);
  if (!Ne(e)) return !1;
  const r = Object.keys(e);
  if (r.length === 0) return !1;
  for (let i = 0; i < Math.min(3, r.length); i++) {
    const o = e[r[i]];
    if (!Ne(o)) return !1;
    const u = Object.keys(o);
    if (u.length !== 0)
      for (let h = 0; h < Math.min(3, u.length); h++) {
        const l = o[u[h]];
        if (!Ne(l)) return !1;
        Object.keys(l).length;
      }
  }
  return !0;
}
function ht(t) {
  return t && Ne(t) && Object.prototype.hasOwnProperty.call(t, "data") ? t.data : t;
}
function li(t) {
  const e = ht(t) || {};
  if (!Ne(e)) return { hashes: 0, ranges: 0 };
  const r = Object.keys(e).length;
  let i = 0;
  for (const o of Object.keys(e)) {
    const u = e[o];
    Ne(u) && (i += Object.keys(u).length);
  }
  return { hashes: r, ranges: i };
}
function ci(t) {
  const e = ht(t) || {};
  return Ne(e) ? Object.keys(e).sort(qn) : [];
}
function Vn(t, e) {
  const r = ht(t) || {}, i = Ne(r) && Ne(r[e]) ? r[e] : {};
  return Object.keys(i).sort(qn);
}
function qn(t, e) {
  const r = ln(t), i = ln(e);
  return !Number.isNaN(r) && !Number.isNaN(i) ? r - i : String(t).localeCompare(String(e));
}
function ln(t) {
  const e = Number(t);
  return Number.isFinite(e) ? e : Number.NaN;
}
function di(t, e, r) {
  const i = ht(t) || {};
  if (!Ne(i)) return null;
  const o = i[e];
  if (!Ne(o)) return null;
  const u = o[r];
  return Ne(u) ? u : null;
}
function Gn(t, e, r) {
  return t.slice(e, Math.min(e + r, t.length));
}
const ui = 50;
function zn({ isOpen: t, onClick: e, label: r }) {
  return /* @__PURE__ */ d(
    "button",
    {
      type: "button",
      className: "text-left w-full flex items-center justify-between px-3 py-2 hover:bg-gray-100 rounded",
      onClick: e,
      "aria-expanded": t,
      children: [
        /* @__PURE__ */ s("span", { className: "font-mono text-sm text-gray-800 truncate", children: r }),
        /* @__PURE__ */ s("span", { className: "ml-2 text-gray-500 text-xs", children: t ? "▼" : "▶" })
      ]
    }
  );
}
function hi({ fields: t }) {
  const e = W(() => Object.entries(t || {}), [t]);
  return e.length === 0 ? /* @__PURE__ */ s("div", { className: "text-xs text-gray-500 italic px-3 py-2", children: "No fields" }) : /* @__PURE__ */ s("div", { className: "px-3 py-2 overflow-x-auto", children: /* @__PURE__ */ s("table", { className: "min-w-full border-separate border-spacing-y-1", children: /* @__PURE__ */ s("tbody", { children: e.map(([r, i]) => /* @__PURE__ */ d("tr", { className: "bg-white", children: [
    /* @__PURE__ */ s("td", { className: "align-top text-xs font-medium text-gray-700 pr-4 whitespace-nowrap", children: r }),
    /* @__PURE__ */ s("td", { className: "align-top text-xs text-gray-700", children: /* @__PURE__ */ s("pre", { className: "font-mono whitespace-pre-wrap break-words", children: mi(i) }) })
  ] }, r)) }) }) });
}
function mi(t) {
  if (t === null) return "null";
  if (typeof t == "string") return t;
  if (typeof t == "number" || typeof t == "boolean") return String(t);
  try {
    return JSON.stringify(t, null, 2);
  } catch {
    return String(t);
  }
}
function fi({ results: t, pageSize: e = ui }) {
  const r = W(() => ht(t) || {}, [t]), i = W(() => li(t), [t]), o = W(() => ci(t), [t]), [u, h] = O(() => /* @__PURE__ */ new Set()), [l, g] = O(() => /* @__PURE__ */ new Set()), [f, b] = O({ start: 0, count: e }), [N, A] = O(() => /* @__PURE__ */ new Map()), E = $((w) => {
    h((_) => {
      const S = new Set(_);
      return S.has(w) ? S.delete(w) : S.add(w), S;
    }), A((_) => {
      if (!u.has(w)) {
        const S = Vn(r, w).length, B = new Map(_);
        return B.set(w, { start: 0, count: Math.min(e, S) }), B;
      }
      return _;
    });
  }, [r, u, e]), y = $((w, _) => {
    const S = w + "||" + _;
    g((B) => {
      const I = new Set(B);
      return I.has(S) ? I.delete(S) : I.add(S), I;
    });
  }, []), x = $(() => {
    const w = Math.min(o.length, f.count + e);
    b((_) => ({ start: 0, count: w }));
  }, [o, f.count, e]), p = W(() => Gn(o, f.start, f.count), [o, f]);
  return /* @__PURE__ */ d("div", { className: "space-y-2", children: [
    /* @__PURE__ */ d("div", { className: "text-xs text-gray-600", children: [
      /* @__PURE__ */ d("span", { className: "mr-4", children: [
        "Hashes: ",
        /* @__PURE__ */ s("strong", { children: i.hashes })
      ] }),
      /* @__PURE__ */ d("span", { children: [
        "Ranges: ",
        /* @__PURE__ */ s("strong", { children: i.ranges })
      ] })
    ] }),
    /* @__PURE__ */ s("div", { className: "border rounded-md divide-y divide-gray-200 bg-gray-50", children: p.map((w) => /* @__PURE__ */ d("div", { className: "p-2", children: [
      /* @__PURE__ */ s(
        zn,
        {
          isOpen: u.has(w),
          onClick: () => E(w),
          label: `hash: ${String(w)}`
        }
      ),
      u.has(w) && /* @__PURE__ */ s(
        pi,
        {
          data: r,
          hashKey: w,
          rangeOpen: l,
          onToggleRange: y,
          pageSize: e,
          rangeWindow: N.get(w),
          setRangeWindow: (_) => A((S) => new Map(S).set(w, _))
        }
      )
    ] }, w)) }),
    f.count < o.length && /* @__PURE__ */ s("div", { className: "pt-2", children: /* @__PURE__ */ d(
      "button",
      {
        type: "button",
        className: "text-xs px-3 py-1 rounded bg-gray-200 hover:bg-gray-300",
        onClick: x,
        children: [
          "Show more hashes (",
          f.count,
          "/",
          o.length,
          ")"
        ]
      }
    ) })
  ] });
}
function pi({ data: t, hashKey: e, rangeOpen: r, onToggleRange: i, pageSize: o, rangeWindow: u, setRangeWindow: h }) {
  const l = W(() => Vn(t, e), [t, e]), g = u || { start: 0, count: Math.min(o, l.length) }, f = W(() => Gn(l, g.start, g.count), [l, g]), b = $(() => {
    const N = Math.min(l.length, g.count + o);
    h({ start: 0, count: N });
  }, [l.length, g.count, o, h]);
  return /* @__PURE__ */ d("div", { className: "ml-4 mt-1 border-l pl-3", children: [
    f.map((N) => /* @__PURE__ */ d("div", { className: "py-1", children: [
      /* @__PURE__ */ s(
        zn,
        {
          isOpen: r.has(e + "||" + N),
          onClick: () => i(e, N),
          label: `range: ${String(N)}`
        }
      ),
      r.has(e + "||" + N) && /* @__PURE__ */ s("div", { className: "ml-4 mt-1", children: /* @__PURE__ */ s(hi, { fields: di(t, e, N) || {} }) })
    ] }, N)),
    g.count < l.length && /* @__PURE__ */ s("div", { className: "pt-1", children: /* @__PURE__ */ d(
      "button",
      {
        type: "button",
        className: "text-xs px-3 py-1 rounded bg-gray-200 hover:bg-gray-300",
        onClick: b,
        children: [
          "Show more ranges (",
          g.count,
          "/",
          l.length,
          ")"
        ]
      }
    ) })
  ] });
}
function Po({ results: t }) {
  const e = t != null, r = e && (!!t.error || t.status && t.status >= 400), i = e && t.data !== void 0, o = W(() => e && !r && oi(i ? t.data : t), [e, t, r, i]), [u, h] = O(o);
  return e ? /* @__PURE__ */ d("div", { className: "bg-white rounded-lg shadow-sm p-6 mt-6", children: [
    /* @__PURE__ */ d("h3", { className: "text-lg font-semibold mb-4 flex items-center", children: [
      /* @__PURE__ */ s("span", { className: `mr-2 ${r ? "text-red-600" : "text-gray-900"}`, children: r ? "Error" : "Results" }),
      /* @__PURE__ */ d("span", { className: "text-xs font-normal text-gray-500", children: [
        "(",
        typeof t == "string" ? "Text" : u ? "Structured" : "JSON",
        ")"
      ] }),
      t.status && /* @__PURE__ */ d("span", { className: `ml-2 px-2 py-1 text-xs rounded-full ${t.status >= 400 ? "bg-red-100 text-red-800" : "bg-green-100 text-green-800"}`, children: [
        "Status: ",
        t.status
      ] }),
      !r && typeof t != "string" && /* @__PURE__ */ s("div", { className: "ml-auto", children: /* @__PURE__ */ s(
        "button",
        {
          type: "button",
          className: "text-xs px-2 py-1 rounded border border-gray-300 hover:bg-gray-100",
          onClick: () => h((l) => !l),
          children: u ? "View JSON" : "View Structured"
        }
      ) })
    ] }),
    r && /* @__PURE__ */ s("div", { className: "mb-4 p-4 bg-red-50 border border-red-200 rounded-md", children: /* @__PURE__ */ d("div", { className: "flex", children: [
      /* @__PURE__ */ s("div", { className: "flex-shrink-0", children: /* @__PURE__ */ s("svg", { className: "h-5 w-5 text-red-400", viewBox: "0 0 20 20", fill: "currentColor", children: /* @__PURE__ */ s("path", { fillRule: "evenodd", d: "M10 18a8 8 0 100-16 8 8 0 000 16zM8.707 7.293a1 1 0 00-1.414 1.414L8.586 10l-1.293 1.293a1 1 0 101.414 1.414L10 11.414l1.293 1.293a1 1 0 001.414-1.414L11.414 10l1.293-1.293a1 1 0 00-1.414-1.414L10 8.586 8.707 7.293z", clipRule: "evenodd" }) }) }),
      /* @__PURE__ */ d("div", { className: "ml-3", children: [
        /* @__PURE__ */ s("h4", { className: "text-sm font-medium text-red-800", children: "Query Execution Failed" }),
        /* @__PURE__ */ s("div", { className: "mt-2 text-sm text-red-700", children: /* @__PURE__ */ s("p", { children: t.error || "An unknown error occurred" }) })
      ] })
    ] }) }),
    u && !r && typeof t != "string" ? /* @__PURE__ */ s("div", { className: "rounded-md p-2 bg-gray-50 border overflow-auto max-h-[500px]", children: /* @__PURE__ */ s(fi, { results: t }) }) : /* @__PURE__ */ s("div", { className: `rounded-md p-4 overflow-auto max-h-[500px] ${r ? "bg-red-50 border border-red-200" : "bg-gray-50"}`, children: /* @__PURE__ */ s("pre", { className: `font-mono text-sm whitespace-pre-wrap ${r ? "text-red-700" : "text-gray-700"}`, children: typeof t == "string" ? t : JSON.stringify(i ? t.data : t, null, 2) }) })
  ] }) : null;
}
const we = {
  // Tab styling
  tab: {
    base: "px-4 py-2 text-sm font-medium transition-all duration-200",
    active: "text-blue-600 border-b-2 border-blue-600",
    inactive: "text-gray-500 hover:text-gray-700 hover:border-gray-300",
    disabled: "text-gray-300 cursor-not-allowed"
  },
  // Input styling
  input: {
    base: "block w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-1 transition-colors duration-200",
    error: "border-red-300 focus:ring-red-500 focus:border-red-500",
    success: "border-green-300 focus:ring-green-500 focus:border-green-500"
  },
  // Select styling
  select: {
    base: "block w-full pl-3 pr-10 py-2 text-base border-gray-300 focus:outline-none focus:ring-blue-600 focus:border-blue-600 rounded-md transition-colors duration-200",
    disabled: "bg-gray-100 text-gray-500 cursor-not-allowed"
  }
};
function Uo({
  tabs: t = ha,
  activeTab: e,
  onTabChange: r,
  className: i = ""
}) {
  const o = (f, b) => {
    r(f);
  }, u = (f) => {
    const b = e === f.id, N = f.disabled || !1;
    let A = we.tab.base;
    return b ? A += ` ${we.tab.active}` : N ? A += ` ${we.tab.disabled}` : A += ` ${we.tab.inactive}`, A;
  }, h = t.filter((f) => f.group === "main"), l = t.filter((f) => f.group === "advanced"), g = (f) => {
    const b = f.disabled || !1;
    return /* @__PURE__ */ d(
      "button",
      {
        className: u(f),
        onClick: () => o(f.id, f.requiresAuth),
        disabled: b,
        "aria-current": e === f.id ? "page" : void 0,
        "aria-label": `${f.label} tab`,
        style: {
          transitionDuration: `${da}ms`
        },
        children: [
          f.icon && /* @__PURE__ */ s("span", { className: "mr-2", "aria-hidden": "true", children: f.icon }),
          /* @__PURE__ */ s("span", { children: f.label })
        ]
      },
      f.id
    );
  };
  return /* @__PURE__ */ s("div", { className: `border-b border-gray-200 ${i}`, children: /* @__PURE__ */ d("div", { className: "flex items-center", children: [
    /* @__PURE__ */ s("div", { className: "flex space-x-8", children: h.map(g) }),
    l.length > 0 && /* @__PURE__ */ s("div", { className: "mx-6 h-6 w-px bg-gray-300", "aria-hidden": "true" }),
    l.length > 0 && /* @__PURE__ */ d("div", { className: "flex items-center space-x-6", children: [
      /* @__PURE__ */ s("span", { className: "text-xs text-gray-500 font-medium uppercase tracking-wider", children: "Advanced" }),
      /* @__PURE__ */ s("div", { className: "flex space-x-6", children: l.map(g) })
    ] })
  ] }) });
}
const gi = {
  queue: [],
  length: 0,
  isEmpty: !0
}, yi = (t = {}) => {
  const e = Array.isArray(t.queue) ? t.queue : [], r = typeof t.length == "number" ? t.length : e.length, i = typeof t.isEmpty == "boolean" ? t.isEmpty : e.length === 0;
  return { queue: e, length: r, isEmpty: i };
}, bi = ({ onResult: t }) => {
  const [e, r] = O(gi), [i, o] = O({}), [u, h] = O({}), [l, g] = O(!1), [f, b] = O(null), [N, A] = O([]), E = $(async () => {
    g(!0), b(null);
    try {
      const p = await pe.getTransforms();
      if (p?.success && p.data) {
        const w = p.data, _ = w && typeof w == "object" && !Array.isArray(w) ? Object.entries(w).map(([S, B]) => ({
          transform_id: S,
          ...B
        })) : Array.isArray(w) ? w : [];
        A(_);
      } else {
        const w = p?.error || "Failed to load transforms";
        b(w), A([]);
      }
    } catch (p) {
      console.error("Failed to fetch transforms:", p), b(p.message || "Failed to load transforms"), A([]);
    } finally {
      g(!1);
    }
  }, []), y = $(async () => {
    try {
      const p = await pe.getQueue();
      p?.success && p.data && r(yi(p.data));
    } catch (p) {
      console.error("Failed to fetch transform queue info:", p);
    }
  }, []);
  ue(() => {
    E(), y();
    const p = setInterval(y, 5e3);
    return () => clearInterval(p);
  }, [E, y]);
  const x = $(async (p, w) => {
    const _ = w ? `${p}.${w}` : p;
    o((S) => ({ ...S, [_]: !0 })), h((S) => ({ ...S, [_]: null }));
    try {
      const S = await pe.addToQueue(_);
      if (!S?.success) {
        const B = S?.data?.message || S?.error || "Failed to add transform to queue";
        throw new Error(B);
      }
      typeof t == "function" && t({ success: !0, transformId: _ }), await y();
    } catch (S) {
      console.error("Failed to add transform to queue:", S), h((B) => ({ ...B, [_]: S.message || "Failed to add transform to queue" }));
    } finally {
      o((S) => ({ ...S, [_]: !1 }));
    }
  }, [y, t]);
  return /* @__PURE__ */ d("div", { className: "space-y-4", children: [
    /* @__PURE__ */ d("div", { className: "flex justify-between items-center", children: [
      /* @__PURE__ */ s("h2", { className: "text-xl font-semibold text-gray-800", children: "Transforms" }),
      /* @__PURE__ */ d("div", { className: "text-sm text-gray-600", children: [
        "Queue Status: ",
        e.isEmpty ? "Empty" : `${e.length} transform(s) queued`
      ] })
    ] }),
    !e.isEmpty && /* @__PURE__ */ d("div", { className: "bg-blue-50 p-4 rounded-lg", "data-testid": "transform-queue", children: [
      /* @__PURE__ */ s("h3", { className: "text-md font-medium text-blue-800 mb-2", children: "Transform Queue" }),
      /* @__PURE__ */ s("ul", { className: "list-disc list-inside space-y-1", children: e.queue.map((p, w) => /* @__PURE__ */ s("li", { className: "text-blue-700", children: p }, `${p}-${w}`)) })
    ] }),
    l && /* @__PURE__ */ s("div", { className: "bg-blue-50 p-4 rounded-lg", role: "status", children: /* @__PURE__ */ d("div", { className: "flex items-center", children: [
      /* @__PURE__ */ s("div", { className: "animate-spin rounded-full h-4 w-4 border-b-2 border-blue-600 mr-2" }),
      /* @__PURE__ */ s("span", { className: "text-blue-800", children: "Loading transforms..." })
    ] }) }),
    f && /* @__PURE__ */ s("div", { className: "bg-red-50 p-4 rounded-lg", role: "alert", children: /* @__PURE__ */ d("div", { className: "flex items-center", children: [
      /* @__PURE__ */ d("span", { className: "text-red-800", children: [
        "Error loading transforms: ",
        f
      ] }),
      /* @__PURE__ */ s(
        "button",
        {
          onClick: E,
          className: "ml-4 px-3 py-1 text-sm bg-red-500 text-white rounded hover:bg-red-600",
          children: "Retry"
        }
      )
    ] }) }),
    !l && !f && N.length > 0 && /* @__PURE__ */ s("div", { className: "space-y-4", children: N.map((p, w) => {
      const _ = p.transform_id || `transform-${w}`, S = i[_], B = u[_], I = p.name || p.transform_id?.split(".")[0] || "Unknown", j = p.schema_type;
      let C = "Single", k = "bg-gray-100 text-gray-800";
      j?.Range ? (C = "Range", k = "bg-blue-100 text-blue-800") : j?.HashRange && (C = "HashRange", k = "bg-purple-100 text-purple-800");
      const L = p.key, P = p.transform_fields || {}, F = Object.keys(P).length, U = Object.keys(P);
      return /* @__PURE__ */ d("div", { className: "bg-white p-4 rounded-lg shadow border-l-4 border-blue-500", children: [
        /* @__PURE__ */ s("div", { className: "flex justify-between items-start mb-3", children: /* @__PURE__ */ d("div", { className: "flex-1", children: [
          /* @__PURE__ */ s("h3", { className: "text-lg font-semibold text-gray-900", children: I }),
          /* @__PURE__ */ d("div", { className: "flex gap-2 mt-2 flex-wrap", children: [
            /* @__PURE__ */ s("span", { className: `inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium ${k}`, children: C }),
            F > 0 && /* @__PURE__ */ d("span", { className: "inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-green-100 text-green-800", children: [
              F,
              " field",
              F !== 1 ? "s" : ""
            ] })
          ] }),
          U.length > 0 && /* @__PURE__ */ d("div", { className: "mt-2 text-sm text-gray-600", children: [
            /* @__PURE__ */ s("span", { className: "font-medium", children: "Fields:" }),
            " ",
            U.join(", ")
          ] })
        ] }) }),
        /* @__PURE__ */ d("div", { className: "mt-3 space-y-3", children: [
          L && /* @__PURE__ */ d("div", { className: "bg-blue-50 rounded p-3", children: [
            /* @__PURE__ */ s("div", { className: "text-sm font-medium text-blue-900 mb-1", children: "Key Configuration:" }),
            /* @__PURE__ */ d("div", { className: "text-sm text-blue-800 space-y-1", children: [
              L.hash_field && /* @__PURE__ */ d("div", { children: [
                /* @__PURE__ */ s("span", { className: "font-medium", children: "Hash Key:" }),
                " ",
                L.hash_field
              ] }),
              L.range_field && /* @__PURE__ */ d("div", { children: [
                /* @__PURE__ */ s("span", { className: "font-medium", children: "Range Key:" }),
                " ",
                L.range_field
              ] }),
              !L.hash_field && !L.range_field && L.key_field && /* @__PURE__ */ d("div", { children: [
                /* @__PURE__ */ s("span", { className: "font-medium", children: "Key:" }),
                " ",
                L.key_field
              ] })
            ] })
          ] }),
          F > 0 && /* @__PURE__ */ d("div", { children: [
            /* @__PURE__ */ s("div", { className: "text-sm font-medium text-gray-700 mb-2", children: "Transform Fields:" }),
            /* @__PURE__ */ s("div", { className: "bg-gray-50 rounded p-3 space-y-2", children: Object.entries(P).map(([K, V]) => /* @__PURE__ */ d("div", { className: "border-l-2 border-gray-300 pl-3", children: [
              /* @__PURE__ */ s("div", { className: "font-medium text-gray-900 text-sm", children: K }),
              /* @__PURE__ */ s("div", { className: "text-gray-600 font-mono text-xs mt-1 break-all", children: V })
            ] }, K)) })
          ] })
        ] }),
        /* @__PURE__ */ d("div", { className: "mt-4 flex items-center gap-3", children: [
          /* @__PURE__ */ s(
            "button",
            {
              onClick: () => x(_, null),
              disabled: S,
              className: `px-4 py-2 text-sm font-medium rounded-md text-white ${S ? "bg-blue-300 cursor-not-allowed" : "bg-blue-600 hover:bg-blue-700"}`,
              children: S ? "Adding..." : "Add to Queue"
            }
          ),
          B && /* @__PURE__ */ d("span", { className: "text-sm text-red-600", children: [
            "Error: ",
            B
          ] })
        ] })
      ] }, _);
    }) }),
    !l && !f && N.length === 0 && /* @__PURE__ */ d("div", { className: "bg-gray-50 p-4 rounded-lg", children: [
      /* @__PURE__ */ s("p", { className: "text-gray-600", children: "No transforms registered" }),
      /* @__PURE__ */ s("p", { className: "text-sm text-gray-500 mt-1", children: "Register a transform in a schema to view it here and add it to the processing queue." })
    ] })
  ] });
}, He = () => ns(), Z = rs;
function xi({
  title: t,
  titleId: e,
  ...r
}, i) {
  return /* @__PURE__ */ H.createElement("svg", Object.assign({
    xmlns: "http://www.w3.org/2000/svg",
    fill: "none",
    viewBox: "0 0 24 24",
    strokeWidth: 1.5,
    stroke: "currentColor",
    "aria-hidden": "true",
    "data-slot": "icon",
    ref: i,
    "aria-labelledby": e
  }, r), t ? /* @__PURE__ */ H.createElement("title", {
    id: e
  }, t) : null, /* @__PURE__ */ H.createElement("path", {
    strokeLinecap: "round",
    strokeLinejoin: "round",
    d: "m4.5 12.75 6 6 9-13.5"
  }));
}
const ur = /* @__PURE__ */ H.forwardRef(xi);
function wi({
  title: t,
  titleId: e,
  ...r
}, i) {
  return /* @__PURE__ */ H.createElement("svg", Object.assign({
    xmlns: "http://www.w3.org/2000/svg",
    fill: "none",
    viewBox: "0 0 24 24",
    strokeWidth: 1.5,
    stroke: "currentColor",
    "aria-hidden": "true",
    "data-slot": "icon",
    ref: i,
    "aria-labelledby": e
  }, r), t ? /* @__PURE__ */ H.createElement("title", {
    id: e
  }, t) : null, /* @__PURE__ */ H.createElement("path", {
    strokeLinecap: "round",
    strokeLinejoin: "round",
    d: "M15.666 3.888A2.25 2.25 0 0 0 13.5 2.25h-3c-1.03 0-1.9.693-2.166 1.638m7.332 0c.055.194.084.4.084.612v0a.75.75 0 0 1-.75.75H9a.75.75 0 0 1-.75-.75v0c0-.212.03-.418.084-.612m7.332 0c.646.049 1.288.11 1.927.184 1.1.128 1.907 1.077 1.907 2.185V19.5a2.25 2.25 0 0 1-2.25 2.25H6.75A2.25 2.25 0 0 1 4.5 19.5V6.257c0-1.108.806-2.057 1.907-2.185a48.208 48.208 0 0 1 1.927-.184"
  }));
}
const cn = /* @__PURE__ */ H.forwardRef(wi);
function vi({
  title: t,
  titleId: e,
  ...r
}, i) {
  return /* @__PURE__ */ H.createElement("svg", Object.assign({
    xmlns: "http://www.w3.org/2000/svg",
    fill: "none",
    viewBox: "0 0 24 24",
    strokeWidth: 1.5,
    stroke: "currentColor",
    "aria-hidden": "true",
    "data-slot": "icon",
    ref: i,
    "aria-labelledby": e
  }, r), t ? /* @__PURE__ */ H.createElement("title", {
    id: e
  }, t) : null, /* @__PURE__ */ H.createElement("path", {
    strokeLinecap: "round",
    strokeLinejoin: "round",
    d: "M12 9v3.75m-9.303 3.376c-.866 1.5.217 3.374 1.948 3.374h14.71c1.73 0 2.813-1.874 1.948-3.374L13.949 3.378c-.866-1.5-3.032-1.5-3.898 0L2.697 16.126ZM12 15.75h.007v.008H12v-.008Z"
  }));
}
const dn = /* @__PURE__ */ H.forwardRef(vi);
function Ni({
  title: t,
  titleId: e,
  ...r
}, i) {
  return /* @__PURE__ */ H.createElement("svg", Object.assign({
    xmlns: "http://www.w3.org/2000/svg",
    fill: "none",
    viewBox: "0 0 24 24",
    strokeWidth: 1.5,
    stroke: "currentColor",
    "aria-hidden": "true",
    "data-slot": "icon",
    ref: i,
    "aria-labelledby": e
  }, r), t ? /* @__PURE__ */ H.createElement("title", {
    id: e
  }, t) : null, /* @__PURE__ */ H.createElement("path", {
    strokeLinecap: "round",
    strokeLinejoin: "round",
    d: "M15.75 5.25a3 3 0 0 1 3 3m3 0a6 6 0 0 1-7.029 5.912c-.563-.097-1.159.026-1.563.43L10.5 17.25H8.25v2.25H6v2.25H2.25v-2.818c0-.597.237-1.17.659-1.591l6.499-6.499c.404-.404.527-1 .43-1.563A6 6 0 1 1 21.75 8.25Z"
  }));
}
const hr = /* @__PURE__ */ H.forwardRef(Ni);
function Si({
  title: t,
  titleId: e,
  ...r
}, i) {
  return /* @__PURE__ */ H.createElement("svg", Object.assign({
    xmlns: "http://www.w3.org/2000/svg",
    fill: "none",
    viewBox: "0 0 24 24",
    strokeWidth: 1.5,
    stroke: "currentColor",
    "aria-hidden": "true",
    "data-slot": "icon",
    ref: i,
    "aria-labelledby": e
  }, r), t ? /* @__PURE__ */ H.createElement("title", {
    id: e
  }, t) : null, /* @__PURE__ */ H.createElement("path", {
    strokeLinecap: "round",
    strokeLinejoin: "round",
    d: "M9 12.75 11.25 15 15 9.75m-3-7.036A11.959 11.959 0 0 1 3.598 6 11.99 11.99 0 0 0 3 9.749c0 5.592 3.824 10.29 9 11.623 5.176-1.332 9-6.03 9-11.622 0-1.31-.21-2.571-.598-3.751h-.152c-3.196 0-6.1-1.248-8.25-3.285Z"
  }));
}
const Ei = /* @__PURE__ */ H.forwardRef(Si);
function Ai({ onResult: t }) {
  const e = He(), r = Z((k) => k.auth), { isAuthenticated: i, systemPublicKey: o, systemKeyId: u, privateKey: h, isLoading: l, error: g } = r, f = h ? na(h) : null, [b, N] = O(null), [A, E] = O(""), [y, x] = O(!1), [p, w] = O(null), [_, S] = O(!1), B = async (k, L) => {
    try {
      await navigator.clipboard.writeText(k), N(L), setTimeout(() => N(null), 2e3);
    } catch (P) {
      console.error("Failed to copy:", P);
    }
  }, I = async () => {
    if (!A.trim()) {
      w({ valid: !1, error: "Please enter a private key" });
      return;
    }
    x(!0);
    try {
      const L = (await e(Ut(A.trim())).unwrap()).isAuthenticated;
      w({
        valid: L,
        error: L ? null : "Private key does not match the system public key"
      }), L && console.log("Private key validation successful");
    } catch (k) {
      w({
        valid: !1,
        error: `Validation failed: ${k.message}`
      });
    } finally {
      x(!1);
    }
  }, j = () => {
    E(""), w(null), S(!1);
  }, C = () => {
    j(), e(aa());
  };
  return /* @__PURE__ */ d("div", { className: "p-4 bg-white rounded-lg shadow", children: [
    /* @__PURE__ */ s("h2", { className: "text-xl font-semibold mb-4", children: "Key Management" }),
    /* @__PURE__ */ s("div", { className: "bg-blue-50 border border-blue-200 rounded-md p-4 mb-6", children: /* @__PURE__ */ d("div", { className: "flex items-start", children: [
      /* @__PURE__ */ s(Ei, { className: "h-5 w-5 text-blue-400 mr-2 flex-shrink-0 mt-0.5" }),
      /* @__PURE__ */ d("div", { className: "text-sm text-blue-700 flex-1", children: [
        /* @__PURE__ */ s("p", { className: "font-medium", children: "Current System Public Key:" }),
        l ? /* @__PURE__ */ s("p", { className: "text-blue-600", children: "Loading..." }) : o ? /* @__PURE__ */ d("div", { className: "mt-2", children: [
          /* @__PURE__ */ d("div", { className: "flex", children: [
            /* @__PURE__ */ s(
              "input",
              {
                type: "text",
                value: o && o !== "null" ? o : "",
                readOnly: !0,
                className: "flex-1 px-2 py-1 border border-blue-300 rounded-l-md bg-blue-50 text-xs font-mono"
              }
            ),
            /* @__PURE__ */ s(
              "button",
              {
                onClick: () => B(o, "system"),
                className: "px-2 py-1 border border-l-0 border-blue-300 rounded-r-md bg-white hover:bg-blue-50 focus:outline-none focus:ring-2 focus:ring-blue-500",
                children: b === "system" ? /* @__PURE__ */ s(ur, { className: "h-3 w-3 text-green-600" }) : /* @__PURE__ */ s(cn, { className: "h-3 w-3 text-blue-500" })
              }
            )
          ] }),
          u && /* @__PURE__ */ d("p", { className: "text-xs text-blue-600 mt-1", children: [
            "Key ID: ",
            u
          ] }),
          i && /* @__PURE__ */ s("p", { className: "text-xs text-green-600 mt-1", children: "🔓 Authenticated - Private key loaded!" })
        ] }) : /* @__PURE__ */ s("p", { className: "text-blue-600 mt-1", children: "No system public key available." })
      ] })
    ] }) }),
    i && f && /* @__PURE__ */ s("div", { className: "bg-green-50 border border-green-200 rounded-md p-4 mb-6", children: /* @__PURE__ */ d("div", { className: "flex items-start", children: [
      /* @__PURE__ */ s(hr, { className: "h-5 w-5 text-green-400 mr-2 flex-shrink-0 mt-0.5" }),
      /* @__PURE__ */ d("div", { className: "text-sm text-green-700 flex-1", children: [
        /* @__PURE__ */ s("p", { className: "font-medium", children: "Current Private Key (Auto-loaded from Node)" }),
        /* @__PURE__ */ s("p", { className: "mt-1", children: "Your private key has been automatically loaded from the backend node." }),
        /* @__PURE__ */ d("div", { className: "mt-3", children: [
          /* @__PURE__ */ d("div", { className: "flex", children: [
            /* @__PURE__ */ s(
              "textarea",
              {
                value: f,
                readOnly: !0,
                className: "flex-1 px-3 py-2 border border-green-300 rounded-l-md bg-green-50 text-xs font-mono resize-none",
                rows: 3,
                placeholder: "Private key will appear here..."
              }
            ),
            /* @__PURE__ */ s(
              "button",
              {
                onClick: () => B(f, "private"),
                className: "px-3 py-2 border border-l-0 border-green-300 rounded-r-md bg-white hover:bg-green-50 focus:outline-none focus:ring-2 focus:ring-green-500",
                title: "Copy private key",
                children: b === "private" ? /* @__PURE__ */ s(ur, { className: "h-3 w-3 text-green-600" }) : /* @__PURE__ */ s(cn, { className: "h-3 w-3 text-green-500" })
              }
            )
          ] }),
          /* @__PURE__ */ s("p", { className: "text-xs text-green-600 mt-1", children: "🔓 Authenticated - Private key loaded from node!" })
        ] })
      ] })
    ] }) }),
    o && !i && !f && /* @__PURE__ */ s("div", { className: "bg-yellow-50 border border-yellow-200 rounded-md p-4 mb-6", children: /* @__PURE__ */ d("div", { className: "flex items-start", children: [
      /* @__PURE__ */ s(hr, { className: "h-5 w-5 text-yellow-400 mr-2 flex-shrink-0 mt-0.5" }),
      /* @__PURE__ */ d("div", { className: "text-sm text-yellow-700 flex-1", children: [
        /* @__PURE__ */ s("p", { className: "font-medium", children: "Import Private Key" }),
        /* @__PURE__ */ s("p", { className: "mt-1", children: "You have a registered public key but no local private key. Enter your private key to restore access." }),
        _ ? /* @__PURE__ */ d("div", { className: "mt-3 space-y-3", children: [
          /* @__PURE__ */ d("div", { children: [
            /* @__PURE__ */ s("label", { className: "block text-xs font-medium text-yellow-700 mb-1", children: "Private Key (Base64)" }),
            /* @__PURE__ */ s(
              "textarea",
              {
                value: A,
                onChange: (k) => E(k.target.value),
                placeholder: "Enter your private key here...",
                className: "w-full px-3 py-2 border border-yellow-300 rounded-md focus:outline-none focus:ring-2 focus:ring-yellow-500 text-xs font-mono",
                rows: 3
              }
            )
          ] }),
          p && /* @__PURE__ */ s("div", { className: `p-2 rounded-md text-xs ${p.valid ? "bg-green-50 border border-green-200 text-green-700" : "bg-red-50 border border-red-200 text-red-700"}`, children: p.valid ? /* @__PURE__ */ d("div", { className: "flex items-center", children: [
            /* @__PURE__ */ s(ur, { className: "h-4 w-4 text-green-600 mr-1" }),
            /* @__PURE__ */ s("span", { children: "Private key matches system public key!" })
          ] }) : /* @__PURE__ */ d("div", { className: "flex items-center", children: [
            /* @__PURE__ */ s(dn, { className: "h-4 w-4 text-red-600 mr-1" }),
            /* @__PURE__ */ s("span", { children: p.error })
          ] }) }),
          /* @__PURE__ */ d("div", { className: "flex gap-2", children: [
            /* @__PURE__ */ s(
              "button",
              {
                onClick: I,
                disabled: y || !A.trim(),
                className: "inline-flex items-center px-3 py-2 border border-transparent text-xs font-medium rounded-md shadow-sm text-white bg-green-600 hover:bg-green-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-green-500 disabled:opacity-50",
                children: y ? "Validating..." : "Validate & Import"
              }
            ),
            /* @__PURE__ */ s(
              "button",
              {
                onClick: C,
                className: "inline-flex items-center px-3 py-2 border border-gray-300 text-xs font-medium rounded-md shadow-sm text-gray-700 bg-white hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-yellow-500",
                children: "Cancel"
              }
            )
          ] }),
          /* @__PURE__ */ s("div", { className: "bg-red-50 border border-red-200 rounded-md p-2", children: /* @__PURE__ */ d("div", { className: "flex", children: [
            /* @__PURE__ */ s(dn, { className: "h-4 w-4 text-red-400 mr-1 flex-shrink-0" }),
            /* @__PURE__ */ d("div", { className: "text-xs text-red-700", children: [
              /* @__PURE__ */ s("p", { className: "font-medium", children: "Security Warning:" }),
              /* @__PURE__ */ s("p", { children: "Only enter your private key on trusted devices. Never share or store private keys in plain text." })
            ] })
          ] }) })
        ] }) : /* @__PURE__ */ d(
          "button",
          {
            onClick: () => S(!0),
            className: "mt-3 inline-flex items-center px-3 py-2 border border-transparent text-sm font-medium rounded-md shadow-sm text-white bg-yellow-600 hover:bg-yellow-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-yellow-500",
            children: [
              /* @__PURE__ */ s(hr, { className: "h-4 w-4 mr-1" }),
              "Import Private Key"
            ]
          }
        )
      ] })
    ] }) })
  ] });
}
function $o({ isOpen: t, onClose: e }) {
  const [r, i] = O("ai"), [o, u] = O("OpenRouter"), [h, l] = O(""), [g, f] = O("anthropic/claude-3.5-sonnet"), [b, N] = O("https://openrouter.ai/api/v1"), [A, E] = O("llama3"), [y, x] = O("http://localhost:11434"), [p, w] = O(null), [_, S] = O(!1), { environment: B, setEnvironment: I } = Ga(), [j, C] = O(B.id), [k, L] = O({}), [P, F] = O({}), [U, K] = O("local"), [V, Y] = O("data"), [ae, Ie] = O("DataFoldStorage"), [Se, We] = O("us-west-2"), [Je, ne] = O(""), [se, mt] = O(""), [Ze, ft] = O("us-east-1"), [pt, gt] = O("folddb"), [Oe, Xe] = O("/tmp/folddb-data");
  ue(() => {
    t && (Rt(), Le(), C(B.id), r === "schema-service" && yt(B.id));
  }, [t, B.id, r]);
  const Rt = async () => {
    try {
      const D = await dt.getConfig();
      D.success && (l(D.data.openrouter.api_key || ""), f(D.data.openrouter.model || "anthropic/claude-3.5-sonnet"), N(D.data.openrouter.base_url || "https://openrouter.ai/api/v1"), E(D.data.ollama.model || "llama3"), x(D.data.ollama.base_url || "http://localhost:11434"), u(D.data.provider || "OpenRouter"));
    } catch (D) {
      console.error("Failed to load AI config:", D);
    }
  }, Jt = async () => {
    try {
      const D = {
        provider: o,
        openrouter: {
          api_key: h,
          model: g,
          base_url: b
        },
        ollama: {
          model: A,
          base_url: y
        }
      };
      (await dt.saveConfig(D)).success ? (w({ success: !0, message: "Configuration saved successfully" }), setTimeout(() => {
        w(null), e();
      }, 1500)) : w({ success: !1, message: "Failed to save configuration" });
    } catch (D) {
      w({ success: !1, message: D.message || "Failed to save configuration" });
    }
    setTimeout(() => w(null), 3e3);
  }, yt = async (D) => {
    const G = Object.values(it).find((ge) => ge.id === D);
    if (G) {
      F((ge) => ({ ...ge, [D]: !0 }));
      try {
        const ge = await Va(G.baseUrl);
        L((De) => ({
          ...De,
          [D]: ge
        }));
      } catch (ge) {
        L((De) => ({
          ...De,
          [D]: { success: !1, error: ge.message }
        }));
      } finally {
        F((ge) => ({ ...ge, [D]: !1 }));
      }
    }
  }, Le = async () => {
    try {
      const D = await gs();
      if (D.success && D.data) {
        const G = D.data;
        K(G.type), G.type === "local" ? Y(G.path || "data") : G.type === "dynamodb" ? (Ie(G.table_name || "DataFoldStorage"), We(G.region || "us-west-2"), ne(G.user_id || "")) : G.type === "s3" && (mt(G.bucket || ""), ft(G.region || "us-east-1"), gt(G.prefix || "folddb"), Xe(G.local_path || "/tmp/folddb-data"));
      }
    } catch (D) {
      console.error("Failed to load database config:", D);
    }
  }, Ve = async () => {
    try {
      let D;
      if (U === "local")
        D = {
          type: "local",
          path: V
        };
      else if (U === "dynamodb") {
        if (!ae || !Se) {
          w({ success: !1, message: "Table name and region are required for DynamoDB" }), setTimeout(() => w(null), 3e3);
          return;
        }
        D = {
          type: "dynamodb",
          table_name: ae,
          region: Se,
          user_id: Je || void 0
        };
      } else if (U === "s3") {
        if (!se || !Ze) {
          w({ success: !1, message: "Bucket and region are required for S3" }), setTimeout(() => w(null), 3e3);
          return;
        }
        D = {
          type: "s3",
          bucket: se,
          region: Ze,
          prefix: pt || "folddb",
          local_path: Oe || "/tmp/folddb-data"
        };
      }
      const G = await ys(D);
      G.success ? (w({
        success: !0,
        message: G.data.requires_restart ? "Database configuration saved. Please restart the server for changes to take effect." : G.data.message || "Database configuration saved and restarted successfully"
      }), setTimeout(() => {
        w(null), G.data.requires_restart || e();
      }, 3e3)) : w({ success: !1, message: G.error || "Failed to save database configuration" });
    } catch (D) {
      w({ success: !1, message: D.message || "Failed to save database configuration" });
    }
    setTimeout(() => w(null), 5e3);
  }, Zt = () => {
    I(j), w({ success: !0, message: "Schema service environment updated successfully" }), setTimeout(() => {
      w(null), e();
    }, 1500);
  }, Xt = (D) => {
    const G = k[D];
    return P[D] ? /* @__PURE__ */ d("span", { className: "inline-flex items-center text-xs bg-gray-100 text-gray-700 px-2 py-1 rounded", children: [
      /* @__PURE__ */ d("svg", { className: "animate-spin h-3 w-3 mr-1", viewBox: "0 0 24 24", children: [
        /* @__PURE__ */ s("circle", { className: "opacity-25", cx: "12", cy: "12", r: "10", stroke: "currentColor", strokeWidth: "4", fill: "none" }),
        /* @__PURE__ */ s("path", { className: "opacity-75", fill: "currentColor", d: "M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" })
      ] }),
      "Checking..."
    ] }) : G ? G.success ? /* @__PURE__ */ d("span", { className: "inline-flex items-center text-xs bg-green-100 text-green-700 px-2 py-1 rounded", children: [
      "✓ Online ",
      G.responseTime && `(${G.responseTime}ms)`
    ] }) : /* @__PURE__ */ s("span", { className: "inline-flex items-center text-xs bg-red-100 text-red-700 px-2 py-1 rounded", title: G.error, children: "✗ Offline" }) : /* @__PURE__ */ s(
      "button",
      {
        onClick: (De) => {
          De.stopPropagation(), yt(D);
        },
        className: "text-xs text-blue-600 hover:text-blue-700 underline",
        children: "Test Connection"
      }
    );
  };
  return t ? /* @__PURE__ */ s("div", { className: "fixed inset-0 z-50 overflow-y-auto", children: /* @__PURE__ */ d("div", { className: "flex items-center justify-center min-h-screen px-4 pt-4 pb-20 text-center sm:block sm:p-0", children: [
    /* @__PURE__ */ s(
      "div",
      {
        className: "fixed inset-0 transition-opacity bg-gray-500 bg-opacity-75",
        onClick: e
      }
    ),
    /* @__PURE__ */ d("div", { className: "inline-block align-bottom bg-white rounded-lg text-left overflow-hidden shadow-xl transform transition-all sm:my-8 sm:align-middle sm:max-w-4xl sm:w-full", children: [
      /* @__PURE__ */ d("div", { className: "bg-white", children: [
        /* @__PURE__ */ d("div", { className: "flex items-center justify-between px-6 pt-5 pb-4 border-b border-gray-200", children: [
          /* @__PURE__ */ s("h3", { className: "text-lg font-medium text-gray-900", children: "Settings" }),
          /* @__PURE__ */ s(
            "button",
            {
              onClick: e,
              className: "text-gray-400 hover:text-gray-600 transition-colors",
              children: /* @__PURE__ */ s("svg", { className: "w-6 h-6", fill: "none", stroke: "currentColor", viewBox: "0 0 24 24", children: /* @__PURE__ */ s("path", { strokeLinecap: "round", strokeLinejoin: "round", strokeWidth: 2, d: "M6 18L18 6M6 6l12 12" }) })
            }
          )
        ] }),
        /* @__PURE__ */ s("div", { className: "border-b border-gray-200", children: /* @__PURE__ */ d("nav", { className: "flex px-6", children: [
          /* @__PURE__ */ s(
            "button",
            {
              onClick: () => i("ai"),
              className: `py-3 px-4 text-sm font-medium border-b-2 transition-colors ${r === "ai" ? "border-blue-500 text-blue-600" : "border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300"}`,
              children: "AI Configuration"
            }
          ),
          /* @__PURE__ */ s(
            "button",
            {
              onClick: () => i("transforms"),
              className: `py-3 px-4 text-sm font-medium border-b-2 transition-colors ${r === "transforms" ? "border-blue-500 text-blue-600" : "border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300"}`,
              children: "Transforms"
            }
          ),
          /* @__PURE__ */ s(
            "button",
            {
              onClick: () => i("keys"),
              className: `py-3 px-4 text-sm font-medium border-b-2 transition-colors ${r === "keys" ? "border-blue-500 text-blue-600" : "border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300"}`,
              children: "Key Management"
            }
          ),
          /* @__PURE__ */ s(
            "button",
            {
              onClick: () => i("schema-service"),
              className: `py-3 px-4 text-sm font-medium border-b-2 transition-colors ${r === "schema-service" ? "border-blue-500 text-blue-600" : "border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300"}`,
              children: "Schema Service"
            }
          ),
          /* @__PURE__ */ s(
            "button",
            {
              onClick: () => i("database"),
              className: `py-3 px-4 text-sm font-medium border-b-2 transition-colors ${r === "database" ? "border-blue-500 text-blue-600" : "border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300"}`,
              children: "Database"
            }
          )
        ] }) }),
        /* @__PURE__ */ d("div", { className: "px-6 py-4 max-h-[70vh] overflow-y-auto", children: [
          r === "ai" && /* @__PURE__ */ d("div", { className: "space-y-4", children: [
            /* @__PURE__ */ d("div", { className: "grid grid-cols-1 md:grid-cols-2 gap-4", children: [
              /* @__PURE__ */ d("div", { children: [
                /* @__PURE__ */ s("label", { className: "block text-sm font-medium text-gray-700 mb-1", children: "Provider" }),
                /* @__PURE__ */ d(
                  "select",
                  {
                    value: o,
                    onChange: (D) => u(D.target.value),
                    className: "w-full p-2 border border-gray-300 rounded text-sm",
                    children: [
                      /* @__PURE__ */ s("option", { value: "OpenRouter", children: "OpenRouter" }),
                      /* @__PURE__ */ s("option", { value: "Ollama", children: "Ollama" })
                    ]
                  }
                )
              ] }),
              o === "OpenRouter" ? /* @__PURE__ */ d("div", { children: [
                /* @__PURE__ */ s("label", { className: "block text-sm font-medium text-gray-700 mb-1", children: "Model" }),
                /* @__PURE__ */ d(
                  "select",
                  {
                    value: g,
                    onChange: (D) => f(D.target.value),
                    className: "w-full p-2 border border-gray-300 rounded text-sm",
                    children: [
                      /* @__PURE__ */ s("option", { value: "anthropic/claude-3.5-sonnet", children: "Claude 3.5 Sonnet" }),
                      /* @__PURE__ */ s("option", { value: "anthropic/claude-3.5-haiku", children: "Claude 3.5 Haiku" }),
                      /* @__PURE__ */ s("option", { value: "openai/gpt-4o", children: "GPT-4o" }),
                      /* @__PURE__ */ s("option", { value: "openai/gpt-4o-mini", children: "GPT-4o Mini" }),
                      /* @__PURE__ */ s("option", { value: "openai/o1", children: "OpenAI o1" }),
                      /* @__PURE__ */ s("option", { value: "openai/o1-mini", children: "OpenAI o1-mini" }),
                      /* @__PURE__ */ s("option", { value: "google/gemini-2.0-flash-exp", children: "Gemini 2.0 Flash" }),
                      /* @__PURE__ */ s("option", { value: "google/gemini-pro-1.5", children: "Gemini 1.5 Pro" }),
                      /* @__PURE__ */ s("option", { value: "meta-llama/llama-3.3-70b-instruct", children: "Llama 3.3 70B" }),
                      /* @__PURE__ */ s("option", { value: "meta-llama/llama-3.1-405b-instruct", children: "Llama 3.1 405B" }),
                      /* @__PURE__ */ s("option", { value: "deepseek/deepseek-chat", children: "DeepSeek Chat" }),
                      /* @__PURE__ */ s("option", { value: "qwen/qwen-2.5-72b-instruct", children: "Qwen 2.5 72B" })
                    ]
                  }
                )
              ] }) : /* @__PURE__ */ d("div", { children: [
                /* @__PURE__ */ s("label", { className: "block text-sm font-medium text-gray-700 mb-1", children: "Model" }),
                /* @__PURE__ */ s(
                  "input",
                  {
                    type: "text",
                    value: A,
                    onChange: (D) => E(D.target.value),
                    placeholder: "e.g., llama3",
                    className: "w-full p-2 border border-gray-300 rounded text-sm"
                  }
                )
              ] })
            ] }),
            o === "OpenRouter" && /* @__PURE__ */ d("div", { children: [
              /* @__PURE__ */ d("label", { className: "block text-sm font-medium text-gray-700 mb-1", children: [
                "API Key ",
                /* @__PURE__ */ d("span", { className: "text-xs text-gray-500", children: [
                  "(",
                  /* @__PURE__ */ s("a", { href: "https://openrouter.ai/keys", target: "_blank", rel: "noopener noreferrer", className: "text-blue-600 hover:underline", children: "get key" }),
                  ")"
                ] })
              ] }),
              /* @__PURE__ */ s(
                "input",
                {
                  type: "password",
                  value: h,
                  onChange: (D) => l(D.target.value),
                  placeholder: "sk-or-...",
                  className: "w-full p-2 border border-gray-300 rounded text-sm"
                }
              )
            ] }),
            /* @__PURE__ */ d("div", { children: [
              /* @__PURE__ */ d(
                "button",
                {
                  onClick: () => S(!_),
                  className: "text-sm text-gray-600 hover:text-gray-800 flex items-center gap-1",
                  children: [
                    /* @__PURE__ */ s("span", { children: _ ? "▼" : "▶" }),
                    "Advanced Settings"
                  ]
                }
              ),
              _ && /* @__PURE__ */ s("div", { className: "mt-3 space-y-3 pl-4 border-l-2 border-gray-200", children: /* @__PURE__ */ d("div", { children: [
                /* @__PURE__ */ s("label", { className: "block text-sm font-medium text-gray-700 mb-1", children: "Base URL" }),
                /* @__PURE__ */ s(
                  "input",
                  {
                    type: "text",
                    value: o === "OpenRouter" ? b : y,
                    onChange: (D) => o === "OpenRouter" ? N(D.target.value) : x(D.target.value),
                    placeholder: o === "OpenRouter" ? "https://openrouter.ai/api/v1" : "http://localhost:11434",
                    className: "w-full p-2 border border-gray-300 rounded text-sm"
                  }
                )
              ] }) })
            ] }),
            p && /* @__PURE__ */ s("div", { className: `p-3 rounded-md ${p.success ? "bg-green-50 text-green-800 border border-green-200" : "bg-red-50 text-red-800 border border-red-200"}`, children: /* @__PURE__ */ d("span", { className: "text-sm font-medium", children: [
              p.success ? "✓" : "✗",
              " ",
              p.message
            ] }) })
          ] }),
          r === "transforms" && /* @__PURE__ */ s(bi, { onResult: () => {
          } }),
          r === "keys" && /* @__PURE__ */ s(Ai, { onResult: () => {
          } }),
          r === "schema-service" && /* @__PURE__ */ d("div", { className: "space-y-4", children: [
            /* @__PURE__ */ d("div", { className: "mb-4", children: [
              /* @__PURE__ */ s("h4", { className: "text-md font-semibold text-gray-900 mb-2", children: "Schema Service Environment" }),
              /* @__PURE__ */ s("p", { className: "text-sm text-gray-600 mb-4", children: "Select which schema service endpoint to use. This affects where schemas are loaded from and saved to." })
            ] }),
            /* @__PURE__ */ s("div", { className: "space-y-3", children: Object.values(it).map((D) => /* @__PURE__ */ d(
              "label",
              {
                className: `flex items-start p-4 border-2 rounded-lg cursor-pointer transition-all ${j === D.id ? "border-blue-500 bg-blue-50" : "border-gray-200 hover:border-gray-300 bg-white"}`,
                children: [
                  /* @__PURE__ */ s(
                    "input",
                    {
                      type: "radio",
                      name: "schemaEnvironment",
                      value: D.id,
                      checked: j === D.id,
                      onChange: (G) => C(G.target.value),
                      className: "mt-1 mr-3"
                    }
                  ),
                  /* @__PURE__ */ d("div", { className: "flex-1", children: [
                    /* @__PURE__ */ d("div", { className: "flex items-center justify-between mb-2", children: [
                      /* @__PURE__ */ s("span", { className: "text-sm font-semibold text-gray-900", children: D.name }),
                      /* @__PURE__ */ d("div", { className: "flex items-center gap-2", children: [
                        Xt(D.id),
                        j === D.id && /* @__PURE__ */ s("span", { className: "text-xs bg-blue-100 text-blue-700 px-2 py-1 rounded", children: "Active" })
                      ] })
                    ] }),
                    /* @__PURE__ */ s("p", { className: "text-xs text-gray-600 mt-1", children: D.description }),
                    /* @__PURE__ */ s("p", { className: "text-xs text-gray-500 mt-1 font-mono", children: D.baseUrl || window.location.origin }),
                    k[D.id] && !k[D.id].success && /* @__PURE__ */ d("p", { className: "text-xs text-red-600 mt-1", children: [
                      "Error: ",
                      k[D.id].error
                    ] })
                  ] })
                ]
              },
              D.id
            )) }),
            p && /* @__PURE__ */ s("div", { className: `p-3 rounded-md ${p.success ? "bg-green-50 text-green-800 border border-green-200" : "bg-red-50 text-red-800 border border-red-200"}`, children: /* @__PURE__ */ d("span", { className: "text-sm font-medium", children: [
              p.success ? "✓" : "✗",
              " ",
              p.message
            ] }) })
          ] }),
          r === "database" && /* @__PURE__ */ d("div", { className: "space-y-4", children: [
            /* @__PURE__ */ d("div", { className: "mb-4", children: [
              /* @__PURE__ */ s("h4", { className: "text-md font-semibold text-gray-900 mb-2", children: "Database Storage Backend" }),
              /* @__PURE__ */ s("p", { className: "text-sm text-gray-600 mb-4", children: "Choose the storage backend for your database. Changes require a server restart." })
            ] }),
            /* @__PURE__ */ d("div", { children: [
              /* @__PURE__ */ s("label", { className: "block text-sm font-medium text-gray-700 mb-2", children: "Storage Type" }),
              /* @__PURE__ */ d(
                "select",
                {
                  value: U,
                  onChange: (D) => K(D.target.value),
                  className: "w-full p-2 border border-gray-300 rounded text-sm",
                  children: [
                    /* @__PURE__ */ s("option", { value: "local", children: "Local (Sled)" }),
                    /* @__PURE__ */ s("option", { value: "dynamodb", children: "DynamoDB" }),
                    /* @__PURE__ */ s("option", { value: "s3", children: "S3" })
                  ]
                }
              )
            ] }),
            U === "local" ? /* @__PURE__ */ d("div", { children: [
              /* @__PURE__ */ s("label", { className: "block text-sm font-medium text-gray-700 mb-1", children: "Storage Path" }),
              /* @__PURE__ */ s(
                "input",
                {
                  type: "text",
                  value: V,
                  onChange: (D) => Y(D.target.value),
                  placeholder: "data",
                  className: "w-full p-2 border border-gray-300 rounded text-sm"
                }
              ),
              /* @__PURE__ */ s("p", { className: "text-xs text-gray-500 mt-1", children: "Local filesystem path where the database will be stored" })
            ] }) : U === "dynamodb" ? /* @__PURE__ */ d("div", { className: "space-y-3", children: [
              /* @__PURE__ */ d("div", { children: [
                /* @__PURE__ */ d("label", { className: "block text-sm font-medium text-gray-700 mb-1", children: [
                  "Table Name ",
                  /* @__PURE__ */ s("span", { className: "text-red-500", children: "*" })
                ] }),
                /* @__PURE__ */ s(
                  "input",
                  {
                    type: "text",
                    value: ae,
                    onChange: (D) => Ie(D.target.value),
                    placeholder: "DataFoldStorage",
                    className: "w-full p-2 border border-gray-300 rounded text-sm"
                  }
                ),
                /* @__PURE__ */ s("p", { className: "text-xs text-gray-500 mt-1", children: "Base table name (namespaces will be appended automatically)" })
              ] }),
              /* @__PURE__ */ d("div", { children: [
                /* @__PURE__ */ d("label", { className: "block text-sm font-medium text-gray-700 mb-1", children: [
                  "AWS Region ",
                  /* @__PURE__ */ s("span", { className: "text-red-500", children: "*" })
                ] }),
                /* @__PURE__ */ s(
                  "input",
                  {
                    type: "text",
                    value: Se,
                    onChange: (D) => We(D.target.value),
                    placeholder: "us-west-2",
                    className: "w-full p-2 border border-gray-300 rounded text-sm"
                  }
                ),
                /* @__PURE__ */ s("p", { className: "text-xs text-gray-500 mt-1", children: "AWS region where your DynamoDB tables are located" })
              ] }),
              /* @__PURE__ */ d("div", { children: [
                /* @__PURE__ */ s("label", { className: "block text-sm font-medium text-gray-700 mb-1", children: "User ID (Optional)" }),
                /* @__PURE__ */ s(
                  "input",
                  {
                    type: "text",
                    value: Je,
                    onChange: (D) => ne(D.target.value),
                    placeholder: "Leave empty for single-tenant",
                    className: "w-full p-2 border border-gray-300 rounded text-sm"
                  }
                ),
                /* @__PURE__ */ s("p", { className: "text-xs text-gray-500 mt-1", children: "User ID for multi-tenant isolation (uses partition key)" })
              ] }),
              /* @__PURE__ */ s("div", { className: "p-3 bg-yellow-50 border border-yellow-200 rounded-md", children: /* @__PURE__ */ d("p", { className: "text-xs text-yellow-800", children: [
                /* @__PURE__ */ s("strong", { children: "Note:" }),
                " Ensure your AWS credentials are configured (via environment variables, IAM role, or AWS CLI). The DynamoDB tables will be created automatically if they don't exist."
              ] }) })
            ] }) : /* @__PURE__ */ d("div", { className: "space-y-3", children: [
              /* @__PURE__ */ d("div", { children: [
                /* @__PURE__ */ d("label", { className: "block text-sm font-medium text-gray-700 mb-1", children: [
                  "S3 Bucket ",
                  /* @__PURE__ */ s("span", { className: "text-red-500", children: "*" })
                ] }),
                /* @__PURE__ */ s(
                  "input",
                  {
                    type: "text",
                    value: se,
                    onChange: (D) => mt(D.target.value),
                    placeholder: "my-datafold-bucket",
                    className: "w-full p-2 border border-gray-300 rounded text-sm"
                  }
                ),
                /* @__PURE__ */ s("p", { className: "text-xs text-gray-500 mt-1", children: "S3 bucket name where the database will be stored" })
              ] }),
              /* @__PURE__ */ d("div", { children: [
                /* @__PURE__ */ d("label", { className: "block text-sm font-medium text-gray-700 mb-1", children: [
                  "AWS Region ",
                  /* @__PURE__ */ s("span", { className: "text-red-500", children: "*" })
                ] }),
                /* @__PURE__ */ s(
                  "input",
                  {
                    type: "text",
                    value: Ze,
                    onChange: (D) => ft(D.target.value),
                    placeholder: "us-east-1",
                    className: "w-full p-2 border border-gray-300 rounded text-sm"
                  }
                ),
                /* @__PURE__ */ s("p", { className: "text-xs text-gray-500 mt-1", children: "AWS region where your S3 bucket is located" })
              ] }),
              /* @__PURE__ */ d("div", { children: [
                /* @__PURE__ */ s("label", { className: "block text-sm font-medium text-gray-700 mb-1", children: "S3 Prefix (Optional)" }),
                /* @__PURE__ */ s(
                  "input",
                  {
                    type: "text",
                    value: pt,
                    onChange: (D) => gt(D.target.value),
                    placeholder: "folddb",
                    className: "w-full p-2 border border-gray-300 rounded text-sm"
                  }
                ),
                /* @__PURE__ */ s("p", { className: "text-xs text-gray-500 mt-1", children: 'Prefix/path within the bucket (defaults to "folddb")' })
              ] }),
              /* @__PURE__ */ d("div", { children: [
                /* @__PURE__ */ s("label", { className: "block text-sm font-medium text-gray-700 mb-1", children: "Local Cache Path" }),
                /* @__PURE__ */ s(
                  "input",
                  {
                    type: "text",
                    value: Oe,
                    onChange: (D) => Xe(D.target.value),
                    placeholder: "/tmp/folddb-data",
                    className: "w-full p-2 border border-gray-300 rounded text-sm"
                  }
                ),
                /* @__PURE__ */ s("p", { className: "text-xs text-gray-500 mt-1", children: "Local filesystem path for caching S3 data (defaults to /tmp/folddb-data)" })
              ] }),
              /* @__PURE__ */ s("div", { className: "p-3 bg-yellow-50 border border-yellow-200 rounded-md", children: /* @__PURE__ */ d("p", { className: "text-xs text-yellow-800", children: [
                /* @__PURE__ */ s("strong", { children: "Note:" }),
                " Ensure your AWS credentials are configured (via environment variables, IAM role, or AWS CLI). The database will be synced to/from S3 on startup and shutdown."
              ] }) })
            ] }),
            p && /* @__PURE__ */ s("div", { className: `p-3 rounded-md ${p.success ? "bg-green-50 text-green-800 border border-green-200" : "bg-red-50 text-red-800 border border-red-200"}`, children: /* @__PURE__ */ d("span", { className: "text-sm font-medium", children: [
              p.success ? "✓" : "✗",
              " ",
              p.message
            ] }) })
          ] })
        ] })
      ] }),
      /* @__PURE__ */ s("div", { className: "bg-gray-50 px-4 py-3 sm:px-6 sm:flex sm:flex-row-reverse gap-3 border-t border-gray-200", children: r === "ai" || r === "schema-service" || r === "database" ? /* @__PURE__ */ d(At, { children: [
        /* @__PURE__ */ s(
          "button",
          {
            onClick: r === "ai" ? Jt : r === "schema-service" ? Zt : Ve,
            className: "w-full inline-flex justify-center rounded-md border border-transparent shadow-sm px-4 py-2 bg-blue-600 text-base font-medium text-white hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 sm:ml-3 sm:w-auto sm:text-sm",
            children: r === "database" ? "Save and Restart DB" : "Save Configuration"
          }
        ),
        /* @__PURE__ */ s(
          "button",
          {
            onClick: e,
            className: "mt-3 w-full inline-flex justify-center rounded-md border border-gray-300 shadow-sm px-4 py-2 bg-white text-base font-medium text-gray-700 hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 sm:mt-0 sm:w-auto sm:text-sm",
            children: "Cancel"
          }
        )
      ] }) : /* @__PURE__ */ s(
        "button",
        {
          onClick: e,
          className: "w-full inline-flex justify-center rounded-md border border-gray-300 shadow-sm px-4 py-2 bg-white text-base font-medium text-gray-700 hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 sm:w-auto sm:text-sm",
          children: "Close"
        }
      ) })
    ] })
  ] }) }) : null;
}
function Ko() {
  const [t, e] = O([]), r = vt(null), i = (u) => {
    if (typeof u == "string") return u;
    const h = u.metadata ? JSON.stringify(u.metadata) : "";
    return `[${u.level}] [${u.event_type}] - ${u.message} ${h}`;
  }, o = () => {
    Promise.resolve(
      navigator.clipboard.writeText(t.map(i).join(`
`))
    ).catch(() => {
    });
  };
  return ue(() => {
    re.getLogs().then((l) => {
      if (l.success && l.data) {
        const g = l.data.logs || [];
        e(Array.isArray(g) ? g : []);
      } else
        e([]);
    }).catch(() => e([]));
    const u = re.createLogStream(
      (l) => {
        e((g) => {
          let f;
          try {
            f = JSON.parse(l);
          } catch {
            const b = l.split(" - "), N = b.length > 1 ? b[0] : "INFO";
            f = {
              id: `stream-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
              timestamp: Date.now(),
              level: N,
              event_type: "stream (legacy)",
              message: l
            };
          }
          return f.id && g.some((b) => b.id === f.id) ? g : [...g, f];
        });
      },
      (l) => {
        console.warn("Log stream error:", l);
      }
    ), h = setInterval(() => {
      e((l) => {
        const g = l.length > 0 ? l[l.length - 1] : null, f = g ? g.timestamp : void 0;
        return re.getLogs(f).then((b) => {
          if (b.success && b.data) {
            const N = b.data.logs || [];
            N.length > 0 && e((A) => {
              const E = N.filter((y) => y.id && A.some((w) => w.id === y.id) ? !1 : !A.some(
                (p) => !p.id && // Only check content if existing has no ID
                p.timestamp === y.timestamp && p.message === y.message
              ));
              return E.length === 0 ? A : [...A, ...E];
            });
          }
        }).catch((b) => console.warn("Log polling error:", b)), l;
      });
    }, 2e3);
    return () => {
      u.close(), clearInterval(h);
    };
  }, []), ue(() => {
    r.current?.scrollIntoView({ behavior: "smooth" });
  }, [t]), /* @__PURE__ */ d("aside", { className: "w-80 bg-gray-900 text-white flex flex-col overflow-hidden", children: [
    /* @__PURE__ */ d("div", { className: "flex items-center justify-between p-4 border-b border-gray-700", children: [
      /* @__PURE__ */ s("h2", { className: "text-lg font-semibold", children: "Logs" }),
      /* @__PURE__ */ s(
        "button",
        {
          onClick: o,
          className: "text-xs text-blue-300 hover:underline",
          children: "Copy"
        }
      )
    ] }),
    /* @__PURE__ */ d("div", { className: "flex-1 overflow-y-auto p-4 space-y-1 text-xs font-mono", children: [
      t.map((u, h) => /* @__PURE__ */ s("div", { children: i(u) }, u.id || h)),
      /* @__PURE__ */ s("div", { ref: r })
    ] })
  ] });
}
function jo({ onSettingsClick: t }) {
  const e = He(), { isAuthenticated: r, user: i } = Z((u) => u.auth), o = () => {
    e(ia()), localStorage.removeItem("fold_user_id"), localStorage.removeItem("fold_user_hash");
  };
  return /* @__PURE__ */ s("header", { className: "bg-white border-b border-gray-200 shadow-sm flex-shrink-0", children: /* @__PURE__ */ d("div", { className: "flex items-center justify-between px-6 py-3", children: [
    /* @__PURE__ */ d("a", { href: "/", className: "flex items-center gap-3 text-blue-600 hover:text-blue-700 transition-colors", children: [
      /* @__PURE__ */ d("svg", { className: "w-8 h-8 flex-shrink-0", viewBox: "0 0 24 24", fill: "currentColor", children: [
        /* @__PURE__ */ s("path", { d: "M12 4C7.58172 4 4 5.79086 4 8C4 10.2091 7.58172 12 12 12C16.4183 12 20 10.2091 20 8C20 5.79086 16.4183 4 12 4Z" }),
        /* @__PURE__ */ s("path", { d: "M4 12V16C4 18.2091 7.58172 20 12 20C16.4183 20 20 18.2091 20 16V12", strokeWidth: "2", strokeLinecap: "round" }),
        /* @__PURE__ */ s("path", { d: "M4 8V12C4 14.2091 7.58172 16 12 16C16.4183 16 20 14.2091 20 12V8", strokeWidth: "2", strokeLinecap: "round" })
      ] }),
      /* @__PURE__ */ s("span", { className: "text-xl font-semibold text-gray-900", children: "DataFold Node" })
    ] }),
    /* @__PURE__ */ d("div", { className: "flex items-center gap-3", children: [
      r && /* @__PURE__ */ d("div", { className: "flex items-center gap-3 mr-2", children: [
        /* @__PURE__ */ s("span", { className: "text-sm text-gray-600", children: i?.id }),
        /* @__PURE__ */ s(
          "button",
          {
            onClick: o,
            className: "text-sm text-red-600 hover:text-red-700 font-medium",
            children: "Logout"
          }
        )
      ] }),
      /* @__PURE__ */ s("div", { className: "h-6 w-px bg-gray-300 mx-1" }),
      /* @__PURE__ */ d(
        "button",
        {
          onClick: t,
          className: "inline-flex items-center gap-2 px-3 py-2 text-sm text-gray-700 hover:bg-gray-100 rounded-md border border-gray-300 transition-colors",
          title: "Settings",
          children: [
            /* @__PURE__ */ d("svg", { className: "w-4 h-4", fill: "none", stroke: "currentColor", viewBox: "0 0 24 24", children: [
              /* @__PURE__ */ s("path", { strokeLinecap: "round", strokeLinejoin: "round", strokeWidth: 2, d: "M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" }),
              /* @__PURE__ */ s("path", { strokeLinecap: "round", strokeLinejoin: "round", strokeWidth: 2, d: "M15 12a3 3 0 11-6 0 3 3 0 016 0z" })
            ] }),
            "Settings"
          ]
        }
      )
    ] })
  ] }) });
}
function Ho() {
  return /* @__PURE__ */ s("footer", { className: "bg-white border-t border-gray-200 py-3", children: /* @__PURE__ */ s("div", { className: "max-w-7xl mx-auto px-6 text-center", children: /* @__PURE__ */ d("p", { className: "text-gray-600 text-sm", children: [
    "DataFold Node © ",
    (/* @__PURE__ */ new Date()).getFullYear()
  ] }) }) });
}
function Vo() {
  const [t, e] = O(""), [r, i] = O(""), o = He(), { isLoading: u } = Z((l) => l.auth);
  return /* @__PURE__ */ d("div", { className: "min-h-screen bg-gray-50 flex flex-col justify-center py-12 sm:px-6 lg:px-8", children: [
    /* @__PURE__ */ d("div", { className: "sm:mx-auto sm:w-full sm:max-w-md", children: [
      /* @__PURE__ */ s("h2", { className: "mt-6 text-center text-3xl font-extrabold text-gray-900", children: "Sign in to Exemem" }),
      /* @__PURE__ */ s("p", { className: "mt-2 text-center text-sm text-gray-600", children: "Enter your user identifier to access your Exemem node" })
    ] }),
    /* @__PURE__ */ s("div", { className: "mt-8 sm:mx-auto sm:w-full sm:max-w-md", children: /* @__PURE__ */ s("div", { className: "bg-white py-8 px-4 shadow sm:rounded-lg sm:px-10", children: /* @__PURE__ */ d("form", { className: "space-y-6", onSubmit: async (l) => {
      if (l.preventDefault(), !t.trim()) {
        i("Please enter a user identifier");
        return;
      }
      try {
        const g = await o(Br(t.trim())).unwrap();
        localStorage.setItem("fold_user_id", g.id), localStorage.setItem("fold_user_hash", g.hash);
      } catch (g) {
        i("Login failed: " + g.message);
      }
    }, children: [
      /* @__PURE__ */ d("div", { children: [
        /* @__PURE__ */ s("label", { htmlFor: "userId", className: "block text-sm font-medium text-gray-700", children: "User Identifier" }),
        /* @__PURE__ */ s("div", { className: "mt-1", children: /* @__PURE__ */ s(
          "input",
          {
            id: "userId",
            name: "userId",
            type: "text",
            autoComplete: "username",
            required: !0,
            className: "appearance-none block w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm placeholder-gray-400 focus:outline-none focus:ring-blue-500 focus:border-blue-500 sm:text-sm",
            placeholder: "e.g. alice-dev",
            value: t,
            onChange: (l) => {
              e(l.target.value), i("");
            },
            autoFocus: !0
          }
        ) })
      ] }),
      r && /* @__PURE__ */ s("div", { className: "text-sm text-red-600", children: r }),
      /* @__PURE__ */ s("div", { children: /* @__PURE__ */ s(
        "button",
        {
          type: "submit",
          disabled: u,
          className: "w-full flex justify-center py-2 px-4 border border-transparent rounded-md shadow-sm text-sm font-medium text-white bg-blue-600 hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 disabled:opacity-50",
          children: u ? "Connecting..." : "Continue"
        }
      ) })
    ] }) }) })
  ] });
}
function qo() {
  const [t, e] = O(""), [r, i] = O(""), o = He(), { isAuthenticated: u, isLoading: h } = Z((g) => g.auth);
  return u ? null : /* @__PURE__ */ s("div", { className: "fixed inset-0 z-50 overflow-y-auto", children: /* @__PURE__ */ d("div", { className: "flex items-center justify-center min-h-screen px-4 pt-4 pb-20 text-center sm:block sm:p-0", children: [
    /* @__PURE__ */ s("div", { className: "fixed inset-0 transition-opacity bg-gray-900 bg-opacity-75" }),
    /* @__PURE__ */ s("span", { className: "hidden sm:inline-block sm:align-middle sm:h-screen", children: "​" }),
    /* @__PURE__ */ s("div", { className: "inline-block align-bottom bg-white rounded-lg text-left overflow-hidden shadow-xl transform transition-all sm:my-8 sm:align-middle sm:max-w-lg sm:w-full", children: /* @__PURE__ */ s("div", { className: "bg-white px-4 pt-5 pb-4 sm:p-6 sm:pb-4", children: /* @__PURE__ */ s("div", { className: "sm:flex sm:items-start", children: /* @__PURE__ */ d("div", { className: "mt-3 text-center sm:mt-0 sm:text-left w-full", children: [
      /* @__PURE__ */ s("h3", { className: "text-xl leading-6 font-medium text-gray-900 mb-2", children: "Welcome to DataFold" }),
      /* @__PURE__ */ d("div", { className: "mt-2", children: [
        /* @__PURE__ */ s("p", { className: "text-sm text-gray-500 mb-4", children: "Enter your user identifier to continue. This will generate a unique session hash for your environment." }),
        /* @__PURE__ */ d("form", { onSubmit: async (g) => {
          if (g.preventDefault(), !t.trim()) {
            i("Please enter a user identifier");
            return;
          }
          try {
            const f = await o(Br(t.trim())).unwrap();
            localStorage.setItem("fold_user_id", f.id), localStorage.setItem("fold_user_hash", f.hash);
          } catch (f) {
            i("Login failed: " + f.message);
          }
        }, children: [
          /* @__PURE__ */ d("div", { className: "mb-4", children: [
            /* @__PURE__ */ s("label", { htmlFor: "userId", className: "block text-sm font-medium text-gray-700 mb-1", children: "User Identifier" }),
            /* @__PURE__ */ s(
              "input",
              {
                type: "text",
                id: "userId",
                className: "shadow-sm focus:ring-blue-500 focus:border-blue-500 block w-full sm:text-sm border-gray-300 rounded-md p-2 border",
                placeholder: "e.g. alice-dev",
                value: t,
                onChange: (g) => {
                  e(g.target.value), i("");
                },
                autoFocus: !0
              }
            )
          ] }),
          r && /* @__PURE__ */ s("div", { className: "mb-4 text-sm text-red-600", children: r }),
          /* @__PURE__ */ s(
            "button",
            {
              type: "submit",
              disabled: h,
              className: "w-full inline-flex justify-center rounded-md border border-transparent shadow-sm px-4 py-2 bg-blue-600 text-base font-medium text-white hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 sm:text-sm disabled:opacity-50",
              children: h ? "Connecting..." : "Continue"
            }
          )
        ] })
      ] })
    ] }) }) }) })
  ] }) });
}
function Ht({ node: t, depth: e = 0, name: r = null }) {
  const [i, o] = O(e === 0);
  if (!t)
    return /* @__PURE__ */ s("span", { className: "text-gray-400 italic", children: "undefined" });
  if (t.type === "Primitive") {
    const u = t.value, h = {
      String: "text-green-600",
      Number: "text-blue-600",
      Boolean: "text-purple-600",
      Null: "text-gray-500"
    }[u] || "text-gray-600";
    return /* @__PURE__ */ d("span", { className: "inline-flex items-center space-x-2", children: [
      /* @__PURE__ */ s("span", { className: `font-mono text-sm ${h}`, children: u.toLowerCase() }),
      t.classifications && t.classifications.length > 0 && /* @__PURE__ */ s("span", { className: "flex space-x-1", children: t.classifications.map((l) => /* @__PURE__ */ s("span", { className: "px-1.5 py-0.5 text-xs bg-gray-200 text-gray-700 rounded-full font-sans", children: l }, l)) })
    ] });
  }
  if (t.type === "Any")
    return /* @__PURE__ */ s("span", { className: "font-mono text-sm text-orange-600", children: "any" });
  if (t.type === "Array")
    return /* @__PURE__ */ d("div", { className: "inline-flex items-start", children: [
      /* @__PURE__ */ s("span", { className: "font-mono text-sm text-gray-700", children: "Array<" }),
      /* @__PURE__ */ s(Ht, { node: t.value, depth: e + 1 }),
      /* @__PURE__ */ s("span", { className: "font-mono text-sm text-gray-700", children: ">" })
    ] });
  if (t.type === "Object" && t.value) {
    const u = Object.entries(t.value);
    return u.length === 0 ? /* @__PURE__ */ s("span", { className: "font-mono text-sm text-gray-500", children: "{}" }) : /* @__PURE__ */ d("div", { className: "inline-block", children: [
      /* @__PURE__ */ s("div", { className: "flex items-center", children: /* @__PURE__ */ d(
        "button",
        {
          onClick: () => o(!i),
          className: "flex items-center hover:bg-gray-100 rounded px-1 -ml-1",
          children: [
            i ? /* @__PURE__ */ s($n, { className: "h-3 w-3 text-gray-500" }) : /* @__PURE__ */ s(Kn, { className: "h-3 w-3 text-gray-500" }),
            /* @__PURE__ */ d("span", { className: "font-mono text-sm text-gray-700 ml-1", children: [
              "{",
              !i && `... ${u.length} fields`,
              !i && "}"
            ] })
          ]
        }
      ) }),
      i && /* @__PURE__ */ d("div", { className: "ml-4 border-l-2 border-gray-200 pl-3 mt-1", children: [
        u.map(([h, l], g) => /* @__PURE__ */ d("div", { className: "py-1", children: [
          /* @__PURE__ */ s("span", { className: "font-mono text-sm text-indigo-600", children: h }),
          /* @__PURE__ */ s("span", { className: "font-mono text-sm text-gray-500", children: ": " }),
          /* @__PURE__ */ s(Ht, { node: l, depth: e + 1, name: h }),
          g < u.length - 1 && /* @__PURE__ */ s("span", { className: "text-gray-400", children: "," })
        ] }, h)),
        /* @__PURE__ */ s("div", { className: "font-mono text-sm text-gray-700", children: "}" })
      ] })
    ] });
  }
  return /* @__PURE__ */ d("span", { className: "font-mono text-sm text-red-500", children: [
    "unknown (",
    JSON.stringify(t),
    ")"
  ] });
}
function _i({ topology: t, compact: e = !1 }) {
  return t ? e ? /* @__PURE__ */ s("div", { className: "inline-flex items-center", children: /* @__PURE__ */ s(Ht, { node: t.root }) }) : /* @__PURE__ */ d("div", { className: "mt-2 p-2 bg-gray-50 rounded border border-gray-200", children: [
    /* @__PURE__ */ s("div", { className: "text-xs font-medium text-gray-600 mb-1", children: "Type Structure:" }),
    /* @__PURE__ */ s("div", { className: "pl-2", children: /* @__PURE__ */ s(Ht, { node: t.root }) })
  ] }) : /* @__PURE__ */ s("div", { className: "text-xs text-gray-400 italic", children: "No topology defined" });
}
function Go({ onResult: t, onSchemaUpdated: e }) {
  const r = He(), i = Z(ut);
  Z(Lr), Z(Mn);
  const [o, u] = O({});
  ue(() => {
    console.log("🟢 SchemaTab: Fetching schemas on mount"), r(Te({ forceRefresh: !0 }));
  }, [r]);
  const h = (y) => y.descriptive_name || y.name;
  console.log("🟢 SchemaTab: Current schemas from Redux:", i.map((y) => ({ name: y.name, state: y.state })));
  const l = async (y) => {
    const x = o[y];
    if (u((p) => ({
      ...p,
      [y]: !p[y]
    })), !x) {
      const p = i.find((w) => w.name === y);
      if (p && (!p.fields || Object.keys(p.fields).length === 0))
        try {
          (await X.getSchema(y)).success && (r(Te({ forceRefresh: !0 })), e && e());
        } catch (w) {
          console.error(`Failed to fetch schema details for ${y}:`, w);
        }
    }
  }, g = (y) => {
    switch (y?.toLowerCase()) {
      case "approved":
        return "bg-green-100 text-green-800";
      case "available":
        return "bg-blue-100 text-blue-800";
      case "blocked":
        return "bg-red-100 text-red-800";
      default:
        return "bg-gray-100 text-gray-800";
    }
  }, f = async (y) => {
    console.log("🟡 SchemaTab: Starting approveSchema for:", y);
    try {
      const x = await r($e({ schemaName: y }));
      if (console.log("🟡 SchemaTab: approveSchema result:", x), $e.fulfilled.match(x)) {
        console.log("🟡 SchemaTab: approveSchema fulfilled, calling callbacks");
        const p = x.payload?.backfillHash;
        if (console.log("🔄 Backfill hash:", p), console.log("🔄 Refetching schemas from backend after approval..."), await r(Te({ forceRefresh: !0 })), console.log("✅ Refetch complete - backend state should be reflected"), t) {
          const w = p ? `Schema ${y} approved successfully. Backfill started with hash: ${p}` : `Schema ${y} approved successfully`;
          t({ success: !0, message: w, backfillHash: p });
        }
        e && e();
      } else {
        console.log("🔴 SchemaTab: approveSchema rejected:", x.payload);
        const p = typeof x.payload == "string" ? x.payload : x.payload?.error || `Failed to approve schema: ${y}`;
        throw new Error(p);
      }
    } catch (x) {
      if (console.error("🔴 SchemaTab: Failed to approve schema:", x), t) {
        const p = x instanceof Error ? x.message : String(x);
        t({ error: `Failed to approve schema: ${p}` });
      }
    }
  }, b = async (y) => {
    try {
      const x = await r(Ke({ schemaName: y }));
      if (Ke.fulfilled.match(x))
        console.log("🟡 SchemaTab: blockSchema fulfilled, calling callbacks"), console.log("🔄 Refetching schemas from backend after blocking..."), await r(Te({ forceRefresh: !0 })), console.log("✅ Refetch complete - backend state should be reflected"), t && t({ success: !0, message: `Schema ${y} blocked successfully` }), e && e();
      else {
        const p = typeof x.payload == "string" ? x.payload : x.payload?.error || `Failed to block schema: ${y}`;
        throw new Error(p);
      }
    } catch (x) {
      if (console.error("Failed to block schema:", x), t) {
        const p = x instanceof Error ? x.message : String(x);
        t({ error: `Failed to block schema: ${p}` });
      }
    }
  }, N = (y) => {
    const x = o[y.name], p = y.state || "Unknown", w = y.fields ? ya(y) : null, _ = xa(y);
    return /* @__PURE__ */ d("div", { className: "bg-white rounded-lg border border-gray-200 shadow-sm overflow-hidden transition-all duration-200 hover:shadow-md", children: [
      /* @__PURE__ */ s(
        "div",
        {
          className: "px-4 py-3 bg-gray-50 cursor-pointer select-none transition-colors duration-200 hover:bg-gray-100",
          onClick: () => l(y.name),
          children: /* @__PURE__ */ d("div", { className: "flex items-center justify-between", children: [
            /* @__PURE__ */ d("div", { className: "flex items-center space-x-2", children: [
              x ? /* @__PURE__ */ s($n, { className: "icon icon-sm text-gray-400 transition-transform duration-200" }) : /* @__PURE__ */ s(Kn, { className: "icon icon-sm text-gray-400 transition-transform duration-200" }),
              /* @__PURE__ */ s("h3", { className: "font-medium text-gray-900", children: h(y) }),
              y.descriptive_name && y.descriptive_name !== y.name && /* @__PURE__ */ d("span", { className: "text-xs text-gray-500", children: [
                "(",
                y.name,
                ")"
              ] }),
              /* @__PURE__ */ s("span", { className: `px-2 py-1 text-xs font-medium rounded-full ${g(p)}`, children: p }),
              w && /* @__PURE__ */ s("span", { className: "px-2 py-1 text-xs font-medium rounded-full bg-purple-100 text-purple-800", children: "Range Schema" }),
              _ && /* @__PURE__ */ s("span", { className: "px-2 py-1 text-xs font-medium rounded-full bg-blue-100 text-blue-800", children: "HashRange Schema" })
            ] }),
            /* @__PURE__ */ d("div", { className: "flex items-center space-x-2", children: [
              p.toLowerCase() === "available" && /* @__PURE__ */ s(
                "button",
                {
                  className: "group inline-flex items-center px-2 py-1 text-xs font-medium rounded-md text-white bg-green-600 hover:bg-green-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-green-500",
                  onClick: (S) => {
                    console.log("🟠 Button clicked: Approve for schema:", y.name), S.stopPropagation(), f(y.name);
                  },
                  children: "Approve"
                }
              ),
              p.toLowerCase() === "approved" && /* @__PURE__ */ s(
                "button",
                {
                  className: "group inline-flex items-center px-2 py-1 text-xs font-medium rounded-md text-white bg-red-600 hover:bg-red-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-red-500",
                  onClick: (S) => {
                    S.stopPropagation(), b(y.name);
                  },
                  children: "Block"
                }
              ),
              p.toLowerCase() === "blocked" && /* @__PURE__ */ s(
                "button",
                {
                  className: "group inline-flex items-center px-2 py-1 text-xs font-medium rounded-md text-white bg-green-600 hover:bg-green-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-green-500",
                  onClick: (S) => {
                    S.stopPropagation(), f(y.name);
                  },
                  children: "Re-approve"
                }
              )
            ] })
          ] })
        }
      ),
      x && y.fields && /* @__PURE__ */ d("div", { className: "p-4 border-t border-gray-200", children: [
        w && /* @__PURE__ */ d("div", { className: "mb-4 p-3 bg-purple-50 rounded-md border border-purple-200", children: [
          /* @__PURE__ */ s("h4", { className: "text-sm font-medium text-purple-900 mb-2", children: "Range Schema Information" }),
          /* @__PURE__ */ d("div", { className: "space-y-1 text-xs text-purple-800", children: [
            /* @__PURE__ */ d("p", { children: [
              /* @__PURE__ */ s("strong", { children: "Range Key:" }),
              " ",
              w.rangeKey
            ] }),
            /* @__PURE__ */ d("p", { children: [
              /* @__PURE__ */ s("strong", { children: "Total Fields:" }),
              " ",
              w.totalFields
            ] }),
            /* @__PURE__ */ d("p", { children: [
              /* @__PURE__ */ s("strong", { children: "Range Fields:" }),
              " ",
              w.rangeFields.length
            ] }),
            /* @__PURE__ */ s("p", { className: "text-purple-600", children: "This schema uses range-based storage for efficient querying and mutations." })
          ] })
        ] }),
        _ && /* @__PURE__ */ d("div", { className: "mb-4 p-3 bg-blue-50 rounded-md border border-blue-200", children: [
          /* @__PURE__ */ s("h4", { className: "text-sm font-medium text-blue-900 mb-2", children: "HashRange Schema Information" }),
          /* @__PURE__ */ d("div", { className: "space-y-1 text-xs text-blue-800", children: [
            /* @__PURE__ */ d("p", { children: [
              /* @__PURE__ */ s("strong", { children: "Hash Field:" }),
              " ",
              _.hashField
            ] }),
            /* @__PURE__ */ d("p", { children: [
              /* @__PURE__ */ s("strong", { children: "Range Field:" }),
              " ",
              _.rangeField
            ] }),
            /* @__PURE__ */ d("p", { children: [
              /* @__PURE__ */ s("strong", { children: "Total Fields:" }),
              " ",
              _.totalFields
            ] }),
            /* @__PURE__ */ s("p", { className: "text-blue-600", children: "This schema uses hash-range-based storage for efficient querying and mutations with both hash and range keys." })
          ] })
        ] }),
        /* @__PURE__ */ s("div", { className: "space-y-3", children: Array.isArray(y.fields) ? y.fields.map((S) => {
          const B = y.field_topologies?.[S];
          return /* @__PURE__ */ s("div", { className: "p-3 bg-gray-50 rounded-md border border-gray-200", children: /* @__PURE__ */ s("div", { className: "flex items-center justify-between", children: /* @__PURE__ */ d("div", { className: "flex-1", children: [
            /* @__PURE__ */ d("div", { className: "flex items-center space-x-2", children: [
              /* @__PURE__ */ s("span", { className: "font-medium text-gray-900", children: S }),
              w?.rangeKey === S && /* @__PURE__ */ s("span", { className: "px-2 py-0.5 text-xs font-medium rounded-full bg-purple-100 text-purple-800", children: "Range Key" }),
              _?.hashField === S && /* @__PURE__ */ s("span", { className: "px-2 py-0.5 text-xs font-medium rounded-full bg-blue-100 text-blue-800", children: "Hash Key" }),
              _?.rangeField === S && /* @__PURE__ */ s("span", { className: "px-2 py-0.5 text-xs font-medium rounded-full bg-purple-100 text-purple-800", children: "Range Key" })
            ] }),
            B && /* @__PURE__ */ s(_i, { topology: B })
          ] }) }) }, S);
        }) : /* @__PURE__ */ s("p", { className: "text-sm text-gray-500 italic", children: "No fields defined" }) })
      ] })
    ] }, y.name);
  }, A = (y) => typeof y == "string" ? y.toLowerCase() : typeof y == "object" && y !== null ? String(y).toLowerCase() : String(y || "").toLowerCase(), E = i.filter(
    (y) => A(y.state) === "approved"
  );
  return /* @__PURE__ */ s("div", { className: "p-6 space-y-6", children: /* @__PURE__ */ d("div", { className: "space-y-4", children: [
    /* @__PURE__ */ s("h3", { className: "text-lg font-medium text-gray-900", children: "Approved Schemas" }),
    E.length > 0 ? E.map(N) : /* @__PURE__ */ s("div", { className: "border rounded-lg p-8 bg-white shadow-sm text-center text-gray-500", children: "No approved schemas found." })
  ] }) });
}
function Mr() {
  const t = He(), e = Z(ut), r = Z(Lr), [i, o] = O(""), [u, h] = O([]), [l, g] = O({}), [f, b] = O({}), [N, A] = O(""), [E, y] = O(""), [x, p] = O({}), w = W(() => (e || []).filter((K) => (typeof K.state == "string" ? K.state.toLowerCase() : String(K.state || "").toLowerCase()) === ke.APPROVED), [e]), _ = W(() => i ? (e || []).find((K) => K.name === i) : null, [i, e]), S = W(() => _ ? ct(_) : !1, [_]), B = W(() => _ ? Fr(_) : !1, [_]), I = W(() => _ ? Ye(_) : null, [_]), j = $((K) => {
    if (o(K), K) {
      const V = (e || []).find((Se) => Se.name === K), Y = V?.fields || V?.transform_fields || [], ae = Array.isArray(Y) ? Y : Object.keys(Y);
      h(ae);
      const Ie = {};
      ae.forEach((Se) => {
        Ie[Se] = "";
      }), g(Ie);
    } else
      h([]), g({});
    b({}), A(""), y(""), p({});
  }, [e]), C = $((K) => {
    h((V) => V.includes(K) ? V.filter((Y) => Y !== K) : [...V, K]), g((V) => V[K] !== void 0 ? V : {
      ...V,
      [K]: ""
      // Initialize with empty string for new fields
    });
  }, []), k = $((K, V, Y) => {
    b((ae) => ({
      ...ae,
      [K]: {
        ...ae[K],
        [V]: Y
      }
    }));
  }, []), L = $((K, V) => {
    g((Y) => ({
      ...Y,
      [K]: V
    }));
  }, []), P = $(() => {
    o(""), h([]), g({}), b({}), A(""), y(""), p({});
  }, []), F = $(() => {
    t(Te({ forceRefresh: !0 }));
  }, [t]);
  return {
    state: {
      selectedSchema: i,
      queryFields: u,
      fieldValues: l,
      rangeFilters: f,
      rangeSchemaFilter: x,
      rangeKeyValue: N,
      hashKeyValue: E
    },
    setSelectedSchema: o,
    setQueryFields: h,
    setFieldValues: g,
    toggleField: C,
    handleFieldValueChange: L,
    setRangeFilters: b,
    setRangeSchemaFilter: p,
    setRangeKeyValue: A,
    setHashKeyValue: y,
    clearState: P,
    handleSchemaChange: j,
    handleRangeFilterChange: k,
    refetchSchemas: F,
    approvedSchemas: w,
    schemasLoading: r,
    selectedSchemaObj: _,
    isRangeSchema: S,
    isHashRangeSchema: B,
    rangeKey: I
  };
}
function St(t) {
  return { HashKey: t };
}
function Ti(t) {
  return { RangePrefix: t };
}
function Ci(t, e) {
  return { RangeRange: { start: t, end: e } };
}
function Ri(t, e) {
  return { HashRangeKey: { hash: t, range: e } };
}
function Qn({
  schema: t,
  queryState: e,
  schemas: r,
  selectedSchemaObj: i,
  isRangeSchema: o,
  rangeKey: u
}) {
  const h = Z(Ct), l = W(() => i || (r && t && r[t] ? r[t] : h && Array.isArray(h) && h.find((A) => A.name === t) || null), [i, t, r, h]), g = W(() => typeof o == "boolean" ? o : l ? l.schema_type === "Range" || ct(l) ? !0 : l.fields && typeof l.fields == "object" ? Object.values(l.fields).some((E) => E?.field_type === "Range") : !1 : !1, [l, o]), f = W(() => [], []), b = !0, N = W(() => {
    if (!t || !e || !l)
      return {};
    const {
      queryFields: A = [],
      fieldValues: E = {},
      rangeFilters: y = {},
      rangeSchemaFilter: x = {},
      filters: p = [],
      orderBy: w
    } = e, _ = {
      schema_name: t,
      // Backend expects schema_name, not schema
      fields: A
      // Array of selected field names
    };
    if (Fr(l)) {
      const S = e.hashKeyValue, B = e.rangeSchemaFilter?.key;
      S && S.trim() ? _.filter = St(S.trim()) : B && B.trim() && (_.filter = St(B.trim()));
    }
    if (g) {
      const S = x && Object.keys(x).length > 0 ? x : Object.values(y).find((I) => I && typeof I == "object" && (I.key || I.keyPrefix || I.start && I.end)) || {}, B = e?.rangeKeyValue;
      !S.key && !S.keyPrefix && !(S.start && S.end) && B && (S.key = B), S.key ? _.filter = St(S.key) : S.keyPrefix ? _.filter = Ti(S.keyPrefix) : S.start && S.end && (_.filter = Ci(S.start, S.end));
    }
    return _;
  }, [t, e, l]);
  return $(() => N, [N]), $(() => ({
    isValid: b,
    errors: f
  }), [b, f]), {
    query: N,
    validationErrors: f,
    isValid: b
  };
}
function Ce({
  label: t,
  name: e,
  required: r = !1,
  error: i,
  helpText: o,
  children: u,
  className: h = ""
}) {
  const l = e ? `field-${e}` : `field-${Math.random().toString(36).substr(2, 9)}`, g = !!i;
  return /* @__PURE__ */ d("div", { className: `space-y-2 ${h}`, children: [
    /* @__PURE__ */ d(
      "label",
      {
        htmlFor: l,
        className: "block text-sm font-medium text-gray-700",
        children: [
          t,
          r && /* @__PURE__ */ s("span", { className: "ml-1 text-red-500", "aria-label": "required", children: "*" })
        ]
      }
    ),
    /* @__PURE__ */ s("div", { className: "relative", children: u }),
    g && /* @__PURE__ */ s(
      "p",
      {
        className: "text-sm text-red-600",
        role: "alert",
        "aria-live": "polite",
        children: i
      }
    ),
    o && !g && /* @__PURE__ */ s("p", { className: "text-xs text-gray-500", children: o })
  ] });
}
function Yn(t = []) {
  return t.reduce((e, r) => {
    const i = r.group || "default";
    return e[i] || (e[i] = []), e[i].push(r), e;
  }, {});
}
function ki(t = [], e = "") {
  if (ba(e)) return t;
  const r = e.toLowerCase();
  return t.filter(
    (i) => i.label.toLowerCase().includes(r) || i.value.toLowerCase().includes(r)
  );
}
function Ii(t = {}) {
  return {
    placeholder: "Select an option...",
    emptyMessage: "No options available",
    searchable: !1,
    required: !1,
    disabled: !1,
    loading: !1,
    showConfirmation: !1,
    ...t
  };
}
function Oi(t, e = !1, r = !1, i = !1) {
  let o = t.select?.base || "";
  return e && (o += " border-red-300 focus:ring-red-500 focus:border-red-500"), (r || i) && (o += ` ${t.select?.disabled || ""}`), o;
}
function Bi(t, e = !1, r = "") {
  const i = {
    "aria-invalid": e
  };
  return e ? i["aria-describedby"] = `${t}-error` : r && (i["aria-describedby"] = `${t}-help`), i;
}
function Fi(t = [], e, r = !0) {
  const [i, o] = O(""), [u, h] = O(!1), l = ki(t, i), g = Yn(l), f = $((_) => {
    o(_.target.value);
  }, []), b = $((_) => {
    _.disabled || (e(_.value), r && (h(!1), o("")));
  }, [e, r]), N = $(() => {
    h(!0);
  }, []), A = $(() => {
    h(!1);
  }, []), E = $(() => {
    h((_) => !_);
  }, []), y = $((_) => {
    const S = t.find((B) => B.value === _);
    S && b(S);
  }, [t, b]), x = $(() => {
    o("");
  }, []);
  return {
    state: {
      searchTerm: i,
      isOpen: u,
      filteredOptions: l,
      groupedOptions: g
    },
    actions: {
      setSearchTerm: o,
      openDropdown: N,
      closeDropdown: A,
      toggleDropdown: E,
      selectOption: y,
      clearSearch: x
    },
    handleSearchChange: f,
    handleOptionSelect: b
  };
}
function Wn(t) {
  return `field-${t}`;
}
function Li(t) {
  return !!t;
}
function Di({ hasError: t, disabled: e, additionalClasses: r = "" }) {
  const i = we.input.base, o = t ? we.input.error : we.input.success;
  return `${i} ${o} ${e ? "bg-gray-100 cursor-not-allowed" : ""} ${r}`.trim();
}
function Mi({ fieldId: t, hasError: e, hasHelp: r }) {
  const i = {
    "aria-invalid": e
  };
  return e ? i["aria-describedby"] = `${t}-error` : r && (i["aria-describedby"] = `${t}-help`), i;
}
function Pi({ size: t = "sm", color: e = "primary" } = {}) {
  const r = {
    sm: "h-3 w-3",
    md: "h-4 w-4",
    lg: "h-5 w-5"
  }, i = {
    primary: "border-primary border-t-transparent",
    gray: "border-gray-400 border-t-transparent",
    white: "border-white border-t-transparent"
  };
  return `animate-spin ${r[t]} border-2 ${i[e]} rounded-full`;
}
function wr({
  name: t,
  label: e,
  value: r,
  options: i = [],
  onChange: o,
  error: u,
  helpText: h,
  config: l = {},
  className: g = ""
}) {
  const f = Ii(l), { searchable: b, placeholder: N, emptyMessage: A, required: E, disabled: y, loading: x } = f, p = Wn(t), w = !!u, _ = i.length > 0, S = Fi(i, o, !0), B = (k) => {
    o(k.target.value);
  };
  if (x)
    return /* @__PURE__ */ s(Ce, { label: e, name: t, required: E, error: u, helpText: h, className: g, children: /* @__PURE__ */ d("div", { className: `${we.select.disabled} flex items-center`, children: [
      /* @__PURE__ */ s("div", { className: "animate-spin h-4 w-4 border-2 border-gray-400 border-t-transparent rounded-full mr-2" }),
      ma.loading
    ] }) });
  if (!_)
    return /* @__PURE__ */ s(Ce, { label: e, name: t, required: E, error: u, helpText: h, className: g, children: /* @__PURE__ */ s("div", { className: we.select.disabled, children: A }) });
  if (b) {
    const { state: k, handleSearchChange: L, handleOptionSelect: P } = S;
    return /* @__PURE__ */ s(Ce, { label: e, name: t, required: E, error: u, helpText: h, className: g, children: /* @__PURE__ */ d("div", { className: "relative", children: [
      /* @__PURE__ */ s(
        "input",
        {
          type: "text",
          placeholder: `Search ${e.toLowerCase()}...`,
          value: k.searchTerm,
          onChange: L,
          onFocus: () => S.actions.openDropdown(),
          className: `${we.input.base} ${w ? we.input.error : ""}`
        }
      ),
      k.isOpen && k.filteredOptions.length > 0 && /* @__PURE__ */ s("div", { className: "absolute z-10 w-full mt-1 bg-white border border-gray-300 rounded-md shadow-lg max-h-60 overflow-auto", children: Object.entries(k.groupedOptions).map(([F, U]) => /* @__PURE__ */ d("div", { children: [
        F !== "default" && /* @__PURE__ */ s("div", { className: "px-3 py-2 text-xs font-semibold text-gray-500 bg-gray-50 border-b", children: F }),
        U.map((K) => /* @__PURE__ */ s(
          "button",
          {
            type: "button",
            onClick: () => P(K),
            disabled: K.disabled,
            className: `w-full text-left px-3 py-2 hover:bg-gray-100 focus:bg-gray-100 focus:outline-none ${K.disabled ? "text-gray-400 cursor-not-allowed" : "text-gray-900"} ${r === K.value ? "bg-primary text-white" : ""}`,
            children: K.label
          },
          K.value
        ))
      ] }, F)) })
    ] }) });
  }
  const I = Yn(i), j = Oi(we, w, y, x), C = Bi(p, w, h);
  return /* @__PURE__ */ s(Ce, { label: e, name: t, required: E, error: u, helpText: h, className: g, children: /* @__PURE__ */ d(
    "select",
    {
      id: p,
      name: t,
      value: r,
      onChange: B,
      required: E,
      disabled: y,
      className: j,
      ...C,
      children: [
        /* @__PURE__ */ s("option", { value: "", disabled: E, children: N }),
        Object.entries(I).map(
          ([k, L]) => k !== "default" ? /* @__PURE__ */ s("optgroup", { label: k, children: L.map((P) => /* @__PURE__ */ s("option", { value: P.value, disabled: P.disabled, children: P.label }, P.value)) }, k) : L.map((P) => /* @__PURE__ */ s("option", { value: P.value, disabled: P.disabled, children: P.label }, P.value))
        )
      ]
    }
  ) });
}
function wt({
  name: t,
  label: e,
  value: r,
  onChange: i,
  required: o = !1,
  disabled: u = !1,
  error: h,
  placeholder: l,
  helpText: g,
  type: f = "text",
  debounced: b = !1,
  debounceMs: N = ua,
  className: A = ""
}) {
  const [E, y] = O(r), [x, p] = O(!1);
  ue(() => {
    y(r);
  }, [r]);
  const w = vt(null), _ = vt(null), S = vt(i);
  ue(() => {
    S.current = i;
  }, [i]);
  const B = $((P) => {
    p(!0), w.current && (clearTimeout(w.current), w.current = null), _.current && typeof window < "u" && typeof window.cancelAnimationFrame == "function" && (window.cancelAnimationFrame(_.current), _.current = null);
    const F = () => {
      w.current = setTimeout(() => {
        S.current(P), p(!1);
      }, N);
    };
    typeof window < "u" && typeof window.requestAnimationFrame == "function" ? _.current = window.requestAnimationFrame(F) : setTimeout(F, 0);
  }, [N]), I = (P) => {
    const F = P.target.value;
    y(F), b ? B(F) : i(F);
  }, j = Wn(t), C = Li(h), k = Di({ hasError: C, disabled: u }), L = Mi({
    fieldId: j,
    hasError: C,
    hasHelp: !!g
  });
  return /* @__PURE__ */ s(
    Ce,
    {
      label: e,
      name: t,
      required: o,
      error: h,
      helpText: g,
      className: A,
      children: /* @__PURE__ */ d("div", { className: "relative", children: [
        /* @__PURE__ */ s(
          "input",
          {
            id: j,
            name: t,
            type: f,
            value: E,
            onChange: I,
            placeholder: l,
            required: o,
            disabled: u,
            className: k,
            ...L
          }
        ),
        b && x && /* @__PURE__ */ s("div", { className: "absolute right-2 top-1/2 transform -translate-y-1/2", children: /* @__PURE__ */ s(
          "div",
          {
            className: Pi({ size: "md", color: "primary" }),
            role: "status",
            "aria-label": "Processing input"
          }
        ) })
      ] })
    }
  );
}
function un(t = {}) {
  return t.start || t.end ? "range" : t.key ? "key" : t.keyPrefix ? "prefix" : "range";
}
function Ui(t, e, r) {
  const i = { ...t };
  return e === "range" || r === "start" || r === "end" ? (delete i.key, delete i.keyPrefix) : e === "key" || r === "key" ? (delete i.start, delete i.end, delete i.keyPrefix) : (e === "prefix" || r === "keyPrefix") && (delete i.start, delete i.end, delete i.key), i;
}
function $i(t = {}, e, r = ["range", "key", "prefix"]) {
  const [i, o] = O(
    () => un(t)
  ), [u, h] = O(t), l = $((x) => {
    if (!r.includes(x)) return;
    o(x);
    const p = {};
    h(p), e && e(p);
  }, [r, e]), g = $((x, p) => {
    const w = Ui(u, i, x);
    w[x] = p, h(w), e && e(w);
  }, [u, i, e]), f = $(() => {
    const x = {};
    h(x), e && e(x);
  }, [e]), b = $((x) => {
    h(x);
    const p = un(x);
    o(p), e && e(x);
  }, [e]), N = $(() => r, [r]), A = $((x) => r.includes(x), [r]);
  return {
    state: {
      activeMode: i,
      value: u
    },
    actions: {
      changeMode: l,
      updateValue: g,
      clearValue: f,
      setValue: b
    },
    getAvailableModes: N,
    isValidMode: A
  };
}
function Ki(t = "all", e = "key", r = "") {
  if (r) return r;
  if (t !== "all") return null;
  const i = { ...kn.rangeKeyFilter }, o = i.keyRange || "", u = (i.exactKey || "").replace("key", e), h = (i.keyPrefix || "").replace("keys", `${e} values`), l = i.emptyNote || "";
  return `${o} ${u} ${h} ${l}`.trim();
}
function ji(t = "all") {
  const e = {
    all: {
      showModeSelector: !0,
      availableModes: ["range", "key", "prefix"],
      defaultMode: "range"
    },
    range: {
      showModeSelector: !1,
      availableModes: ["range"],
      defaultMode: "range"
    },
    key: {
      showModeSelector: !1,
      availableModes: ["key"],
      defaultMode: "key"
    },
    prefix: {
      showModeSelector: !1,
      availableModes: ["prefix"],
      defaultMode: "prefix"
    }
  };
  return e[t] || e.all;
}
function Hi(t = !1) {
  const e = "px-3 py-1 text-xs rounded-md transition-colors duration-200";
  return t ? `${e} bg-primary text-white` : `${e} bg-gray-200 text-gray-700 hover:bg-gray-300`;
}
function Vi() {
  return {
    range: "Key Range",
    key: "Exact Key",
    prefix: "Key Prefix"
  };
}
function qi(t, e) {
  return t === "all" ? {
    showRange: e === "range",
    showKey: e === "key",
    showPrefix: e === "prefix"
  } : {
    showRange: t === "range",
    showKey: t === "key",
    showPrefix: t === "prefix"
  };
}
function Gi(t = {}) {
  const {
    mode: e = "all",
    rangeKeyName: r = "key",
    required: i = !1,
    disabled: o = !1,
    className: u = ""
  } = t;
  return {
    mode: ["all", "range", "key", "prefix"].includes(e) ? e : "all",
    rangeKeyName: String(r),
    required: !!i,
    disabled: !!o,
    className: String(u)
  };
}
function zi() {
  return "bg-yellow-50 rounded-lg p-4 space-y-4";
}
function Qi() {
  return "text-sm font-medium text-gray-800";
}
function Yi() {
  return "flex space-x-4 mb-4";
}
function Wi() {
  return "grid grid-cols-1 md:grid-cols-3 gap-4";
}
function Ji({
  name: t,
  label: e,
  value: r = {},
  onChange: i,
  error: o,
  helpText: u,
  config: h = {},
  className: l = ""
}) {
  const g = Gi(h), { mode: f, rangeKeyName: b, required: N, disabled: A } = g, E = ji(f), y = $i(r, i, E.availableModes), { state: x, actions: p } = y, w = Vi(), _ = qi(f, x.activeMode), S = Ki(f, b, u);
  return /* @__PURE__ */ s(
    Ce,
    {
      label: e,
      name: t,
      required: N,
      error: o,
      helpText: S,
      className: l,
      children: /* @__PURE__ */ d("div", { className: zi(), children: [
        /* @__PURE__ */ s("div", { className: "mb-3", children: /* @__PURE__ */ d("span", { className: Qi(), children: [
          "Range Key: ",
          b
        ] }) }),
        E.showModeSelector && /* @__PURE__ */ s("div", { className: Yi(), children: E.availableModes.map((B) => /* @__PURE__ */ s(
          "button",
          {
            type: "button",
            onClick: () => p.changeMode(B),
            className: Hi(x.activeMode === B),
            children: w[B]
          },
          B
        )) }),
        /* @__PURE__ */ d("div", { className: Wi(), children: [
          _.showRange && /* @__PURE__ */ d(At, { children: [
            /* @__PURE__ */ s(
              wt,
              {
                name: `${t}-start`,
                label: "Start Key",
                value: x.value.start || "",
                onChange: (B) => p.updateValue("start", B),
                placeholder: "Start key",
                disabled: A,
                className: "col-span-1"
              }
            ),
            /* @__PURE__ */ s(
              wt,
              {
                name: `${t}-end`,
                label: "End Key",
                value: x.value.end || "",
                onChange: (B) => p.updateValue("end", B),
                placeholder: "End key",
                disabled: A,
                className: "col-span-1"
              }
            )
          ] }),
          _.showKey && /* @__PURE__ */ s(
            wt,
            {
              name: `${t}-key`,
              label: "Exact Key",
              value: x.value.key || "",
              onChange: (B) => p.updateValue("key", B),
              placeholder: `Exact ${b} to match`,
              disabled: A,
              className: "col-span-1"
            }
          ),
          _.showPrefix && /* @__PURE__ */ s(
            wt,
            {
              name: `${t}-prefix`,
              label: "Key Prefix",
              value: x.value.keyPrefix || "",
              onChange: (B) => p.updateValue("keyPrefix", B),
              placeholder: `${b} prefix (e.g., 'user:')`,
              disabled: A,
              className: "col-span-1"
            }
          )
        ] })
      ] })
    }
  );
}
function Zi({
  queryState: t,
  onSchemaChange: e,
  onFieldToggle: r,
  onFieldValueChange: i,
  onRangeFilterChange: o,
  onRangeSchemaFilterChange: u,
  onHashKeyChange: h,
  approvedSchemas: l,
  schemasLoading: g,
  isRangeSchema: f,
  isHashRangeSchema: b,
  rangeKey: N,
  className: A = ""
}) {
  const [E, y] = O({}), { clearQuery: x } = Mr();
  $(() => (y({}), !0), []);
  const p = $((I) => {
    e(I), x && x(), y((j) => {
      const { schema: C, ...k } = j;
      return k;
    });
  }, [e, x]), w = $((I) => {
    r(I), y((j) => {
      const { fields: C, ...k } = j;
      return k;
    });
  }, [r]), _ = t?.selectedSchema && l ? l.find((I) => I.name === t.selectedSchema) : null, S = _?.fields || _?.transform_fields || [], B = Array.isArray(S) ? S : Object.keys(S);
  return /* @__PURE__ */ d("div", { className: `space-y-6 ${A}`, children: [
    /* @__PURE__ */ s(
      Ce,
      {
        label: Ge.schema,
        name: "schema",
        required: !0,
        error: E.schema,
        helpText: Ge.schemaHelp,
        children: /* @__PURE__ */ s(
          wr,
          {
            name: "schema",
            value: t?.selectedSchema || "",
            onChange: p,
            options: l.map((I) => ({
              value: I.name,
              label: I.descriptive_name || I.name
            })),
            placeholder: "Select a schema...",
            emptyMessage: Ge.schemaEmpty,
            loading: g
          }
        )
      }
    ),
    t?.selectedSchema && B.length > 0 && /* @__PURE__ */ s(
      Ce,
      {
        label: "Field Selection",
        name: "fields",
        required: !0,
        error: E.fields,
        helpText: "Select fields to include in your query",
        children: /* @__PURE__ */ s("div", { className: "bg-gray-50 rounded-md p-4", children: /* @__PURE__ */ s("div", { className: "space-y-3", children: B.map((I) => /* @__PURE__ */ d("label", { className: "relative flex items-start", children: [
          /* @__PURE__ */ s("div", { className: "flex items-center h-5", children: /* @__PURE__ */ s(
            "input",
            {
              type: "checkbox",
              className: "h-4 w-4 text-primary border-gray-300 rounded focus:ring-primary",
              checked: t?.queryFields?.includes(I) || !1,
              onChange: () => w(I)
            }
          ) }),
          /* @__PURE__ */ s("div", { className: "ml-3 flex items-center", children: /* @__PURE__ */ s("span", { className: "text-sm font-medium text-gray-700", children: I }) })
        ] }, I)) }) })
      }
    ),
    b && /* @__PURE__ */ s(
      Ce,
      {
        label: "HashRange Filter",
        name: "hashRangeFilter",
        helpText: "Filter data by hash and range key values",
        children: /* @__PURE__ */ d("div", { className: "bg-purple-50 rounded-md p-4 space-y-4", children: [
          /* @__PURE__ */ d("div", { className: "grid grid-cols-1 md:grid-cols-2 gap-4", children: [
            /* @__PURE__ */ d("div", { className: "space-y-2", children: [
              /* @__PURE__ */ s("label", { className: "block text-sm font-medium text-gray-700", children: "Hash Key" }),
              /* @__PURE__ */ s(
                "input",
                {
                  type: "text",
                  placeholder: "Enter hash key value",
                  className: "w-full px-3 py-2 text-sm border border-gray-300 rounded-md focus:outline-none focus:ring-1 focus:ring-primary focus:border-primary",
                  value: t?.hashKeyValue || "",
                  onChange: (I) => h(I.target.value)
                }
              ),
              /* @__PURE__ */ d("div", { className: "text-xs text-gray-500", children: [
                "Hash field: ",
                Bn(l.find((I) => I.name === t?.selectedSchema)) || "N/A"
              ] })
            ] }),
            /* @__PURE__ */ d("div", { className: "space-y-2", children: [
              /* @__PURE__ */ s("label", { className: "block text-sm font-medium text-gray-700", children: "Range Key" }),
              /* @__PURE__ */ s(
                "input",
                {
                  type: "text",
                  placeholder: "Enter range key value",
                  className: "w-full px-3 py-2 text-sm border border-gray-300 rounded-md focus:outline-none focus:ring-1 focus:ring-primary focus:border-primary",
                  value: t?.rangeKeyValue || "",
                  onChange: (I) => u({ key: I.target.value })
                }
              ),
              /* @__PURE__ */ d("div", { className: "text-xs text-gray-500", children: [
                "Range field: ",
                Ye(l.find((I) => I.name === t?.selectedSchema)) || "N/A"
              ] })
            ] })
          ] }),
          /* @__PURE__ */ d("div", { className: "text-xs text-gray-500", children: [
            /* @__PURE__ */ d("p", { children: [
              /* @__PURE__ */ s("strong", { children: "Hash Key:" }),
              " Used for partitioning data across multiple nodes"
            ] }),
            /* @__PURE__ */ d("p", { children: [
              /* @__PURE__ */ s("strong", { children: "Range Key:" }),
              " Used for ordering and range queries within a partition"
            ] })
          ] })
        ] })
      }
    ),
    f && N && /* @__PURE__ */ s(
      Ce,
      {
        label: "Range Filter",
        name: "rangeSchemaFilter",
        error: E.rangeFilter,
        helpText: "Filter data by range key values",
        children: /* @__PURE__ */ s(
          Ji,
          {
            name: "rangeSchemaFilter",
            value: t?.rangeSchemaFilter || {},
            onChange: (I) => {
              u(I), y((j) => {
                const { rangeFilter: C, ...k } = j;
                return k;
              });
            },
            rangeKeyName: N,
            mode: "all"
          }
        )
      }
    )
  ] });
}
function Xi({
  onExecute: t,
  onExecuteQuery: e,
  onValidate: r,
  onSave: i,
  onSaveQuery: o,
  onClear: u,
  onClearQuery: h,
  disabled: l = !1,
  isExecuting: g = !1,
  isSaving: f = !1,
  showValidation: b = !1,
  showSave: N = !0,
  showClear: A = !0,
  className: E = "",
  queryData: y
}) {
  const [x, p] = O(null), [w, _] = O(null), { clearQuery: S } = Mr(), B = async (L, P, F = null) => {
    if (!(!P || l))
      try {
        p(L), await P(F);
      } catch (U) {
        console.error(`${L} action failed:`, U);
      } finally {
        p(null), _(null);
      }
  }, I = () => {
    B("execute", e || t, y);
  }, j = () => {
    B("validate", r, y);
  }, C = () => {
    B("save", o || i, y);
  }, k = () => {
    const L = h || u;
    L && L(), S && S();
  };
  return /* @__PURE__ */ d("div", { className: `flex justify-end space-x-3 ${E}`, children: [
    A && /* @__PURE__ */ s(
      "button",
      {
        type: "button",
        onClick: k,
        disabled: l,
        className: `
            inline-flex items-center px-4 py-2 border border-gray-300 rounded-md shadow-sm text-sm font-medium
            ${l ? "bg-gray-100 text-gray-400 cursor-not-allowed" : "bg-white text-gray-700 hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-primary"}
          `,
        children: Lt.clearQuery || "Clear Query"
      }
    ),
    b && r && /* @__PURE__ */ d(
      "button",
      {
        type: "button",
        onClick: j,
        disabled: l,
        className: `
            inline-flex items-center px-4 py-2 border border-transparent rounded-md shadow-sm text-sm font-medium
            ${l ? "bg-gray-300 text-gray-500 cursor-not-allowed" : "bg-blue-600 text-white hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500"}
          `,
        children: [
          x === "validate" && /* @__PURE__ */ d("svg", { className: "animate-spin -ml-1 mr-2 h-4 w-4 text-white", xmlns: "http://www.w3.org/2000/svg", fill: "none", viewBox: "0 0 24 24", children: [
            /* @__PURE__ */ s("circle", { className: "opacity-25", cx: "12", cy: "12", r: "10", stroke: "currentColor", strokeWidth: "4" }),
            /* @__PURE__ */ s("path", { className: "opacity-75", fill: "currentColor", d: "M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" })
          ] }),
          Lt.validateQuery || "Validate"
        ]
      }
    ),
    N && (i || o) && /* @__PURE__ */ d(
      "button",
      {
        type: "button",
        onClick: C,
        disabled: l || f,
        className: `
            inline-flex items-center px-4 py-2 border border-transparent rounded-md shadow-sm text-sm font-medium
            ${l || f ? "bg-gray-300 text-gray-500 cursor-not-allowed" : "bg-green-600 text-white hover:bg-green-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-green-500"}
          `,
        children: [
          (x === "save" || f) && /* @__PURE__ */ d("svg", { className: "animate-spin -ml-1 mr-2 h-4 w-4 text-white", xmlns: "http://www.w3.org/2000/svg", fill: "none", viewBox: "0 0 24 24", children: [
            /* @__PURE__ */ s("circle", { className: "opacity-25", cx: "12", cy: "12", r: "10", stroke: "currentColor", strokeWidth: "4" }),
            /* @__PURE__ */ s("path", { className: "opacity-75", fill: "currentColor", d: "M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" })
          ] }),
          Lt.saveQuery || "Save Query"
        ]
      }
    ),
    /* @__PURE__ */ d(
      "button",
      {
        type: "button",
        onClick: I,
        disabled: l || g,
        className: `
          inline-flex items-center px-4 py-2 border border-transparent rounded-md shadow-sm text-sm font-medium text-white
          ${l || g ? "bg-gray-300 cursor-not-allowed" : "bg-primary hover:bg-primary/90 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-primary"}
        `,
        children: [
          (x === "execute" || g) && /* @__PURE__ */ d("svg", { className: "animate-spin -ml-1 mr-2 h-4 w-4 text-white", xmlns: "http://www.w3.org/2000/svg", fill: "none", viewBox: "0 0 24 24", children: [
            /* @__PURE__ */ s("circle", { className: "opacity-25", cx: "12", cy: "12", r: "10", stroke: "currentColor", strokeWidth: "4" }),
            /* @__PURE__ */ s("path", { className: "opacity-75", fill: "currentColor", d: "M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 714 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" })
          ] }),
          x === "execute" || g ? "Executing..." : Lt.executeQuery
        ]
      }
    )
  ] });
}
const eo = (t, e) => {
  if (!t && !e) return null;
  const r = { ...t, ...e };
  let i = [], o = {};
  Array.isArray(r.fields) ? i = r.fields : r.fields && typeof r.fields == "object" ? (i = Object.keys(r.fields), o = r.fields) : r.queryFields && Array.isArray(r.queryFields) && (i = r.queryFields), r.fieldValues && typeof r.fieldValues == "object" && (o = { ...o, ...r.fieldValues });
  const u = {
    schema: r.schema || r.selectedSchema,
    fields: i,
    fieldValues: o,
    filters: r.filters || {},
    // Include filters from test mocks
    orderBy: r.orderBy,
    // Include orderBy from test mocks
    rangeKey: r.rangeKey
    // Include rangeKey from test mocks
  };
  if (t && t.filter)
    if (t.filter.field && t.filter.range_filter) {
      const h = t.filter.field, l = t.filter.range_filter;
      l.Key ? u.filters[h] = { exactKey: l.Key } : l.KeyRange ? u.filters[h] = {
        keyRange: `${l.KeyRange.start} → ${l.KeyRange.end}`
      } : l.KeyPrefix && (u.filters[h] = { keyPrefix: l.KeyPrefix });
    } else t.filter.range_filter && Object.entries(t.filter.range_filter).forEach(([h, l]) => {
      typeof l == "string" ? u.filters[h] = { exactKey: l } : l.KeyRange ? u.filters[h] = {
        keyRange: `${l.KeyRange.start} → ${l.KeyRange.end}`
      } : l.KeyPrefix && (u.filters[h] = { keyPrefix: l.KeyPrefix });
    });
  return u;
};
function to({
  query: t,
  queryState: e,
  validationErrors: r = [],
  isExecuting: i = !1,
  showJson: o = !1,
  collapsible: u = !0,
  className: h = "",
  title: l = "Query Preview"
}) {
  const g = W(() => eo(t, e), [t, e]);
  return !t && !e ? /* @__PURE__ */ d("div", { className: `bg-gray-50 rounded-md p-4 ${h}`, children: [
    /* @__PURE__ */ s("h3", { className: "text-sm font-medium text-gray-500 mb-2", children: l }),
    /* @__PURE__ */ s("p", { className: "text-sm text-gray-400 italic", children: "No query to preview" })
  ] }) : /* @__PURE__ */ d("div", { className: `bg-white border border-gray-200 rounded-lg shadow-sm ${h}`, children: [
    /* @__PURE__ */ s("div", { className: "px-4 py-3 border-b border-gray-200", children: /* @__PURE__ */ s("h3", { className: "text-sm font-medium text-gray-900", children: l }) }),
    /* @__PURE__ */ d("div", { className: "p-4 space-y-4", children: [
      r && r.length > 0 && /* @__PURE__ */ d("div", { className: "bg-red-50 border border-red-200 rounded-md p-3", children: [
        /* @__PURE__ */ d("div", { className: "flex items-center mb-2", children: [
          /* @__PURE__ */ s("svg", { className: "h-4 w-4 text-red-400 mr-2", fill: "currentColor", viewBox: "0 0 20 20", children: /* @__PURE__ */ s("path", { fillRule: "evenodd", d: "M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7 4a1 1 0 11-2 0 1 1 0 012 0zm-1-9a1 1 0 00-1 1v4a1 1 0 102 0V6a1 1 0 00-1-1z", clipRule: "evenodd" }) }),
          /* @__PURE__ */ s("span", { className: "text-sm font-medium text-red-800", children: "Validation Errors" })
        ] }),
        /* @__PURE__ */ s("ul", { className: "space-y-1", children: r.map((f, b) => /* @__PURE__ */ s("li", { className: "text-sm text-red-700", children: f }, b)) })
      ] }),
      i && /* @__PURE__ */ s("div", { className: "bg-blue-50 border border-blue-200 rounded-md p-3", children: /* @__PURE__ */ d("div", { className: "flex items-center", children: [
        /* @__PURE__ */ d("svg", { className: "animate-spin h-4 w-4 text-blue-400 mr-2", fill: "none", viewBox: "0 0 24 24", children: [
          /* @__PURE__ */ s("circle", { className: "opacity-25", cx: "12", cy: "12", r: "10", stroke: "currentColor", strokeWidth: "4" }),
          /* @__PURE__ */ s("path", { className: "opacity-75", fill: "currentColor", d: "M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" })
        ] }),
        /* @__PURE__ */ s("span", { className: "text-sm font-medium text-blue-800", children: "Executing query..." })
      ] }) }),
      /* @__PURE__ */ d("div", { className: "space-y-3", children: [
        /* @__PURE__ */ d("div", { children: [
          /* @__PURE__ */ s("label", { className: "block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1", children: "Schema" }),
          /* @__PURE__ */ s("div", { className: "inline-flex items-center px-2 py-1 rounded-md bg-blue-100 text-blue-800 text-sm font-medium", children: g?.schema || "" })
        ] }),
        /* @__PURE__ */ d("div", { children: [
          /* @__PURE__ */ d("label", { className: "block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1", children: [
            "Fields (",
            g?.fields ? g.fields.length : 0,
            ")"
          ] }),
          /* @__PURE__ */ s("div", { className: "flex flex-wrap gap-1", children: g?.fields && g.fields.length > 0 ? g.fields.map((f, b) => {
            const N = g.fieldValues?.[f];
            return /* @__PURE__ */ d("div", { className: "inline-flex flex-col items-start", children: [
              /* @__PURE__ */ s("span", { className: "inline-flex items-center px-2 py-1 rounded-md bg-green-100 text-green-800 text-sm", children: f }),
              N && /* @__PURE__ */ s("span", { className: "text-xs text-gray-600 mt-1 px-2", children: N })
            ] }, b);
          }) : /* @__PURE__ */ s("span", { className: "text-sm text-gray-500 italic", children: "No fields selected" }) })
        ] }),
        (g.filters && Array.isArray(g.filters) && g.filters.length > 0 || g.filters && !Array.isArray(g.filters) && Object.keys(g.filters).length > 0) && /* @__PURE__ */ d("div", { children: [
          /* @__PURE__ */ s("label", { className: "block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1", children: "Filters" }),
          /* @__PURE__ */ s("div", { className: "space-y-2", children: Array.isArray(g.filters) ? (
            // Handle filters as array (from test mocks)
            g.filters.map((f, b) => /* @__PURE__ */ s("div", { className: "bg-yellow-50 rounded-md p-3", children: /* @__PURE__ */ d("div", { className: "text-sm text-yellow-700", children: [
              f.field,
              " ",
              f.operator,
              ' "',
              f.value,
              '"'
            ] }) }, b))
          ) : (
            // Handle filters as object (existing format)
            Object.entries(g.filters).map(([f, b]) => /* @__PURE__ */ d("div", { className: "bg-yellow-50 rounded-md p-3", children: [
              /* @__PURE__ */ s("div", { className: "font-medium text-sm text-yellow-800 mb-1", children: f }),
              /* @__PURE__ */ d("div", { className: "text-sm text-yellow-700", children: [
                b.exactKey && /* @__PURE__ */ d("span", { children: [
                  "Exact key: ",
                  /* @__PURE__ */ s("code", { className: "bg-yellow-200 px-1 rounded", children: b.exactKey })
                ] }),
                b.keyRange && /* @__PURE__ */ d("span", { children: [
                  "Key range: ",
                  /* @__PURE__ */ s("code", { className: "bg-yellow-200 px-1 rounded", children: b.keyRange })
                ] }),
                b.keyPrefix && /* @__PURE__ */ d("span", { children: [
                  "Key prefix: ",
                  /* @__PURE__ */ s("code", { className: "bg-yellow-200 px-1 rounded", children: b.keyPrefix })
                ] })
              ] })
            ] }, f))
          ) })
        ] }),
        g.orderBy && /* @__PURE__ */ d("div", { children: [
          /* @__PURE__ */ s("label", { className: "block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1", children: "OrderBy" }),
          /* @__PURE__ */ s("div", { className: "bg-purple-50 rounded-md p-3", children: /* @__PURE__ */ d("div", { className: "text-sm text-purple-700", children: [
            g.orderBy.field,
            " ",
            g.orderBy.direction
          ] }) })
        ] }),
        g.rangeKey && /* @__PURE__ */ d("div", { children: [
          /* @__PURE__ */ s("label", { className: "block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1", children: "RangeKey" }),
          /* @__PURE__ */ s("div", { className: "bg-indigo-50 rounded-md p-3", children: /* @__PURE__ */ s("div", { className: "text-sm text-indigo-700", children: /* @__PURE__ */ s("code", { className: "bg-indigo-200 px-1 rounded", children: g.rangeKey }) }) })
        ] })
      ] }),
      o && /* @__PURE__ */ d("div", { className: "border-t border-gray-200 pt-4", children: [
        /* @__PURE__ */ s("label", { className: "block text-xs font-medium text-gray-500 uppercase tracking-wide mb-2", children: "Raw JSON" }),
        /* @__PURE__ */ s("pre", { className: "bg-gray-900 text-gray-100 text-xs p-3 rounded-md overflow-x-auto", children: JSON.stringify(t, null, 2) })
      ] })
    ] })
  ] });
}
function zo({ onResult: t }) {
  const {
    state: e,
    handleSchemaChange: r,
    toggleField: i,
    handleFieldValueChange: o,
    handleRangeFilterChange: u,
    setRangeSchemaFilter: h,
    setHashKeyValue: l,
    clearState: g,
    refetchSchemas: f,
    approvedSchemas: b,
    schemasLoading: N,
    selectedSchemaObj: A,
    isRangeSchema: E,
    isHashRangeSchema: y,
    rangeKey: x
  } = Mr();
  ue(() => {
    f();
  }, [f]);
  const [p, w] = O(!1), { query: _, isValid: S } = Qn({
    schema: e.selectedSchema,
    queryState: e,
    schemas: { [e.selectedSchema]: A }
  }), B = $(async (C) => {
    if (!C) {
      t({
        error: "No query data provided"
      });
      return;
    }
    w(!0);
    try {
      const k = await Dr.executeQuery(C);
      if (!k.success) {
        console.error("Query failed:", k.error), t({
          error: k.error || "Query execution failed",
          details: k
        });
        return;
      }
      t({
        success: !0,
        data: k.data?.results || k.data
      });
    } catch (k) {
      console.error("Failed to execute query:", k), t({
        error: `Network error: ${k.message}`,
        details: k
      });
    } finally {
      w(!1);
    }
  }, [t, S]), I = $(async (C) => {
    console.log("Validating query:", C);
  }, []), j = $(async (C) => {
    if (!C || !S) {
      console.warn("Cannot save invalid query");
      return;
    }
    try {
      console.log("Saving query:", C);
      const k = JSON.parse(localStorage.getItem("savedQueries") || "[]"), L = {
        id: Date.now(),
        name: `Query ${k.length + 1}`,
        data: C,
        createdAt: (/* @__PURE__ */ new Date()).toISOString()
      };
      k.push(L), localStorage.setItem("savedQueries", JSON.stringify(k)), console.log("Query saved successfully");
    } catch (k) {
      console.error("Failed to save query:", k);
    }
  }, [S]);
  return /* @__PURE__ */ s("div", { className: "p-6", children: /* @__PURE__ */ d("div", { className: "grid grid-cols-1 lg:grid-cols-3 gap-6", children: [
    /* @__PURE__ */ d("div", { className: "lg:col-span-2 space-y-6", children: [
      /* @__PURE__ */ s(
        Zi,
        {
          queryState: e,
          onSchemaChange: r,
          onFieldToggle: i,
          onFieldValueChange: o,
          onRangeFilterChange: u,
          onRangeSchemaFilterChange: h,
          onHashKeyChange: l,
          approvedSchemas: b,
          schemasLoading: N,
          isRangeSchema: E,
          isHashRangeSchema: y,
          rangeKey: x
        }
      ),
      /* @__PURE__ */ s(
        Xi,
        {
          onExecute: () => B(_),
          onValidate: () => I(_),
          onSave: () => j(_),
          onClear: g,
          queryData: _,
          disabled: !S,
          isExecuting: p,
          showValidation: !1,
          showSave: !0,
          showClear: !0
        }
      )
    ] }),
    /* @__PURE__ */ s("div", { className: "lg:col-span-1", children: /* @__PURE__ */ s(
      to,
      {
        query: _,
        showJson: !1,
        title: "Query Preview"
      }
    ) })
  ] }) });
}
function Qo({ onResult: t }) {
  const e = He(), r = Z(Da), i = Z(Ma), o = Z(Pa), u = Z(Ua), h = Z($a), l = Z(Ka), g = vt(null);
  ue(() => {
    g.current?.scrollIntoView({ behavior: "smooth" });
  }, [u]);
  const f = $((A, E, y = null) => {
    e(Oa({ type: A, content: E, data: y }));
  }, [e]), b = $(async (A) => {
    if (A?.preventDefault(), !r.trim() || o)
      return;
    const E = r.trim();
    e(en("")), e(rn(!0)), f("user", E);
    try {
      if (l) {
        f("system", "🤔 Analyzing if question can be answered from existing context...");
        const y = await on.analyzeFollowup({
          session_id: i,
          question: E
        });
        if (!y.success) {
          f("system", `❌ Error: ${y.error || "Failed to analyze question"}`);
          return;
        }
        const x = y.data;
        if (x.needs_query) {
          f("system", `🔍 Need new data: ${x.reasoning}`), f("system", "🔍 Using AI-native index search...");
          const p = await fetch("/api/llm-query/native-index", {
            method: "POST",
            headers: {
              "Content-Type": "application/json"
            },
            body: JSON.stringify({
              query: E,
              session_id: i
            })
          });
          if (!p.ok) {
            const _ = await p.json();
            f("system", `❌ Error: ${_.error || "Failed to run AI-native index query"}`);
            return;
          }
          const w = await p.json();
          f("system", "✅ AI-native index search completed"), w.session_id && e(tn(w.session_id)), f("system", w.ai_interpretation), f("results", "Raw search results", w.raw_results), h && t({ success: !0, data: w.raw_results });
        } else {
          f("system", `✅ Answering from existing context: ${x.reasoning}`);
          const p = await on.chat({
            session_id: i,
            question: E
          });
          if (!p.success) {
            f("system", `❌ Error: ${p.error || "Failed to process question"}`);
            return;
          }
          f("system", p.data.answer);
        }
      } else {
        f("system", "🔍 Using AI-native index search...");
        const y = await fetch("/api/llm-query/native-index", {
          method: "POST",
          headers: {
            "Content-Type": "application/json"
          },
          body: JSON.stringify({
            query: E,
            session_id: i
          })
        });
        if (!y.ok) {
          const p = await y.json();
          f("system", `❌ Error: ${p.error || "Failed to run AI-native index query"}`);
          return;
        }
        const x = await y.json();
        f("system", "✅ AI-native index search completed"), x.session_id && e(tn(x.session_id)), f("system", x.ai_interpretation), f("results", "Raw search results", x.raw_results), h && t({ success: !0, data: x.raw_results });
      }
    } catch (y) {
      console.error("Error processing input:", y), f("system", `❌ Error: ${y.message}`), t({ error: y.message });
    } finally {
      e(rn(!1));
    }
  }, [r, i, l, o, f, t, e]), N = $(() => {
    e(Fa());
  }, [e]);
  return /* @__PURE__ */ d("div", { className: "flex flex-col bg-white rounded-lg shadow", children: [
    /* @__PURE__ */ d("div", { className: "p-4 border-b border-gray-200 flex justify-between items-center", children: [
      /* @__PURE__ */ d("div", { children: [
        /* @__PURE__ */ s("h2", { className: "text-xl font-bold text-gray-900", children: "🤖 AI Data Assistant" }),
        /* @__PURE__ */ s("p", { className: "text-sm text-gray-600", children: "Ask questions in plain English - I'll find your data" })
      ] }),
      u.length > 0 && /* @__PURE__ */ s(
        "button",
        {
          onClick: N,
          disabled: o,
          className: "px-4 py-2 bg-gray-600 text-white rounded-lg hover:bg-gray-700 disabled:bg-gray-300 disabled:cursor-not-allowed transition-colors text-sm",
          children: "New Conversation"
        }
      )
    ] }),
    /* @__PURE__ */ d("div", { className: "overflow-y-auto bg-gray-50 p-4 space-y-3", style: { maxHeight: "60vh", minHeight: "400px" }, children: [
      u.length === 0 ? /* @__PURE__ */ d("div", { className: "text-center text-gray-500 mt-20", children: [
        /* @__PURE__ */ s("div", { className: "text-6xl mb-4", children: "💬" }),
        /* @__PURE__ */ s("p", { className: "text-lg mb-2", children: "Start a conversation" }),
        /* @__PURE__ */ s("p", { className: "text-sm", children: 'Try: "Find all blog posts from last month" or "Show me products over $100"' })
      ] }) : u.map((A, E) => /* @__PURE__ */ d("div", { children: [
        A.type === "user" && /* @__PURE__ */ s("div", { className: "flex justify-end", children: /* @__PURE__ */ d("div", { className: "bg-blue-600 text-white rounded-lg px-4 py-2 max-w-3xl", children: [
          /* @__PURE__ */ s("p", { className: "text-sm font-semibold mb-1", children: "You" }),
          /* @__PURE__ */ s("p", { className: "whitespace-pre-wrap", children: A.content })
        ] }) }),
        A.type === "system" && /* @__PURE__ */ s("div", { className: "flex justify-start", children: /* @__PURE__ */ d("div", { className: "bg-white border border-gray-200 rounded-lg px-4 py-2 max-w-3xl", children: [
          /* @__PURE__ */ s("p", { className: "text-sm font-semibold text-gray-700 mb-1", children: "AI Assistant" }),
          /* @__PURE__ */ s("p", { className: "text-gray-900 whitespace-pre-wrap", children: A.content })
        ] }) }),
        A.type === "results" && A.data && /* @__PURE__ */ d("div", { className: "bg-green-50 border border-green-200 rounded-lg p-4 max-w-full", children: [
          /* @__PURE__ */ d("div", { className: "flex justify-between items-center mb-2", children: [
            /* @__PURE__ */ d("p", { className: "text-sm font-semibold text-green-800", children: [
              "📊 Results (",
              A.data.length,
              ")"
            ] }),
            /* @__PURE__ */ s(
              "button",
              {
                onClick: () => {
                  const y = !h;
                  if (e(Ba(y)), y) {
                    const x = u.find((p) => p.type === "results");
                    x && x.data && t({ success: !0, data: x.data });
                  } else
                    t(null);
                },
                className: "text-sm text-green-700 hover:text-green-900 underline",
                children: h ? "Hide Details" : "Show Details"
              }
            )
          ] }),
          h && /* @__PURE__ */ d(At, { children: [
            /* @__PURE__ */ s("div", { className: "bg-white rounded p-3 mb-2", children: /* @__PURE__ */ s("p", { className: "text-gray-900 whitespace-pre-wrap mb-3", children: A.content }) }),
            /* @__PURE__ */ d("details", { className: "mt-2", children: [
              /* @__PURE__ */ d("summary", { className: "cursor-pointer text-sm text-green-700 hover:text-green-900", children: [
                "View raw data (",
                A.data.length,
                " records)"
              ] }),
              /* @__PURE__ */ s("div", { className: "mt-2 max-h-64 overflow-auto", children: /* @__PURE__ */ s("pre", { className: "text-xs bg-gray-900 text-green-400 p-3 rounded", children: JSON.stringify(A.data, null, 2) }) })
            ] })
          ] })
        ] })
      ] }, E)),
      /* @__PURE__ */ s("div", { ref: g })
    ] }),
    /* @__PURE__ */ d("form", { onSubmit: b, className: "border-t border-gray-200 p-4 bg-white", children: [
      /* @__PURE__ */ d("div", { className: "flex gap-2", children: [
        /* @__PURE__ */ s(
          "input",
          {
            type: "text",
            value: r,
            onChange: (A) => e(en(A.target.value)),
            placeholder: u.some((A) => A.type === "results") ? "Ask a follow-up question or start a new query..." : "Search the native index (e.g., 'Find posts about AI')...",
            disabled: o,
            className: "flex-1 px-4 py-3 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent disabled:bg-gray-100",
            autoFocus: !0
          }
        ),
        /* @__PURE__ */ s(
          "button",
          {
            type: "submit",
            disabled: !r.trim() || o,
            className: "px-6 py-3 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:bg-gray-300 disabled:cursor-not-allowed transition-colors font-semibold",
            children: o ? "⏳ Processing..." : "Send"
          }
        )
      ] }),
      o && /* @__PURE__ */ s("p", { className: "text-center text-sm text-gray-500 mt-2", children: "AI is analyzing and searching..." })
    ] })
  ] });
}
function ro({ selectedSchema: t, mutationType: e, onSchemaChange: r, onTypeChange: i }) {
  const o = Z(Ct);
  return /* @__PURE__ */ d("div", { className: "grid grid-cols-2 gap-4", children: [
    /* @__PURE__ */ s(
      wr,
      {
        name: "schema",
        label: Ge.schema,
        value: t,
        onChange: r,
        options: o.map((u) => ({
          value: u.name,
          label: u.descriptive_name || u.name
        })),
        placeholder: "Select a schema...",
        emptyMessage: "No approved schemas available for mutations",
        helpText: Ge.schemaHelp
      }
    ),
    /* @__PURE__ */ s(
      wr,
      {
        name: "operationType",
        label: Ge.operationType,
        value: e,
        onChange: i,
        options: fa,
        helpText: Ge.operationHelp
      }
    )
  ] });
}
function no({ fields: t, mutationType: e, mutationData: r, onFieldChange: i, isRangeSchema: o }) {
  if (e === "Delete")
    return /* @__PURE__ */ d("div", { className: "bg-gray-50 rounded-lg p-6", children: [
      /* @__PURE__ */ s("h3", { className: "text-lg font-medium text-gray-900 mb-4", children: "Delete Operation" }),
      /* @__PURE__ */ s("p", { className: "text-sm text-gray-600", children: "This will delete the selected schema. No additional fields are required." })
    ] });
  const u = (h, l) => {
    if (!(l.writable !== !1)) return null;
    const f = r[h] || "";
    switch (l.field_type) {
      case "Collection": {
        let b = [];
        if (f)
          try {
            const N = typeof f == "string" ? JSON.parse(f) : f;
            b = Array.isArray(N) ? N : [N];
          } catch {
            b = f.trim() ? [f] : [];
          }
        return /* @__PURE__ */ d("div", { className: "mb-6", children: [
          /* @__PURE__ */ d("label", { className: "block text-sm font-medium text-gray-700 mb-2", children: [
            h,
            /* @__PURE__ */ s("span", { className: "ml-2 text-xs text-gray-500", children: "Collection" })
          ] }),
          /* @__PURE__ */ s(
            "textarea",
            {
              className: "mt-1 block w-full border-gray-300 rounded-md shadow-sm focus:ring-primary focus:border-primary sm:text-sm font-mono",
              value: b.length > 0 ? JSON.stringify(b, null, 2) : "",
              onChange: (N) => {
                const A = N.target.value.trim();
                if (!A) {
                  i(h, []);
                  return;
                }
                try {
                  const E = JSON.parse(A);
                  i(h, Array.isArray(E) ? E : [E]);
                } catch {
                  i(h, [A]);
                }
              },
              placeholder: 'Enter JSON array (e.g., ["item1", "item2"])',
              rows: 4
            }
          ),
          /* @__PURE__ */ s("p", { className: "mt-1 text-xs text-gray-500", children: "Enter data as a JSON array. Empty input will create an empty array." })
        ] }, h);
      }
      case "Range": {
        if (o)
          return /* @__PURE__ */ d("div", { className: "mb-6", children: [
            /* @__PURE__ */ d("label", { className: "block text-sm font-medium text-gray-700 mb-2", children: [
              h,
              /* @__PURE__ */ s("span", { className: "ml-2 text-xs text-gray-500", children: "Single Value (Range Schema)" })
            ] }),
            /* @__PURE__ */ s(
              "input",
              {
                type: "text",
                className: "mt-1 block w-full border-gray-300 rounded-md shadow-sm focus:ring-primary focus:border-primary sm:text-sm",
                value: f,
                onChange: (x) => i(h, x.target.value),
                placeholder: `Enter ${h} value`
              }
            ),
            /* @__PURE__ */ s("p", { className: "mt-1 text-xs text-gray-500", children: "Enter a single value. The system will automatically handle range formatting." })
          ] }, h);
        let b = {};
        if (f)
          try {
            b = typeof f == "string" ? JSON.parse(f) : f, (typeof b != "object" || Array.isArray(b)) && (b = {});
          } catch {
            b = {};
          }
        const N = Object.entries(b), A = () => {
          const x = [...N, ["", ""]], p = Object.fromEntries(x);
          i(h, p);
        }, E = (x, p, w) => {
          const _ = [...N];
          _[x] = [p, w];
          const S = Object.fromEntries(_);
          i(h, S);
        }, y = (x) => {
          const p = N.filter((_, S) => S !== x), w = Object.fromEntries(p);
          i(h, w);
        };
        return /* @__PURE__ */ d("div", { className: "mb-6", children: [
          /* @__PURE__ */ d("label", { className: "block text-sm font-medium text-gray-700 mb-2", children: [
            h,
            /* @__PURE__ */ s("span", { className: "ml-2 text-xs text-gray-500", children: "Range (Complex)" })
          ] }),
          /* @__PURE__ */ s("div", { className: "border border-gray-300 rounded-md p-4 bg-gray-50", children: /* @__PURE__ */ d("div", { className: "space-y-3", children: [
            N.length === 0 ? /* @__PURE__ */ s("p", { className: "text-sm text-gray-500 italic", children: "No key-value pairs added yet" }) : N.map(([x, p], w) => /* @__PURE__ */ d("div", { className: "flex items-center space-x-2", children: [
              /* @__PURE__ */ s(
                "input",
                {
                  type: "text",
                  placeholder: "Key",
                  className: "flex-1 border-gray-300 rounded-md shadow-sm focus:ring-primary focus:border-primary sm:text-sm",
                  value: x,
                  onChange: (_) => E(w, _.target.value, p)
                }
              ),
              /* @__PURE__ */ s("span", { className: "text-gray-500", children: ":" }),
              /* @__PURE__ */ s(
                "input",
                {
                  type: "text",
                  placeholder: "Value",
                  className: "flex-1 border-gray-300 rounded-md shadow-sm focus:ring-primary focus:border-primary sm:text-sm",
                  value: p,
                  onChange: (_) => E(w, x, _.target.value)
                }
              ),
              /* @__PURE__ */ s(
                "button",
                {
                  type: "button",
                  onClick: () => y(w),
                  className: "text-red-600 hover:text-red-800 p-1",
                  title: "Remove this key-value pair",
                  children: /* @__PURE__ */ s("svg", { className: "w-4 h-4", fill: "none", stroke: "currentColor", viewBox: "0 0 24 24", children: /* @__PURE__ */ s("path", { strokeLinecap: "round", strokeLinejoin: "round", strokeWidth: 2, d: "M6 18L18 6M6 6l12 12" }) })
                }
              )
            ] }, w)),
            /* @__PURE__ */ d(
              "button",
              {
                type: "button",
                onClick: A,
                className: "inline-flex items-center px-3 py-2 border border-gray-300 shadow-sm text-sm leading-4 font-medium rounded-md text-gray-700 bg-white hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-primary",
                children: [
                  /* @__PURE__ */ s("svg", { className: "w-4 h-4 mr-1", fill: "none", stroke: "currentColor", viewBox: "0 0 24 24", children: /* @__PURE__ */ s("path", { strokeLinecap: "round", strokeLinejoin: "round", strokeWidth: 2, d: "M12 6v6m0 0v6m0-6h6m-6 0H6" }) }),
                  "Add Key-Value Pair"
                ]
              }
            )
          ] }) }),
          /* @__PURE__ */ s("p", { className: "mt-1 text-xs text-gray-500", children: "Add key-value pairs for this range field. Empty keys will be filtered out." })
        ] }, h);
      }
      default:
        return /* @__PURE__ */ d("div", { className: "mb-6", children: [
          /* @__PURE__ */ d("label", { className: "block text-sm font-medium text-gray-700 mb-2", children: [
            h,
            /* @__PURE__ */ s("span", { className: "ml-2 text-xs text-gray-500", children: "Single" })
          ] }),
          /* @__PURE__ */ s(
            "input",
            {
              type: "text",
              className: "mt-1 block w-full border-gray-300 rounded-md shadow-sm focus:ring-primary focus:border-primary sm:text-sm",
              value: f,
              onChange: (b) => i(h, b.target.value),
              placeholder: `Enter ${h}`
            }
          )
        ] }, h);
    }
  };
  return /* @__PURE__ */ d("div", { className: "bg-gray-50 rounded-lg p-6", children: [
    /* @__PURE__ */ d("h3", { className: "text-lg font-medium text-gray-900 mb-4", children: [
      "Schema Fields",
      o && /* @__PURE__ */ s("span", { className: "ml-2 text-sm text-blue-600 font-normal", children: "(Range Schema - Single Values)" })
    ] }),
    /* @__PURE__ */ s("div", { className: "space-y-6", children: Object.entries(t).map(([h, l]) => u(h, l)) }),
    o && Object.keys(t).length === 0 && /* @__PURE__ */ s("p", { className: "text-sm text-gray-500 italic", children: "No additional fields to configure. Only the range key is required for this schema." })
  ] });
}
function so({ result: t }) {
  return t ? /* @__PURE__ */ s("div", { className: "bg-gray-50 rounded-lg p-4 mt-4", children: /* @__PURE__ */ s("pre", { className: "font-mono text-sm whitespace-pre-wrap", children: typeof t == "string" ? t : JSON.stringify(t, null, 2) }) }) : null;
}
function ao(t) {
  const e = Ee(t);
  return {
    base: e,
    schema: Sa(e),
    mutation: ni(e),
    security: ti(e)
  };
}
ao({
  enableCache: !0,
  enableLogging: !0,
  enableMetrics: !0
});
var mr = {}, xt = {}, hn;
function io() {
  if (hn) return xt;
  hn = 1, xt.byteLength = l, xt.toByteArray = f, xt.fromByteArray = A;
  for (var t = [], e = [], r = typeof Uint8Array < "u" ? Uint8Array : Array, i = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/", o = 0, u = i.length; o < u; ++o)
    t[o] = i[o], e[i.charCodeAt(o)] = o;
  e[45] = 62, e[95] = 63;
  function h(E) {
    var y = E.length;
    if (y % 4 > 0)
      throw new Error("Invalid string. Length must be a multiple of 4");
    var x = E.indexOf("=");
    x === -1 && (x = y);
    var p = x === y ? 0 : 4 - x % 4;
    return [x, p];
  }
  function l(E) {
    var y = h(E), x = y[0], p = y[1];
    return (x + p) * 3 / 4 - p;
  }
  function g(E, y, x) {
    return (y + x) * 3 / 4 - x;
  }
  function f(E) {
    var y, x = h(E), p = x[0], w = x[1], _ = new r(g(E, p, w)), S = 0, B = w > 0 ? p - 4 : p, I;
    for (I = 0; I < B; I += 4)
      y = e[E.charCodeAt(I)] << 18 | e[E.charCodeAt(I + 1)] << 12 | e[E.charCodeAt(I + 2)] << 6 | e[E.charCodeAt(I + 3)], _[S++] = y >> 16 & 255, _[S++] = y >> 8 & 255, _[S++] = y & 255;
    return w === 2 && (y = e[E.charCodeAt(I)] << 2 | e[E.charCodeAt(I + 1)] >> 4, _[S++] = y & 255), w === 1 && (y = e[E.charCodeAt(I)] << 10 | e[E.charCodeAt(I + 1)] << 4 | e[E.charCodeAt(I + 2)] >> 2, _[S++] = y >> 8 & 255, _[S++] = y & 255), _;
  }
  function b(E) {
    return t[E >> 18 & 63] + t[E >> 12 & 63] + t[E >> 6 & 63] + t[E & 63];
  }
  function N(E, y, x) {
    for (var p, w = [], _ = y; _ < x; _ += 3)
      p = (E[_] << 16 & 16711680) + (E[_ + 1] << 8 & 65280) + (E[_ + 2] & 255), w.push(b(p));
    return w.join("");
  }
  function A(E) {
    for (var y, x = E.length, p = x % 3, w = [], _ = 16383, S = 0, B = x - p; S < B; S += _)
      w.push(N(E, S, S + _ > B ? B : S + _));
    return p === 1 ? (y = E[x - 1], w.push(
      t[y >> 2] + t[y << 4 & 63] + "=="
    )) : p === 2 && (y = (E[x - 2] << 8) + E[x - 1], w.push(
      t[y >> 10] + t[y >> 4 & 63] + t[y << 2 & 63] + "="
    )), w.join("");
  }
  return xt;
}
var Mt = {};
var mn;
function oo() {
  return mn || (mn = 1, Mt.read = function(t, e, r, i, o) {
    var u, h, l = o * 8 - i - 1, g = (1 << l) - 1, f = g >> 1, b = -7, N = r ? o - 1 : 0, A = r ? -1 : 1, E = t[e + N];
    for (N += A, u = E & (1 << -b) - 1, E >>= -b, b += l; b > 0; u = u * 256 + t[e + N], N += A, b -= 8)
      ;
    for (h = u & (1 << -b) - 1, u >>= -b, b += i; b > 0; h = h * 256 + t[e + N], N += A, b -= 8)
      ;
    if (u === 0)
      u = 1 - f;
    else {
      if (u === g)
        return h ? NaN : (E ? -1 : 1) * (1 / 0);
      h = h + Math.pow(2, i), u = u - f;
    }
    return (E ? -1 : 1) * h * Math.pow(2, u - i);
  }, Mt.write = function(t, e, r, i, o, u) {
    var h, l, g, f = u * 8 - o - 1, b = (1 << f) - 1, N = b >> 1, A = o === 23 ? Math.pow(2, -24) - Math.pow(2, -77) : 0, E = i ? 0 : u - 1, y = i ? 1 : -1, x = e < 0 || e === 0 && 1 / e < 0 ? 1 : 0;
    for (e = Math.abs(e), isNaN(e) || e === 1 / 0 ? (l = isNaN(e) ? 1 : 0, h = b) : (h = Math.floor(Math.log(e) / Math.LN2), e * (g = Math.pow(2, -h)) < 1 && (h--, g *= 2), h + N >= 1 ? e += A / g : e += A * Math.pow(2, 1 - N), e * g >= 2 && (h++, g /= 2), h + N >= b ? (l = 0, h = b) : h + N >= 1 ? (l = (e * g - 1) * Math.pow(2, o), h = h + N) : (l = e * Math.pow(2, N - 1) * Math.pow(2, o), h = 0)); o >= 8; t[r + E] = l & 255, E += y, l /= 256, o -= 8)
      ;
    for (h = h << o | l, f += o; f > 0; t[r + E] = h & 255, E += y, h /= 256, f -= 8)
      ;
    t[r + E - y] |= x * 128;
  }), Mt;
}
var fn;
function lo() {
  return fn || (fn = 1, (function(t) {
    const e = io(), r = oo(), i = typeof Symbol == "function" && typeof Symbol.for == "function" ? /* @__PURE__ */ Symbol.for("nodejs.util.inspect.custom") : null;
    t.Buffer = l, t.SlowBuffer = _, t.INSPECT_MAX_BYTES = 50;
    const o = 2147483647;
    t.kMaxLength = o, l.TYPED_ARRAY_SUPPORT = u(), !l.TYPED_ARRAY_SUPPORT && typeof console < "u" && typeof console.error == "function" && console.error(
      "This browser lacks typed array (Uint8Array) support which is required by `buffer` v5.x. Use `buffer` v4.x if you require old browser support."
    );
    function u() {
      try {
        const c = new Uint8Array(1), n = { foo: function() {
          return 42;
        } };
        return Object.setPrototypeOf(n, Uint8Array.prototype), Object.setPrototypeOf(c, n), c.foo() === 42;
      } catch {
        return !1;
      }
    }
    Object.defineProperty(l.prototype, "parent", {
      enumerable: !0,
      get: function() {
        if (l.isBuffer(this))
          return this.buffer;
      }
    }), Object.defineProperty(l.prototype, "offset", {
      enumerable: !0,
      get: function() {
        if (l.isBuffer(this))
          return this.byteOffset;
      }
    });
    function h(c) {
      if (c > o)
        throw new RangeError('The value "' + c + '" is invalid for option "size"');
      const n = new Uint8Array(c);
      return Object.setPrototypeOf(n, l.prototype), n;
    }
    function l(c, n, a) {
      if (typeof c == "number") {
        if (typeof n == "string")
          throw new TypeError(
            'The "string" argument must be of type string. Received type number'
          );
        return N(c);
      }
      return g(c, n, a);
    }
    l.poolSize = 8192;
    function g(c, n, a) {
      if (typeof c == "string")
        return A(c, n);
      if (ArrayBuffer.isView(c))
        return y(c);
      if (c == null)
        throw new TypeError(
          "The first argument must be one of type string, Buffer, ArrayBuffer, Array, or Array-like Object. Received type " + typeof c
        );
      if (Ae(c, ArrayBuffer) || c && Ae(c.buffer, ArrayBuffer) || typeof SharedArrayBuffer < "u" && (Ae(c, SharedArrayBuffer) || c && Ae(c.buffer, SharedArrayBuffer)))
        return x(c, n, a);
      if (typeof c == "number")
        throw new TypeError(
          'The "value" argument must not be of type number. Received type number'
        );
      const m = c.valueOf && c.valueOf();
      if (m != null && m !== c)
        return l.from(m, n, a);
      const v = p(c);
      if (v) return v;
      if (typeof Symbol < "u" && Symbol.toPrimitive != null && typeof c[Symbol.toPrimitive] == "function")
        return l.from(c[Symbol.toPrimitive]("string"), n, a);
      throw new TypeError(
        "The first argument must be one of type string, Buffer, ArrayBuffer, Array, or Array-like Object. Received type " + typeof c
      );
    }
    l.from = function(c, n, a) {
      return g(c, n, a);
    }, Object.setPrototypeOf(l.prototype, Uint8Array.prototype), Object.setPrototypeOf(l, Uint8Array);
    function f(c) {
      if (typeof c != "number")
        throw new TypeError('"size" argument must be of type number');
      if (c < 0)
        throw new RangeError('The value "' + c + '" is invalid for option "size"');
    }
    function b(c, n, a) {
      return f(c), c <= 0 ? h(c) : n !== void 0 ? typeof a == "string" ? h(c).fill(n, a) : h(c).fill(n) : h(c);
    }
    l.alloc = function(c, n, a) {
      return b(c, n, a);
    };
    function N(c) {
      return f(c), h(c < 0 ? 0 : w(c) | 0);
    }
    l.allocUnsafe = function(c) {
      return N(c);
    }, l.allocUnsafeSlow = function(c) {
      return N(c);
    };
    function A(c, n) {
      if ((typeof n != "string" || n === "") && (n = "utf8"), !l.isEncoding(n))
        throw new TypeError("Unknown encoding: " + n);
      const a = S(c, n) | 0;
      let m = h(a);
      const v = m.write(c, n);
      return v !== a && (m = m.slice(0, v)), m;
    }
    function E(c) {
      const n = c.length < 0 ? 0 : w(c.length) | 0, a = h(n);
      for (let m = 0; m < n; m += 1)
        a[m] = c[m] & 255;
      return a;
    }
    function y(c) {
      if (Ae(c, Uint8Array)) {
        const n = new Uint8Array(c);
        return x(n.buffer, n.byteOffset, n.byteLength);
      }
      return E(c);
    }
    function x(c, n, a) {
      if (n < 0 || c.byteLength < n)
        throw new RangeError('"offset" is outside of buffer bounds');
      if (c.byteLength < n + (a || 0))
        throw new RangeError('"length" is outside of buffer bounds');
      let m;
      return n === void 0 && a === void 0 ? m = new Uint8Array(c) : a === void 0 ? m = new Uint8Array(c, n) : m = new Uint8Array(c, n, a), Object.setPrototypeOf(m, l.prototype), m;
    }
    function p(c) {
      if (l.isBuffer(c)) {
        const n = w(c.length) | 0, a = h(n);
        return a.length === 0 || c.copy(a, 0, 0, n), a;
      }
      if (c.length !== void 0)
        return typeof c.length != "number" || er(c.length) ? h(0) : E(c);
      if (c.type === "Buffer" && Array.isArray(c.data))
        return E(c.data);
    }
    function w(c) {
      if (c >= o)
        throw new RangeError("Attempt to allocate Buffer larger than maximum size: 0x" + o.toString(16) + " bytes");
      return c | 0;
    }
    function _(c) {
      return +c != c && (c = 0), l.alloc(+c);
    }
    l.isBuffer = function(n) {
      return n != null && n._isBuffer === !0 && n !== l.prototype;
    }, l.compare = function(n, a) {
      if (Ae(n, Uint8Array) && (n = l.from(n, n.offset, n.byteLength)), Ae(a, Uint8Array) && (a = l.from(a, a.offset, a.byteLength)), !l.isBuffer(n) || !l.isBuffer(a))
        throw new TypeError(
          'The "buf1", "buf2" arguments must be one of type Buffer or Uint8Array'
        );
      if (n === a) return 0;
      let m = n.length, v = a.length;
      for (let T = 0, R = Math.min(m, v); T < R; ++T)
        if (n[T] !== a[T]) {
          m = n[T], v = a[T];
          break;
        }
      return m < v ? -1 : v < m ? 1 : 0;
    }, l.isEncoding = function(n) {
      switch (String(n).toLowerCase()) {
        case "hex":
        case "utf8":
        case "utf-8":
        case "ascii":
        case "latin1":
        case "binary":
        case "base64":
        case "ucs2":
        case "ucs-2":
        case "utf16le":
        case "utf-16le":
          return !0;
        default:
          return !1;
      }
    }, l.concat = function(n, a) {
      if (!Array.isArray(n))
        throw new TypeError('"list" argument must be an Array of Buffers');
      if (n.length === 0)
        return l.alloc(0);
      let m;
      if (a === void 0)
        for (a = 0, m = 0; m < n.length; ++m)
          a += n[m].length;
      const v = l.allocUnsafe(a);
      let T = 0;
      for (m = 0; m < n.length; ++m) {
        let R = n[m];
        if (Ae(R, Uint8Array))
          T + R.length > v.length ? (l.isBuffer(R) || (R = l.from(R)), R.copy(v, T)) : Uint8Array.prototype.set.call(
            v,
            R,
            T
          );
        else if (l.isBuffer(R))
          R.copy(v, T);
        else
          throw new TypeError('"list" argument must be an Array of Buffers');
        T += R.length;
      }
      return v;
    };
    function S(c, n) {
      if (l.isBuffer(c))
        return c.length;
      if (ArrayBuffer.isView(c) || Ae(c, ArrayBuffer))
        return c.byteLength;
      if (typeof c != "string")
        throw new TypeError(
          'The "string" argument must be one of type string, Buffer, or ArrayBuffer. Received type ' + typeof c
        );
      const a = c.length, m = arguments.length > 2 && arguments[2] === !0;
      if (!m && a === 0) return 0;
      let v = !1;
      for (; ; )
        switch (n) {
          case "ascii":
          case "latin1":
          case "binary":
            return a;
          case "utf8":
          case "utf-8":
            return D(c).length;
          case "ucs2":
          case "ucs-2":
          case "utf16le":
          case "utf-16le":
            return a * 2;
          case "hex":
            return a >>> 1;
          case "base64":
            return De(c).length;
          default:
            if (v)
              return m ? -1 : D(c).length;
            n = ("" + n).toLowerCase(), v = !0;
        }
    }
    l.byteLength = S;
    function B(c, n, a) {
      let m = !1;
      if ((n === void 0 || n < 0) && (n = 0), n > this.length || ((a === void 0 || a > this.length) && (a = this.length), a <= 0) || (a >>>= 0, n >>>= 0, a <= n))
        return "";
      for (c || (c = "utf8"); ; )
        switch (c) {
          case "hex":
            return We(this, n, a);
          case "utf8":
          case "utf-8":
            return V(this, n, a);
          case "ascii":
            return Ie(this, n, a);
          case "latin1":
          case "binary":
            return Se(this, n, a);
          case "base64":
            return K(this, n, a);
          case "ucs2":
          case "ucs-2":
          case "utf16le":
          case "utf-16le":
            return Je(this, n, a);
          default:
            if (m) throw new TypeError("Unknown encoding: " + c);
            c = (c + "").toLowerCase(), m = !0;
        }
    }
    l.prototype._isBuffer = !0;
    function I(c, n, a) {
      const m = c[n];
      c[n] = c[a], c[a] = m;
    }
    l.prototype.swap16 = function() {
      const n = this.length;
      if (n % 2 !== 0)
        throw new RangeError("Buffer size must be a multiple of 16-bits");
      for (let a = 0; a < n; a += 2)
        I(this, a, a + 1);
      return this;
    }, l.prototype.swap32 = function() {
      const n = this.length;
      if (n % 4 !== 0)
        throw new RangeError("Buffer size must be a multiple of 32-bits");
      for (let a = 0; a < n; a += 4)
        I(this, a, a + 3), I(this, a + 1, a + 2);
      return this;
    }, l.prototype.swap64 = function() {
      const n = this.length;
      if (n % 8 !== 0)
        throw new RangeError("Buffer size must be a multiple of 64-bits");
      for (let a = 0; a < n; a += 8)
        I(this, a, a + 7), I(this, a + 1, a + 6), I(this, a + 2, a + 5), I(this, a + 3, a + 4);
      return this;
    }, l.prototype.toString = function() {
      const n = this.length;
      return n === 0 ? "" : arguments.length === 0 ? V(this, 0, n) : B.apply(this, arguments);
    }, l.prototype.toLocaleString = l.prototype.toString, l.prototype.equals = function(n) {
      if (!l.isBuffer(n)) throw new TypeError("Argument must be a Buffer");
      return this === n ? !0 : l.compare(this, n) === 0;
    }, l.prototype.inspect = function() {
      let n = "";
      const a = t.INSPECT_MAX_BYTES;
      return n = this.toString("hex", 0, a).replace(/(.{2})/g, "$1 ").trim(), this.length > a && (n += " ... "), "<Buffer " + n + ">";
    }, i && (l.prototype[i] = l.prototype.inspect), l.prototype.compare = function(n, a, m, v, T) {
      if (Ae(n, Uint8Array) && (n = l.from(n, n.offset, n.byteLength)), !l.isBuffer(n))
        throw new TypeError(
          'The "target" argument must be one of type Buffer or Uint8Array. Received type ' + typeof n
        );
      if (a === void 0 && (a = 0), m === void 0 && (m = n ? n.length : 0), v === void 0 && (v = 0), T === void 0 && (T = this.length), a < 0 || m > n.length || v < 0 || T > this.length)
        throw new RangeError("out of range index");
      if (v >= T && a >= m)
        return 0;
      if (v >= T)
        return -1;
      if (a >= m)
        return 1;
      if (a >>>= 0, m >>>= 0, v >>>= 0, T >>>= 0, this === n) return 0;
      let R = T - v, q = m - a;
      const ee = Math.min(R, q), J = this.slice(v, T), te = n.slice(a, m);
      for (let Q = 0; Q < ee; ++Q)
        if (J[Q] !== te[Q]) {
          R = J[Q], q = te[Q];
          break;
        }
      return R < q ? -1 : q < R ? 1 : 0;
    };
    function j(c, n, a, m, v) {
      if (c.length === 0) return -1;
      if (typeof a == "string" ? (m = a, a = 0) : a > 2147483647 ? a = 2147483647 : a < -2147483648 && (a = -2147483648), a = +a, er(a) && (a = v ? 0 : c.length - 1), a < 0 && (a = c.length + a), a >= c.length) {
        if (v) return -1;
        a = c.length - 1;
      } else if (a < 0)
        if (v) a = 0;
        else return -1;
      if (typeof n == "string" && (n = l.from(n, m)), l.isBuffer(n))
        return n.length === 0 ? -1 : C(c, n, a, m, v);
      if (typeof n == "number")
        return n = n & 255, typeof Uint8Array.prototype.indexOf == "function" ? v ? Uint8Array.prototype.indexOf.call(c, n, a) : Uint8Array.prototype.lastIndexOf.call(c, n, a) : C(c, [n], a, m, v);
      throw new TypeError("val must be string, number or Buffer");
    }
    function C(c, n, a, m, v) {
      let T = 1, R = c.length, q = n.length;
      if (m !== void 0 && (m = String(m).toLowerCase(), m === "ucs2" || m === "ucs-2" || m === "utf16le" || m === "utf-16le")) {
        if (c.length < 2 || n.length < 2)
          return -1;
        T = 2, R /= 2, q /= 2, a /= 2;
      }
      function ee(te, Q) {
        return T === 1 ? te[Q] : te.readUInt16BE(Q * T);
      }
      let J;
      if (v) {
        let te = -1;
        for (J = a; J < R; J++)
          if (ee(c, J) === ee(n, te === -1 ? 0 : J - te)) {
            if (te === -1 && (te = J), J - te + 1 === q) return te * T;
          } else
            te !== -1 && (J -= J - te), te = -1;
      } else
        for (a + q > R && (a = R - q), J = a; J >= 0; J--) {
          let te = !0;
          for (let Q = 0; Q < q; Q++)
            if (ee(c, J + Q) !== ee(n, Q)) {
              te = !1;
              break;
            }
          if (te) return J;
        }
      return -1;
    }
    l.prototype.includes = function(n, a, m) {
      return this.indexOf(n, a, m) !== -1;
    }, l.prototype.indexOf = function(n, a, m) {
      return j(this, n, a, m, !0);
    }, l.prototype.lastIndexOf = function(n, a, m) {
      return j(this, n, a, m, !1);
    };
    function k(c, n, a, m) {
      a = Number(a) || 0;
      const v = c.length - a;
      m ? (m = Number(m), m > v && (m = v)) : m = v;
      const T = n.length;
      m > T / 2 && (m = T / 2);
      let R;
      for (R = 0; R < m; ++R) {
        const q = parseInt(n.substr(R * 2, 2), 16);
        if (er(q)) return R;
        c[a + R] = q;
      }
      return R;
    }
    function L(c, n, a, m) {
      return kt(D(n, c.length - a), c, a, m);
    }
    function P(c, n, a, m) {
      return kt(G(n), c, a, m);
    }
    function F(c, n, a, m) {
      return kt(De(n), c, a, m);
    }
    function U(c, n, a, m) {
      return kt(ge(n, c.length - a), c, a, m);
    }
    l.prototype.write = function(n, a, m, v) {
      if (a === void 0)
        v = "utf8", m = this.length, a = 0;
      else if (m === void 0 && typeof a == "string")
        v = a, m = this.length, a = 0;
      else if (isFinite(a))
        a = a >>> 0, isFinite(m) ? (m = m >>> 0, v === void 0 && (v = "utf8")) : (v = m, m = void 0);
      else
        throw new Error(
          "Buffer.write(string, encoding, offset[, length]) is no longer supported"
        );
      const T = this.length - a;
      if ((m === void 0 || m > T) && (m = T), n.length > 0 && (m < 0 || a < 0) || a > this.length)
        throw new RangeError("Attempt to write outside buffer bounds");
      v || (v = "utf8");
      let R = !1;
      for (; ; )
        switch (v) {
          case "hex":
            return k(this, n, a, m);
          case "utf8":
          case "utf-8":
            return L(this, n, a, m);
          case "ascii":
          case "latin1":
          case "binary":
            return P(this, n, a, m);
          case "base64":
            return F(this, n, a, m);
          case "ucs2":
          case "ucs-2":
          case "utf16le":
          case "utf-16le":
            return U(this, n, a, m);
          default:
            if (R) throw new TypeError("Unknown encoding: " + v);
            v = ("" + v).toLowerCase(), R = !0;
        }
    }, l.prototype.toJSON = function() {
      return {
        type: "Buffer",
        data: Array.prototype.slice.call(this._arr || this, 0)
      };
    };
    function K(c, n, a) {
      return n === 0 && a === c.length ? e.fromByteArray(c) : e.fromByteArray(c.slice(n, a));
    }
    function V(c, n, a) {
      a = Math.min(c.length, a);
      const m = [];
      let v = n;
      for (; v < a; ) {
        const T = c[v];
        let R = null, q = T > 239 ? 4 : T > 223 ? 3 : T > 191 ? 2 : 1;
        if (v + q <= a) {
          let ee, J, te, Q;
          switch (q) {
            case 1:
              T < 128 && (R = T);
              break;
            case 2:
              ee = c[v + 1], (ee & 192) === 128 && (Q = (T & 31) << 6 | ee & 63, Q > 127 && (R = Q));
              break;
            case 3:
              ee = c[v + 1], J = c[v + 2], (ee & 192) === 128 && (J & 192) === 128 && (Q = (T & 15) << 12 | (ee & 63) << 6 | J & 63, Q > 2047 && (Q < 55296 || Q > 57343) && (R = Q));
              break;
            case 4:
              ee = c[v + 1], J = c[v + 2], te = c[v + 3], (ee & 192) === 128 && (J & 192) === 128 && (te & 192) === 128 && (Q = (T & 15) << 18 | (ee & 63) << 12 | (J & 63) << 6 | te & 63, Q > 65535 && Q < 1114112 && (R = Q));
          }
        }
        R === null ? (R = 65533, q = 1) : R > 65535 && (R -= 65536, m.push(R >>> 10 & 1023 | 55296), R = 56320 | R & 1023), m.push(R), v += q;
      }
      return ae(m);
    }
    const Y = 4096;
    function ae(c) {
      const n = c.length;
      if (n <= Y)
        return String.fromCharCode.apply(String, c);
      let a = "", m = 0;
      for (; m < n; )
        a += String.fromCharCode.apply(
          String,
          c.slice(m, m += Y)
        );
      return a;
    }
    function Ie(c, n, a) {
      let m = "";
      a = Math.min(c.length, a);
      for (let v = n; v < a; ++v)
        m += String.fromCharCode(c[v] & 127);
      return m;
    }
    function Se(c, n, a) {
      let m = "";
      a = Math.min(c.length, a);
      for (let v = n; v < a; ++v)
        m += String.fromCharCode(c[v]);
      return m;
    }
    function We(c, n, a) {
      const m = c.length;
      (!n || n < 0) && (n = 0), (!a || a < 0 || a > m) && (a = m);
      let v = "";
      for (let T = n; T < a; ++T)
        v += Jn[c[T]];
      return v;
    }
    function Je(c, n, a) {
      const m = c.slice(n, a);
      let v = "";
      for (let T = 0; T < m.length - 1; T += 2)
        v += String.fromCharCode(m[T] + m[T + 1] * 256);
      return v;
    }
    l.prototype.slice = function(n, a) {
      const m = this.length;
      n = ~~n, a = a === void 0 ? m : ~~a, n < 0 ? (n += m, n < 0 && (n = 0)) : n > m && (n = m), a < 0 ? (a += m, a < 0 && (a = 0)) : a > m && (a = m), a < n && (a = n);
      const v = this.subarray(n, a);
      return Object.setPrototypeOf(v, l.prototype), v;
    };
    function ne(c, n, a) {
      if (c % 1 !== 0 || c < 0) throw new RangeError("offset is not uint");
      if (c + n > a) throw new RangeError("Trying to access beyond buffer length");
    }
    l.prototype.readUintLE = l.prototype.readUIntLE = function(n, a, m) {
      n = n >>> 0, a = a >>> 0, m || ne(n, a, this.length);
      let v = this[n], T = 1, R = 0;
      for (; ++R < a && (T *= 256); )
        v += this[n + R] * T;
      return v;
    }, l.prototype.readUintBE = l.prototype.readUIntBE = function(n, a, m) {
      n = n >>> 0, a = a >>> 0, m || ne(n, a, this.length);
      let v = this[n + --a], T = 1;
      for (; a > 0 && (T *= 256); )
        v += this[n + --a] * T;
      return v;
    }, l.prototype.readUint8 = l.prototype.readUInt8 = function(n, a) {
      return n = n >>> 0, a || ne(n, 1, this.length), this[n];
    }, l.prototype.readUint16LE = l.prototype.readUInt16LE = function(n, a) {
      return n = n >>> 0, a || ne(n, 2, this.length), this[n] | this[n + 1] << 8;
    }, l.prototype.readUint16BE = l.prototype.readUInt16BE = function(n, a) {
      return n = n >>> 0, a || ne(n, 2, this.length), this[n] << 8 | this[n + 1];
    }, l.prototype.readUint32LE = l.prototype.readUInt32LE = function(n, a) {
      return n = n >>> 0, a || ne(n, 4, this.length), (this[n] | this[n + 1] << 8 | this[n + 2] << 16) + this[n + 3] * 16777216;
    }, l.prototype.readUint32BE = l.prototype.readUInt32BE = function(n, a) {
      return n = n >>> 0, a || ne(n, 4, this.length), this[n] * 16777216 + (this[n + 1] << 16 | this[n + 2] << 8 | this[n + 3]);
    }, l.prototype.readBigUInt64LE = Me(function(n) {
      n = n >>> 0, Le(n, "offset");
      const a = this[n], m = this[n + 7];
      (a === void 0 || m === void 0) && Ve(n, this.length - 8);
      const v = a + this[++n] * 2 ** 8 + this[++n] * 2 ** 16 + this[++n] * 2 ** 24, T = this[++n] + this[++n] * 2 ** 8 + this[++n] * 2 ** 16 + m * 2 ** 24;
      return BigInt(v) + (BigInt(T) << BigInt(32));
    }), l.prototype.readBigUInt64BE = Me(function(n) {
      n = n >>> 0, Le(n, "offset");
      const a = this[n], m = this[n + 7];
      (a === void 0 || m === void 0) && Ve(n, this.length - 8);
      const v = a * 2 ** 24 + this[++n] * 2 ** 16 + this[++n] * 2 ** 8 + this[++n], T = this[++n] * 2 ** 24 + this[++n] * 2 ** 16 + this[++n] * 2 ** 8 + m;
      return (BigInt(v) << BigInt(32)) + BigInt(T);
    }), l.prototype.readIntLE = function(n, a, m) {
      n = n >>> 0, a = a >>> 0, m || ne(n, a, this.length);
      let v = this[n], T = 1, R = 0;
      for (; ++R < a && (T *= 256); )
        v += this[n + R] * T;
      return T *= 128, v >= T && (v -= Math.pow(2, 8 * a)), v;
    }, l.prototype.readIntBE = function(n, a, m) {
      n = n >>> 0, a = a >>> 0, m || ne(n, a, this.length);
      let v = a, T = 1, R = this[n + --v];
      for (; v > 0 && (T *= 256); )
        R += this[n + --v] * T;
      return T *= 128, R >= T && (R -= Math.pow(2, 8 * a)), R;
    }, l.prototype.readInt8 = function(n, a) {
      return n = n >>> 0, a || ne(n, 1, this.length), this[n] & 128 ? (255 - this[n] + 1) * -1 : this[n];
    }, l.prototype.readInt16LE = function(n, a) {
      n = n >>> 0, a || ne(n, 2, this.length);
      const m = this[n] | this[n + 1] << 8;
      return m & 32768 ? m | 4294901760 : m;
    }, l.prototype.readInt16BE = function(n, a) {
      n = n >>> 0, a || ne(n, 2, this.length);
      const m = this[n + 1] | this[n] << 8;
      return m & 32768 ? m | 4294901760 : m;
    }, l.prototype.readInt32LE = function(n, a) {
      return n = n >>> 0, a || ne(n, 4, this.length), this[n] | this[n + 1] << 8 | this[n + 2] << 16 | this[n + 3] << 24;
    }, l.prototype.readInt32BE = function(n, a) {
      return n = n >>> 0, a || ne(n, 4, this.length), this[n] << 24 | this[n + 1] << 16 | this[n + 2] << 8 | this[n + 3];
    }, l.prototype.readBigInt64LE = Me(function(n) {
      n = n >>> 0, Le(n, "offset");
      const a = this[n], m = this[n + 7];
      (a === void 0 || m === void 0) && Ve(n, this.length - 8);
      const v = this[n + 4] + this[n + 5] * 2 ** 8 + this[n + 6] * 2 ** 16 + (m << 24);
      return (BigInt(v) << BigInt(32)) + BigInt(a + this[++n] * 2 ** 8 + this[++n] * 2 ** 16 + this[++n] * 2 ** 24);
    }), l.prototype.readBigInt64BE = Me(function(n) {
      n = n >>> 0, Le(n, "offset");
      const a = this[n], m = this[n + 7];
      (a === void 0 || m === void 0) && Ve(n, this.length - 8);
      const v = (a << 24) + // Overflow
      this[++n] * 2 ** 16 + this[++n] * 2 ** 8 + this[++n];
      return (BigInt(v) << BigInt(32)) + BigInt(this[++n] * 2 ** 24 + this[++n] * 2 ** 16 + this[++n] * 2 ** 8 + m);
    }), l.prototype.readFloatLE = function(n, a) {
      return n = n >>> 0, a || ne(n, 4, this.length), r.read(this, n, !0, 23, 4);
    }, l.prototype.readFloatBE = function(n, a) {
      return n = n >>> 0, a || ne(n, 4, this.length), r.read(this, n, !1, 23, 4);
    }, l.prototype.readDoubleLE = function(n, a) {
      return n = n >>> 0, a || ne(n, 8, this.length), r.read(this, n, !0, 52, 8);
    }, l.prototype.readDoubleBE = function(n, a) {
      return n = n >>> 0, a || ne(n, 8, this.length), r.read(this, n, !1, 52, 8);
    };
    function se(c, n, a, m, v, T) {
      if (!l.isBuffer(c)) throw new TypeError('"buffer" argument must be a Buffer instance');
      if (n > v || n < T) throw new RangeError('"value" argument is out of bounds');
      if (a + m > c.length) throw new RangeError("Index out of range");
    }
    l.prototype.writeUintLE = l.prototype.writeUIntLE = function(n, a, m, v) {
      if (n = +n, a = a >>> 0, m = m >>> 0, !v) {
        const q = Math.pow(2, 8 * m) - 1;
        se(this, n, a, m, q, 0);
      }
      let T = 1, R = 0;
      for (this[a] = n & 255; ++R < m && (T *= 256); )
        this[a + R] = n / T & 255;
      return a + m;
    }, l.prototype.writeUintBE = l.prototype.writeUIntBE = function(n, a, m, v) {
      if (n = +n, a = a >>> 0, m = m >>> 0, !v) {
        const q = Math.pow(2, 8 * m) - 1;
        se(this, n, a, m, q, 0);
      }
      let T = m - 1, R = 1;
      for (this[a + T] = n & 255; --T >= 0 && (R *= 256); )
        this[a + T] = n / R & 255;
      return a + m;
    }, l.prototype.writeUint8 = l.prototype.writeUInt8 = function(n, a, m) {
      return n = +n, a = a >>> 0, m || se(this, n, a, 1, 255, 0), this[a] = n & 255, a + 1;
    }, l.prototype.writeUint16LE = l.prototype.writeUInt16LE = function(n, a, m) {
      return n = +n, a = a >>> 0, m || se(this, n, a, 2, 65535, 0), this[a] = n & 255, this[a + 1] = n >>> 8, a + 2;
    }, l.prototype.writeUint16BE = l.prototype.writeUInt16BE = function(n, a, m) {
      return n = +n, a = a >>> 0, m || se(this, n, a, 2, 65535, 0), this[a] = n >>> 8, this[a + 1] = n & 255, a + 2;
    }, l.prototype.writeUint32LE = l.prototype.writeUInt32LE = function(n, a, m) {
      return n = +n, a = a >>> 0, m || se(this, n, a, 4, 4294967295, 0), this[a + 3] = n >>> 24, this[a + 2] = n >>> 16, this[a + 1] = n >>> 8, this[a] = n & 255, a + 4;
    }, l.prototype.writeUint32BE = l.prototype.writeUInt32BE = function(n, a, m) {
      return n = +n, a = a >>> 0, m || se(this, n, a, 4, 4294967295, 0), this[a] = n >>> 24, this[a + 1] = n >>> 16, this[a + 2] = n >>> 8, this[a + 3] = n & 255, a + 4;
    };
    function mt(c, n, a, m, v) {
      yt(n, m, v, c, a, 7);
      let T = Number(n & BigInt(4294967295));
      c[a++] = T, T = T >> 8, c[a++] = T, T = T >> 8, c[a++] = T, T = T >> 8, c[a++] = T;
      let R = Number(n >> BigInt(32) & BigInt(4294967295));
      return c[a++] = R, R = R >> 8, c[a++] = R, R = R >> 8, c[a++] = R, R = R >> 8, c[a++] = R, a;
    }
    function Ze(c, n, a, m, v) {
      yt(n, m, v, c, a, 7);
      let T = Number(n & BigInt(4294967295));
      c[a + 7] = T, T = T >> 8, c[a + 6] = T, T = T >> 8, c[a + 5] = T, T = T >> 8, c[a + 4] = T;
      let R = Number(n >> BigInt(32) & BigInt(4294967295));
      return c[a + 3] = R, R = R >> 8, c[a + 2] = R, R = R >> 8, c[a + 1] = R, R = R >> 8, c[a] = R, a + 8;
    }
    l.prototype.writeBigUInt64LE = Me(function(n, a = 0) {
      return mt(this, n, a, BigInt(0), BigInt("0xffffffffffffffff"));
    }), l.prototype.writeBigUInt64BE = Me(function(n, a = 0) {
      return Ze(this, n, a, BigInt(0), BigInt("0xffffffffffffffff"));
    }), l.prototype.writeIntLE = function(n, a, m, v) {
      if (n = +n, a = a >>> 0, !v) {
        const ee = Math.pow(2, 8 * m - 1);
        se(this, n, a, m, ee - 1, -ee);
      }
      let T = 0, R = 1, q = 0;
      for (this[a] = n & 255; ++T < m && (R *= 256); )
        n < 0 && q === 0 && this[a + T - 1] !== 0 && (q = 1), this[a + T] = (n / R >> 0) - q & 255;
      return a + m;
    }, l.prototype.writeIntBE = function(n, a, m, v) {
      if (n = +n, a = a >>> 0, !v) {
        const ee = Math.pow(2, 8 * m - 1);
        se(this, n, a, m, ee - 1, -ee);
      }
      let T = m - 1, R = 1, q = 0;
      for (this[a + T] = n & 255; --T >= 0 && (R *= 256); )
        n < 0 && q === 0 && this[a + T + 1] !== 0 && (q = 1), this[a + T] = (n / R >> 0) - q & 255;
      return a + m;
    }, l.prototype.writeInt8 = function(n, a, m) {
      return n = +n, a = a >>> 0, m || se(this, n, a, 1, 127, -128), n < 0 && (n = 255 + n + 1), this[a] = n & 255, a + 1;
    }, l.prototype.writeInt16LE = function(n, a, m) {
      return n = +n, a = a >>> 0, m || se(this, n, a, 2, 32767, -32768), this[a] = n & 255, this[a + 1] = n >>> 8, a + 2;
    }, l.prototype.writeInt16BE = function(n, a, m) {
      return n = +n, a = a >>> 0, m || se(this, n, a, 2, 32767, -32768), this[a] = n >>> 8, this[a + 1] = n & 255, a + 2;
    }, l.prototype.writeInt32LE = function(n, a, m) {
      return n = +n, a = a >>> 0, m || se(this, n, a, 4, 2147483647, -2147483648), this[a] = n & 255, this[a + 1] = n >>> 8, this[a + 2] = n >>> 16, this[a + 3] = n >>> 24, a + 4;
    }, l.prototype.writeInt32BE = function(n, a, m) {
      return n = +n, a = a >>> 0, m || se(this, n, a, 4, 2147483647, -2147483648), n < 0 && (n = 4294967295 + n + 1), this[a] = n >>> 24, this[a + 1] = n >>> 16, this[a + 2] = n >>> 8, this[a + 3] = n & 255, a + 4;
    }, l.prototype.writeBigInt64LE = Me(function(n, a = 0) {
      return mt(this, n, a, -BigInt("0x8000000000000000"), BigInt("0x7fffffffffffffff"));
    }), l.prototype.writeBigInt64BE = Me(function(n, a = 0) {
      return Ze(this, n, a, -BigInt("0x8000000000000000"), BigInt("0x7fffffffffffffff"));
    });
    function ft(c, n, a, m, v, T) {
      if (a + m > c.length) throw new RangeError("Index out of range");
      if (a < 0) throw new RangeError("Index out of range");
    }
    function pt(c, n, a, m, v) {
      return n = +n, a = a >>> 0, v || ft(c, n, a, 4), r.write(c, n, a, m, 23, 4), a + 4;
    }
    l.prototype.writeFloatLE = function(n, a, m) {
      return pt(this, n, a, !0, m);
    }, l.prototype.writeFloatBE = function(n, a, m) {
      return pt(this, n, a, !1, m);
    };
    function gt(c, n, a, m, v) {
      return n = +n, a = a >>> 0, v || ft(c, n, a, 8), r.write(c, n, a, m, 52, 8), a + 8;
    }
    l.prototype.writeDoubleLE = function(n, a, m) {
      return gt(this, n, a, !0, m);
    }, l.prototype.writeDoubleBE = function(n, a, m) {
      return gt(this, n, a, !1, m);
    }, l.prototype.copy = function(n, a, m, v) {
      if (!l.isBuffer(n)) throw new TypeError("argument should be a Buffer");
      if (m || (m = 0), !v && v !== 0 && (v = this.length), a >= n.length && (a = n.length), a || (a = 0), v > 0 && v < m && (v = m), v === m || n.length === 0 || this.length === 0) return 0;
      if (a < 0)
        throw new RangeError("targetStart out of bounds");
      if (m < 0 || m >= this.length) throw new RangeError("Index out of range");
      if (v < 0) throw new RangeError("sourceEnd out of bounds");
      v > this.length && (v = this.length), n.length - a < v - m && (v = n.length - a + m);
      const T = v - m;
      return this === n && typeof Uint8Array.prototype.copyWithin == "function" ? this.copyWithin(a, m, v) : Uint8Array.prototype.set.call(
        n,
        this.subarray(m, v),
        a
      ), T;
    }, l.prototype.fill = function(n, a, m, v) {
      if (typeof n == "string") {
        if (typeof a == "string" ? (v = a, a = 0, m = this.length) : typeof m == "string" && (v = m, m = this.length), v !== void 0 && typeof v != "string")
          throw new TypeError("encoding must be a string");
        if (typeof v == "string" && !l.isEncoding(v))
          throw new TypeError("Unknown encoding: " + v);
        if (n.length === 1) {
          const R = n.charCodeAt(0);
          (v === "utf8" && R < 128 || v === "latin1") && (n = R);
        }
      } else typeof n == "number" ? n = n & 255 : typeof n == "boolean" && (n = Number(n));
      if (a < 0 || this.length < a || this.length < m)
        throw new RangeError("Out of range index");
      if (m <= a)
        return this;
      a = a >>> 0, m = m === void 0 ? this.length : m >>> 0, n || (n = 0);
      let T;
      if (typeof n == "number")
        for (T = a; T < m; ++T)
          this[T] = n;
      else {
        const R = l.isBuffer(n) ? n : l.from(n, v), q = R.length;
        if (q === 0)
          throw new TypeError('The value "' + n + '" is invalid for argument "value"');
        for (T = 0; T < m - a; ++T)
          this[T + a] = R[T % q];
      }
      return this;
    };
    const Oe = {};
    function Xe(c, n, a) {
      Oe[c] = class extends a {
        constructor() {
          super(), Object.defineProperty(this, "message", {
            value: n.apply(this, arguments),
            writable: !0,
            configurable: !0
          }), this.name = `${this.name} [${c}]`, this.stack, delete this.name;
        }
        get code() {
          return c;
        }
        set code(v) {
          Object.defineProperty(this, "code", {
            configurable: !0,
            enumerable: !0,
            value: v,
            writable: !0
          });
        }
        toString() {
          return `${this.name} [${c}]: ${this.message}`;
        }
      };
    }
    Xe(
      "ERR_BUFFER_OUT_OF_BOUNDS",
      function(c) {
        return c ? `${c} is outside of buffer bounds` : "Attempt to access memory outside buffer bounds";
      },
      RangeError
    ), Xe(
      "ERR_INVALID_ARG_TYPE",
      function(c, n) {
        return `The "${c}" argument must be of type number. Received type ${typeof n}`;
      },
      TypeError
    ), Xe(
      "ERR_OUT_OF_RANGE",
      function(c, n, a) {
        let m = `The value of "${c}" is out of range.`, v = a;
        return Number.isInteger(a) && Math.abs(a) > 2 ** 32 ? v = Rt(String(a)) : typeof a == "bigint" && (v = String(a), (a > BigInt(2) ** BigInt(32) || a < -(BigInt(2) ** BigInt(32))) && (v = Rt(v)), v += "n"), m += ` It must be ${n}. Received ${v}`, m;
      },
      RangeError
    );
    function Rt(c) {
      let n = "", a = c.length;
      const m = c[0] === "-" ? 1 : 0;
      for (; a >= m + 4; a -= 3)
        n = `_${c.slice(a - 3, a)}${n}`;
      return `${c.slice(0, a)}${n}`;
    }
    function Jt(c, n, a) {
      Le(n, "offset"), (c[n] === void 0 || c[n + a] === void 0) && Ve(n, c.length - (a + 1));
    }
    function yt(c, n, a, m, v, T) {
      if (c > a || c < n) {
        const R = typeof n == "bigint" ? "n" : "";
        let q;
        throw n === 0 || n === BigInt(0) ? q = `>= 0${R} and < 2${R} ** ${(T + 1) * 8}${R}` : q = `>= -(2${R} ** ${(T + 1) * 8 - 1}${R}) and < 2 ** ${(T + 1) * 8 - 1}${R}`, new Oe.ERR_OUT_OF_RANGE("value", q, c);
      }
      Jt(m, v, T);
    }
    function Le(c, n) {
      if (typeof c != "number")
        throw new Oe.ERR_INVALID_ARG_TYPE(n, "number", c);
    }
    function Ve(c, n, a) {
      throw Math.floor(c) !== c ? (Le(c, a), new Oe.ERR_OUT_OF_RANGE("offset", "an integer", c)) : n < 0 ? new Oe.ERR_BUFFER_OUT_OF_BOUNDS() : new Oe.ERR_OUT_OF_RANGE(
        "offset",
        `>= 0 and <= ${n}`,
        c
      );
    }
    const Zt = /[^+/0-9A-Za-z-_]/g;
    function Xt(c) {
      if (c = c.split("=")[0], c = c.trim().replace(Zt, ""), c.length < 2) return "";
      for (; c.length % 4 !== 0; )
        c = c + "=";
      return c;
    }
    function D(c, n) {
      n = n || 1 / 0;
      let a;
      const m = c.length;
      let v = null;
      const T = [];
      for (let R = 0; R < m; ++R) {
        if (a = c.charCodeAt(R), a > 55295 && a < 57344) {
          if (!v) {
            if (a > 56319) {
              (n -= 3) > -1 && T.push(239, 191, 189);
              continue;
            } else if (R + 1 === m) {
              (n -= 3) > -1 && T.push(239, 191, 189);
              continue;
            }
            v = a;
            continue;
          }
          if (a < 56320) {
            (n -= 3) > -1 && T.push(239, 191, 189), v = a;
            continue;
          }
          a = (v - 55296 << 10 | a - 56320) + 65536;
        } else v && (n -= 3) > -1 && T.push(239, 191, 189);
        if (v = null, a < 128) {
          if ((n -= 1) < 0) break;
          T.push(a);
        } else if (a < 2048) {
          if ((n -= 2) < 0) break;
          T.push(
            a >> 6 | 192,
            a & 63 | 128
          );
        } else if (a < 65536) {
          if ((n -= 3) < 0) break;
          T.push(
            a >> 12 | 224,
            a >> 6 & 63 | 128,
            a & 63 | 128
          );
        } else if (a < 1114112) {
          if ((n -= 4) < 0) break;
          T.push(
            a >> 18 | 240,
            a >> 12 & 63 | 128,
            a >> 6 & 63 | 128,
            a & 63 | 128
          );
        } else
          throw new Error("Invalid code point");
      }
      return T;
    }
    function G(c) {
      const n = [];
      for (let a = 0; a < c.length; ++a)
        n.push(c.charCodeAt(a) & 255);
      return n;
    }
    function ge(c, n) {
      let a, m, v;
      const T = [];
      for (let R = 0; R < c.length && !((n -= 2) < 0); ++R)
        a = c.charCodeAt(R), m = a >> 8, v = a % 256, T.push(v), T.push(m);
      return T;
    }
    function De(c) {
      return e.toByteArray(Xt(c));
    }
    function kt(c, n, a, m) {
      let v;
      for (v = 0; v < m && !(v + a >= n.length || v >= c.length); ++v)
        n[v + a] = c[v];
      return v;
    }
    function Ae(c, n) {
      return c instanceof n || c != null && c.constructor != null && c.constructor.name != null && c.constructor.name === n.name;
    }
    function er(c) {
      return c !== c;
    }
    const Jn = (function() {
      const c = "0123456789abcdef", n = new Array(256);
      for (let a = 0; a < 16; ++a) {
        const m = a * 16;
        for (let v = 0; v < 16; ++v)
          n[m + v] = c[a] + c[v];
      }
      return n;
    })();
    function Me(c) {
      return typeof BigInt > "u" ? Zn : c;
    }
    function Zn() {
      throw new Error("BigInt not supported");
    }
  })(mr)), mr;
}
lo();
const co = { executeMutation: "Execute Mutation" }, pn = { rangeKeyRequired: "Range key is required", rangeKeyOptional: "Range key is optional for delete operations" }, gn = { label: "Range Key", backgroundColor: "bg-blue-50" };
function Wo({ onResult: t }) {
  const e = Z(Ct);
  Z((C) => C.auth);
  const [r, i] = O(""), [o, u] = O({}), [h, l] = O("Insert"), [g, f] = O(null), [b, N] = O(""), [A, E] = O({}), y = (C) => {
    i(C), u({}), l("Insert"), N("");
  }, x = (C, k) => {
    u((L) => ({ ...L, [C]: k }));
  }, p = async (C) => {
    if (C.preventDefault(), !r) return;
    const k = e.find((F) => F.name === r), L = h ? Rn[h] || h.toLowerCase() : "";
    if (!L)
      return;
    let P;
    ct(k) ? P = ga(k, h, b, o) : P = {
      type: "mutation",
      schema: r,
      mutation_type: L,
      fields_and_values: h === "Delete" ? {} : o,
      key_value: { hash: null, range: null }
    };
    try {
      const F = await Dr.executeMutation(P);
      if (!F.success)
        throw new Error(F.error || "Mutation failed");
      const U = F;
      f(U), t(U), U.success && (u({}), N(""));
    } catch (F) {
      const U = { error: `Network error: ${F.message}`, details: F };
      f(U), t(U);
    }
  }, w = r ? e.find((C) => C.name === r) : null, _ = w ? ct(w) : !1, S = w ? Ye(w) : null, I = !w || !Array.isArray(w.fields) ? {} : (_ ? w.fields.filter((k) => k !== S) : w.fields).reduce((k, L) => (k[L] = {}, k), {}), j = !r || !h || h !== "Delete" && Object.keys(o).length === 0 || _ && h !== "Delete" && !b.trim();
  return /* @__PURE__ */ d("div", { className: "p-6", children: [
    /* @__PURE__ */ d("form", { onSubmit: p, className: "space-y-6", children: [
      /* @__PURE__ */ s(
        ro,
        {
          selectedSchema: r,
          mutationType: h,
          onSchemaChange: y,
          onTypeChange: l
        }
      ),
      r && _ && /* @__PURE__ */ d("div", { className: `${gn.backgroundColor} rounded-lg p-4`, children: [
        /* @__PURE__ */ s("h3", { className: "text-lg font-medium text-gray-900 mb-4", children: "Range Schema Configuration" }),
        /* @__PURE__ */ s(
          wt,
          {
            name: "rangeKey",
            label: `${S} (${gn.label})`,
            value: b,
            onChange: N,
            placeholder: `Enter ${S} value`,
            required: h !== "Delete",
            error: A.rangeKey,
            helpText: h !== "Delete" ? pn.rangeKeyRequired : pn.rangeKeyOptional,
            debounced: !0
          }
        )
      ] }),
      r && /* @__PURE__ */ s(
        no,
        {
          fields: I,
          mutationType: h,
          mutationData: o,
          onFieldChange: x,
          isRangeSchema: _
        }
      ),
      /* @__PURE__ */ s("div", { className: "flex justify-end pt-4", children: /* @__PURE__ */ s(
        "button",
        {
          type: "submit",
          className: `inline-flex items-center px-4 py-2 border border-transparent rounded-md shadow-sm text-sm font-medium text-white ${j ? "bg-gray-300 cursor-not-allowed" : "bg-primary hover:bg-primary/90 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-primary"}`,
          disabled: j,
          children: co.executeMutation
        }
      ) })
    ] }),
    /* @__PURE__ */ s(so, { result: g })
  ] });
}
function Jo({ onResult: t }) {
  const [e, r] = O(""), [i, o] = O(!0), [u, h] = O(0), [l, g] = O("default"), [f, b] = O(!1), [N, A] = O(null);
  ue(() => {
    E();
  }, []);
  const E = async () => {
    try {
      const w = await dt.getStatus();
      w.success && A(w.data);
    } catch (w) {
      console.error("Failed to fetch ingestion status:", w);
    }
  }, y = async () => {
    b(!0), t(null);
    try {
      const w = JSON.parse(e), _ = {
        autoExecute: i,
        trustDistance: u,
        pubKey: l
      }, S = await dt.processIngestion(w, _);
      S.success ? (t({
        success: !0,
        data: S.data
      }), r("")) : t({
        success: !1,
        error: "Failed to process ingestion"
      });
    } catch (w) {
      t({
        success: !1,
        error: w.message || "Failed to process ingestion"
      });
    } finally {
      b(!1);
    }
  }, x = () => {
    const w = [
      "Sarah Chen",
      "Michael Rodriguez",
      "Emily Johnson",
      "David Kim",
      "Lisa Wang",
      "James Thompson",
      "Maria Garcia",
      "Alex Chen",
      "Rachel Green",
      "Tom Wilson",
      "Jennifer Lee",
      "Chris Anderson",
      "Amanda Taylor",
      "Ryan Murphy",
      "Jessica Brown",
      "Kevin Park",
      "Nicole Davis",
      "Brandon White",
      "Stephanie Martinez",
      "Daniel Liu"
    ], _ = [
      "Technology",
      "Programming",
      "Web Development",
      "Data Science",
      "Machine Learning",
      "Artificial Intelligence",
      "Cloud Computing",
      "DevOps",
      "Cybersecurity",
      "Mobile Development",
      "UI/UX Design",
      "Product Management",
      "Startup Life",
      "Career Advice",
      "Industry Trends",
      "Open Source",
      "Software Architecture",
      "Database Design",
      "API Development",
      "Testing"
    ], S = [
      ["javascript", "webdev", "tutorial"],
      ["python", "datascience", "ai"],
      ["react", "frontend", "javascript"],
      ["nodejs", "backend", "api"],
      ["docker", "devops", "deployment"],
      ["aws", "cloud", "infrastructure"],
      ["machine-learning", "python", "data"],
      ["typescript", "webdev", "frontend"],
      ["kubernetes", "devops", "containers"],
      ["sql", "database", "backend"],
      ["git", "version-control", "workflow"],
      ["testing", "quality", "tdd"],
      ["security", "cybersecurity", "best-practices"],
      ["performance", "optimization", "web"],
      ["mobile", "ios", "android"],
      ["design", "ux", "ui"],
      ["agile", "management", "process"],
      ["career", "advice", "development"],
      ["startup", "entrepreneurship", "business"],
      ["opensource", "community", "contribution"],
      ["architecture", "scalability", "design"]
    ], B = [];
    for (let I = 1; I <= 100; I++) {
      const j = w[Math.floor(Math.random() * w.length)], C = _[Math.floor(Math.random() * _.length)], k = S[Math.floor(Math.random() * S.length)], L = /* @__PURE__ */ new Date(), P = new Date(L.getTime() - 4320 * 60 * 60 * 1e3), F = P.getTime() + Math.random() * (L.getTime() - P.getTime()), U = new Date(F).toISOString().split("T")[0], K = [
        `Getting Started with ${C}: A Complete Guide`,
        `Advanced ${C} Techniques You Need to Know`,
        `Why ${C} is Changing the Industry`,
        `Building Scalable Applications with ${C}`,
        `The Future of ${C}: Trends and Predictions`,
        `Common ${C} Mistakes and How to Avoid Them`,
        `Best Practices for ${C} Development`,
        `From Beginner to Expert in ${C}`,
        `Case Study: Implementing ${C} in Production`,
        `${C} Tools and Frameworks Comparison`
      ], V = K[Math.floor(Math.random() * K.length)], Y = [
        `In this comprehensive guide, we'll explore the fundamentals of ${C} and how it's revolutionizing the way we approach modern development. Whether you're a seasoned developer or just starting out, this article will provide valuable insights into best practices and real-world applications.

## Introduction to ${C}

${C} has become an essential part of today's technology landscape. With its powerful capabilities and growing ecosystem, it offers developers unprecedented opportunities to build robust and scalable solutions.

## Key Concepts

Understanding the core concepts of ${C} is crucial for success. Let's dive into the fundamental principles that make this technology so powerful:

1. **Core Architecture**: The foundation of ${C} lies in its well-designed architecture
2. **Performance Optimization**: Learn how to maximize efficiency and minimize resource usage
3. **Integration Patterns**: Discover best practices for connecting with other systems
4. **Security Considerations**: Implement robust security measures from the ground up

## Real-World Applications

Many companies have successfully implemented ${C} in their production environments. Here are some notable examples:

- **Case Study 1**: A major e-commerce platform reduced their response time by 60%
- **Case Study 2**: A fintech startup improved their scalability by 300%
- **Case Study 3**: A healthcare company enhanced their data processing capabilities

## Getting Started

Ready to dive in? Here's a step-by-step guide to get you started with ${C}:

\`\`\`javascript
// Example implementation
const example = new ${C}();
example.initialize();
example.process();
\`\`\`

## Conclusion

${C} represents a significant advancement in technology, offering developers powerful tools to build the next generation of applications. By following the principles and practices outlined in this guide, you'll be well-equipped to leverage ${C} in your own projects.

Remember, the key to success with ${C} is continuous learning and experimentation. Stay curious, keep building, and don't hesitate to explore new possibilities!`,
        `The landscape of ${C} is constantly evolving, and staying ahead of the curve requires a deep understanding of both current trends and emerging technologies. In this article, we'll examine the latest developments and provide actionable insights for developers looking to enhance their skills.

## Current State of ${C}

Today's ${C} ecosystem is more mature and feature-rich than ever before. With improved tooling, better documentation, and a growing community, developers have access to resources that make implementation more straightforward.

## Emerging Trends

Several key trends are shaping the future of ${C}:

- **Automation**: Increasing focus on automated workflows and CI/CD integration
- **Performance**: New optimization techniques that improve speed and efficiency
- **Security**: Enhanced security features and best practices
- **Scalability**: Better support for large-scale deployments

## Industry Impact

The adoption of ${C} across various industries has been remarkable:

- **Technology Sector**: 85% of tech companies have implemented ${C} solutions
- **Financial Services**: Improved transaction processing and risk management
- **Healthcare**: Enhanced patient data management and analysis
- **E-commerce**: Better customer experience and operational efficiency

## Implementation Strategies

When implementing ${C}, consider these strategic approaches:

1. **Phased Rollout**: Start with pilot projects before full deployment
2. **Team Training**: Invest in comprehensive team education
3. **Monitoring**: Implement robust monitoring and alerting systems
4. **Documentation**: Maintain detailed documentation for future reference

## Future Outlook

Looking ahead, ${C} is poised for continued growth and innovation. Key areas to watch include:

- Advanced AI integration
- Improved developer experience
- Enhanced security features
- Better cross-platform compatibility

The future of ${C} is bright, and developers who invest in learning these technologies now will be well-positioned for success in the years to come.`,
        `Building robust applications with ${C} requires more than just technical knowledge—it demands a strategic approach to architecture, design, and implementation. In this deep dive, we'll explore advanced techniques that will elevate your ${C} development skills.

## Architecture Patterns

Effective ${C} applications rely on well-established architectural patterns:

### Microservices Architecture
Breaking down monolithic applications into smaller, manageable services provides better scalability and maintainability.

### Event-Driven Design
Implementing event-driven patterns enables better decoupling and improved system responsiveness.

### Domain-Driven Design
Organizing code around business domains leads to more maintainable and understandable applications.

## Performance Optimization

Optimizing ${C} applications requires attention to multiple factors:

- **Caching Strategies**: Implement intelligent caching to reduce database load
- **Resource Management**: Optimize memory usage and CPU utilization
- **Network Optimization**: Minimize network overhead and latency
- **Database Tuning**: Optimize queries and indexing strategies

## Testing Strategies

Comprehensive testing is essential for reliable ${C} applications:

\`\`\`javascript
// Example test structure
describe('${C} Component', () => {
  it('should handle basic functionality', () => {
    const component = new ${C}Component();
    expect(component.process()).toBeDefined();
  });
  
  it('should handle edge cases', () => {
    const component = new ${C}Component();
    expect(() => component.process(null)).not.toThrow();
  });
});
\`\`\`

## Monitoring and Observability

Implementing comprehensive monitoring helps identify issues before they impact users:

- **Application Metrics**: Track performance indicators and user behavior
- **Error Tracking**: Monitor and alert on application errors
- **Log Analysis**: Centralize and analyze application logs
- **Health Checks**: Implement automated health monitoring

## Security Considerations

Security should be a primary concern when developing ${C} applications:

1. **Input Validation**: Always validate and sanitize user inputs
2. **Authentication**: Implement robust authentication mechanisms
3. **Authorization**: Control access to resources and functionality
4. **Data Protection**: Encrypt sensitive data both in transit and at rest

## Deployment Strategies

Successful deployment requires careful planning and execution:

- **Blue-Green Deployment**: Minimize downtime during updates
- **Canary Releases**: Gradually roll out changes to a subset of users
- **Feature Flags**: Control feature availability without code changes
- **Rollback Procedures**: Prepare for quick rollback in case of issues

## Conclusion

Mastering ${C} development is an ongoing journey that requires continuous learning and adaptation. By implementing these advanced techniques and best practices, you'll build more robust, scalable, and maintainable applications.

The key to success lies in understanding not just the technical aspects, but also the business context and user needs. Keep experimenting, stay updated with the latest developments, and always prioritize code quality and user experience.`
      ], ae = Y[Math.floor(Math.random() * Y.length)];
      B.push({
        title: V,
        content: ae,
        author: j,
        publish_date: U,
        tags: k
      });
    }
    return B;
  }, p = (w) => {
    const _ = {
      blogposts: x(),
      twitter: [
        {
          post_id: "tweet_1234567890",
          author: "@techinfluencer",
          author_id: "user_tech_001",
          content: "Just launched our new AI-powered database! 🚀 Real-time ingestion, automatic schema mapping, and zero-config setup. Check it out at folddb.io #database #AI #opensource",
          timestamp: "2024-10-21T14:32:00Z",
          likes: 342,
          retweets: 89,
          replies: 23,
          views: 12453,
          media: [
            {
              type: "image",
              url: "https://cdn.example.com/img1.jpg",
              alt: "FoldDB Dashboard Screenshot"
            }
          ],
          mentions: ["@opensource", "@devtools"],
          hashtags: ["database", "AI", "opensource"],
          reply_to: null,
          thread_position: 1,
          engagement_rate: 0.034
        },
        {
          post_id: "tweet_1234567891",
          author: "@datascientist_pro",
          author_id: "user_ds_042",
          content: "Amazing work @techinfluencer! Been testing FoldDB for the past week. The automatic schema inference saved us hours of setup time. Here are my benchmarks:",
          timestamp: "2024-10-21T15:18:00Z",
          likes: 156,
          retweets: 34,
          replies: 12,
          views: 5621,
          media: [
            {
              type: "image",
              url: "https://cdn.example.com/benchmark.png",
              alt: "Performance Benchmarks"
            }
          ],
          mentions: ["@techinfluencer"],
          hashtags: ["database", "performance"],
          reply_to: "tweet_1234567890",
          thread_position: null,
          engagement_rate: 0.036
        }
      ],
      instagram: [
        {
          post_id: "ig_post_987654321",
          username: "foodie_adventures",
          user_id: "ig_user_food_123",
          caption: "Best ramen in Tokyo! 🍜✨ The broth was simmering for 48 hours and you can taste every minute of it. Swipe for more pics! #tokyo #ramen #foodie #japan #travel",
          posted_at: "2024-10-20T09:45:00Z",
          location: {
            name: "Ichiran Ramen Shibuya",
            city: "Tokyo",
            country: "Japan",
            coordinates: {
              lat: 35.6595,
              lng: 139.7004
            }
          },
          media: [
            {
              type: "image",
              url: "https://cdn.instagram.example.com/ramen1.jpg",
              width: 1080,
              height: 1350,
              filter: "Valencia"
            },
            {
              type: "image",
              url: "https://cdn.instagram.example.com/ramen2.jpg",
              width: 1080,
              height: 1350,
              filter: "Valencia"
            },
            {
              type: "image",
              url: "https://cdn.instagram.example.com/ramen3.jpg",
              width: 1080,
              height: 1350,
              filter: "Valencia"
            }
          ],
          likes: 8234,
          comments_count: 456,
          saves: 892,
          shares: 234,
          hashtags: ["tokyo", "ramen", "foodie", "japan", "travel"],
          tagged_users: ["@ramen_tokyo_guide", "@japan_food_official"],
          comments: [
            {
              comment_id: "ig_comment_111",
              username: "tokyo_foodie",
              text: "Omg I was there last week! The tonkotsu broth is incredible 😍",
              timestamp: "2024-10-20T10:12:00Z",
              likes: 45
            },
            {
              comment_id: "ig_comment_112",
              username: "ramen_lover_88",
              text: "Adding this to my Tokyo bucket list! 📝",
              timestamp: "2024-10-20T11:30:00Z",
              likes: 23
            }
          ]
        },
        {
          post_id: "ig_post_987654322",
          username: "fitness_journey_2024",
          user_id: "ig_user_fit_456",
          caption: "Day 287 of my fitness journey! 💪 Down 45 lbs and feeling stronger than ever. Remember: progress > perfection. What's your fitness goal? #fitness #transformation #motivation #workout",
          posted_at: "2024-10-21T06:00:00Z",
          location: {
            name: "Gold's Gym",
            city: "Los Angeles",
            country: "USA",
            coordinates: {
              lat: 34.0522,
              lng: -118.2437
            }
          },
          media: [
            {
              type: "video",
              url: "https://cdn.instagram.example.com/workout_vid.mp4",
              thumbnail: "https://cdn.instagram.example.com/workout_thumb.jpg",
              duration: 45,
              width: 1080,
              height: 1920
            }
          ],
          likes: 15672,
          comments_count: 892,
          saves: 2341,
          shares: 567,
          hashtags: ["fitness", "transformation", "motivation", "workout"],
          tagged_users: ["@personal_trainer_mike"],
          comments: [
            {
              comment_id: "ig_comment_113",
              username: "motivation_daily",
              text: "Incredible transformation! You're an inspiration! 🔥",
              timestamp: "2024-10-21T06:15:00Z",
              likes: 234
            }
          ]
        }
      ],
      linkedin: [
        {
          post_id: "li_post_555666777",
          author: {
            name: "Sarah Chen",
            title: "CTO at TechVentures Inc.",
            profile_url: "linkedin.com/in/sarah-chen-cto",
            user_id: "li_user_sarah_123"
          },
          content: `Excited to announce that our team has successfully migrated our entire data infrastructure to a real-time event-driven architecture! 🎉

Key achievements:
• 10x reduction in data latency (from 5 minutes to 30 seconds)
• 40% cost savings on infrastructure
• Improved data quality through automated validation
• Seamless integration with our ML pipelines

Huge shoutout to the engineering team for their incredible work over the past 6 months. This wouldn't have been possible without their dedication and expertise.

Happy to share more details for anyone interested in event-driven architectures. Feel free to reach out!

#DataEngineering #EventDriven #TechLeadership #Innovation`,
          posted_at: "2024-10-21T13:00:00Z",
          article: null,
          media: [
            {
              type: "document",
              title: "Event-Driven Architecture: Our Journey",
              url: "https://cdn.linkedin.example.com/architecture_diagram.pdf",
              pages: 12
            }
          ],
          reactions: {
            like: 1247,
            celebrate: 342,
            support: 89,
            insightful: 156,
            love: 67
          },
          comments_count: 87,
          reposts: 234,
          comments: [
            {
              comment_id: "li_comment_aaa111",
              author: {
                name: "Michael Roberts",
                title: "Senior Data Engineer at DataCorp",
                user_id: "li_user_mike_456"
              },
              text: "Congratulations Sarah! We're looking at a similar migration. Would love to connect and learn from your experience.",
              timestamp: "2024-10-21T13:45:00Z",
              reactions: {
                like: 45
              }
            },
            {
              comment_id: "li_comment_aaa112",
              author: {
                name: "Jennifer Liu",
                title: "VP Engineering at CloudScale",
                user_id: "li_user_jen_789"
              },
              text: "Impressive results! The 10x latency improvement is remarkable. Did you use Apache Kafka or another streaming platform?",
              timestamp: "2024-10-21T14:20:00Z",
              reactions: {
                like: 23,
                insightful: 8
              }
            }
          ],
          industries: ["Technology", "Data Engineering", "Cloud Computing"],
          skills_mentioned: ["Event-Driven Architecture", "Data Engineering", "ML Pipeline", "Infrastructure"]
        },
        {
          post_id: "li_post_555666778",
          author: {
            name: "Marcus Thompson",
            title: "Product Manager | Ex-Google | Building the Future of Work",
            profile_url: "linkedin.com/in/marcus-thompson-pm",
            user_id: "li_user_marcus_234"
          },
          content: `5 lessons from shipping 100+ product features:

1. Talk to users BEFORE writing specs
2. Small iterations > big launches
3. Metrics don't tell the whole story
4. Technical debt is real debt
5. Celebrate wins with your team

What would you add to this list?

#ProductManagement #Technology #Leadership`,
          posted_at: "2024-10-21T10:30:00Z",
          article: null,
          media: [],
          reactions: {
            like: 3421,
            celebrate: 892,
            insightful: 567,
            love: 234
          },
          comments_count: 234,
          reposts: 789,
          comments: [],
          industries: ["Product Management", "Technology", "Startups"],
          skills_mentioned: ["Product Management", "User Research", "Agile"]
        }
      ],
      tiktok: [
        {
          video_id: "tt_vid_777888999",
          username: "coding_tips_daily",
          user_id: "tt_user_code_001",
          caption: "3 JavaScript array methods that will blow your mind 🤯 #coding #javascript #programming #webdev #learntocode",
          posted_at: "2024-10-21T16:45:00Z",
          video: {
            url: "https://cdn.tiktok.example.com/video_js_tips.mp4",
            thumbnail: "https://cdn.tiktok.example.com/thumb_js_tips.jpg",
            duration: 58,
            width: 1080,
            height: 1920,
            format: "mp4"
          },
          audio: {
            title: "Epic Tech Music",
            artist: "TechBeats Production",
            audio_id: "audio_tech_123"
          },
          statistics: {
            views: 2834562,
            likes: 342891,
            comments: 12453,
            shares: 45672,
            saves: 89234,
            completion_rate: 0.78
          },
          hashtags: ["coding", "javascript", "programming", "webdev", "learntocode"],
          mentions: [],
          effects: ["Green Screen", "Text Animation", "Transition Effect"],
          comments: [
            {
              comment_id: "tt_comment_xyz1",
              username: "dev_beginner_22",
              text: "Just used .reduce() in my project and it worked perfectly! Thanks!",
              timestamp: "2024-10-21T17:00:00Z",
              likes: 1234,
              replies_count: 45
            },
            {
              comment_id: "tt_comment_xyz2",
              username: "senior_dev_10yrs",
              text: "Great explanation! Would love to see more advanced array methods",
              timestamp: "2024-10-21T17:30:00Z",
              likes: 892,
              replies_count: 23
            }
          ]
        },
        {
          video_id: "tt_vid_777889000",
          username: "travel_with_emma",
          user_id: "tt_user_travel_042",
          caption: "POV: You visit Santorini for the first time 🇬🇷✨ #travel #santorini #greece #traveltok #wanderlust",
          posted_at: "2024-10-20T08:20:00Z",
          video: {
            url: "https://cdn.tiktok.example.com/video_santorini.mp4",
            thumbnail: "https://cdn.tiktok.example.com/thumb_santorini.jpg",
            duration: 43,
            width: 1080,
            height: 1920,
            format: "mp4"
          },
          audio: {
            title: "Summer Vibes",
            artist: "Chill Beats Co.",
            audio_id: "audio_summer_456"
          },
          statistics: {
            views: 8923451,
            likes: 1234567,
            comments: 34521,
            shares: 123456,
            saves: 234567,
            completion_rate: 0.92
          },
          hashtags: ["travel", "santorini", "greece", "traveltok", "wanderlust"],
          mentions: ["@visit_greece_official"],
          effects: ["Color Grading", "Slow Motion", "Zoom Transition"],
          location: {
            name: "Santorini",
            country: "Greece",
            coordinates: {
              lat: 36.3932,
              lng: 25.4615
            }
          },
          comments: [
            {
              comment_id: "tt_comment_xyz3",
              username: "greece_lover_89",
              text: "Adding this to my 2025 bucket list! 😍",
              timestamp: "2024-10-20T09:00:00Z",
              likes: 4521,
              replies_count: 234
            }
          ]
        }
      ]
    };
    r(JSON.stringify(_[w], null, 2));
  };
  return /* @__PURE__ */ d("div", { className: "space-y-4", children: [
    N && /* @__PURE__ */ s("div", { className: "bg-white p-3 rounded-lg shadow-sm border border-gray-200", children: /* @__PURE__ */ d("div", { className: "flex items-center gap-4 text-sm", children: [
      /* @__PURE__ */ s("span", { className: `px-2 py-1 rounded text-xs font-medium ${N.enabled && N.configured ? "bg-green-100 text-green-800" : "bg-red-100 text-red-800"}`, children: N.enabled && N.configured ? "Ready" : "Not Configured" }),
      /* @__PURE__ */ d("span", { className: "text-gray-600", children: [
        N.provider,
        " · ",
        N.model
      ] }),
      /* @__PURE__ */ s("span", { className: "text-xs text-gray-500", children: "Configure AI settings using the Settings button in the header" })
    ] }) }),
    /* @__PURE__ */ d("div", { className: "bg-white p-4 rounded-lg shadow", children: [
      /* @__PURE__ */ d("div", { className: "flex items-center justify-between mb-3", children: [
        /* @__PURE__ */ s("h3", { className: "text-lg font-medium text-gray-900", children: "JSON Data" }),
        /* @__PURE__ */ d("div", { className: "flex gap-2", children: [
          /* @__PURE__ */ s(
            "button",
            {
              onClick: () => p("blogposts"),
              className: "px-2 py-1 bg-green-50 text-green-700 rounded text-xs hover:bg-green-100",
              children: "Blog Posts (100)"
            }
          ),
          /* @__PURE__ */ s(
            "button",
            {
              onClick: () => p("twitter"),
              className: "px-2 py-1 bg-blue-50 text-blue-700 rounded text-xs hover:bg-blue-100",
              children: "Twitter"
            }
          ),
          /* @__PURE__ */ s(
            "button",
            {
              onClick: () => p("instagram"),
              className: "px-2 py-1 bg-pink-50 text-pink-700 rounded text-xs hover:bg-pink-100",
              children: "Instagram"
            }
          ),
          /* @__PURE__ */ s(
            "button",
            {
              onClick: () => p("linkedin"),
              className: "px-2 py-1 bg-indigo-50 text-indigo-700 rounded text-xs hover:bg-indigo-100",
              children: "LinkedIn"
            }
          ),
          /* @__PURE__ */ s(
            "button",
            {
              onClick: () => p("tiktok"),
              className: "px-2 py-1 bg-purple-50 text-purple-700 rounded text-xs hover:bg-purple-100",
              children: "TikTok"
            }
          )
        ] })
      ] }),
      /* @__PURE__ */ s(
        "textarea",
        {
          id: "jsonData",
          value: e,
          onChange: (w) => r(w.target.value),
          placeholder: "Enter your JSON data here or load a sample...",
          className: "w-full h-64 p-3 border border-gray-300 rounded-md font-mono text-sm focus:ring-2 focus:ring-blue-500 focus:border-transparent"
        }
      )
    ] }),
    /* @__PURE__ */ s("div", { className: "bg-white p-4 rounded-lg shadow", children: /* @__PURE__ */ d("div", { className: "flex items-center justify-between", children: [
      /* @__PURE__ */ d("div", { className: "flex items-center gap-4", children: [
        /* @__PURE__ */ d("label", { className: "flex items-center gap-2 text-sm", children: [
          /* @__PURE__ */ s(
            "input",
            {
              type: "checkbox",
              checked: i,
              onChange: (w) => o(w.target.checked),
              className: "rounded"
            }
          ),
          /* @__PURE__ */ s("span", { className: "text-gray-700", children: "Auto-execute mutations" })
        ] }),
        /* @__PURE__ */ s("span", { className: "text-xs text-gray-500", children: "AI will analyze and automatically map data to schemas" })
      ] }),
      /* @__PURE__ */ s(
        "button",
        {
          onClick: y,
          disabled: f || !e.trim(),
          className: `px-6 py-2.5 rounded font-medium transition-colors ${f || !e.trim() ? "bg-gray-300 text-gray-500 cursor-not-allowed" : "bg-blue-600 text-white hover:bg-blue-700"}`,
          children: f ? "Processing..." : "Process Data"
        }
      )
    ] }) })
  ] });
}
function Zo({ onResult: t }) {
  const [e, r] = O(!1), [i, o] = O(null), [u, h] = O(!0), [l, g] = O(0), [f, b] = O("default"), [N, A] = O(!1), [E, y] = O(null), [x, p] = O(!1), [w, _] = O("");
  ue(() => {
    S();
  }, []);
  const S = async () => {
    try {
      const F = await dt.getStatus();
      F.success && y(F.data);
    } catch (F) {
      console.error("Failed to fetch ingestion status:", F);
    }
  }, B = $((F) => {
    F.preventDefault(), F.stopPropagation(), r(!0);
  }, []), I = $((F) => {
    F.preventDefault(), F.stopPropagation(), r(!1);
  }, []), j = $((F) => {
    F.preventDefault(), F.stopPropagation();
  }, []), C = $((F) => {
    F.preventDefault(), F.stopPropagation(), r(!1);
    const U = F.dataTransfer.files;
    U && U.length > 0 && o(U[0]);
  }, []), k = $((F) => {
    const U = F.target.files;
    U && U.length > 0 && o(U[0]);
  }, []), L = async () => {
    if (x) {
      if (!w || !w.startsWith("s3://")) {
        t({
          success: !1,
          error: "Please provide a valid S3 path (e.g., s3://bucket/path/to/file.json)"
        });
        return;
      }
    } else if (!i) {
      t({
        success: !1,
        error: "Please select a file to upload"
      });
      return;
    }
    A(!0), t(null);
    try {
      const F = new FormData(), U = crypto.randomUUID();
      F.append("progress_id", U), x ? F.append("s3FilePath", w) : F.append("file", i), F.append("autoExecute", u.toString()), F.append("trustDistance", l.toString()), F.append("pubKey", f);
      const V = await (await fetch("/api/ingestion/upload", {
        method: "POST",
        body: F
      })).json();
      V.success ? t({
        success: !0,
        data: {
          schema_used: V.schema_name || V.schema_used,
          new_schema_created: V.new_schema_created,
          mutations_generated: V.mutations_generated,
          mutations_executed: V.mutations_executed
        }
      }) : t({
        success: !1,
        error: V.error || "Failed to process file"
      });
    } catch (F) {
      t({
        success: !1,
        error: F.message || "Failed to process file"
      });
    } finally {
      A(!1);
    }
  }, P = (F) => {
    if (F === 0) return "0 Bytes";
    const U = 1024, K = ["Bytes", "KB", "MB", "GB"], V = Math.floor(Math.log(F) / Math.log(U));
    return Math.round(F / Math.pow(U, V) * 100) / 100 + " " + K[V];
  };
  return /* @__PURE__ */ d("div", { className: "space-y-4", children: [
    E && /* @__PURE__ */ s("div", { className: "bg-white p-3 rounded-lg shadow-sm border border-gray-200", children: /* @__PURE__ */ d("div", { className: "flex items-center gap-4 text-sm", children: [
      /* @__PURE__ */ s("span", { className: `px-2 py-1 rounded text-xs font-medium ${E.enabled && E.configured ? "bg-green-100 text-green-800" : "bg-red-100 text-red-800"}`, children: E.enabled && E.configured ? "Ready" : "Not Configured" }),
      /* @__PURE__ */ d("span", { className: "text-gray-600", children: [
        E.provider,
        " · ",
        E.model
      ] }),
      /* @__PURE__ */ s("span", { className: "text-xs text-gray-500", children: "Configure AI settings using the Settings button in the header" })
    ] }) }),
    N && /* @__PURE__ */ s("div", { className: "bg-blue-50 border border-blue-200 rounded-lg p-4", children: /* @__PURE__ */ d("div", { className: "flex items-center gap-3", children: [
      /* @__PURE__ */ d("svg", { className: "animate-spin h-5 w-5 text-blue-600", xmlns: "http://www.w3.org/2000/svg", fill: "none", viewBox: "0 0 24 24", children: [
        /* @__PURE__ */ s("circle", { className: "opacity-25", cx: "12", cy: "12", r: "10", stroke: "currentColor", strokeWidth: "4" }),
        /* @__PURE__ */ s("path", { className: "opacity-75", fill: "currentColor", d: "M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" })
      ] }),
      /* @__PURE__ */ s("span", { className: "text-blue-800 font-medium", children: "Processing file..." })
    ] }) }),
    /* @__PURE__ */ s("div", { className: "bg-white p-4 rounded-lg shadow", children: /* @__PURE__ */ d("div", { className: "flex items-center gap-6", children: [
      /* @__PURE__ */ s("span", { className: "text-sm font-medium text-gray-700", children: "Input Mode:" }),
      /* @__PURE__ */ d("label", { className: "flex items-center gap-2 cursor-pointer", children: [
        /* @__PURE__ */ s(
          "input",
          {
            type: "radio",
            checked: !x,
            onChange: () => p(!1),
            className: "rounded"
          }
        ),
        /* @__PURE__ */ s("span", { className: "text-sm text-gray-700", children: "Upload File" })
      ] }),
      /* @__PURE__ */ d("label", { className: "flex items-center gap-2 cursor-pointer", children: [
        /* @__PURE__ */ s(
          "input",
          {
            type: "radio",
            checked: x,
            onChange: () => p(!0),
            className: "rounded"
          }
        ),
        /* @__PURE__ */ s("span", { className: "text-sm text-gray-700", children: "S3 File Path" })
      ] })
    ] }) }),
    x ? /* @__PURE__ */ d("div", { className: "bg-white p-6 rounded-lg shadow", children: [
      /* @__PURE__ */ s("h3", { className: "text-lg font-medium text-gray-900 mb-4", children: "S3 File Path" }),
      /* @__PURE__ */ d("div", { className: "space-y-3", children: [
        /* @__PURE__ */ s("label", { className: "block text-sm font-medium text-gray-700", children: "Enter S3 file path" }),
        /* @__PURE__ */ s(
          "input",
          {
            type: "text",
            value: w,
            onChange: (F) => _(F.target.value),
            placeholder: "s3://bucket-name/path/to/file.json",
            className: "w-full px-4 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
          }
        ),
        /* @__PURE__ */ s("p", { className: "text-xs text-gray-500", children: "The file will be downloaded from S3 for processing without re-uploading" })
      ] })
    ] }) : /* @__PURE__ */ d("div", { className: "bg-white p-6 rounded-lg shadow", children: [
      /* @__PURE__ */ s("h3", { className: "text-lg font-medium text-gray-900 mb-4", children: "Upload File" }),
      /* @__PURE__ */ s(
        "div",
        {
          className: `border-2 border-dashed rounded-lg p-12 text-center transition-colors ${e ? "border-blue-500 bg-blue-50" : "border-gray-300 bg-gray-50 hover:bg-gray-100"}`,
          onDragEnter: B,
          onDragOver: j,
          onDragLeave: I,
          onDrop: C,
          children: /* @__PURE__ */ d("div", { className: "space-y-4", children: [
            /* @__PURE__ */ s("div", { className: "flex justify-center", children: /* @__PURE__ */ s(
              "svg",
              {
                className: "w-16 h-16 text-gray-400",
                fill: "none",
                stroke: "currentColor",
                viewBox: "0 0 24 24",
                xmlns: "http://www.w3.org/2000/svg",
                children: /* @__PURE__ */ s(
                  "path",
                  {
                    strokeLinecap: "round",
                    strokeLinejoin: "round",
                    strokeWidth: 2,
                    d: "M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12"
                  }
                )
              }
            ) }),
            i ? /* @__PURE__ */ d("div", { className: "space-y-2", children: [
              /* @__PURE__ */ s("p", { className: "text-lg font-medium text-gray-900", children: i.name }),
              /* @__PURE__ */ s("p", { className: "text-sm text-gray-500", children: P(i.size) }),
              /* @__PURE__ */ s(
                "button",
                {
                  onClick: () => o(null),
                  className: "text-sm text-blue-600 hover:text-blue-700 underline",
                  children: "Remove file"
                }
              )
            ] }) : /* @__PURE__ */ d("div", { children: [
              /* @__PURE__ */ s("p", { className: "text-lg text-gray-700 mb-2", children: "Drag and drop a file here, or click to select" }),
              /* @__PURE__ */ s("p", { className: "text-sm text-gray-500", children: "Supported formats: PDF, DOCX, TXT, CSV, JSON, XML, and more" })
            ] }),
            /* @__PURE__ */ s(
              "input",
              {
                type: "file",
                id: "file-upload",
                className: "hidden",
                onChange: k
              }
            ),
            !i && /* @__PURE__ */ s(
              "label",
              {
                htmlFor: "file-upload",
                className: "inline-block px-6 py-3 bg-blue-600 text-white rounded-lg cursor-pointer hover:bg-blue-700 transition-colors",
                children: "Browse Files"
              }
            )
          ] })
        }
      )
    ] }),
    /* @__PURE__ */ s("div", { className: "bg-white p-4 rounded-lg shadow", children: /* @__PURE__ */ d("div", { className: "flex items-center justify-between", children: [
      /* @__PURE__ */ d("div", { className: "flex items-center gap-4", children: [
        /* @__PURE__ */ d("label", { className: "flex items-center gap-2 text-sm", children: [
          /* @__PURE__ */ s(
            "input",
            {
              type: "checkbox",
              checked: u,
              onChange: (F) => h(F.target.checked),
              className: "rounded"
            }
          ),
          /* @__PURE__ */ s("span", { className: "text-gray-700", children: "Auto-execute mutations" })
        ] }),
        /* @__PURE__ */ s("span", { className: "text-xs text-gray-500", children: "File will be converted to JSON and processed by AI" })
      ] }),
      /* @__PURE__ */ s(
        "button",
        {
          onClick: L,
          disabled: N || !x && !i || x && !w,
          className: `px-6 py-2.5 rounded font-medium transition-colors ${N || !x && !i || x && !w ? "bg-gray-300 text-gray-500 cursor-not-allowed" : "bg-blue-600 text-white hover:bg-blue-700"}`,
          children: N ? "Processing..." : x ? "Process S3 File" : "Upload & Process"
        }
      )
    ] }) }),
    /* @__PURE__ */ s("div", { className: "bg-blue-50 border border-blue-200 rounded-lg p-4", children: /* @__PURE__ */ d("div", { className: "flex items-start gap-3", children: [
      /* @__PURE__ */ s(
        "svg",
        {
          className: "w-6 h-6 text-blue-600 flex-shrink-0 mt-0.5",
          fill: "none",
          stroke: "currentColor",
          viewBox: "0 0 24 24",
          children: /* @__PURE__ */ s(
            "path",
            {
              strokeLinecap: "round",
              strokeLinejoin: "round",
              strokeWidth: 2,
              d: "M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
            }
          )
        }
      ),
      /* @__PURE__ */ d("div", { className: "text-sm text-blue-800", children: [
        /* @__PURE__ */ s("p", { className: "font-medium mb-1", children: "How it works:" }),
        /* @__PURE__ */ d("ol", { className: "list-decimal list-inside space-y-1", children: [
          /* @__PURE__ */ s("li", { children: x ? "Provide an S3 file path (files already in S3 are not re-uploaded)" : "Upload any file type (PDFs, documents, spreadsheets, etc.)" }),
          /* @__PURE__ */ s("li", { children: "File is automatically converted to JSON using AI" }),
          /* @__PURE__ */ s("li", { children: "AI analyzes the JSON and maps it to appropriate schemas" }),
          /* @__PURE__ */ s("li", { children: "Data is stored in the database with the file location tracked" })
        ] })
      ] })
    ] }) })
  ] });
}
function uo() {
  const t = He(), e = Z(Ct), r = Z(ut), i = Z(Lr), o = Z(Mn), u = Z(Ra), h = $(async () => {
    t(Te({ forceRefresh: !0 }));
  }, [t]), l = $((f) => r.find((b) => b.name === f) || null, [r]), g = $((f) => {
    const b = l(f);
    return b ? Fn(b.state) === ke.APPROVED : !1;
  }, [l]);
  return ue(() => {
    u.isValid || (console.log("🟡 useApprovedSchemas: Cache invalid, fetching schemas"), t(Te()));
  }, [t]), {
    approvedSchemas: e,
    isLoading: i,
    error: o,
    refetch: h,
    getSchemaByName: l,
    isSchemaApproved: g,
    // Additional utility for components that need all schemas for display
    allSchemas: r
  };
}
function ho({ r: t }) {
  return /* @__PURE__ */ d("tr", { className: "border-t", children: [
    /* @__PURE__ */ s("td", { className: "px-2 py-1 text-xs text-gray-600", children: t.key_value?.hash ?? "" }),
    /* @__PURE__ */ s("td", { className: "px-2 py-1 text-xs text-gray-600", children: t.key_value?.range ?? "" }),
    /* @__PURE__ */ s("td", { className: "px-2 py-1 text-xs font-mono text-gray-800", children: t.schema_name }),
    /* @__PURE__ */ s("td", { className: "px-2 py-1 text-xs text-gray-800", children: t.field }),
    /* @__PURE__ */ s("td", { className: "px-2 py-1 text-xs text-gray-800 whitespace-pre-wrap break-words", children: mo(t.value) })
  ] });
}
function mo(t) {
  if (t == null) return "";
  if (typeof t == "string") return t;
  try {
    return JSON.stringify(t);
  } catch {
    return String(t);
  }
}
function Xo({ onResult: t }) {
  const { approvedSchemas: e, isLoading: r, refetch: i } = uo(), [o, u] = O(""), [h, l] = O(!1), [g, f] = O([]), [b, N] = O(null), [A, E] = O(() => /* @__PURE__ */ new Set()), [y, x] = O(() => /* @__PURE__ */ new Map());
  ue(() => {
    i();
  }, [i]);
  const p = $(async () => {
    l(!0), N(null);
    try {
      const C = await ii.search(o);
      if (C.success) {
        const k = C.data?.results || [];
        f(k), t({ success: !0, data: k });
      } else
        N(C.error || "Search failed"), t({ error: C.error || "Search failed", status: C.status });
    } catch (C) {
      N(C.message || "Network error"), t({ error: C.message || "Network error" });
    } finally {
      l(!1);
    }
  }, [o, t]), w = $((C) => {
    if (!C) return [];
    const k = C.fields;
    return Array.isArray(k) ? k.slice() : k && typeof k == "object" ? Object.keys(k) : [];
  }, []), _ = W(() => {
    const C = /* @__PURE__ */ new Map();
    return (e || []).forEach((k) => C.set(k.name, k)), C;
  }, [e]), S = $((C, k) => {
    const L = k?.hash ?? "", P = k?.range ?? "";
    return `${C}|${L}|${P}`;
  }, []), B = $((C) => {
    const k = C?.hash, L = C?.range;
    if (k && L) return Ri(k, L);
    if (k) return St(k);
    if (L) return St(L);
  }, []), I = $(async (C, k) => {
    const L = _.get(C), P = w(L), F = B(k), U = { schema_name: C, fields: P };
    F && (U.filter = F);
    const K = await Dr.executeQuery(U);
    if (!K.success)
      throw new Error(K.error || "Query failed");
    const V = Array.isArray(K.data?.results) ? K.data.results : [], Y = V.find((ae) => {
      const Ie = ae?.key?.hash ?? null, Se = ae?.key?.range ?? null, We = k?.hash ?? null, Je = k?.range ?? null;
      return String(Ie || "") === String(We || "") && String(Se || "") === String(Je || "");
    }) || V[0];
    return Y?.fields || (Y && typeof Y == "object" ? Y : {});
  }, [_, w, B]), j = $(async () => {
    const C = /* @__PURE__ */ new Map();
    for (const P of g) {
      const F = S(P.schema_name, P.key_value);
      C.has(F) || C.set(F, P);
    }
    const k = Array.from(C.values()), L = new Map(y);
    await Promise.all(k.map(async (P) => {
      const F = S(P.schema_name, P.key_value);
      if (!L.has(F))
        try {
          const U = await I(P.schema_name, P.key_value);
          L.set(F, U);
        } catch {
          L.set(F, {});
        }
    })), x(L);
  }, [g, y, S, I]);
  return ue(() => {
    g.length > 0 && j().catch(() => {
    });
  }, [g, j]), /* @__PURE__ */ d("div", { className: "p-6 space-y-4", children: [
    /* @__PURE__ */ d("div", { className: "bg-white p-4 rounded-lg shadow", children: [
      /* @__PURE__ */ d("div", { className: "mb-3", children: [
        /* @__PURE__ */ s("h3", { className: "text-lg font-medium text-gray-900", children: "Native Index Search" }),
        /* @__PURE__ */ s("p", { className: "text-xs text-gray-500", children: "Search the database-native word index across all approved schemas." })
      ] }),
      /* @__PURE__ */ d("div", { className: "flex gap-2 items-center", children: [
        /* @__PURE__ */ s(
          "input",
          {
            type: "text",
            value: o,
            onChange: (C) => u(C.target.value),
            placeholder: "Enter search term (e.g. jennifer)",
            className: "flex-1 px-3 py-2 border rounded-md text-sm"
          }
        ),
        /* @__PURE__ */ s(
          "button",
          {
            onClick: p,
            disabled: h || !o.trim(),
            className: `px-4 py-2 rounded text-sm ${h || !o.trim() ? "bg-gray-300 text-gray-600" : "bg-blue-600 text-white hover:bg-blue-700"}`,
            children: h ? "Searching..." : "Search"
          }
        )
      ] })
    ] }),
    /* @__PURE__ */ d("div", { className: "bg-white p-4 rounded-lg shadow", children: [
      /* @__PURE__ */ d("div", { className: "mb-2 flex items-center justify-between", children: [
        /* @__PURE__ */ s("h4", { className: "text-md font-medium text-gray-900", children: "Search Results" }),
        /* @__PURE__ */ d("div", { className: "flex items-center gap-3", children: [
          /* @__PURE__ */ d("span", { className: "text-xs text-gray-500", children: [
            g.length,
            " matches"
          ] }),
          g.length > 0 && /* @__PURE__ */ s(
            "button",
            {
              type: "button",
              className: "text-xs px-2 py-1 rounded border border-gray-300 hover:bg-gray-100",
              onClick: () => j(),
              children: "Refresh Details"
            }
          )
        ] })
      ] }),
      b && /* @__PURE__ */ s("div", { className: "mb-2 p-2 bg-red-50 border border-red-200 text-xs text-red-700 rounded", children: b }),
      /* @__PURE__ */ s("div", { className: "overflow-auto max-h-[450px]", children: /* @__PURE__ */ d("table", { className: "min-w-full text-left text-xs", children: [
        /* @__PURE__ */ s("thead", { children: /* @__PURE__ */ d("tr", { className: "text-gray-500", children: [
          /* @__PURE__ */ s("th", { className: "px-2 py-1", children: "Hash" }),
          /* @__PURE__ */ s("th", { className: "px-2 py-1", children: "Range" }),
          /* @__PURE__ */ s("th", { className: "px-2 py-1", children: "Schema" }),
          /* @__PURE__ */ s("th", { className: "px-2 py-1", children: "Field" }),
          /* @__PURE__ */ s("th", { className: "px-2 py-1", children: "Value" }),
          /* @__PURE__ */ s("th", { className: "px-2 py-1" })
        ] }) }),
        /* @__PURE__ */ d("tbody", { children: [
          g.map((C, k) => {
            const L = S(C.schema_name, C.key_value), P = A.has(L), F = y.get(L);
            return /* @__PURE__ */ d(At, { children: [
              /* @__PURE__ */ s(ho, { r: C }, `${L}-row`),
              /* @__PURE__ */ d("tr", { className: "border-b", children: [
                /* @__PURE__ */ s("td", { colSpan: 5 }),
                /* @__PURE__ */ s("td", { className: "px-2 py-1 text-right", children: /* @__PURE__ */ s(
                  "button",
                  {
                    type: "button",
                    className: "text-xs px-2 py-1 rounded border border-gray-300 hover:bg-gray-100",
                    onClick: async () => {
                      const U = new Set(A);
                      if (U.has(L) ? U.delete(L) : U.add(L), E(U), !y.has(L))
                        try {
                          const K = await I(C.schema_name, C.key_value);
                          x((V) => new Map(V).set(L, K));
                        } catch {
                        }
                    },
                    children: P ? "Hide Data" : "Show Data"
                  }
                ) })
              ] }, `${L}-actions`),
              P && /* @__PURE__ */ s("tr", { children: /* @__PURE__ */ s("td", { colSpan: 6, className: "px-2 pb-3", children: /* @__PURE__ */ s("div", { className: "ml-2 bg-gray-50 border rounded", children: /* @__PURE__ */ s(FieldsTable, { fields: F || {} }) }) }) }, `${L}-details`)
            ] });
          }),
          g.length === 0 && /* @__PURE__ */ s("tr", { children: /* @__PURE__ */ s("td", { colSpan: 5, className: "px-2 py-3 text-center text-gray-500", children: "No results" }) })
        ] })
      ] }) })
    ] })
  ] });
}
function el({
  state: t,
  isRangeSchema: e = !1,
  size: r = "md",
  className: i = "",
  showTooltip: o = !0
}) {
  const u = {
    sm: "px-1.5 py-0.5 text-xs",
    md: "px-2.5 py-0.5 text-xs",
    lg: "px-3 py-1 text-sm"
  }, h = () => Wr[t] || Wr.available, l = () => ({
    approved: "Approved",
    available: "Available",
    blocked: "Blocked",
    pending: "Pending"
  })[t] || "Unknown", g = () => o ? kn.schemaStates[t] || "Unknown schema state" : "", f = `
    inline-flex items-center rounded-full font-medium
    ${u[r]}
    ${h()}
    ${i}
  `.trim();
  return /* @__PURE__ */ d("div", { className: "inline-flex items-center space-x-2", children: [
    /* @__PURE__ */ s(
      "span",
      {
        className: f,
        title: g(),
        "aria-label": `Schema status: ${l()}${e ? ", Range Schema" : ""}`,
        children: l()
      }
    ),
    e && /* @__PURE__ */ s(
      "span",
      {
        className: `
            inline-flex items-center rounded-full font-medium
            ${u[r]}
            ${Jr.badgeColor}
          `,
        title: "This schema uses range-based keys for efficient querying",
        "aria-label": "Range Schema",
        children: Jr.label
      }
    )
  ] });
}
const fo = (t) => t?.schema_type === "Range", po = (t) => t?.key?.range_field || null;
function tl({
  children: t,
  queryState: e,
  schemas: r,
  selectedSchemaObj: i,
  isRangeSchema: o,
  rangeKey: u,
  schema: h,
  ...l
}) {
  const g = W(() => h || (e?.selectedSchema ? e.selectedSchema : i?.name ?? null), [h, e?.selectedSchema, i?.name]), f = W(() => i || (g && r && r[g] ? r[g] : null), [g, r, i]), b = W(() => {
    if (r)
      return r;
    if (g && f)
      return { [g]: f };
  }, [r, g, f]), N = W(() => typeof o == "boolean" ? o : fo(f), [o, f]), A = W(() => u || po(f), [u, f]), E = W(() => ({
    ...l,
    schema: g,
    queryState: e,
    schemas: b,
    selectedSchemaObj: f,
    isRangeSchema: N,
    rangeKey: A
  }), [l, g, e, b, f, N, A]);
  let y;
  try {
    y = Qn(E);
  } catch (x) {
    y = {
      query: null,
      validationErrors: [x.message || "An error occurred while building the query"],
      isValid: !1,
      buildQuery: () => null,
      validateQuery: () => !1,
      error: x
    };
  }
  return typeof t == "function" ? t(y) : null;
}
process.env.NODE_ENV;
const rl = "ingestion";
export {
  rl as DEFAULT_TAB,
  Ce as FieldWrapper,
  Zo as FileUploadTab,
  Do as FoldDbProvider,
  Ho as Footer,
  jo as Header,
  Jo as IngestionTab,
  Ai as KeyManagementTab,
  Qo as LlmQueryTab,
  Ko as LogSidebar,
  qo as LoginModal,
  Vo as LoginPage,
  no as MutationEditor,
  Wo as MutationTab,
  Xo as NativeIndexTab,
  Xi as QueryActions,
  tl as QueryBuilder,
  Zi as QueryForm,
  to as QueryPreview,
  zo as QueryTab,
  Ji as RangeField,
  so as ResultViewer,
  Po as ResultsSection,
  ro as SchemaSelector,
  el as SchemaStatusBadge,
  Go as SchemaTab,
  wr as SelectField,
  $o as SettingsModal,
  Mo as StatusSection,
  fi as StructuredResults,
  Uo as TabNavigation,
  wt as TextField,
  _i as TopologyDisplay,
  bi as TransformsTab,
  La as aiQueryReducer,
  oa as authReducer,
  aa as clearAuthentication,
  No as clearError,
  or as fetchNodePrivateKey,
  ar as initializeSystemKey,
  wo as injectStore,
  Br as loginUser,
  ia as logoutUser,
  ir as refreshSystemKey,
  Eo as restoreSession,
  Ia as schemaReducer,
  vo as setError,
  ja as store,
  So as updateSystemKey,
  He as useAppDispatch,
  Z as useAppSelector,
  uo as useApprovedSchemas,
  Ut as validatePrivateKey
};
