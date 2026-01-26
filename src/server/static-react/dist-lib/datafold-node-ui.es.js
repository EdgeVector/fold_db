var es = Object.defineProperty;
var ts = (t, e, r) => e in t ? es(t, e, { enumerable: !0, configurable: !0, writable: !0, value: r }) : t[e] = r;
var qe = (t, e, r) => ts(t, typeof e != "symbol" ? e + "" : e, r);
import { jsx as n, jsxs as c, Fragment as Rt } from "react/jsx-runtime";
import * as q from "react";
import { createContext as rs, useState as B, useContext as ns, useEffect as de, useMemo as Z, useCallback as $, useRef as At } from "react";
import { Provider as ss, useSelector as as, useDispatch as is } from "react-redux";
import { createAsyncThunk as Ze, createSlice as vr, createSelector as Xe, configureStore as os } from "@reduxjs/toolkit";
const ls = {
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
}, cs = {
  ROOT: "/api"
}, Y = ls, ds = 3e4, us = 3, hs = 1e3, le = {
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
}, Ve = {
  // 3 minutes - schema state, transforms
  STANDARD: 3e5,
  // 1 hour - system public key
  // Semantic aliases
  SYSTEM_STATUS: 3e4,
  SCHEMA_DATA: 3e5,
  SECURITY_STATUS: 6e4,
  SYSTEM_PUBLIC_KEY: 36e5
}, we = {
  BAD_REQUEST: 400,
  UNAUTHORIZED: 401,
  FORBIDDEN: 403,
  NOT_FOUND: 404,
  INTERNAL_SERVER_ERROR: 500,
  BAD_GATEWAY: 502,
  SERVICE_UNAVAILABLE: 503
}, sr = {
  JSON: "application/json",
  FORM_DATA: "multipart/form-data",
  URL_ENCODED: "application/x-www-form-urlencoded",
  TEXT: "text/plain"
}, Dt = {
  CONTENT_TYPE: "Content-Type",
  AUTHORIZATION: "Authorization",
  SIGNED_REQUEST: "X-Signed-Request",
  REQUEST_ID: "X-Request-ID",
  AUTHENTICATED: "X-Authenticated"
}, ge = {
  NETWORK_ERROR: "Network connection failed. Please check your internet connection.",
  TIMEOUT_ERROR: "Request timed out. Please try again.",
  AUTHENTICATION_ERROR: "Authentication required. Please ensure you are properly authenticated.",
  SCHEMA_STATE_ERROR: "Schema operation not allowed. Only approved schemas can be accessed.",
  SERVER_ERROR: "Server error occurred. Please try again later.",
  VALIDATION_ERROR: "Request validation failed. Please check your input.",
  NOT_FOUND_ERROR: "Requested resource not found.",
  PERMISSION_ERROR: "Permission denied. You do not have access to this resource.",
  RATE_LIMIT_ERROR: "Too many requests. Please wait before trying again."
}, _t = {
  DEFAULT_TTL_MS: Ve.STANDARD,
  MAX_CACHE_SIZE: 100,
  SCHEMA_CACHE_TTL_MS: Ve.SCHEMA_DATA,
  SYSTEM_STATUS_CACHE_TTL_MS: Ve.SYSTEM_STATUS
}, pr = {
  RETRYABLE_STATUS_CODES: [408, 429, 500, 502, 503, 504],
  EXPONENTIAL_BACKOFF_MULTIPLIER: 2,
  MAX_RETRY_DELAY_MS: 1e4
}, qt = {
  // Use relative path for CloudFront compatibility
  BASE_URL: "/api"
}, ve = {
  AVAILABLE: "available",
  APPROVED: "approved",
  BLOCKED: "blocked"
}, jt = {
  SYSTEM_STATUS: "system-status",
  SECURITY_STATUS: "security-status",
  SYSTEM_PUBLIC_KEY: "system-public-key"
};
class me extends Error {
  constructor(e, r = 0, i = {}) {
    super(e), this.name = "ApiError", this.status = r, this.response = i.response, this.isNetworkError = i.isNetworkError || !1, this.isTimeoutError = i.isTimeoutError || !1, this.isRetryable = this.determineRetryability(r, i.isNetworkError, i.isTimeoutError), this.requestId = i.requestId, this.timestamp = Date.now(), this.code = i.code, this.details = i.details, Object.setPrototypeOf(this, me.prototype);
  }
  /**
   * Determines if an error is retryable based on status code and error type
   */
  determineRetryability(e, r, i) {
    return r || i ? !0 : pr.RETRYABLE_STATUS_CODES.includes(e);
  }
  /**
   * Convert error to a user-friendly message
   */
  toUserMessage() {
    if (this.isNetworkError)
      return ge.NETWORK_ERROR;
    if (this.isTimeoutError)
      return ge.TIMEOUT_ERROR;
    switch (this.status) {
      case we.UNAUTHORIZED:
        return ge.AUTHENTICATION_ERROR;
      case we.FORBIDDEN:
        return ge.PERMISSION_ERROR;
      case we.NOT_FOUND:
        return ge.NOT_FOUND_ERROR;
      case we.BAD_REQUEST:
        return ge.VALIDATION_ERROR;
      case we.INTERNAL_SERVER_ERROR:
      case we.BAD_GATEWAY:
      case we.SERVICE_UNAVAILABLE:
        return ge.SERVER_ERROR;
      case 429:
        return ge.RATE_LIMIT_ERROR;
      default:
        return this.message || ge.SERVER_ERROR;
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
class Nr extends me {
  constructor(e = ge.AUTHENTICATION_ERROR, r) {
    super(e, we.UNAUTHORIZED, {
      code: "AUTH_ERROR",
      requestId: r
    }), this.name = "AuthenticationError", Object.setPrototypeOf(this, Nr.prototype);
  }
}
class Sr extends me {
  constructor(e, r, i, o = ge.SCHEMA_STATE_ERROR) {
    super(o, we.FORBIDDEN, {
      code: "SCHEMA_STATE_ERROR",
      details: { schemaName: e, currentState: r, operation: i }
    }), this.name = "SchemaStateError", this.schemaName = e, this.currentState = r, this.operation = i, Object.setPrototypeOf(this, Sr.prototype);
  }
}
class Er extends me {
  constructor(e = ge.NETWORK_ERROR, r) {
    super(e, 0, {
      isNetworkError: !0,
      code: "NETWORK_ERROR",
      requestId: r
    }), this.name = "NetworkError", Object.setPrototypeOf(this, Er.prototype);
  }
}
class Ar extends me {
  constructor(e, r) {
    super(`Request timed out after ${e}ms`, 408, {
      isTimeoutError: !0,
      code: "TIMEOUT_ERROR",
      requestId: r,
      details: { timeoutMs: e }
    }), this.name = "TimeoutError", this.timeoutMs = e, Object.setPrototypeOf(this, Ar.prototype);
  }
}
class _r extends me {
  constructor(e, r) {
    super("Request validation failed", we.BAD_REQUEST, {
      code: "VALIDATION_ERROR",
      requestId: r,
      details: { validationErrors: e }
    }), this.name = "ValidationError", this.validationErrors = e, Object.setPrototypeOf(this, _r.prototype);
  }
}
class Tr extends me {
  constructor(e, r) {
    const i = e ? `Rate limit exceeded. Retry after ${e} seconds.` : ge.RATE_LIMIT_ERROR;
    super(i, 429, {
      code: "RATE_LIMIT_ERROR",
      requestId: r,
      details: { retryAfter: e }
    }), this.name = "RateLimitError", this.retryAfter = e, Object.setPrototypeOf(this, Tr.prototype);
  }
}
class St {
  /**
   * Create an ApiError from a fetch response
   */
  static async fromResponse(e, r) {
    let i = {};
    try {
      const d = await e.text();
      d && (i = JSON.parse(d));
    } catch {
    }
    const o = typeof i.error == "string" ? i.error : typeof i.message == "string" ? i.message : `HTTP ${e.status}`;
    if (e.status === we.UNAUTHORIZED)
      return new Nr(o, r || "");
    if (e.status === 429) {
      const d = e.headers.get("Retry-After");
      return new Tr(d ? parseInt(d) : void 0, r);
    }
    return e.status === we.BAD_REQUEST && i.validationErrors ? new _r(i.validationErrors, r || "") : new me(o, e.status, {
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
    return new Er(e.message, r);
  }
  /**
   * Create an ApiError from a timeout
   */
  static fromTimeout(e, r) {
    return new Ar(e, r);
  }
  /**
   * Create a schema state error
   */
  static fromSchemaState(e, r, i) {
    return new Sr(e, r, i);
  }
}
function ms(t) {
  return t instanceof me;
}
function fs(t) {
  return ms(t) && t.isRetryable;
}
class ps {
  constructor(e = _t.MAX_CACHE_SIZE) {
    this.cache = /* @__PURE__ */ new Map(), this.maxSize = e;
  }
  get(e) {
    const r = this.cache.get(e);
    return r ? Date.now() > r.timestamp + r.ttl ? (this.cache.delete(e), null) : r.data : null;
  }
  set(e, r, i = _t.DEFAULT_TTL_MS) {
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
class gs {
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
class gn {
  constructor(e = {}) {
    this.requestInterceptors = [], this.responseInterceptors = [], this.errorInterceptors = [], this.metrics = [], this.config = {
      baseUrl: e.baseUrl || qt.BASE_URL,
      timeout: e.timeout || ds,
      retryAttempts: e.retryAttempts || us,
      retryDelay: e.retryDelay || hs,
      defaultHeaders: e.defaultHeaders || {},
      enableCache: e.enableCache !== !1,
      enableLogging: e.enableLogging !== !1,
      enableMetrics: e.enableMetrics !== !1
    }, this.cache = new ps(), this.requestQueue = new gs();
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
    if (e.length > _t.MAX_CACHE_SIZE)
      throw new me(
        `Batch size exceeds limit of ${_t.MAX_CACHE_SIZE}`
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
          const d = o instanceof me ? o : new me(o.message);
          return {
            id: i.id,
            success: !1,
            error: d.message,
            status: d.status
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
    var f, p;
    const d = o.requestId || this.generateRequestId(), h = Date.now();
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
        requestId: d,
        timestamp: h,
        priority: o.priority || "normal"
      }
    };
    try {
      for (const T of this.requestInterceptors)
        l = await T(l);
      if (l.validateSchema && await this.validateSchemaAccess(
        r,
        e,
        o.validateSchema || !0
      ), e === "GET" && this.config.enableCache && o.cacheable !== !1) {
        const T = this.generateCacheKey(l.url, l.headers), b = this.cache.get(T);
        if (b)
          return {
            ...b,
            meta: {
              ...b.meta,
              cached: !0,
              fromCache: !0,
              requestId: d,
              timestamp: ((f = b.meta) == null ? void 0 : f.timestamp) || Date.now()
            }
          };
      }
      const x = `${e}:${l.url}:${JSON.stringify(i)}`, g = await this.requestQueue.getOrCreate(
        x,
        () => this.executeRequest(l)
      );
      if (e === "GET" && this.config.enableCache && o.cacheable !== !1 && g.success) {
        const T = this.generateCacheKey(l.url, l.headers), b = o.cacheTtl || _t.DEFAULT_TTL_MS;
        this.cache.set(T, g, b);
      }
      let N = g;
      for (const T of this.responseInterceptors)
        N = await T(
          N
        );
      return this.config.enableMetrics && this.recordMetrics({
        requestId: d,
        url: l.url,
        method: e,
        startTime: h,
        endTime: Date.now(),
        duration: Date.now() - h,
        status: g.status,
        cached: ((p = g.meta) == null ? void 0 : p.cached) || !1
      }), N;
    } catch (x) {
      let g = x instanceof me ? x : St.fromNetworkError(x, d);
      for (const N of this.errorInterceptors)
        g = await N(g);
      throw this.config.enableMetrics && this.recordMetrics({
        requestId: d,
        url: l.url,
        method: e,
        startTime: h,
        endTime: Date.now(),
        duration: Date.now() - h,
        error: g.message
      }), g;
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
        if (r = o instanceof me ? o : St.fromNetworkError(o, e.metadata.requestId), i === e.retries || !fs(r))
          break;
        const d = Math.min(
          this.config.retryDelay * Math.pow(pr.EXPONENTIAL_BACKOFF_MULTIPLIER, i),
          pr.MAX_RETRY_DELAY_MS
        );
        await this.sleep(d);
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
      if (e.body && !o[Dt.CONTENT_TYPE] && (o[Dt.CONTENT_TYPE] = sr.JSON), o[Dt.REQUEST_ID] = e.metadata.requestId, typeof window < "u") {
        const l = localStorage.getItem("fold_user_hash") || localStorage.getItem("exemem_user_hash");
        l && (o["x-user-hash"] = l, o["x-user-id"] = l);
      }
      const d = {
        method: e.method,
        headers: o,
        signal: e.abortSignal || r.signal
      };
      e.body && e.method !== "GET" && (d.body = this.serializeBody(
        e.body,
        o[Dt.CONTENT_TYPE]
      ));
      const h = await fetch(e.url, d);
      return clearTimeout(i), await this.handleResponse(h, e.metadata.requestId);
    } catch (o) {
      throw clearTimeout(i), o.name === "AbortError" ? St.fromTimeout(
        e.timeout,
        e.metadata.requestId
      ) : St.fromNetworkError(o, e.metadata.requestId);
    }
  }
  /**
   * Handle HTTP response and convert to standardized format
   */
  async handleResponse(e, r) {
    if (!e.ok)
      throw await St.fromResponse(e, r);
    let i;
    const o = e.headers.get("content-type");
    try {
      o != null && o.includes("application/json") ? i = await e.json() : i = await e.text();
    } catch {
      throw new me("Failed to parse response", e.status, {
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
    o[1];
    const d = typeof i == "boolean" ? {} : i;
    if ((e.includes("/mutation") || e.includes("/query")) && d.requiresApproved !== !1) {
      console.warn(
        "Store not injected into ApiClient, skipping schema validation"
      );
      return;
    }
  }
  /**
   * Serialize request body based on content type
   */
  serializeBody(e, r) {
    return r === sr.JSON ? JSON.stringify(e) : r === sr.FORM_DATA ? e : String(e);
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
new gn();
function Ae(t) {
  return new gn(t);
}
class ys {
  constructor(e) {
    this.client = e || Ae({
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
    const r = e ? `${Y.LIST_LOGS}?since=${e}` : Y.LIST_LOGS;
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
      Y.RESET_DATABASE,
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
    return this.client.get(Y.GET_SYSTEM_STATUS, {
      requiresAuth: !1,
      // Status is public for monitoring
      timeout: le.QUICK,
      retries: ce.CRITICAL,
      // Multiple retries for critical system data
      cacheable: !0,
      cacheTtl: Ve.SYSTEM_STATUS,
      // Cache for 30 seconds
      cacheKey: jt.SYSTEM_STATUS
    });
  }
  /**
   * Get the node's private key
   * UNPROTECTED - No authentication required for UI access
   * 
   * @returns Promise resolving to private key response
   */
  async getNodePrivateKey() {
    return this.client.get(Y.GET_NODE_PRIVATE_KEY, {
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
    return this.client.get(Y.GET_NODE_PUBLIC_KEY, {
      requiresAuth: !1,
      // Public key is safe to share
      timeout: le.QUICK,
      retries: ce.STANDARD,
      cacheable: !0,
      cacheTtl: Ve.SYSTEM_STATUS,
      // Cache for 30 seconds
      cacheKey: jt.SYSTEM_PUBLIC_KEY
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
    const i = Y.STREAM_LOGS, o = i.startsWith("http") ? i : `${qt.BASE_URL}${i.startsWith("/") ? "" : "/"}${i}`, d = new EventSource(o);
    return d.onmessage = (h) => {
      e(h.data);
    }, r && (d.onerror = r), d;
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
      cacheTtl: Ve.SYSTEM_STATUS,
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
const ae = new ys();
ae.getLogs.bind(ae);
ae.resetDatabase.bind(ae);
ae.getSystemStatus.bind(ae);
const Cr = ae.getNodePrivateKey.bind(ae);
ae.getNodePublicKey.bind(ae);
const bs = ae.getDatabaseConfig.bind(ae), xs = ae.updateDatabaseConfig.bind(ae);
ae.createLogStream.bind(ae);
ae.validateResetRequest.bind(ae);
/*! noble-ed25519 - MIT License (c) 2019 Paul Miller (paulmillr.com) */
const ws = {
  p: 0x7fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffedn,
  n: 0x1000000000000000000000000000000014def9dea2f79cd65812631a5cf5d3edn,
  a: 0x7fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffecn,
  d: 0x52036cee2b6ffe738cc740797779e89800700a4d4141d8ab75eb4dca135978a3n,
  Gx: 0x216936d3cd6e53fec0a4e231fdd6dc5c692cc7609525a7b2c9562d608f25d51an,
  Gy: 0x6666666666666666666666666666666666666666666666666666666666666658n
}, { p: he, n: Kt, Gx: $r, Gy: Kr, a: ar, d: ir } = ws, vs = 8n, Ct = 32, yn = 64, Se = (t = "") => {
  throw new Error(t);
}, Ns = (t) => typeof t == "bigint", bn = (t) => typeof t == "string", Ss = (t) => t instanceof Uint8Array || ArrayBuffer.isView(t) && t.constructor.name === "Uint8Array", mt = (t, e) => !Ss(t) || typeof e == "number" && e > 0 && t.length !== e ? Se("Uint8Array expected") : t, Qt = (t) => new Uint8Array(t), Rr = (t) => Uint8Array.from(t), xn = (t, e) => t.toString(16).padStart(e, "0"), Ir = (t) => Array.from(mt(t)).map((e) => xn(e, 2)).join(""), De = { _0: 48, _9: 57, A: 65, F: 70, a: 97, f: 102 }, Hr = (t) => {
  if (t >= De._0 && t <= De._9)
    return t - De._0;
  if (t >= De.A && t <= De.F)
    return t - (De.A - 10);
  if (t >= De.a && t <= De.f)
    return t - (De.a - 10);
}, kr = (t) => {
  const e = "hex invalid";
  if (!bn(t))
    return Se(e);
  const r = t.length, i = r / 2;
  if (r % 2)
    return Se(e);
  const o = Qt(i);
  for (let d = 0, h = 0; d < i; d++, h += 2) {
    const l = Hr(t.charCodeAt(h)), f = Hr(t.charCodeAt(h + 1));
    if (l === void 0 || f === void 0)
      return Se(e);
    o[d] = l * 16 + f;
  }
  return o;
}, wn = (t, e) => mt(bn(t) ? kr(t) : Rr(mt(t)), e), vn = () => globalThis == null ? void 0 : globalThis.crypto, Es = () => {
  var t;
  return ((t = vn()) == null ? void 0 : t.subtle) ?? Se("crypto.subtle must be defined");
}, jr = (...t) => {
  const e = Qt(t.reduce((i, o) => i + mt(o).length, 0));
  let r = 0;
  return t.forEach((i) => {
    e.set(i, r), r += i.length;
  }), e;
}, As = (t = Ct) => vn().getRandomValues(Qt(t)), Vt = BigInt, Qe = (t, e, r, i = "bad number: out of range") => Ns(t) && e <= t && t < r ? t : Se(i), M = (t, e = he) => {
  const r = t % e;
  return r >= 0n ? r : e + r;
}, _s = (t) => M(t, Kt), Nn = (t, e) => {
  (t === 0n || e <= 0n) && Se("no inverse n=" + t + " mod=" + e);
  let r = M(t, e), i = e, o = 0n, d = 1n;
  for (; r !== 0n; ) {
    const h = i / r, l = i % r, f = o - d * h;
    i = r, r = l, o = d, d = f;
  }
  return i === 1n ? M(o, e) : Se("no inverse");
}, Vr = (t) => t instanceof We ? t : Se("Point expected"), gr = 2n ** 256n, Ce = class Ce {
  constructor(e, r, i, o) {
    qe(this, "ex");
    qe(this, "ey");
    qe(this, "ez");
    qe(this, "et");
    const d = gr;
    this.ex = Qe(e, 0n, d), this.ey = Qe(r, 0n, d), this.ez = Qe(i, 1n, d), this.et = Qe(o, 0n, d), Object.freeze(this);
  }
  static fromAffine(e) {
    return new Ce(e.x, e.y, 1n, M(e.x * e.y));
  }
  /** RFC8032 5.1.3: Uint8Array to Point. */
  static fromBytes(e, r = !1) {
    const i = ir, o = Rr(mt(e, Ct)), d = e[31];
    o[31] = d & -129;
    const h = Sn(o);
    Qe(h, 0n, r ? gr : he);
    const f = M(h * h), p = M(f - 1n), x = M(i * f + 1n);
    let { isValid: g, value: N } = Rs(p, x);
    g || Se("bad point: y not sqrt");
    const T = (N & 1n) === 1n, b = (d & 128) !== 0;
    return !r && N === 0n && b && Se("bad point: x==0, isLastByteOdd"), b !== T && (N = M(-N)), new Ce(N, h, 1n, M(N * h));
  }
  /** Checks if the point is valid and on-curve. */
  assertValidity() {
    const e = ar, r = ir, i = this;
    if (i.is0())
      throw new Error("bad point: ZERO");
    const { ex: o, ey: d, ez: h, et: l } = i, f = M(o * o), p = M(d * d), x = M(h * h), g = M(x * x), N = M(f * e), T = M(x * M(N + p)), b = M(g + M(r * M(f * p)));
    if (T !== b)
      throw new Error("bad point: equation left != right (1)");
    const E = M(o * d), y = M(h * l);
    if (E !== y)
      throw new Error("bad point: equation left != right (2)");
    return this;
  }
  /** Equality check: compare points P&Q. */
  equals(e) {
    const { ex: r, ey: i, ez: o } = this, { ex: d, ey: h, ez: l } = Vr(e), f = M(r * l), p = M(d * o), x = M(i * l), g = M(h * o);
    return f === p && x === g;
  }
  is0() {
    return this.equals(ut);
  }
  /** Flip point over y coordinate. */
  negate() {
    return new Ce(M(-this.ex), this.ey, this.ez, M(-this.et));
  }
  /** Point doubling. Complete formula. Cost: `4M + 4S + 1*a + 6add + 1*2`. */
  double() {
    const { ex: e, ey: r, ez: i } = this, o = ar, d = M(e * e), h = M(r * r), l = M(2n * M(i * i)), f = M(o * d), p = e + r, x = M(M(p * p) - d - h), g = f + h, N = g - l, T = f - h, b = M(x * N), E = M(g * T), y = M(x * T), w = M(N * g);
    return new Ce(b, E, w, y);
  }
  /** Point addition. Complete formula. Cost: `8M + 1*k + 8add + 1*2`. */
  add(e) {
    const { ex: r, ey: i, ez: o, et: d } = this, { ex: h, ey: l, ez: f, et: p } = Vr(e), x = ar, g = ir, N = M(r * h), T = M(i * l), b = M(d * g * p), E = M(o * f), y = M((r + i) * (h + l) - N - T), w = M(E - b), S = M(E + b), _ = M(T - x * N), R = M(y * w), F = M(S * _), P = M(y * _), C = M(w * S);
    return new Ce(R, F, C, P);
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
      return ut;
    if (Qe(e, 1n, Kt), e === 1n)
      return this;
    if (this.equals(ft))
      return Ls(e).p;
    let i = ut, o = ft;
    for (let d = this; e > 0n; d = d.double(), e >>= 1n)
      e & 1n ? i = i.add(d) : r && (o = o.add(d));
    return i;
  }
  /** Convert point to 2d xy affine point. (X, Y, Z) ∋ (x=X/Z, y=Y/Z) */
  toAffine() {
    const { ex: e, ey: r, ez: i } = this;
    if (this.equals(ut))
      return { x: 0n, y: 1n };
    const o = Nn(i, he);
    return M(i * o) !== 1n && Se("invalid inverse"), { x: M(e * o), y: M(r * o) };
  }
  toBytes() {
    const { x: e, y: r } = this.assertValidity().toAffine(), i = Ts(r);
    return i[31] |= e & 1n ? 128 : 0, i;
  }
  toHex() {
    return Ir(this.toBytes());
  }
  // encode to hex string
  clearCofactor() {
    return this.multiply(Vt(vs), !1);
  }
  isSmallOrder() {
    return this.clearCofactor().is0();
  }
  isTorsionFree() {
    let e = this.multiply(Kt / 2n, !1).double();
    return Kt % 2n && (e = e.add(this)), e.is0();
  }
  static fromHex(e, r) {
    return Ce.fromBytes(wn(e), r);
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
};
qe(Ce, "BASE"), qe(Ce, "ZERO");
let We = Ce;
const ft = new We($r, Kr, 1n, M($r * Kr)), ut = new We(0n, 1n, 1n, 0n);
We.BASE = ft;
We.ZERO = ut;
const Ts = (t) => kr(xn(Qe(t, 0n, gr), yn)).reverse(), Sn = (t) => Vt("0x" + Ir(Rr(mt(t)).reverse())), Te = (t, e) => {
  let r = t;
  for (; e-- > 0n; )
    r *= r, r %= he;
  return r;
}, Cs = (t) => {
  const r = t * t % he * t % he, i = Te(r, 2n) * r % he, o = Te(i, 1n) * t % he, d = Te(o, 5n) * o % he, h = Te(d, 10n) * d % he, l = Te(h, 20n) * h % he, f = Te(l, 40n) * l % he, p = Te(f, 80n) * f % he, x = Te(p, 80n) * f % he, g = Te(x, 10n) * d % he;
  return { pow_p_5_8: Te(g, 2n) * t % he, b2: r };
}, Gr = 0x2b8324804fc1df0b2b4d00993dfbd7a72f431806ad2fe478c4ee1b274a0ea0b0n, Rs = (t, e) => {
  const r = M(e * e * e), i = M(r * r * e), o = Cs(t * i).pow_p_5_8;
  let d = M(t * r * o);
  const h = M(e * d * d), l = d, f = M(d * Gr), p = h === t, x = h === M(-t), g = h === M(-t * Gr);
  return p && (d = l), (x || g) && (d = f), (M(d) & 1n) === 1n && (d = M(-d)), { isValid: p || x, value: d };
}, Is = (t) => _s(Sn(t)), ks = (...t) => yr.sha512Async(...t), Bs = (t) => {
  const e = t.slice(0, Ct);
  e[0] &= 248, e[31] &= 127, e[31] |= 64;
  const r = t.slice(Ct, yn), i = Is(e), o = ft.multiply(i), d = o.toBytes();
  return { head: e, prefix: r, scalar: i, point: o, pointBytes: d };
}, Fs = (t) => ks(wn(t, Ct)).then(Bs), Br = (t) => Fs(t).then((e) => e.pointBytes), yr = {
  sha512Async: async (...t) => {
    const e = Es(), r = jr(...t);
    return Qt(await e.digest("SHA-512", r.buffer));
  },
  sha512Sync: void 0,
  bytesToHex: Ir,
  hexToBytes: kr,
  concatBytes: jr,
  mod: M,
  invert: Nn,
  randomBytes: As
}, Gt = 8, Os = 256, En = Math.ceil(Os / Gt) + 1, br = 2 ** (Gt - 1), Ds = () => {
  const t = [];
  let e = ft, r = e;
  for (let i = 0; i < En; i++) {
    r = e, t.push(r);
    for (let o = 1; o < br; o++)
      r = r.add(e), t.push(r);
    e = r.double();
  }
  return t;
};
let zr;
const qr = (t, e) => {
  const r = e.negate();
  return t ? r : e;
}, Ls = (t) => {
  const e = zr || (zr = Ds());
  let r = ut, i = ft;
  const o = 2 ** Gt, d = o, h = Vt(o - 1), l = Vt(Gt);
  for (let f = 0; f < En; f++) {
    let p = Number(t & h);
    t >>= l, p > br && (p -= d, t += 1n);
    const x = f * br, g = x, N = x + Math.abs(p) - 1, T = f % 2 !== 0, b = p < 0;
    p === 0 ? i = i.add(qr(T, e[g])) : r = r.add(qr(b, e[N]));
  }
  return { p: r, f: i };
};
/*! noble-hashes - MIT License (c) 2022 Paul Miller (paulmillr.com) */
function Ms(t) {
  return t instanceof Uint8Array || ArrayBuffer.isView(t) && t.constructor.name === "Uint8Array";
}
function Fr(t, ...e) {
  if (!Ms(t))
    throw new Error("Uint8Array expected");
  if (e.length > 0 && !e.includes(t.length))
    throw new Error("Uint8Array expected of length " + e + ", got length=" + t.length);
}
function Qr(t, e = !0) {
  if (t.destroyed)
    throw new Error("Hash instance has been destroyed");
  if (e && t.finished)
    throw new Error("Hash#digest() has already been called");
}
function Ps(t, e) {
  Fr(t);
  const r = e.outputLen;
  if (t.length < r)
    throw new Error("digestInto() expects output buffer of length at least " + r);
}
function xr(...t) {
  for (let e = 0; e < t.length; e++)
    t[e].fill(0);
}
function or(t) {
  return new DataView(t.buffer, t.byteOffset, t.byteLength);
}
function Us(t) {
  if (typeof t != "string")
    throw new Error("string expected");
  return new Uint8Array(new TextEncoder().encode(t));
}
function An(t) {
  return typeof t == "string" && (t = Us(t)), Fr(t), t;
}
class $s {
}
function Ks(t) {
  const e = (i) => t().update(An(i)).digest(), r = t();
  return e.outputLen = r.outputLen, e.blockLen = r.blockLen, e.create = () => t(), e;
}
function Hs(t, e, r, i) {
  if (typeof t.setBigUint64 == "function")
    return t.setBigUint64(e, r, i);
  const o = BigInt(32), d = BigInt(4294967295), h = Number(r >> o & d), l = Number(r & d), f = i ? 4 : 0, p = i ? 0 : 4;
  t.setUint32(e + f, h, i), t.setUint32(e + p, l, i);
}
class js extends $s {
  constructor(e, r, i, o) {
    super(), this.finished = !1, this.length = 0, this.pos = 0, this.destroyed = !1, this.blockLen = e, this.outputLen = r, this.padOffset = i, this.isLE = o, this.buffer = new Uint8Array(e), this.view = or(this.buffer);
  }
  update(e) {
    Qr(this), e = An(e), Fr(e);
    const { view: r, buffer: i, blockLen: o } = this, d = e.length;
    for (let h = 0; h < d; ) {
      const l = Math.min(o - this.pos, d - h);
      if (l === o) {
        const f = or(e);
        for (; o <= d - h; h += o)
          this.process(f, h);
        continue;
      }
      i.set(e.subarray(h, h + l), this.pos), this.pos += l, h += l, this.pos === o && (this.process(r, 0), this.pos = 0);
    }
    return this.length += e.length, this.roundClean(), this;
  }
  digestInto(e) {
    Qr(this), Ps(e, this), this.finished = !0;
    const { buffer: r, view: i, blockLen: o, isLE: d } = this;
    let { pos: h } = this;
    r[h++] = 128, xr(this.buffer.subarray(h)), this.padOffset > o - h && (this.process(i, 0), h = 0);
    for (let g = h; g < o; g++)
      r[g] = 0;
    Hs(i, o - 8, BigInt(this.length * 8), d), this.process(i, 0);
    const l = or(e), f = this.outputLen;
    if (f % 4)
      throw new Error("_sha2: outputLen should be aligned to 32bit");
    const p = f / 4, x = this.get();
    if (p > x.length)
      throw new Error("_sha2: outputLen bigger than state");
    for (let g = 0; g < p; g++)
      l.setUint32(4 * g, x[g], d);
  }
  digest() {
    const { buffer: e, outputLen: r } = this;
    this.digestInto(e);
    const i = e.slice(0, r);
    return this.destroy(), i;
  }
  _cloneInto(e) {
    e || (e = new this.constructor()), e.set(...this.get());
    const { blockLen: r, buffer: i, length: o, finished: d, destroyed: h, pos: l } = this;
    return e.destroyed = h, e.finished = d, e.length = o, e.pos = l, o % r && e.buffer.set(i), e;
  }
  clone() {
    return this._cloneInto();
  }
}
const ue = /* @__PURE__ */ Uint32Array.from([
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
]), Lt = /* @__PURE__ */ BigInt(2 ** 32 - 1), Yr = /* @__PURE__ */ BigInt(32);
function Vs(t, e = !1) {
  return e ? { h: Number(t & Lt), l: Number(t >> Yr & Lt) } : { h: Number(t >> Yr & Lt) | 0, l: Number(t & Lt) | 0 };
}
function Gs(t, e = !1) {
  const r = t.length;
  let i = new Uint32Array(r), o = new Uint32Array(r);
  for (let d = 0; d < r; d++) {
    const { h, l } = Vs(t[d], e);
    [i[d], o[d]] = [h, l];
  }
  return [i, o];
}
const Wr = (t, e, r) => t >>> r, Jr = (t, e, r) => t << 32 - r | e >>> r, at = (t, e, r) => t >>> r | e << 32 - r, it = (t, e, r) => t << 32 - r | e >>> r, Mt = (t, e, r) => t << 64 - r | e >>> r - 32, Pt = (t, e, r) => t >>> r - 32 | e << 64 - r;
function Le(t, e, r, i) {
  const o = (e >>> 0) + (i >>> 0);
  return { h: t + r + (o / 2 ** 32 | 0) | 0, l: o | 0 };
}
const zs = (t, e, r) => (t >>> 0) + (e >>> 0) + (r >>> 0), qs = (t, e, r, i) => e + r + i + (t / 2 ** 32 | 0) | 0, Qs = (t, e, r, i) => (t >>> 0) + (e >>> 0) + (r >>> 0) + (i >>> 0), Ys = (t, e, r, i, o) => e + r + i + o + (t / 2 ** 32 | 0) | 0, Ws = (t, e, r, i, o) => (t >>> 0) + (e >>> 0) + (r >>> 0) + (i >>> 0) + (o >>> 0), Js = (t, e, r, i, o, d) => e + r + i + o + d + (t / 2 ** 32 | 0) | 0, _n = Gs([
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
].map((t) => BigInt(t))), Zs = _n[0], Xs = _n[1], $e = /* @__PURE__ */ new Uint32Array(80), Ke = /* @__PURE__ */ new Uint32Array(80);
class ea extends js {
  constructor(e = 64) {
    super(128, e, 16, !1), this.Ah = ue[0] | 0, this.Al = ue[1] | 0, this.Bh = ue[2] | 0, this.Bl = ue[3] | 0, this.Ch = ue[4] | 0, this.Cl = ue[5] | 0, this.Dh = ue[6] | 0, this.Dl = ue[7] | 0, this.Eh = ue[8] | 0, this.El = ue[9] | 0, this.Fh = ue[10] | 0, this.Fl = ue[11] | 0, this.Gh = ue[12] | 0, this.Gl = ue[13] | 0, this.Hh = ue[14] | 0, this.Hl = ue[15] | 0;
  }
  // prettier-ignore
  get() {
    const { Ah: e, Al: r, Bh: i, Bl: o, Ch: d, Cl: h, Dh: l, Dl: f, Eh: p, El: x, Fh: g, Fl: N, Gh: T, Gl: b, Hh: E, Hl: y } = this;
    return [e, r, i, o, d, h, l, f, p, x, g, N, T, b, E, y];
  }
  // prettier-ignore
  set(e, r, i, o, d, h, l, f, p, x, g, N, T, b, E, y) {
    this.Ah = e | 0, this.Al = r | 0, this.Bh = i | 0, this.Bl = o | 0, this.Ch = d | 0, this.Cl = h | 0, this.Dh = l | 0, this.Dl = f | 0, this.Eh = p | 0, this.El = x | 0, this.Fh = g | 0, this.Fl = N | 0, this.Gh = T | 0, this.Gl = b | 0, this.Hh = E | 0, this.Hl = y | 0;
  }
  process(e, r) {
    for (let _ = 0; _ < 16; _++, r += 4)
      $e[_] = e.getUint32(r), Ke[_] = e.getUint32(r += 4);
    for (let _ = 16; _ < 80; _++) {
      const R = $e[_ - 15] | 0, F = Ke[_ - 15] | 0, P = at(R, F, 1) ^ at(R, F, 8) ^ Wr(R, F, 7), C = it(R, F, 1) ^ it(R, F, 8) ^ Jr(R, F, 7), I = $e[_ - 2] | 0, O = Ke[_ - 2] | 0, U = at(I, O, 19) ^ Mt(I, O, 61) ^ Wr(I, O, 6), K = it(I, O, 19) ^ Pt(I, O, 61) ^ Jr(I, O, 6), V = Qs(C, K, Ke[_ - 7], Ke[_ - 16]), H = Ys(V, P, U, $e[_ - 7], $e[_ - 16]);
      $e[_] = H | 0, Ke[_] = V | 0;
    }
    let { Ah: i, Al: o, Bh: d, Bl: h, Ch: l, Cl: f, Dh: p, Dl: x, Eh: g, El: N, Fh: T, Fl: b, Gh: E, Gl: y, Hh: w, Hl: S } = this;
    for (let _ = 0; _ < 80; _++) {
      const R = at(g, N, 14) ^ at(g, N, 18) ^ Mt(g, N, 41), F = it(g, N, 14) ^ it(g, N, 18) ^ Pt(g, N, 41), P = g & T ^ ~g & E, C = N & b ^ ~N & y, I = Ws(S, F, C, Xs[_], Ke[_]), O = Js(I, w, R, P, Zs[_], $e[_]), U = I | 0, K = at(i, o, 28) ^ Mt(i, o, 34) ^ Mt(i, o, 39), V = it(i, o, 28) ^ Pt(i, o, 34) ^ Pt(i, o, 39), H = i & d ^ i & l ^ d & l, G = o & h ^ o & f ^ h & f;
      w = E | 0, S = y | 0, E = T | 0, y = b | 0, T = g | 0, b = N | 0, { h: g, l: N } = Le(p | 0, x | 0, O | 0, U | 0), p = l | 0, x = f | 0, l = d | 0, f = h | 0, d = i | 0, h = o | 0;
      const L = zs(U, V, G);
      i = qs(L, O, K, H), o = L | 0;
    }
    ({ h: i, l: o } = Le(this.Ah | 0, this.Al | 0, i | 0, o | 0)), { h: d, l: h } = Le(this.Bh | 0, this.Bl | 0, d | 0, h | 0), { h: l, l: f } = Le(this.Ch | 0, this.Cl | 0, l | 0, f | 0), { h: p, l: x } = Le(this.Dh | 0, this.Dl | 0, p | 0, x | 0), { h: g, l: N } = Le(this.Eh | 0, this.El | 0, g | 0, N | 0), { h: T, l: b } = Le(this.Fh | 0, this.Fl | 0, T | 0, b | 0), { h: E, l: y } = Le(this.Gh | 0, this.Gl | 0, E | 0, y | 0), { h: w, l: S } = Le(this.Hh | 0, this.Hl | 0, w | 0, S | 0), this.set(i, o, d, h, l, f, p, x, g, N, T, b, E, y, w, S);
  }
  roundClean() {
    xr($e, Ke);
  }
  destroy() {
    xr(this.buffer), this.set(0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0);
  }
}
const ta = /* @__PURE__ */ Ks(() => new ea()), ra = ta, na = (t) => typeof Buffer < "u" ? Buffer.from(t, "base64") : Uint8Array.from(atob(t), (e) => e.charCodeAt(0)), sa = (t) => {
  if (typeof Buffer < "u")
    return Buffer.from(t).toString("base64");
  const e = Array.from(t, (r) => String.fromCharCode(r)).join("");
  return btoa(e);
};
function Yt(t) {
  return na(t);
}
function aa(t) {
  return sa(t);
}
yr.sha512Sync = (...t) => ra(yr.concatBytes(...t));
const ia = {
  isAuthenticated: !1,
  systemPublicKey: null,
  systemKeyId: null,
  privateKey: null,
  publicKeyId: null,
  isLoading: !1,
  error: null
}, lr = Ze(
  "auth/initializeSystemKey",
  async (t, { rejectWithValue: e }) => {
    try {
      const r = await Cr();
      if (console.log("initializeSystemKey thunk response:", r), r.success && r.data && r.data.private_key) {
        const i = Yt(r.data.private_key), o = await Br(i);
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
), Ht = Ze(
  "auth/validatePrivateKey",
  async (t, { getState: e, rejectWithValue: r }) => {
    const i = e(), { systemPublicKey: o, systemKeyId: d } = i.auth;
    if (!o || !d)
      return r("System public key not available");
    try {
      console.log("🔑 Converting private key from base64...");
      const h = Yt(t);
      console.log("🔑 Generating public key from private key...");
      const l = await Br(h), f = btoa(
        String.fromCharCode(...l)
      ), p = f === o;
      return console.log("🔑 Key comparison:", {
        derived: f,
        system: o,
        matches: p
      }), p ? {
        privateKey: h,
        publicKeyId: d,
        isAuthenticated: !0
      } : r("Private key does not match system public key");
    } catch (h) {
      return console.error("Private key validation failed:", h), r(
        h instanceof Error ? h.message : "Private key validation failed"
      );
    }
  }
), cr = Ze(
  "auth/refreshSystemKey",
  async (t, { rejectWithValue: e }) => {
    for (let o = 1; o <= 5; o++)
      try {
        const d = await Cr();
        if (d.success && d.data && d.data.private_key) {
          const h = Yt(d.data.private_key), l = await Br(h);
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
      } catch (d) {
        if (o === 5)
          return e(
            d instanceof Error ? d.message : "Failed to fetch node private key"
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
), dr = Ze(
  "auth/fetchNodePrivateKey",
  async (t, { rejectWithValue: e }) => {
    try {
      const r = await Cr();
      return console.log("fetchNodePrivateKey thunk response:", r), r.success && r.data && r.data.private_key ? {
        privateKey: Yt(r.data.private_key),
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
), Or = Ze(
  "auth/loginUser",
  async (t, { rejectWithValue: e }) => {
    try {
      const i = new TextEncoder().encode(t), o = await crypto.subtle.digest("SHA-256", i), l = Array.from(new Uint8Array(o)).map((f) => f.toString(16).padStart(2, "0")).join("").substring(0, 32);
      return { id: t, hash: l };
    } catch {
      return e("Failed to generate user hash");
    }
  }
), Tn = vr({
  name: "auth",
  initialState: ia,
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
    t.addCase(lr.pending, (e) => {
      e.isLoading = !0, e.error = null;
    }).addCase(lr.fulfilled, (e, r) => {
      console.log("initializeSystemKey.fulfilled", r.payload), e.isLoading = !1, e.systemPublicKey = r.payload.systemPublicKey, e.systemKeyId = r.payload.systemKeyId, e.privateKey = r.payload.privateKey, e.error = null;
    }).addCase(lr.rejected, (e, r) => {
      e.isLoading = !1, e.error = r.payload;
    }).addCase(Ht.pending, (e) => {
      e.isLoading = !0, e.error = null;
    }).addCase(Ht.fulfilled, (e, r) => {
      e.isLoading = !1, e.isAuthenticated = r.payload.isAuthenticated, e.privateKey = r.payload.privateKey, e.publicKeyId = r.payload.publicKeyId, e.error = null;
    }).addCase(Ht.rejected, (e, r) => {
      e.isLoading = !1, e.isAuthenticated = !1, e.privateKey = null, e.publicKeyId = null, e.error = r.payload;
    }).addCase(cr.pending, (e) => {
      e.isLoading = !0, e.error = null;
    }).addCase(cr.fulfilled, (e, r) => {
      e.isLoading = !1, e.systemPublicKey = r.payload.systemPublicKey, e.systemKeyId = r.payload.systemKeyId, e.privateKey = r.payload.privateKey, e.user || (e.isAuthenticated = !1), e.error = null;
    }).addCase(cr.rejected, (e, r) => {
      e.isLoading = !1, e.systemPublicKey = null, e.systemKeyId = null, e.error = r.payload;
    }).addCase(dr.pending, (e) => {
      e.isLoading = !0, e.error = null;
    }).addCase(dr.fulfilled, (e, r) => {
      e.isLoading = !1, e.privateKey = r.payload.privateKey, e.publicKeyId = r.payload.publicKeyId, e.error = null;
    }).addCase(dr.rejected, (e, r) => {
      e.isLoading = !1, e.error = r.payload;
    }).addCase(Or.fulfilled, (e, r) => {
      e.isAuthenticated = !0, e.user = r.payload, e.error = null;
    });
  }
}), {
  clearAuthentication: oa,
  setError: Fo,
  clearError: Oo,
  updateSystemKey: Do,
  logoutUser: la,
  restoreSession: Lo
} = Tn.actions, ca = Tn.reducer, da = 3e5, ur = 3, It = {
  // Async thunk action types
  FETCH_SCHEMAS: "schemas/fetchSchemas",
  APPROVE_SCHEMA: "schemas/approveSchema",
  BLOCK_SCHEMA: "schemas/blockSchema",
  UNLOAD_SCHEMA: "schemas/unloadSchema",
  LOAD_SCHEMA: "schemas/loadSchema"
}, kt = {
  // Network and API errors
  FETCH_FAILED: "Failed to fetch schemas from server",
  // Schema operation errors
  APPROVE_FAILED: "Failed to approve schema",
  BLOCK_FAILED: "Failed to block schema",
  UNLOAD_FAILED: "Failed to unload schema",
  LOAD_FAILED: "Failed to load schema"
}, Fe = {
  AVAILABLE: "available",
  APPROVED: "approved",
  BLOCKED: "blocked",
  LOADING: "loading",
  ERROR: "error"
};
process.env.NODE_ENV, process.env.NODE_ENV;
process.env.NODE_ENV, process.env.NODE_ENV;
const ua = {
  MUTATION_WRAPPER_KEY: "value"
}, ha = 200, ma = 300, fa = [
  // Main features
  { id: "ingestion", label: "Ingestion", icon: "📥", group: "main" },
  { id: "file-upload", label: "File Upload", icon: "📄", group: "main" },
  { id: "llm-query", label: "AI Query", icon: "🤖", group: "main" },
  // Developer/Advanced features
  { id: "schemas", label: "Schemas", icon: "📊", group: "advanced" },
  { id: "query", label: "Query", icon: "🔍", group: "advanced" },
  { id: "mutation", label: "Mutation", icon: "✏️", group: "advanced" },
  { id: "native-index", label: "Native Index Query", icon: "🧭", group: "advanced" }
], Ut = {
  executeQuery: "Execute Query"
}, Ye = {
  schema: "Schema",
  schemaEmpty: "No schemas available",
  schemaHelp: "Select a schema to work with",
  operationType: "Operation Type",
  operationHelp: "Select the type of operation to perform"
}, pa = {
  loading: "Loading..."
}, ga = [
  { value: "Insert", label: "Insert" },
  { value: "Update", label: "Update" },
  { value: "Delete", label: "Delete" }
], Cn = {
  Insert: "create",
  Create: "create",
  Update: "update",
  Delete: "delete"
}, Zr = {
  approved: "bg-green-100 text-green-800",
  available: "bg-blue-100 text-blue-800",
  blocked: "bg-red-100 text-red-800",
  pending: "bg-yellow-100 text-yellow-800"
}, Rn = {
  schemaStates: {
    approved: "Schema is approved for use in queries and mutations",
    available: "Schema is available but requires approval before use",
    blocked: "Schema is blocked and cannot be used",
    pending: "Schema approval is pending review",
    unknown: "Schema state is unknown or invalid"
  }
}, Xr = {
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
function Dr(t) {
  return !t || typeof t != "object" ? !1 : In(t) === "HashRange";
}
function kn(t) {
  if (typeof t != "string") return null;
  const e = t.split(".");
  return e[e.length - 1] || t;
}
function pt(t) {
  return !t || typeof t != "object" ? !1 : In(t) === "Range";
}
function et(t) {
  var r;
  if (!t || typeof t != "object") return null;
  const e = (r = t == null ? void 0 : t.key) == null ? void 0 : r.range_field;
  return typeof e == "string" && e.trim() ? kn(e) : null;
}
function Bn(t) {
  var r;
  if (!t || typeof t != "object") return null;
  const e = (r = t == null ? void 0 : t.key) == null ? void 0 : r.hash_field;
  return e && typeof e == "string" && e.trim() ? kn(e) : null;
}
function ya(t) {
  if (!pt(t))
    return {};
  const e = et(t);
  if (!Array.isArray(t.fields))
    throw new Error(`Expected schema.fields to be an array for range schema "${t.name}", got ${typeof t.fields}`);
  return t.fields.reduce((r, i) => (i !== e && (r[i] = {}), r), {});
}
function ba(t, e, r, i) {
  const o = typeof e == "string" ? Cn[e] || e.toLowerCase() : "", d = o === "delete", h = {
    type: "mutation",
    schema: t.name,
    mutation_type: o
  }, l = et(t);
  if (d)
    h.fields_and_values = {}, h.key_value = { hash: null, range: null }, r && r.trim() && l && (h.fields_and_values[l] = r.trim(), h.key_value.range = r.trim());
  else {
    const f = {};
    r && r.trim() && l && (f[l] = r.trim()), Object.entries(i).forEach(([p, x]) => {
      if (p !== l) {
        const g = ua.MUTATION_WRAPPER_KEY;
        typeof x == "string" || typeof x == "number" || typeof x == "boolean" ? f[p] = { [g]: x } : typeof x == "object" && x !== null ? f[p] = x : f[p] = { [g]: x };
      }
    }), h.fields_and_values = f, h.key_value = {
      hash: null,
      range: r && r.trim() ? r.trim() : null
    };
  }
  return h;
}
function xa(t) {
  return pt(t) ? {
    isRangeSchema: !0,
    rangeKey: et(t),
    rangeFields: [],
    // Declarative schemas don't store field types
    nonRangeKeyFields: ya(t),
    totalFields: Array.isArray(t.fields) ? t.fields.length : 0
  } : null;
}
function Fn(t) {
  return typeof t == "string" ? t.toLowerCase() : typeof t == "object" && t !== null ? t.state ? String(t.state).toLowerCase() : String(t).toLowerCase() : String(t || "").toLowerCase();
}
function wa(t) {
  return t == null;
}
function va(t) {
  return Dr(t) ? {
    isHashRangeSchema: !0,
    hashField: Bn(t),
    rangeField: et(t),
    totalFields: Array.isArray(t.fields) ? t.fields.length : 0
  } : null;
}
const $t = ve.AVAILABLE, Na = /* @__PURE__ */ new Set([
  ve.AVAILABLE,
  ve.APPROVED,
  ve.BLOCKED,
  "loading",
  "error"
]);
function Sa(t) {
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
function Ea(t) {
  var r;
  return !t || typeof t != "object" ? void 0 : [
    t.state,
    t.schema_state,
    t.schemaState,
    t.status,
    t.current_state,
    (r = t.schema) == null ? void 0 : r.state
  ].find((i) => i !== void 0);
}
class Wt {
  constructor(e) {
    this.client = e || Ae({
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
    const e = await this.client.get(Y.LIST_SCHEMAS, {
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
    return this.client.get(Y.GET_SCHEMA(e), {
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
    var o;
    if (!Object.values(ve).includes(e))
      throw new Error(`Invalid schema state: ${e}. Must be one of: ${Object.values(ve).join(", ")}`);
    const r = await this.getSchemas();
    return !r.success || !r.data ? { success: !1, error: "Failed to fetch schemas", status: r.status, data: { data: [], state: e } } : {
      success: !0,
      data: { data: r.data.filter((d) => d.state === e).map((d) => d.name), state: e },
      status: 200,
      meta: { timestamp: Date.now(), cached: ((o = r.meta) == null ? void 0 : o.cached) || !1 }
    };
  }
  /**
   * Get all schemas with their state mappings (computed client-side)
   * UNPROTECTED - No authentication required
   */
  async getAllSchemasWithState() {
    var o;
    const e = await this.getSchemas();
    if (!e.success || !e.data)
      return {
        success: !1,
        error: "Failed to fetch schemas",
        status: e.status,
        data: {}
      };
    const r = Array.isArray(e.data) ? e.data : [], i = {};
    return r.forEach((d) => {
      const h = Sa(d);
      if (!h) {
        typeof console < "u" && console.warn && console.warn("[schemaClient.getAllSchemasWithState] Encountered schema entry without a name, skipping entry.");
        return;
      }
      const l = Ea(d), f = Fn(l);
      if (!l || f.length === 0) {
        typeof console < "u" && console.warn && console.warn(
          `[schemaClient.getAllSchemasWithState] Missing schema state for '${h}', defaulting to '${$t}'.`
        ), i[h] = $t;
        return;
      }
      if (!Na.has(f)) {
        typeof console < "u" && console.warn && console.warn(
          `[schemaClient.getAllSchemasWithState] Unrecognized schema state '${String(l)}' for '${h}', defaulting to '${$t}'.`
        ), i[h] = $t;
        return;
      }
      i[h] = f;
    }), {
      success: !0,
      data: i,
      status: e.status ?? 200,
      meta: {
        ...e.meta,
        timestamp: Date.now(),
        cached: ((o = e.meta) == null ? void 0 : o.cached) ?? !1
      }
    };
  }
  /**
   * Get schema status summary (computed client-side)
   * UNPROTECTED - No authentication required
   */
  async getSchemaStatus() {
    var o;
    const e = await this.getSchemas();
    if (!e.success || !e.data)
      return { success: !1, error: "Failed to fetch schemas", status: e.status, data: { available: 0, approved: 0, blocked: 0, total: 0 } };
    const r = e.data;
    return { success: !0, data: {
      available: r.filter((d) => d.state === ve.AVAILABLE).length,
      approved: r.filter((d) => d.state === ve.APPROVED).length,
      blocked: r.filter((d) => d.state === ve.BLOCKED).length,
      total: r.length
    }, status: 200, meta: { timestamp: Date.now(), cached: ((o = e.meta) == null ? void 0 : o.cached) || !1 } };
  }
  /**
   * Approve a schema (transition to approved state)
   * UNPROTECTED - No authentication required
   * SCHEMA-002 Compliance: Only available schemas can be approved
   */
  async approveSchema(e) {
    return this.client.post(
      Y.APPROVE_SCHEMA(e),
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
      Y.BLOCK_SCHEMA(e),
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
    var e;
    try {
      const r = await this.getSchemas();
      return !r.success || !r.data ? { success: !1, error: "Failed to fetch schemas", status: r.status, data: [] } : { success: !0, data: r.data.filter((o) => o.state === ve.APPROVED), status: 200, meta: { timestamp: Date.now(), cached: (e = r.meta) == null ? void 0 : e.cached } };
    } catch (r) {
      return { success: !1, error: r.message || "Failed to fetch approved schemas", status: r.status || 500, data: [] };
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
      return o.state !== ve.APPROVED ? {
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
const re = new Wt();
function Aa(t) {
  return new Wt(t);
}
re.getSchemasByState.bind(re);
re.getAllSchemasWithState.bind(re);
re.getSchemaStatus.bind(re);
re.getSchema.bind(re);
re.approveSchema.bind(re);
re.blockSchema.bind(re);
re.loadSchema.bind(re);
re.unloadSchema.bind(re);
re.getApprovedSchemas.bind(re);
re.validateSchemaForOperation.bind(re);
re.getBackfillStatus.bind(re);
const pe = {
  APPROVE: "approve",
  BLOCK: "block",
  UNLOAD: "unload",
  LOAD: "load"
}, On = (t, e) => t ? Date.now() - t < e : !1, _a = (t, e, r = Date.now()) => ({
  schemaName: t,
  error: e,
  timestamp: r
}), Ta = (t, e, r, i) => ({
  schemaName: t,
  newState: e,
  timestamp: Date.now(),
  updatedSchema: r,
  backfillHash: i
}), Jt = (t, e, r, i) => Ze(
  t,
  async ({ schemaName: o, options: d = {} }, { getState: h, rejectWithValue: l }) => {
    var p;
    h().schemas.schemas[o];
    try {
      const x = await e(o);
      if (!x.success)
        throw new Error(x.error || i);
      const g = (p = x.data) == null ? void 0 : p.backfill_hash;
      return Ta(o, r, void 0, g);
    } catch (x) {
      return l(
        _a(
          o,
          x instanceof Error ? x.message : i
        )
      );
    }
  }
), be = (t, e) => ({
  pending: (r, i) => {
    const o = i.meta.arg.schemaName;
    r.loading.operations[o] = !0, delete r.errors.operations[o];
  },
  fulfilled: (r, i) => {
    const { schemaName: o, newState: d, updatedSchema: h } = i.payload;
    r.loading.operations[o] = !1, r.schemas[o] && (r.schemas[o].state = d, h && Object.assign(r.schemas[o], h), r.schemas[o].lastOperation = {
      type: e,
      timestamp: Date.now(),
      success: !0
    });
  },
  rejected: (r, i) => {
    const { schemaName: o, error: d } = i.payload;
    r.loading.operations[o] = !1, r.errors.operations[o] = d, r.schemas[o] && (r.schemas[o].lastOperation = {
      type: e,
      timestamp: Date.now(),
      success: !1,
      error: d
    });
  }
}), en = {
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
    ttl: da,
    version: "1.0.0",
    lastUpdated: null
  },
  activeSchema: null
}, Ie = Ze(
  It.FETCH_SCHEMAS,
  async (t = {}, { getState: e, rejectWithValue: r }) => {
    const i = e(), { lastFetched: o, cache: d } = i.schemas;
    if (!t.forceRefresh && On(o, d.ttl))
      return {
        schemas: Object.values(i.schemas.schemas),
        timestamp: o
      };
    const h = new Wt(
      Ae({
        baseUrl: qt.BASE_URL,
        // Use main API base URL (/api)
        enableCache: !0,
        enableLogging: !0,
        enableMetrics: !0
      })
    );
    t.forceRefresh && (console.log("🔄 Force refresh requested - clearing API client cache"), h.clearCache());
    let l = null;
    for (let p = 1; p <= ur; p++)
      try {
        const x = await h.getSchemas();
        if (!x.success)
          throw new Error(`Failed to fetch schemas: ${x.error || "Unknown error"}`);
        console.log("📁 Raw schemas response:", x.data);
        const g = x.data || [];
        if (!Array.isArray(g))
          throw new Error(`Schemas response is not an array: ${typeof g}`);
        const N = g.map((b) => {
          if (!b.name)
            if (console.warn("⚠️ Schema missing name field:", b), b.schema && b.schema.name)
              b.name = b.schema.name;
            else
              return console.error("❌ Schema has no name field and cannot be displayed:", b), null;
          let E = Fe.AVAILABLE;
          return b.state && (typeof b.state == "string" ? E = b.state.toLowerCase() : typeof b.state == "object" && b.state.state ? E = String(b.state.state).toLowerCase() : E = String(b.state).toLowerCase()), console.log("🟢 fetchSchemas: Using backend schema for", b.name, "with state:", E), {
            ...b,
            state: E
          };
        }).filter((b) => b !== null);
        console.log("✅ Using backend schemas directly:", N.map((b) => ({ name: b.name, state: b.state })));
        const T = Date.now();
        return {
          schemas: N,
          timestamp: T
        };
      } catch (x) {
        if (l = x instanceof Error ? x : new Error("Unknown error"), p < ur) {
          const N = typeof window < "u" && window.__TEST_ENV__ === !0 ? 10 : 1e3 * p;
          await new Promise((T) => setTimeout(T, N));
        }
      }
    const f = `Failed to fetch schemas after ${ur} attempts: ${(l == null ? void 0 : l.message) || "Unknown error"}`;
    return r(f);
  }
), Zt = () => new Wt(
  Ae({
    baseUrl: qt.BASE_URL,
    // Use main API base URL (/api)
    enableCache: !0,
    enableLogging: !0,
    enableMetrics: !0
  })
), He = Jt(
  It.APPROVE_SCHEMA,
  (t) => Zt().approveSchema(t),
  Fe.APPROVED,
  kt.APPROVE_FAILED
), je = Jt(
  It.BLOCK_SCHEMA,
  (t) => Zt().blockSchema(t),
  Fe.BLOCKED,
  kt.BLOCK_FAILED
), ot = Jt(
  It.UNLOAD_SCHEMA,
  (t) => Zt().unloadSchema(t),
  Fe.AVAILABLE,
  kt.UNLOAD_FAILED
), lt = Jt(
  It.LOAD_SCHEMA,
  (t) => Zt().loadSchema(t),
  Fe.APPROVED,
  kt.LOAD_FAILED
), Dn = vr({
  name: "schemas",
  initialState: en,
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
        type: pe.APPROVE,
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
      Object.assign(t, en);
    }
  },
  extraReducers: (t) => {
    t.addCase(Ie.pending, (e) => {
      e.loading.fetch = !0, e.errors.fetch = null;
    }).addCase(Ie.fulfilled, (e, r) => {
      e.loading.fetch = !1, e.errors.fetch = null;
      const i = {};
      r.payload.schemas.forEach((o) => {
        i[o.name] = o;
      }), e.schemas = i, e.lastFetched = r.payload.timestamp, e.cache.lastUpdated = r.payload.timestamp;
    }).addCase(Ie.rejected, (e, r) => {
      e.loading.fetch = !1, e.errors.fetch = r.payload || kt.FETCH_FAILED;
    }).addCase(He.pending, be(He, pe.APPROVE).pending).addCase(He.fulfilled, be(He, pe.APPROVE).fulfilled).addCase(He.rejected, be(He, pe.APPROVE).rejected).addCase(je.pending, be(je, pe.BLOCK).pending).addCase(je.fulfilled, be(je, pe.BLOCK).fulfilled).addCase(je.rejected, be(je, pe.BLOCK).rejected).addCase(ot.pending, be(ot, pe.UNLOAD).pending).addCase(ot.fulfilled, be(ot, pe.UNLOAD).fulfilled).addCase(ot.rejected, be(ot, pe.UNLOAD).rejected).addCase(lt.pending, be(lt, pe.LOAD).pending).addCase(lt.fulfilled, be(lt, pe.LOAD).fulfilled).addCase(lt.rejected, be(lt, pe.LOAD).rejected);
  }
}), Ca = (t) => t.schemas, gt = (t) => Object.values(t.schemas.schemas), Ra = (t) => t.schemas.schemas, Bt = Xe(
  [gt],
  (t) => t.filter((e) => (typeof e.state == "string" ? e.state.toLowerCase() : typeof e.state == "object" && e.state !== null && e.state.state ? String(e.state.state).toLowerCase() : String(e.state || "").toLowerCase()) === Fe.APPROVED)
), Ia = Xe(
  [gt],
  (t) => t.filter((e) => e.state === Fe.AVAILABLE)
);
Xe(
  [gt],
  (t) => t.filter((e) => e.state === Fe.BLOCKED)
);
Xe(
  [Bt],
  (t) => t.filter((e) => {
    var r;
    return ((r = e.rangeInfo) == null ? void 0 : r.isRangeSchema) === !0;
  })
);
Xe(
  [Ia],
  (t) => t.filter((e) => {
    var r;
    return ((r = e.rangeInfo) == null ? void 0 : r.isRangeSchema) === !0;
  })
);
const Lr = (t) => t.schemas.loading.fetch, Ln = (t) => t.schemas.errors.fetch, ka = Xe(
  [Ca],
  (t) => ({
    isValid: On(t.lastFetched, t.cache.ttl),
    lastFetched: t.lastFetched,
    ttl: t.cache.ttl
  })
), Ba = (t) => t.schemas.activeSchema;
Xe(
  [Ba, Ra],
  (t, e) => t && e[t] || null
);
const {
  setActiveSchema: Mo,
  updateSchemaStatus: Po,
  setLoading: Uo,
  setError: $o,
  clearError: Ko,
  clearOperationError: Ho,
  invalidateCache: jo,
  resetSchemas: Vo
} = Dn.actions, Fa = Dn.reducer, tn = {
  inputText: "",
  sessionId: null,
  isProcessing: !1,
  conversationLog: [],
  showResults: !1
}, Mn = vr({
  name: "aiQuery",
  initialState: tn,
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
    resetAIQueryState: () => tn
  }
}), {
  setInputText: rn,
  clearInputText: Go,
  setSessionId: nn,
  setIsProcessing: sn,
  addMessage: Oa,
  clearConversation: zo,
  setShowResults: Da,
  startNewConversation: La,
  resetAIQueryState: qo
} = Mn.actions, Ma = Mn.reducer, Pa = (t) => t.aiQuery.inputText, Ua = (t) => t.aiQuery.sessionId, $a = (t) => t.aiQuery.isProcessing, Ka = (t) => t.aiQuery.conversationLog, Ha = (t) => t.aiQuery.showResults, ja = (t) => t.aiQuery.sessionId && t.aiQuery.conversationLog.some((e) => e.type === "results"), Va = os({
  reducer: {
    auth: ca,
    schemas: Fa,
    aiQuery: Ma
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
function Ga() {
  console.log("🔄 Schema client reset - will use new configuration on next request");
}
async function za(t) {
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
    const d = await fetch(i, o), h = Date.now() - e;
    return d.ok ? {
      success: !0,
      status: (await d.json()).status || "online",
      responseTime: h
    } : {
      success: !1,
      error: `HTTP ${d.status}: ${d.statusText}`,
      responseTime: h
    };
  } catch (o) {
    const d = Date.now() - e;
    return {
      success: !1,
      error: o.name === "TimeoutError" ? "Connection timeout" : o.message,
      responseTime: d
    };
  }
}
const ht = {
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
}, an = "schemaServiceEnvironment", Pn = rs({
  environment: ht.LOCAL,
  setEnvironment: () => {
  },
  getSchemaServiceBaseUrl: () => ""
});
function qa({ children: t }) {
  const [e, r] = B(() => {
    const d = localStorage.getItem(an);
    if (d) {
      const h = Object.values(ht).find((l) => l.id === d);
      if (h) return h;
    }
    return ht.LOCAL;
  }), i = (d) => {
    const h = Object.values(ht).find((l) => l.id === d);
    h && (r(h), localStorage.setItem(an, d), Ga(), console.log(`Schema service environment changed to: ${h.name} (${h.baseUrl || "same origin"})`), console.log("🔄 Schema client has been reset - next request will use new endpoint"));
  }, o = () => e.baseUrl || "";
  return /* @__PURE__ */ n(Pn.Provider, { value: { environment: e, setEnvironment: i, getSchemaServiceBaseUrl: o }, children: t });
}
function Qa() {
  const t = ns(Pn);
  if (!t)
    throw new Error("useSchemaServiceConfig must be used within SchemaServiceConfigProvider");
  return t;
}
const Qo = ({ children: t, store: e }) => /* @__PURE__ */ n(ss, { store: e || Va, children: /* @__PURE__ */ n(qa, { children: t }) });
function Ya({
  title: t,
  titleId: e,
  ...r
}, i) {
  return /* @__PURE__ */ q.createElement("svg", Object.assign({
    xmlns: "http://www.w3.org/2000/svg",
    viewBox: "0 0 24 24",
    fill: "currentColor",
    "aria-hidden": "true",
    "data-slot": "icon",
    ref: i,
    "aria-labelledby": e
  }, r), t ? /* @__PURE__ */ q.createElement("title", {
    id: e
  }, t) : null, /* @__PURE__ */ q.createElement("path", {
    fillRule: "evenodd",
    d: "M2.25 12c0-5.385 4.365-9.75 9.75-9.75s9.75 4.365 9.75 9.75-4.365 9.75-9.75 9.75S2.25 17.385 2.25 12Zm13.36-1.814a.75.75 0 1 0-1.22-.872l-3.236 4.53L9.53 12.22a.75.75 0 0 0-1.06 1.06l2.25 2.25a.75.75 0 0 0 1.14-.094l3.75-5.25Z",
    clipRule: "evenodd"
  }));
}
const Wa = /* @__PURE__ */ q.forwardRef(Ya);
function Ja({
  title: t,
  titleId: e,
  ...r
}, i) {
  return /* @__PURE__ */ q.createElement("svg", Object.assign({
    xmlns: "http://www.w3.org/2000/svg",
    viewBox: "0 0 24 24",
    fill: "currentColor",
    "aria-hidden": "true",
    "data-slot": "icon",
    ref: i,
    "aria-labelledby": e
  }, r), t ? /* @__PURE__ */ q.createElement("title", {
    id: e
  }, t) : null, /* @__PURE__ */ q.createElement("path", {
    fillRule: "evenodd",
    d: "M12.53 16.28a.75.75 0 0 1-1.06 0l-7.5-7.5a.75.75 0 0 1 1.06-1.06L12 14.69l6.97-6.97a.75.75 0 1 1 1.06 1.06l-7.5 7.5Z",
    clipRule: "evenodd"
  }));
}
const Un = /* @__PURE__ */ q.forwardRef(Ja);
function Za({
  title: t,
  titleId: e,
  ...r
}, i) {
  return /* @__PURE__ */ q.createElement("svg", Object.assign({
    xmlns: "http://www.w3.org/2000/svg",
    viewBox: "0 0 24 24",
    fill: "currentColor",
    "aria-hidden": "true",
    "data-slot": "icon",
    ref: i,
    "aria-labelledby": e
  }, r), t ? /* @__PURE__ */ q.createElement("title", {
    id: e
  }, t) : null, /* @__PURE__ */ q.createElement("path", {
    fillRule: "evenodd",
    d: "M16.28 11.47a.75.75 0 0 1 0 1.06l-7.5 7.5a.75.75 0 0 1-1.06-1.06L14.69 12 7.72 5.03a.75.75 0 0 1 1.06-1.06l7.5 7.5Z",
    clipRule: "evenodd"
  }));
}
const $n = /* @__PURE__ */ q.forwardRef(Za);
function Xa({
  title: t,
  titleId: e,
  ...r
}, i) {
  return /* @__PURE__ */ q.createElement("svg", Object.assign({
    xmlns: "http://www.w3.org/2000/svg",
    viewBox: "0 0 24 24",
    fill: "currentColor",
    "aria-hidden": "true",
    "data-slot": "icon",
    ref: i,
    "aria-labelledby": e
  }, r), t ? /* @__PURE__ */ q.createElement("title", {
    id: e
  }, t) : null, /* @__PURE__ */ q.createElement("path", {
    fillRule: "evenodd",
    d: "M16.5 4.478v.227a48.816 48.816 0 0 1 3.878.512.75.75 0 1 1-.256 1.478l-.209-.035-1.005 13.07a3 3 0 0 1-2.991 2.77H8.084a3 3 0 0 1-2.991-2.77L4.087 6.66l-.209.035a.75.75 0 0 1-.256-1.478A48.567 48.567 0 0 1 7.5 4.705v-.227c0-1.564 1.213-2.9 2.816-2.951a52.662 52.662 0 0 1 3.369 0c1.603.051 2.815 1.387 2.815 2.951Zm-6.136-1.452a51.196 51.196 0 0 1 3.273 0C14.39 3.05 15 3.684 15 4.478v.113a49.488 49.488 0 0 0-6 0v-.113c0-.794.609-1.428 1.364-1.452Zm-.355 5.945a.75.75 0 1 0-1.5.058l.347 9a.75.75 0 1 0 1.499-.058l-.346-9Zm5.48.058a.75.75 0 1 0-1.498-.058l-.347 9a.75.75 0 0 0 1.5.058l.345-9Z",
    clipRule: "evenodd"
  }));
}
const on = /* @__PURE__ */ q.forwardRef(Xa);
class Kn {
  constructor(e) {
    this.client = e || Ae({
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
      Y.GET_SYSTEM_PUBLIC_KEY,
      {
        requiresAuth: !1,
        // System public key is public
        timeout: le.QUICK,
        retries: ce.CRITICAL,
        // Multiple retries for critical system data
        cacheable: !0,
        // Cache system public key
        cacheTtl: Ve.SYSTEM_PUBLIC_KEY,
        // Cache for 1 hour (system key doesn't change often)
        cacheKey: jt.SYSTEM_PUBLIC_KEY
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
    return this.client.get(Y.GET_SYSTEM_STATUS, {
      timeout: le.QUICK,
      retries: ce.STANDARD,
      cacheable: !0,
      cacheTtl: Ve.SECURITY_STATUS,
      // Cache for 1 minute
      cacheKey: jt.SECURITY_STATUS
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
const Be = new Kn();
function ei(t) {
  return new Kn(t);
}
Be.getSystemPublicKey.bind(Be);
Be.validatePublicKeyFormat.bind(Be);
Be.validateSignedMessage.bind(Be);
Be.getSecurityStatus.bind(Be);
Be.verifyMessage.bind(Be);
class ti {
  constructor(e) {
    this.client = e || Ae({
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
    return this.client.get(Y.LIST_TRANSFORMS, {
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
    return this.client.get(Y.GET_TRANSFORM_QUEUE, {
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
      Y.ADD_TO_TRANSFORM_QUEUE(e),
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
    return this.client.get(Y.GET_ALL_BACKFILLS, {
      requiresAuth: !1,
      timeout: 5e3,
      retries: 2,
      cacheable: !1
    });
  }
  /**
   * Get active (in-progress) backfills
   * UNPROTECTED - No authentication required for backfill monitoring
   * 
   * @returns Promise resolving to active backfill information
   */
  async getActiveBackfills() {
    return this.client.get(Y.GET_ACTIVE_BACKFILLS, {
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
    return this.client.get(Y.GET_BACKFILL(e), {
      requiresAuth: !1,
      timeout: 5e3,
      retries: 2,
      cacheable: !1
    });
  }
  /**
   * Get transform execution statistics
   * UNPROTECTED - No authentication required for statistics monitoring
   * 
   * @returns Promise resolving to transform statistics
   */
  async getStatistics() {
    return this.client.get(Y.GET_TRANSFORM_STATISTICS, {
      requiresAuth: !1,
      timeout: 5e3,
      retries: 2,
      cacheable: !1
    });
  }
  /**
   * Get backfill-specific statistics aggregated from all backfills
   * UNPROTECTED - No authentication required for backfill monitoring
   * 
   * @returns Promise resolving to backfill statistics
   */
  async getBackfillStatistics() {
    return this.client.get(Y.GET_BACKFILL_STATISTICS, {
      requiresAuth: !1,
      timeout: 5e3,
      retries: 2,
      cacheable: !1
    });
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
const fe = new ti();
fe.getTransforms.bind(fe);
fe.getQueue.bind(fe);
fe.addToQueue.bind(fe);
fe.refreshQueue.bind(fe);
fe.getTransform.bind(fe);
class Hn {
  constructor(e) {
    this.client = e || Ae({
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
      Y.EXECUTE_MUTATION,
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
      Y.EXECUTE_MUTATIONS_BATCH,
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
    return this.client.post(Y.EXECUTE_QUERY, e, {
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
      Y.EXECUTE_QUERY,
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
        Y.GET_SCHEMA(e),
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
      const i = r.data, o = i.state === ve.APPROVED;
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
const Mr = new Hn();
function ri(t) {
  return new Hn(t);
}
class ni {
  constructor(e) {
    this.client = e || Ae({
      baseUrl: cs.ROOT,
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
    return this.client.get(Y.GET_STATUS, {
      requiresAuth: !1,
      // Status endpoint is public
      timeout: le.QUICK,
      retries: ce.STANDARD,
      cacheable: !1
      // Status should always be fresh
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
      Y.GET_INGESTION_CONFIG,
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
      Y.GET_INGESTION_CONFIG,
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
      Y.VALIDATE_JSON,
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
    const i = {
      data: e,
      auto_execute: r.autoExecute ?? !0,
      trust_distance: r.trustDistance ?? 0,
      pub_key: r.pubKey ?? "default",
      progress_id: r.progressId || crypto.randomUUID()
    }, o = this.validateIngestionRequest(i);
    if (!o.isValid)
      throw new Error(
        `Invalid ingestion request: ${o.errors.join(", ")}`
      );
    return this.client.post(
      Y.PROCESS_JSON,
      i,
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
      progress_id: r.progressId || crypto.randomUUID()
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
  /**
   * Get ingestion progress by ID
   *
   * @param id Progress ID
   * @returns Promise resolving to progress information
   */
  async getProgress(e) {
    return this.client.get(`/ingestion/progress/${e}`, {
      requiresAuth: !1,
      timeout: le.QUICK,
      retries: ce.STANDARD,
      cacheable: !1
    });
  }
  /**
   * Get all active ingestion progress
   *
   * @returns Promise resolving to all active progress
   */
  async getAllProgress() {
    return this.client.get("/ingestion/progress", {
      requiresAuth: !1,
      timeout: le.QUICK,
      retries: ce.STANDARD,
      cacheable: !1
    });
  }
  clearCache() {
    this.client.clearCache();
  }
}
const Je = new ni(), ct = Ae({
  timeout: le.AI_PROCESSING,
  retries: ce.LIMITED
}), ln = {
  /**
   * Run a query in a single step (analyze + execute with internal polling loop)
   */
  async runQuery(t) {
    return ct.post("/llm-query/run", t);
  },
  /**
   * Analyze a natural language query
   */
  async analyzeQuery(t) {
    return ct.post("/llm-query/analyze", t);
  },
  /**
   * Execute a query plan
   */
  async executeQueryPlan(t) {
    return ct.post("/llm-query/execute", t);
  },
  /**
   * Ask a follow-up question about results
   */
  async chat(t) {
    return ct.post("/llm-query/chat", t);
  },
  /**
   * Analyze if a follow-up question can be answered from existing context
   */
  async analyzeFollowup(t) {
    return ct.post("/llm-query/analyze-followup", t);
  },
  /**
   * Get backfill status by hash
   */
  async getBackfillStatus(t) {
    return ct.get(`/llm-query/backfill/${t}`);
  }
};
class si {
  constructor(e) {
    this.client = e || Ae({ enableCache: !0, enableLogging: !0 });
  }
  async search(e) {
    const r = `${Y.NATIVE_INDEX_SEARCH}?term=${encodeURIComponent(e)}`;
    return this.client.get(r, {
      timeout: 8e3,
      retries: 2,
      cacheable: !0,
      cacheTtl: 6e4
    });
  }
}
const ai = new si();
function Yo() {
  const [t, e] = B(!1), [r, i] = B(!1), [o, d] = B(null), [h, l] = B([]);
  de(() => {
    let g = !0, N;
    const T = async () => {
      try {
        const b = await Je.getAllProgress();
        if (g && b.success && b.data) {
          const E = b.data.progress || [], y = E.filter((_) => !_.is_complete), w = E.filter((_) => _.is_complete).sort((_, R) => (R.completed_at || 0) - (_.completed_at || 0)).slice(0, 1);
          let S = [];
          y.length > 0 ? S = [...y].sort(
            (_, R) => (R.started_at || 0) - (_.started_at || 0)
          ).slice(0, 3) : w.length > 0 && (S = w), l(S);
        }
        g && (N = setTimeout(T, 1e3));
      } catch (b) {
        console.error("Error polling progress:", b), g && (N = setTimeout(T, 2e3));
      }
    };
    return T(), () => {
      g = !1, N && clearTimeout(N);
    };
  }, []);
  const f = async () => {
    i(!0), d(null);
    try {
      const g = await ae.resetDatabase(!0);
      g.success && g.data ? (d({ type: "success", message: g.data.message }), setTimeout(() => {
        window.location.reload();
      }, 2e3)) : d({ type: "error", message: g.error || "Reset failed" });
    } catch (g) {
      d({ type: "error", message: `Network error: ${g.message}` });
    } finally {
      i(!1), e(!1);
    }
  }, p = () => t ? /* @__PURE__ */ n("div", { className: "fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50", children: /* @__PURE__ */ c("div", { className: "bg-white rounded-lg p-6 max-w-md w-full mx-4", children: [
    /* @__PURE__ */ c("div", { className: "flex items-center gap-3 mb-4", children: [
      /* @__PURE__ */ n(on, { className: "w-6 h-6 text-red-500" }),
      /* @__PURE__ */ n("h3", { className: "text-lg font-semibold text-gray-900", children: "Reset Database" })
    ] }),
    /* @__PURE__ */ c("div", { className: "mb-6", children: [
      /* @__PURE__ */ n("p", { className: "text-gray-700 mb-2", children: "This will permanently delete all data and restart the node:" }),
      /* @__PURE__ */ c("ul", { className: "list-disc list-inside text-sm text-gray-600 space-y-1", children: [
        /* @__PURE__ */ n("li", { children: "All schemas will be removed" }),
        /* @__PURE__ */ n("li", { children: "All stored data will be deleted" }),
        /* @__PURE__ */ n("li", { children: "Network connections will be reset" }),
        /* @__PURE__ */ n("li", { children: "This action cannot be undone" })
      ] })
    ] }),
    /* @__PURE__ */ c("div", { className: "flex gap-3 justify-end", children: [
      /* @__PURE__ */ n(
        "button",
        {
          onClick: () => e(!1),
          className: "px-4 py-2 text-sm font-medium text-gray-700 bg-gray-100 rounded-md hover:bg-gray-200 transition-colors",
          disabled: r,
          children: "Cancel"
        }
      ),
      /* @__PURE__ */ n(
        "button",
        {
          onClick: f,
          disabled: r,
          className: "px-4 py-2 text-sm font-medium text-white bg-red-600 rounded-md hover:bg-red-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors",
          children: r ? "Resetting..." : "Reset Database"
        }
      )
    ] })
  ] }) }) : null, x = (g) => {
    var F, P;
    const N = g.is_complete, T = g.is_failed, b = !N, E = g.job_type === "indexing";
    let y = "";
    if (g.started_at) {
      const C = typeof g.started_at == "number" ? g.started_at * 1e3 : new Date(g.started_at).getTime(), I = g.completed_at ? typeof g.completed_at == "number" ? g.completed_at * 1e3 : new Date(g.completed_at).getTime() : Date.now(), O = Math.floor((I - C) / 1e3);
      y = O > 0 ? `${O}s` : "Just started";
    }
    let w;
    T ? w = "red" : N ? w = "green" : E ? w = "purple" : w = "blue";
    const S = T ? "Failed" : N ? "Complete" : "Active", _ = E ? "Indexing Job" : "Ingestion Job", R = [];
    return y && R.push(y), ((F = g.results) == null ? void 0 : F.mutations_executed) !== void 0 && R.push(`${g.results.mutations_executed} items`), E && ((P = g.results) == null ? void 0 : P.total_operations_processed) !== void 0 && R.push(`${g.results.total_operations_processed} indexed`), /* @__PURE__ */ c("div", { className: `p-4 rounded-lg border-2 border-${w}-200 bg-${w}-50`, children: [
      /* @__PURE__ */ c("div", { className: "flex items-center justify-between mb-2", children: [
        /* @__PURE__ */ c("div", { className: "flex items-center gap-2", children: [
          /* @__PURE__ */ n("div", { className: `w-2.5 h-2.5 rounded-full bg-${w}-500 ${b ? "animate-pulse" : ""}` }),
          /* @__PURE__ */ n("h3", { className: `font-semibold text-${w}-900`, children: _ })
        ] }),
        /* @__PURE__ */ n("span", { className: `text-xs font-medium px-2 py-1 rounded bg-${w}-100 text-${w}-700`, children: S })
      ] }),
      /* @__PURE__ */ n("p", { className: `text-sm text-${w}-700 truncate`, title: g.status_message, children: g.status_message }),
      R.length > 0 && /* @__PURE__ */ n("div", { className: "mt-2 flex flex-wrap gap-2", children: R.map((C, I) => /* @__PURE__ */ n("span", { className: `text-xs font-medium px-2 py-1 rounded bg-${w}-100 text-${w}-800`, children: C }, I)) }),
      b && !T && /* @__PURE__ */ c("div", { className: "mt-3", children: [
        /* @__PURE__ */ c("div", { className: "flex items-center justify-between mb-1", children: [
          /* @__PURE__ */ n("span", { className: `text-xs font-medium text-${w}-700`, children: "Progress" }),
          /* @__PURE__ */ c("span", { className: `text-xs font-semibold text-${w}-900`, children: [
            g.progress_percentage,
            "%"
          ] })
        ] }),
        /* @__PURE__ */ n("div", { className: `w-full bg-${w}-200 rounded-full h-2`, children: /* @__PURE__ */ n(
          "div",
          {
            className: `bg-${w}-600 h-2 rounded-full transition-all duration-300`,
            style: { width: `${g.progress_percentage}%` }
          }
        ) })
      ] })
    ] }, g.id);
  };
  return /* @__PURE__ */ c(Rt, { children: [
    /* @__PURE__ */ c("div", { className: "bg-white rounded-lg shadow-sm p-4 mb-6", children: [
      /* @__PURE__ */ c("div", { className: "flex items-center justify-between mb-4", children: [
        /* @__PURE__ */ c("div", { className: "flex items-center gap-2", children: [
          /* @__PURE__ */ n(Wa, { className: "w-5 h-5 text-green-500" }),
          /* @__PURE__ */ n("h2", { className: "text-lg font-semibold text-gray-900", children: "System Status" })
        ] }),
        /* @__PURE__ */ c(
          "button",
          {
            onClick: () => e(!0),
            className: "flex items-center gap-2 px-3 py-1.5 text-sm font-medium text-red-600 border border-red-200 rounded-md hover:bg-red-50 hover:border-red-300 transition-colors",
            disabled: r,
            children: [
              /* @__PURE__ */ n(on, { className: "w-4 h-4" }),
              "Reset Database"
            ]
          }
        )
      ] }),
      /* @__PURE__ */ c("div", { className: "grid grid-cols-1 md:grid-cols-2 gap-4", children: [
        h.length > 0 && h.map((g) => x(g)),
        h.length === 0 && /* @__PURE__ */ n("div", { className: "p-4 rounded-lg border-2 border-dashed border-gray-200 bg-gray-50 flex items-center justify-center text-gray-400 text-sm", children: "No active jobs" })
      ] }),
      o && /* @__PURE__ */ n("div", { className: `mt-3 p-3 rounded-md text-sm ${o.type === "success" ? "bg-green-50 text-green-800 border border-green-200" : "bg-red-50 text-red-800 border border-red-200"}`, children: o.message })
    ] }),
    /* @__PURE__ */ n(p, {})
  ] });
}
function Ee(t) {
  return t !== null && typeof t == "object" && !Array.isArray(t);
}
function ii(t) {
  const e = yt(t);
  if (!Ee(e)) return !1;
  const r = Object.keys(e);
  if (r.length === 0) return !1;
  for (let i = 0; i < Math.min(3, r.length); i++) {
    const o = e[r[i]];
    if (!Ee(o)) return !1;
    const d = Object.keys(o);
    if (d.length !== 0)
      for (let h = 0; h < Math.min(3, d.length); h++) {
        const l = o[d[h]];
        if (!Ee(l)) return !1;
        Object.keys(l).length;
      }
  }
  return !0;
}
function yt(t) {
  return t && Ee(t) && Object.prototype.hasOwnProperty.call(t, "data") ? t.data : t;
}
function oi(t) {
  const e = yt(t) || {};
  if (!Ee(e)) return { hashes: 0, ranges: 0 };
  const r = Object.keys(e).length;
  let i = 0;
  for (const o of Object.keys(e)) {
    const d = e[o];
    Ee(d) && (i += Object.keys(d).length);
  }
  return { hashes: r, ranges: i };
}
function li(t) {
  const e = yt(t) || {};
  return Ee(e) ? Object.keys(e).sort(Vn) : [];
}
function jn(t, e) {
  const r = yt(t) || {}, i = Ee(r) && Ee(r[e]) ? r[e] : {};
  return Object.keys(i).sort(Vn);
}
function Vn(t, e) {
  const r = cn(t), i = cn(e);
  return !Number.isNaN(r) && !Number.isNaN(i) ? r - i : String(t).localeCompare(String(e));
}
function cn(t) {
  const e = Number(t);
  return Number.isFinite(e) ? e : Number.NaN;
}
function ci(t, e, r) {
  const i = yt(t) || {};
  if (!Ee(i)) return null;
  const o = i[e];
  if (!Ee(o)) return null;
  const d = o[r];
  return Ee(d) ? d : null;
}
function Gn(t, e, r) {
  return t.slice(e, Math.min(e + r, t.length));
}
const di = 50;
function zn({ isOpen: t, onClick: e, label: r }) {
  return /* @__PURE__ */ c(
    "button",
    {
      type: "button",
      className: "text-left w-full flex items-center justify-between px-3 py-2 hover:bg-gray-100 rounded",
      onClick: e,
      "aria-expanded": t,
      children: [
        /* @__PURE__ */ n("span", { className: "font-mono text-sm text-gray-800 truncate", children: r }),
        /* @__PURE__ */ n("span", { className: "ml-2 text-gray-500 text-xs", children: t ? "▼" : "▶" })
      ]
    }
  );
}
function ui({ fields: t }) {
  const e = Z(() => Object.entries(t || {}), [t]);
  return e.length === 0 ? /* @__PURE__ */ n("div", { className: "text-xs text-gray-500 italic px-3 py-2", children: "No fields" }) : /* @__PURE__ */ n("div", { className: "px-3 py-2 overflow-x-auto", children: /* @__PURE__ */ n("table", { className: "min-w-full border-separate border-spacing-y-1", children: /* @__PURE__ */ n("tbody", { children: e.map(([r, i]) => /* @__PURE__ */ c("tr", { className: "bg-white", children: [
    /* @__PURE__ */ n("td", { className: "align-top text-xs font-medium text-gray-700 pr-4 whitespace-nowrap", children: r }),
    /* @__PURE__ */ n("td", { className: "align-top text-xs text-gray-700", children: /* @__PURE__ */ n("pre", { className: "font-mono whitespace-pre-wrap break-words", children: hi(i) }) })
  ] }, r)) }) }) });
}
function hi(t) {
  if (t === null) return "null";
  if (typeof t == "string") return t;
  if (typeof t == "number" || typeof t == "boolean") return String(t);
  try {
    return JSON.stringify(t, null, 2);
  } catch {
    return String(t);
  }
}
function mi({ results: t, pageSize: e = di }) {
  const r = Z(() => yt(t) || {}, [t]), i = Z(() => oi(t), [t]), o = Z(() => li(t), [t]), [d, h] = B(() => /* @__PURE__ */ new Set()), [l, f] = B(() => /* @__PURE__ */ new Set()), [p, x] = B({ start: 0, count: e }), [g, N] = B(() => /* @__PURE__ */ new Map()), T = $((w) => {
    h((S) => {
      const _ = new Set(S);
      return _.has(w) ? _.delete(w) : _.add(w), _;
    }), N((S) => {
      if (!d.has(w)) {
        const _ = jn(r, w).length, R = new Map(S);
        return R.set(w, { start: 0, count: Math.min(e, _) }), R;
      }
      return S;
    });
  }, [r, d, e]), b = $((w, S) => {
    const _ = w + "||" + S;
    f((R) => {
      const F = new Set(R);
      return F.has(_) ? F.delete(_) : F.add(_), F;
    });
  }, []), E = $(() => {
    const w = Math.min(o.length, p.count + e);
    x((S) => ({ start: 0, count: w }));
  }, [o, p.count, e]), y = Z(() => Gn(o, p.start, p.count), [o, p]);
  return /* @__PURE__ */ c("div", { className: "space-y-2", children: [
    /* @__PURE__ */ c("div", { className: "text-xs text-gray-600", children: [
      /* @__PURE__ */ c("span", { className: "mr-4", children: [
        "Hashes: ",
        /* @__PURE__ */ n("strong", { children: i.hashes })
      ] }),
      /* @__PURE__ */ c("span", { children: [
        "Ranges: ",
        /* @__PURE__ */ n("strong", { children: i.ranges })
      ] })
    ] }),
    /* @__PURE__ */ n("div", { className: "border rounded-md divide-y divide-gray-200 bg-gray-50", children: y.map((w) => /* @__PURE__ */ c("div", { className: "p-2", children: [
      /* @__PURE__ */ n(
        zn,
        {
          isOpen: d.has(w),
          onClick: () => T(w),
          label: `hash: ${String(w)}`
        }
      ),
      d.has(w) && /* @__PURE__ */ n(
        fi,
        {
          data: r,
          hashKey: w,
          rangeOpen: l,
          onToggleRange: b,
          pageSize: e,
          rangeWindow: g.get(w),
          setRangeWindow: (S) => N((_) => new Map(_).set(w, S))
        }
      )
    ] }, w)) }),
    p.count < o.length && /* @__PURE__ */ n("div", { className: "pt-2", children: /* @__PURE__ */ c(
      "button",
      {
        type: "button",
        className: "text-xs px-3 py-1 rounded bg-gray-200 hover:bg-gray-300",
        onClick: E,
        children: [
          "Show more hashes (",
          p.count,
          "/",
          o.length,
          ")"
        ]
      }
    ) })
  ] });
}
function fi({ data: t, hashKey: e, rangeOpen: r, onToggleRange: i, pageSize: o, rangeWindow: d, setRangeWindow: h }) {
  const l = Z(() => jn(t, e), [t, e]), f = d || { start: 0, count: Math.min(o, l.length) }, p = Z(() => Gn(l, f.start, f.count), [l, f]), x = $(() => {
    const g = Math.min(l.length, f.count + o);
    h({ start: 0, count: g });
  }, [l.length, f.count, o, h]);
  return /* @__PURE__ */ c("div", { className: "ml-4 mt-1 border-l pl-3", children: [
    p.map((g) => /* @__PURE__ */ c("div", { className: "py-1", children: [
      /* @__PURE__ */ n(
        zn,
        {
          isOpen: r.has(e + "||" + g),
          onClick: () => i(e, g),
          label: `range: ${String(g)}`
        }
      ),
      r.has(e + "||" + g) && /* @__PURE__ */ n("div", { className: "ml-4 mt-1", children: /* @__PURE__ */ n(ui, { fields: ci(t, e, g) || {} }) })
    ] }, g)),
    f.count < l.length && /* @__PURE__ */ n("div", { className: "pt-1", children: /* @__PURE__ */ c(
      "button",
      {
        type: "button",
        className: "text-xs px-3 py-1 rounded bg-gray-200 hover:bg-gray-300",
        onClick: x,
        children: [
          "Show more ranges (",
          f.count,
          "/",
          l.length,
          ")"
        ]
      }
    ) })
  ] });
}
function Wo({ results: t }) {
  const e = t != null, r = e && (!!t.error || t.status && t.status >= 400), i = e && t.data !== void 0, o = Z(() => e && !r && ii(i ? t.data : t), [e, t, r, i]), [d, h] = B(o);
  return e ? /* @__PURE__ */ c("div", { className: "bg-white rounded-lg shadow-sm p-6 mt-6", children: [
    /* @__PURE__ */ c("h3", { className: "text-lg font-semibold mb-4 flex items-center", children: [
      /* @__PURE__ */ n("span", { className: `mr-2 ${r ? "text-red-600" : "text-gray-900"}`, children: r ? "Error" : "Results" }),
      /* @__PURE__ */ c("span", { className: "text-xs font-normal text-gray-500", children: [
        "(",
        typeof t == "string" ? "Text" : d ? "Structured" : "JSON",
        ")"
      ] }),
      t.status && /* @__PURE__ */ c("span", { className: `ml-2 px-2 py-1 text-xs rounded-full ${t.status >= 400 ? "bg-red-100 text-red-800" : "bg-green-100 text-green-800"}`, children: [
        "Status: ",
        t.status
      ] }),
      !r && typeof t != "string" && /* @__PURE__ */ n("div", { className: "ml-auto", children: /* @__PURE__ */ n(
        "button",
        {
          type: "button",
          className: "text-xs px-2 py-1 rounded border border-gray-300 hover:bg-gray-100",
          onClick: () => h((l) => !l),
          children: d ? "View JSON" : "View Structured"
        }
      ) })
    ] }),
    r && /* @__PURE__ */ n("div", { className: "mb-4 p-4 bg-red-50 border border-red-200 rounded-md", children: /* @__PURE__ */ c("div", { className: "flex", children: [
      /* @__PURE__ */ n("div", { className: "flex-shrink-0", children: /* @__PURE__ */ n("svg", { className: "h-5 w-5 text-red-400", viewBox: "0 0 20 20", fill: "currentColor", children: /* @__PURE__ */ n("path", { fillRule: "evenodd", d: "M10 18a8 8 0 100-16 8 8 0 000 16zM8.707 7.293a1 1 0 00-1.414 1.414L8.586 10l-1.293 1.293a1 1 0 101.414 1.414L10 11.414l1.293 1.293a1 1 0 001.414-1.414L11.414 10l1.293-1.293a1 1 0 00-1.414-1.414L10 8.586 8.707 7.293z", clipRule: "evenodd" }) }) }),
      /* @__PURE__ */ c("div", { className: "ml-3", children: [
        /* @__PURE__ */ n("h4", { className: "text-sm font-medium text-red-800", children: "Query Execution Failed" }),
        /* @__PURE__ */ n("div", { className: "mt-2 text-sm text-red-700", children: /* @__PURE__ */ n("p", { children: t.error || "An unknown error occurred" }) })
      ] })
    ] }) }),
    d && !r && typeof t != "string" ? /* @__PURE__ */ n("div", { className: "rounded-md p-2 bg-gray-50 border overflow-auto max-h-[500px]", children: /* @__PURE__ */ n(mi, { results: t }) }) : /* @__PURE__ */ n("div", { className: `rounded-md p-4 overflow-auto max-h-[500px] ${r ? "bg-red-50 border border-red-200" : "bg-gray-50"}`, children: /* @__PURE__ */ n("pre", { className: `font-mono text-sm whitespace-pre-wrap ${r ? "text-red-700" : "text-gray-700"}`, children: typeof t == "string" ? t : JSON.stringify(i ? t.data : t, null, 2) }) })
  ] }) : null;
}
const Ne = {
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
function Jo({
  tabs: t = fa,
  activeTab: e,
  onTabChange: r,
  className: i = ""
}) {
  const o = (p, x) => {
    r(p);
  }, d = (p) => {
    const x = e === p.id, g = p.disabled || !1;
    let N = Ne.tab.base;
    return x ? N += ` ${Ne.tab.active}` : g ? N += ` ${Ne.tab.disabled}` : N += ` ${Ne.tab.inactive}`, N;
  }, h = t.filter((p) => p.group === "main"), l = t.filter((p) => p.group === "advanced"), f = (p) => {
    const x = p.disabled || !1;
    return /* @__PURE__ */ c(
      "button",
      {
        className: d(p),
        onClick: () => o(p.id, p.requiresAuth),
        disabled: x,
        "aria-current": e === p.id ? "page" : void 0,
        "aria-label": `${p.label} tab`,
        style: {
          transitionDuration: `${ha}ms`
        },
        children: [
          p.icon && /* @__PURE__ */ n("span", { className: "mr-2", "aria-hidden": "true", children: p.icon }),
          /* @__PURE__ */ n("span", { children: p.label })
        ]
      },
      p.id
    );
  };
  return /* @__PURE__ */ n("div", { className: `border-b border-gray-200 ${i}`, children: /* @__PURE__ */ c("div", { className: "flex items-center", children: [
    /* @__PURE__ */ n("div", { className: "flex space-x-8", children: h.map(f) }),
    l.length > 0 && /* @__PURE__ */ n("div", { className: "mx-6 h-6 w-px bg-gray-300", "aria-hidden": "true" }),
    l.length > 0 && /* @__PURE__ */ c("div", { className: "flex items-center space-x-6", children: [
      /* @__PURE__ */ n("span", { className: "text-xs text-gray-500 font-medium uppercase tracking-wider", children: "Advanced" }),
      /* @__PURE__ */ n("div", { className: "flex space-x-6", children: l.map(f) })
    ] })
  ] }) });
}
const dn = {
  InProgress: { color: "text-blue-700 bg-blue-50", icon: "⏳" },
  Completed: { color: "text-green-700 bg-green-50", icon: "✅" },
  Failed: { color: "text-red-700 bg-red-50", icon: "❌" },
  default: { color: "text-gray-700 bg-gray-50", icon: "❓" }
}, qn = (t) => new Date(t * 1e3).toLocaleString(), pi = (t, e) => {
  const r = (e || Math.floor(Date.now() / 1e3)) - t;
  return r < 60 ? `${r}s` : r < 3600 ? `${Math.floor(r / 60)}m ${r % 60}s` : `${Math.floor(r / 3600)}h ${Math.floor(r % 3600 / 60)}m`;
}, gi = (t, e) => {
  const r = t + e;
  return r === 0 ? "N/A" : `${Math.round(t / r * 100)}%`;
}, yi = ({ backfill: t }) => {
  const e = dn[t.status] || dn.default;
  return /* @__PURE__ */ c("div", { className: `p-3 rounded-lg border ${e.color}`, children: [
    /* @__PURE__ */ c("div", { className: "flex justify-between items-start mb-2", children: [
      /* @__PURE__ */ c("div", { className: "flex items-center gap-2", children: [
        /* @__PURE__ */ n("span", { className: "text-xl", children: e.icon }),
        /* @__PURE__ */ c("div", { children: [
          /* @__PURE__ */ n("div", { className: "font-semibold", children: t.transform_id }),
          /* @__PURE__ */ c("div", { className: "text-xs opacity-80", children: [
            "Source: ",
            t.schema_name
          ] })
        ] })
      ] }),
      /* @__PURE__ */ c("div", { className: "text-xs text-right", children: [
        /* @__PURE__ */ c("div", { children: [
          "Started: ",
          qn(t.start_time)
        ] }),
        /* @__PURE__ */ c("div", { children: [
          "Duration: ",
          pi(t.start_time, t.end_time)
        ] })
      ] })
    ] }),
    /* @__PURE__ */ n(bi, { backfill: t }),
    t.status === "InProgress" && t.mutations_expected > 0 && /* @__PURE__ */ n(xi, { backfill: t })
  ] });
}, bi = ({ backfill: t }) => {
  const { status: e } = t;
  return e === "InProgress" ? /* @__PURE__ */ c("div", { className: "grid grid-cols-2 md:grid-cols-3 gap-2 text-sm mt-2", children: [
    /* @__PURE__ */ c("div", { children: [
      /* @__PURE__ */ n("span", { className: "font-medium", children: "Mutations:" }),
      " ",
      t.mutations_completed,
      " / ",
      t.mutations_expected
    ] }),
    t.mutations_failed > 0 && /* @__PURE__ */ c("div", { className: "text-red-600", children: [
      /* @__PURE__ */ n("span", { className: "font-medium", children: "Failed:" }),
      " ",
      t.mutations_failed
    ] })
  ] }) : e === "Completed" ? /* @__PURE__ */ c("div", { className: "grid grid-cols-2 md:grid-cols-3 gap-2 text-sm mt-2", children: [
    /* @__PURE__ */ c("div", { children: [
      /* @__PURE__ */ n("span", { className: "font-medium", children: "Mutations:" }),
      " ",
      t.mutations_completed
    ] }),
    /* @__PURE__ */ c("div", { children: [
      /* @__PURE__ */ n("span", { className: "font-medium", children: "Records:" }),
      " ",
      t.records_produced
    ] }),
    /* @__PURE__ */ c("div", { children: [
      /* @__PURE__ */ n("span", { className: "font-medium", children: "Completed:" }),
      " ",
      t.end_time && qn(t.end_time)
    ] })
  ] }) : e === "Failed" && t.error ? /* @__PURE__ */ n("div", { className: "grid grid-cols-2 md:grid-cols-3 gap-2 text-sm mt-2", children: /* @__PURE__ */ c("div", { className: "col-span-2 md:col-span-3", children: [
    /* @__PURE__ */ n("span", { className: "font-medium", children: "Error:" }),
    " ",
    t.error
  ] }) }) : null;
}, xi = ({ backfill: t }) => {
  const e = Math.round(t.mutations_completed / t.mutations_expected * 100);
  return /* @__PURE__ */ c("div", { className: "mt-2", children: [
    /* @__PURE__ */ n("div", { className: "w-full bg-gray-200 rounded-full h-2", children: /* @__PURE__ */ n(
      "div",
      {
        className: "bg-blue-600 h-2 rounded-full transition-all duration-300",
        style: { width: `${e}%` }
      }
    ) }),
    /* @__PURE__ */ c("div", { className: "text-xs text-right mt-1", children: [
      e,
      "% complete"
    ] })
  ] });
}, wi = () => {
  const [t, e] = B([]), [r, i] = B(null), [o, d] = B(!0), [h, l] = B(null), [f, p] = B(!1), x = $(async () => {
    try {
      const y = await fe.getAllBackfills();
      if (!(y != null && y.success) || !y.data)
        throw new Error((y == null ? void 0 : y.error) || "Failed to fetch backfills - invalid response");
      e(y.data), l(null);
    } catch (y) {
      throw console.error("Failed to fetch backfills:", y), l(y.message || "Failed to load backfills"), y;
    }
  }, []), g = $(async () => {
    try {
      const y = await fe.getBackfillStatistics();
      if (!(y != null && y.success) || !y.data)
        throw new Error((y == null ? void 0 : y.error) || "Failed to fetch backfill statistics - invalid response");
      i(y.data), l(null);
    } catch (y) {
      throw console.error("Failed to fetch backfill statistics:", y), l(y.message || "Failed to load statistics"), y;
    } finally {
      d(!1);
    }
  }, []);
  de(() => {
    x(), g();
    const y = setInterval(() => {
      x(), g();
    }, 3e3);
    return () => clearInterval(y);
  }, [x, g]);
  const N = t.filter((y) => y.status === "InProgress"), T = t.filter((y) => y.status === "Completed"), b = t.filter((y) => y.status === "Failed"), E = f ? t : N;
  return o ? /* @__PURE__ */ n("div", { className: "bg-gray-50 p-4 rounded-lg", children: /* @__PURE__ */ c("div", { className: "flex items-center", children: [
    /* @__PURE__ */ n("div", { className: "animate-spin rounded-full h-4 w-4 border-b-2 border-gray-600 mr-2" }),
    /* @__PURE__ */ n("span", { className: "text-gray-800", children: "Loading backfill information..." })
  ] }) }) : h ? /* @__PURE__ */ n("div", { className: "bg-red-50 p-4 rounded-lg", role: "alert", children: /* @__PURE__ */ c("span", { className: "text-red-800", children: [
    "Error: ",
    h
  ] }) }) : /* @__PURE__ */ c("div", { className: "space-y-4", children: [
    r && /* @__PURE__ */ c("div", { className: "bg-gray-50 p-4 rounded-lg", children: [
      /* @__PURE__ */ n("h3", { className: "text-md font-medium text-gray-800 mb-3", children: "Backfill Statistics" }),
      /* @__PURE__ */ c("div", { className: "grid grid-cols-2 md:grid-cols-4 gap-4 text-sm", children: [
        /* @__PURE__ */ c("div", { children: [
          /* @__PURE__ */ n("div", { className: "text-gray-600", children: "Total Mutations" }),
          /* @__PURE__ */ n("div", { className: "text-lg font-semibold text-gray-900", children: r.total_mutations_completed })
        ] }),
        /* @__PURE__ */ c("div", { children: [
          /* @__PURE__ */ n("div", { className: "text-gray-600", children: "Success Rate" }),
          /* @__PURE__ */ n("div", { className: "text-lg font-semibold text-green-700", children: gi(r.total_mutations_completed, r.total_mutations_failed) })
        ] }),
        /* @__PURE__ */ c("div", { children: [
          /* @__PURE__ */ n("div", { className: "text-gray-600", children: "Backfills" }),
          /* @__PURE__ */ n("div", { className: "text-lg font-semibold text-blue-700", children: r.total_backfills })
        ] }),
        /* @__PURE__ */ c("div", { children: [
          /* @__PURE__ */ n("div", { className: "text-gray-600", children: "Failures" }),
          /* @__PURE__ */ n("div", { className: "text-lg font-semibold text-red-700", children: r.total_mutations_failed })
        ] })
      ] })
    ] }),
    /* @__PURE__ */ c("div", { className: "bg-gray-50 p-4 rounded-lg", children: [
      /* @__PURE__ */ c("div", { className: "flex justify-between items-center mb-3", children: [
        /* @__PURE__ */ n("h3", { className: "text-md font-medium text-gray-800", children: "Backfills" }),
        /* @__PURE__ */ c("div", { className: "flex items-center gap-4", children: [
          /* @__PURE__ */ c("div", { className: "text-sm text-gray-600", children: [
            "Active: ",
            N.length,
            " | Completed: ",
            T.length,
            " | Failed: ",
            b.length
          ] }),
          /* @__PURE__ */ n(
            "button",
            {
              onClick: () => p(!f),
              className: "px-3 py-1 text-sm bg-gray-200 text-gray-800 rounded hover:bg-gray-300",
              children: f ? "Show Active Only" : "Show All"
            }
          )
        ] })
      ] }),
      E.length === 0 ? /* @__PURE__ */ n("div", { className: "text-gray-600 text-sm", children: f ? "No backfills recorded" : "No active backfills" }) : /* @__PURE__ */ n("div", { className: "space-y-3", children: E.map((y) => /* @__PURE__ */ n(
        yi,
        {
          backfill: y
        },
        `${y.transform_id}-${y.start_time}`
      )) })
    ] })
  ] });
}, vi = {
  queue: [],
  length: 0,
  isEmpty: !0
}, Ni = (t = {}) => {
  const e = Array.isArray(t.queue) ? t.queue : [], r = typeof t.length == "number" ? t.length : e.length, i = typeof t.isEmpty == "boolean" ? t.isEmpty : e.length === 0;
  return { queue: e, length: r, isEmpty: i };
}, Si = ({ onResult: t }) => {
  const [e, r] = B(vi), [i, o] = B({}), [d, h] = B({}), [l, f] = B(!1), [p, x] = B(null), [g, N] = B([]), T = $(async () => {
    f(!0), x(null);
    try {
      const y = await fe.getTransforms();
      if (y != null && y.success && y.data) {
        const w = y.data, S = w && typeof w == "object" && !Array.isArray(w) ? Object.entries(w).map(([_, R]) => ({
          transform_id: _,
          ...R
        })) : Array.isArray(w) ? w : [];
        N(S);
      } else {
        const w = (y == null ? void 0 : y.error) || "Failed to load transforms";
        x(w), N([]);
      }
    } catch (y) {
      console.error("Failed to fetch transforms:", y), x(y.message || "Failed to load transforms"), N([]);
    } finally {
      f(!1);
    }
  }, []), b = $(async () => {
    try {
      const y = await fe.getQueue();
      y != null && y.success && y.data && r(Ni(y.data));
    } catch (y) {
      console.error("Failed to fetch transform queue info:", y);
    }
  }, []);
  de(() => {
    T(), b();
    const y = setInterval(b, 5e3);
    return () => clearInterval(y);
  }, [T, b]);
  const E = $(async (y, w) => {
    var _;
    const S = w ? `${y}.${w}` : y;
    o((R) => ({ ...R, [S]: !0 })), h((R) => ({ ...R, [S]: null }));
    try {
      const R = await fe.addToQueue(S);
      if (!(R != null && R.success)) {
        const F = ((_ = R == null ? void 0 : R.data) == null ? void 0 : _.message) || (R == null ? void 0 : R.error) || "Failed to add transform to queue";
        throw new Error(F);
      }
      typeof t == "function" && t({ success: !0, transformId: S }), await b();
    } catch (R) {
      console.error("Failed to add transform to queue:", R), h((F) => ({ ...F, [S]: R.message || "Failed to add transform to queue" }));
    } finally {
      o((R) => ({ ...R, [S]: !1 }));
    }
  }, [b, t]);
  return /* @__PURE__ */ c("div", { className: "space-y-4", children: [
    /* @__PURE__ */ c("div", { className: "flex justify-between items-center", children: [
      /* @__PURE__ */ n("h2", { className: "text-xl font-semibold text-gray-800", children: "Transforms" }),
      /* @__PURE__ */ c("div", { className: "text-sm text-gray-600", children: [
        "Queue Status: ",
        e.isEmpty ? "Empty" : `${e.length} transform(s) queued`
      ] })
    ] }),
    /* @__PURE__ */ n(wi, {}),
    !e.isEmpty && /* @__PURE__ */ c("div", { className: "bg-blue-50 p-4 rounded-lg", "data-testid": "transform-queue", children: [
      /* @__PURE__ */ n("h3", { className: "text-md font-medium text-blue-800 mb-2", children: "Transform Queue" }),
      /* @__PURE__ */ n("ul", { className: "list-disc list-inside space-y-1", children: e.queue.map((y, w) => /* @__PURE__ */ n("li", { className: "text-blue-700", children: y }, `${y}-${w}`)) })
    ] }),
    l && /* @__PURE__ */ n("div", { className: "bg-blue-50 p-4 rounded-lg", role: "status", children: /* @__PURE__ */ c("div", { className: "flex items-center", children: [
      /* @__PURE__ */ n("div", { className: "animate-spin rounded-full h-4 w-4 border-b-2 border-blue-600 mr-2" }),
      /* @__PURE__ */ n("span", { className: "text-blue-800", children: "Loading transforms..." })
    ] }) }),
    p && /* @__PURE__ */ n("div", { className: "bg-red-50 p-4 rounded-lg", role: "alert", children: /* @__PURE__ */ c("div", { className: "flex items-center", children: [
      /* @__PURE__ */ c("span", { className: "text-red-800", children: [
        "Error loading transforms: ",
        p
      ] }),
      /* @__PURE__ */ n(
        "button",
        {
          onClick: T,
          className: "ml-4 px-3 py-1 text-sm bg-red-500 text-white rounded hover:bg-red-600",
          children: "Retry"
        }
      )
    ] }) }),
    !l && !p && g.length > 0 && /* @__PURE__ */ n("div", { className: "space-y-4", children: g.map((y, w) => {
      var H;
      const S = y.transform_id || `transform-${w}`, _ = i[S], R = d[S], F = y.name || ((H = y.transform_id) == null ? void 0 : H.split(".")[0]) || "Unknown", P = y.schema_type;
      let C = "Single", I = "bg-gray-100 text-gray-800";
      P != null && P.Range ? (C = "Range", I = "bg-blue-100 text-blue-800") : P != null && P.HashRange && (C = "HashRange", I = "bg-purple-100 text-purple-800");
      const O = y.key, U = y.transform_fields || {}, K = Object.keys(U).length, V = Object.keys(U);
      return /* @__PURE__ */ c("div", { className: "bg-white p-4 rounded-lg shadow border-l-4 border-blue-500", children: [
        /* @__PURE__ */ n("div", { className: "flex justify-between items-start mb-3", children: /* @__PURE__ */ c("div", { className: "flex-1", children: [
          /* @__PURE__ */ n("h3", { className: "text-lg font-semibold text-gray-900", children: F }),
          /* @__PURE__ */ c("div", { className: "flex gap-2 mt-2 flex-wrap", children: [
            /* @__PURE__ */ n("span", { className: `inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium ${I}`, children: C }),
            K > 0 && /* @__PURE__ */ c("span", { className: "inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-green-100 text-green-800", children: [
              K,
              " field",
              K !== 1 ? "s" : ""
            ] })
          ] }),
          V.length > 0 && /* @__PURE__ */ c("div", { className: "mt-2 text-sm text-gray-600", children: [
            /* @__PURE__ */ n("span", { className: "font-medium", children: "Fields:" }),
            " ",
            V.join(", ")
          ] })
        ] }) }),
        /* @__PURE__ */ c("div", { className: "mt-3 space-y-3", children: [
          O && /* @__PURE__ */ c("div", { className: "bg-blue-50 rounded p-3", children: [
            /* @__PURE__ */ n("div", { className: "text-sm font-medium text-blue-900 mb-1", children: "Key Configuration:" }),
            /* @__PURE__ */ c("div", { className: "text-sm text-blue-800 space-y-1", children: [
              O.hash_field && /* @__PURE__ */ c("div", { children: [
                /* @__PURE__ */ n("span", { className: "font-medium", children: "Hash Key:" }),
                " ",
                O.hash_field
              ] }),
              O.range_field && /* @__PURE__ */ c("div", { children: [
                /* @__PURE__ */ n("span", { className: "font-medium", children: "Range Key:" }),
                " ",
                O.range_field
              ] }),
              !O.hash_field && !O.range_field && O.key_field && /* @__PURE__ */ c("div", { children: [
                /* @__PURE__ */ n("span", { className: "font-medium", children: "Key:" }),
                " ",
                O.key_field
              ] })
            ] })
          ] }),
          K > 0 && /* @__PURE__ */ c("div", { children: [
            /* @__PURE__ */ n("div", { className: "text-sm font-medium text-gray-700 mb-2", children: "Transform Fields:" }),
            /* @__PURE__ */ n("div", { className: "bg-gray-50 rounded p-3 space-y-2", children: Object.entries(U).map(([G, L]) => /* @__PURE__ */ c("div", { className: "border-l-2 border-gray-300 pl-3", children: [
              /* @__PURE__ */ n("div", { className: "font-medium text-gray-900 text-sm", children: G }),
              /* @__PURE__ */ n("div", { className: "text-gray-600 font-mono text-xs mt-1 break-all", children: L })
            ] }, G)) })
          ] })
        ] }),
        /* @__PURE__ */ c("div", { className: "mt-4 flex items-center gap-3", children: [
          /* @__PURE__ */ n(
            "button",
            {
              onClick: () => E(S, null),
              disabled: _,
              className: `px-4 py-2 text-sm font-medium rounded-md text-white ${_ ? "bg-blue-300 cursor-not-allowed" : "bg-blue-600 hover:bg-blue-700"}`,
              children: _ ? "Adding..." : "Add to Queue"
            }
          ),
          R && /* @__PURE__ */ c("span", { className: "text-sm text-red-600", children: [
            "Error: ",
            R
          ] })
        ] })
      ] }, S);
    }) }),
    !l && !p && g.length === 0 && /* @__PURE__ */ c("div", { className: "bg-gray-50 p-4 rounded-lg", children: [
      /* @__PURE__ */ n("p", { className: "text-gray-600", children: "No transforms registered" }),
      /* @__PURE__ */ n("p", { className: "text-sm text-gray-500 mt-1", children: "Register a transform in a schema to view it here and add it to the processing queue." })
    ] })
  ] });
}, Ge = () => is(), te = as;
function Ei({
  title: t,
  titleId: e,
  ...r
}, i) {
  return /* @__PURE__ */ q.createElement("svg", Object.assign({
    xmlns: "http://www.w3.org/2000/svg",
    fill: "none",
    viewBox: "0 0 24 24",
    strokeWidth: 1.5,
    stroke: "currentColor",
    "aria-hidden": "true",
    "data-slot": "icon",
    ref: i,
    "aria-labelledby": e
  }, r), t ? /* @__PURE__ */ q.createElement("title", {
    id: e
  }, t) : null, /* @__PURE__ */ q.createElement("path", {
    strokeLinecap: "round",
    strokeLinejoin: "round",
    d: "m4.5 12.75 6 6 9-13.5"
  }));
}
const hr = /* @__PURE__ */ q.forwardRef(Ei);
function Ai({
  title: t,
  titleId: e,
  ...r
}, i) {
  return /* @__PURE__ */ q.createElement("svg", Object.assign({
    xmlns: "http://www.w3.org/2000/svg",
    fill: "none",
    viewBox: "0 0 24 24",
    strokeWidth: 1.5,
    stroke: "currentColor",
    "aria-hidden": "true",
    "data-slot": "icon",
    ref: i,
    "aria-labelledby": e
  }, r), t ? /* @__PURE__ */ q.createElement("title", {
    id: e
  }, t) : null, /* @__PURE__ */ q.createElement("path", {
    strokeLinecap: "round",
    strokeLinejoin: "round",
    d: "M15.666 3.888A2.25 2.25 0 0 0 13.5 2.25h-3c-1.03 0-1.9.693-2.166 1.638m7.332 0c.055.194.084.4.084.612v0a.75.75 0 0 1-.75.75H9a.75.75 0 0 1-.75-.75v0c0-.212.03-.418.084-.612m7.332 0c.646.049 1.288.11 1.927.184 1.1.128 1.907 1.077 1.907 2.185V19.5a2.25 2.25 0 0 1-2.25 2.25H6.75A2.25 2.25 0 0 1 4.5 19.5V6.257c0-1.108.806-2.057 1.907-2.185a48.208 48.208 0 0 1 1.927-.184"
  }));
}
const un = /* @__PURE__ */ q.forwardRef(Ai);
function _i({
  title: t,
  titleId: e,
  ...r
}, i) {
  return /* @__PURE__ */ q.createElement("svg", Object.assign({
    xmlns: "http://www.w3.org/2000/svg",
    fill: "none",
    viewBox: "0 0 24 24",
    strokeWidth: 1.5,
    stroke: "currentColor",
    "aria-hidden": "true",
    "data-slot": "icon",
    ref: i,
    "aria-labelledby": e
  }, r), t ? /* @__PURE__ */ q.createElement("title", {
    id: e
  }, t) : null, /* @__PURE__ */ q.createElement("path", {
    strokeLinecap: "round",
    strokeLinejoin: "round",
    d: "M12 9v3.75m-9.303 3.376c-.866 1.5.217 3.374 1.948 3.374h14.71c1.73 0 2.813-1.874 1.948-3.374L13.949 3.378c-.866-1.5-3.032-1.5-3.898 0L2.697 16.126ZM12 15.75h.007v.008H12v-.008Z"
  }));
}
const hn = /* @__PURE__ */ q.forwardRef(_i);
function Ti({
  title: t,
  titleId: e,
  ...r
}, i) {
  return /* @__PURE__ */ q.createElement("svg", Object.assign({
    xmlns: "http://www.w3.org/2000/svg",
    fill: "none",
    viewBox: "0 0 24 24",
    strokeWidth: 1.5,
    stroke: "currentColor",
    "aria-hidden": "true",
    "data-slot": "icon",
    ref: i,
    "aria-labelledby": e
  }, r), t ? /* @__PURE__ */ q.createElement("title", {
    id: e
  }, t) : null, /* @__PURE__ */ q.createElement("path", {
    strokeLinecap: "round",
    strokeLinejoin: "round",
    d: "M15.75 5.25a3 3 0 0 1 3 3m3 0a6 6 0 0 1-7.029 5.912c-.563-.097-1.159.026-1.563.43L10.5 17.25H8.25v2.25H6v2.25H2.25v-2.818c0-.597.237-1.17.659-1.591l6.499-6.499c.404-.404.527-1 .43-1.563A6 6 0 1 1 21.75 8.25Z"
  }));
}
const mr = /* @__PURE__ */ q.forwardRef(Ti);
function Ci({
  title: t,
  titleId: e,
  ...r
}, i) {
  return /* @__PURE__ */ q.createElement("svg", Object.assign({
    xmlns: "http://www.w3.org/2000/svg",
    fill: "none",
    viewBox: "0 0 24 24",
    strokeWidth: 1.5,
    stroke: "currentColor",
    "aria-hidden": "true",
    "data-slot": "icon",
    ref: i,
    "aria-labelledby": e
  }, r), t ? /* @__PURE__ */ q.createElement("title", {
    id: e
  }, t) : null, /* @__PURE__ */ q.createElement("path", {
    strokeLinecap: "round",
    strokeLinejoin: "round",
    d: "M9 12.75 11.25 15 15 9.75m-3-7.036A11.959 11.959 0 0 1 3.598 6 11.99 11.99 0 0 0 3 9.749c0 5.592 3.824 10.29 9 11.623 5.176-1.332 9-6.03 9-11.622 0-1.31-.21-2.571-.598-3.751h-.152c-3.196 0-6.1-1.248-8.25-3.285Z"
  }));
}
const Ri = /* @__PURE__ */ q.forwardRef(Ci);
function Ii({ onResult: t }) {
  const e = Ge(), r = te((I) => I.auth), { isAuthenticated: i, systemPublicKey: o, systemKeyId: d, privateKey: h, isLoading: l, error: f } = r, p = h ? aa(h) : null, [x, g] = B(null), [N, T] = B(""), [b, E] = B(!1), [y, w] = B(null), [S, _] = B(!1), R = async (I, O) => {
    try {
      await navigator.clipboard.writeText(I), g(O), setTimeout(() => g(null), 2e3);
    } catch (U) {
      console.error("Failed to copy:", U);
    }
  }, F = async () => {
    if (!N.trim()) {
      w({ valid: !1, error: "Please enter a private key" });
      return;
    }
    E(!0);
    try {
      const O = (await e(Ht(N.trim())).unwrap()).isAuthenticated;
      w({
        valid: O,
        error: O ? null : "Private key does not match the system public key"
      }), O && console.log("Private key validation successful");
    } catch (I) {
      w({
        valid: !1,
        error: `Validation failed: ${I.message}`
      });
    } finally {
      E(!1);
    }
  }, P = () => {
    T(""), w(null), _(!1);
  }, C = () => {
    P(), e(oa());
  };
  return /* @__PURE__ */ c("div", { className: "p-4 bg-white rounded-lg shadow", children: [
    /* @__PURE__ */ n("h2", { className: "text-xl font-semibold mb-4", children: "Key Management" }),
    /* @__PURE__ */ n("div", { className: "bg-blue-50 border border-blue-200 rounded-md p-4 mb-6", children: /* @__PURE__ */ c("div", { className: "flex items-start", children: [
      /* @__PURE__ */ n(Ri, { className: "h-5 w-5 text-blue-400 mr-2 flex-shrink-0 mt-0.5" }),
      /* @__PURE__ */ c("div", { className: "text-sm text-blue-700 flex-1", children: [
        /* @__PURE__ */ n("p", { className: "font-medium", children: "Current System Public Key:" }),
        l ? /* @__PURE__ */ n("p", { className: "text-blue-600", children: "Loading..." }) : o ? /* @__PURE__ */ c("div", { className: "mt-2", children: [
          /* @__PURE__ */ c("div", { className: "flex", children: [
            /* @__PURE__ */ n(
              "input",
              {
                type: "text",
                value: o && o !== "null" ? o : "",
                readOnly: !0,
                className: "flex-1 px-2 py-1 border border-blue-300 rounded-l-md bg-blue-50 text-xs font-mono"
              }
            ),
            /* @__PURE__ */ n(
              "button",
              {
                onClick: () => R(o, "system"),
                className: "px-2 py-1 border border-l-0 border-blue-300 rounded-r-md bg-white hover:bg-blue-50 focus:outline-none focus:ring-2 focus:ring-blue-500",
                children: x === "system" ? /* @__PURE__ */ n(hr, { className: "h-3 w-3 text-green-600" }) : /* @__PURE__ */ n(un, { className: "h-3 w-3 text-blue-500" })
              }
            )
          ] }),
          d && /* @__PURE__ */ c("p", { className: "text-xs text-blue-600 mt-1", children: [
            "Key ID: ",
            d
          ] }),
          i && /* @__PURE__ */ n("p", { className: "text-xs text-green-600 mt-1", children: "🔓 Authenticated - Private key loaded!" })
        ] }) : /* @__PURE__ */ n("p", { className: "text-blue-600 mt-1", children: "No system public key available." })
      ] })
    ] }) }),
    i && p && /* @__PURE__ */ n("div", { className: "bg-green-50 border border-green-200 rounded-md p-4 mb-6", children: /* @__PURE__ */ c("div", { className: "flex items-start", children: [
      /* @__PURE__ */ n(mr, { className: "h-5 w-5 text-green-400 mr-2 flex-shrink-0 mt-0.5" }),
      /* @__PURE__ */ c("div", { className: "text-sm text-green-700 flex-1", children: [
        /* @__PURE__ */ n("p", { className: "font-medium", children: "Current Private Key (Auto-loaded from Node)" }),
        /* @__PURE__ */ n("p", { className: "mt-1", children: "Your private key has been automatically loaded from the backend node." }),
        /* @__PURE__ */ c("div", { className: "mt-3", children: [
          /* @__PURE__ */ c("div", { className: "flex", children: [
            /* @__PURE__ */ n(
              "textarea",
              {
                value: p,
                readOnly: !0,
                className: "flex-1 px-3 py-2 border border-green-300 rounded-l-md bg-green-50 text-xs font-mono resize-none",
                rows: 3,
                placeholder: "Private key will appear here..."
              }
            ),
            /* @__PURE__ */ n(
              "button",
              {
                onClick: () => R(p, "private"),
                className: "px-3 py-2 border border-l-0 border-green-300 rounded-r-md bg-white hover:bg-green-50 focus:outline-none focus:ring-2 focus:ring-green-500",
                title: "Copy private key",
                children: x === "private" ? /* @__PURE__ */ n(hr, { className: "h-3 w-3 text-green-600" }) : /* @__PURE__ */ n(un, { className: "h-3 w-3 text-green-500" })
              }
            )
          ] }),
          /* @__PURE__ */ n("p", { className: "text-xs text-green-600 mt-1", children: "🔓 Authenticated - Private key loaded from node!" })
        ] })
      ] })
    ] }) }),
    o && !i && !p && /* @__PURE__ */ n("div", { className: "bg-yellow-50 border border-yellow-200 rounded-md p-4 mb-6", children: /* @__PURE__ */ c("div", { className: "flex items-start", children: [
      /* @__PURE__ */ n(mr, { className: "h-5 w-5 text-yellow-400 mr-2 flex-shrink-0 mt-0.5" }),
      /* @__PURE__ */ c("div", { className: "text-sm text-yellow-700 flex-1", children: [
        /* @__PURE__ */ n("p", { className: "font-medium", children: "Import Private Key" }),
        /* @__PURE__ */ n("p", { className: "mt-1", children: "You have a registered public key but no local private key. Enter your private key to restore access." }),
        S ? /* @__PURE__ */ c("div", { className: "mt-3 space-y-3", children: [
          /* @__PURE__ */ c("div", { children: [
            /* @__PURE__ */ n("label", { className: "block text-xs font-medium text-yellow-700 mb-1", children: "Private Key (Base64)" }),
            /* @__PURE__ */ n(
              "textarea",
              {
                value: N,
                onChange: (I) => T(I.target.value),
                placeholder: "Enter your private key here...",
                className: "w-full px-3 py-2 border border-yellow-300 rounded-md focus:outline-none focus:ring-2 focus:ring-yellow-500 text-xs font-mono",
                rows: 3
              }
            )
          ] }),
          y && /* @__PURE__ */ n("div", { className: `p-2 rounded-md text-xs ${y.valid ? "bg-green-50 border border-green-200 text-green-700" : "bg-red-50 border border-red-200 text-red-700"}`, children: y.valid ? /* @__PURE__ */ c("div", { className: "flex items-center", children: [
            /* @__PURE__ */ n(hr, { className: "h-4 w-4 text-green-600 mr-1" }),
            /* @__PURE__ */ n("span", { children: "Private key matches system public key!" })
          ] }) : /* @__PURE__ */ c("div", { className: "flex items-center", children: [
            /* @__PURE__ */ n(hn, { className: "h-4 w-4 text-red-600 mr-1" }),
            /* @__PURE__ */ n("span", { children: y.error })
          ] }) }),
          /* @__PURE__ */ c("div", { className: "flex gap-2", children: [
            /* @__PURE__ */ n(
              "button",
              {
                onClick: F,
                disabled: b || !N.trim(),
                className: "inline-flex items-center px-3 py-2 border border-transparent text-xs font-medium rounded-md shadow-sm text-white bg-green-600 hover:bg-green-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-green-500 disabled:opacity-50",
                children: b ? "Validating..." : "Validate & Import"
              }
            ),
            /* @__PURE__ */ n(
              "button",
              {
                onClick: C,
                className: "inline-flex items-center px-3 py-2 border border-gray-300 text-xs font-medium rounded-md shadow-sm text-gray-700 bg-white hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-yellow-500",
                children: "Cancel"
              }
            )
          ] }),
          /* @__PURE__ */ n("div", { className: "bg-red-50 border border-red-200 rounded-md p-2", children: /* @__PURE__ */ c("div", { className: "flex", children: [
            /* @__PURE__ */ n(hn, { className: "h-4 w-4 text-red-400 mr-1 flex-shrink-0" }),
            /* @__PURE__ */ c("div", { className: "text-xs text-red-700", children: [
              /* @__PURE__ */ n("p", { className: "font-medium", children: "Security Warning:" }),
              /* @__PURE__ */ n("p", { children: "Only enter your private key on trusted devices. Never share or store private keys in plain text." })
            ] })
          ] }) })
        ] }) : /* @__PURE__ */ c(
          "button",
          {
            onClick: () => _(!0),
            className: "mt-3 inline-flex items-center px-3 py-2 border border-transparent text-sm font-medium rounded-md shadow-sm text-white bg-yellow-600 hover:bg-yellow-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-yellow-500",
            children: [
              /* @__PURE__ */ n(mr, { className: "h-4 w-4 mr-1" }),
              "Import Private Key"
            ]
          }
        )
      ] })
    ] }) })
  ] });
}
function Zo({ isOpen: t, onClose: e }) {
  const [r, i] = B("ai"), [o, d] = B("OpenRouter"), [h, l] = B(""), [f, p] = B("anthropic/claude-3.5-sonnet"), [x, g] = B("https://openrouter.ai/api/v1"), [N, T] = B("llama3"), [b, E] = B("http://localhost:11434"), [y, w] = B(null), [S, _] = B(!1), { environment: R, setEnvironment: F } = Qa(), [P, C] = B(R.id), [I, O] = B({}), [U, K] = B({}), [V, H] = B("local"), [G, L] = B("data"), [j, J] = B("DataFoldStorage"), [ie, tt] = B("us-west-2"), [rt, X] = B(""), [oe, bt] = B(""), [nt, xt] = B("us-east-1"), [wt, vt] = B("folddb"), [Oe, st] = B("/tmp/folddb-data");
  de(() => {
    t && (Ft(), Me(), C(R.id), r === "schema-service" && Nt(R.id));
  }, [t, R.id, r]);
  const Ft = async () => {
    try {
      const D = await Je.getConfig();
      D.success && (l(D.data.openrouter.api_key || ""), p(D.data.openrouter.model || "anthropic/claude-3.5-sonnet"), g(D.data.openrouter.base_url || "https://openrouter.ai/api/v1"), T(D.data.ollama.model || "llama3"), E(D.data.ollama.base_url || "http://localhost:11434"), d(D.data.provider || "OpenRouter"));
    } catch (D) {
      console.error("Failed to load AI config:", D);
    }
  }, er = async () => {
    try {
      const D = {
        provider: o,
        openrouter: {
          api_key: h,
          model: f,
          base_url: x
        },
        ollama: {
          model: N,
          base_url: b
        }
      };
      (await Je.saveConfig(D)).success ? (w({ success: !0, message: "Configuration saved successfully" }), setTimeout(() => {
        w(null), e();
      }, 1500)) : w({ success: !1, message: "Failed to save configuration" });
    } catch (D) {
      w({ success: !1, message: D.message || "Failed to save configuration" });
    }
    setTimeout(() => w(null), 3e3);
  }, Nt = async (D) => {
    const Q = Object.values(ht).find((ye) => ye.id === D);
    if (Q) {
      K((ye) => ({ ...ye, [D]: !0 }));
      try {
        const ye = await za(Q.baseUrl);
        O((Pe) => ({
          ...Pe,
          [D]: ye
        }));
      } catch (ye) {
        O((Pe) => ({
          ...Pe,
          [D]: { success: !1, error: ye.message }
        }));
      } finally {
        K((ye) => ({ ...ye, [D]: !1 }));
      }
    }
  }, Me = async () => {
    try {
      const D = await bs();
      if (D.success && D.data) {
        const Q = D.data;
        H(Q.type), Q.type === "local" ? L(Q.path || "data") : Q.type === "dynamodb" ? (J(Q.table_name || "DataFoldStorage"), tt(Q.region || "us-west-2"), X(Q.user_id || "")) : Q.type === "s3" && (bt(Q.bucket || ""), xt(Q.region || "us-east-1"), vt(Q.prefix || "folddb"), st(Q.local_path || "/tmp/folddb-data"));
      }
    } catch (D) {
      console.error("Failed to load database config:", D);
    }
  }, ze = async () => {
    try {
      let D;
      if (V === "local")
        D = {
          type: "local",
          path: G
        };
      else if (V === "dynamodb") {
        if (!j || !ie) {
          w({ success: !1, message: "Table name and region are required for DynamoDB" }), setTimeout(() => w(null), 3e3);
          return;
        }
        D = {
          type: "dynamodb",
          table_name: j,
          region: ie,
          user_id: rt || void 0
        };
      } else if (V === "s3") {
        if (!oe || !nt) {
          w({ success: !1, message: "Bucket and region are required for S3" }), setTimeout(() => w(null), 3e3);
          return;
        }
        D = {
          type: "s3",
          bucket: oe,
          region: nt,
          prefix: wt || "folddb",
          local_path: Oe || "/tmp/folddb-data"
        };
      }
      const Q = await xs(D);
      Q.success ? (w({
        success: !0,
        message: Q.data.requires_restart ? "Database configuration saved. Please restart the server for changes to take effect." : Q.data.message || "Database configuration saved and restarted successfully"
      }), setTimeout(() => {
        w(null), Q.data.requires_restart || e();
      }, 3e3)) : w({ success: !1, message: Q.error || "Failed to save database configuration" });
    } catch (D) {
      w({ success: !1, message: D.message || "Failed to save database configuration" });
    }
    setTimeout(() => w(null), 5e3);
  }, tr = () => {
    F(P), w({ success: !0, message: "Schema service environment updated successfully" }), setTimeout(() => {
      w(null), e();
    }, 1500);
  }, rr = (D) => {
    const Q = I[D];
    return U[D] ? /* @__PURE__ */ c("span", { className: "inline-flex items-center text-xs bg-gray-100 text-gray-700 px-2 py-1 rounded", children: [
      /* @__PURE__ */ c("svg", { className: "animate-spin h-3 w-3 mr-1", viewBox: "0 0 24 24", children: [
        /* @__PURE__ */ n("circle", { className: "opacity-25", cx: "12", cy: "12", r: "10", stroke: "currentColor", strokeWidth: "4", fill: "none" }),
        /* @__PURE__ */ n("path", { className: "opacity-75", fill: "currentColor", d: "M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" })
      ] }),
      "Checking..."
    ] }) : Q ? Q.success ? /* @__PURE__ */ c("span", { className: "inline-flex items-center text-xs bg-green-100 text-green-700 px-2 py-1 rounded", children: [
      "✓ Online ",
      Q.responseTime && `(${Q.responseTime}ms)`
    ] }) : /* @__PURE__ */ n("span", { className: "inline-flex items-center text-xs bg-red-100 text-red-700 px-2 py-1 rounded", title: Q.error, children: "✗ Offline" }) : /* @__PURE__ */ n(
      "button",
      {
        onClick: (Pe) => {
          Pe.stopPropagation(), Nt(D);
        },
        className: "text-xs text-blue-600 hover:text-blue-700 underline",
        children: "Test Connection"
      }
    );
  };
  return t ? /* @__PURE__ */ n("div", { className: "fixed inset-0 z-50 overflow-y-auto", children: /* @__PURE__ */ c("div", { className: "flex items-center justify-center min-h-screen px-4 pt-4 pb-20 text-center sm:block sm:p-0", children: [
    /* @__PURE__ */ n(
      "div",
      {
        className: "fixed inset-0 transition-opacity bg-gray-500 bg-opacity-75",
        onClick: e
      }
    ),
    /* @__PURE__ */ c("div", { className: "inline-block align-bottom bg-white rounded-lg text-left overflow-hidden shadow-xl transform transition-all sm:my-8 sm:align-middle sm:max-w-4xl sm:w-full", children: [
      /* @__PURE__ */ c("div", { className: "bg-white", children: [
        /* @__PURE__ */ c("div", { className: "flex items-center justify-between px-6 pt-5 pb-4 border-b border-gray-200", children: [
          /* @__PURE__ */ n("h3", { className: "text-lg font-medium text-gray-900", children: "Settings" }),
          /* @__PURE__ */ n(
            "button",
            {
              onClick: e,
              className: "text-gray-400 hover:text-gray-600 transition-colors",
              children: /* @__PURE__ */ n("svg", { className: "w-6 h-6", fill: "none", stroke: "currentColor", viewBox: "0 0 24 24", children: /* @__PURE__ */ n("path", { strokeLinecap: "round", strokeLinejoin: "round", strokeWidth: 2, d: "M6 18L18 6M6 6l12 12" }) })
            }
          )
        ] }),
        /* @__PURE__ */ n("div", { className: "border-b border-gray-200", children: /* @__PURE__ */ c("nav", { className: "flex px-6", children: [
          /* @__PURE__ */ n(
            "button",
            {
              onClick: () => i("ai"),
              className: `py-3 px-4 text-sm font-medium border-b-2 transition-colors ${r === "ai" ? "border-blue-500 text-blue-600" : "border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300"}`,
              children: "AI Configuration"
            }
          ),
          /* @__PURE__ */ n(
            "button",
            {
              onClick: () => i("transforms"),
              className: `py-3 px-4 text-sm font-medium border-b-2 transition-colors ${r === "transforms" ? "border-blue-500 text-blue-600" : "border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300"}`,
              children: "Transforms"
            }
          ),
          /* @__PURE__ */ n(
            "button",
            {
              onClick: () => i("keys"),
              className: `py-3 px-4 text-sm font-medium border-b-2 transition-colors ${r === "keys" ? "border-blue-500 text-blue-600" : "border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300"}`,
              children: "Key Management"
            }
          ),
          /* @__PURE__ */ n(
            "button",
            {
              onClick: () => i("schema-service"),
              className: `py-3 px-4 text-sm font-medium border-b-2 transition-colors ${r === "schema-service" ? "border-blue-500 text-blue-600" : "border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300"}`,
              children: "Schema Service"
            }
          ),
          /* @__PURE__ */ n(
            "button",
            {
              onClick: () => i("database"),
              className: `py-3 px-4 text-sm font-medium border-b-2 transition-colors ${r === "database" ? "border-blue-500 text-blue-600" : "border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300"}`,
              children: "Database"
            }
          )
        ] }) }),
        /* @__PURE__ */ c("div", { className: "px-6 py-4 max-h-[70vh] overflow-y-auto", children: [
          r === "ai" && /* @__PURE__ */ c("div", { className: "space-y-4", children: [
            /* @__PURE__ */ c("div", { className: "grid grid-cols-1 md:grid-cols-2 gap-4", children: [
              /* @__PURE__ */ c("div", { children: [
                /* @__PURE__ */ n("label", { className: "block text-sm font-medium text-gray-700 mb-1", children: "Provider" }),
                /* @__PURE__ */ c(
                  "select",
                  {
                    value: o,
                    onChange: (D) => d(D.target.value),
                    className: "w-full p-2 border border-gray-300 rounded text-sm",
                    children: [
                      /* @__PURE__ */ n("option", { value: "OpenRouter", children: "OpenRouter" }),
                      /* @__PURE__ */ n("option", { value: "Ollama", children: "Ollama" })
                    ]
                  }
                )
              ] }),
              o === "OpenRouter" ? /* @__PURE__ */ c("div", { children: [
                /* @__PURE__ */ n("label", { className: "block text-sm font-medium text-gray-700 mb-1", children: "Model" }),
                /* @__PURE__ */ c(
                  "select",
                  {
                    value: f,
                    onChange: (D) => p(D.target.value),
                    className: "w-full p-2 border border-gray-300 rounded text-sm",
                    children: [
                      /* @__PURE__ */ n("option", { value: "anthropic/claude-3.5-sonnet", children: "Claude 3.5 Sonnet" }),
                      /* @__PURE__ */ n("option", { value: "anthropic/claude-3.5-haiku", children: "Claude 3.5 Haiku" }),
                      /* @__PURE__ */ n("option", { value: "openai/gpt-4o", children: "GPT-4o" }),
                      /* @__PURE__ */ n("option", { value: "openai/gpt-4o-mini", children: "GPT-4o Mini" }),
                      /* @__PURE__ */ n("option", { value: "openai/o1", children: "OpenAI o1" }),
                      /* @__PURE__ */ n("option", { value: "openai/o1-mini", children: "OpenAI o1-mini" }),
                      /* @__PURE__ */ n("option", { value: "google/gemini-2.0-flash-exp", children: "Gemini 2.0 Flash" }),
                      /* @__PURE__ */ n("option", { value: "google/gemini-pro-1.5", children: "Gemini 1.5 Pro" }),
                      /* @__PURE__ */ n("option", { value: "meta-llama/llama-3.3-70b-instruct", children: "Llama 3.3 70B" }),
                      /* @__PURE__ */ n("option", { value: "meta-llama/llama-3.1-405b-instruct", children: "Llama 3.1 405B" }),
                      /* @__PURE__ */ n("option", { value: "deepseek/deepseek-chat", children: "DeepSeek Chat" }),
                      /* @__PURE__ */ n("option", { value: "qwen/qwen-2.5-72b-instruct", children: "Qwen 2.5 72B" })
                    ]
                  }
                )
              ] }) : /* @__PURE__ */ c("div", { children: [
                /* @__PURE__ */ n("label", { className: "block text-sm font-medium text-gray-700 mb-1", children: "Model" }),
                /* @__PURE__ */ n(
                  "input",
                  {
                    type: "text",
                    value: N,
                    onChange: (D) => T(D.target.value),
                    placeholder: "e.g., llama3",
                    className: "w-full p-2 border border-gray-300 rounded text-sm"
                  }
                )
              ] })
            ] }),
            o === "OpenRouter" && /* @__PURE__ */ c("div", { children: [
              /* @__PURE__ */ c("label", { className: "block text-sm font-medium text-gray-700 mb-1", children: [
                "API Key ",
                /* @__PURE__ */ c("span", { className: "text-xs text-gray-500", children: [
                  "(",
                  /* @__PURE__ */ n("a", { href: "https://openrouter.ai/keys", target: "_blank", rel: "noopener noreferrer", className: "text-blue-600 hover:underline", children: "get key" }),
                  ")"
                ] })
              ] }),
              /* @__PURE__ */ n(
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
            /* @__PURE__ */ c("div", { children: [
              /* @__PURE__ */ c(
                "button",
                {
                  onClick: () => _(!S),
                  className: "text-sm text-gray-600 hover:text-gray-800 flex items-center gap-1",
                  children: [
                    /* @__PURE__ */ n("span", { children: S ? "▼" : "▶" }),
                    "Advanced Settings"
                  ]
                }
              ),
              S && /* @__PURE__ */ n("div", { className: "mt-3 space-y-3 pl-4 border-l-2 border-gray-200", children: /* @__PURE__ */ c("div", { children: [
                /* @__PURE__ */ n("label", { className: "block text-sm font-medium text-gray-700 mb-1", children: "Base URL" }),
                /* @__PURE__ */ n(
                  "input",
                  {
                    type: "text",
                    value: o === "OpenRouter" ? x : b,
                    onChange: (D) => o === "OpenRouter" ? g(D.target.value) : E(D.target.value),
                    placeholder: o === "OpenRouter" ? "https://openrouter.ai/api/v1" : "http://localhost:11434",
                    className: "w-full p-2 border border-gray-300 rounded text-sm"
                  }
                )
              ] }) })
            ] }),
            y && /* @__PURE__ */ n("div", { className: `p-3 rounded-md ${y.success ? "bg-green-50 text-green-800 border border-green-200" : "bg-red-50 text-red-800 border border-red-200"}`, children: /* @__PURE__ */ c("span", { className: "text-sm font-medium", children: [
              y.success ? "✓" : "✗",
              " ",
              y.message
            ] }) })
          ] }),
          r === "transforms" && /* @__PURE__ */ n(Si, { onResult: () => {
          } }),
          r === "keys" && /* @__PURE__ */ n(Ii, { onResult: () => {
          } }),
          r === "schema-service" && /* @__PURE__ */ c("div", { className: "space-y-4", children: [
            /* @__PURE__ */ c("div", { className: "mb-4", children: [
              /* @__PURE__ */ n("h4", { className: "text-md font-semibold text-gray-900 mb-2", children: "Schema Service Environment" }),
              /* @__PURE__ */ n("p", { className: "text-sm text-gray-600 mb-4", children: "Select which schema service endpoint to use. This affects where schemas are loaded from and saved to." })
            ] }),
            /* @__PURE__ */ n("div", { className: "space-y-3", children: Object.values(ht).map((D) => /* @__PURE__ */ c(
              "label",
              {
                className: `flex items-start p-4 border-2 rounded-lg cursor-pointer transition-all ${P === D.id ? "border-blue-500 bg-blue-50" : "border-gray-200 hover:border-gray-300 bg-white"}`,
                children: [
                  /* @__PURE__ */ n(
                    "input",
                    {
                      type: "radio",
                      name: "schemaEnvironment",
                      value: D.id,
                      checked: P === D.id,
                      onChange: (Q) => C(Q.target.value),
                      className: "mt-1 mr-3"
                    }
                  ),
                  /* @__PURE__ */ c("div", { className: "flex-1", children: [
                    /* @__PURE__ */ c("div", { className: "flex items-center justify-between mb-2", children: [
                      /* @__PURE__ */ n("span", { className: "text-sm font-semibold text-gray-900", children: D.name }),
                      /* @__PURE__ */ c("div", { className: "flex items-center gap-2", children: [
                        rr(D.id),
                        P === D.id && /* @__PURE__ */ n("span", { className: "text-xs bg-blue-100 text-blue-700 px-2 py-1 rounded", children: "Active" })
                      ] })
                    ] }),
                    /* @__PURE__ */ n("p", { className: "text-xs text-gray-600 mt-1", children: D.description }),
                    /* @__PURE__ */ n("p", { className: "text-xs text-gray-500 mt-1 font-mono", children: D.baseUrl || window.location.origin }),
                    I[D.id] && !I[D.id].success && /* @__PURE__ */ c("p", { className: "text-xs text-red-600 mt-1", children: [
                      "Error: ",
                      I[D.id].error
                    ] })
                  ] })
                ]
              },
              D.id
            )) }),
            y && /* @__PURE__ */ n("div", { className: `p-3 rounded-md ${y.success ? "bg-green-50 text-green-800 border border-green-200" : "bg-red-50 text-red-800 border border-red-200"}`, children: /* @__PURE__ */ c("span", { className: "text-sm font-medium", children: [
              y.success ? "✓" : "✗",
              " ",
              y.message
            ] }) })
          ] }),
          r === "database" && /* @__PURE__ */ c("div", { className: "space-y-4", children: [
            /* @__PURE__ */ c("div", { className: "mb-4", children: [
              /* @__PURE__ */ n("h4", { className: "text-md font-semibold text-gray-900 mb-2", children: "Database Storage Backend" }),
              /* @__PURE__ */ n("p", { className: "text-sm text-gray-600 mb-4", children: "Choose the storage backend for your database. Changes require a server restart." })
            ] }),
            /* @__PURE__ */ c("div", { children: [
              /* @__PURE__ */ n("label", { className: "block text-sm font-medium text-gray-700 mb-2", children: "Storage Type" }),
              /* @__PURE__ */ c(
                "select",
                {
                  value: V,
                  onChange: (D) => H(D.target.value),
                  className: "w-full p-2 border border-gray-300 rounded text-sm",
                  children: [
                    /* @__PURE__ */ n("option", { value: "local", children: "Local (Sled)" }),
                    /* @__PURE__ */ n("option", { value: "dynamodb", children: "DynamoDB" }),
                    /* @__PURE__ */ n("option", { value: "s3", children: "S3" })
                  ]
                }
              )
            ] }),
            V === "local" ? /* @__PURE__ */ c("div", { children: [
              /* @__PURE__ */ n("label", { className: "block text-sm font-medium text-gray-700 mb-1", children: "Storage Path" }),
              /* @__PURE__ */ n(
                "input",
                {
                  type: "text",
                  value: G,
                  onChange: (D) => L(D.target.value),
                  placeholder: "data",
                  className: "w-full p-2 border border-gray-300 rounded text-sm"
                }
              ),
              /* @__PURE__ */ n("p", { className: "text-xs text-gray-500 mt-1", children: "Local filesystem path where the database will be stored" })
            ] }) : V === "dynamodb" ? /* @__PURE__ */ c("div", { className: "space-y-3", children: [
              /* @__PURE__ */ c("div", { children: [
                /* @__PURE__ */ c("label", { className: "block text-sm font-medium text-gray-700 mb-1", children: [
                  "Table Name ",
                  /* @__PURE__ */ n("span", { className: "text-red-500", children: "*" })
                ] }),
                /* @__PURE__ */ n(
                  "input",
                  {
                    type: "text",
                    value: j,
                    onChange: (D) => J(D.target.value),
                    placeholder: "DataFoldStorage",
                    className: "w-full p-2 border border-gray-300 rounded text-sm"
                  }
                ),
                /* @__PURE__ */ n("p", { className: "text-xs text-gray-500 mt-1", children: "Base table name (namespaces will be appended automatically)" })
              ] }),
              /* @__PURE__ */ c("div", { children: [
                /* @__PURE__ */ c("label", { className: "block text-sm font-medium text-gray-700 mb-1", children: [
                  "AWS Region ",
                  /* @__PURE__ */ n("span", { className: "text-red-500", children: "*" })
                ] }),
                /* @__PURE__ */ n(
                  "input",
                  {
                    type: "text",
                    value: ie,
                    onChange: (D) => tt(D.target.value),
                    placeholder: "us-west-2",
                    className: "w-full p-2 border border-gray-300 rounded text-sm"
                  }
                ),
                /* @__PURE__ */ n("p", { className: "text-xs text-gray-500 mt-1", children: "AWS region where your DynamoDB tables are located" })
              ] }),
              /* @__PURE__ */ c("div", { children: [
                /* @__PURE__ */ n("label", { className: "block text-sm font-medium text-gray-700 mb-1", children: "User ID (Optional)" }),
                /* @__PURE__ */ n(
                  "input",
                  {
                    type: "text",
                    value: rt,
                    onChange: (D) => X(D.target.value),
                    placeholder: "Leave empty for single-tenant",
                    className: "w-full p-2 border border-gray-300 rounded text-sm"
                  }
                ),
                /* @__PURE__ */ n("p", { className: "text-xs text-gray-500 mt-1", children: "User ID for multi-tenant isolation (uses partition key)" })
              ] }),
              /* @__PURE__ */ n("div", { className: "p-3 bg-yellow-50 border border-yellow-200 rounded-md", children: /* @__PURE__ */ c("p", { className: "text-xs text-yellow-800", children: [
                /* @__PURE__ */ n("strong", { children: "Note:" }),
                " Ensure your AWS credentials are configured (via environment variables, IAM role, or AWS CLI). The DynamoDB tables will be created automatically if they don't exist."
              ] }) })
            ] }) : /* @__PURE__ */ c("div", { className: "space-y-3", children: [
              /* @__PURE__ */ c("div", { children: [
                /* @__PURE__ */ c("label", { className: "block text-sm font-medium text-gray-700 mb-1", children: [
                  "S3 Bucket ",
                  /* @__PURE__ */ n("span", { className: "text-red-500", children: "*" })
                ] }),
                /* @__PURE__ */ n(
                  "input",
                  {
                    type: "text",
                    value: oe,
                    onChange: (D) => bt(D.target.value),
                    placeholder: "my-datafold-bucket",
                    className: "w-full p-2 border border-gray-300 rounded text-sm"
                  }
                ),
                /* @__PURE__ */ n("p", { className: "text-xs text-gray-500 mt-1", children: "S3 bucket name where the database will be stored" })
              ] }),
              /* @__PURE__ */ c("div", { children: [
                /* @__PURE__ */ c("label", { className: "block text-sm font-medium text-gray-700 mb-1", children: [
                  "AWS Region ",
                  /* @__PURE__ */ n("span", { className: "text-red-500", children: "*" })
                ] }),
                /* @__PURE__ */ n(
                  "input",
                  {
                    type: "text",
                    value: nt,
                    onChange: (D) => xt(D.target.value),
                    placeholder: "us-east-1",
                    className: "w-full p-2 border border-gray-300 rounded text-sm"
                  }
                ),
                /* @__PURE__ */ n("p", { className: "text-xs text-gray-500 mt-1", children: "AWS region where your S3 bucket is located" })
              ] }),
              /* @__PURE__ */ c("div", { children: [
                /* @__PURE__ */ n("label", { className: "block text-sm font-medium text-gray-700 mb-1", children: "S3 Prefix (Optional)" }),
                /* @__PURE__ */ n(
                  "input",
                  {
                    type: "text",
                    value: wt,
                    onChange: (D) => vt(D.target.value),
                    placeholder: "folddb",
                    className: "w-full p-2 border border-gray-300 rounded text-sm"
                  }
                ),
                /* @__PURE__ */ n("p", { className: "text-xs text-gray-500 mt-1", children: 'Prefix/path within the bucket (defaults to "folddb")' })
              ] }),
              /* @__PURE__ */ c("div", { children: [
                /* @__PURE__ */ n("label", { className: "block text-sm font-medium text-gray-700 mb-1", children: "Local Cache Path" }),
                /* @__PURE__ */ n(
                  "input",
                  {
                    type: "text",
                    value: Oe,
                    onChange: (D) => st(D.target.value),
                    placeholder: "/tmp/folddb-data",
                    className: "w-full p-2 border border-gray-300 rounded text-sm"
                  }
                ),
                /* @__PURE__ */ n("p", { className: "text-xs text-gray-500 mt-1", children: "Local filesystem path for caching S3 data (defaults to /tmp/folddb-data)" })
              ] }),
              /* @__PURE__ */ n("div", { className: "p-3 bg-yellow-50 border border-yellow-200 rounded-md", children: /* @__PURE__ */ c("p", { className: "text-xs text-yellow-800", children: [
                /* @__PURE__ */ n("strong", { children: "Note:" }),
                " Ensure your AWS credentials are configured (via environment variables, IAM role, or AWS CLI). The database will be synced to/from S3 on startup and shutdown."
              ] }) })
            ] }),
            y && /* @__PURE__ */ n("div", { className: `p-3 rounded-md ${y.success ? "bg-green-50 text-green-800 border border-green-200" : "bg-red-50 text-red-800 border border-red-200"}`, children: /* @__PURE__ */ c("span", { className: "text-sm font-medium", children: [
              y.success ? "✓" : "✗",
              " ",
              y.message
            ] }) })
          ] })
        ] })
      ] }),
      /* @__PURE__ */ n("div", { className: "bg-gray-50 px-4 py-3 sm:px-6 sm:flex sm:flex-row-reverse gap-3 border-t border-gray-200", children: r === "ai" || r === "schema-service" || r === "database" ? /* @__PURE__ */ c(Rt, { children: [
        /* @__PURE__ */ n(
          "button",
          {
            onClick: r === "ai" ? er : r === "schema-service" ? tr : ze,
            className: "w-full inline-flex justify-center rounded-md border border-transparent shadow-sm px-4 py-2 bg-blue-600 text-base font-medium text-white hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 sm:ml-3 sm:w-auto sm:text-sm",
            children: r === "database" ? "Save and Restart DB" : "Save Configuration"
          }
        ),
        /* @__PURE__ */ n(
          "button",
          {
            onClick: e,
            className: "mt-3 w-full inline-flex justify-center rounded-md border border-gray-300 shadow-sm px-4 py-2 bg-white text-base font-medium text-gray-700 hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 sm:mt-0 sm:w-auto sm:text-sm",
            children: "Cancel"
          }
        )
      ] }) : /* @__PURE__ */ n(
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
function Xo() {
  const [t, e] = B([]), r = At(null), i = (d) => {
    if (typeof d == "string") return d;
    const h = d.metadata ? JSON.stringify(d.metadata) : "";
    return `[${d.level}] [${d.event_type}] - ${d.message} ${h}`;
  }, o = () => {
    Promise.resolve(
      navigator.clipboard.writeText(t.map(i).join(`
`))
    ).catch(() => {
    });
  };
  return de(() => {
    ae.getLogs().then((l) => {
      if (l.success && l.data) {
        const f = l.data.logs || [];
        e(Array.isArray(f) ? f : []);
      } else
        e([]);
    }).catch(() => e([]));
    const d = ae.createLogStream(
      (l) => {
        e((f) => {
          let p;
          try {
            p = JSON.parse(l);
          } catch {
            const x = l.split(" - "), g = x.length > 1 ? x[0] : "INFO";
            p = {
              id: `stream-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
              timestamp: Date.now(),
              level: g,
              event_type: "stream (legacy)",
              message: l
            };
          }
          return p.id && f.some((x) => x.id === p.id) ? f : [...f, p];
        });
      },
      (l) => {
        console.warn("Log stream error:", l);
      }
    ), h = setInterval(() => {
      e((l) => {
        const f = l.length > 0 ? l[l.length - 1] : null, p = f ? f.timestamp : void 0;
        return ae.getLogs(p).then((x) => {
          if (x.success && x.data) {
            const g = x.data.logs || [];
            g.length > 0 && e((N) => {
              const T = g.filter((b) => b.id && N.some((w) => w.id === b.id) ? !1 : !N.some(
                (y) => !y.id && // Only check content if existing has no ID
                y.timestamp === b.timestamp && y.message === b.message
              ));
              return T.length === 0 ? N : [...N, ...T];
            });
          }
        }).catch((x) => console.warn("Log polling error:", x)), l;
      });
    }, 2e3);
    return () => {
      d.close(), clearInterval(h);
    };
  }, []), de(() => {
    var d;
    (d = r.current) == null || d.scrollIntoView({ behavior: "smooth" });
  }, [t]), /* @__PURE__ */ c("aside", { className: "w-80 bg-gray-900 text-white flex flex-col overflow-hidden", children: [
    /* @__PURE__ */ c("div", { className: "flex items-center justify-between p-4 border-b border-gray-700", children: [
      /* @__PURE__ */ n("h2", { className: "text-lg font-semibold", children: "Logs" }),
      /* @__PURE__ */ n(
        "button",
        {
          onClick: o,
          className: "text-xs text-blue-300 hover:underline",
          children: "Copy"
        }
      )
    ] }),
    /* @__PURE__ */ c("div", { className: "flex-1 overflow-y-auto p-4 space-y-1 text-xs font-mono", children: [
      t.map((d, h) => /* @__PURE__ */ n("div", { children: i(d) }, d.id || h)),
      /* @__PURE__ */ n("div", { ref: r })
    ] })
  ] });
}
function el({ onSettingsClick: t }) {
  const e = Ge(), { isAuthenticated: r, user: i } = te((d) => d.auth), o = () => {
    e(la()), localStorage.removeItem("fold_user_id"), localStorage.removeItem("fold_user_hash");
  };
  return /* @__PURE__ */ n("header", { className: "bg-white border-b border-gray-200 shadow-sm flex-shrink-0", children: /* @__PURE__ */ c("div", { className: "flex items-center justify-between px-6 py-3", children: [
    /* @__PURE__ */ c("a", { href: "/", className: "flex items-center gap-3 text-blue-600 hover:text-blue-700 transition-colors", children: [
      /* @__PURE__ */ c("svg", { className: "w-8 h-8 flex-shrink-0", viewBox: "0 0 24 24", fill: "currentColor", children: [
        /* @__PURE__ */ n("path", { d: "M12 4C7.58172 4 4 5.79086 4 8C4 10.2091 7.58172 12 12 12C16.4183 12 20 10.2091 20 8C20 5.79086 16.4183 4 12 4Z" }),
        /* @__PURE__ */ n("path", { d: "M4 12V16C4 18.2091 7.58172 20 12 20C16.4183 20 20 18.2091 20 16V12", strokeWidth: "2", strokeLinecap: "round" }),
        /* @__PURE__ */ n("path", { d: "M4 8V12C4 14.2091 7.58172 16 12 16C16.4183 16 20 14.2091 20 12V8", strokeWidth: "2", strokeLinecap: "round" })
      ] }),
      /* @__PURE__ */ n("span", { className: "text-xl font-semibold text-gray-900", children: "DataFold Node" })
    ] }),
    /* @__PURE__ */ c("div", { className: "flex items-center gap-3", children: [
      r && /* @__PURE__ */ c("div", { className: "flex items-center gap-3 mr-2", children: [
        /* @__PURE__ */ n("span", { className: "text-sm text-gray-600", children: i == null ? void 0 : i.id }),
        /* @__PURE__ */ n(
          "button",
          {
            onClick: o,
            className: "text-sm text-red-600 hover:text-red-700 font-medium",
            children: "Logout"
          }
        )
      ] }),
      /* @__PURE__ */ n("div", { className: "h-6 w-px bg-gray-300 mx-1" }),
      /* @__PURE__ */ c(
        "button",
        {
          onClick: t,
          className: "inline-flex items-center gap-2 px-3 py-2 text-sm text-gray-700 hover:bg-gray-100 rounded-md border border-gray-300 transition-colors",
          title: "Settings",
          children: [
            /* @__PURE__ */ c("svg", { className: "w-4 h-4", fill: "none", stroke: "currentColor", viewBox: "0 0 24 24", children: [
              /* @__PURE__ */ n("path", { strokeLinecap: "round", strokeLinejoin: "round", strokeWidth: 2, d: "M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" }),
              /* @__PURE__ */ n("path", { strokeLinecap: "round", strokeLinejoin: "round", strokeWidth: 2, d: "M15 12a3 3 0 11-6 0 3 3 0 016 0z" })
            ] }),
            "Settings"
          ]
        }
      )
    ] })
  ] }) });
}
function tl() {
  return /* @__PURE__ */ n("footer", { className: "bg-white border-t border-gray-200 py-3", children: /* @__PURE__ */ n("div", { className: "max-w-7xl mx-auto px-6 text-center", children: /* @__PURE__ */ c("p", { className: "text-gray-600 text-sm", children: [
    "DataFold Node © ",
    (/* @__PURE__ */ new Date()).getFullYear()
  ] }) }) });
}
function rl() {
  const [t, e] = B(""), [r, i] = B(""), o = Ge(), { isLoading: d } = te((l) => l.auth);
  return /* @__PURE__ */ c("div", { className: "min-h-screen bg-gray-50 flex flex-col justify-center py-12 sm:px-6 lg:px-8", children: [
    /* @__PURE__ */ c("div", { className: "sm:mx-auto sm:w-full sm:max-w-md", children: [
      /* @__PURE__ */ n("h2", { className: "mt-6 text-center text-3xl font-extrabold text-gray-900", children: "Sign in to Exemem" }),
      /* @__PURE__ */ n("p", { className: "mt-2 text-center text-sm text-gray-600", children: "Enter your user identifier to access your Exemem node" })
    ] }),
    /* @__PURE__ */ n("div", { className: "mt-8 sm:mx-auto sm:w-full sm:max-w-md", children: /* @__PURE__ */ n("div", { className: "bg-white py-8 px-4 shadow sm:rounded-lg sm:px-10", children: /* @__PURE__ */ c("form", { className: "space-y-6", onSubmit: async (l) => {
      if (l.preventDefault(), !t.trim()) {
        i("Please enter a user identifier");
        return;
      }
      try {
        const f = await o(Or(t.trim())).unwrap();
        localStorage.setItem("fold_user_id", f.id), localStorage.setItem("fold_user_hash", f.hash);
      } catch (f) {
        i("Login failed: " + f.message);
      }
    }, children: [
      /* @__PURE__ */ c("div", { children: [
        /* @__PURE__ */ n("label", { htmlFor: "userId", className: "block text-sm font-medium text-gray-700", children: "User Identifier" }),
        /* @__PURE__ */ n("div", { className: "mt-1", children: /* @__PURE__ */ n(
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
      r && /* @__PURE__ */ n("div", { className: "text-sm text-red-600", children: r }),
      /* @__PURE__ */ n("div", { children: /* @__PURE__ */ n(
        "button",
        {
          type: "submit",
          disabled: d,
          className: "w-full flex justify-center py-2 px-4 border border-transparent rounded-md shadow-sm text-sm font-medium text-white bg-blue-600 hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 disabled:opacity-50",
          children: d ? "Connecting..." : "Continue"
        }
      ) })
    ] }) }) })
  ] });
}
function nl() {
  const [t, e] = B(""), [r, i] = B(""), o = Ge(), { isAuthenticated: d, isLoading: h } = te((f) => f.auth);
  return d ? null : /* @__PURE__ */ n("div", { className: "fixed inset-0 z-50 overflow-y-auto", children: /* @__PURE__ */ c("div", { className: "flex items-center justify-center min-h-screen px-4 pt-4 pb-20 text-center sm:block sm:p-0", children: [
    /* @__PURE__ */ n("div", { className: "fixed inset-0 transition-opacity bg-gray-900 bg-opacity-75" }),
    /* @__PURE__ */ n("span", { className: "hidden sm:inline-block sm:align-middle sm:h-screen", children: "​" }),
    /* @__PURE__ */ n("div", { className: "inline-block align-bottom bg-white rounded-lg text-left overflow-hidden shadow-xl transform transition-all sm:my-8 sm:align-middle sm:max-w-lg sm:w-full", children: /* @__PURE__ */ n("div", { className: "bg-white px-4 pt-5 pb-4 sm:p-6 sm:pb-4", children: /* @__PURE__ */ n("div", { className: "sm:flex sm:items-start", children: /* @__PURE__ */ c("div", { className: "mt-3 text-center sm:mt-0 sm:text-left w-full", children: [
      /* @__PURE__ */ n("h3", { className: "text-xl leading-6 font-medium text-gray-900 mb-2", children: "Welcome to DataFold" }),
      /* @__PURE__ */ c("div", { className: "mt-2", children: [
        /* @__PURE__ */ n("p", { className: "text-sm text-gray-500 mb-4", children: "Enter your user identifier to continue. This will generate a unique session hash for your environment." }),
        /* @__PURE__ */ c("form", { onSubmit: async (f) => {
          if (f.preventDefault(), !t.trim()) {
            i("Please enter a user identifier");
            return;
          }
          try {
            const p = await o(Or(t.trim())).unwrap();
            localStorage.setItem("fold_user_id", p.id), localStorage.setItem("fold_user_hash", p.hash);
          } catch (p) {
            i("Login failed: " + p.message);
          }
        }, children: [
          /* @__PURE__ */ c("div", { className: "mb-4", children: [
            /* @__PURE__ */ n("label", { htmlFor: "userId", className: "block text-sm font-medium text-gray-700 mb-1", children: "User Identifier" }),
            /* @__PURE__ */ n(
              "input",
              {
                type: "text",
                id: "userId",
                className: "shadow-sm focus:ring-blue-500 focus:border-blue-500 block w-full sm:text-sm border-gray-300 rounded-md p-2 border",
                placeholder: "e.g. alice-dev",
                value: t,
                onChange: (f) => {
                  e(f.target.value), i("");
                },
                autoFocus: !0
              }
            )
          ] }),
          r && /* @__PURE__ */ n("div", { className: "mb-4 text-sm text-red-600", children: r }),
          /* @__PURE__ */ n(
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
function ki({ progress: t, className: e = "" }) {
  if (!t)
    return null;
  const r = (d) => {
    switch (d) {
      case "ValidatingConfig":
        return "bg-blue-500";
      case "PreparingSchemas":
        return "bg-indigo-500";
      case "FlatteningData":
        return "bg-purple-500";
      case "GettingAIRecommendation":
        return "bg-pink-500";
      case "SettingUpSchema":
        return "bg-red-500";
      case "GeneratingMutations":
        return "bg-orange-500";
      case "ExecutingMutations":
        return "bg-yellow-500";
      case "Completed":
        return "bg-green-500";
      case "Failed":
        return "bg-red-600";
      default:
        return "bg-gray-500";
    }
  }, i = (d) => {
    switch (d) {
      case "ValidatingConfig":
        return "Validating Configuration";
      case "PreparingSchemas":
        return "Preparing Schemas";
      case "FlatteningData":
        return "Processing Data";
      case "GettingAIRecommendation":
        return "AI Analysis";
      case "SettingUpSchema":
        return "Setting Up Schema";
      case "GeneratingMutations":
        return "Generating Mutations";
      case "ExecutingMutations":
        return "Executing Mutations";
      case "Completed":
        return "Completed";
      case "Failed":
        return "Failed";
      default:
        return d;
    }
  }, o = (d, h) => {
    const l = new Date(d * 1e3), f = h ? new Date(h * 1e3) : /* @__PURE__ */ new Date(), p = Math.round((f - l) / 1e3);
    if (p < 60)
      return `${p}s`;
    {
      const x = Math.floor(p / 60), g = p % 60;
      return `${x}m ${g}s`;
    }
  };
  return /* @__PURE__ */ c("div", { className: `bg-white p-4 rounded-lg shadow border ${e}`, children: [
    /* @__PURE__ */ c("div", { className: "flex items-center justify-between mb-3", children: [
      /* @__PURE__ */ c("div", { className: "flex items-center gap-2", children: [
        /* @__PURE__ */ n("div", { className: `w-3 h-3 rounded-full ${r(t.current_step)}` }),
        /* @__PURE__ */ n("h3", { className: "text-sm font-medium text-gray-900", children: i(t.current_step) })
      ] }),
      /* @__PURE__ */ n("div", { className: "text-xs text-gray-500", children: o(t.started_at, t.completed_at) })
    ] }),
    /* @__PURE__ */ c("div", { className: "mb-3", children: [
      /* @__PURE__ */ c("div", { className: "flex justify-between text-xs text-gray-600 mb-1", children: [
        /* @__PURE__ */ c("span", { children: [
          t.progress_percentage,
          "%"
        ] }),
        /* @__PURE__ */ n("span", { children: t.status_message })
      ] }),
      /* @__PURE__ */ n("div", { className: "w-full bg-gray-200 rounded-full h-2", children: /* @__PURE__ */ n(
        "div",
        {
          className: `h-2 rounded-full transition-all duration-300 ${r(t.current_step)}`,
          style: { width: `${t.progress_percentage}%` }
        }
      ) })
    ] }),
    t.results && /* @__PURE__ */ n("div", { className: "mt-3 p-3 bg-green-50 rounded-md", children: /* @__PURE__ */ c("div", { className: "text-sm text-green-800", children: [
      /* @__PURE__ */ n("div", { className: "font-medium mb-1", children: "Ingestion Complete!" }),
      /* @__PURE__ */ c("div", { className: "text-xs space-y-1", children: [
        /* @__PURE__ */ c("div", { children: [
          "Schema: ",
          t.results.schema_name
        ] }),
        /* @__PURE__ */ c("div", { children: [
          "New Schema: ",
          t.results.new_schema_created ? "Yes" : "No"
        ] }),
        /* @__PURE__ */ c("div", { children: [
          "Mutations Generated: ",
          t.results.mutations_generated
        ] }),
        /* @__PURE__ */ c("div", { children: [
          "Mutations Executed: ",
          t.results.mutations_executed
        ] })
      ] })
    ] }) }),
    t.error_message && /* @__PURE__ */ n("div", { className: "mt-3 p-3 bg-red-50 rounded-md", children: /* @__PURE__ */ c("div", { className: "text-sm text-red-800", children: [
      /* @__PURE__ */ n("div", { className: "font-medium mb-1", children: "Ingestion Failed" }),
      /* @__PURE__ */ n("div", { className: "text-xs", children: t.error_message })
    ] }) }),
    /* @__PURE__ */ n("div", { className: "mt-4", children: /* @__PURE__ */ n("div", { className: "flex justify-between text-xs text-gray-500", children: [
      "ValidatingConfig",
      "PreparingSchemas",
      "FlatteningData",
      "GettingAIRecommendation",
      "SettingUpSchema",
      "GeneratingMutations",
      "ExecutingMutations"
    ].map((d, h) => {
      const l = t.current_step === d, f = t.progress_percentage > (h + 1) * 12.5;
      return /* @__PURE__ */ c("div", { className: "flex flex-col items-center", children: [
        /* @__PURE__ */ n(
          "div",
          {
            className: `w-2 h-2 rounded-full mb-1 ${l || f ? r(d) : "bg-gray-300"}`
          }
        ),
        /* @__PURE__ */ n("span", { className: "text-xs text-center max-w-16 leading-tight", children: i(d).split(" ")[0] })
      ] }, d);
    }) }) })
  ] });
}
function zt({ node: t, depth: e = 0, name: r = null }) {
  const [i, o] = B(e === 0);
  if (!t)
    return /* @__PURE__ */ n("span", { className: "text-gray-400 italic", children: "undefined" });
  if (t.type === "Primitive") {
    const d = t.value, h = {
      String: "text-green-600",
      Number: "text-blue-600",
      Boolean: "text-purple-600",
      Null: "text-gray-500"
    }[d] || "text-gray-600";
    return /* @__PURE__ */ c("span", { className: "inline-flex items-center space-x-2", children: [
      /* @__PURE__ */ n("span", { className: `font-mono text-sm ${h}`, children: d.toLowerCase() }),
      t.classifications && t.classifications.length > 0 && /* @__PURE__ */ n("span", { className: "flex space-x-1", children: t.classifications.map((l) => /* @__PURE__ */ n("span", { className: "px-1.5 py-0.5 text-xs bg-gray-200 text-gray-700 rounded-full font-sans", children: l }, l)) })
    ] });
  }
  if (t.type === "Any")
    return /* @__PURE__ */ n("span", { className: "font-mono text-sm text-orange-600", children: "any" });
  if (t.type === "Array")
    return /* @__PURE__ */ c("div", { className: "inline-flex items-start", children: [
      /* @__PURE__ */ n("span", { className: "font-mono text-sm text-gray-700", children: "Array<" }),
      /* @__PURE__ */ n(zt, { node: t.value, depth: e + 1 }),
      /* @__PURE__ */ n("span", { className: "font-mono text-sm text-gray-700", children: ">" })
    ] });
  if (t.type === "Object" && t.value) {
    const d = Object.entries(t.value);
    return d.length === 0 ? /* @__PURE__ */ n("span", { className: "font-mono text-sm text-gray-500", children: "{}" }) : /* @__PURE__ */ c("div", { className: "inline-block", children: [
      /* @__PURE__ */ n("div", { className: "flex items-center", children: /* @__PURE__ */ c(
        "button",
        {
          onClick: () => o(!i),
          className: "flex items-center hover:bg-gray-100 rounded px-1 -ml-1",
          children: [
            i ? /* @__PURE__ */ n(Un, { className: "h-3 w-3 text-gray-500" }) : /* @__PURE__ */ n($n, { className: "h-3 w-3 text-gray-500" }),
            /* @__PURE__ */ c("span", { className: "font-mono text-sm text-gray-700 ml-1", children: [
              "{",
              !i && `... ${d.length} fields`,
              !i && "}"
            ] })
          ]
        }
      ) }),
      i && /* @__PURE__ */ c("div", { className: "ml-4 border-l-2 border-gray-200 pl-3 mt-1", children: [
        d.map(([h, l], f) => /* @__PURE__ */ c("div", { className: "py-1", children: [
          /* @__PURE__ */ n("span", { className: "font-mono text-sm text-indigo-600", children: h }),
          /* @__PURE__ */ n("span", { className: "font-mono text-sm text-gray-500", children: ": " }),
          /* @__PURE__ */ n(zt, { node: l, depth: e + 1, name: h }),
          f < d.length - 1 && /* @__PURE__ */ n("span", { className: "text-gray-400", children: "," })
        ] }, h)),
        /* @__PURE__ */ n("div", { className: "font-mono text-sm text-gray-700", children: "}" })
      ] })
    ] });
  }
  return /* @__PURE__ */ c("span", { className: "font-mono text-sm text-red-500", children: [
    "unknown (",
    JSON.stringify(t),
    ")"
  ] });
}
function Bi({ topology: t, compact: e = !1 }) {
  return t ? e ? /* @__PURE__ */ n("div", { className: "inline-flex items-center", children: /* @__PURE__ */ n(zt, { node: t.root }) }) : /* @__PURE__ */ c("div", { className: "mt-2 p-2 bg-gray-50 rounded border border-gray-200", children: [
    /* @__PURE__ */ n("div", { className: "text-xs font-medium text-gray-600 mb-1", children: "Type Structure:" }),
    /* @__PURE__ */ n("div", { className: "pl-2", children: /* @__PURE__ */ n(zt, { node: t.root }) })
  ] }) : /* @__PURE__ */ n("div", { className: "text-xs text-gray-400 italic", children: "No topology defined" });
}
function sl({ onResult: t, onSchemaUpdated: e }) {
  const r = Ge(), i = te(gt);
  te(Lr), te(Ln);
  const [o, d] = B({});
  de(() => {
    console.log("🟢 SchemaTab: Fetching schemas on mount"), r(Ie({ forceRefresh: !0 }));
  }, [r]);
  const h = (b) => b.descriptive_name || b.name;
  console.log("🟢 SchemaTab: Current schemas from Redux:", i.map((b) => ({ name: b.name, state: b.state })));
  const l = async (b) => {
    const E = o[b];
    if (d((y) => ({
      ...y,
      [b]: !y[b]
    })), !E) {
      const y = i.find((w) => w.name === b);
      if (y && (!y.fields || Object.keys(y.fields).length === 0))
        try {
          (await re.getSchema(b)).success && (r(Ie({ forceRefresh: !0 })), e && e());
        } catch (w) {
          console.error(`Failed to fetch schema details for ${b}:`, w);
        }
    }
  }, f = (b) => {
    switch (b == null ? void 0 : b.toLowerCase()) {
      case "approved":
        return "bg-green-100 text-green-800";
      case "available":
        return "bg-blue-100 text-blue-800";
      case "blocked":
        return "bg-red-100 text-red-800";
      default:
        return "bg-gray-100 text-gray-800";
    }
  }, p = async (b) => {
    var E, y;
    console.log("🟡 SchemaTab: Starting approveSchema for:", b);
    try {
      const w = await r(He({ schemaName: b }));
      if (console.log("🟡 SchemaTab: approveSchema result:", w), He.fulfilled.match(w)) {
        console.log("🟡 SchemaTab: approveSchema fulfilled, calling callbacks");
        const S = (E = w.payload) == null ? void 0 : E.backfillHash;
        if (console.log("🔄 Backfill hash:", S), console.log("🔄 Refetching schemas from backend after approval..."), await r(Ie({ forceRefresh: !0 })), console.log("✅ Refetch complete - backend state should be reflected"), t) {
          const _ = S ? `Schema ${b} approved successfully. Backfill started with hash: ${S}` : `Schema ${b} approved successfully`;
          t({ success: !0, message: _, backfillHash: S });
        }
        e && e();
      } else {
        console.log("🔴 SchemaTab: approveSchema rejected:", w.payload);
        const S = typeof w.payload == "string" ? w.payload : ((y = w.payload) == null ? void 0 : y.error) || `Failed to approve schema: ${b}`;
        throw new Error(S);
      }
    } catch (w) {
      if (console.error("🔴 SchemaTab: Failed to approve schema:", w), t) {
        const S = w instanceof Error ? w.message : String(w);
        t({ error: `Failed to approve schema: ${S}` });
      }
    }
  }, x = async (b) => {
    var E;
    try {
      const y = await r(je({ schemaName: b }));
      if (je.fulfilled.match(y))
        console.log("🟡 SchemaTab: blockSchema fulfilled, calling callbacks"), console.log("🔄 Refetching schemas from backend after blocking..."), await r(Ie({ forceRefresh: !0 })), console.log("✅ Refetch complete - backend state should be reflected"), t && t({ success: !0, message: `Schema ${b} blocked successfully` }), e && e();
      else {
        const w = typeof y.payload == "string" ? y.payload : ((E = y.payload) == null ? void 0 : E.error) || `Failed to block schema: ${b}`;
        throw new Error(w);
      }
    } catch (y) {
      if (console.error("Failed to block schema:", y), t) {
        const w = y instanceof Error ? y.message : String(y);
        t({ error: `Failed to block schema: ${w}` });
      }
    }
  }, g = (b) => {
    const E = o[b.name], y = b.state || "Unknown", w = b.fields ? xa(b) : null, S = va(b);
    return /* @__PURE__ */ c("div", { className: "bg-white rounded-lg border border-gray-200 shadow-sm overflow-hidden transition-all duration-200 hover:shadow-md", children: [
      /* @__PURE__ */ n(
        "div",
        {
          className: "px-4 py-3 bg-gray-50 cursor-pointer select-none transition-colors duration-200 hover:bg-gray-100",
          onClick: () => l(b.name),
          children: /* @__PURE__ */ c("div", { className: "flex items-center justify-between", children: [
            /* @__PURE__ */ c("div", { className: "flex items-center space-x-2", children: [
              E ? /* @__PURE__ */ n(Un, { className: "icon icon-sm text-gray-400 transition-transform duration-200" }) : /* @__PURE__ */ n($n, { className: "icon icon-sm text-gray-400 transition-transform duration-200" }),
              /* @__PURE__ */ n("h3", { className: "font-medium text-gray-900", children: h(b) }),
              b.descriptive_name && b.descriptive_name !== b.name && /* @__PURE__ */ c("span", { className: "text-xs text-gray-500", children: [
                "(",
                b.name,
                ")"
              ] }),
              /* @__PURE__ */ n("span", { className: `px-2 py-1 text-xs font-medium rounded-full ${f(y)}`, children: y }),
              w && /* @__PURE__ */ n("span", { className: "px-2 py-1 text-xs font-medium rounded-full bg-purple-100 text-purple-800", children: "Range Schema" }),
              S && /* @__PURE__ */ n("span", { className: "px-2 py-1 text-xs font-medium rounded-full bg-blue-100 text-blue-800", children: "HashRange Schema" })
            ] }),
            /* @__PURE__ */ c("div", { className: "flex items-center space-x-2", children: [
              y.toLowerCase() === "available" && /* @__PURE__ */ n(
                "button",
                {
                  className: "group inline-flex items-center px-2 py-1 text-xs font-medium rounded-md text-white bg-green-600 hover:bg-green-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-green-500",
                  onClick: (_) => {
                    console.log("🟠 Button clicked: Approve for schema:", b.name), _.stopPropagation(), p(b.name);
                  },
                  children: "Approve"
                }
              ),
              y.toLowerCase() === "approved" && /* @__PURE__ */ n(
                "button",
                {
                  className: "group inline-flex items-center px-2 py-1 text-xs font-medium rounded-md text-white bg-red-600 hover:bg-red-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-red-500",
                  onClick: (_) => {
                    _.stopPropagation(), x(b.name);
                  },
                  children: "Block"
                }
              ),
              y.toLowerCase() === "blocked" && /* @__PURE__ */ n(
                "button",
                {
                  className: "group inline-flex items-center px-2 py-1 text-xs font-medium rounded-md text-white bg-green-600 hover:bg-green-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-green-500",
                  onClick: (_) => {
                    _.stopPropagation(), p(b.name);
                  },
                  children: "Re-approve"
                }
              )
            ] })
          ] })
        }
      ),
      E && b.fields && /* @__PURE__ */ c("div", { className: "p-4 border-t border-gray-200", children: [
        w && /* @__PURE__ */ c("div", { className: "mb-4 p-3 bg-purple-50 rounded-md border border-purple-200", children: [
          /* @__PURE__ */ n("h4", { className: "text-sm font-medium text-purple-900 mb-2", children: "Range Schema Information" }),
          /* @__PURE__ */ c("div", { className: "space-y-1 text-xs text-purple-800", children: [
            /* @__PURE__ */ c("p", { children: [
              /* @__PURE__ */ n("strong", { children: "Range Key:" }),
              " ",
              w.rangeKey
            ] }),
            /* @__PURE__ */ c("p", { children: [
              /* @__PURE__ */ n("strong", { children: "Total Fields:" }),
              " ",
              w.totalFields
            ] }),
            /* @__PURE__ */ c("p", { children: [
              /* @__PURE__ */ n("strong", { children: "Range Fields:" }),
              " ",
              w.rangeFields.length
            ] }),
            /* @__PURE__ */ n("p", { className: "text-purple-600", children: "This schema uses range-based storage for efficient querying and mutations." })
          ] })
        ] }),
        S && /* @__PURE__ */ c("div", { className: "mb-4 p-3 bg-blue-50 rounded-md border border-blue-200", children: [
          /* @__PURE__ */ n("h4", { className: "text-sm font-medium text-blue-900 mb-2", children: "HashRange Schema Information" }),
          /* @__PURE__ */ c("div", { className: "space-y-1 text-xs text-blue-800", children: [
            /* @__PURE__ */ c("p", { children: [
              /* @__PURE__ */ n("strong", { children: "Hash Field:" }),
              " ",
              S.hashField
            ] }),
            /* @__PURE__ */ c("p", { children: [
              /* @__PURE__ */ n("strong", { children: "Range Field:" }),
              " ",
              S.rangeField
            ] }),
            /* @__PURE__ */ c("p", { children: [
              /* @__PURE__ */ n("strong", { children: "Total Fields:" }),
              " ",
              S.totalFields
            ] }),
            /* @__PURE__ */ n("p", { className: "text-blue-600", children: "This schema uses hash-range-based storage for efficient querying and mutations with both hash and range keys." })
          ] })
        ] }),
        /* @__PURE__ */ n("div", { className: "space-y-3", children: Array.isArray(b.fields) ? b.fields.map((_) => {
          var F;
          const R = (F = b.field_topologies) == null ? void 0 : F[_];
          return /* @__PURE__ */ n("div", { className: "p-3 bg-gray-50 rounded-md border border-gray-200", children: /* @__PURE__ */ n("div", { className: "flex items-center justify-between", children: /* @__PURE__ */ c("div", { className: "flex-1", children: [
            /* @__PURE__ */ c("div", { className: "flex items-center space-x-2", children: [
              /* @__PURE__ */ n("span", { className: "font-medium text-gray-900", children: _ }),
              (w == null ? void 0 : w.rangeKey) === _ && /* @__PURE__ */ n("span", { className: "px-2 py-0.5 text-xs font-medium rounded-full bg-purple-100 text-purple-800", children: "Range Key" }),
              (S == null ? void 0 : S.hashField) === _ && /* @__PURE__ */ n("span", { className: "px-2 py-0.5 text-xs font-medium rounded-full bg-blue-100 text-blue-800", children: "Hash Key" }),
              (S == null ? void 0 : S.rangeField) === _ && /* @__PURE__ */ n("span", { className: "px-2 py-0.5 text-xs font-medium rounded-full bg-purple-100 text-purple-800", children: "Range Key" })
            ] }),
            R && /* @__PURE__ */ n(Bi, { topology: R })
          ] }) }) }, _);
        }) : /* @__PURE__ */ n("p", { className: "text-sm text-gray-500 italic", children: "No fields defined" }) })
      ] })
    ] }, b.name);
  }, N = (b) => typeof b == "string" ? b.toLowerCase() : typeof b == "object" && b !== null ? String(b).toLowerCase() : String(b || "").toLowerCase(), T = i.filter(
    (b) => N(b.state) === "approved"
  );
  return /* @__PURE__ */ n("div", { className: "p-6 space-y-6", children: /* @__PURE__ */ c("div", { className: "space-y-4", children: [
    /* @__PURE__ */ n("h3", { className: "text-lg font-medium text-gray-900", children: "Approved Schemas" }),
    T.length > 0 ? T.map(g) : /* @__PURE__ */ n("div", { className: "border rounded-lg p-8 bg-white shadow-sm text-center text-gray-500", children: "No approved schemas found." })
  ] }) });
}
function Pr() {
  const t = Ge(), e = te(gt), r = te(Lr), [i, o] = B(""), [d, h] = B([]), [l, f] = B({}), [p, x] = B({}), [g, N] = B(""), [T, b] = B(""), [E, y] = B({}), w = Z(() => (e || []).filter((H) => (typeof H.state == "string" ? H.state.toLowerCase() : String(H.state || "").toLowerCase()) === Fe.APPROVED), [e]), S = Z(() => i ? (e || []).find((H) => H.name === i) : null, [i, e]), _ = Z(() => S ? pt(S) : !1, [S]), R = Z(() => S ? Dr(S) : !1, [S]), F = Z(() => S ? et(S) : null, [S]), P = $((H) => {
    if (o(H), H) {
      const G = (e || []).find((ie) => ie.name === H), L = (G == null ? void 0 : G.fields) || (G == null ? void 0 : G.transform_fields) || [], j = Array.isArray(L) ? L : Object.keys(L);
      h(j);
      const J = {};
      j.forEach((ie) => {
        J[ie] = "";
      }), f(J);
    } else
      h([]), f({});
    x({}), N(""), b(""), y({});
  }, [e]), C = $((H) => {
    h((G) => G.includes(H) ? G.filter((L) => L !== H) : [...G, H]), f((G) => G[H] !== void 0 ? G : {
      ...G,
      [H]: ""
      // Initialize with empty string for new fields
    });
  }, []), I = $((H, G, L) => {
    x((j) => ({
      ...j,
      [H]: {
        ...j[H],
        [G]: L
      }
    }));
  }, []), O = $((H, G) => {
    f((L) => ({
      ...L,
      [H]: G
    }));
  }, []), U = $(() => {
    o(""), h([]), f({}), x({}), N(""), b(""), y({});
  }, []), K = $(() => {
    t(Ie({ forceRefresh: !0 }));
  }, [t]);
  return {
    state: {
      selectedSchema: i,
      queryFields: d,
      fieldValues: l,
      rangeFilters: p,
      rangeSchemaFilter: E,
      rangeKeyValue: g,
      hashKeyValue: T
    },
    setSelectedSchema: o,
    setQueryFields: h,
    setFieldValues: f,
    toggleField: C,
    handleFieldValueChange: O,
    setRangeFilters: x,
    setRangeSchemaFilter: y,
    setRangeKeyValue: N,
    setHashKeyValue: b,
    clearState: U,
    handleSchemaChange: P,
    handleRangeFilterChange: I,
    refetchSchemas: K,
    approvedSchemas: w,
    schemasLoading: r,
    selectedSchemaObj: S,
    isRangeSchema: _,
    isHashRangeSchema: R,
    rangeKey: F
  };
}
function Tt(t) {
  return { HashKey: t };
}
function Fi(t) {
  return { RangePrefix: t };
}
function Oi(t, e) {
  return { RangeRange: { start: t, end: e } };
}
function Di(t, e) {
  return { HashRangeKey: { hash: t, range: e } };
}
function Qn({
  schema: t,
  queryState: e,
  schemas: r,
  selectedSchemaObj: i,
  isRangeSchema: o,
  rangeKey: d
}) {
  const h = te(Bt), l = Z(() => i || (r && t && r[t] ? r[t] : h && Array.isArray(h) && h.find((N) => N.name === t) || null), [i, t, r, h]), f = Z(() => typeof o == "boolean" ? o : l ? l.schema_type === "Range" || pt(l) ? !0 : l.fields && typeof l.fields == "object" ? Object.values(l.fields).some((T) => (T == null ? void 0 : T.field_type) === "Range") : !1 : !1, [l, o]), p = Z(() => [], []), x = !0, g = Z(() => {
    var _;
    if (!t || !e || !l)
      return {};
    const {
      queryFields: N = [],
      fieldValues: T = {},
      rangeFilters: b = {},
      rangeSchemaFilter: E = {},
      filters: y = [],
      orderBy: w
    } = e, S = {
      schema_name: t,
      // Backend expects schema_name, not schema
      fields: N
      // Array of selected field names
    };
    if (Dr(l)) {
      const R = e.hashKeyValue, F = (_ = e.rangeSchemaFilter) == null ? void 0 : _.key;
      R && R.trim() ? S.filter = Tt(R.trim()) : F && F.trim() && (S.filter = Tt(F.trim()));
    }
    if (f) {
      const R = E && Object.keys(E).length > 0 ? E : Object.values(b).find((P) => P && typeof P == "object" && (P.key || P.keyPrefix || P.start && P.end)) || {}, F = e == null ? void 0 : e.rangeKeyValue;
      !R.key && !R.keyPrefix && !(R.start && R.end) && F && (R.key = F), R.key ? S.filter = Tt(R.key) : R.keyPrefix ? S.filter = Fi(R.keyPrefix) : R.start && R.end && (S.filter = Oi(R.start, R.end));
    }
    return S;
  }, [t, e, l]);
  return $(() => g, [g]), $(() => ({
    isValid: x,
    errors: p
  }), [x, p]), {
    query: g,
    validationErrors: p,
    isValid: x
  };
}
function ke({
  label: t,
  name: e,
  required: r = !1,
  error: i,
  helpText: o,
  children: d,
  className: h = ""
}) {
  const l = e ? `field-${e}` : `field-${Math.random().toString(36).substr(2, 9)}`, f = !!i;
  return /* @__PURE__ */ c("div", { className: `space-y-2 ${h}`, children: [
    /* @__PURE__ */ c(
      "label",
      {
        htmlFor: l,
        className: "block text-sm font-medium text-gray-700",
        children: [
          t,
          r && /* @__PURE__ */ n("span", { className: "ml-1 text-red-500", "aria-label": "required", children: "*" })
        ]
      }
    ),
    /* @__PURE__ */ n("div", { className: "relative", children: d }),
    f && /* @__PURE__ */ n(
      "p",
      {
        className: "text-sm text-red-600",
        role: "alert",
        "aria-live": "polite",
        children: i
      }
    ),
    o && !f && /* @__PURE__ */ n("p", { className: "text-xs text-gray-500", children: o })
  ] });
}
function Yn(t = []) {
  return t.reduce((e, r) => {
    const i = r.group || "default";
    return e[i] || (e[i] = []), e[i].push(r), e;
  }, {});
}
function Li(t = [], e = "") {
  if (wa(e)) return t;
  const r = e.toLowerCase();
  return t.filter(
    (i) => i.label.toLowerCase().includes(r) || i.value.toLowerCase().includes(r)
  );
}
function Mi(t = {}) {
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
function Pi(t, e = !1, r = !1, i = !1) {
  var d, h;
  let o = ((d = t.select) == null ? void 0 : d.base) || "";
  return e && (o += " border-red-300 focus:ring-red-500 focus:border-red-500"), (r || i) && (o += ` ${((h = t.select) == null ? void 0 : h.disabled) || ""}`), o;
}
function Ui(t, e = !1, r = "") {
  const i = {
    "aria-invalid": e
  };
  return e ? i["aria-describedby"] = `${t}-error` : r && (i["aria-describedby"] = `${t}-help`), i;
}
function $i(t = [], e, r = !0) {
  const [i, o] = B(""), [d, h] = B(!1), l = Li(t, i), f = Yn(l), p = $((S) => {
    o(S.target.value);
  }, []), x = $((S) => {
    S.disabled || (e(S.value), r && (h(!1), o("")));
  }, [e, r]), g = $(() => {
    h(!0);
  }, []), N = $(() => {
    h(!1);
  }, []), T = $(() => {
    h((S) => !S);
  }, []), b = $((S) => {
    const _ = t.find((R) => R.value === S);
    _ && x(_);
  }, [t, x]), E = $(() => {
    o("");
  }, []);
  return {
    state: {
      searchTerm: i,
      isOpen: d,
      filteredOptions: l,
      groupedOptions: f
    },
    actions: {
      setSearchTerm: o,
      openDropdown: g,
      closeDropdown: N,
      toggleDropdown: T,
      selectOption: b,
      clearSearch: E
    },
    handleSearchChange: p,
    handleOptionSelect: x
  };
}
function Wn(t) {
  return `field-${t}`;
}
function Ki(t) {
  return !!t;
}
function Hi({ hasError: t, disabled: e, additionalClasses: r = "" }) {
  const i = Ne.input.base, o = t ? Ne.input.error : Ne.input.success;
  return `${i} ${o} ${e ? "bg-gray-100 cursor-not-allowed" : ""} ${r}`.trim();
}
function ji({ fieldId: t, hasError: e, hasHelp: r }) {
  const i = {
    "aria-invalid": e
  };
  return e ? i["aria-describedby"] = `${t}-error` : r && (i["aria-describedby"] = `${t}-help`), i;
}
function Vi({ size: t = "sm", color: e = "primary" } = {}) {
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
  error: d,
  helpText: h,
  config: l = {},
  className: f = ""
}) {
  const p = Mi(l), { searchable: x, placeholder: g, emptyMessage: N, required: T, disabled: b, loading: E } = p, y = Wn(t), w = !!d, S = i.length > 0, _ = $i(i, o, !0), R = (I) => {
    o(I.target.value);
  };
  if (E)
    return /* @__PURE__ */ n(ke, { label: e, name: t, required: T, error: d, helpText: h, className: f, children: /* @__PURE__ */ c("div", { className: `${Ne.select.disabled} flex items-center`, children: [
      /* @__PURE__ */ n("div", { className: "animate-spin h-4 w-4 border-2 border-gray-400 border-t-transparent rounded-full mr-2" }),
      pa.loading
    ] }) });
  if (!S)
    return /* @__PURE__ */ n(ke, { label: e, name: t, required: T, error: d, helpText: h, className: f, children: /* @__PURE__ */ n("div", { className: Ne.select.disabled, children: N }) });
  if (x) {
    const { state: I, handleSearchChange: O, handleOptionSelect: U } = _;
    return /* @__PURE__ */ n(ke, { label: e, name: t, required: T, error: d, helpText: h, className: f, children: /* @__PURE__ */ c("div", { className: "relative", children: [
      /* @__PURE__ */ n(
        "input",
        {
          type: "text",
          placeholder: `Search ${e.toLowerCase()}...`,
          value: I.searchTerm,
          onChange: O,
          onFocus: () => _.actions.openDropdown(),
          className: `${Ne.input.base} ${w ? Ne.input.error : ""}`
        }
      ),
      I.isOpen && I.filteredOptions.length > 0 && /* @__PURE__ */ n("div", { className: "absolute z-10 w-full mt-1 bg-white border border-gray-300 rounded-md shadow-lg max-h-60 overflow-auto", children: Object.entries(I.groupedOptions).map(([K, V]) => /* @__PURE__ */ c("div", { children: [
        K !== "default" && /* @__PURE__ */ n("div", { className: "px-3 py-2 text-xs font-semibold text-gray-500 bg-gray-50 border-b", children: K }),
        V.map((H) => /* @__PURE__ */ n(
          "button",
          {
            type: "button",
            onClick: () => U(H),
            disabled: H.disabled,
            className: `w-full text-left px-3 py-2 hover:bg-gray-100 focus:bg-gray-100 focus:outline-none ${H.disabled ? "text-gray-400 cursor-not-allowed" : "text-gray-900"} ${r === H.value ? "bg-primary text-white" : ""}`,
            children: H.label
          },
          H.value
        ))
      ] }, K)) })
    ] }) });
  }
  const F = Yn(i), P = Pi(Ne, w, b, E), C = Ui(y, w, h);
  return /* @__PURE__ */ n(ke, { label: e, name: t, required: T, error: d, helpText: h, className: f, children: /* @__PURE__ */ c(
    "select",
    {
      id: y,
      name: t,
      value: r,
      onChange: R,
      required: T,
      disabled: b,
      className: P,
      ...C,
      children: [
        /* @__PURE__ */ n("option", { value: "", disabled: T, children: g }),
        Object.entries(F).map(
          ([I, O]) => I !== "default" ? /* @__PURE__ */ n("optgroup", { label: I, children: O.map((U) => /* @__PURE__ */ n("option", { value: U.value, disabled: U.disabled, children: U.label }, U.value)) }, I) : O.map((U) => /* @__PURE__ */ n("option", { value: U.value, disabled: U.disabled, children: U.label }, U.value))
        )
      ]
    }
  ) });
}
function Et({
  name: t,
  label: e,
  value: r,
  onChange: i,
  required: o = !1,
  disabled: d = !1,
  error: h,
  placeholder: l,
  helpText: f,
  type: p = "text",
  debounced: x = !1,
  debounceMs: g = ma,
  className: N = ""
}) {
  const [T, b] = B(r), [E, y] = B(!1);
  de(() => {
    b(r);
  }, [r]);
  const w = At(null), S = At(null), _ = At(i);
  de(() => {
    _.current = i;
  }, [i]);
  const R = $((U) => {
    y(!0), w.current && (clearTimeout(w.current), w.current = null), S.current && typeof window < "u" && typeof window.cancelAnimationFrame == "function" && (window.cancelAnimationFrame(S.current), S.current = null);
    const K = () => {
      w.current = setTimeout(() => {
        _.current(U), y(!1);
      }, g);
    };
    typeof window < "u" && typeof window.requestAnimationFrame == "function" ? S.current = window.requestAnimationFrame(K) : setTimeout(K, 0);
  }, [g]), F = (U) => {
    const K = U.target.value;
    b(K), x ? R(K) : i(K);
  }, P = Wn(t), C = Ki(h), I = Hi({ hasError: C, disabled: d }), O = ji({
    fieldId: P,
    hasError: C,
    hasHelp: !!f
  });
  return /* @__PURE__ */ n(
    ke,
    {
      label: e,
      name: t,
      required: o,
      error: h,
      helpText: f,
      className: N,
      children: /* @__PURE__ */ c("div", { className: "relative", children: [
        /* @__PURE__ */ n(
          "input",
          {
            id: P,
            name: t,
            type: p,
            value: T,
            onChange: F,
            placeholder: l,
            required: o,
            disabled: d,
            className: I,
            ...O
          }
        ),
        x && E && /* @__PURE__ */ n("div", { className: "absolute right-2 top-1/2 transform -translate-y-1/2", children: /* @__PURE__ */ n(
          "div",
          {
            className: Vi({ size: "md", color: "primary" }),
            role: "status",
            "aria-label": "Processing input"
          }
        ) })
      ] })
    }
  );
}
function mn(t = {}) {
  return t.start || t.end ? "range" : t.key ? "key" : t.keyPrefix ? "prefix" : "range";
}
function Gi(t, e, r) {
  const i = { ...t };
  return e === "range" || r === "start" || r === "end" ? (delete i.key, delete i.keyPrefix) : e === "key" || r === "key" ? (delete i.start, delete i.end, delete i.keyPrefix) : (e === "prefix" || r === "keyPrefix") && (delete i.start, delete i.end, delete i.key), i;
}
function zi(t = {}, e, r = ["range", "key", "prefix"]) {
  const [i, o] = B(
    () => mn(t)
  ), [d, h] = B(t), l = $((E) => {
    if (!r.includes(E)) return;
    o(E);
    const y = {};
    h(y), e && e(y);
  }, [r, e]), f = $((E, y) => {
    const w = Gi(d, i, E);
    w[E] = y, h(w), e && e(w);
  }, [d, i, e]), p = $(() => {
    const E = {};
    h(E), e && e(E);
  }, [e]), x = $((E) => {
    h(E);
    const y = mn(E);
    o(y), e && e(E);
  }, [e]), g = $(() => r, [r]), N = $((E) => r.includes(E), [r]);
  return {
    state: {
      activeMode: i,
      value: d
    },
    actions: {
      changeMode: l,
      updateValue: f,
      clearValue: p,
      setValue: x
    },
    getAvailableModes: g,
    isValidMode: N
  };
}
function qi(t = "all", e = "key", r = "") {
  if (r) return r;
  if (t !== "all") return null;
  const i = { ...Rn.rangeKeyFilter }, o = i.keyRange || "", d = (i.exactKey || "").replace("key", e), h = (i.keyPrefix || "").replace("keys", `${e} values`), l = i.emptyNote || "";
  return `${o} ${d} ${h} ${l}`.trim();
}
function Qi(t = "all") {
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
function Yi(t = !1) {
  const e = "px-3 py-1 text-xs rounded-md transition-colors duration-200";
  return t ? `${e} bg-primary text-white` : `${e} bg-gray-200 text-gray-700 hover:bg-gray-300`;
}
function Wi() {
  return {
    range: "Key Range",
    key: "Exact Key",
    prefix: "Key Prefix"
  };
}
function Ji(t, e) {
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
function Zi(t = {}) {
  const {
    mode: e = "all",
    rangeKeyName: r = "key",
    required: i = !1,
    disabled: o = !1,
    className: d = ""
  } = t;
  return {
    mode: ["all", "range", "key", "prefix"].includes(e) ? e : "all",
    rangeKeyName: String(r),
    required: !!i,
    disabled: !!o,
    className: String(d)
  };
}
function Xi() {
  return "bg-yellow-50 rounded-lg p-4 space-y-4";
}
function eo() {
  return "text-sm font-medium text-gray-800";
}
function to() {
  return "flex space-x-4 mb-4";
}
function ro() {
  return "grid grid-cols-1 md:grid-cols-3 gap-4";
}
function no({
  name: t,
  label: e,
  value: r = {},
  onChange: i,
  error: o,
  helpText: d,
  config: h = {},
  className: l = ""
}) {
  const f = Zi(h), { mode: p, rangeKeyName: x, required: g, disabled: N } = f, T = Qi(p), b = zi(r, i, T.availableModes), { state: E, actions: y } = b, w = Wi(), S = Ji(p, E.activeMode), _ = qi(p, x, d);
  return /* @__PURE__ */ n(
    ke,
    {
      label: e,
      name: t,
      required: g,
      error: o,
      helpText: _,
      className: l,
      children: /* @__PURE__ */ c("div", { className: Xi(), children: [
        /* @__PURE__ */ n("div", { className: "mb-3", children: /* @__PURE__ */ c("span", { className: eo(), children: [
          "Range Key: ",
          x
        ] }) }),
        T.showModeSelector && /* @__PURE__ */ n("div", { className: to(), children: T.availableModes.map((R) => /* @__PURE__ */ n(
          "button",
          {
            type: "button",
            onClick: () => y.changeMode(R),
            className: Yi(E.activeMode === R),
            children: w[R]
          },
          R
        )) }),
        /* @__PURE__ */ c("div", { className: ro(), children: [
          S.showRange && /* @__PURE__ */ c(Rt, { children: [
            /* @__PURE__ */ n(
              Et,
              {
                name: `${t}-start`,
                label: "Start Key",
                value: E.value.start || "",
                onChange: (R) => y.updateValue("start", R),
                placeholder: "Start key",
                disabled: N,
                className: "col-span-1"
              }
            ),
            /* @__PURE__ */ n(
              Et,
              {
                name: `${t}-end`,
                label: "End Key",
                value: E.value.end || "",
                onChange: (R) => y.updateValue("end", R),
                placeholder: "End key",
                disabled: N,
                className: "col-span-1"
              }
            )
          ] }),
          S.showKey && /* @__PURE__ */ n(
            Et,
            {
              name: `${t}-key`,
              label: "Exact Key",
              value: E.value.key || "",
              onChange: (R) => y.updateValue("key", R),
              placeholder: `Exact ${x} to match`,
              disabled: N,
              className: "col-span-1"
            }
          ),
          S.showPrefix && /* @__PURE__ */ n(
            Et,
            {
              name: `${t}-prefix`,
              label: "Key Prefix",
              value: E.value.keyPrefix || "",
              onChange: (R) => y.updateValue("keyPrefix", R),
              placeholder: `${x} prefix (e.g., 'user:')`,
              disabled: N,
              className: "col-span-1"
            }
          )
        ] })
      ] })
    }
  );
}
function so({
  queryState: t,
  onSchemaChange: e,
  onFieldToggle: r,
  onFieldValueChange: i,
  onRangeFilterChange: o,
  onRangeSchemaFilterChange: d,
  onHashKeyChange: h,
  approvedSchemas: l,
  schemasLoading: f,
  isRangeSchema: p,
  isHashRangeSchema: x,
  rangeKey: g,
  className: N = ""
}) {
  const [T, b] = B({}), { clearQuery: E } = Pr();
  $(() => (b({}), !0), []);
  const y = $((F) => {
    e(F), E && E(), b((P) => {
      const { schema: C, ...I } = P;
      return I;
    });
  }, [e, E]), w = $((F) => {
    r(F), b((P) => {
      const { fields: C, ...I } = P;
      return I;
    });
  }, [r]), S = t != null && t.selectedSchema && l ? l.find((F) => F.name === t.selectedSchema) : null, _ = (S == null ? void 0 : S.fields) || (S == null ? void 0 : S.transform_fields) || [], R = Array.isArray(_) ? _ : Object.keys(_);
  return /* @__PURE__ */ c("div", { className: `space-y-6 ${N}`, children: [
    /* @__PURE__ */ n(
      ke,
      {
        label: Ye.schema,
        name: "schema",
        required: !0,
        error: T.schema,
        helpText: Ye.schemaHelp,
        children: /* @__PURE__ */ n(
          wr,
          {
            name: "schema",
            value: (t == null ? void 0 : t.selectedSchema) || "",
            onChange: y,
            options: l.map((F) => ({
              value: F.name,
              label: F.descriptive_name || F.name
            })),
            placeholder: "Select a schema...",
            emptyMessage: Ye.schemaEmpty,
            loading: f
          }
        )
      }
    ),
    (t == null ? void 0 : t.selectedSchema) && R.length > 0 && /* @__PURE__ */ n(
      ke,
      {
        label: "Field Selection",
        name: "fields",
        required: !0,
        error: T.fields,
        helpText: "Select fields to include in your query",
        children: /* @__PURE__ */ n("div", { className: "bg-gray-50 rounded-md p-4", children: /* @__PURE__ */ n("div", { className: "space-y-3", children: R.map((F) => {
          var P;
          return /* @__PURE__ */ c("label", { className: "relative flex items-start", children: [
            /* @__PURE__ */ n("div", { className: "flex items-center h-5", children: /* @__PURE__ */ n(
              "input",
              {
                type: "checkbox",
                className: "h-4 w-4 text-primary border-gray-300 rounded focus:ring-primary",
                checked: ((P = t == null ? void 0 : t.queryFields) == null ? void 0 : P.includes(F)) || !1,
                onChange: () => w(F)
              }
            ) }),
            /* @__PURE__ */ n("div", { className: "ml-3 flex items-center", children: /* @__PURE__ */ n("span", { className: "text-sm font-medium text-gray-700", children: F }) })
          ] }, F);
        }) }) })
      }
    ),
    x && /* @__PURE__ */ n(
      ke,
      {
        label: "HashRange Filter",
        name: "hashRangeFilter",
        helpText: "Filter data by hash and range key values",
        children: /* @__PURE__ */ c("div", { className: "bg-purple-50 rounded-md p-4 space-y-4", children: [
          /* @__PURE__ */ c("div", { className: "grid grid-cols-1 md:grid-cols-2 gap-4", children: [
            /* @__PURE__ */ c("div", { className: "space-y-2", children: [
              /* @__PURE__ */ n("label", { className: "block text-sm font-medium text-gray-700", children: "Hash Key" }),
              /* @__PURE__ */ n(
                "input",
                {
                  type: "text",
                  placeholder: "Enter hash key value",
                  className: "w-full px-3 py-2 text-sm border border-gray-300 rounded-md focus:outline-none focus:ring-1 focus:ring-primary focus:border-primary",
                  value: (t == null ? void 0 : t.hashKeyValue) || "",
                  onChange: (F) => h(F.target.value)
                }
              ),
              /* @__PURE__ */ c("div", { className: "text-xs text-gray-500", children: [
                "Hash field: ",
                Bn(l.find((F) => F.name === (t == null ? void 0 : t.selectedSchema))) || "N/A"
              ] })
            ] }),
            /* @__PURE__ */ c("div", { className: "space-y-2", children: [
              /* @__PURE__ */ n("label", { className: "block text-sm font-medium text-gray-700", children: "Range Key" }),
              /* @__PURE__ */ n(
                "input",
                {
                  type: "text",
                  placeholder: "Enter range key value",
                  className: "w-full px-3 py-2 text-sm border border-gray-300 rounded-md focus:outline-none focus:ring-1 focus:ring-primary focus:border-primary",
                  value: (t == null ? void 0 : t.rangeKeyValue) || "",
                  onChange: (F) => d({ key: F.target.value })
                }
              ),
              /* @__PURE__ */ c("div", { className: "text-xs text-gray-500", children: [
                "Range field: ",
                et(l.find((F) => F.name === (t == null ? void 0 : t.selectedSchema))) || "N/A"
              ] })
            ] })
          ] }),
          /* @__PURE__ */ c("div", { className: "text-xs text-gray-500", children: [
            /* @__PURE__ */ c("p", { children: [
              /* @__PURE__ */ n("strong", { children: "Hash Key:" }),
              " Used for partitioning data across multiple nodes"
            ] }),
            /* @__PURE__ */ c("p", { children: [
              /* @__PURE__ */ n("strong", { children: "Range Key:" }),
              " Used for ordering and range queries within a partition"
            ] })
          ] })
        ] })
      }
    ),
    p && g && /* @__PURE__ */ n(
      ke,
      {
        label: "Range Filter",
        name: "rangeSchemaFilter",
        error: T.rangeFilter,
        helpText: "Filter data by range key values",
        children: /* @__PURE__ */ n(
          no,
          {
            name: "rangeSchemaFilter",
            value: (t == null ? void 0 : t.rangeSchemaFilter) || {},
            onChange: (F) => {
              d(F), b((P) => {
                const { rangeFilter: C, ...I } = P;
                return I;
              });
            },
            rangeKeyName: g,
            mode: "all"
          }
        )
      }
    )
  ] });
}
function ao({
  onExecute: t,
  onExecuteQuery: e,
  onValidate: r,
  onSave: i,
  onSaveQuery: o,
  onClear: d,
  onClearQuery: h,
  disabled: l = !1,
  isExecuting: f = !1,
  isSaving: p = !1,
  showValidation: x = !1,
  showSave: g = !0,
  showClear: N = !0,
  className: T = "",
  queryData: b
}) {
  const [E, y] = B(null), [w, S] = B(null), { clearQuery: _ } = Pr(), R = async (O, U, K = null) => {
    if (!(!U || l))
      try {
        y(O), await U(K);
      } catch (V) {
        console.error(`${O} action failed:`, V);
      } finally {
        y(null), S(null);
      }
  }, F = () => {
    R("execute", e || t, b);
  }, P = () => {
    R("validate", r, b);
  }, C = () => {
    R("save", o || i, b);
  }, I = () => {
    const O = h || d;
    O && O(), _ && _();
  };
  return /* @__PURE__ */ c("div", { className: `flex justify-end space-x-3 ${T}`, children: [
    N && /* @__PURE__ */ n(
      "button",
      {
        type: "button",
        onClick: I,
        disabled: l,
        className: `
            inline-flex items-center px-4 py-2 border border-gray-300 rounded-md shadow-sm text-sm font-medium
            ${l ? "bg-gray-100 text-gray-400 cursor-not-allowed" : "bg-white text-gray-700 hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-primary"}
          `,
        children: Ut.clearQuery || "Clear Query"
      }
    ),
    x && r && /* @__PURE__ */ c(
      "button",
      {
        type: "button",
        onClick: P,
        disabled: l,
        className: `
            inline-flex items-center px-4 py-2 border border-transparent rounded-md shadow-sm text-sm font-medium
            ${l ? "bg-gray-300 text-gray-500 cursor-not-allowed" : "bg-blue-600 text-white hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500"}
          `,
        children: [
          E === "validate" && /* @__PURE__ */ c("svg", { className: "animate-spin -ml-1 mr-2 h-4 w-4 text-white", xmlns: "http://www.w3.org/2000/svg", fill: "none", viewBox: "0 0 24 24", children: [
            /* @__PURE__ */ n("circle", { className: "opacity-25", cx: "12", cy: "12", r: "10", stroke: "currentColor", strokeWidth: "4" }),
            /* @__PURE__ */ n("path", { className: "opacity-75", fill: "currentColor", d: "M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" })
          ] }),
          Ut.validateQuery || "Validate"
        ]
      }
    ),
    g && (i || o) && /* @__PURE__ */ c(
      "button",
      {
        type: "button",
        onClick: C,
        disabled: l || p,
        className: `
            inline-flex items-center px-4 py-2 border border-transparent rounded-md shadow-sm text-sm font-medium
            ${l || p ? "bg-gray-300 text-gray-500 cursor-not-allowed" : "bg-green-600 text-white hover:bg-green-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-green-500"}
          `,
        children: [
          (E === "save" || p) && /* @__PURE__ */ c("svg", { className: "animate-spin -ml-1 mr-2 h-4 w-4 text-white", xmlns: "http://www.w3.org/2000/svg", fill: "none", viewBox: "0 0 24 24", children: [
            /* @__PURE__ */ n("circle", { className: "opacity-25", cx: "12", cy: "12", r: "10", stroke: "currentColor", strokeWidth: "4" }),
            /* @__PURE__ */ n("path", { className: "opacity-75", fill: "currentColor", d: "M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" })
          ] }),
          Ut.saveQuery || "Save Query"
        ]
      }
    ),
    /* @__PURE__ */ c(
      "button",
      {
        type: "button",
        onClick: F,
        disabled: l || f,
        className: `
          inline-flex items-center px-4 py-2 border border-transparent rounded-md shadow-sm text-sm font-medium text-white
          ${l || f ? "bg-gray-300 cursor-not-allowed" : "bg-primary hover:bg-primary/90 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-primary"}
        `,
        children: [
          (E === "execute" || f) && /* @__PURE__ */ c("svg", { className: "animate-spin -ml-1 mr-2 h-4 w-4 text-white", xmlns: "http://www.w3.org/2000/svg", fill: "none", viewBox: "0 0 24 24", children: [
            /* @__PURE__ */ n("circle", { className: "opacity-25", cx: "12", cy: "12", r: "10", stroke: "currentColor", strokeWidth: "4" }),
            /* @__PURE__ */ n("path", { className: "opacity-75", fill: "currentColor", d: "M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 714 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" })
          ] }),
          E === "execute" || f ? "Executing..." : Ut.executeQuery
        ]
      }
    )
  ] });
}
const io = (t, e) => {
  if (!t && !e) return null;
  const r = { ...t, ...e };
  let i = [], o = {};
  Array.isArray(r.fields) ? i = r.fields : r.fields && typeof r.fields == "object" ? (i = Object.keys(r.fields), o = r.fields) : r.queryFields && Array.isArray(r.queryFields) && (i = r.queryFields), r.fieldValues && typeof r.fieldValues == "object" && (o = { ...o, ...r.fieldValues });
  const d = {
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
      l.Key ? d.filters[h] = { exactKey: l.Key } : l.KeyRange ? d.filters[h] = {
        keyRange: `${l.KeyRange.start} → ${l.KeyRange.end}`
      } : l.KeyPrefix && (d.filters[h] = { keyPrefix: l.KeyPrefix });
    } else t.filter.range_filter && Object.entries(t.filter.range_filter).forEach(([h, l]) => {
      typeof l == "string" ? d.filters[h] = { exactKey: l } : l.KeyRange ? d.filters[h] = {
        keyRange: `${l.KeyRange.start} → ${l.KeyRange.end}`
      } : l.KeyPrefix && (d.filters[h] = { keyPrefix: l.KeyPrefix });
    });
  return d;
};
function oo({
  query: t,
  queryState: e,
  validationErrors: r = [],
  isExecuting: i = !1,
  showJson: o = !1,
  collapsible: d = !0,
  className: h = "",
  title: l = "Query Preview"
}) {
  const f = Z(() => io(t, e), [t, e]);
  return !t && !e ? /* @__PURE__ */ c("div", { className: `bg-gray-50 rounded-md p-4 ${h}`, children: [
    /* @__PURE__ */ n("h3", { className: "text-sm font-medium text-gray-500 mb-2", children: l }),
    /* @__PURE__ */ n("p", { className: "text-sm text-gray-400 italic", children: "No query to preview" })
  ] }) : /* @__PURE__ */ c("div", { className: `bg-white border border-gray-200 rounded-lg shadow-sm ${h}`, children: [
    /* @__PURE__ */ n("div", { className: "px-4 py-3 border-b border-gray-200", children: /* @__PURE__ */ n("h3", { className: "text-sm font-medium text-gray-900", children: l }) }),
    /* @__PURE__ */ c("div", { className: "p-4 space-y-4", children: [
      r && r.length > 0 && /* @__PURE__ */ c("div", { className: "bg-red-50 border border-red-200 rounded-md p-3", children: [
        /* @__PURE__ */ c("div", { className: "flex items-center mb-2", children: [
          /* @__PURE__ */ n("svg", { className: "h-4 w-4 text-red-400 mr-2", fill: "currentColor", viewBox: "0 0 20 20", children: /* @__PURE__ */ n("path", { fillRule: "evenodd", d: "M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7 4a1 1 0 11-2 0 1 1 0 012 0zm-1-9a1 1 0 00-1 1v4a1 1 0 102 0V6a1 1 0 00-1-1z", clipRule: "evenodd" }) }),
          /* @__PURE__ */ n("span", { className: "text-sm font-medium text-red-800", children: "Validation Errors" })
        ] }),
        /* @__PURE__ */ n("ul", { className: "space-y-1", children: r.map((p, x) => /* @__PURE__ */ n("li", { className: "text-sm text-red-700", children: p }, x)) })
      ] }),
      i && /* @__PURE__ */ n("div", { className: "bg-blue-50 border border-blue-200 rounded-md p-3", children: /* @__PURE__ */ c("div", { className: "flex items-center", children: [
        /* @__PURE__ */ c("svg", { className: "animate-spin h-4 w-4 text-blue-400 mr-2", fill: "none", viewBox: "0 0 24 24", children: [
          /* @__PURE__ */ n("circle", { className: "opacity-25", cx: "12", cy: "12", r: "10", stroke: "currentColor", strokeWidth: "4" }),
          /* @__PURE__ */ n("path", { className: "opacity-75", fill: "currentColor", d: "M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" })
        ] }),
        /* @__PURE__ */ n("span", { className: "text-sm font-medium text-blue-800", children: "Executing query..." })
      ] }) }),
      /* @__PURE__ */ c("div", { className: "space-y-3", children: [
        /* @__PURE__ */ c("div", { children: [
          /* @__PURE__ */ n("label", { className: "block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1", children: "Schema" }),
          /* @__PURE__ */ n("div", { className: "inline-flex items-center px-2 py-1 rounded-md bg-blue-100 text-blue-800 text-sm font-medium", children: (f == null ? void 0 : f.schema) || "" })
        ] }),
        /* @__PURE__ */ c("div", { children: [
          /* @__PURE__ */ c("label", { className: "block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1", children: [
            "Fields (",
            f != null && f.fields ? f.fields.length : 0,
            ")"
          ] }),
          /* @__PURE__ */ n("div", { className: "flex flex-wrap gap-1", children: f != null && f.fields && f.fields.length > 0 ? f.fields.map((p, x) => {
            var N;
            const g = (N = f.fieldValues) == null ? void 0 : N[p];
            return /* @__PURE__ */ c("div", { className: "inline-flex flex-col items-start", children: [
              /* @__PURE__ */ n("span", { className: "inline-flex items-center px-2 py-1 rounded-md bg-green-100 text-green-800 text-sm", children: p }),
              g && /* @__PURE__ */ n("span", { className: "text-xs text-gray-600 mt-1 px-2", children: g })
            ] }, x);
          }) : /* @__PURE__ */ n("span", { className: "text-sm text-gray-500 italic", children: "No fields selected" }) })
        ] }),
        (f.filters && Array.isArray(f.filters) && f.filters.length > 0 || f.filters && !Array.isArray(f.filters) && Object.keys(f.filters).length > 0) && /* @__PURE__ */ c("div", { children: [
          /* @__PURE__ */ n("label", { className: "block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1", children: "Filters" }),
          /* @__PURE__ */ n("div", { className: "space-y-2", children: Array.isArray(f.filters) ? (
            // Handle filters as array (from test mocks)
            f.filters.map((p, x) => /* @__PURE__ */ n("div", { className: "bg-yellow-50 rounded-md p-3", children: /* @__PURE__ */ c("div", { className: "text-sm text-yellow-700", children: [
              p.field,
              " ",
              p.operator,
              ' "',
              p.value,
              '"'
            ] }) }, x))
          ) : (
            // Handle filters as object (existing format)
            Object.entries(f.filters).map(([p, x]) => /* @__PURE__ */ c("div", { className: "bg-yellow-50 rounded-md p-3", children: [
              /* @__PURE__ */ n("div", { className: "font-medium text-sm text-yellow-800 mb-1", children: p }),
              /* @__PURE__ */ c("div", { className: "text-sm text-yellow-700", children: [
                x.exactKey && /* @__PURE__ */ c("span", { children: [
                  "Exact key: ",
                  /* @__PURE__ */ n("code", { className: "bg-yellow-200 px-1 rounded", children: x.exactKey })
                ] }),
                x.keyRange && /* @__PURE__ */ c("span", { children: [
                  "Key range: ",
                  /* @__PURE__ */ n("code", { className: "bg-yellow-200 px-1 rounded", children: x.keyRange })
                ] }),
                x.keyPrefix && /* @__PURE__ */ c("span", { children: [
                  "Key prefix: ",
                  /* @__PURE__ */ n("code", { className: "bg-yellow-200 px-1 rounded", children: x.keyPrefix })
                ] })
              ] })
            ] }, p))
          ) })
        ] }),
        f.orderBy && /* @__PURE__ */ c("div", { children: [
          /* @__PURE__ */ n("label", { className: "block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1", children: "OrderBy" }),
          /* @__PURE__ */ n("div", { className: "bg-purple-50 rounded-md p-3", children: /* @__PURE__ */ c("div", { className: "text-sm text-purple-700", children: [
            f.orderBy.field,
            " ",
            f.orderBy.direction
          ] }) })
        ] }),
        f.rangeKey && /* @__PURE__ */ c("div", { children: [
          /* @__PURE__ */ n("label", { className: "block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1", children: "RangeKey" }),
          /* @__PURE__ */ n("div", { className: "bg-indigo-50 rounded-md p-3", children: /* @__PURE__ */ n("div", { className: "text-sm text-indigo-700", children: /* @__PURE__ */ n("code", { className: "bg-indigo-200 px-1 rounded", children: f.rangeKey }) }) })
        ] })
      ] }),
      o && /* @__PURE__ */ c("div", { className: "border-t border-gray-200 pt-4", children: [
        /* @__PURE__ */ n("label", { className: "block text-xs font-medium text-gray-500 uppercase tracking-wide mb-2", children: "Raw JSON" }),
        /* @__PURE__ */ n("pre", { className: "bg-gray-900 text-gray-100 text-xs p-3 rounded-md overflow-x-auto", children: JSON.stringify(t, null, 2) })
      ] })
    ] })
  ] });
}
function al({ onResult: t }) {
  const {
    state: e,
    handleSchemaChange: r,
    toggleField: i,
    handleFieldValueChange: o,
    handleRangeFilterChange: d,
    setRangeSchemaFilter: h,
    setHashKeyValue: l,
    clearState: f,
    refetchSchemas: p,
    approvedSchemas: x,
    schemasLoading: g,
    selectedSchemaObj: N,
    isRangeSchema: T,
    isHashRangeSchema: b,
    rangeKey: E
  } = Pr();
  de(() => {
    p();
  }, [p]);
  const [y, w] = B(!1), { query: S, isValid: _ } = Qn({
    schema: e.selectedSchema,
    queryState: e,
    schemas: { [e.selectedSchema]: N }
  }), R = $(async (C) => {
    if (!C) {
      t({
        error: "No query data provided"
      });
      return;
    }
    w(!0);
    try {
      const I = await Mr.executeQuery(C);
      if (!I.success) {
        console.error("Query failed:", I.error), t({
          error: I.error || "Query execution failed",
          details: I
        });
        return;
      }
      t({
        success: !0,
        data: I.data
        // The actual query results are directly in response.data
      });
    } catch (I) {
      console.error("Failed to execute query:", I), t({
        error: `Network error: ${I.message}`,
        details: I
      });
    } finally {
      w(!1);
    }
  }, [t, _]), F = $(async (C) => {
    console.log("Validating query:", C);
  }, []), P = $(async (C) => {
    if (!C || !_) {
      console.warn("Cannot save invalid query");
      return;
    }
    try {
      console.log("Saving query:", C);
      const I = JSON.parse(localStorage.getItem("savedQueries") || "[]"), O = {
        id: Date.now(),
        name: `Query ${I.length + 1}`,
        data: C,
        createdAt: (/* @__PURE__ */ new Date()).toISOString()
      };
      I.push(O), localStorage.setItem("savedQueries", JSON.stringify(I)), console.log("Query saved successfully");
    } catch (I) {
      console.error("Failed to save query:", I);
    }
  }, [_]);
  return /* @__PURE__ */ n("div", { className: "p-6", children: /* @__PURE__ */ c("div", { className: "grid grid-cols-1 lg:grid-cols-3 gap-6", children: [
    /* @__PURE__ */ c("div", { className: "lg:col-span-2 space-y-6", children: [
      /* @__PURE__ */ n(
        so,
        {
          queryState: e,
          onSchemaChange: r,
          onFieldToggle: i,
          onFieldValueChange: o,
          onRangeFilterChange: d,
          onRangeSchemaFilterChange: h,
          onHashKeyChange: l,
          approvedSchemas: x,
          schemasLoading: g,
          isRangeSchema: T,
          isHashRangeSchema: b,
          rangeKey: E
        }
      ),
      /* @__PURE__ */ n(
        ao,
        {
          onExecute: () => R(S),
          onValidate: () => F(S),
          onSave: () => P(S),
          onClear: f,
          queryData: S,
          disabled: !_,
          isExecuting: y,
          showValidation: !1,
          showSave: !0,
          showClear: !0
        }
      )
    ] }),
    /* @__PURE__ */ n("div", { className: "lg:col-span-1", children: /* @__PURE__ */ n(
      oo,
      {
        query: S,
        showJson: !1,
        title: "Query Preview"
      }
    ) })
  ] }) });
}
function il({ onResult: t }) {
  const e = Ge(), r = te(Pa), i = te(Ua), o = te($a), d = te(Ka), h = te(Ha), l = te(ja), f = At(null);
  de(() => {
    var N;
    (N = f.current) == null || N.scrollIntoView({ behavior: "smooth" });
  }, [d]);
  const p = $((N, T, b = null) => {
    e(Oa({ type: N, content: T, data: b }));
  }, [e]), x = $(async (N) => {
    if (N == null || N.preventDefault(), !r.trim() || o)
      return;
    const T = r.trim();
    e(rn("")), e(sn(!0)), p("user", T);
    try {
      if (l) {
        p("system", "🤔 Analyzing if question can be answered from existing context...");
        const b = await ln.analyzeFollowup({
          session_id: i,
          question: T
        });
        if (!b.success) {
          p("system", `❌ Error: ${b.error || "Failed to analyze question"}`);
          return;
        }
        const E = b.data;
        if (E.needs_query) {
          p("system", `🔍 Need new data: ${E.reasoning}`), p("system", "🔍 Using AI-native index search...");
          const y = await fetch("/api/llm-query/native-index", {
            method: "POST",
            headers: {
              "Content-Type": "application/json"
            },
            body: JSON.stringify({
              query: T,
              session_id: i
            })
          });
          if (!y.ok) {
            const S = await y.json();
            p("system", `❌ Error: ${S.error || "Failed to run AI-native index query"}`);
            return;
          }
          const w = await y.json();
          p("system", "✅ AI-native index search completed"), w.session_id && e(nn(w.session_id)), p("system", w.ai_interpretation), p("results", "Raw search results", w.raw_results), h && t({ success: !0, data: w.raw_results });
        } else {
          p("system", `✅ Answering from existing context: ${E.reasoning}`);
          const y = await ln.chat({
            session_id: i,
            question: T
          });
          if (!y.success) {
            p("system", `❌ Error: ${y.error || "Failed to process question"}`);
            return;
          }
          p("system", y.data.answer);
        }
      } else {
        p("system", "🔍 Using AI-native index search...");
        const b = await fetch("/api/llm-query/native-index", {
          method: "POST",
          headers: {
            "Content-Type": "application/json"
          },
          body: JSON.stringify({
            query: T,
            session_id: i
          })
        });
        if (!b.ok) {
          const y = await b.json();
          p("system", `❌ Error: ${y.error || "Failed to run AI-native index query"}`);
          return;
        }
        const E = await b.json();
        p("system", "✅ AI-native index search completed"), E.session_id && e(nn(E.session_id)), p("system", E.ai_interpretation), p("results", "Raw search results", E.raw_results), h && t({ success: !0, data: E.raw_results });
      }
    } catch (b) {
      console.error("Error processing input:", b), p("system", `❌ Error: ${b.message}`), t({ error: b.message });
    } finally {
      e(sn(!1));
    }
  }, [r, i, l, o, p, t, e]), g = $(() => {
    e(La());
  }, [e]);
  return /* @__PURE__ */ c("div", { className: "flex flex-col bg-white rounded-lg shadow", children: [
    /* @__PURE__ */ c("div", { className: "p-4 border-b border-gray-200 flex justify-between items-center", children: [
      /* @__PURE__ */ c("div", { children: [
        /* @__PURE__ */ n("h2", { className: "text-xl font-bold text-gray-900", children: "🤖 AI Data Assistant" }),
        /* @__PURE__ */ n("p", { className: "text-sm text-gray-600", children: "Ask questions in plain English - I'll find your data" })
      ] }),
      d.length > 0 && /* @__PURE__ */ n(
        "button",
        {
          onClick: g,
          disabled: o,
          className: "px-4 py-2 bg-gray-600 text-white rounded-lg hover:bg-gray-700 disabled:bg-gray-300 disabled:cursor-not-allowed transition-colors text-sm",
          children: "New Conversation"
        }
      )
    ] }),
    /* @__PURE__ */ c("div", { className: "overflow-y-auto bg-gray-50 p-4 space-y-3", style: { maxHeight: "60vh", minHeight: "400px" }, children: [
      d.length === 0 ? /* @__PURE__ */ c("div", { className: "text-center text-gray-500 mt-20", children: [
        /* @__PURE__ */ n("div", { className: "text-6xl mb-4", children: "💬" }),
        /* @__PURE__ */ n("p", { className: "text-lg mb-2", children: "Start a conversation" }),
        /* @__PURE__ */ n("p", { className: "text-sm", children: 'Try: "Find all blog posts from last month" or "Show me products over $100"' })
      ] }) : d.map((N, T) => /* @__PURE__ */ c("div", { children: [
        N.type === "user" && /* @__PURE__ */ n("div", { className: "flex justify-end", children: /* @__PURE__ */ c("div", { className: "bg-blue-600 text-white rounded-lg px-4 py-2 max-w-3xl", children: [
          /* @__PURE__ */ n("p", { className: "text-sm font-semibold mb-1", children: "You" }),
          /* @__PURE__ */ n("p", { className: "whitespace-pre-wrap", children: N.content })
        ] }) }),
        N.type === "system" && /* @__PURE__ */ n("div", { className: "flex justify-start", children: /* @__PURE__ */ c("div", { className: "bg-white border border-gray-200 rounded-lg px-4 py-2 max-w-3xl", children: [
          /* @__PURE__ */ n("p", { className: "text-sm font-semibold text-gray-700 mb-1", children: "AI Assistant" }),
          /* @__PURE__ */ n("p", { className: "text-gray-900 whitespace-pre-wrap", children: N.content })
        ] }) }),
        N.type === "results" && N.data && /* @__PURE__ */ c("div", { className: "bg-green-50 border border-green-200 rounded-lg p-4 max-w-full", children: [
          /* @__PURE__ */ c("div", { className: "flex justify-between items-center mb-2", children: [
            /* @__PURE__ */ c("p", { className: "text-sm font-semibold text-green-800", children: [
              "📊 Results (",
              N.data.length,
              ")"
            ] }),
            /* @__PURE__ */ n(
              "button",
              {
                onClick: () => {
                  const b = !h;
                  if (e(Da(b)), b) {
                    const E = d.find((y) => y.type === "results");
                    E && E.data && t({ success: !0, data: E.data });
                  } else
                    t(null);
                },
                className: "text-sm text-green-700 hover:text-green-900 underline",
                children: h ? "Hide Details" : "Show Details"
              }
            )
          ] }),
          h && /* @__PURE__ */ c(Rt, { children: [
            /* @__PURE__ */ n("div", { className: "bg-white rounded p-3 mb-2", children: /* @__PURE__ */ n("p", { className: "text-gray-900 whitespace-pre-wrap mb-3", children: N.content }) }),
            /* @__PURE__ */ c("details", { className: "mt-2", children: [
              /* @__PURE__ */ c("summary", { className: "cursor-pointer text-sm text-green-700 hover:text-green-900", children: [
                "View raw data (",
                N.data.length,
                " records)"
              ] }),
              /* @__PURE__ */ n("div", { className: "mt-2 max-h-64 overflow-auto", children: /* @__PURE__ */ n("pre", { className: "text-xs bg-gray-900 text-green-400 p-3 rounded", children: JSON.stringify(N.data, null, 2) }) })
            ] })
          ] })
        ] })
      ] }, T)),
      /* @__PURE__ */ n("div", { ref: f })
    ] }),
    /* @__PURE__ */ c("form", { onSubmit: x, className: "border-t border-gray-200 p-4 bg-white", children: [
      /* @__PURE__ */ c("div", { className: "flex gap-2", children: [
        /* @__PURE__ */ n(
          "input",
          {
            type: "text",
            value: r,
            onChange: (N) => e(rn(N.target.value)),
            placeholder: d.some((N) => N.type === "results") ? "Ask a follow-up question or start a new query..." : "Search the native index (e.g., 'Find posts about AI')...",
            disabled: o,
            className: "flex-1 px-4 py-3 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent disabled:bg-gray-100",
            autoFocus: !0
          }
        ),
        /* @__PURE__ */ n(
          "button",
          {
            type: "submit",
            disabled: !r.trim() || o,
            className: "px-6 py-3 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:bg-gray-300 disabled:cursor-not-allowed transition-colors font-semibold",
            children: o ? "⏳ Processing..." : "Send"
          }
        )
      ] }),
      o && /* @__PURE__ */ n("p", { className: "text-center text-sm text-gray-500 mt-2", children: "AI is analyzing and searching..." })
    ] })
  ] });
}
function lo({ selectedSchema: t, mutationType: e, onSchemaChange: r, onTypeChange: i }) {
  const o = te(Bt);
  return /* @__PURE__ */ c("div", { className: "grid grid-cols-2 gap-4", children: [
    /* @__PURE__ */ n(
      wr,
      {
        name: "schema",
        label: Ye.schema,
        value: t,
        onChange: r,
        options: o.map((d) => ({
          value: d.name,
          label: d.descriptive_name || d.name
        })),
        placeholder: "Select a schema...",
        emptyMessage: "No approved schemas available for mutations",
        helpText: Ye.schemaHelp
      }
    ),
    /* @__PURE__ */ n(
      wr,
      {
        name: "operationType",
        label: Ye.operationType,
        value: e,
        onChange: i,
        options: ga,
        helpText: Ye.operationHelp
      }
    )
  ] });
}
function co({ fields: t, mutationType: e, mutationData: r, onFieldChange: i, isRangeSchema: o }) {
  if (e === "Delete")
    return /* @__PURE__ */ c("div", { className: "bg-gray-50 rounded-lg p-6", children: [
      /* @__PURE__ */ n("h3", { className: "text-lg font-medium text-gray-900 mb-4", children: "Delete Operation" }),
      /* @__PURE__ */ n("p", { className: "text-sm text-gray-600", children: "This will delete the selected schema. No additional fields are required." })
    ] });
  const d = (h, l) => {
    if (!(l.writable !== !1)) return null;
    const p = r[h] || "";
    switch (l.field_type) {
      case "Collection": {
        let x = [];
        if (p)
          try {
            const g = typeof p == "string" ? JSON.parse(p) : p;
            x = Array.isArray(g) ? g : [g];
          } catch {
            x = p.trim() ? [p] : [];
          }
        return /* @__PURE__ */ c("div", { className: "mb-6", children: [
          /* @__PURE__ */ c("label", { className: "block text-sm font-medium text-gray-700 mb-2", children: [
            h,
            /* @__PURE__ */ n("span", { className: "ml-2 text-xs text-gray-500", children: "Collection" })
          ] }),
          /* @__PURE__ */ n(
            "textarea",
            {
              className: "mt-1 block w-full border-gray-300 rounded-md shadow-sm focus:ring-primary focus:border-primary sm:text-sm font-mono",
              value: x.length > 0 ? JSON.stringify(x, null, 2) : "",
              onChange: (g) => {
                const N = g.target.value.trim();
                if (!N) {
                  i(h, []);
                  return;
                }
                try {
                  const T = JSON.parse(N);
                  i(h, Array.isArray(T) ? T : [T]);
                } catch {
                  i(h, [N]);
                }
              },
              placeholder: 'Enter JSON array (e.g., ["item1", "item2"])',
              rows: 4
            }
          ),
          /* @__PURE__ */ n("p", { className: "mt-1 text-xs text-gray-500", children: "Enter data as a JSON array. Empty input will create an empty array." })
        ] }, h);
      }
      case "Range": {
        if (o)
          return /* @__PURE__ */ c("div", { className: "mb-6", children: [
            /* @__PURE__ */ c("label", { className: "block text-sm font-medium text-gray-700 mb-2", children: [
              h,
              /* @__PURE__ */ n("span", { className: "ml-2 text-xs text-gray-500", children: "Single Value (Range Schema)" })
            ] }),
            /* @__PURE__ */ n(
              "input",
              {
                type: "text",
                className: "mt-1 block w-full border-gray-300 rounded-md shadow-sm focus:ring-primary focus:border-primary sm:text-sm",
                value: p,
                onChange: (E) => i(h, E.target.value),
                placeholder: `Enter ${h} value`
              }
            ),
            /* @__PURE__ */ n("p", { className: "mt-1 text-xs text-gray-500", children: "Enter a single value. The system will automatically handle range formatting." })
          ] }, h);
        let x = {};
        if (p)
          try {
            x = typeof p == "string" ? JSON.parse(p) : p, (typeof x != "object" || Array.isArray(x)) && (x = {});
          } catch {
            x = {};
          }
        const g = Object.entries(x), N = () => {
          const E = [...g, ["", ""]], y = Object.fromEntries(E);
          i(h, y);
        }, T = (E, y, w) => {
          const S = [...g];
          S[E] = [y, w];
          const _ = Object.fromEntries(S);
          i(h, _);
        }, b = (E) => {
          const y = g.filter((S, _) => _ !== E), w = Object.fromEntries(y);
          i(h, w);
        };
        return /* @__PURE__ */ c("div", { className: "mb-6", children: [
          /* @__PURE__ */ c("label", { className: "block text-sm font-medium text-gray-700 mb-2", children: [
            h,
            /* @__PURE__ */ n("span", { className: "ml-2 text-xs text-gray-500", children: "Range (Complex)" })
          ] }),
          /* @__PURE__ */ n("div", { className: "border border-gray-300 rounded-md p-4 bg-gray-50", children: /* @__PURE__ */ c("div", { className: "space-y-3", children: [
            g.length === 0 ? /* @__PURE__ */ n("p", { className: "text-sm text-gray-500 italic", children: "No key-value pairs added yet" }) : g.map(([E, y], w) => /* @__PURE__ */ c("div", { className: "flex items-center space-x-2", children: [
              /* @__PURE__ */ n(
                "input",
                {
                  type: "text",
                  placeholder: "Key",
                  className: "flex-1 border-gray-300 rounded-md shadow-sm focus:ring-primary focus:border-primary sm:text-sm",
                  value: E,
                  onChange: (S) => T(w, S.target.value, y)
                }
              ),
              /* @__PURE__ */ n("span", { className: "text-gray-500", children: ":" }),
              /* @__PURE__ */ n(
                "input",
                {
                  type: "text",
                  placeholder: "Value",
                  className: "flex-1 border-gray-300 rounded-md shadow-sm focus:ring-primary focus:border-primary sm:text-sm",
                  value: y,
                  onChange: (S) => T(w, E, S.target.value)
                }
              ),
              /* @__PURE__ */ n(
                "button",
                {
                  type: "button",
                  onClick: () => b(w),
                  className: "text-red-600 hover:text-red-800 p-1",
                  title: "Remove this key-value pair",
                  children: /* @__PURE__ */ n("svg", { className: "w-4 h-4", fill: "none", stroke: "currentColor", viewBox: "0 0 24 24", children: /* @__PURE__ */ n("path", { strokeLinecap: "round", strokeLinejoin: "round", strokeWidth: 2, d: "M6 18L18 6M6 6l12 12" }) })
                }
              )
            ] }, w)),
            /* @__PURE__ */ c(
              "button",
              {
                type: "button",
                onClick: N,
                className: "inline-flex items-center px-3 py-2 border border-gray-300 shadow-sm text-sm leading-4 font-medium rounded-md text-gray-700 bg-white hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-primary",
                children: [
                  /* @__PURE__ */ n("svg", { className: "w-4 h-4 mr-1", fill: "none", stroke: "currentColor", viewBox: "0 0 24 24", children: /* @__PURE__ */ n("path", { strokeLinecap: "round", strokeLinejoin: "round", strokeWidth: 2, d: "M12 6v6m0 0v6m0-6h6m-6 0H6" }) }),
                  "Add Key-Value Pair"
                ]
              }
            )
          ] }) }),
          /* @__PURE__ */ n("p", { className: "mt-1 text-xs text-gray-500", children: "Add key-value pairs for this range field. Empty keys will be filtered out." })
        ] }, h);
      }
      default:
        return /* @__PURE__ */ c("div", { className: "mb-6", children: [
          /* @__PURE__ */ c("label", { className: "block text-sm font-medium text-gray-700 mb-2", children: [
            h,
            /* @__PURE__ */ n("span", { className: "ml-2 text-xs text-gray-500", children: "Single" })
          ] }),
          /* @__PURE__ */ n(
            "input",
            {
              type: "text",
              className: "mt-1 block w-full border-gray-300 rounded-md shadow-sm focus:ring-primary focus:border-primary sm:text-sm",
              value: p,
              onChange: (x) => i(h, x.target.value),
              placeholder: `Enter ${h}`
            }
          )
        ] }, h);
    }
  };
  return /* @__PURE__ */ c("div", { className: "bg-gray-50 rounded-lg p-6", children: [
    /* @__PURE__ */ c("h3", { className: "text-lg font-medium text-gray-900 mb-4", children: [
      "Schema Fields",
      o && /* @__PURE__ */ n("span", { className: "ml-2 text-sm text-blue-600 font-normal", children: "(Range Schema - Single Values)" })
    ] }),
    /* @__PURE__ */ n("div", { className: "space-y-6", children: Object.entries(t).map(([h, l]) => d(h, l)) }),
    o && Object.keys(t).length === 0 && /* @__PURE__ */ n("p", { className: "text-sm text-gray-500 italic", children: "No additional fields to configure. Only the range key is required for this schema." })
  ] });
}
function uo({ result: t }) {
  return t ? /* @__PURE__ */ n("div", { className: "bg-gray-50 rounded-lg p-4 mt-4", children: /* @__PURE__ */ n("pre", { className: "font-mono text-sm whitespace-pre-wrap", children: typeof t == "string" ? t : JSON.stringify(t, null, 2) }) }) : null;
}
function ho(t) {
  const e = Ae(t);
  return {
    base: e,
    schema: Aa(e),
    mutation: ri(e),
    security: ei(e)
  };
}
ho({
  enableCache: !0,
  enableLogging: !0,
  enableMetrics: !0
});
var mo = {}, Xt = {};
Xt.byteLength = go;
Xt.toByteArray = bo;
Xt.fromByteArray = vo;
var Re = [], xe = [], fo = typeof Uint8Array < "u" ? Uint8Array : Array, fr = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
for (var dt = 0, po = fr.length; dt < po; ++dt)
  Re[dt] = fr[dt], xe[fr.charCodeAt(dt)] = dt;
xe[45] = 62;
xe[95] = 63;
function Jn(t) {
  var e = t.length;
  if (e % 4 > 0)
    throw new Error("Invalid string. Length must be a multiple of 4");
  var r = t.indexOf("=");
  r === -1 && (r = e);
  var i = r === e ? 0 : 4 - r % 4;
  return [r, i];
}
function go(t) {
  var e = Jn(t), r = e[0], i = e[1];
  return (r + i) * 3 / 4 - i;
}
function yo(t, e, r) {
  return (e + r) * 3 / 4 - r;
}
function bo(t) {
  var e, r = Jn(t), i = r[0], o = r[1], d = new fo(yo(t, i, o)), h = 0, l = o > 0 ? i - 4 : i, f;
  for (f = 0; f < l; f += 4)
    e = xe[t.charCodeAt(f)] << 18 | xe[t.charCodeAt(f + 1)] << 12 | xe[t.charCodeAt(f + 2)] << 6 | xe[t.charCodeAt(f + 3)], d[h++] = e >> 16 & 255, d[h++] = e >> 8 & 255, d[h++] = e & 255;
  return o === 2 && (e = xe[t.charCodeAt(f)] << 2 | xe[t.charCodeAt(f + 1)] >> 4, d[h++] = e & 255), o === 1 && (e = xe[t.charCodeAt(f)] << 10 | xe[t.charCodeAt(f + 1)] << 4 | xe[t.charCodeAt(f + 2)] >> 2, d[h++] = e >> 8 & 255, d[h++] = e & 255), d;
}
function xo(t) {
  return Re[t >> 18 & 63] + Re[t >> 12 & 63] + Re[t >> 6 & 63] + Re[t & 63];
}
function wo(t, e, r) {
  for (var i, o = [], d = e; d < r; d += 3)
    i = (t[d] << 16 & 16711680) + (t[d + 1] << 8 & 65280) + (t[d + 2] & 255), o.push(xo(i));
  return o.join("");
}
function vo(t) {
  for (var e, r = t.length, i = r % 3, o = [], d = 16383, h = 0, l = r - i; h < l; h += d)
    o.push(wo(t, h, h + d > l ? l : h + d));
  return i === 1 ? (e = t[r - 1], o.push(
    Re[e >> 2] + Re[e << 4 & 63] + "=="
  )) : i === 2 && (e = (t[r - 2] << 8) + t[r - 1], o.push(
    Re[e >> 10] + Re[e >> 4 & 63] + Re[e << 2 & 63] + "="
  )), o.join("");
}
var Ur = {};
/*! ieee754. BSD-3-Clause License. Feross Aboukhadijeh <https://feross.org/opensource> */
Ur.read = function(t, e, r, i, o) {
  var d, h, l = o * 8 - i - 1, f = (1 << l) - 1, p = f >> 1, x = -7, g = r ? o - 1 : 0, N = r ? -1 : 1, T = t[e + g];
  for (g += N, d = T & (1 << -x) - 1, T >>= -x, x += l; x > 0; d = d * 256 + t[e + g], g += N, x -= 8)
    ;
  for (h = d & (1 << -x) - 1, d >>= -x, x += i; x > 0; h = h * 256 + t[e + g], g += N, x -= 8)
    ;
  if (d === 0)
    d = 1 - p;
  else {
    if (d === f)
      return h ? NaN : (T ? -1 : 1) * (1 / 0);
    h = h + Math.pow(2, i), d = d - p;
  }
  return (T ? -1 : 1) * h * Math.pow(2, d - i);
};
Ur.write = function(t, e, r, i, o, d) {
  var h, l, f, p = d * 8 - o - 1, x = (1 << p) - 1, g = x >> 1, N = o === 23 ? Math.pow(2, -24) - Math.pow(2, -77) : 0, T = i ? 0 : d - 1, b = i ? 1 : -1, E = e < 0 || e === 0 && 1 / e < 0 ? 1 : 0;
  for (e = Math.abs(e), isNaN(e) || e === 1 / 0 ? (l = isNaN(e) ? 1 : 0, h = x) : (h = Math.floor(Math.log(e) / Math.LN2), e * (f = Math.pow(2, -h)) < 1 && (h--, f *= 2), h + g >= 1 ? e += N / f : e += N * Math.pow(2, 1 - g), e * f >= 2 && (h++, f /= 2), h + g >= x ? (l = 0, h = x) : h + g >= 1 ? (l = (e * f - 1) * Math.pow(2, o), h = h + g) : (l = e * Math.pow(2, g - 1) * Math.pow(2, o), h = 0)); o >= 8; t[r + T] = l & 255, T += b, l /= 256, o -= 8)
    ;
  for (h = h << o | l, p += o; p > 0; t[r + T] = h & 255, T += b, h /= 256, p -= 8)
    ;
  t[r + T - b] |= E * 128;
};
/*!
 * The buffer module from node.js, for the browser.
 *
 * @author   Feross Aboukhadijeh <https://feross.org>
 * @license  MIT
 */
(function(t) {
  const e = Xt, r = Ur, i = typeof Symbol == "function" && typeof Symbol.for == "function" ? Symbol.for("nodejs.util.inspect.custom") : null;
  t.Buffer = l, t.SlowBuffer = S, t.INSPECT_MAX_BYTES = 50;
  const o = 2147483647;
  t.kMaxLength = o, l.TYPED_ARRAY_SUPPORT = d(), !l.TYPED_ARRAY_SUPPORT && typeof console < "u" && typeof console.error == "function" && console.error(
    "This browser lacks typed array (Uint8Array) support which is required by `buffer` v5.x. Use `buffer` v4.x if you require old browser support."
  );
  function d() {
    try {
      const u = new Uint8Array(1), s = { foo: function() {
        return 42;
      } };
      return Object.setPrototypeOf(s, Uint8Array.prototype), Object.setPrototypeOf(u, s), u.foo() === 42;
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
  function h(u) {
    if (u > o)
      throw new RangeError('The value "' + u + '" is invalid for option "size"');
    const s = new Uint8Array(u);
    return Object.setPrototypeOf(s, l.prototype), s;
  }
  function l(u, s, a) {
    if (typeof u == "number") {
      if (typeof s == "string")
        throw new TypeError(
          'The "string" argument must be of type string. Received type number'
        );
      return g(u);
    }
    return f(u, s, a);
  }
  l.poolSize = 8192;
  function f(u, s, a) {
    if (typeof u == "string")
      return N(u, s);
    if (ArrayBuffer.isView(u))
      return b(u);
    if (u == null)
      throw new TypeError(
        "The first argument must be one of type string, Buffer, ArrayBuffer, Array, or Array-like Object. Received type " + typeof u
      );
    if (_e(u, ArrayBuffer) || u && _e(u.buffer, ArrayBuffer) || typeof SharedArrayBuffer < "u" && (_e(u, SharedArrayBuffer) || u && _e(u.buffer, SharedArrayBuffer)))
      return E(u, s, a);
    if (typeof u == "number")
      throw new TypeError(
        'The "value" argument must not be of type number. Received type number'
      );
    const m = u.valueOf && u.valueOf();
    if (m != null && m !== u)
      return l.from(m, s, a);
    const v = y(u);
    if (v) return v;
    if (typeof Symbol < "u" && Symbol.toPrimitive != null && typeof u[Symbol.toPrimitive] == "function")
      return l.from(u[Symbol.toPrimitive]("string"), s, a);
    throw new TypeError(
      "The first argument must be one of type string, Buffer, ArrayBuffer, Array, or Array-like Object. Received type " + typeof u
    );
  }
  l.from = function(u, s, a) {
    return f(u, s, a);
  }, Object.setPrototypeOf(l.prototype, Uint8Array.prototype), Object.setPrototypeOf(l, Uint8Array);
  function p(u) {
    if (typeof u != "number")
      throw new TypeError('"size" argument must be of type number');
    if (u < 0)
      throw new RangeError('The value "' + u + '" is invalid for option "size"');
  }
  function x(u, s, a) {
    return p(u), u <= 0 ? h(u) : s !== void 0 ? typeof a == "string" ? h(u).fill(s, a) : h(u).fill(s) : h(u);
  }
  l.alloc = function(u, s, a) {
    return x(u, s, a);
  };
  function g(u) {
    return p(u), h(u < 0 ? 0 : w(u) | 0);
  }
  l.allocUnsafe = function(u) {
    return g(u);
  }, l.allocUnsafeSlow = function(u) {
    return g(u);
  };
  function N(u, s) {
    if ((typeof s != "string" || s === "") && (s = "utf8"), !l.isEncoding(s))
      throw new TypeError("Unknown encoding: " + s);
    const a = _(u, s) | 0;
    let m = h(a);
    const v = m.write(u, s);
    return v !== a && (m = m.slice(0, v)), m;
  }
  function T(u) {
    const s = u.length < 0 ? 0 : w(u.length) | 0, a = h(s);
    for (let m = 0; m < s; m += 1)
      a[m] = u[m] & 255;
    return a;
  }
  function b(u) {
    if (_e(u, Uint8Array)) {
      const s = new Uint8Array(u);
      return E(s.buffer, s.byteOffset, s.byteLength);
    }
    return T(u);
  }
  function E(u, s, a) {
    if (s < 0 || u.byteLength < s)
      throw new RangeError('"offset" is outside of buffer bounds');
    if (u.byteLength < s + (a || 0))
      throw new RangeError('"length" is outside of buffer bounds');
    let m;
    return s === void 0 && a === void 0 ? m = new Uint8Array(u) : a === void 0 ? m = new Uint8Array(u, s) : m = new Uint8Array(u, s, a), Object.setPrototypeOf(m, l.prototype), m;
  }
  function y(u) {
    if (l.isBuffer(u)) {
      const s = w(u.length) | 0, a = h(s);
      return a.length === 0 || u.copy(a, 0, 0, s), a;
    }
    if (u.length !== void 0)
      return typeof u.length != "number" || nr(u.length) ? h(0) : T(u);
    if (u.type === "Buffer" && Array.isArray(u.data))
      return T(u.data);
  }
  function w(u) {
    if (u >= o)
      throw new RangeError("Attempt to allocate Buffer larger than maximum size: 0x" + o.toString(16) + " bytes");
    return u | 0;
  }
  function S(u) {
    return +u != u && (u = 0), l.alloc(+u);
  }
  l.isBuffer = function(s) {
    return s != null && s._isBuffer === !0 && s !== l.prototype;
  }, l.compare = function(s, a) {
    if (_e(s, Uint8Array) && (s = l.from(s, s.offset, s.byteLength)), _e(a, Uint8Array) && (a = l.from(a, a.offset, a.byteLength)), !l.isBuffer(s) || !l.isBuffer(a))
      throw new TypeError(
        'The "buf1", "buf2" arguments must be one of type Buffer or Uint8Array'
      );
    if (s === a) return 0;
    let m = s.length, v = a.length;
    for (let A = 0, k = Math.min(m, v); A < k; ++A)
      if (s[A] !== a[A]) {
        m = s[A], v = a[A];
        break;
      }
    return m < v ? -1 : v < m ? 1 : 0;
  }, l.isEncoding = function(s) {
    switch (String(s).toLowerCase()) {
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
  }, l.concat = function(s, a) {
    if (!Array.isArray(s))
      throw new TypeError('"list" argument must be an Array of Buffers');
    if (s.length === 0)
      return l.alloc(0);
    let m;
    if (a === void 0)
      for (a = 0, m = 0; m < s.length; ++m)
        a += s[m].length;
    const v = l.allocUnsafe(a);
    let A = 0;
    for (m = 0; m < s.length; ++m) {
      let k = s[m];
      if (_e(k, Uint8Array))
        A + k.length > v.length ? (l.isBuffer(k) || (k = l.from(k)), k.copy(v, A)) : Uint8Array.prototype.set.call(
          v,
          k,
          A
        );
      else if (l.isBuffer(k))
        k.copy(v, A);
      else
        throw new TypeError('"list" argument must be an Array of Buffers');
      A += k.length;
    }
    return v;
  };
  function _(u, s) {
    if (l.isBuffer(u))
      return u.length;
    if (ArrayBuffer.isView(u) || _e(u, ArrayBuffer))
      return u.byteLength;
    if (typeof u != "string")
      throw new TypeError(
        'The "string" argument must be one of type string, Buffer, or ArrayBuffer. Received type ' + typeof u
      );
    const a = u.length, m = arguments.length > 2 && arguments[2] === !0;
    if (!m && a === 0) return 0;
    let v = !1;
    for (; ; )
      switch (s) {
        case "ascii":
        case "latin1":
        case "binary":
          return a;
        case "utf8":
        case "utf-8":
          return D(u).length;
        case "ucs2":
        case "ucs-2":
        case "utf16le":
        case "utf-16le":
          return a * 2;
        case "hex":
          return a >>> 1;
        case "base64":
          return Pe(u).length;
        default:
          if (v)
            return m ? -1 : D(u).length;
          s = ("" + s).toLowerCase(), v = !0;
      }
  }
  l.byteLength = _;
  function R(u, s, a) {
    let m = !1;
    if ((s === void 0 || s < 0) && (s = 0), s > this.length || ((a === void 0 || a > this.length) && (a = this.length), a <= 0) || (a >>>= 0, s >>>= 0, a <= s))
      return "";
    for (u || (u = "utf8"); ; )
      switch (u) {
        case "hex":
          return tt(this, s, a);
        case "utf8":
        case "utf-8":
          return G(this, s, a);
        case "ascii":
          return J(this, s, a);
        case "latin1":
        case "binary":
          return ie(this, s, a);
        case "base64":
          return H(this, s, a);
        case "ucs2":
        case "ucs-2":
        case "utf16le":
        case "utf-16le":
          return rt(this, s, a);
        default:
          if (m) throw new TypeError("Unknown encoding: " + u);
          u = (u + "").toLowerCase(), m = !0;
      }
  }
  l.prototype._isBuffer = !0;
  function F(u, s, a) {
    const m = u[s];
    u[s] = u[a], u[a] = m;
  }
  l.prototype.swap16 = function() {
    const s = this.length;
    if (s % 2 !== 0)
      throw new RangeError("Buffer size must be a multiple of 16-bits");
    for (let a = 0; a < s; a += 2)
      F(this, a, a + 1);
    return this;
  }, l.prototype.swap32 = function() {
    const s = this.length;
    if (s % 4 !== 0)
      throw new RangeError("Buffer size must be a multiple of 32-bits");
    for (let a = 0; a < s; a += 4)
      F(this, a, a + 3), F(this, a + 1, a + 2);
    return this;
  }, l.prototype.swap64 = function() {
    const s = this.length;
    if (s % 8 !== 0)
      throw new RangeError("Buffer size must be a multiple of 64-bits");
    for (let a = 0; a < s; a += 8)
      F(this, a, a + 7), F(this, a + 1, a + 6), F(this, a + 2, a + 5), F(this, a + 3, a + 4);
    return this;
  }, l.prototype.toString = function() {
    const s = this.length;
    return s === 0 ? "" : arguments.length === 0 ? G(this, 0, s) : R.apply(this, arguments);
  }, l.prototype.toLocaleString = l.prototype.toString, l.prototype.equals = function(s) {
    if (!l.isBuffer(s)) throw new TypeError("Argument must be a Buffer");
    return this === s ? !0 : l.compare(this, s) === 0;
  }, l.prototype.inspect = function() {
    let s = "";
    const a = t.INSPECT_MAX_BYTES;
    return s = this.toString("hex", 0, a).replace(/(.{2})/g, "$1 ").trim(), this.length > a && (s += " ... "), "<Buffer " + s + ">";
  }, i && (l.prototype[i] = l.prototype.inspect), l.prototype.compare = function(s, a, m, v, A) {
    if (_e(s, Uint8Array) && (s = l.from(s, s.offset, s.byteLength)), !l.isBuffer(s))
      throw new TypeError(
        'The "target" argument must be one of type Buffer or Uint8Array. Received type ' + typeof s
      );
    if (a === void 0 && (a = 0), m === void 0 && (m = s ? s.length : 0), v === void 0 && (v = 0), A === void 0 && (A = this.length), a < 0 || m > s.length || v < 0 || A > this.length)
      throw new RangeError("out of range index");
    if (v >= A && a >= m)
      return 0;
    if (v >= A)
      return -1;
    if (a >= m)
      return 1;
    if (a >>>= 0, m >>>= 0, v >>>= 0, A >>>= 0, this === s) return 0;
    let k = A - v, z = m - a;
    const ne = Math.min(k, z), ee = this.slice(v, A), se = s.slice(a, m);
    for (let W = 0; W < ne; ++W)
      if (ee[W] !== se[W]) {
        k = ee[W], z = se[W];
        break;
      }
    return k < z ? -1 : z < k ? 1 : 0;
  };
  function P(u, s, a, m, v) {
    if (u.length === 0) return -1;
    if (typeof a == "string" ? (m = a, a = 0) : a > 2147483647 ? a = 2147483647 : a < -2147483648 && (a = -2147483648), a = +a, nr(a) && (a = v ? 0 : u.length - 1), a < 0 && (a = u.length + a), a >= u.length) {
      if (v) return -1;
      a = u.length - 1;
    } else if (a < 0)
      if (v) a = 0;
      else return -1;
    if (typeof s == "string" && (s = l.from(s, m)), l.isBuffer(s))
      return s.length === 0 ? -1 : C(u, s, a, m, v);
    if (typeof s == "number")
      return s = s & 255, typeof Uint8Array.prototype.indexOf == "function" ? v ? Uint8Array.prototype.indexOf.call(u, s, a) : Uint8Array.prototype.lastIndexOf.call(u, s, a) : C(u, [s], a, m, v);
    throw new TypeError("val must be string, number or Buffer");
  }
  function C(u, s, a, m, v) {
    let A = 1, k = u.length, z = s.length;
    if (m !== void 0 && (m = String(m).toLowerCase(), m === "ucs2" || m === "ucs-2" || m === "utf16le" || m === "utf-16le")) {
      if (u.length < 2 || s.length < 2)
        return -1;
      A = 2, k /= 2, z /= 2, a /= 2;
    }
    function ne(se, W) {
      return A === 1 ? se[W] : se.readUInt16BE(W * A);
    }
    let ee;
    if (v) {
      let se = -1;
      for (ee = a; ee < k; ee++)
        if (ne(u, ee) === ne(s, se === -1 ? 0 : ee - se)) {
          if (se === -1 && (se = ee), ee - se + 1 === z) return se * A;
        } else
          se !== -1 && (ee -= ee - se), se = -1;
    } else
      for (a + z > k && (a = k - z), ee = a; ee >= 0; ee--) {
        let se = !0;
        for (let W = 0; W < z; W++)
          if (ne(u, ee + W) !== ne(s, W)) {
            se = !1;
            break;
          }
        if (se) return ee;
      }
    return -1;
  }
  l.prototype.includes = function(s, a, m) {
    return this.indexOf(s, a, m) !== -1;
  }, l.prototype.indexOf = function(s, a, m) {
    return P(this, s, a, m, !0);
  }, l.prototype.lastIndexOf = function(s, a, m) {
    return P(this, s, a, m, !1);
  };
  function I(u, s, a, m) {
    a = Number(a) || 0;
    const v = u.length - a;
    m ? (m = Number(m), m > v && (m = v)) : m = v;
    const A = s.length;
    m > A / 2 && (m = A / 2);
    let k;
    for (k = 0; k < m; ++k) {
      const z = parseInt(s.substr(k * 2, 2), 16);
      if (nr(z)) return k;
      u[a + k] = z;
    }
    return k;
  }
  function O(u, s, a, m) {
    return Ot(D(s, u.length - a), u, a, m);
  }
  function U(u, s, a, m) {
    return Ot(Q(s), u, a, m);
  }
  function K(u, s, a, m) {
    return Ot(Pe(s), u, a, m);
  }
  function V(u, s, a, m) {
    return Ot(ye(s, u.length - a), u, a, m);
  }
  l.prototype.write = function(s, a, m, v) {
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
    const A = this.length - a;
    if ((m === void 0 || m > A) && (m = A), s.length > 0 && (m < 0 || a < 0) || a > this.length)
      throw new RangeError("Attempt to write outside buffer bounds");
    v || (v = "utf8");
    let k = !1;
    for (; ; )
      switch (v) {
        case "hex":
          return I(this, s, a, m);
        case "utf8":
        case "utf-8":
          return O(this, s, a, m);
        case "ascii":
        case "latin1":
        case "binary":
          return U(this, s, a, m);
        case "base64":
          return K(this, s, a, m);
        case "ucs2":
        case "ucs-2":
        case "utf16le":
        case "utf-16le":
          return V(this, s, a, m);
        default:
          if (k) throw new TypeError("Unknown encoding: " + v);
          v = ("" + v).toLowerCase(), k = !0;
      }
  }, l.prototype.toJSON = function() {
    return {
      type: "Buffer",
      data: Array.prototype.slice.call(this._arr || this, 0)
    };
  };
  function H(u, s, a) {
    return s === 0 && a === u.length ? e.fromByteArray(u) : e.fromByteArray(u.slice(s, a));
  }
  function G(u, s, a) {
    a = Math.min(u.length, a);
    const m = [];
    let v = s;
    for (; v < a; ) {
      const A = u[v];
      let k = null, z = A > 239 ? 4 : A > 223 ? 3 : A > 191 ? 2 : 1;
      if (v + z <= a) {
        let ne, ee, se, W;
        switch (z) {
          case 1:
            A < 128 && (k = A);
            break;
          case 2:
            ne = u[v + 1], (ne & 192) === 128 && (W = (A & 31) << 6 | ne & 63, W > 127 && (k = W));
            break;
          case 3:
            ne = u[v + 1], ee = u[v + 2], (ne & 192) === 128 && (ee & 192) === 128 && (W = (A & 15) << 12 | (ne & 63) << 6 | ee & 63, W > 2047 && (W < 55296 || W > 57343) && (k = W));
            break;
          case 4:
            ne = u[v + 1], ee = u[v + 2], se = u[v + 3], (ne & 192) === 128 && (ee & 192) === 128 && (se & 192) === 128 && (W = (A & 15) << 18 | (ne & 63) << 12 | (ee & 63) << 6 | se & 63, W > 65535 && W < 1114112 && (k = W));
        }
      }
      k === null ? (k = 65533, z = 1) : k > 65535 && (k -= 65536, m.push(k >>> 10 & 1023 | 55296), k = 56320 | k & 1023), m.push(k), v += z;
    }
    return j(m);
  }
  const L = 4096;
  function j(u) {
    const s = u.length;
    if (s <= L)
      return String.fromCharCode.apply(String, u);
    let a = "", m = 0;
    for (; m < s; )
      a += String.fromCharCode.apply(
        String,
        u.slice(m, m += L)
      );
    return a;
  }
  function J(u, s, a) {
    let m = "";
    a = Math.min(u.length, a);
    for (let v = s; v < a; ++v)
      m += String.fromCharCode(u[v] & 127);
    return m;
  }
  function ie(u, s, a) {
    let m = "";
    a = Math.min(u.length, a);
    for (let v = s; v < a; ++v)
      m += String.fromCharCode(u[v]);
    return m;
  }
  function tt(u, s, a) {
    const m = u.length;
    (!s || s < 0) && (s = 0), (!a || a < 0 || a > m) && (a = m);
    let v = "";
    for (let A = s; A < a; ++A)
      v += Zn[u[A]];
    return v;
  }
  function rt(u, s, a) {
    const m = u.slice(s, a);
    let v = "";
    for (let A = 0; A < m.length - 1; A += 2)
      v += String.fromCharCode(m[A] + m[A + 1] * 256);
    return v;
  }
  l.prototype.slice = function(s, a) {
    const m = this.length;
    s = ~~s, a = a === void 0 ? m : ~~a, s < 0 ? (s += m, s < 0 && (s = 0)) : s > m && (s = m), a < 0 ? (a += m, a < 0 && (a = 0)) : a > m && (a = m), a < s && (a = s);
    const v = this.subarray(s, a);
    return Object.setPrototypeOf(v, l.prototype), v;
  };
  function X(u, s, a) {
    if (u % 1 !== 0 || u < 0) throw new RangeError("offset is not uint");
    if (u + s > a) throw new RangeError("Trying to access beyond buffer length");
  }
  l.prototype.readUintLE = l.prototype.readUIntLE = function(s, a, m) {
    s = s >>> 0, a = a >>> 0, m || X(s, a, this.length);
    let v = this[s], A = 1, k = 0;
    for (; ++k < a && (A *= 256); )
      v += this[s + k] * A;
    return v;
  }, l.prototype.readUintBE = l.prototype.readUIntBE = function(s, a, m) {
    s = s >>> 0, a = a >>> 0, m || X(s, a, this.length);
    let v = this[s + --a], A = 1;
    for (; a > 0 && (A *= 256); )
      v += this[s + --a] * A;
    return v;
  }, l.prototype.readUint8 = l.prototype.readUInt8 = function(s, a) {
    return s = s >>> 0, a || X(s, 1, this.length), this[s];
  }, l.prototype.readUint16LE = l.prototype.readUInt16LE = function(s, a) {
    return s = s >>> 0, a || X(s, 2, this.length), this[s] | this[s + 1] << 8;
  }, l.prototype.readUint16BE = l.prototype.readUInt16BE = function(s, a) {
    return s = s >>> 0, a || X(s, 2, this.length), this[s] << 8 | this[s + 1];
  }, l.prototype.readUint32LE = l.prototype.readUInt32LE = function(s, a) {
    return s = s >>> 0, a || X(s, 4, this.length), (this[s] | this[s + 1] << 8 | this[s + 2] << 16) + this[s + 3] * 16777216;
  }, l.prototype.readUint32BE = l.prototype.readUInt32BE = function(s, a) {
    return s = s >>> 0, a || X(s, 4, this.length), this[s] * 16777216 + (this[s + 1] << 16 | this[s + 2] << 8 | this[s + 3]);
  }, l.prototype.readBigUInt64LE = Ue(function(s) {
    s = s >>> 0, Me(s, "offset");
    const a = this[s], m = this[s + 7];
    (a === void 0 || m === void 0) && ze(s, this.length - 8);
    const v = a + this[++s] * 2 ** 8 + this[++s] * 2 ** 16 + this[++s] * 2 ** 24, A = this[++s] + this[++s] * 2 ** 8 + this[++s] * 2 ** 16 + m * 2 ** 24;
    return BigInt(v) + (BigInt(A) << BigInt(32));
  }), l.prototype.readBigUInt64BE = Ue(function(s) {
    s = s >>> 0, Me(s, "offset");
    const a = this[s], m = this[s + 7];
    (a === void 0 || m === void 0) && ze(s, this.length - 8);
    const v = a * 2 ** 24 + this[++s] * 2 ** 16 + this[++s] * 2 ** 8 + this[++s], A = this[++s] * 2 ** 24 + this[++s] * 2 ** 16 + this[++s] * 2 ** 8 + m;
    return (BigInt(v) << BigInt(32)) + BigInt(A);
  }), l.prototype.readIntLE = function(s, a, m) {
    s = s >>> 0, a = a >>> 0, m || X(s, a, this.length);
    let v = this[s], A = 1, k = 0;
    for (; ++k < a && (A *= 256); )
      v += this[s + k] * A;
    return A *= 128, v >= A && (v -= Math.pow(2, 8 * a)), v;
  }, l.prototype.readIntBE = function(s, a, m) {
    s = s >>> 0, a = a >>> 0, m || X(s, a, this.length);
    let v = a, A = 1, k = this[s + --v];
    for (; v > 0 && (A *= 256); )
      k += this[s + --v] * A;
    return A *= 128, k >= A && (k -= Math.pow(2, 8 * a)), k;
  }, l.prototype.readInt8 = function(s, a) {
    return s = s >>> 0, a || X(s, 1, this.length), this[s] & 128 ? (255 - this[s] + 1) * -1 : this[s];
  }, l.prototype.readInt16LE = function(s, a) {
    s = s >>> 0, a || X(s, 2, this.length);
    const m = this[s] | this[s + 1] << 8;
    return m & 32768 ? m | 4294901760 : m;
  }, l.prototype.readInt16BE = function(s, a) {
    s = s >>> 0, a || X(s, 2, this.length);
    const m = this[s + 1] | this[s] << 8;
    return m & 32768 ? m | 4294901760 : m;
  }, l.prototype.readInt32LE = function(s, a) {
    return s = s >>> 0, a || X(s, 4, this.length), this[s] | this[s + 1] << 8 | this[s + 2] << 16 | this[s + 3] << 24;
  }, l.prototype.readInt32BE = function(s, a) {
    return s = s >>> 0, a || X(s, 4, this.length), this[s] << 24 | this[s + 1] << 16 | this[s + 2] << 8 | this[s + 3];
  }, l.prototype.readBigInt64LE = Ue(function(s) {
    s = s >>> 0, Me(s, "offset");
    const a = this[s], m = this[s + 7];
    (a === void 0 || m === void 0) && ze(s, this.length - 8);
    const v = this[s + 4] + this[s + 5] * 2 ** 8 + this[s + 6] * 2 ** 16 + (m << 24);
    return (BigInt(v) << BigInt(32)) + BigInt(a + this[++s] * 2 ** 8 + this[++s] * 2 ** 16 + this[++s] * 2 ** 24);
  }), l.prototype.readBigInt64BE = Ue(function(s) {
    s = s >>> 0, Me(s, "offset");
    const a = this[s], m = this[s + 7];
    (a === void 0 || m === void 0) && ze(s, this.length - 8);
    const v = (a << 24) + // Overflow
    this[++s] * 2 ** 16 + this[++s] * 2 ** 8 + this[++s];
    return (BigInt(v) << BigInt(32)) + BigInt(this[++s] * 2 ** 24 + this[++s] * 2 ** 16 + this[++s] * 2 ** 8 + m);
  }), l.prototype.readFloatLE = function(s, a) {
    return s = s >>> 0, a || X(s, 4, this.length), r.read(this, s, !0, 23, 4);
  }, l.prototype.readFloatBE = function(s, a) {
    return s = s >>> 0, a || X(s, 4, this.length), r.read(this, s, !1, 23, 4);
  }, l.prototype.readDoubleLE = function(s, a) {
    return s = s >>> 0, a || X(s, 8, this.length), r.read(this, s, !0, 52, 8);
  }, l.prototype.readDoubleBE = function(s, a) {
    return s = s >>> 0, a || X(s, 8, this.length), r.read(this, s, !1, 52, 8);
  };
  function oe(u, s, a, m, v, A) {
    if (!l.isBuffer(u)) throw new TypeError('"buffer" argument must be a Buffer instance');
    if (s > v || s < A) throw new RangeError('"value" argument is out of bounds');
    if (a + m > u.length) throw new RangeError("Index out of range");
  }
  l.prototype.writeUintLE = l.prototype.writeUIntLE = function(s, a, m, v) {
    if (s = +s, a = a >>> 0, m = m >>> 0, !v) {
      const z = Math.pow(2, 8 * m) - 1;
      oe(this, s, a, m, z, 0);
    }
    let A = 1, k = 0;
    for (this[a] = s & 255; ++k < m && (A *= 256); )
      this[a + k] = s / A & 255;
    return a + m;
  }, l.prototype.writeUintBE = l.prototype.writeUIntBE = function(s, a, m, v) {
    if (s = +s, a = a >>> 0, m = m >>> 0, !v) {
      const z = Math.pow(2, 8 * m) - 1;
      oe(this, s, a, m, z, 0);
    }
    let A = m - 1, k = 1;
    for (this[a + A] = s & 255; --A >= 0 && (k *= 256); )
      this[a + A] = s / k & 255;
    return a + m;
  }, l.prototype.writeUint8 = l.prototype.writeUInt8 = function(s, a, m) {
    return s = +s, a = a >>> 0, m || oe(this, s, a, 1, 255, 0), this[a] = s & 255, a + 1;
  }, l.prototype.writeUint16LE = l.prototype.writeUInt16LE = function(s, a, m) {
    return s = +s, a = a >>> 0, m || oe(this, s, a, 2, 65535, 0), this[a] = s & 255, this[a + 1] = s >>> 8, a + 2;
  }, l.prototype.writeUint16BE = l.prototype.writeUInt16BE = function(s, a, m) {
    return s = +s, a = a >>> 0, m || oe(this, s, a, 2, 65535, 0), this[a] = s >>> 8, this[a + 1] = s & 255, a + 2;
  }, l.prototype.writeUint32LE = l.prototype.writeUInt32LE = function(s, a, m) {
    return s = +s, a = a >>> 0, m || oe(this, s, a, 4, 4294967295, 0), this[a + 3] = s >>> 24, this[a + 2] = s >>> 16, this[a + 1] = s >>> 8, this[a] = s & 255, a + 4;
  }, l.prototype.writeUint32BE = l.prototype.writeUInt32BE = function(s, a, m) {
    return s = +s, a = a >>> 0, m || oe(this, s, a, 4, 4294967295, 0), this[a] = s >>> 24, this[a + 1] = s >>> 16, this[a + 2] = s >>> 8, this[a + 3] = s & 255, a + 4;
  };
  function bt(u, s, a, m, v) {
    Nt(s, m, v, u, a, 7);
    let A = Number(s & BigInt(4294967295));
    u[a++] = A, A = A >> 8, u[a++] = A, A = A >> 8, u[a++] = A, A = A >> 8, u[a++] = A;
    let k = Number(s >> BigInt(32) & BigInt(4294967295));
    return u[a++] = k, k = k >> 8, u[a++] = k, k = k >> 8, u[a++] = k, k = k >> 8, u[a++] = k, a;
  }
  function nt(u, s, a, m, v) {
    Nt(s, m, v, u, a, 7);
    let A = Number(s & BigInt(4294967295));
    u[a + 7] = A, A = A >> 8, u[a + 6] = A, A = A >> 8, u[a + 5] = A, A = A >> 8, u[a + 4] = A;
    let k = Number(s >> BigInt(32) & BigInt(4294967295));
    return u[a + 3] = k, k = k >> 8, u[a + 2] = k, k = k >> 8, u[a + 1] = k, k = k >> 8, u[a] = k, a + 8;
  }
  l.prototype.writeBigUInt64LE = Ue(function(s, a = 0) {
    return bt(this, s, a, BigInt(0), BigInt("0xffffffffffffffff"));
  }), l.prototype.writeBigUInt64BE = Ue(function(s, a = 0) {
    return nt(this, s, a, BigInt(0), BigInt("0xffffffffffffffff"));
  }), l.prototype.writeIntLE = function(s, a, m, v) {
    if (s = +s, a = a >>> 0, !v) {
      const ne = Math.pow(2, 8 * m - 1);
      oe(this, s, a, m, ne - 1, -ne);
    }
    let A = 0, k = 1, z = 0;
    for (this[a] = s & 255; ++A < m && (k *= 256); )
      s < 0 && z === 0 && this[a + A - 1] !== 0 && (z = 1), this[a + A] = (s / k >> 0) - z & 255;
    return a + m;
  }, l.prototype.writeIntBE = function(s, a, m, v) {
    if (s = +s, a = a >>> 0, !v) {
      const ne = Math.pow(2, 8 * m - 1);
      oe(this, s, a, m, ne - 1, -ne);
    }
    let A = m - 1, k = 1, z = 0;
    for (this[a + A] = s & 255; --A >= 0 && (k *= 256); )
      s < 0 && z === 0 && this[a + A + 1] !== 0 && (z = 1), this[a + A] = (s / k >> 0) - z & 255;
    return a + m;
  }, l.prototype.writeInt8 = function(s, a, m) {
    return s = +s, a = a >>> 0, m || oe(this, s, a, 1, 127, -128), s < 0 && (s = 255 + s + 1), this[a] = s & 255, a + 1;
  }, l.prototype.writeInt16LE = function(s, a, m) {
    return s = +s, a = a >>> 0, m || oe(this, s, a, 2, 32767, -32768), this[a] = s & 255, this[a + 1] = s >>> 8, a + 2;
  }, l.prototype.writeInt16BE = function(s, a, m) {
    return s = +s, a = a >>> 0, m || oe(this, s, a, 2, 32767, -32768), this[a] = s >>> 8, this[a + 1] = s & 255, a + 2;
  }, l.prototype.writeInt32LE = function(s, a, m) {
    return s = +s, a = a >>> 0, m || oe(this, s, a, 4, 2147483647, -2147483648), this[a] = s & 255, this[a + 1] = s >>> 8, this[a + 2] = s >>> 16, this[a + 3] = s >>> 24, a + 4;
  }, l.prototype.writeInt32BE = function(s, a, m) {
    return s = +s, a = a >>> 0, m || oe(this, s, a, 4, 2147483647, -2147483648), s < 0 && (s = 4294967295 + s + 1), this[a] = s >>> 24, this[a + 1] = s >>> 16, this[a + 2] = s >>> 8, this[a + 3] = s & 255, a + 4;
  }, l.prototype.writeBigInt64LE = Ue(function(s, a = 0) {
    return bt(this, s, a, -BigInt("0x8000000000000000"), BigInt("0x7fffffffffffffff"));
  }), l.prototype.writeBigInt64BE = Ue(function(s, a = 0) {
    return nt(this, s, a, -BigInt("0x8000000000000000"), BigInt("0x7fffffffffffffff"));
  });
  function xt(u, s, a, m, v, A) {
    if (a + m > u.length) throw new RangeError("Index out of range");
    if (a < 0) throw new RangeError("Index out of range");
  }
  function wt(u, s, a, m, v) {
    return s = +s, a = a >>> 0, v || xt(u, s, a, 4), r.write(u, s, a, m, 23, 4), a + 4;
  }
  l.prototype.writeFloatLE = function(s, a, m) {
    return wt(this, s, a, !0, m);
  }, l.prototype.writeFloatBE = function(s, a, m) {
    return wt(this, s, a, !1, m);
  };
  function vt(u, s, a, m, v) {
    return s = +s, a = a >>> 0, v || xt(u, s, a, 8), r.write(u, s, a, m, 52, 8), a + 8;
  }
  l.prototype.writeDoubleLE = function(s, a, m) {
    return vt(this, s, a, !0, m);
  }, l.prototype.writeDoubleBE = function(s, a, m) {
    return vt(this, s, a, !1, m);
  }, l.prototype.copy = function(s, a, m, v) {
    if (!l.isBuffer(s)) throw new TypeError("argument should be a Buffer");
    if (m || (m = 0), !v && v !== 0 && (v = this.length), a >= s.length && (a = s.length), a || (a = 0), v > 0 && v < m && (v = m), v === m || s.length === 0 || this.length === 0) return 0;
    if (a < 0)
      throw new RangeError("targetStart out of bounds");
    if (m < 0 || m >= this.length) throw new RangeError("Index out of range");
    if (v < 0) throw new RangeError("sourceEnd out of bounds");
    v > this.length && (v = this.length), s.length - a < v - m && (v = s.length - a + m);
    const A = v - m;
    return this === s && typeof Uint8Array.prototype.copyWithin == "function" ? this.copyWithin(a, m, v) : Uint8Array.prototype.set.call(
      s,
      this.subarray(m, v),
      a
    ), A;
  }, l.prototype.fill = function(s, a, m, v) {
    if (typeof s == "string") {
      if (typeof a == "string" ? (v = a, a = 0, m = this.length) : typeof m == "string" && (v = m, m = this.length), v !== void 0 && typeof v != "string")
        throw new TypeError("encoding must be a string");
      if (typeof v == "string" && !l.isEncoding(v))
        throw new TypeError("Unknown encoding: " + v);
      if (s.length === 1) {
        const k = s.charCodeAt(0);
        (v === "utf8" && k < 128 || v === "latin1") && (s = k);
      }
    } else typeof s == "number" ? s = s & 255 : typeof s == "boolean" && (s = Number(s));
    if (a < 0 || this.length < a || this.length < m)
      throw new RangeError("Out of range index");
    if (m <= a)
      return this;
    a = a >>> 0, m = m === void 0 ? this.length : m >>> 0, s || (s = 0);
    let A;
    if (typeof s == "number")
      for (A = a; A < m; ++A)
        this[A] = s;
    else {
      const k = l.isBuffer(s) ? s : l.from(s, v), z = k.length;
      if (z === 0)
        throw new TypeError('The value "' + s + '" is invalid for argument "value"');
      for (A = 0; A < m - a; ++A)
        this[A + a] = k[A % z];
    }
    return this;
  };
  const Oe = {};
  function st(u, s, a) {
    Oe[u] = class extends a {
      constructor() {
        super(), Object.defineProperty(this, "message", {
          value: s.apply(this, arguments),
          writable: !0,
          configurable: !0
        }), this.name = `${this.name} [${u}]`, this.stack, delete this.name;
      }
      get code() {
        return u;
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
        return `${this.name} [${u}]: ${this.message}`;
      }
    };
  }
  st(
    "ERR_BUFFER_OUT_OF_BOUNDS",
    function(u) {
      return u ? `${u} is outside of buffer bounds` : "Attempt to access memory outside buffer bounds";
    },
    RangeError
  ), st(
    "ERR_INVALID_ARG_TYPE",
    function(u, s) {
      return `The "${u}" argument must be of type number. Received type ${typeof s}`;
    },
    TypeError
  ), st(
    "ERR_OUT_OF_RANGE",
    function(u, s, a) {
      let m = `The value of "${u}" is out of range.`, v = a;
      return Number.isInteger(a) && Math.abs(a) > 2 ** 32 ? v = Ft(String(a)) : typeof a == "bigint" && (v = String(a), (a > BigInt(2) ** BigInt(32) || a < -(BigInt(2) ** BigInt(32))) && (v = Ft(v)), v += "n"), m += ` It must be ${s}. Received ${v}`, m;
    },
    RangeError
  );
  function Ft(u) {
    let s = "", a = u.length;
    const m = u[0] === "-" ? 1 : 0;
    for (; a >= m + 4; a -= 3)
      s = `_${u.slice(a - 3, a)}${s}`;
    return `${u.slice(0, a)}${s}`;
  }
  function er(u, s, a) {
    Me(s, "offset"), (u[s] === void 0 || u[s + a] === void 0) && ze(s, u.length - (a + 1));
  }
  function Nt(u, s, a, m, v, A) {
    if (u > a || u < s) {
      const k = typeof s == "bigint" ? "n" : "";
      let z;
      throw s === 0 || s === BigInt(0) ? z = `>= 0${k} and < 2${k} ** ${(A + 1) * 8}${k}` : z = `>= -(2${k} ** ${(A + 1) * 8 - 1}${k}) and < 2 ** ${(A + 1) * 8 - 1}${k}`, new Oe.ERR_OUT_OF_RANGE("value", z, u);
    }
    er(m, v, A);
  }
  function Me(u, s) {
    if (typeof u != "number")
      throw new Oe.ERR_INVALID_ARG_TYPE(s, "number", u);
  }
  function ze(u, s, a) {
    throw Math.floor(u) !== u ? (Me(u, a), new Oe.ERR_OUT_OF_RANGE("offset", "an integer", u)) : s < 0 ? new Oe.ERR_BUFFER_OUT_OF_BOUNDS() : new Oe.ERR_OUT_OF_RANGE(
      "offset",
      `>= 0 and <= ${s}`,
      u
    );
  }
  const tr = /[^+/0-9A-Za-z-_]/g;
  function rr(u) {
    if (u = u.split("=")[0], u = u.trim().replace(tr, ""), u.length < 2) return "";
    for (; u.length % 4 !== 0; )
      u = u + "=";
    return u;
  }
  function D(u, s) {
    s = s || 1 / 0;
    let a;
    const m = u.length;
    let v = null;
    const A = [];
    for (let k = 0; k < m; ++k) {
      if (a = u.charCodeAt(k), a > 55295 && a < 57344) {
        if (!v) {
          if (a > 56319) {
            (s -= 3) > -1 && A.push(239, 191, 189);
            continue;
          } else if (k + 1 === m) {
            (s -= 3) > -1 && A.push(239, 191, 189);
            continue;
          }
          v = a;
          continue;
        }
        if (a < 56320) {
          (s -= 3) > -1 && A.push(239, 191, 189), v = a;
          continue;
        }
        a = (v - 55296 << 10 | a - 56320) + 65536;
      } else v && (s -= 3) > -1 && A.push(239, 191, 189);
      if (v = null, a < 128) {
        if ((s -= 1) < 0) break;
        A.push(a);
      } else if (a < 2048) {
        if ((s -= 2) < 0) break;
        A.push(
          a >> 6 | 192,
          a & 63 | 128
        );
      } else if (a < 65536) {
        if ((s -= 3) < 0) break;
        A.push(
          a >> 12 | 224,
          a >> 6 & 63 | 128,
          a & 63 | 128
        );
      } else if (a < 1114112) {
        if ((s -= 4) < 0) break;
        A.push(
          a >> 18 | 240,
          a >> 12 & 63 | 128,
          a >> 6 & 63 | 128,
          a & 63 | 128
        );
      } else
        throw new Error("Invalid code point");
    }
    return A;
  }
  function Q(u) {
    const s = [];
    for (let a = 0; a < u.length; ++a)
      s.push(u.charCodeAt(a) & 255);
    return s;
  }
  function ye(u, s) {
    let a, m, v;
    const A = [];
    for (let k = 0; k < u.length && !((s -= 2) < 0); ++k)
      a = u.charCodeAt(k), m = a >> 8, v = a % 256, A.push(v), A.push(m);
    return A;
  }
  function Pe(u) {
    return e.toByteArray(rr(u));
  }
  function Ot(u, s, a, m) {
    let v;
    for (v = 0; v < m && !(v + a >= s.length || v >= u.length); ++v)
      s[v + a] = u[v];
    return v;
  }
  function _e(u, s) {
    return u instanceof s || u != null && u.constructor != null && u.constructor.name != null && u.constructor.name === s.name;
  }
  function nr(u) {
    return u !== u;
  }
  const Zn = function() {
    const u = "0123456789abcdef", s = new Array(256);
    for (let a = 0; a < 16; ++a) {
      const m = a * 16;
      for (let v = 0; v < 16; ++v)
        s[m + v] = u[a] + u[v];
    }
    return s;
  }();
  function Ue(u) {
    return typeof BigInt > "u" ? Xn : u;
  }
  function Xn() {
    throw new Error("BigInt not supported");
  }
})(mo);
const No = { executeMutation: "Execute Mutation" }, fn = { rangeKeyRequired: "Range key is required", rangeKeyOptional: "Range key is optional for delete operations" }, pn = { label: "Range Key", backgroundColor: "bg-blue-50" };
function ll({ onResult: t }) {
  const e = te(Bt);
  te((C) => C.auth);
  const [r, i] = B(""), [o, d] = B({}), [h, l] = B("Insert"), [f, p] = B(null), [x, g] = B(""), [N, T] = B({}), b = (C) => {
    i(C), d({}), l("Insert"), g("");
  }, E = (C, I) => {
    d((O) => ({ ...O, [C]: I }));
  }, y = async (C) => {
    if (C.preventDefault(), !r) return;
    const I = e.find((K) => K.name === r), O = h ? Cn[h] || h.toLowerCase() : "";
    if (!O)
      return;
    let U;
    pt(I) ? U = ba(I, h, x, o) : U = {
      type: "mutation",
      schema: r,
      mutation_type: O,
      fields_and_values: h === "Delete" ? {} : o,
      key_value: { hash: null, range: null }
    };
    try {
      const K = await Mr.executeMutation(U);
      if (!K.success)
        throw new Error(K.error || "Mutation failed");
      const V = K;
      p(V), t(V), V.success && (d({}), g(""));
    } catch (K) {
      const V = { error: `Network error: ${K.message}`, details: K };
      p(V), t(V);
    }
  }, w = r ? e.find((C) => C.name === r) : null, S = w ? pt(w) : !1, _ = w ? et(w) : null, F = !w || !Array.isArray(w.fields) ? {} : (S ? w.fields.filter((I) => I !== _) : w.fields).reduce((I, O) => (I[O] = {}, I), {}), P = !r || !h || h !== "Delete" && Object.keys(o).length === 0 || S && h !== "Delete" && !x.trim();
  return /* @__PURE__ */ c("div", { className: "p-6", children: [
    /* @__PURE__ */ c("form", { onSubmit: y, className: "space-y-6", children: [
      /* @__PURE__ */ n(
        lo,
        {
          selectedSchema: r,
          mutationType: h,
          onSchemaChange: b,
          onTypeChange: l
        }
      ),
      r && S && /* @__PURE__ */ c("div", { className: `${pn.backgroundColor} rounded-lg p-4`, children: [
        /* @__PURE__ */ n("h3", { className: "text-lg font-medium text-gray-900 mb-4", children: "Range Schema Configuration" }),
        /* @__PURE__ */ n(
          Et,
          {
            name: "rangeKey",
            label: `${_} (${pn.label})`,
            value: x,
            onChange: g,
            placeholder: `Enter ${_} value`,
            required: h !== "Delete",
            error: N.rangeKey,
            helpText: h !== "Delete" ? fn.rangeKeyRequired : fn.rangeKeyOptional,
            debounced: !0
          }
        )
      ] }),
      r && /* @__PURE__ */ n(
        co,
        {
          fields: F,
          mutationType: h,
          mutationData: o,
          onFieldChange: E,
          isRangeSchema: S
        }
      ),
      /* @__PURE__ */ n("div", { className: "flex justify-end pt-4", children: /* @__PURE__ */ n(
        "button",
        {
          type: "submit",
          className: `inline-flex items-center px-4 py-2 border border-transparent rounded-md shadow-sm text-sm font-medium text-white ${P ? "bg-gray-300 cursor-not-allowed" : "bg-primary hover:bg-primary/90 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-primary"}`,
          disabled: P,
          children: No.executeMutation
        }
      ) })
    ] }),
    /* @__PURE__ */ n(uo, { result: f })
  ] });
}
function cl({ onResult: t }) {
  const [e, r] = B(""), [i, o] = B(!0), [d, h] = B(0), [l, f] = B("default"), [p, x] = B(!1), [g, N] = B(null);
  de(() => {
    T();
  }, []);
  const T = async () => {
    try {
      const w = await Je.getStatus();
      w.success && N(w.data);
    } catch (w) {
      console.error("Failed to fetch ingestion status:", w);
    }
  }, b = async () => {
    x(!0), t(null);
    try {
      const w = JSON.parse(e), S = crypto.randomUUID(), _ = {
        autoExecute: i,
        trustDistance: d,
        pubKey: l,
        progressId: S
      }, R = await Je.processIngestion(w, _);
      R.success ? R.data.progress_id ? (console.log("🟢 IngestionTab: Dispatching ingestion-started event", R.data.progress_id), window.dispatchEvent(new CustomEvent("ingestion-started", {
        detail: { progressId: R.data.progress_id }
      })), r(""), x(!1)) : (t(R.data), r(""), x(!1)) : (t({
        success: !1,
        error: "Failed to process ingestion"
      }), x(!1));
    } catch (w) {
      t({
        success: !1,
        error: w.message || "Failed to process ingestion"
      }), x(!1);
    }
  }, E = () => {
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
    ], S = [
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
    ], _ = [
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
    ], R = [];
    for (let F = 1; F <= 100; F++) {
      const P = w[Math.floor(Math.random() * w.length)], C = S[Math.floor(Math.random() * S.length)], I = _[Math.floor(Math.random() * _.length)], O = /* @__PURE__ */ new Date(), U = new Date(O.getTime() - 6 * 30 * 24 * 60 * 60 * 1e3), K = U.getTime() + Math.random() * (O.getTime() - U.getTime()), V = new Date(K).toISOString().split("T")[0], H = [
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
      ], G = H[Math.floor(Math.random() * H.length)], L = [
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
      ], j = L[Math.floor(Math.random() * L.length)];
      R.push({
        title: G,
        content: j,
        author: P,
        publish_date: V,
        tags: I
      });
    }
    return R;
  }, y = (w) => {
    const S = {
      blogposts: E(),
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
    r(JSON.stringify(S[w], null, 2));
  };
  return /* @__PURE__ */ c("div", { className: "space-y-4", children: [
    g && /* @__PURE__ */ n("div", { className: "bg-white p-3 rounded-lg shadow-sm border border-gray-200", children: /* @__PURE__ */ c("div", { className: "flex items-center gap-4 text-sm", children: [
      /* @__PURE__ */ n("span", { className: `px-2 py-1 rounded text-xs font-medium ${g.enabled && g.configured ? "bg-green-100 text-green-800" : "bg-red-100 text-red-800"}`, children: g.enabled && g.configured ? "Ready" : "Not Configured" }),
      /* @__PURE__ */ c("span", { className: "text-gray-600", children: [
        g.provider,
        " · ",
        g.model
      ] }),
      /* @__PURE__ */ n("span", { className: "text-xs text-gray-500", children: "Configure AI settings using the Settings button in the header" })
    ] }) }),
    /* @__PURE__ */ c("div", { className: "bg-white p-4 rounded-lg shadow", children: [
      /* @__PURE__ */ c("div", { className: "flex items-center justify-between mb-3", children: [
        /* @__PURE__ */ n("h3", { className: "text-lg font-medium text-gray-900", children: "JSON Data" }),
        /* @__PURE__ */ c("div", { className: "flex gap-2", children: [
          /* @__PURE__ */ n(
            "button",
            {
              onClick: () => y("blogposts"),
              className: "px-2 py-1 bg-green-50 text-green-700 rounded text-xs hover:bg-green-100",
              children: "Blog Posts (100)"
            }
          ),
          /* @__PURE__ */ n(
            "button",
            {
              onClick: () => y("twitter"),
              className: "px-2 py-1 bg-blue-50 text-blue-700 rounded text-xs hover:bg-blue-100",
              children: "Twitter"
            }
          ),
          /* @__PURE__ */ n(
            "button",
            {
              onClick: () => y("instagram"),
              className: "px-2 py-1 bg-pink-50 text-pink-700 rounded text-xs hover:bg-pink-100",
              children: "Instagram"
            }
          ),
          /* @__PURE__ */ n(
            "button",
            {
              onClick: () => y("linkedin"),
              className: "px-2 py-1 bg-indigo-50 text-indigo-700 rounded text-xs hover:bg-indigo-100",
              children: "LinkedIn"
            }
          ),
          /* @__PURE__ */ n(
            "button",
            {
              onClick: () => y("tiktok"),
              className: "px-2 py-1 bg-purple-50 text-purple-700 rounded text-xs hover:bg-purple-100",
              children: "TikTok"
            }
          )
        ] })
      ] }),
      /* @__PURE__ */ n(
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
    /* @__PURE__ */ n("div", { className: "bg-white p-4 rounded-lg shadow", children: /* @__PURE__ */ c("div", { className: "flex items-center justify-between", children: [
      /* @__PURE__ */ c("div", { className: "flex items-center gap-4", children: [
        /* @__PURE__ */ c("label", { className: "flex items-center gap-2 text-sm", children: [
          /* @__PURE__ */ n(
            "input",
            {
              type: "checkbox",
              checked: i,
              onChange: (w) => o(w.target.checked),
              className: "rounded"
            }
          ),
          /* @__PURE__ */ n("span", { className: "text-gray-700", children: "Auto-execute mutations" })
        ] }),
        /* @__PURE__ */ n("span", { className: "text-xs text-gray-500", children: "AI will analyze and automatically map data to schemas" })
      ] }),
      /* @__PURE__ */ n(
        "button",
        {
          onClick: b,
          disabled: p || !e.trim(),
          className: `px-6 py-2.5 rounded font-medium transition-colors ${p || !e.trim() ? "bg-gray-300 text-gray-500 cursor-not-allowed" : "bg-blue-600 text-white hover:bg-blue-700"}`,
          children: p ? "Processing..." : "Process Data"
        }
      )
    ] }) })
  ] });
}
function dl({ onResult: t }) {
  const [e, r] = B(!1), [i, o] = B(null), [d, h] = B(!0), [l, f] = B(0), [p, x] = B("default"), [g, N] = B(!1), [T, b] = B(null), [E, y] = B(null), [w, S] = B(null), [_, R] = B(!1), [F, P] = B("");
  de(() => {
    C();
  }, []), de(() => {
    if (!w) return;
    const L = async () => {
      try {
        const J = await Je.getProgress(w);
        J.success && J.data && (y(J.data), J.data.is_complete && (N(!1), S(null), J.data.results ? t({
          success: !0,
          data: {
            schema_used: J.data.results.schema_name,
            new_schema_created: J.data.results.new_schema_created,
            mutations_generated: J.data.results.mutations_generated,
            mutations_executed: J.data.results.mutations_executed
          }
        }) : J.data.error_message && t({
          success: !1,
          error: J.data.error_message
        })));
      } catch (J) {
        console.error("Failed to fetch progress:", J);
      }
    };
    L();
    const j = setInterval(L, 200);
    return () => clearInterval(j);
  }, [w, t]);
  const C = async () => {
    try {
      const L = await Je.getStatus();
      L.success && b(L.data);
    } catch (L) {
      console.error("Failed to fetch ingestion status:", L);
    }
  }, I = $((L) => {
    L.preventDefault(), L.stopPropagation(), r(!0);
  }, []), O = $((L) => {
    L.preventDefault(), L.stopPropagation(), r(!1);
  }, []), U = $((L) => {
    L.preventDefault(), L.stopPropagation();
  }, []), K = $((L) => {
    L.preventDefault(), L.stopPropagation(), r(!1);
    const j = L.dataTransfer.files;
    j && j.length > 0 && o(j[0]);
  }, []), V = $((L) => {
    const j = L.target.files;
    j && j.length > 0 && o(j[0]);
  }, []), H = async () => {
    if (_) {
      if (!F || !F.startsWith("s3://")) {
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
    N(!0);
    const L = crypto.randomUUID();
    S(L), t(null), y({
      progress_percentage: 0,
      status_message: _ ? "Processing S3 file..." : "Uploading file...",
      current_step: "ValidatingConfig",
      is_complete: !1,
      started_at: (/* @__PURE__ */ new Date()).toISOString()
    }), await new Promise((j) => setTimeout(j, 100));
    try {
      const j = new FormData();
      _ ? j.append("s3FilePath", F) : j.append("file", i), j.append("autoExecute", d.toString()), j.append("trustDistance", l.toString()), j.append("pubKey", p), j.append("progress_id", L);
      const ie = await (await fetch("/api/ingestion/upload", {
        method: "POST",
        body: j
      })).json();
      ie.success && ie.progress_id ? (S(ie.progress_id), console.log("🟢 FileUploadTab: Dispatching ingestion-started event", ie.progress_id), window.dispatchEvent(new CustomEvent("ingestion-started", {
        detail: { progressId: ie.progress_id }
      })), console.log("🟢 FileUploadTab: Event dispatched")) : (t({
        success: !1,
        error: ie.error || "Failed to process file"
      }), N(!1), y(null));
    } catch (j) {
      t({
        success: !1,
        error: j.message || "Failed to process file"
      }), N(!1), y(null);
    }
  }, G = (L) => {
    if (L === 0) return "0 Bytes";
    const j = 1024, J = ["Bytes", "KB", "MB", "GB"], ie = Math.floor(Math.log(L) / Math.log(j));
    return Math.round(L / Math.pow(j, ie) * 100) / 100 + " " + J[ie];
  };
  return /* @__PURE__ */ c("div", { className: "space-y-4", children: [
    T && /* @__PURE__ */ n("div", { className: "bg-white p-3 rounded-lg shadow-sm border border-gray-200", children: /* @__PURE__ */ c("div", { className: "flex items-center gap-4 text-sm", children: [
      /* @__PURE__ */ n("span", { className: `px-2 py-1 rounded text-xs font-medium ${T.enabled && T.configured ? "bg-green-100 text-green-800" : "bg-red-100 text-red-800"}`, children: T.enabled && T.configured ? "Ready" : "Not Configured" }),
      /* @__PURE__ */ c("span", { className: "text-gray-600", children: [
        T.provider,
        " · ",
        T.model
      ] }),
      /* @__PURE__ */ n("span", { className: "text-xs text-gray-500", children: "Configure AI settings using the Settings button in the header" })
    ] }) }),
    E && /* @__PURE__ */ n(ki, { progress: E }),
    /* @__PURE__ */ n("div", { className: "bg-white p-4 rounded-lg shadow", children: /* @__PURE__ */ c("div", { className: "flex items-center gap-6", children: [
      /* @__PURE__ */ n("span", { className: "text-sm font-medium text-gray-700", children: "Input Mode:" }),
      /* @__PURE__ */ c("label", { className: "flex items-center gap-2 cursor-pointer", children: [
        /* @__PURE__ */ n(
          "input",
          {
            type: "radio",
            checked: !_,
            onChange: () => R(!1),
            className: "rounded"
          }
        ),
        /* @__PURE__ */ n("span", { className: "text-sm text-gray-700", children: "Upload File" })
      ] }),
      /* @__PURE__ */ c("label", { className: "flex items-center gap-2 cursor-pointer", children: [
        /* @__PURE__ */ n(
          "input",
          {
            type: "radio",
            checked: _,
            onChange: () => R(!0),
            className: "rounded"
          }
        ),
        /* @__PURE__ */ n("span", { className: "text-sm text-gray-700", children: "S3 File Path" })
      ] })
    ] }) }),
    _ ? /* @__PURE__ */ c("div", { className: "bg-white p-6 rounded-lg shadow", children: [
      /* @__PURE__ */ n("h3", { className: "text-lg font-medium text-gray-900 mb-4", children: "S3 File Path" }),
      /* @__PURE__ */ c("div", { className: "space-y-3", children: [
        /* @__PURE__ */ n("label", { className: "block text-sm font-medium text-gray-700", children: "Enter S3 file path" }),
        /* @__PURE__ */ n(
          "input",
          {
            type: "text",
            value: F,
            onChange: (L) => P(L.target.value),
            placeholder: "s3://bucket-name/path/to/file.json",
            className: "w-full px-4 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
          }
        ),
        /* @__PURE__ */ n("p", { className: "text-xs text-gray-500", children: "The file will be downloaded from S3 for processing without re-uploading" })
      ] })
    ] }) : /* @__PURE__ */ c("div", { className: "bg-white p-6 rounded-lg shadow", children: [
      /* @__PURE__ */ n("h3", { className: "text-lg font-medium text-gray-900 mb-4", children: "Upload File" }),
      /* @__PURE__ */ n(
        "div",
        {
          className: `border-2 border-dashed rounded-lg p-12 text-center transition-colors ${e ? "border-blue-500 bg-blue-50" : "border-gray-300 bg-gray-50 hover:bg-gray-100"}`,
          onDragEnter: I,
          onDragOver: U,
          onDragLeave: O,
          onDrop: K,
          children: /* @__PURE__ */ c("div", { className: "space-y-4", children: [
            /* @__PURE__ */ n("div", { className: "flex justify-center", children: /* @__PURE__ */ n(
              "svg",
              {
                className: "w-16 h-16 text-gray-400",
                fill: "none",
                stroke: "currentColor",
                viewBox: "0 0 24 24",
                xmlns: "http://www.w3.org/2000/svg",
                children: /* @__PURE__ */ n(
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
            i ? /* @__PURE__ */ c("div", { className: "space-y-2", children: [
              /* @__PURE__ */ n("p", { className: "text-lg font-medium text-gray-900", children: i.name }),
              /* @__PURE__ */ n("p", { className: "text-sm text-gray-500", children: G(i.size) }),
              /* @__PURE__ */ n(
                "button",
                {
                  onClick: () => o(null),
                  className: "text-sm text-blue-600 hover:text-blue-700 underline",
                  children: "Remove file"
                }
              )
            ] }) : /* @__PURE__ */ c("div", { children: [
              /* @__PURE__ */ n("p", { className: "text-lg text-gray-700 mb-2", children: "Drag and drop a file here, or click to select" }),
              /* @__PURE__ */ n("p", { className: "text-sm text-gray-500", children: "Supported formats: PDF, DOCX, TXT, CSV, JSON, XML, and more" })
            ] }),
            /* @__PURE__ */ n(
              "input",
              {
                type: "file",
                id: "file-upload",
                className: "hidden",
                onChange: V
              }
            ),
            !i && /* @__PURE__ */ n(
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
    /* @__PURE__ */ n("div", { className: "bg-white p-4 rounded-lg shadow", children: /* @__PURE__ */ c("div", { className: "flex items-center justify-between", children: [
      /* @__PURE__ */ c("div", { className: "flex items-center gap-4", children: [
        /* @__PURE__ */ c("label", { className: "flex items-center gap-2 text-sm", children: [
          /* @__PURE__ */ n(
            "input",
            {
              type: "checkbox",
              checked: d,
              onChange: (L) => h(L.target.checked),
              className: "rounded"
            }
          ),
          /* @__PURE__ */ n("span", { className: "text-gray-700", children: "Auto-execute mutations" })
        ] }),
        /* @__PURE__ */ n("span", { className: "text-xs text-gray-500", children: "File will be converted to JSON and processed by AI" })
      ] }),
      /* @__PURE__ */ n(
        "button",
        {
          onClick: H,
          disabled: g || !_ && !i || _ && !F,
          className: `px-6 py-2.5 rounded font-medium transition-colors ${g || !_ && !i || _ && !F ? "bg-gray-300 text-gray-500 cursor-not-allowed" : "bg-blue-600 text-white hover:bg-blue-700"}`,
          children: g ? "Processing..." : _ ? "Process S3 File" : "Upload & Process"
        }
      )
    ] }) }),
    /* @__PURE__ */ n("div", { className: "bg-blue-50 border border-blue-200 rounded-lg p-4", children: /* @__PURE__ */ c("div", { className: "flex items-start gap-3", children: [
      /* @__PURE__ */ n(
        "svg",
        {
          className: "w-6 h-6 text-blue-600 flex-shrink-0 mt-0.5",
          fill: "none",
          stroke: "currentColor",
          viewBox: "0 0 24 24",
          children: /* @__PURE__ */ n(
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
      /* @__PURE__ */ c("div", { className: "text-sm text-blue-800", children: [
        /* @__PURE__ */ n("p", { className: "font-medium mb-1", children: "How it works:" }),
        /* @__PURE__ */ c("ol", { className: "list-decimal list-inside space-y-1", children: [
          /* @__PURE__ */ n("li", { children: _ ? "Provide an S3 file path (files already in S3 are not re-uploaded)" : "Upload any file type (PDFs, documents, spreadsheets, etc.)" }),
          /* @__PURE__ */ n("li", { children: "File is automatically converted to JSON using AI" }),
          /* @__PURE__ */ n("li", { children: "AI analyzes the JSON and maps it to appropriate schemas" }),
          /* @__PURE__ */ n("li", { children: "Data is stored in the database with the file location tracked" })
        ] })
      ] })
    ] }) })
  ] });
}
function So() {
  const t = Ge(), e = te(Bt), r = te(gt), i = te(Lr), o = te(Ln), d = te(ka), h = $(async () => {
    t(Ie({ forceRefresh: !0 }));
  }, [t]), l = $((p) => r.find((x) => x.name === p) || null, [r]), f = $((p) => {
    const x = l(p);
    return x ? Fn(x.state) === Fe.APPROVED : !1;
  }, [l]);
  return de(() => {
    d.isValid || (console.log("🟡 useApprovedSchemas: Cache invalid, fetching schemas"), t(Ie()));
  }, [t]), {
    approvedSchemas: e,
    isLoading: i,
    error: o,
    refetch: h,
    getSchemaByName: l,
    isSchemaApproved: f,
    // Additional utility for components that need all schemas for display
    allSchemas: r
  };
}
function Eo({ r: t }) {
  var e, r;
  return /* @__PURE__ */ c("tr", { className: "border-t", children: [
    /* @__PURE__ */ n("td", { className: "px-2 py-1 text-xs text-gray-600", children: ((e = t.key_value) == null ? void 0 : e.hash) ?? "" }),
    /* @__PURE__ */ n("td", { className: "px-2 py-1 text-xs text-gray-600", children: ((r = t.key_value) == null ? void 0 : r.range) ?? "" }),
    /* @__PURE__ */ n("td", { className: "px-2 py-1 text-xs font-mono text-gray-800", children: t.schema_name }),
    /* @__PURE__ */ n("td", { className: "px-2 py-1 text-xs text-gray-800", children: t.field }),
    /* @__PURE__ */ n("td", { className: "px-2 py-1 text-xs text-gray-800 whitespace-pre-wrap break-words", children: Ao(t.value) })
  ] });
}
function Ao(t) {
  if (t == null) return "";
  if (typeof t == "string") return t;
  try {
    return JSON.stringify(t);
  } catch {
    return String(t);
  }
}
function ul({ onResult: t }) {
  const { approvedSchemas: e, isLoading: r, refetch: i } = So(), [o, d] = B(""), [h, l] = B(!1), [f, p] = B([]), [x, g] = B(null), [N, T] = B(() => /* @__PURE__ */ new Set()), [b, E] = B(() => /* @__PURE__ */ new Map());
  de(() => {
    i();
  }, [i]);
  const y = $(async () => {
    l(!0), g(null);
    try {
      const C = await ai.search(o);
      C.success ? (p(C.data || []), t({ success: !0, data: C.data || [] })) : (g(C.error || "Search failed"), t({ error: C.error || "Search failed", status: C.status }));
    } catch (C) {
      g(C.message || "Network error"), t({ error: C.message || "Network error" });
    } finally {
      l(!1);
    }
  }, [o, t]), w = $((C) => {
    if (!C) return [];
    const I = C.fields;
    return Array.isArray(I) ? I.slice() : I && typeof I == "object" ? Object.keys(I) : [];
  }, []), S = Z(() => {
    const C = /* @__PURE__ */ new Map();
    return (e || []).forEach((I) => C.set(I.name, I)), C;
  }, [e]), _ = $((C, I) => {
    const O = (I == null ? void 0 : I.hash) ?? "", U = (I == null ? void 0 : I.range) ?? "";
    return `${C}|${O}|${U}`;
  }, []), R = $((C) => {
    const I = C == null ? void 0 : C.hash, O = C == null ? void 0 : C.range;
    if (I && O) return Di(I, O);
    if (I) return Tt(I);
    if (O) return Tt(O);
  }, []), F = $(async (C, I) => {
    const O = S.get(C), U = w(O), K = R(I), V = { schema_name: C, fields: U };
    K && (V.filter = K);
    const H = await Mr.executeQuery(V);
    if (!H.success)
      throw new Error(H.error || "Query failed");
    const G = Array.isArray(H.data) ? H.data : [], L = G.find((j) => {
      var X, oe;
      const J = ((X = j == null ? void 0 : j.key) == null ? void 0 : X.hash) ?? null, ie = ((oe = j == null ? void 0 : j.key) == null ? void 0 : oe.range) ?? null, tt = (I == null ? void 0 : I.hash) ?? null, rt = (I == null ? void 0 : I.range) ?? null;
      return String(J || "") === String(tt || "") && String(ie || "") === String(rt || "");
    }) || G[0];
    return (L == null ? void 0 : L.fields) || (L && typeof L == "object" ? L : {});
  }, [S, w, R]), P = $(async () => {
    const C = /* @__PURE__ */ new Map();
    for (const U of f) {
      const K = _(U.schema_name, U.key_value);
      C.has(K) || C.set(K, U);
    }
    const I = Array.from(C.values()), O = new Map(b);
    await Promise.all(I.map(async (U) => {
      const K = _(U.schema_name, U.key_value);
      if (!O.has(K))
        try {
          const V = await F(U.schema_name, U.key_value);
          O.set(K, V);
        } catch {
          O.set(K, {});
        }
    })), E(O);
  }, [f, b, _, F]);
  return de(() => {
    f.length > 0 && P().catch(() => {
    });
  }, [f, P]), /* @__PURE__ */ c("div", { className: "p-6 space-y-4", children: [
    /* @__PURE__ */ c("div", { className: "bg-white p-4 rounded-lg shadow", children: [
      /* @__PURE__ */ c("div", { className: "mb-3", children: [
        /* @__PURE__ */ n("h3", { className: "text-lg font-medium text-gray-900", children: "Native Index Search" }),
        /* @__PURE__ */ n("p", { className: "text-xs text-gray-500", children: "Search the database-native word index across all approved schemas." })
      ] }),
      /* @__PURE__ */ c("div", { className: "flex gap-2 items-center", children: [
        /* @__PURE__ */ n(
          "input",
          {
            type: "text",
            value: o,
            onChange: (C) => d(C.target.value),
            placeholder: "Enter search term (e.g. jennifer)",
            className: "flex-1 px-3 py-2 border rounded-md text-sm"
          }
        ),
        /* @__PURE__ */ n(
          "button",
          {
            onClick: y,
            disabled: h || !o.trim(),
            className: `px-4 py-2 rounded text-sm ${h || !o.trim() ? "bg-gray-300 text-gray-600" : "bg-blue-600 text-white hover:bg-blue-700"}`,
            children: h ? "Searching..." : "Search"
          }
        )
      ] })
    ] }),
    /* @__PURE__ */ c("div", { className: "bg-white p-4 rounded-lg shadow", children: [
      /* @__PURE__ */ c("div", { className: "mb-2 flex items-center justify-between", children: [
        /* @__PURE__ */ n("h4", { className: "text-md font-medium text-gray-900", children: "Search Results" }),
        /* @__PURE__ */ c("div", { className: "flex items-center gap-3", children: [
          /* @__PURE__ */ c("span", { className: "text-xs text-gray-500", children: [
            f.length,
            " matches"
          ] }),
          f.length > 0 && /* @__PURE__ */ n(
            "button",
            {
              type: "button",
              className: "text-xs px-2 py-1 rounded border border-gray-300 hover:bg-gray-100",
              onClick: () => P(),
              children: "Refresh Details"
            }
          )
        ] })
      ] }),
      x && /* @__PURE__ */ n("div", { className: "mb-2 p-2 bg-red-50 border border-red-200 text-xs text-red-700 rounded", children: x }),
      /* @__PURE__ */ n("div", { className: "overflow-auto max-h-[450px]", children: /* @__PURE__ */ c("table", { className: "min-w-full text-left text-xs", children: [
        /* @__PURE__ */ n("thead", { children: /* @__PURE__ */ c("tr", { className: "text-gray-500", children: [
          /* @__PURE__ */ n("th", { className: "px-2 py-1", children: "Hash" }),
          /* @__PURE__ */ n("th", { className: "px-2 py-1", children: "Range" }),
          /* @__PURE__ */ n("th", { className: "px-2 py-1", children: "Schema" }),
          /* @__PURE__ */ n("th", { className: "px-2 py-1", children: "Field" }),
          /* @__PURE__ */ n("th", { className: "px-2 py-1", children: "Value" }),
          /* @__PURE__ */ n("th", { className: "px-2 py-1" })
        ] }) }),
        /* @__PURE__ */ c("tbody", { children: [
          f.map((C, I) => {
            const O = _(C.schema_name, C.key_value), U = N.has(O), K = b.get(O);
            return /* @__PURE__ */ c(Rt, { children: [
              /* @__PURE__ */ n(Eo, { r: C }, `${O}-row`),
              /* @__PURE__ */ c("tr", { className: "border-b", children: [
                /* @__PURE__ */ n("td", { colSpan: 5 }),
                /* @__PURE__ */ n("td", { className: "px-2 py-1 text-right", children: /* @__PURE__ */ n(
                  "button",
                  {
                    type: "button",
                    className: "text-xs px-2 py-1 rounded border border-gray-300 hover:bg-gray-100",
                    onClick: async () => {
                      const V = new Set(N);
                      if (V.has(O) ? V.delete(O) : V.add(O), T(V), !b.has(O))
                        try {
                          const H = await F(C.schema_name, C.key_value);
                          E((G) => new Map(G).set(O, H));
                        } catch {
                        }
                    },
                    children: U ? "Hide Data" : "Show Data"
                  }
                ) })
              ] }, `${O}-actions`),
              U && /* @__PURE__ */ n("tr", { children: /* @__PURE__ */ n("td", { colSpan: 6, className: "px-2 pb-3", children: /* @__PURE__ */ n("div", { className: "ml-2 bg-gray-50 border rounded", children: /* @__PURE__ */ n(FieldsTable, { fields: K || {} }) }) }) }, `${O}-details`)
            ] });
          }),
          f.length === 0 && /* @__PURE__ */ n("tr", { children: /* @__PURE__ */ n("td", { colSpan: 5, className: "px-2 py-3 text-center text-gray-500", children: "No results" }) })
        ] })
      ] }) })
    ] })
  ] });
}
function hl({
  state: t,
  isRangeSchema: e = !1,
  size: r = "md",
  className: i = "",
  showTooltip: o = !0
}) {
  const d = {
    sm: "px-1.5 py-0.5 text-xs",
    md: "px-2.5 py-0.5 text-xs",
    lg: "px-3 py-1 text-sm"
  }, h = () => Zr[t] || Zr.available, l = () => ({
    approved: "Approved",
    available: "Available",
    blocked: "Blocked",
    pending: "Pending"
  })[t] || "Unknown", f = () => o ? Rn.schemaStates[t] || "Unknown schema state" : "", p = `
    inline-flex items-center rounded-full font-medium
    ${d[r]}
    ${h()}
    ${i}
  `.trim();
  return /* @__PURE__ */ c("div", { className: "inline-flex items-center space-x-2", children: [
    /* @__PURE__ */ n(
      "span",
      {
        className: p,
        title: f(),
        "aria-label": `Schema status: ${l()}${e ? ", Range Schema" : ""}`,
        children: l()
      }
    ),
    e && /* @__PURE__ */ n(
      "span",
      {
        className: `
            inline-flex items-center rounded-full font-medium
            ${d[r]}
            ${Xr.badgeColor}
          `,
        title: "This schema uses range-based keys for efficient querying",
        "aria-label": "Range Schema",
        children: Xr.label
      }
    )
  ] });
}
const _o = (t) => (t == null ? void 0 : t.schema_type) === "Range", To = (t) => {
  var e;
  return ((e = t == null ? void 0 : t.key) == null ? void 0 : e.range_field) || null;
};
function ml({
  children: t,
  queryState: e,
  schemas: r,
  selectedSchemaObj: i,
  isRangeSchema: o,
  rangeKey: d,
  schema: h,
  ...l
}) {
  const f = Z(() => h || (e != null && e.selectedSchema ? e.selectedSchema : (i == null ? void 0 : i.name) ?? null), [h, e == null ? void 0 : e.selectedSchema, i == null ? void 0 : i.name]), p = Z(() => i || (f && r && r[f] ? r[f] : null), [f, r, i]), x = Z(() => {
    if (r)
      return r;
    if (f && p)
      return { [f]: p };
  }, [r, f, p]), g = Z(() => typeof o == "boolean" ? o : _o(p), [o, p]), N = Z(() => d || To(p), [d, p]), T = Z(() => ({
    ...l,
    schema: f,
    queryState: e,
    schemas: x,
    selectedSchemaObj: p,
    isRangeSchema: g,
    rangeKey: N
  }), [l, f, e, x, p, g, N]);
  let b;
  try {
    b = Qn(T);
  } catch (E) {
    b = {
      query: null,
      validationErrors: [E.message || "An error occurred while building the query"],
      isValid: !1,
      buildQuery: () => null,
      validateQuery: () => !1,
      error: E
    };
  }
  return typeof t == "function" ? t(b) : null;
}
process.env.NODE_ENV;
const fl = "ingestion";
export {
  wi as BackfillMonitor,
  fl as DEFAULT_TAB,
  ke as FieldWrapper,
  dl as FileUploadTab,
  Qo as FoldDbProvider,
  tl as Footer,
  el as Header,
  cl as IngestionTab,
  Ii as KeyManagementTab,
  il as LlmQueryTab,
  Xo as LogSidebar,
  nl as LoginModal,
  rl as LoginPage,
  co as MutationEditor,
  ll as MutationTab,
  ul as NativeIndexTab,
  ki as ProgressBar,
  ao as QueryActions,
  ml as QueryBuilder,
  so as QueryForm,
  oo as QueryPreview,
  al as QueryTab,
  no as RangeField,
  uo as ResultViewer,
  Wo as ResultsSection,
  lo as SchemaSelector,
  hl as SchemaStatusBadge,
  sl as SchemaTab,
  wr as SelectField,
  Zo as SettingsModal,
  Yo as StatusSection,
  mi as StructuredResults,
  Jo as TabNavigation,
  Et as TextField,
  Bi as TopologyDisplay,
  Si as TransformsTab,
  Ma as aiQueryReducer,
  ca as authReducer,
  oa as clearAuthentication,
  Oo as clearError,
  dr as fetchNodePrivateKey,
  lr as initializeSystemKey,
  Or as loginUser,
  la as logoutUser,
  cr as refreshSystemKey,
  Lo as restoreSession,
  Fa as schemaReducer,
  Fo as setError,
  Va as store,
  Do as updateSystemKey,
  Ge as useAppDispatch,
  te as useAppSelector,
  So as useApprovedSchemas,
  Ht as validatePrivateKey
};
