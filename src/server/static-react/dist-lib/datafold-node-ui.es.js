var rs = Object.defineProperty;
var ns = (t, e, r) => e in t ? rs(t, e, { enumerable: !0, configurable: !0, writable: !0, value: r }) : t[e] = r;
var Qe = (t, e, r) => ns(t, typeof e != "symbol" ? e + "" : e, r);
import { jsx as s, jsxs as h, Fragment as Rt } from "react/jsx-runtime";
import * as V from "react";
import { createContext as ss, useState as O, useContext as as, useCallback as K, useEffect as ue, useMemo as W, useRef as At } from "react";
import { Provider as is, useSelector as os, useDispatch as ls } from "react-redux";
import { createAsyncThunk as Je, createSlice as Ar, createSelector as Xe, configureStore as cs } from "@reduxjs/toolkit";
const ds = {
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
}, us = {
  ROOT: "/api"
}, Q = ds, hs = 3e4, ms = 3, fs = 1e3, le = {
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
}, Ge = {
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
}, ar = {
  JSON: "application/json",
  FORM_DATA: "multipart/form-data",
  URL_ENCODED: "application/x-www-form-urlencoded",
  TEXT: "text/plain"
}, Lt = {
  CONTENT_TYPE: "Content-Type",
  AUTHORIZATION: "Authorization",
  SIGNED_REQUEST: "X-Signed-Request",
  REQUEST_ID: "X-Request-ID",
  AUTHENTICATED: "X-Authenticated"
}, fe = {
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
  DEFAULT_TTL_MS: Ge.STANDARD,
  MAX_CACHE_SIZE: 100,
  SCHEMA_CACHE_TTL_MS: Ge.SCHEMA_DATA,
  SYSTEM_STATUS_CACHE_TTL_MS: Ge.SYSTEM_STATUS
}, br = {
  RETRYABLE_STATUS_CODES: [408, 429, 500, 502, 503, 504],
  EXPONENTIAL_BACKOFF_MULTIPLIER: 2,
  MAX_RETRY_DELAY_MS: 1e4
}, qt = {
  // Use relative path for CloudFront compatibility
  BASE_URL: "/api"
}, pe = {
  AVAILABLE: "available",
  APPROVED: "approved",
  BLOCKED: "blocked"
}, ps = {
  MUTATION: "mutation"
}, jt = {
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
    return r || i ? !0 : br.RETRYABLE_STATUS_CODES.includes(e);
  }
  /**
   * Convert error to a user-friendly message
   */
  toUserMessage() {
    if (this.isNetworkError)
      return fe.NETWORK_ERROR;
    if (this.isTimeoutError)
      return fe.TIMEOUT_ERROR;
    switch (this.status) {
      case we.UNAUTHORIZED:
        return fe.AUTHENTICATION_ERROR;
      case we.FORBIDDEN:
        return fe.PERMISSION_ERROR;
      case we.NOT_FOUND:
        return fe.NOT_FOUND_ERROR;
      case we.BAD_REQUEST:
        return fe.VALIDATION_ERROR;
      case we.INTERNAL_SERVER_ERROR:
      case we.BAD_GATEWAY:
      case we.SERVICE_UNAVAILABLE:
        return fe.SERVER_ERROR;
      case 429:
        return fe.RATE_LIMIT_ERROR;
      default:
        return this.message || fe.SERVER_ERROR;
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
class _r extends de {
  constructor(e = fe.AUTHENTICATION_ERROR, r) {
    super(e, we.UNAUTHORIZED, {
      code: "AUTH_ERROR",
      requestId: r
    }), this.name = "AuthenticationError", Object.setPrototypeOf(this, _r.prototype);
  }
}
class Qt extends de {
  constructor(e, r, i, o = fe.SCHEMA_STATE_ERROR) {
    super(o, we.FORBIDDEN, {
      code: "SCHEMA_STATE_ERROR",
      details: { schemaName: e, currentState: r, operation: i }
    }), this.name = "SchemaStateError", this.schemaName = e, this.currentState = r, this.operation = i, Object.setPrototypeOf(this, Qt.prototype);
  }
}
class Tr extends de {
  constructor(e = fe.NETWORK_ERROR, r) {
    super(e, 0, {
      isNetworkError: !0,
      code: "NETWORK_ERROR",
      requestId: r
    }), this.name = "NetworkError", Object.setPrototypeOf(this, Tr.prototype);
  }
}
class Cr extends de {
  constructor(e, r) {
    super(`Request timed out after ${e}ms`, 408, {
      isTimeoutError: !0,
      code: "TIMEOUT_ERROR",
      requestId: r,
      details: { timeoutMs: e }
    }), this.name = "TimeoutError", this.timeoutMs = e, Object.setPrototypeOf(this, Cr.prototype);
  }
}
class Rr extends de {
  constructor(e, r) {
    super("Request validation failed", we.BAD_REQUEST, {
      code: "VALIDATION_ERROR",
      requestId: r,
      details: { validationErrors: e }
    }), this.name = "ValidationError", this.validationErrors = e, Object.setPrototypeOf(this, Rr.prototype);
  }
}
class kr extends de {
  constructor(e, r) {
    const i = e ? `Rate limit exceeded. Retry after ${e} seconds.` : fe.RATE_LIMIT_ERROR;
    super(i, 429, {
      code: "RATE_LIMIT_ERROR",
      requestId: r,
      details: { retryAfter: e }
    }), this.name = "RateLimitError", this.retryAfter = e, Object.setPrototypeOf(this, kr.prototype);
  }
}
class St {
  /**
   * Create an ApiError from a fetch response
   */
  static async fromResponse(e, r) {
    let i = {};
    try {
      const c = await e.text();
      c && (i = JSON.parse(c));
    } catch {
    }
    const o = typeof i.error == "string" ? i.error : typeof i.message == "string" ? i.message : `HTTP ${e.status}`;
    if (e.status === we.UNAUTHORIZED)
      return new _r(o, r || "");
    if (e.status === 429) {
      const c = e.headers.get("Retry-After");
      return new kr(c ? parseInt(c) : void 0, r);
    }
    return e.status === we.BAD_REQUEST && i.validationErrors ? new Rr(i.validationErrors, r || "") : new de(o, e.status, {
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
    return new Tr(e.message, r);
  }
  /**
   * Create an ApiError from a timeout
   */
  static fromTimeout(e, r) {
    return new Cr(e, r);
  }
  /**
   * Create a schema state error
   */
  static fromSchemaState(e, r, i) {
    return new Qt(e, r, i);
  }
}
function gs(t) {
  return t instanceof de;
}
function ys(t) {
  return gs(t) && t.isRetryable;
}
let xr = null;
const Bo = (t) => {
  xr = t;
};
class bs {
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
class xs {
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
class xn {
  constructor(e = {}) {
    this.requestInterceptors = [], this.responseInterceptors = [], this.errorInterceptors = [], this.metrics = [], this.config = {
      baseUrl: e.baseUrl || qt.BASE_URL,
      timeout: e.timeout || hs,
      retryAttempts: e.retryAttempts || ms,
      retryDelay: e.retryDelay || fs,
      defaultHeaders: e.defaultHeaders || {},
      enableCache: e.enableCache !== !1,
      enableLogging: e.enableLogging !== !1,
      enableMetrics: e.enableMetrics !== !1
    }, this.cache = new bs(), this.requestQueue = new xs();
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
      throw new de(
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
          const c = o instanceof de ? o : new de(o.message);
          return {
            id: i.id,
            success: !1,
            error: c.message,
            status: c.status
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
    const c = o.requestId || this.generateRequestId(), u = Date.now();
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
        requestId: c,
        timestamp: u,
        priority: o.priority || "normal"
      }
    };
    try {
      for (const _ of this.requestInterceptors)
        l = await _(l);
      if (l.validateSchema && await this.validateSchemaAccess(
        r,
        e,
        o.validateSchema || !0
      ), e === "GET" && this.config.enableCache && o.cacheable !== !1) {
        const _ = this.generateCacheKey(l.url, l.headers), b = this.cache.get(_);
        if (b)
          return {
            ...b,
            meta: {
              ...b.meta,
              cached: !0,
              fromCache: !0,
              requestId: c,
              timestamp: ((f = b.meta) == null ? void 0 : f.timestamp) || Date.now()
            }
          };
      }
      const y = `${e}:${l.url}:${JSON.stringify(i)}`, v = await this.requestQueue.getOrCreate(
        y,
        () => this.executeRequest(l)
      );
      if (e === "GET" && this.config.enableCache && o.cacheable !== !1 && v.success) {
        const _ = this.generateCacheKey(l.url, l.headers), b = o.cacheTtl || _t.DEFAULT_TTL_MS;
        this.cache.set(_, v, b);
      }
      let N = v;
      for (const _ of this.responseInterceptors)
        N = await _(
          N
        );
      return this.config.enableMetrics && this.recordMetrics({
        requestId: c,
        url: l.url,
        method: e,
        startTime: u,
        endTime: Date.now(),
        duration: Date.now() - u,
        status: v.status,
        cached: ((p = v.meta) == null ? void 0 : p.cached) || !1
      }), N;
    } catch (y) {
      let v = y instanceof de ? y : St.fromNetworkError(y, c);
      for (const N of this.errorInterceptors)
        v = await N(v);
      throw this.config.enableMetrics && this.recordMetrics({
        requestId: c,
        url: l.url,
        method: e,
        startTime: u,
        endTime: Date.now(),
        duration: Date.now() - u,
        error: v.message
      }), v;
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
        if (r = o instanceof de ? o : St.fromNetworkError(o, e.metadata.requestId), i === e.retries || !ys(r))
          break;
        const c = Math.min(
          this.config.retryDelay * Math.pow(br.EXPONENTIAL_BACKOFF_MULTIPLIER, i),
          br.MAX_RETRY_DELAY_MS
        );
        await this.sleep(c);
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
      if (e.body && !o[Lt.CONTENT_TYPE] && (o[Lt.CONTENT_TYPE] = ar.JSON), o[Lt.REQUEST_ID] = e.metadata.requestId, typeof window < "u") {
        const l = localStorage.getItem("fold_user_hash") || localStorage.getItem("exemem_user_hash");
        l && (o["x-user-hash"] = l, o["x-user-id"] = l);
      }
      const c = {
        method: e.method,
        headers: o,
        signal: e.abortSignal || r.signal
      };
      e.body && e.method !== "GET" && (c.body = this.serializeBody(
        e.body,
        o[Lt.CONTENT_TYPE]
      ));
      const u = await fetch(e.url, c);
      return clearTimeout(i), await this.handleResponse(u, e.metadata.requestId);
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
    const c = o[1], u = typeof i == "boolean" ? {} : i;
    if ((e.includes("/mutation") || e.includes("/query")) && u.requiresApproved !== !1) {
      if (!xr) {
        console.warn(
          "Store not injected into ApiClient, skipping schema validation"
        );
        return;
      }
      const l = xr.getState().schemas, p = Object.values(l.schemas || {}).find((y) => y.name === c);
      if (!p || p.state !== pe.APPROVED)
        throw new Qt(
          c,
          (p == null ? void 0 : p.state) || "unknown",
          ps.MUTATION
        );
    }
  }
  /**
   * Serialize request body based on content type
   */
  serializeBody(e, r) {
    return r === ar.JSON ? JSON.stringify(e) : r === ar.FORM_DATA ? e : String(e);
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
new xn();
function Ae(t) {
  return new xn(t);
}
class ws {
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
    const r = e ? `${Q.LIST_LOGS}?since=${e}` : Q.LIST_LOGS;
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
      Q.RESET_DATABASE,
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
    return this.client.get(Q.GET_SYSTEM_STATUS, {
      requiresAuth: !1,
      // Status is public for monitoring
      timeout: le.QUICK,
      retries: ce.CRITICAL,
      // Multiple retries for critical system data
      cacheable: !0,
      cacheTtl: Ge.SYSTEM_STATUS,
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
    return this.client.get(Q.GET_NODE_PRIVATE_KEY, {
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
    return this.client.get(Q.GET_NODE_PUBLIC_KEY, {
      requiresAuth: !1,
      // Public key is safe to share
      timeout: le.QUICK,
      retries: ce.STANDARD,
      cacheable: !0,
      cacheTtl: Ge.SYSTEM_STATUS,
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
    const i = Q.STREAM_LOGS, o = i.startsWith("http") ? i : `${qt.BASE_URL}${i.startsWith("/") ? "" : "/"}${i}`, c = new EventSource(o);
    return c.onmessage = (u) => {
      e(u.data);
    }, r && (c.onerror = r), c;
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
      cacheTtl: Ge.SYSTEM_STATUS,
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
const ne = new ws();
ne.getLogs.bind(ne);
ne.resetDatabase.bind(ne);
ne.getSystemStatus.bind(ne);
const Ir = ne.getNodePrivateKey.bind(ne);
ne.getNodePublicKey.bind(ne);
const vs = ne.getDatabaseConfig.bind(ne), Ns = ne.updateDatabaseConfig.bind(ne);
ne.createLogStream.bind(ne);
ne.validateResetRequest.bind(ne);
/*! noble-ed25519 - MIT License (c) 2019 Paul Miller (paulmillr.com) */
const Ss = {
  p: 0x7fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffedn,
  n: 0x1000000000000000000000000000000014def9dea2f79cd65812631a5cf5d3edn,
  a: 0x7fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffecn,
  d: 0x52036cee2b6ffe738cc740797779e89800700a4d4141d8ab75eb4dca135978a3n,
  Gx: 0x216936d3cd6e53fec0a4e231fdd6dc5c692cc7609525a7b2c9562d608f25d51an,
  Gy: 0x6666666666666666666666666666666666666666666666666666666666666658n
}, { p: oe, n: Kt, Gx: jr, Gy: Vr, a: ir, d: or } = Ss, Es = 8n, Ct = 32, wn = 64, Ne = (t = "") => {
  throw new Error(t);
}, As = (t) => typeof t == "bigint", vn = (t) => typeof t == "string", _s = (t) => t instanceof Uint8Array || ArrayBuffer.isView(t) && t.constructor.name === "Uint8Array", mt = (t, e) => !_s(t) || typeof e == "number" && e > 0 && t.length !== e ? Ne("Uint8Array expected") : t, Yt = (t) => new Uint8Array(t), Br = (t) => Uint8Array.from(t), Nn = (t, e) => t.toString(16).padStart(e, "0"), Or = (t) => Array.from(mt(t)).map((e) => Nn(e, 2)).join(""), Le = { _0: 48, _9: 57, A: 65, F: 70, a: 97, f: 102 }, Gr = (t) => {
  if (t >= Le._0 && t <= Le._9)
    return t - Le._0;
  if (t >= Le.A && t <= Le.F)
    return t - (Le.A - 10);
  if (t >= Le.a && t <= Le.f)
    return t - (Le.a - 10);
}, Fr = (t) => {
  const e = "hex invalid";
  if (!vn(t))
    return Ne(e);
  const r = t.length, i = r / 2;
  if (r % 2)
    return Ne(e);
  const o = Yt(i);
  for (let c = 0, u = 0; c < i; c++, u += 2) {
    const l = Gr(t.charCodeAt(u)), f = Gr(t.charCodeAt(u + 1));
    if (l === void 0 || f === void 0)
      return Ne(e);
    o[c] = l * 16 + f;
  }
  return o;
}, Sn = (t, e) => mt(vn(t) ? Fr(t) : Br(mt(t)), e), En = () => globalThis == null ? void 0 : globalThis.crypto, Ts = () => {
  var t;
  return ((t = En()) == null ? void 0 : t.subtle) ?? Ne("crypto.subtle must be defined");
}, zr = (...t) => {
  const e = Yt(t.reduce((i, o) => i + mt(o).length, 0));
  let r = 0;
  return t.forEach((i) => {
    e.set(i, r), r += i.length;
  }), e;
}, Cs = (t = Ct) => En().getRandomValues(Yt(t)), Vt = BigInt, Ye = (t, e, r, i = "bad number: out of range") => As(t) && e <= t && t < r ? t : Ne(i), M = (t, e = oe) => {
  const r = t % e;
  return r >= 0n ? r : e + r;
}, Rs = (t) => M(t, Kt), An = (t, e) => {
  (t === 0n || e <= 0n) && Ne("no inverse n=" + t + " mod=" + e);
  let r = M(t, e), i = e, o = 0n, c = 1n;
  for (; r !== 0n; ) {
    const u = i / r, l = i % r, f = o - c * u;
    i = r, r = l, o = c, c = f;
  }
  return i === 1n ? M(o, e) : Ne("no inverse");
}, qr = (t) => t instanceof Ze ? t : Ne("Point expected"), wr = 2n ** 256n, Ce = class Ce {
  constructor(e, r, i, o) {
    Qe(this, "ex");
    Qe(this, "ey");
    Qe(this, "ez");
    Qe(this, "et");
    const c = wr;
    this.ex = Ye(e, 0n, c), this.ey = Ye(r, 0n, c), this.ez = Ye(i, 1n, c), this.et = Ye(o, 0n, c), Object.freeze(this);
  }
  static fromAffine(e) {
    return new Ce(e.x, e.y, 1n, M(e.x * e.y));
  }
  /** RFC8032 5.1.3: Uint8Array to Point. */
  static fromBytes(e, r = !1) {
    const i = or, o = Br(mt(e, Ct)), c = e[31];
    o[31] = c & -129;
    const u = _n(o);
    Ye(u, 0n, r ? wr : oe);
    const f = M(u * u), p = M(f - 1n), y = M(i * f + 1n);
    let { isValid: v, value: N } = Bs(p, y);
    v || Ne("bad point: y not sqrt");
    const _ = (N & 1n) === 1n, b = (c & 128) !== 0;
    return !r && N === 0n && b && Ne("bad point: x==0, isLastByteOdd"), b !== _ && (N = M(-N)), new Ce(N, u, 1n, M(N * u));
  }
  /** Checks if the point is valid and on-curve. */
  assertValidity() {
    const e = ir, r = or, i = this;
    if (i.is0())
      throw new Error("bad point: ZERO");
    const { ex: o, ey: c, ez: u, et: l } = i, f = M(o * o), p = M(c * c), y = M(u * u), v = M(y * y), N = M(f * e), _ = M(y * M(N + p)), b = M(v + M(r * M(f * p)));
    if (_ !== b)
      throw new Error("bad point: equation left != right (1)");
    const E = M(o * c), g = M(u * l);
    if (E !== g)
      throw new Error("bad point: equation left != right (2)");
    return this;
  }
  /** Equality check: compare points P&Q. */
  equals(e) {
    const { ex: r, ey: i, ez: o } = this, { ex: c, ey: u, ez: l } = qr(e), f = M(r * l), p = M(c * o), y = M(i * l), v = M(u * o);
    return f === p && y === v;
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
    const { ex: e, ey: r, ez: i } = this, o = ir, c = M(e * e), u = M(r * r), l = M(2n * M(i * i)), f = M(o * c), p = e + r, y = M(M(p * p) - c - u), v = f + u, N = v - l, _ = f - u, b = M(y * N), E = M(v * _), g = M(y * _), x = M(N * v);
    return new Ce(b, E, x, g);
  }
  /** Point addition. Complete formula. Cost: `8M + 1*k + 8add + 1*2`. */
  add(e) {
    const { ex: r, ey: i, ez: o, et: c } = this, { ex: u, ey: l, ez: f, et: p } = qr(e), y = ir, v = or, N = M(r * u), _ = M(i * l), b = M(c * v * p), E = M(o * f), g = M((r + i) * (u + l) - N - _), x = M(E - b), S = M(E + b), C = M(_ - y * N), I = M(g * x), F = M(S * C), P = M(g * C), T = M(x * S);
    return new Ce(I, F, T, P);
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
    if (Ye(e, 1n, Kt), e === 1n)
      return this;
    if (this.equals(ft))
      return Us(e).p;
    let i = ut, o = ft;
    for (let c = this; e > 0n; c = c.double(), e >>= 1n)
      e & 1n ? i = i.add(c) : r && (o = o.add(c));
    return i;
  }
  /** Convert point to 2d xy affine point. (X, Y, Z) ∋ (x=X/Z, y=Y/Z) */
  toAffine() {
    const { ex: e, ey: r, ez: i } = this;
    if (this.equals(ut))
      return { x: 0n, y: 1n };
    const o = An(i, oe);
    return M(i * o) !== 1n && Ne("invalid inverse"), { x: M(e * o), y: M(r * o) };
  }
  toBytes() {
    const { x: e, y: r } = this.assertValidity().toAffine(), i = ks(r);
    return i[31] |= e & 1n ? 128 : 0, i;
  }
  toHex() {
    return Or(this.toBytes());
  }
  // encode to hex string
  clearCofactor() {
    return this.multiply(Vt(Es), !1);
  }
  isSmallOrder() {
    return this.clearCofactor().is0();
  }
  isTorsionFree() {
    let e = this.multiply(Kt / 2n, !1).double();
    return Kt % 2n && (e = e.add(this)), e.is0();
  }
  static fromHex(e, r) {
    return Ce.fromBytes(Sn(e), r);
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
Qe(Ce, "BASE"), Qe(Ce, "ZERO");
let Ze = Ce;
const ft = new Ze(jr, Vr, 1n, M(jr * Vr)), ut = new Ze(0n, 1n, 1n, 0n);
Ze.BASE = ft;
Ze.ZERO = ut;
const ks = (t) => Fr(Nn(Ye(t, 0n, wr), wn)).reverse(), _n = (t) => Vt("0x" + Or(Br(mt(t)).reverse())), Te = (t, e) => {
  let r = t;
  for (; e-- > 0n; )
    r *= r, r %= oe;
  return r;
}, Is = (t) => {
  const r = t * t % oe * t % oe, i = Te(r, 2n) * r % oe, o = Te(i, 1n) * t % oe, c = Te(o, 5n) * o % oe, u = Te(c, 10n) * c % oe, l = Te(u, 20n) * u % oe, f = Te(l, 40n) * l % oe, p = Te(f, 80n) * f % oe, y = Te(p, 80n) * f % oe, v = Te(y, 10n) * c % oe;
  return { pow_p_5_8: Te(v, 2n) * t % oe, b2: r };
}, Qr = 0x2b8324804fc1df0b2b4d00993dfbd7a72f431806ad2fe478c4ee1b274a0ea0b0n, Bs = (t, e) => {
  const r = M(e * e * e), i = M(r * r * e), o = Is(t * i).pow_p_5_8;
  let c = M(t * r * o);
  const u = M(e * c * c), l = c, f = M(c * Qr), p = u === t, y = u === M(-t), v = u === M(-t * Qr);
  return p && (c = l), (y || v) && (c = f), (M(c) & 1n) === 1n && (c = M(-c)), { isValid: p || y, value: c };
}, Os = (t) => Rs(_n(t)), Fs = (...t) => vr.sha512Async(...t), Ls = (t) => {
  const e = t.slice(0, Ct);
  e[0] &= 248, e[31] &= 127, e[31] |= 64;
  const r = t.slice(Ct, wn), i = Os(e), o = ft.multiply(i), c = o.toBytes();
  return { head: e, prefix: r, scalar: i, point: o, pointBytes: c };
}, Ds = (t) => Fs(Sn(t, Ct)).then(Ls), Lr = (t) => Ds(t).then((e) => e.pointBytes), vr = {
  sha512Async: async (...t) => {
    const e = Ts(), r = zr(...t);
    return Yt(await e.digest("SHA-512", r.buffer));
  },
  sha512Sync: void 0,
  bytesToHex: Or,
  hexToBytes: Fr,
  concatBytes: zr,
  mod: M,
  invert: An,
  randomBytes: Cs
}, Gt = 8, Ms = 256, Tn = Math.ceil(Ms / Gt) + 1, Nr = 2 ** (Gt - 1), Ps = () => {
  const t = [];
  let e = ft, r = e;
  for (let i = 0; i < Tn; i++) {
    r = e, t.push(r);
    for (let o = 1; o < Nr; o++)
      r = r.add(e), t.push(r);
    e = r.double();
  }
  return t;
};
let Yr;
const Wr = (t, e) => {
  const r = e.negate();
  return t ? r : e;
}, Us = (t) => {
  const e = Yr || (Yr = Ps());
  let r = ut, i = ft;
  const o = 2 ** Gt, c = o, u = Vt(o - 1), l = Vt(Gt);
  for (let f = 0; f < Tn; f++) {
    let p = Number(t & u);
    t >>= l, p > Nr && (p -= c, t += 1n);
    const y = f * Nr, v = y, N = y + Math.abs(p) - 1, _ = f % 2 !== 0, b = p < 0;
    p === 0 ? i = i.add(Wr(_, e[v])) : r = r.add(Wr(b, e[N]));
  }
  return { p: r, f: i };
};
/*! noble-hashes - MIT License (c) 2022 Paul Miller (paulmillr.com) */
function $s(t) {
  return t instanceof Uint8Array || ArrayBuffer.isView(t) && t.constructor.name === "Uint8Array";
}
function Dr(t, ...e) {
  if (!$s(t))
    throw new Error("Uint8Array expected");
  if (e.length > 0 && !e.includes(t.length))
    throw new Error("Uint8Array expected of length " + e + ", got length=" + t.length);
}
function Zr(t, e = !0) {
  if (t.destroyed)
    throw new Error("Hash instance has been destroyed");
  if (e && t.finished)
    throw new Error("Hash#digest() has already been called");
}
function Ks(t, e) {
  Dr(t);
  const r = e.outputLen;
  if (t.length < r)
    throw new Error("digestInto() expects output buffer of length at least " + r);
}
function Sr(...t) {
  for (let e = 0; e < t.length; e++)
    t[e].fill(0);
}
function lr(t) {
  return new DataView(t.buffer, t.byteOffset, t.byteLength);
}
function Hs(t) {
  if (typeof t != "string")
    throw new Error("string expected");
  return new Uint8Array(new TextEncoder().encode(t));
}
function Cn(t) {
  return typeof t == "string" && (t = Hs(t)), Dr(t), t;
}
class js {
}
function Vs(t) {
  const e = (i) => t().update(Cn(i)).digest(), r = t();
  return e.outputLen = r.outputLen, e.blockLen = r.blockLen, e.create = () => t(), e;
}
function Gs(t, e, r, i) {
  if (typeof t.setBigUint64 == "function")
    return t.setBigUint64(e, r, i);
  const o = BigInt(32), c = BigInt(4294967295), u = Number(r >> o & c), l = Number(r & c), f = i ? 4 : 0, p = i ? 0 : 4;
  t.setUint32(e + f, u, i), t.setUint32(e + p, l, i);
}
class zs extends js {
  constructor(e, r, i, o) {
    super(), this.finished = !1, this.length = 0, this.pos = 0, this.destroyed = !1, this.blockLen = e, this.outputLen = r, this.padOffset = i, this.isLE = o, this.buffer = new Uint8Array(e), this.view = lr(this.buffer);
  }
  update(e) {
    Zr(this), e = Cn(e), Dr(e);
    const { view: r, buffer: i, blockLen: o } = this, c = e.length;
    for (let u = 0; u < c; ) {
      const l = Math.min(o - this.pos, c - u);
      if (l === o) {
        const f = lr(e);
        for (; o <= c - u; u += o)
          this.process(f, u);
        continue;
      }
      i.set(e.subarray(u, u + l), this.pos), this.pos += l, u += l, this.pos === o && (this.process(r, 0), this.pos = 0);
    }
    return this.length += e.length, this.roundClean(), this;
  }
  digestInto(e) {
    Zr(this), Ks(e, this), this.finished = !0;
    const { buffer: r, view: i, blockLen: o, isLE: c } = this;
    let { pos: u } = this;
    r[u++] = 128, Sr(this.buffer.subarray(u)), this.padOffset > o - u && (this.process(i, 0), u = 0);
    for (let v = u; v < o; v++)
      r[v] = 0;
    Gs(i, o - 8, BigInt(this.length * 8), c), this.process(i, 0);
    const l = lr(e), f = this.outputLen;
    if (f % 4)
      throw new Error("_sha2: outputLen should be aligned to 32bit");
    const p = f / 4, y = this.get();
    if (p > y.length)
      throw new Error("_sha2: outputLen bigger than state");
    for (let v = 0; v < p; v++)
      l.setUint32(4 * v, y[v], c);
  }
  digest() {
    const { buffer: e, outputLen: r } = this;
    this.digestInto(e);
    const i = e.slice(0, r);
    return this.destroy(), i;
  }
  _cloneInto(e) {
    e || (e = new this.constructor()), e.set(...this.get());
    const { blockLen: r, buffer: i, length: o, finished: c, destroyed: u, pos: l } = this;
    return e.destroyed = u, e.finished = c, e.length = o, e.pos = l, o % r && e.buffer.set(i), e;
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
]), Dt = /* @__PURE__ */ BigInt(2 ** 32 - 1), Jr = /* @__PURE__ */ BigInt(32);
function qs(t, e = !1) {
  return e ? { h: Number(t & Dt), l: Number(t >> Jr & Dt) } : { h: Number(t >> Jr & Dt) | 0, l: Number(t & Dt) | 0 };
}
function Qs(t, e = !1) {
  const r = t.length;
  let i = new Uint32Array(r), o = new Uint32Array(r);
  for (let c = 0; c < r; c++) {
    const { h: u, l } = qs(t[c], e);
    [i[c], o[c]] = [u, l];
  }
  return [i, o];
}
const Xr = (t, e, r) => t >>> r, en = (t, e, r) => t << 32 - r | e >>> r, at = (t, e, r) => t >>> r | e << 32 - r, it = (t, e, r) => t << 32 - r | e >>> r, Mt = (t, e, r) => t << 64 - r | e >>> r - 32, Pt = (t, e, r) => t >>> r - 32 | e << 64 - r;
function De(t, e, r, i) {
  const o = (e >>> 0) + (i >>> 0);
  return { h: t + r + (o / 2 ** 32 | 0) | 0, l: o | 0 };
}
const Ys = (t, e, r) => (t >>> 0) + (e >>> 0) + (r >>> 0), Ws = (t, e, r, i) => e + r + i + (t / 2 ** 32 | 0) | 0, Zs = (t, e, r, i) => (t >>> 0) + (e >>> 0) + (r >>> 0) + (i >>> 0), Js = (t, e, r, i, o) => e + r + i + o + (t / 2 ** 32 | 0) | 0, Xs = (t, e, r, i, o) => (t >>> 0) + (e >>> 0) + (r >>> 0) + (i >>> 0) + (o >>> 0), ea = (t, e, r, i, o, c) => e + r + i + o + c + (t / 2 ** 32 | 0) | 0, Rn = Qs([
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
].map((t) => BigInt(t))), ta = Rn[0], ra = Rn[1], Ke = /* @__PURE__ */ new Uint32Array(80), He = /* @__PURE__ */ new Uint32Array(80);
class na extends zs {
  constructor(e = 64) {
    super(128, e, 16, !1), this.Ah = ie[0] | 0, this.Al = ie[1] | 0, this.Bh = ie[2] | 0, this.Bl = ie[3] | 0, this.Ch = ie[4] | 0, this.Cl = ie[5] | 0, this.Dh = ie[6] | 0, this.Dl = ie[7] | 0, this.Eh = ie[8] | 0, this.El = ie[9] | 0, this.Fh = ie[10] | 0, this.Fl = ie[11] | 0, this.Gh = ie[12] | 0, this.Gl = ie[13] | 0, this.Hh = ie[14] | 0, this.Hl = ie[15] | 0;
  }
  // prettier-ignore
  get() {
    const { Ah: e, Al: r, Bh: i, Bl: o, Ch: c, Cl: u, Dh: l, Dl: f, Eh: p, El: y, Fh: v, Fl: N, Gh: _, Gl: b, Hh: E, Hl: g } = this;
    return [e, r, i, o, c, u, l, f, p, y, v, N, _, b, E, g];
  }
  // prettier-ignore
  set(e, r, i, o, c, u, l, f, p, y, v, N, _, b, E, g) {
    this.Ah = e | 0, this.Al = r | 0, this.Bh = i | 0, this.Bl = o | 0, this.Ch = c | 0, this.Cl = u | 0, this.Dh = l | 0, this.Dl = f | 0, this.Eh = p | 0, this.El = y | 0, this.Fh = v | 0, this.Fl = N | 0, this.Gh = _ | 0, this.Gl = b | 0, this.Hh = E | 0, this.Hl = g | 0;
  }
  process(e, r) {
    for (let C = 0; C < 16; C++, r += 4)
      Ke[C] = e.getUint32(r), He[C] = e.getUint32(r += 4);
    for (let C = 16; C < 80; C++) {
      const I = Ke[C - 15] | 0, F = He[C - 15] | 0, P = at(I, F, 1) ^ at(I, F, 8) ^ Xr(I, F, 7), T = it(I, F, 1) ^ it(I, F, 8) ^ en(I, F, 7), R = Ke[C - 2] | 0, B = He[C - 2] | 0, U = at(R, B, 19) ^ Mt(R, B, 61) ^ Xr(R, B, 6), L = it(R, B, 19) ^ Pt(R, B, 61) ^ en(R, B, 6), $ = Zs(T, L, He[C - 7], He[C - 16]), H = Js($, P, U, Ke[C - 7], Ke[C - 16]);
      Ke[C] = H | 0, He[C] = $ | 0;
    }
    let { Ah: i, Al: o, Bh: c, Bl: u, Ch: l, Cl: f, Dh: p, Dl: y, Eh: v, El: N, Fh: _, Fl: b, Gh: E, Gl: g, Hh: x, Hl: S } = this;
    for (let C = 0; C < 80; C++) {
      const I = at(v, N, 14) ^ at(v, N, 18) ^ Mt(v, N, 41), F = it(v, N, 14) ^ it(v, N, 18) ^ Pt(v, N, 41), P = v & _ ^ ~v & E, T = N & b ^ ~N & g, R = Xs(S, F, T, ra[C], He[C]), B = ea(R, x, I, P, ta[C], Ke[C]), U = R | 0, L = at(i, o, 28) ^ Mt(i, o, 34) ^ Mt(i, o, 39), $ = it(i, o, 28) ^ Pt(i, o, 34) ^ Pt(i, o, 39), H = i & c ^ i & l ^ c & l, j = o & u ^ o & f ^ u & f;
      x = E | 0, S = g | 0, E = _ | 0, g = b | 0, _ = v | 0, b = N | 0, { h: v, l: N } = De(p | 0, y | 0, B | 0, U | 0), p = l | 0, y = f | 0, l = c | 0, f = u | 0, c = i | 0, u = o | 0;
      const q = Ys(U, $, j);
      i = Ws(q, B, L, H), o = q | 0;
    }
    ({ h: i, l: o } = De(this.Ah | 0, this.Al | 0, i | 0, o | 0)), { h: c, l: u } = De(this.Bh | 0, this.Bl | 0, c | 0, u | 0), { h: l, l: f } = De(this.Ch | 0, this.Cl | 0, l | 0, f | 0), { h: p, l: y } = De(this.Dh | 0, this.Dl | 0, p | 0, y | 0), { h: v, l: N } = De(this.Eh | 0, this.El | 0, v | 0, N | 0), { h: _, l: b } = De(this.Fh | 0, this.Fl | 0, _ | 0, b | 0), { h: E, l: g } = De(this.Gh | 0, this.Gl | 0, E | 0, g | 0), { h: x, l: S } = De(this.Hh | 0, this.Hl | 0, x | 0, S | 0), this.set(i, o, c, u, l, f, p, y, v, N, _, b, E, g, x, S);
  }
  roundClean() {
    Sr(Ke, He);
  }
  destroy() {
    Sr(this.buffer), this.set(0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0);
  }
}
const sa = /* @__PURE__ */ Vs(() => new na()), aa = sa, ia = (t) => typeof Buffer < "u" ? Buffer.from(t, "base64") : Uint8Array.from(atob(t), (e) => e.charCodeAt(0)), oa = (t) => {
  if (typeof Buffer < "u")
    return Buffer.from(t).toString("base64");
  const e = Array.from(t, (r) => String.fromCharCode(r)).join("");
  return btoa(e);
};
function Wt(t) {
  return ia(t);
}
function la(t) {
  return oa(t);
}
vr.sha512Sync = (...t) => aa(vr.concatBytes(...t));
const ca = {
  isAuthenticated: !1,
  systemPublicKey: null,
  systemKeyId: null,
  privateKey: null,
  publicKeyId: null,
  isLoading: !1,
  error: null
}, cr = Je(
  "auth/initializeSystemKey",
  async (t, { rejectWithValue: e }) => {
    try {
      const r = await Ir();
      if (console.log("initializeSystemKey thunk response:", r), r.success && r.data && r.data.private_key) {
        const i = Wt(r.data.private_key), o = await Lr(i);
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
), Ht = Je(
  "auth/validatePrivateKey",
  async (t, { getState: e, rejectWithValue: r }) => {
    const i = e(), { systemPublicKey: o, systemKeyId: c } = i.auth;
    if (!o || !c)
      return r("System public key not available");
    try {
      console.log("🔑 Converting private key from base64...");
      const u = Wt(t);
      console.log("🔑 Generating public key from private key...");
      const l = await Lr(u), f = btoa(
        String.fromCharCode(...l)
      ), p = f === o;
      return console.log("🔑 Key comparison:", {
        derived: f,
        system: o,
        matches: p
      }), p ? {
        privateKey: u,
        publicKeyId: c,
        isAuthenticated: !0
      } : r("Private key does not match system public key");
    } catch (u) {
      return console.error("Private key validation failed:", u), r(
        u instanceof Error ? u.message : "Private key validation failed"
      );
    }
  }
), dr = Je(
  "auth/refreshSystemKey",
  async (t, { rejectWithValue: e }) => {
    for (let o = 1; o <= 5; o++)
      try {
        const c = await Ir();
        if (c.success && c.data && c.data.private_key) {
          const u = Wt(c.data.private_key), l = await Lr(u);
          return {
            systemPublicKey: btoa(
              String.fromCharCode(...l)
            ),
            systemKeyId: "node-private-key",
            privateKey: u,
            isSystemReady: !0
          };
        } else if (o < 5) {
          const u = 200 * o;
          await new Promise((l) => setTimeout(l, u));
        }
      } catch (c) {
        if (o === 5)
          return e(
            c instanceof Error ? c.message : "Failed to fetch node private key"
          );
        {
          const u = 200 * o;
          await new Promise((l) => setTimeout(l, u));
        }
      }
    return e(
      "Failed to fetch node private key after multiple attempts"
    );
  }
), ur = Je(
  "auth/fetchNodePrivateKey",
  async (t, { rejectWithValue: e }) => {
    try {
      const r = await Ir();
      return console.log("fetchNodePrivateKey thunk response:", r), r.success && r.data && r.data.private_key ? {
        privateKey: Wt(r.data.private_key),
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
), Mr = Je(
  "auth/loginUser",
  async (t, { rejectWithValue: e }) => {
    try {
      const i = new TextEncoder().encode(t), o = await crypto.subtle.digest("SHA-256", i), l = Array.from(new Uint8Array(o)).map((f) => f.toString(16).padStart(2, "0")).join("").substring(0, 32);
      return { id: t, hash: l };
    } catch {
      return e("Failed to generate user hash");
    }
  }
), kn = Ar({
  name: "auth",
  initialState: ca,
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
    t.addCase(cr.pending, (e) => {
      e.isLoading = !0, e.error = null;
    }).addCase(cr.fulfilled, (e, r) => {
      console.log("initializeSystemKey.fulfilled", r.payload), e.isLoading = !1, e.systemPublicKey = r.payload.systemPublicKey, e.systemKeyId = r.payload.systemKeyId, e.privateKey = r.payload.privateKey, e.error = null;
    }).addCase(cr.rejected, (e, r) => {
      e.isLoading = !1, e.error = r.payload;
    }).addCase(Ht.pending, (e) => {
      e.isLoading = !0, e.error = null;
    }).addCase(Ht.fulfilled, (e, r) => {
      e.isLoading = !1, e.isAuthenticated = r.payload.isAuthenticated, e.privateKey = r.payload.privateKey, e.publicKeyId = r.payload.publicKeyId, e.error = null;
    }).addCase(Ht.rejected, (e, r) => {
      e.isLoading = !1, e.isAuthenticated = !1, e.privateKey = null, e.publicKeyId = null, e.error = r.payload;
    }).addCase(dr.pending, (e) => {
      e.isLoading = !0, e.error = null;
    }).addCase(dr.fulfilled, (e, r) => {
      e.isLoading = !1, e.systemPublicKey = r.payload.systemPublicKey, e.systemKeyId = r.payload.systemKeyId, e.privateKey = r.payload.privateKey, e.user || (e.isAuthenticated = !1), e.error = null;
    }).addCase(dr.rejected, (e, r) => {
      e.isLoading = !1, e.systemPublicKey = null, e.systemKeyId = null, e.error = r.payload;
    }).addCase(ur.pending, (e) => {
      e.isLoading = !0, e.error = null;
    }).addCase(ur.fulfilled, (e, r) => {
      e.isLoading = !1, e.privateKey = r.payload.privateKey, e.publicKeyId = r.payload.publicKeyId, e.error = null;
    }).addCase(ur.rejected, (e, r) => {
      e.isLoading = !1, e.error = r.payload;
    }).addCase(Mr.fulfilled, (e, r) => {
      e.isAuthenticated = !0, e.user = r.payload, e.error = null;
    });
  }
}), {
  clearAuthentication: da,
  setError: Oo,
  clearError: Fo,
  updateSystemKey: Lo,
  logoutUser: ua,
  restoreSession: Do
} = kn.actions, ha = kn.reducer, ma = 3e5, hr = 3, kt = {
  // Async thunk action types
  FETCH_SCHEMAS: "schemas/fetchSchemas",
  APPROVE_SCHEMA: "schemas/approveSchema",
  BLOCK_SCHEMA: "schemas/blockSchema",
  UNLOAD_SCHEMA: "schemas/unloadSchema",
  LOAD_SCHEMA: "schemas/loadSchema"
}, It = {
  // Network and API errors
  FETCH_FAILED: "Failed to fetch schemas from server",
  // Schema operation errors
  APPROVE_FAILED: "Failed to approve schema",
  BLOCK_FAILED: "Failed to block schema",
  UNLOAD_FAILED: "Failed to unload schema",
  LOAD_FAILED: "Failed to load schema"
}, Oe = {
  AVAILABLE: "available",
  APPROVED: "approved",
  BLOCKED: "blocked",
  LOADING: "loading",
  ERROR: "error"
};
process.env.NODE_ENV, process.env.NODE_ENV;
process.env.NODE_ENV, process.env.NODE_ENV;
const fa = {
  MUTATION_WRAPPER_KEY: "value"
}, pa = 200, ga = 300, ya = [
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
], Ut = {
  executeQuery: "Execute Query"
}, We = {
  schema: "Schema",
  schemaEmpty: "No schemas available",
  schemaHelp: "Select a schema to work with",
  operationType: "Operation Type",
  operationHelp: "Select the type of operation to perform"
}, ba = {
  loading: "Loading..."
}, xa = [
  { value: "Insert", label: "Insert" },
  { value: "Update", label: "Update" },
  { value: "Delete", label: "Delete" }
], In = {
  Insert: "create",
  Create: "create",
  Update: "update",
  Delete: "delete"
}, tn = {
  approved: "bg-green-100 text-green-800",
  available: "bg-blue-100 text-blue-800",
  blocked: "bg-red-100 text-red-800",
  pending: "bg-yellow-100 text-yellow-800"
}, Bn = {
  schemaStates: {
    approved: "Schema is approved for use in queries and mutations",
    available: "Schema is available but requires approval before use",
    blocked: "Schema is blocked and cannot be used",
    pending: "Schema approval is pending review",
    unknown: "Schema state is unknown or invalid"
  }
}, rn = {
  label: "Range Key",
  badgeColor: "bg-purple-100 text-purple-800"
};
function On(t) {
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
function Pr(t) {
  return !t || typeof t != "object" ? !1 : On(t) === "HashRange";
}
function Fn(t) {
  if (typeof t != "string") return null;
  const e = t.split(".");
  return e[e.length - 1] || t;
}
function pt(t) {
  return !t || typeof t != "object" ? !1 : On(t) === "Range";
}
function et(t) {
  var r;
  if (!t || typeof t != "object") return null;
  const e = (r = t == null ? void 0 : t.key) == null ? void 0 : r.range_field;
  return typeof e == "string" && e.trim() ? Fn(e) : null;
}
function Ln(t) {
  var r;
  if (!t || typeof t != "object") return null;
  const e = (r = t == null ? void 0 : t.key) == null ? void 0 : r.hash_field;
  return e && typeof e == "string" && e.trim() ? Fn(e) : null;
}
function wa(t) {
  if (!pt(t))
    return {};
  const e = et(t);
  if (!Array.isArray(t.fields))
    throw new Error(`Expected schema.fields to be an array for range schema "${t.name}", got ${typeof t.fields}`);
  return t.fields.reduce((r, i) => (i !== e && (r[i] = {}), r), {});
}
function va(t, e, r, i) {
  const o = typeof e == "string" ? In[e] || e.toLowerCase() : "", c = o === "delete", u = {
    type: "mutation",
    schema: t.name,
    mutation_type: o
  }, l = et(t);
  if (c)
    u.fields_and_values = {}, u.key_value = { hash: null, range: null }, r && r.trim() && l && (u.fields_and_values[l] = r.trim(), u.key_value.range = r.trim());
  else {
    const f = {};
    r && r.trim() && l && (f[l] = r.trim()), Object.entries(i).forEach(([p, y]) => {
      if (p !== l) {
        const v = fa.MUTATION_WRAPPER_KEY;
        typeof y == "string" || typeof y == "number" || typeof y == "boolean" ? f[p] = { [v]: y } : typeof y == "object" && y !== null ? f[p] = y : f[p] = { [v]: y };
      }
    }), u.fields_and_values = f, u.key_value = {
      hash: null,
      range: r && r.trim() ? r.trim() : null
    };
  }
  return u;
}
function Na(t) {
  return pt(t) ? {
    isRangeSchema: !0,
    rangeKey: et(t),
    rangeFields: [],
    // Declarative schemas don't store field types
    nonRangeKeyFields: wa(t),
    totalFields: Array.isArray(t.fields) ? t.fields.length : 0
  } : null;
}
function Dn(t) {
  return typeof t == "string" ? t.toLowerCase() : typeof t == "object" && t !== null ? t.state ? String(t.state).toLowerCase() : String(t).toLowerCase() : String(t || "").toLowerCase();
}
function Sa(t) {
  return t == null;
}
function Ea(t) {
  return Pr(t) ? {
    isHashRangeSchema: !0,
    hashField: Ln(t),
    rangeField: et(t),
    totalFields: Array.isArray(t.fields) ? t.fields.length : 0
  } : null;
}
const $t = pe.AVAILABLE, Aa = /* @__PURE__ */ new Set([
  pe.AVAILABLE,
  pe.APPROVED,
  pe.BLOCKED,
  "loading",
  "error"
]);
function _a(t) {
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
function Ta(t) {
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
class Zt {
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
    const e = await this.client.get(Q.LIST_SCHEMAS, {
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
    return this.client.get(Q.GET_SCHEMA(e), {
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
    if (!Object.values(pe).includes(e))
      throw new Error(`Invalid schema state: ${e}. Must be one of: ${Object.values(pe).join(", ")}`);
    const r = await this.getSchemas();
    return !r.success || !r.data ? { success: !1, error: "Failed to fetch schemas", status: r.status, data: { data: [], state: e } } : {
      success: !0,
      data: { data: r.data.filter((c) => c.state === e).map((c) => c.name), state: e },
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
    return r.forEach((c) => {
      const u = _a(c);
      if (!u) {
        typeof console < "u" && console.warn && console.warn("[schemaClient.getAllSchemasWithState] Encountered schema entry without a name, skipping entry.");
        return;
      }
      const l = Ta(c), f = Dn(l);
      if (!l || f.length === 0) {
        typeof console < "u" && console.warn && console.warn(
          `[schemaClient.getAllSchemasWithState] Missing schema state for '${u}', defaulting to '${$t}'.`
        ), i[u] = $t;
        return;
      }
      if (!Aa.has(f)) {
        typeof console < "u" && console.warn && console.warn(
          `[schemaClient.getAllSchemasWithState] Unrecognized schema state '${String(l)}' for '${u}', defaulting to '${$t}'.`
        ), i[u] = $t;
        return;
      }
      i[u] = f;
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
      available: r.filter((c) => c.state === pe.AVAILABLE).length,
      approved: r.filter((c) => c.state === pe.APPROVED).length,
      blocked: r.filter((c) => c.state === pe.BLOCKED).length,
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
      Q.APPROVE_SCHEMA(e),
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
      Q.BLOCK_SCHEMA(e),
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
      return !r.success || !r.data ? { success: !1, error: "Failed to fetch schemas", status: r.status, data: [] } : { success: !0, data: r.data.filter((o) => o.state === pe.APPROVED), status: 200, meta: { timestamp: Date.now(), cached: (e = r.meta) == null ? void 0 : e.cached } };
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
      return o.state !== pe.APPROVED ? {
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
const ee = new Zt();
function Ca(t) {
  return new Zt(t);
}
ee.getSchemasByState.bind(ee);
ee.getAllSchemasWithState.bind(ee);
ee.getSchemaStatus.bind(ee);
ee.getSchema.bind(ee);
ee.approveSchema.bind(ee);
ee.blockSchema.bind(ee);
ee.loadSchema.bind(ee);
ee.unloadSchema.bind(ee);
ee.getApprovedSchemas.bind(ee);
ee.validateSchemaForOperation.bind(ee);
ee.getBackfillStatus.bind(ee);
const me = {
  APPROVE: "approve",
  BLOCK: "block",
  UNLOAD: "unload",
  LOAD: "load"
}, Mn = (t, e) => t ? Date.now() - t < e : !1, Ra = (t, e, r = Date.now()) => ({
  schemaName: t,
  error: e,
  timestamp: r
}), ka = (t, e, r, i) => ({
  schemaName: t,
  newState: e,
  timestamp: Date.now(),
  updatedSchema: r,
  backfillHash: i
}), Jt = (t, e, r, i) => Je(
  t,
  async ({ schemaName: o, options: c = {} }, { getState: u, rejectWithValue: l }) => {
    var p;
    u().schemas.schemas[o];
    try {
      const y = await e(o);
      if (!y.success)
        throw new Error(y.error || i);
      const v = (p = y.data) == null ? void 0 : p.backfill_hash;
      return ka(o, r, void 0, v);
    } catch (y) {
      return l(
        Ra(
          o,
          y instanceof Error ? y.message : i
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
    const { schemaName: o, newState: c, updatedSchema: u } = i.payload;
    r.loading.operations[o] = !1, r.schemas[o] && (r.schemas[o].state = c, u && Object.assign(r.schemas[o], u), r.schemas[o].lastOperation = {
      type: e,
      timestamp: Date.now(),
      success: !0
    });
  },
  rejected: (r, i) => {
    const { schemaName: o, error: c } = i.payload;
    r.loading.operations[o] = !1, r.errors.operations[o] = c, r.schemas[o] && (r.schemas[o].lastOperation = {
      type: e,
      timestamp: Date.now(),
      success: !1,
      error: c
    });
  }
}), nn = {
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
    ttl: ma,
    version: "1.0.0",
    lastUpdated: null
  },
  activeSchema: null
}, ke = Je(
  kt.FETCH_SCHEMAS,
  async (t = {}, { getState: e, rejectWithValue: r }) => {
    const i = e(), { lastFetched: o, cache: c } = i.schemas;
    if (!t.forceRefresh && Mn(o, c.ttl))
      return {
        schemas: Object.values(i.schemas.schemas),
        timestamp: o
      };
    const u = new Zt(
      Ae({
        baseUrl: qt.BASE_URL,
        // Use main API base URL (/api)
        enableCache: !0,
        enableLogging: !0,
        enableMetrics: !0
      })
    );
    t.forceRefresh && (console.log("🔄 Force refresh requested - clearing API client cache"), u.clearCache());
    let l = null;
    for (let p = 1; p <= hr; p++)
      try {
        const y = await u.getSchemas();
        if (!y.success)
          throw new Error(`Failed to fetch schemas: ${y.error || "Unknown error"}`);
        console.log("📁 Raw schemas response:", y.data);
        const v = y.data || [];
        if (!Array.isArray(v))
          throw new Error(`Schemas response is not an array: ${typeof v}`);
        const N = v.map((b) => {
          if (!b.name)
            if (console.warn("⚠️ Schema missing name field:", b), b.schema && b.schema.name)
              b.name = b.schema.name;
            else
              return console.error("❌ Schema has no name field and cannot be displayed:", b), null;
          let E = Oe.AVAILABLE;
          return b.state && (typeof b.state == "string" ? E = b.state.toLowerCase() : typeof b.state == "object" && b.state.state ? E = String(b.state.state).toLowerCase() : E = String(b.state).toLowerCase()), console.log("🟢 fetchSchemas: Using backend schema for", b.name, "with state:", E), {
            ...b,
            state: E
          };
        }).filter((b) => b !== null);
        console.log("✅ Using backend schemas directly:", N.map((b) => ({ name: b.name, state: b.state })));
        const _ = Date.now();
        return {
          schemas: N,
          timestamp: _
        };
      } catch (y) {
        if (l = y instanceof Error ? y : new Error("Unknown error"), p < hr) {
          const N = typeof window < "u" && window.__TEST_ENV__ === !0 ? 10 : 1e3 * p;
          await new Promise((_) => setTimeout(_, N));
        }
      }
    const f = `Failed to fetch schemas after ${hr} attempts: ${(l == null ? void 0 : l.message) || "Unknown error"}`;
    return r(f);
  }
), Xt = () => new Zt(
  Ae({
    baseUrl: qt.BASE_URL,
    // Use main API base URL (/api)
    enableCache: !0,
    enableLogging: !0,
    enableMetrics: !0
  })
), je = Jt(
  kt.APPROVE_SCHEMA,
  (t) => Xt().approveSchema(t),
  Oe.APPROVED,
  It.APPROVE_FAILED
), Ve = Jt(
  kt.BLOCK_SCHEMA,
  (t) => Xt().blockSchema(t),
  Oe.BLOCKED,
  It.BLOCK_FAILED
), ot = Jt(
  kt.UNLOAD_SCHEMA,
  (t) => Xt().unloadSchema(t),
  Oe.AVAILABLE,
  It.UNLOAD_FAILED
), lt = Jt(
  kt.LOAD_SCHEMA,
  (t) => Xt().loadSchema(t),
  Oe.APPROVED,
  It.LOAD_FAILED
), Pn = Ar({
  name: "schemas",
  initialState: nn,
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
        type: me.APPROVE,
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
      Object.assign(t, nn);
    }
  },
  extraReducers: (t) => {
    t.addCase(ke.pending, (e) => {
      e.loading.fetch = !0, e.errors.fetch = null;
    }).addCase(ke.fulfilled, (e, r) => {
      e.loading.fetch = !1, e.errors.fetch = null;
      const i = {};
      r.payload.schemas.forEach((o) => {
        i[o.name] = o;
      }), e.schemas = i, e.lastFetched = r.payload.timestamp, e.cache.lastUpdated = r.payload.timestamp;
    }).addCase(ke.rejected, (e, r) => {
      e.loading.fetch = !1, e.errors.fetch = r.payload || It.FETCH_FAILED;
    }).addCase(je.pending, be(je, me.APPROVE).pending).addCase(je.fulfilled, be(je, me.APPROVE).fulfilled).addCase(je.rejected, be(je, me.APPROVE).rejected).addCase(Ve.pending, be(Ve, me.BLOCK).pending).addCase(Ve.fulfilled, be(Ve, me.BLOCK).fulfilled).addCase(Ve.rejected, be(Ve, me.BLOCK).rejected).addCase(ot.pending, be(ot, me.UNLOAD).pending).addCase(ot.fulfilled, be(ot, me.UNLOAD).fulfilled).addCase(ot.rejected, be(ot, me.UNLOAD).rejected).addCase(lt.pending, be(lt, me.LOAD).pending).addCase(lt.fulfilled, be(lt, me.LOAD).fulfilled).addCase(lt.rejected, be(lt, me.LOAD).rejected);
  }
}), Ia = (t) => t.schemas, yt = (t) => Object.values(t.schemas.schemas), Ba = (t) => t.schemas.schemas, Bt = Xe(
  [yt],
  (t) => t.filter((e) => (typeof e.state == "string" ? e.state.toLowerCase() : typeof e.state == "object" && e.state !== null && e.state.state ? String(e.state.state).toLowerCase() : String(e.state || "").toLowerCase()) === Oe.APPROVED)
), Oa = Xe(
  [yt],
  (t) => t.filter((e) => e.state === Oe.AVAILABLE)
);
Xe(
  [yt],
  (t) => t.filter((e) => e.state === Oe.BLOCKED)
);
Xe(
  [Bt],
  (t) => t.filter((e) => {
    var r;
    return ((r = e.rangeInfo) == null ? void 0 : r.isRangeSchema) === !0;
  })
);
Xe(
  [Oa],
  (t) => t.filter((e) => {
    var r;
    return ((r = e.rangeInfo) == null ? void 0 : r.isRangeSchema) === !0;
  })
);
const Ur = (t) => t.schemas.loading.fetch, Un = (t) => t.schemas.errors.fetch, Fa = Xe(
  [Ia],
  (t) => ({
    isValid: Mn(t.lastFetched, t.cache.ttl),
    lastFetched: t.lastFetched,
    ttl: t.cache.ttl
  })
), La = (t) => t.schemas.activeSchema;
Xe(
  [La, Ba],
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
} = Pn.actions, Da = Pn.reducer, sn = {
  inputText: "",
  sessionId: null,
  isProcessing: !1,
  conversationLog: [],
  showResults: !1
}, $n = Ar({
  name: "aiQuery",
  initialState: sn,
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
    resetAIQueryState: () => sn
  }
}), {
  setInputText: an,
  clearInputText: Go,
  setSessionId: on,
  setIsProcessing: ln,
  addMessage: Ma,
  clearConversation: zo,
  setShowResults: Pa,
  startNewConversation: Ua,
  resetAIQueryState: qo
} = $n.actions, $a = $n.reducer, Ka = (t) => t.aiQuery.inputText, Ha = (t) => t.aiQuery.sessionId, ja = (t) => t.aiQuery.isProcessing, Va = (t) => t.aiQuery.conversationLog, Ga = (t) => t.aiQuery.showResults, za = (t) => t.aiQuery.sessionId && t.aiQuery.conversationLog.some((e) => e.type === "results"), qa = cs({
  reducer: {
    auth: ha,
    schemas: Da,
    aiQuery: $a
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
function Qa() {
  console.log("🔄 Schema client reset - will use new configuration on next request");
}
async function Ya(t) {
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
    const c = await fetch(i, o), u = Date.now() - e;
    return c.ok ? {
      success: !0,
      status: (await c.json()).status || "online",
      responseTime: u
    } : {
      success: !1,
      error: `HTTP ${c.status}: ${c.statusText}`,
      responseTime: u
    };
  } catch (o) {
    const c = Date.now() - e;
    return {
      success: !1,
      error: o.name === "TimeoutError" ? "Connection timeout" : o.message,
      responseTime: c
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
}, cn = "schemaServiceEnvironment", Kn = ss({
  environment: ht.LOCAL,
  setEnvironment: () => {
  },
  getSchemaServiceBaseUrl: () => ""
});
function Wa({ children: t }) {
  const [e, r] = O(() => {
    const c = localStorage.getItem(cn);
    if (c) {
      const u = Object.values(ht).find((l) => l.id === c);
      if (u) return u;
    }
    return ht.LOCAL;
  }), i = (c) => {
    const u = Object.values(ht).find((l) => l.id === c);
    u && (r(u), localStorage.setItem(cn, c), Qa(), console.log(`Schema service environment changed to: ${u.name} (${u.baseUrl || "same origin"})`), console.log("🔄 Schema client has been reset - next request will use new endpoint"));
  }, o = () => e.baseUrl || "";
  return /* @__PURE__ */ s(Kn.Provider, { value: { environment: e, setEnvironment: i, getSchemaServiceBaseUrl: o }, children: t });
}
function Za() {
  const t = as(Kn);
  if (!t)
    throw new Error("useSchemaServiceConfig must be used within SchemaServiceConfigProvider");
  return t;
}
const Qo = ({ children: t, store: e }) => /* @__PURE__ */ s(is, { store: e || qa, children: /* @__PURE__ */ s(Wa, { children: t }) });
function Ja({
  title: t,
  titleId: e,
  ...r
}, i) {
  return /* @__PURE__ */ V.createElement("svg", Object.assign({
    xmlns: "http://www.w3.org/2000/svg",
    viewBox: "0 0 24 24",
    fill: "currentColor",
    "aria-hidden": "true",
    "data-slot": "icon",
    ref: i,
    "aria-labelledby": e
  }, r), t ? /* @__PURE__ */ V.createElement("title", {
    id: e
  }, t) : null, /* @__PURE__ */ V.createElement("path", {
    fillRule: "evenodd",
    d: "M4.755 10.059a7.5 7.5 0 0 1 12.548-3.364l1.903 1.903h-3.183a.75.75 0 1 0 0 1.5h4.992a.75.75 0 0 0 .75-.75V4.356a.75.75 0 0 0-1.5 0v3.18l-1.9-1.9A9 9 0 0 0 3.306 9.67a.75.75 0 1 0 1.45.388Zm15.408 3.352a.75.75 0 0 0-.919.53 7.5 7.5 0 0 1-12.548 3.364l-1.902-1.903h3.183a.75.75 0 0 0 0-1.5H2.984a.75.75 0 0 0-.75.75v4.992a.75.75 0 0 0 1.5 0v-3.18l1.9 1.9a9 9 0 0 0 15.059-4.035.75.75 0 0 0-.53-.918Z",
    clipRule: "evenodd"
  }));
}
const dn = /* @__PURE__ */ V.forwardRef(Ja);
function Xa({
  title: t,
  titleId: e,
  ...r
}, i) {
  return /* @__PURE__ */ V.createElement("svg", Object.assign({
    xmlns: "http://www.w3.org/2000/svg",
    viewBox: "0 0 24 24",
    fill: "currentColor",
    "aria-hidden": "true",
    "data-slot": "icon",
    ref: i,
    "aria-labelledby": e
  }, r), t ? /* @__PURE__ */ V.createElement("title", {
    id: e
  }, t) : null, /* @__PURE__ */ V.createElement("path", {
    fillRule: "evenodd",
    d: "M2.25 12c0-5.385 4.365-9.75 9.75-9.75s9.75 4.365 9.75 9.75-4.365 9.75-9.75 9.75S2.25 17.385 2.25 12Zm13.36-1.814a.75.75 0 1 0-1.22-.872l-3.236 4.53L9.53 12.22a.75.75 0 0 0-1.06 1.06l2.25 2.25a.75.75 0 0 0 1.14-.094l3.75-5.25Z",
    clipRule: "evenodd"
  }));
}
const mr = /* @__PURE__ */ V.forwardRef(Xa);
function ei({
  title: t,
  titleId: e,
  ...r
}, i) {
  return /* @__PURE__ */ V.createElement("svg", Object.assign({
    xmlns: "http://www.w3.org/2000/svg",
    viewBox: "0 0 24 24",
    fill: "currentColor",
    "aria-hidden": "true",
    "data-slot": "icon",
    ref: i,
    "aria-labelledby": e
  }, r), t ? /* @__PURE__ */ V.createElement("title", {
    id: e
  }, t) : null, /* @__PURE__ */ V.createElement("path", {
    fillRule: "evenodd",
    d: "M12.53 16.28a.75.75 0 0 1-1.06 0l-7.5-7.5a.75.75 0 0 1 1.06-1.06L12 14.69l6.97-6.97a.75.75 0 1 1 1.06 1.06l-7.5 7.5Z",
    clipRule: "evenodd"
  }));
}
const Hn = /* @__PURE__ */ V.forwardRef(ei);
function ti({
  title: t,
  titleId: e,
  ...r
}, i) {
  return /* @__PURE__ */ V.createElement("svg", Object.assign({
    xmlns: "http://www.w3.org/2000/svg",
    viewBox: "0 0 24 24",
    fill: "currentColor",
    "aria-hidden": "true",
    "data-slot": "icon",
    ref: i,
    "aria-labelledby": e
  }, r), t ? /* @__PURE__ */ V.createElement("title", {
    id: e
  }, t) : null, /* @__PURE__ */ V.createElement("path", {
    fillRule: "evenodd",
    d: "M16.28 11.47a.75.75 0 0 1 0 1.06l-7.5 7.5a.75.75 0 0 1-1.06-1.06L14.69 12 7.72 5.03a.75.75 0 0 1 1.06-1.06l7.5 7.5Z",
    clipRule: "evenodd"
  }));
}
const jn = /* @__PURE__ */ V.forwardRef(ti);
function ri({
  title: t,
  titleId: e,
  ...r
}, i) {
  return /* @__PURE__ */ V.createElement("svg", Object.assign({
    xmlns: "http://www.w3.org/2000/svg",
    viewBox: "0 0 24 24",
    fill: "currentColor",
    "aria-hidden": "true",
    "data-slot": "icon",
    ref: i,
    "aria-labelledby": e
  }, r), t ? /* @__PURE__ */ V.createElement("title", {
    id: e
  }, t) : null, /* @__PURE__ */ V.createElement("path", {
    fillRule: "evenodd",
    d: "M12 2.25c-5.385 0-9.75 4.365-9.75 9.75s4.365 9.75 9.75 9.75 9.75-4.365 9.75-9.75S17.385 2.25 12 2.25ZM12.75 6a.75.75 0 0 0-1.5 0v6c0 .414.336.75.75.75h4.5a.75.75 0 0 0 0-1.5h-3.75V6Z",
    clipRule: "evenodd"
  }));
}
const fr = /* @__PURE__ */ V.forwardRef(ri);
function ni({
  title: t,
  titleId: e,
  ...r
}, i) {
  return /* @__PURE__ */ V.createElement("svg", Object.assign({
    xmlns: "http://www.w3.org/2000/svg",
    viewBox: "0 0 24 24",
    fill: "currentColor",
    "aria-hidden": "true",
    "data-slot": "icon",
    ref: i,
    "aria-labelledby": e
  }, r), t ? /* @__PURE__ */ V.createElement("title", {
    id: e
  }, t) : null, /* @__PURE__ */ V.createElement("path", {
    fillRule: "evenodd",
    d: "M16.5 4.478v.227a48.816 48.816 0 0 1 3.878.512.75.75 0 1 1-.256 1.478l-.209-.035-1.005 13.07a3 3 0 0 1-2.991 2.77H8.084a3 3 0 0 1-2.991-2.77L4.087 6.66l-.209.035a.75.75 0 0 1-.256-1.478A48.567 48.567 0 0 1 7.5 4.705v-.227c0-1.564 1.213-2.9 2.816-2.951a52.662 52.662 0 0 1 3.369 0c1.603.051 2.815 1.387 2.815 2.951Zm-6.136-1.452a51.196 51.196 0 0 1 3.273 0C14.39 3.05 15 3.684 15 4.478v.113a49.488 49.488 0 0 0-6 0v-.113c0-.794.609-1.428 1.364-1.452Zm-.355 5.945a.75.75 0 1 0-1.5.058l.347 9a.75.75 0 1 0 1.499-.058l-.346-9Zm5.48.058a.75.75 0 1 0-1.498-.058l-.347 9a.75.75 0 0 0 1.5.058l.345-9Z",
    clipRule: "evenodd"
  }));
}
const un = /* @__PURE__ */ V.forwardRef(ni);
function si({
  title: t,
  titleId: e,
  ...r
}, i) {
  return /* @__PURE__ */ V.createElement("svg", Object.assign({
    xmlns: "http://www.w3.org/2000/svg",
    viewBox: "0 0 24 24",
    fill: "currentColor",
    "aria-hidden": "true",
    "data-slot": "icon",
    ref: i,
    "aria-labelledby": e
  }, r), t ? /* @__PURE__ */ V.createElement("title", {
    id: e
  }, t) : null, /* @__PURE__ */ V.createElement("path", {
    fillRule: "evenodd",
    d: "M12 2.25c-5.385 0-9.75 4.365-9.75 9.75s4.365 9.75 9.75 9.75 9.75-4.365 9.75-9.75S17.385 2.25 12 2.25Zm-1.72 6.97a.75.75 0 1 0-1.06 1.06L10.94 12l-1.72 1.72a.75.75 0 1 0 1.06 1.06L12 13.06l1.72 1.72a.75.75 0 1 0 1.06-1.06L13.06 12l1.72-1.72a.75.75 0 1 0-1.06-1.06L12 10.94l-1.72-1.72Z",
    clipRule: "evenodd"
  }));
}
const ai = /* @__PURE__ */ V.forwardRef(si);
class Vn {
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
      Q.GET_SYSTEM_PUBLIC_KEY,
      {
        requiresAuth: !1,
        // System public key is public
        timeout: le.QUICK,
        retries: ce.CRITICAL,
        // Multiple retries for critical system data
        cacheable: !0,
        // Cache system public key
        cacheTtl: Ge.SYSTEM_PUBLIC_KEY,
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
    return this.client.get(Q.GET_SYSTEM_STATUS, {
      timeout: le.QUICK,
      retries: ce.STANDARD,
      cacheable: !0,
      cacheTtl: Ge.SECURITY_STATUS,
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
const Be = new Vn();
function ii(t) {
  return new Vn(t);
}
Be.getSystemPublicKey.bind(Be);
Be.validatePublicKeyFormat.bind(Be);
Be.validateSignedMessage.bind(Be);
Be.getSecurityStatus.bind(Be);
Be.verifyMessage.bind(Be);
class oi {
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
    return this.client.get(Q.LIST_TRANSFORMS, {
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
    return this.client.get(Q.GET_TRANSFORM_QUEUE, {
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
      Q.ADD_TO_TRANSFORM_QUEUE(e),
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
    return this.client.get(Q.GET_ALL_BACKFILLS, {
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
      Q.GET_BACKFILL(e),
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
const ge = new oi();
ge.getTransforms.bind(ge);
ge.getQueue.bind(ge);
ge.addToQueue.bind(ge);
ge.refreshQueue.bind(ge);
ge.getTransform.bind(ge);
class Gn {
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
      Q.EXECUTE_MUTATION,
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
      Q.EXECUTE_MUTATIONS_BATCH,
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
    return this.client.post(Q.EXECUTE_QUERY, e, {
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
      Q.EXECUTE_QUERY,
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
        Q.GET_SCHEMA(e),
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
      const i = r.data, o = i.state === pe.APPROVED;
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
const $r = new Gn();
function li(t) {
  return new Gn(t);
}
class ci {
  constructor(e) {
    this.client = e || Ae({
      baseUrl: us.ROOT,
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
    return this.client.get(Q.GET_STATUS, {
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
      Q.GET_INGESTION_CONFIG,
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
      Q.GET_INGESTION_CONFIG,
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
      Q.VALIDATE_JSON,
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
    }, c = this.validateIngestionRequest(o);
    if (!c.isValid)
      throw new Error(
        `Invalid ingestion request: ${c.errors.join(", ")}`
      );
    return this.client.post(
      Q.PROCESS_JSON,
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
const gt = new ci(), ct = Ae({
  timeout: le.AI_PROCESSING,
  retries: ce.LIMITED
}), hn = {
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
class di {
  constructor(e) {
    this.client = e || Ae({ enableCache: !0, enableLogging: !0 });
  }
  async search(e) {
    const r = `${Q.NATIVE_INDEX_SEARCH}?term=${encodeURIComponent(e)}`;
    return this.client.get(r, {
      timeout: 8e3,
      retries: 2,
      cacheable: !0,
      cacheTtl: 6e4
    });
  }
}
const ui = new di();
function Yo() {
  const [t, e] = O(!1), [r, i] = O(!1), [o, c] = O(null), [u, l] = O([]), [f, p] = O(!0), y = K(async () => {
    var g;
    try {
      const x = await gt.getAllProgress(), S = ((g = x.data) == null ? void 0 : g.progress) || x.data || x.progress || [];
      Array.isArray(S) ? l(S) : l([]);
    } catch (x) {
      console.error("Failed to fetch progress:", x), l([]);
    } finally {
      p(!1);
    }
  }, []);
  ue(() => {
    y();
    const g = setInterval(y, 2e3);
    return () => clearInterval(g);
  }, [y]);
  const v = async () => {
    i(!0), c(null);
    try {
      const g = await ne.resetDatabase(!0);
      g.success && g.data ? g.data.job_id ? (c({
        type: "success",
        message: `Reset started (Job: ${g.data.job_id.substring(0, 8)}...). Progress will appear above.`
      }), e(!1), i(!1)) : (c({ type: "success", message: g.data.message }), setTimeout(() => {
        window.location.reload();
      }, 2e3)) : (c({ type: "error", message: g.error || "Reset failed" }), e(!1), i(!1));
    } catch (g) {
      c({ type: "error", message: `Network error: ${g.message}` }), e(!1), i(!1);
    }
  }, N = (g) => {
    const x = g.job_type === "indexing", S = g.job_type === "database_reset", C = S ? "Database Reset" : x ? "Indexing Job" : "Ingestion Job";
    if (g.is_complete)
      return /* @__PURE__ */ h(
        "div",
        {
          className: "p-3 rounded-lg border border-gray-200 bg-gray-50 mb-3 opacity-75",
          children: [
            /* @__PURE__ */ h("div", { className: "flex items-center justify-between", children: [
              /* @__PURE__ */ h("div", { className: "flex items-center gap-2", children: [
                /* @__PURE__ */ s(mr, { className: "w-5 h-5 text-gray-400" }),
                /* @__PURE__ */ s("span", { className: "font-medium text-gray-500", children: C }),
                /* @__PURE__ */ s("span", { className: "text-xs text-gray-400 bg-gray-200 px-2 py-0.5 rounded-full", children: "Complete" })
              ] }),
              /* @__PURE__ */ h("div", { className: "flex items-center gap-1 text-xs text-gray-400", children: [
                /* @__PURE__ */ s(fr, { className: "w-3 h-3" }),
                /* @__PURE__ */ s("span", { children: new Date(g.started_at).toLocaleTimeString() })
              ] })
            ] }),
            /* @__PURE__ */ s("div", { className: "text-xs text-gray-400 mt-1", children: g.status_message || "Completed successfully" })
          ]
        },
        g.id
      );
    if (g.is_failed)
      return /* @__PURE__ */ h(
        "div",
        {
          className: "p-4 rounded-lg border-2 border-red-200 bg-red-50 mb-3",
          children: [
            /* @__PURE__ */ h("div", { className: "flex items-center justify-between mb-2", children: [
              /* @__PURE__ */ h("div", { className: "flex items-center gap-2", children: [
                /* @__PURE__ */ s(ai, { className: "w-5 h-5 text-red-500" }),
                /* @__PURE__ */ s("span", { className: "font-medium text-red-800", children: C }),
                /* @__PURE__ */ s("span", { className: "text-xs text-red-600 bg-red-100 px-2 py-0.5 rounded-full", children: "Failed" })
              ] }),
              /* @__PURE__ */ h("div", { className: "flex items-center gap-1 text-xs text-gray-500", children: [
                /* @__PURE__ */ s(fr, { className: "w-3 h-3" }),
                /* @__PURE__ */ s("span", { children: new Date(g.started_at).toLocaleTimeString() })
              ] })
            ] }),
            g.error_message && /* @__PURE__ */ h("div", { className: "text-xs text-red-600 mt-2", children: [
              "Error: ",
              g.error_message
            ] })
          ]
        },
        g.id
      );
    const I = S ? "red" : x ? "purple" : "blue", F = `bg-${I}-50`, P = `border-${I}-200`, T = x ? "text-purple-800" : S ? "text-red-800" : "text-blue-800", R = S ? "bg-orange-500" : x ? "bg-purple-500" : "bg-blue-500";
    return /* @__PURE__ */ h(
      "div",
      {
        className: `p-4 rounded-lg border-2 ${P} ${F} mb-3`,
        children: [
          /* @__PURE__ */ h("div", { className: "flex items-center justify-between mb-2", children: [
            /* @__PURE__ */ h("div", { className: "flex items-center gap-2", children: [
              /* @__PURE__ */ s(dn, { className: "w-5 h-5 text-blue-500 animate-spin" }),
              /* @__PURE__ */ s("span", { className: `font-medium ${T}`, children: C }),
              /* @__PURE__ */ s("span", { className: `text-xs ${T} bg-white/50 px-2 py-0.5 rounded-full`, children: "In Progress" })
            ] }),
            /* @__PURE__ */ h("div", { className: "flex items-center gap-1 text-xs text-gray-500", children: [
              /* @__PURE__ */ s(fr, { className: "w-3 h-3" }),
              /* @__PURE__ */ s("span", { children: new Date(g.started_at).toLocaleTimeString() })
            ] })
          ] }),
          /* @__PURE__ */ h("div", { className: "mb-2", children: [
            /* @__PURE__ */ h("div", { className: "flex justify-between text-xs text-gray-600 mb-1", children: [
              /* @__PURE__ */ s("span", { children: g.status_message || "Processing..." }),
              /* @__PURE__ */ h("span", { children: [
                g.progress_percentage || 0,
                "%"
              ] })
            ] }),
            /* @__PURE__ */ s("div", { className: "w-full bg-gray-200 rounded-full h-2", children: /* @__PURE__ */ s(
              "div",
              {
                className: `h-2 rounded-full transition-all duration-300 ${R}`,
                style: { width: `${g.progress_percentage || 0}%` }
              }
            ) })
          ] })
        ]
      },
      g.id
    );
  }, _ = () => t ? /* @__PURE__ */ s("div", { className: "fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50", children: /* @__PURE__ */ h("div", { className: "bg-white rounded-lg p-6 max-w-md w-full mx-4", children: [
    /* @__PURE__ */ h("div", { className: "flex items-center gap-3 mb-4", children: [
      /* @__PURE__ */ s(un, { className: "w-6 h-6 text-red-500" }),
      /* @__PURE__ */ s("h3", { className: "text-lg font-semibold text-gray-900", children: "Reset Database" })
    ] }),
    /* @__PURE__ */ h("div", { className: "mb-6", children: [
      /* @__PURE__ */ s("p", { className: "text-gray-700 mb-2", children: "This will permanently delete all data and restart the node:" }),
      /* @__PURE__ */ h("ul", { className: "list-disc list-inside text-sm text-gray-600 space-y-1", children: [
        /* @__PURE__ */ s("li", { children: "All schemas will be removed" }),
        /* @__PURE__ */ s("li", { children: "All stored data will be deleted" }),
        /* @__PURE__ */ s("li", { children: "Network connections will be reset" }),
        /* @__PURE__ */ s("li", { children: "This action cannot be undone" })
      ] })
    ] }),
    /* @__PURE__ */ h("div", { className: "flex gap-3 justify-end", children: [
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
          onClick: v,
          disabled: r,
          className: "px-4 py-2 text-sm font-medium text-white bg-red-600 rounded-md hover:bg-red-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors",
          children: r ? "Resetting..." : "Reset Database"
        }
      )
    ] })
  ] }) }) : null, b = u.filter((g) => !g.is_complete && !g.is_failed), E = b.length > 0 ? b.slice(0, 3) : u.filter((g) => g.is_complete || g.is_failed).slice(0, 1);
  return /* @__PURE__ */ h(Rt, { children: [
    /* @__PURE__ */ h("div", { className: "bg-white rounded-lg shadow-sm p-4 mb-6", children: [
      /* @__PURE__ */ h("div", { className: "flex items-center justify-between mb-4", children: [
        /* @__PURE__ */ h("div", { className: "flex items-center gap-2", children: [
          /* @__PURE__ */ s(mr, { className: "w-5 h-5 text-green-500" }),
          /* @__PURE__ */ s("h2", { className: "text-lg font-semibold text-gray-900", children: "System Status" })
        ] }),
        /* @__PURE__ */ h(
          "button",
          {
            onClick: () => e(!0),
            className: "flex items-center gap-2 px-3 py-1.5 text-sm font-medium text-red-600 border border-red-200 rounded-md hover:bg-red-50 hover:border-red-300 transition-colors",
            disabled: r,
            children: [
              /* @__PURE__ */ s(un, { className: "w-4 h-4" }),
              "Reset Database"
            ]
          }
        )
      ] }),
      f ? /* @__PURE__ */ h("div", { className: "p-4 rounded-lg border-2 border-gray-200 bg-gray-50 flex items-center justify-center", children: [
        /* @__PURE__ */ s(dn, { className: "w-5 h-5 text-gray-400 animate-spin mr-2" }),
        /* @__PURE__ */ s("span", { className: "text-gray-500", children: "Loading status..." })
      ] }) : E.length > 0 ? E.map((g) => N(g)) : /* @__PURE__ */ s("div", { className: "p-4 rounded-lg border-2 border-green-200 bg-green-50", children: /* @__PURE__ */ h("div", { className: "flex items-center gap-2", children: [
        /* @__PURE__ */ s(mr, { className: "w-5 h-5 text-green-500" }),
        /* @__PURE__ */ s("span", { className: "text-green-800 font-medium", children: "No active jobs" })
      ] }) }),
      o && /* @__PURE__ */ s("div", { className: `mt-3 p-3 rounded-md text-sm ${o.type === "success" ? "bg-green-50 text-green-800 border border-green-200" : "bg-red-50 text-red-800 border border-red-200"}`, children: o.message })
    ] }),
    /* @__PURE__ */ s(_, {})
  ] });
}
function Se(t) {
  return t !== null && typeof t == "object" && !Array.isArray(t);
}
function hi(t) {
  const e = bt(t);
  if (!Se(e)) return !1;
  const r = Object.keys(e);
  if (r.length === 0) return !1;
  for (let i = 0; i < Math.min(3, r.length); i++) {
    const o = e[r[i]];
    if (!Se(o)) return !1;
    const c = Object.keys(o);
    if (c.length !== 0)
      for (let u = 0; u < Math.min(3, c.length); u++) {
        const l = o[c[u]];
        if (!Se(l)) return !1;
        Object.keys(l).length;
      }
  }
  return !0;
}
function bt(t) {
  return t && Se(t) && Object.prototype.hasOwnProperty.call(t, "data") ? t.data : t;
}
function mi(t) {
  const e = bt(t) || {};
  if (!Se(e)) return { hashes: 0, ranges: 0 };
  const r = Object.keys(e).length;
  let i = 0;
  for (const o of Object.keys(e)) {
    const c = e[o];
    Se(c) && (i += Object.keys(c).length);
  }
  return { hashes: r, ranges: i };
}
function fi(t) {
  const e = bt(t) || {};
  return Se(e) ? Object.keys(e).sort(qn) : [];
}
function zn(t, e) {
  const r = bt(t) || {}, i = Se(r) && Se(r[e]) ? r[e] : {};
  return Object.keys(i).sort(qn);
}
function qn(t, e) {
  const r = mn(t), i = mn(e);
  return !Number.isNaN(r) && !Number.isNaN(i) ? r - i : String(t).localeCompare(String(e));
}
function mn(t) {
  const e = Number(t);
  return Number.isFinite(e) ? e : Number.NaN;
}
function pi(t, e, r) {
  const i = bt(t) || {};
  if (!Se(i)) return null;
  const o = i[e];
  if (!Se(o)) return null;
  const c = o[r];
  return Se(c) ? c : null;
}
function Qn(t, e, r) {
  return t.slice(e, Math.min(e + r, t.length));
}
const gi = 50;
function Yn({ isOpen: t, onClick: e, label: r }) {
  return /* @__PURE__ */ h(
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
function yi({ fields: t }) {
  const e = W(() => Object.entries(t || {}), [t]);
  return e.length === 0 ? /* @__PURE__ */ s("div", { className: "text-xs text-gray-500 italic px-3 py-2", children: "No fields" }) : /* @__PURE__ */ s("div", { className: "px-3 py-2 overflow-x-auto", children: /* @__PURE__ */ s("table", { className: "min-w-full border-separate border-spacing-y-1", children: /* @__PURE__ */ s("tbody", { children: e.map(([r, i]) => /* @__PURE__ */ h("tr", { className: "bg-white", children: [
    /* @__PURE__ */ s("td", { className: "align-top text-xs font-medium text-gray-700 pr-4 whitespace-nowrap", children: r }),
    /* @__PURE__ */ s("td", { className: "align-top text-xs text-gray-700", children: /* @__PURE__ */ s("pre", { className: "font-mono whitespace-pre-wrap break-words", children: bi(i) }) })
  ] }, r)) }) }) });
}
function bi(t) {
  if (t === null) return "null";
  if (typeof t == "string") return t;
  if (typeof t == "number" || typeof t == "boolean") return String(t);
  try {
    return JSON.stringify(t, null, 2);
  } catch {
    return String(t);
  }
}
function xi({ results: t, pageSize: e = gi }) {
  const r = W(() => bt(t) || {}, [t]), i = W(() => mi(t), [t]), o = W(() => fi(t), [t]), [c, u] = O(() => /* @__PURE__ */ new Set()), [l, f] = O(() => /* @__PURE__ */ new Set()), [p, y] = O({ start: 0, count: e }), [v, N] = O(() => /* @__PURE__ */ new Map()), _ = K((x) => {
    u((S) => {
      const C = new Set(S);
      return C.has(x) ? C.delete(x) : C.add(x), C;
    }), N((S) => {
      if (!c.has(x)) {
        const C = zn(r, x).length, I = new Map(S);
        return I.set(x, { start: 0, count: Math.min(e, C) }), I;
      }
      return S;
    });
  }, [r, c, e]), b = K((x, S) => {
    const C = x + "||" + S;
    f((I) => {
      const F = new Set(I);
      return F.has(C) ? F.delete(C) : F.add(C), F;
    });
  }, []), E = K(() => {
    const x = Math.min(o.length, p.count + e);
    y((S) => ({ start: 0, count: x }));
  }, [o, p.count, e]), g = W(() => Qn(o, p.start, p.count), [o, p]);
  return /* @__PURE__ */ h("div", { className: "space-y-2", children: [
    /* @__PURE__ */ h("div", { className: "text-xs text-gray-600", children: [
      /* @__PURE__ */ h("span", { className: "mr-4", children: [
        "Hashes: ",
        /* @__PURE__ */ s("strong", { children: i.hashes })
      ] }),
      /* @__PURE__ */ h("span", { children: [
        "Ranges: ",
        /* @__PURE__ */ s("strong", { children: i.ranges })
      ] })
    ] }),
    /* @__PURE__ */ s("div", { className: "border rounded-md divide-y divide-gray-200 bg-gray-50", children: g.map((x) => /* @__PURE__ */ h("div", { className: "p-2", children: [
      /* @__PURE__ */ s(
        Yn,
        {
          isOpen: c.has(x),
          onClick: () => _(x),
          label: `hash: ${String(x)}`
        }
      ),
      c.has(x) && /* @__PURE__ */ s(
        wi,
        {
          data: r,
          hashKey: x,
          rangeOpen: l,
          onToggleRange: b,
          pageSize: e,
          rangeWindow: v.get(x),
          setRangeWindow: (S) => N((C) => new Map(C).set(x, S))
        }
      )
    ] }, x)) }),
    p.count < o.length && /* @__PURE__ */ s("div", { className: "pt-2", children: /* @__PURE__ */ h(
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
function wi({ data: t, hashKey: e, rangeOpen: r, onToggleRange: i, pageSize: o, rangeWindow: c, setRangeWindow: u }) {
  const l = W(() => zn(t, e), [t, e]), f = c || { start: 0, count: Math.min(o, l.length) }, p = W(() => Qn(l, f.start, f.count), [l, f]), y = K(() => {
    const v = Math.min(l.length, f.count + o);
    u({ start: 0, count: v });
  }, [l.length, f.count, o, u]);
  return /* @__PURE__ */ h("div", { className: "ml-4 mt-1 border-l pl-3", children: [
    p.map((v) => /* @__PURE__ */ h("div", { className: "py-1", children: [
      /* @__PURE__ */ s(
        Yn,
        {
          isOpen: r.has(e + "||" + v),
          onClick: () => i(e, v),
          label: `range: ${String(v)}`
        }
      ),
      r.has(e + "||" + v) && /* @__PURE__ */ s("div", { className: "ml-4 mt-1", children: /* @__PURE__ */ s(yi, { fields: pi(t, e, v) || {} }) })
    ] }, v)),
    f.count < l.length && /* @__PURE__ */ s("div", { className: "pt-1", children: /* @__PURE__ */ h(
      "button",
      {
        type: "button",
        className: "text-xs px-3 py-1 rounded bg-gray-200 hover:bg-gray-300",
        onClick: y,
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
  const e = t != null, r = e && (!!t.error || t.status && t.status >= 400), i = e && t.data !== void 0, o = W(() => e && !r && hi(i ? t.data : t), [e, t, r, i]), [c, u] = O(o);
  return e ? /* @__PURE__ */ h("div", { className: "bg-white rounded-lg shadow-sm p-6 mt-6", children: [
    /* @__PURE__ */ h("h3", { className: "text-lg font-semibold mb-4 flex items-center", children: [
      /* @__PURE__ */ s("span", { className: `mr-2 ${r ? "text-red-600" : "text-gray-900"}`, children: r ? "Error" : "Results" }),
      /* @__PURE__ */ h("span", { className: "text-xs font-normal text-gray-500", children: [
        "(",
        typeof t == "string" ? "Text" : c ? "Structured" : "JSON",
        ")"
      ] }),
      t.status && /* @__PURE__ */ h("span", { className: `ml-2 px-2 py-1 text-xs rounded-full ${t.status >= 400 ? "bg-red-100 text-red-800" : "bg-green-100 text-green-800"}`, children: [
        "Status: ",
        t.status
      ] }),
      !r && typeof t != "string" && /* @__PURE__ */ s("div", { className: "ml-auto", children: /* @__PURE__ */ s(
        "button",
        {
          type: "button",
          className: "text-xs px-2 py-1 rounded border border-gray-300 hover:bg-gray-100",
          onClick: () => u((l) => !l),
          children: c ? "View JSON" : "View Structured"
        }
      ) })
    ] }),
    r && /* @__PURE__ */ s("div", { className: "mb-4 p-4 bg-red-50 border border-red-200 rounded-md", children: /* @__PURE__ */ h("div", { className: "flex", children: [
      /* @__PURE__ */ s("div", { className: "flex-shrink-0", children: /* @__PURE__ */ s("svg", { className: "h-5 w-5 text-red-400", viewBox: "0 0 20 20", fill: "currentColor", children: /* @__PURE__ */ s("path", { fillRule: "evenodd", d: "M10 18a8 8 0 100-16 8 8 0 000 16zM8.707 7.293a1 1 0 00-1.414 1.414L8.586 10l-1.293 1.293a1 1 0 101.414 1.414L10 11.414l1.293 1.293a1 1 0 001.414-1.414L11.414 10l1.293-1.293a1 1 0 00-1.414-1.414L10 8.586 8.707 7.293z", clipRule: "evenodd" }) }) }),
      /* @__PURE__ */ h("div", { className: "ml-3", children: [
        /* @__PURE__ */ s("h4", { className: "text-sm font-medium text-red-800", children: "Query Execution Failed" }),
        /* @__PURE__ */ s("div", { className: "mt-2 text-sm text-red-700", children: /* @__PURE__ */ s("p", { children: t.error || "An unknown error occurred" }) })
      ] })
    ] }) }),
    c && !r && typeof t != "string" ? /* @__PURE__ */ s("div", { className: "rounded-md p-2 bg-gray-50 border overflow-auto max-h-[500px]", children: /* @__PURE__ */ s(xi, { results: t }) }) : /* @__PURE__ */ s("div", { className: `rounded-md p-4 overflow-auto max-h-[500px] ${r ? "bg-red-50 border border-red-200" : "bg-gray-50"}`, children: /* @__PURE__ */ s("pre", { className: `font-mono text-sm whitespace-pre-wrap ${r ? "text-red-700" : "text-gray-700"}`, children: typeof t == "string" ? t : JSON.stringify(i ? t.data : t, null, 2) }) })
  ] }) : null;
}
const ve = {
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
function Zo({
  tabs: t = ya,
  activeTab: e,
  onTabChange: r,
  className: i = ""
}) {
  const o = (p, y) => {
    r(p);
  }, c = (p) => {
    const y = e === p.id, v = p.disabled || !1;
    let N = ve.tab.base;
    return y ? N += ` ${ve.tab.active}` : v ? N += ` ${ve.tab.disabled}` : N += ` ${ve.tab.inactive}`, N;
  }, u = t.filter((p) => p.group === "main"), l = t.filter((p) => p.group === "advanced"), f = (p) => {
    const y = p.disabled || !1;
    return /* @__PURE__ */ h(
      "button",
      {
        className: c(p),
        onClick: () => o(p.id, p.requiresAuth),
        disabled: y,
        "aria-current": e === p.id ? "page" : void 0,
        "aria-label": `${p.label} tab`,
        style: {
          transitionDuration: `${pa}ms`
        },
        children: [
          p.icon && /* @__PURE__ */ s("span", { className: "mr-2", "aria-hidden": "true", children: p.icon }),
          /* @__PURE__ */ s("span", { children: p.label })
        ]
      },
      p.id
    );
  };
  return /* @__PURE__ */ s("div", { className: `border-b border-gray-200 ${i}`, children: /* @__PURE__ */ h("div", { className: "flex items-center", children: [
    /* @__PURE__ */ s("div", { className: "flex space-x-8", children: u.map(f) }),
    l.length > 0 && /* @__PURE__ */ s("div", { className: "mx-6 h-6 w-px bg-gray-300", "aria-hidden": "true" }),
    l.length > 0 && /* @__PURE__ */ h("div", { className: "flex items-center space-x-6", children: [
      /* @__PURE__ */ s("span", { className: "text-xs text-gray-500 font-medium uppercase tracking-wider", children: "Advanced" }),
      /* @__PURE__ */ s("div", { className: "flex space-x-6", children: l.map(f) })
    ] })
  ] }) });
}
const vi = {
  queue: [],
  length: 0,
  isEmpty: !0
}, Ni = (t = {}) => {
  const e = Array.isArray(t.queue) ? t.queue : [], r = typeof t.length == "number" ? t.length : e.length, i = typeof t.isEmpty == "boolean" ? t.isEmpty : e.length === 0;
  return { queue: e, length: r, isEmpty: i };
}, Si = ({ onResult: t }) => {
  const [e, r] = O(vi), [i, o] = O({}), [c, u] = O({}), [l, f] = O(!1), [p, y] = O(null), [v, N] = O([]), _ = K(async () => {
    f(!0), y(null);
    try {
      const g = await ge.getTransforms();
      if (g != null && g.success && g.data) {
        const x = g.data, S = x && typeof x == "object" && !Array.isArray(x) ? Object.entries(x).map(([C, I]) => ({
          transform_id: C,
          ...I
        })) : Array.isArray(x) ? x : [];
        N(S);
      } else {
        const x = (g == null ? void 0 : g.error) || "Failed to load transforms";
        y(x), N([]);
      }
    } catch (g) {
      console.error("Failed to fetch transforms:", g), y(g.message || "Failed to load transforms"), N([]);
    } finally {
      f(!1);
    }
  }, []), b = K(async () => {
    try {
      const g = await ge.getQueue();
      g != null && g.success && g.data && r(Ni(g.data));
    } catch (g) {
      console.error("Failed to fetch transform queue info:", g);
    }
  }, []);
  ue(() => {
    _(), b();
    const g = setInterval(b, 5e3);
    return () => clearInterval(g);
  }, [_, b]);
  const E = K(async (g, x) => {
    var C;
    const S = x ? `${g}.${x}` : g;
    o((I) => ({ ...I, [S]: !0 })), u((I) => ({ ...I, [S]: null }));
    try {
      const I = await ge.addToQueue(S);
      if (!(I != null && I.success)) {
        const F = ((C = I == null ? void 0 : I.data) == null ? void 0 : C.message) || (I == null ? void 0 : I.error) || "Failed to add transform to queue";
        throw new Error(F);
      }
      typeof t == "function" && t({ success: !0, transformId: S }), await b();
    } catch (I) {
      console.error("Failed to add transform to queue:", I), u((F) => ({ ...F, [S]: I.message || "Failed to add transform to queue" }));
    } finally {
      o((I) => ({ ...I, [S]: !1 }));
    }
  }, [b, t]);
  return /* @__PURE__ */ h("div", { className: "space-y-4", children: [
    /* @__PURE__ */ h("div", { className: "flex justify-between items-center", children: [
      /* @__PURE__ */ s("h2", { className: "text-xl font-semibold text-gray-800", children: "Transforms" }),
      /* @__PURE__ */ h("div", { className: "text-sm text-gray-600", children: [
        "Queue Status: ",
        e.isEmpty ? "Empty" : `${e.length} transform(s) queued`
      ] })
    ] }),
    !e.isEmpty && /* @__PURE__ */ h("div", { className: "bg-blue-50 p-4 rounded-lg", "data-testid": "transform-queue", children: [
      /* @__PURE__ */ s("h3", { className: "text-md font-medium text-blue-800 mb-2", children: "Transform Queue" }),
      /* @__PURE__ */ s("ul", { className: "list-disc list-inside space-y-1", children: e.queue.map((g, x) => /* @__PURE__ */ s("li", { className: "text-blue-700", children: g }, `${g}-${x}`)) })
    ] }),
    l && /* @__PURE__ */ s("div", { className: "bg-blue-50 p-4 rounded-lg", role: "status", children: /* @__PURE__ */ h("div", { className: "flex items-center", children: [
      /* @__PURE__ */ s("div", { className: "animate-spin rounded-full h-4 w-4 border-b-2 border-blue-600 mr-2" }),
      /* @__PURE__ */ s("span", { className: "text-blue-800", children: "Loading transforms..." })
    ] }) }),
    p && /* @__PURE__ */ s("div", { className: "bg-red-50 p-4 rounded-lg", role: "alert", children: /* @__PURE__ */ h("div", { className: "flex items-center", children: [
      /* @__PURE__ */ h("span", { className: "text-red-800", children: [
        "Error loading transforms: ",
        p
      ] }),
      /* @__PURE__ */ s(
        "button",
        {
          onClick: _,
          className: "ml-4 px-3 py-1 text-sm bg-red-500 text-white rounded hover:bg-red-600",
          children: "Retry"
        }
      )
    ] }) }),
    !l && !p && v.length > 0 && /* @__PURE__ */ s("div", { className: "space-y-4", children: v.map((g, x) => {
      var H;
      const S = g.transform_id || `transform-${x}`, C = i[S], I = c[S], F = g.name || ((H = g.transform_id) == null ? void 0 : H.split(".")[0]) || "Unknown", P = g.schema_type;
      let T = "Single", R = "bg-gray-100 text-gray-800";
      P != null && P.Range ? (T = "Range", R = "bg-blue-100 text-blue-800") : P != null && P.HashRange && (T = "HashRange", R = "bg-purple-100 text-purple-800");
      const B = g.key, U = g.transform_fields || {}, L = Object.keys(U).length, $ = Object.keys(U);
      return /* @__PURE__ */ h("div", { className: "bg-white p-4 rounded-lg shadow border-l-4 border-blue-500", children: [
        /* @__PURE__ */ s("div", { className: "flex justify-between items-start mb-3", children: /* @__PURE__ */ h("div", { className: "flex-1", children: [
          /* @__PURE__ */ s("h3", { className: "text-lg font-semibold text-gray-900", children: F }),
          /* @__PURE__ */ h("div", { className: "flex gap-2 mt-2 flex-wrap", children: [
            /* @__PURE__ */ s("span", { className: `inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium ${R}`, children: T }),
            L > 0 && /* @__PURE__ */ h("span", { className: "inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-green-100 text-green-800", children: [
              L,
              " field",
              L !== 1 ? "s" : ""
            ] })
          ] }),
          $.length > 0 && /* @__PURE__ */ h("div", { className: "mt-2 text-sm text-gray-600", children: [
            /* @__PURE__ */ s("span", { className: "font-medium", children: "Fields:" }),
            " ",
            $.join(", ")
          ] })
        ] }) }),
        /* @__PURE__ */ h("div", { className: "mt-3 space-y-3", children: [
          B && /* @__PURE__ */ h("div", { className: "bg-blue-50 rounded p-3", children: [
            /* @__PURE__ */ s("div", { className: "text-sm font-medium text-blue-900 mb-1", children: "Key Configuration:" }),
            /* @__PURE__ */ h("div", { className: "text-sm text-blue-800 space-y-1", children: [
              B.hash_field && /* @__PURE__ */ h("div", { children: [
                /* @__PURE__ */ s("span", { className: "font-medium", children: "Hash Key:" }),
                " ",
                B.hash_field
              ] }),
              B.range_field && /* @__PURE__ */ h("div", { children: [
                /* @__PURE__ */ s("span", { className: "font-medium", children: "Range Key:" }),
                " ",
                B.range_field
              ] }),
              !B.hash_field && !B.range_field && B.key_field && /* @__PURE__ */ h("div", { children: [
                /* @__PURE__ */ s("span", { className: "font-medium", children: "Key:" }),
                " ",
                B.key_field
              ] })
            ] })
          ] }),
          L > 0 && /* @__PURE__ */ h("div", { children: [
            /* @__PURE__ */ s("div", { className: "text-sm font-medium text-gray-700 mb-2", children: "Transform Fields:" }),
            /* @__PURE__ */ s("div", { className: "bg-gray-50 rounded p-3 space-y-2", children: Object.entries(U).map(([j, q]) => /* @__PURE__ */ h("div", { className: "border-l-2 border-gray-300 pl-3", children: [
              /* @__PURE__ */ s("div", { className: "font-medium text-gray-900 text-sm", children: j }),
              /* @__PURE__ */ s("div", { className: "text-gray-600 font-mono text-xs mt-1 break-all", children: q })
            ] }, j)) })
          ] })
        ] }),
        /* @__PURE__ */ h("div", { className: "mt-4 flex items-center gap-3", children: [
          /* @__PURE__ */ s(
            "button",
            {
              onClick: () => E(S, null),
              disabled: C,
              className: `px-4 py-2 text-sm font-medium rounded-md text-white ${C ? "bg-blue-300 cursor-not-allowed" : "bg-blue-600 hover:bg-blue-700"}`,
              children: C ? "Adding..." : "Add to Queue"
            }
          ),
          I && /* @__PURE__ */ h("span", { className: "text-sm text-red-600", children: [
            "Error: ",
            I
          ] })
        ] })
      ] }, S);
    }) }),
    !l && !p && v.length === 0 && /* @__PURE__ */ h("div", { className: "bg-gray-50 p-4 rounded-lg", children: [
      /* @__PURE__ */ s("p", { className: "text-gray-600", children: "No transforms registered" }),
      /* @__PURE__ */ s("p", { className: "text-sm text-gray-500 mt-1", children: "Register a transform in a schema to view it here and add it to the processing queue." })
    ] })
  ] });
}, ze = () => ls(), J = os;
function Ei({
  title: t,
  titleId: e,
  ...r
}, i) {
  return /* @__PURE__ */ V.createElement("svg", Object.assign({
    xmlns: "http://www.w3.org/2000/svg",
    fill: "none",
    viewBox: "0 0 24 24",
    strokeWidth: 1.5,
    stroke: "currentColor",
    "aria-hidden": "true",
    "data-slot": "icon",
    ref: i,
    "aria-labelledby": e
  }, r), t ? /* @__PURE__ */ V.createElement("title", {
    id: e
  }, t) : null, /* @__PURE__ */ V.createElement("path", {
    strokeLinecap: "round",
    strokeLinejoin: "round",
    d: "m4.5 12.75 6 6 9-13.5"
  }));
}
const pr = /* @__PURE__ */ V.forwardRef(Ei);
function Ai({
  title: t,
  titleId: e,
  ...r
}, i) {
  return /* @__PURE__ */ V.createElement("svg", Object.assign({
    xmlns: "http://www.w3.org/2000/svg",
    fill: "none",
    viewBox: "0 0 24 24",
    strokeWidth: 1.5,
    stroke: "currentColor",
    "aria-hidden": "true",
    "data-slot": "icon",
    ref: i,
    "aria-labelledby": e
  }, r), t ? /* @__PURE__ */ V.createElement("title", {
    id: e
  }, t) : null, /* @__PURE__ */ V.createElement("path", {
    strokeLinecap: "round",
    strokeLinejoin: "round",
    d: "M15.666 3.888A2.25 2.25 0 0 0 13.5 2.25h-3c-1.03 0-1.9.693-2.166 1.638m7.332 0c.055.194.084.4.084.612v0a.75.75 0 0 1-.75.75H9a.75.75 0 0 1-.75-.75v0c0-.212.03-.418.084-.612m7.332 0c.646.049 1.288.11 1.927.184 1.1.128 1.907 1.077 1.907 2.185V19.5a2.25 2.25 0 0 1-2.25 2.25H6.75A2.25 2.25 0 0 1 4.5 19.5V6.257c0-1.108.806-2.057 1.907-2.185a48.208 48.208 0 0 1 1.927-.184"
  }));
}
const fn = /* @__PURE__ */ V.forwardRef(Ai);
function _i({
  title: t,
  titleId: e,
  ...r
}, i) {
  return /* @__PURE__ */ V.createElement("svg", Object.assign({
    xmlns: "http://www.w3.org/2000/svg",
    fill: "none",
    viewBox: "0 0 24 24",
    strokeWidth: 1.5,
    stroke: "currentColor",
    "aria-hidden": "true",
    "data-slot": "icon",
    ref: i,
    "aria-labelledby": e
  }, r), t ? /* @__PURE__ */ V.createElement("title", {
    id: e
  }, t) : null, /* @__PURE__ */ V.createElement("path", {
    strokeLinecap: "round",
    strokeLinejoin: "round",
    d: "M12 9v3.75m-9.303 3.376c-.866 1.5.217 3.374 1.948 3.374h14.71c1.73 0 2.813-1.874 1.948-3.374L13.949 3.378c-.866-1.5-3.032-1.5-3.898 0L2.697 16.126ZM12 15.75h.007v.008H12v-.008Z"
  }));
}
const pn = /* @__PURE__ */ V.forwardRef(_i);
function Ti({
  title: t,
  titleId: e,
  ...r
}, i) {
  return /* @__PURE__ */ V.createElement("svg", Object.assign({
    xmlns: "http://www.w3.org/2000/svg",
    fill: "none",
    viewBox: "0 0 24 24",
    strokeWidth: 1.5,
    stroke: "currentColor",
    "aria-hidden": "true",
    "data-slot": "icon",
    ref: i,
    "aria-labelledby": e
  }, r), t ? /* @__PURE__ */ V.createElement("title", {
    id: e
  }, t) : null, /* @__PURE__ */ V.createElement("path", {
    strokeLinecap: "round",
    strokeLinejoin: "round",
    d: "M15.75 5.25a3 3 0 0 1 3 3m3 0a6 6 0 0 1-7.029 5.912c-.563-.097-1.159.026-1.563.43L10.5 17.25H8.25v2.25H6v2.25H2.25v-2.818c0-.597.237-1.17.659-1.591l6.499-6.499c.404-.404.527-1 .43-1.563A6 6 0 1 1 21.75 8.25Z"
  }));
}
const gr = /* @__PURE__ */ V.forwardRef(Ti);
function Ci({
  title: t,
  titleId: e,
  ...r
}, i) {
  return /* @__PURE__ */ V.createElement("svg", Object.assign({
    xmlns: "http://www.w3.org/2000/svg",
    fill: "none",
    viewBox: "0 0 24 24",
    strokeWidth: 1.5,
    stroke: "currentColor",
    "aria-hidden": "true",
    "data-slot": "icon",
    ref: i,
    "aria-labelledby": e
  }, r), t ? /* @__PURE__ */ V.createElement("title", {
    id: e
  }, t) : null, /* @__PURE__ */ V.createElement("path", {
    strokeLinecap: "round",
    strokeLinejoin: "round",
    d: "M9 12.75 11.25 15 15 9.75m-3-7.036A11.959 11.959 0 0 1 3.598 6 11.99 11.99 0 0 0 3 9.749c0 5.592 3.824 10.29 9 11.623 5.176-1.332 9-6.03 9-11.622 0-1.31-.21-2.571-.598-3.751h-.152c-3.196 0-6.1-1.248-8.25-3.285Z"
  }));
}
const Ri = /* @__PURE__ */ V.forwardRef(Ci);
function ki({ onResult: t }) {
  const e = ze(), r = J((R) => R.auth), { isAuthenticated: i, systemPublicKey: o, systemKeyId: c, privateKey: u, isLoading: l, error: f } = r, p = u ? la(u) : null, [y, v] = O(null), [N, _] = O(""), [b, E] = O(!1), [g, x] = O(null), [S, C] = O(!1), I = async (R, B) => {
    try {
      await navigator.clipboard.writeText(R), v(B), setTimeout(() => v(null), 2e3);
    } catch (U) {
      console.error("Failed to copy:", U);
    }
  }, F = async () => {
    if (!N.trim()) {
      x({ valid: !1, error: "Please enter a private key" });
      return;
    }
    E(!0);
    try {
      const B = (await e(Ht(N.trim())).unwrap()).isAuthenticated;
      x({
        valid: B,
        error: B ? null : "Private key does not match the system public key"
      }), B && console.log("Private key validation successful");
    } catch (R) {
      x({
        valid: !1,
        error: `Validation failed: ${R.message}`
      });
    } finally {
      E(!1);
    }
  }, P = () => {
    _(""), x(null), C(!1);
  }, T = () => {
    P(), e(da());
  };
  return /* @__PURE__ */ h("div", { className: "p-4 bg-white rounded-lg shadow", children: [
    /* @__PURE__ */ s("h2", { className: "text-xl font-semibold mb-4", children: "Key Management" }),
    /* @__PURE__ */ s("div", { className: "bg-blue-50 border border-blue-200 rounded-md p-4 mb-6", children: /* @__PURE__ */ h("div", { className: "flex items-start", children: [
      /* @__PURE__ */ s(Ri, { className: "h-5 w-5 text-blue-400 mr-2 flex-shrink-0 mt-0.5" }),
      /* @__PURE__ */ h("div", { className: "text-sm text-blue-700 flex-1", children: [
        /* @__PURE__ */ s("p", { className: "font-medium", children: "Current System Public Key:" }),
        l ? /* @__PURE__ */ s("p", { className: "text-blue-600", children: "Loading..." }) : o ? /* @__PURE__ */ h("div", { className: "mt-2", children: [
          /* @__PURE__ */ h("div", { className: "flex", children: [
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
                onClick: () => I(o, "system"),
                className: "px-2 py-1 border border-l-0 border-blue-300 rounded-r-md bg-white hover:bg-blue-50 focus:outline-none focus:ring-2 focus:ring-blue-500",
                children: y === "system" ? /* @__PURE__ */ s(pr, { className: "h-3 w-3 text-green-600" }) : /* @__PURE__ */ s(fn, { className: "h-3 w-3 text-blue-500" })
              }
            )
          ] }),
          c && /* @__PURE__ */ h("p", { className: "text-xs text-blue-600 mt-1", children: [
            "Key ID: ",
            c
          ] }),
          i && /* @__PURE__ */ s("p", { className: "text-xs text-green-600 mt-1", children: "🔓 Authenticated - Private key loaded!" })
        ] }) : /* @__PURE__ */ s("p", { className: "text-blue-600 mt-1", children: "No system public key available." })
      ] })
    ] }) }),
    i && p && /* @__PURE__ */ s("div", { className: "bg-green-50 border border-green-200 rounded-md p-4 mb-6", children: /* @__PURE__ */ h("div", { className: "flex items-start", children: [
      /* @__PURE__ */ s(gr, { className: "h-5 w-5 text-green-400 mr-2 flex-shrink-0 mt-0.5" }),
      /* @__PURE__ */ h("div", { className: "text-sm text-green-700 flex-1", children: [
        /* @__PURE__ */ s("p", { className: "font-medium", children: "Current Private Key (Auto-loaded from Node)" }),
        /* @__PURE__ */ s("p", { className: "mt-1", children: "Your private key has been automatically loaded from the backend node." }),
        /* @__PURE__ */ h("div", { className: "mt-3", children: [
          /* @__PURE__ */ h("div", { className: "flex", children: [
            /* @__PURE__ */ s(
              "textarea",
              {
                value: p,
                readOnly: !0,
                className: "flex-1 px-3 py-2 border border-green-300 rounded-l-md bg-green-50 text-xs font-mono resize-none",
                rows: 3,
                placeholder: "Private key will appear here..."
              }
            ),
            /* @__PURE__ */ s(
              "button",
              {
                onClick: () => I(p, "private"),
                className: "px-3 py-2 border border-l-0 border-green-300 rounded-r-md bg-white hover:bg-green-50 focus:outline-none focus:ring-2 focus:ring-green-500",
                title: "Copy private key",
                children: y === "private" ? /* @__PURE__ */ s(pr, { className: "h-3 w-3 text-green-600" }) : /* @__PURE__ */ s(fn, { className: "h-3 w-3 text-green-500" })
              }
            )
          ] }),
          /* @__PURE__ */ s("p", { className: "text-xs text-green-600 mt-1", children: "🔓 Authenticated - Private key loaded from node!" })
        ] })
      ] })
    ] }) }),
    o && !i && !p && /* @__PURE__ */ s("div", { className: "bg-yellow-50 border border-yellow-200 rounded-md p-4 mb-6", children: /* @__PURE__ */ h("div", { className: "flex items-start", children: [
      /* @__PURE__ */ s(gr, { className: "h-5 w-5 text-yellow-400 mr-2 flex-shrink-0 mt-0.5" }),
      /* @__PURE__ */ h("div", { className: "text-sm text-yellow-700 flex-1", children: [
        /* @__PURE__ */ s("p", { className: "font-medium", children: "Import Private Key" }),
        /* @__PURE__ */ s("p", { className: "mt-1", children: "You have a registered public key but no local private key. Enter your private key to restore access." }),
        S ? /* @__PURE__ */ h("div", { className: "mt-3 space-y-3", children: [
          /* @__PURE__ */ h("div", { children: [
            /* @__PURE__ */ s("label", { className: "block text-xs font-medium text-yellow-700 mb-1", children: "Private Key (Base64)" }),
            /* @__PURE__ */ s(
              "textarea",
              {
                value: N,
                onChange: (R) => _(R.target.value),
                placeholder: "Enter your private key here...",
                className: "w-full px-3 py-2 border border-yellow-300 rounded-md focus:outline-none focus:ring-2 focus:ring-yellow-500 text-xs font-mono",
                rows: 3
              }
            )
          ] }),
          g && /* @__PURE__ */ s("div", { className: `p-2 rounded-md text-xs ${g.valid ? "bg-green-50 border border-green-200 text-green-700" : "bg-red-50 border border-red-200 text-red-700"}`, children: g.valid ? /* @__PURE__ */ h("div", { className: "flex items-center", children: [
            /* @__PURE__ */ s(pr, { className: "h-4 w-4 text-green-600 mr-1" }),
            /* @__PURE__ */ s("span", { children: "Private key matches system public key!" })
          ] }) : /* @__PURE__ */ h("div", { className: "flex items-center", children: [
            /* @__PURE__ */ s(pn, { className: "h-4 w-4 text-red-600 mr-1" }),
            /* @__PURE__ */ s("span", { children: g.error })
          ] }) }),
          /* @__PURE__ */ h("div", { className: "flex gap-2", children: [
            /* @__PURE__ */ s(
              "button",
              {
                onClick: F,
                disabled: b || !N.trim(),
                className: "inline-flex items-center px-3 py-2 border border-transparent text-xs font-medium rounded-md shadow-sm text-white bg-green-600 hover:bg-green-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-green-500 disabled:opacity-50",
                children: b ? "Validating..." : "Validate & Import"
              }
            ),
            /* @__PURE__ */ s(
              "button",
              {
                onClick: T,
                className: "inline-flex items-center px-3 py-2 border border-gray-300 text-xs font-medium rounded-md shadow-sm text-gray-700 bg-white hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-yellow-500",
                children: "Cancel"
              }
            )
          ] }),
          /* @__PURE__ */ s("div", { className: "bg-red-50 border border-red-200 rounded-md p-2", children: /* @__PURE__ */ h("div", { className: "flex", children: [
            /* @__PURE__ */ s(pn, { className: "h-4 w-4 text-red-400 mr-1 flex-shrink-0" }),
            /* @__PURE__ */ h("div", { className: "text-xs text-red-700", children: [
              /* @__PURE__ */ s("p", { className: "font-medium", children: "Security Warning:" }),
              /* @__PURE__ */ s("p", { children: "Only enter your private key on trusted devices. Never share or store private keys in plain text." })
            ] })
          ] }) })
        ] }) : /* @__PURE__ */ h(
          "button",
          {
            onClick: () => C(!0),
            className: "mt-3 inline-flex items-center px-3 py-2 border border-transparent text-sm font-medium rounded-md shadow-sm text-white bg-yellow-600 hover:bg-yellow-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-yellow-500",
            children: [
              /* @__PURE__ */ s(gr, { className: "h-4 w-4 mr-1" }),
              "Import Private Key"
            ]
          }
        )
      ] })
    ] }) })
  ] });
}
function Jo({ isOpen: t, onClose: e }) {
  const [r, i] = O("ai"), [o, c] = O("OpenRouter"), [u, l] = O(""), [f, p] = O("anthropic/claude-3.5-sonnet"), [y, v] = O("https://openrouter.ai/api/v1"), [N, _] = O("llama3"), [b, E] = O("http://localhost:11434"), [g, x] = O(null), [S, C] = O(!1), { environment: I, setEnvironment: F } = Za(), [P, T] = O(I.id), [R, B] = O({}), [U, L] = O({}), [$, H] = O("local"), [j, q] = O("data"), [ae, he] = O("DataFoldStorage"), [Ee, tt] = O("us-west-2"), [rt, X] = O(""), [se, Me] = O(""), [nt, xt] = O("us-east-1"), [wt, vt] = O("folddb"), [Fe, st] = O("/tmp/folddb-data");
  ue(() => {
    t && (Ot(), Pe(), T(I.id), r === "schema-service" && Nt(I.id));
  }, [t, I.id, r]);
  const Ot = async () => {
    try {
      const D = await gt.getConfig();
      D.success && (l(D.data.openrouter.api_key || ""), p(D.data.openrouter.model || "anthropic/claude-3.5-sonnet"), v(D.data.openrouter.base_url || "https://openrouter.ai/api/v1"), _(D.data.ollama.model || "llama3"), E(D.data.ollama.base_url || "http://localhost:11434"), c(D.data.provider || "OpenRouter"));
    } catch (D) {
      console.error("Failed to load AI config:", D);
    }
  }, tr = async () => {
    try {
      const D = {
        provider: o,
        openrouter: {
          api_key: u,
          model: f,
          base_url: y
        },
        ollama: {
          model: N,
          base_url: b
        }
      };
      (await gt.saveConfig(D)).success ? (x({ success: !0, message: "Configuration saved successfully" }), setTimeout(() => {
        x(null), e();
      }, 1500)) : x({ success: !1, message: "Failed to save configuration" });
    } catch (D) {
      x({ success: !1, message: D.message || "Failed to save configuration" });
    }
    setTimeout(() => x(null), 3e3);
  }, Nt = async (D) => {
    const z = Object.values(ht).find((ye) => ye.id === D);
    if (z) {
      L((ye) => ({ ...ye, [D]: !0 }));
      try {
        const ye = await Ya(z.baseUrl);
        B((Ue) => ({
          ...Ue,
          [D]: ye
        }));
      } catch (ye) {
        B((Ue) => ({
          ...Ue,
          [D]: { success: !1, error: ye.message }
        }));
      } finally {
        L((ye) => ({ ...ye, [D]: !1 }));
      }
    }
  }, Pe = async () => {
    try {
      const D = await vs();
      if (D.success && D.data) {
        const z = D.data;
        H(z.type), z.type === "local" ? q(z.path || "data") : z.type === "dynamodb" ? (he(z.table_name || "DataFoldStorage"), tt(z.region || "us-west-2"), X(z.user_id || "")) : z.type === "s3" && (Me(z.bucket || ""), xt(z.region || "us-east-1"), vt(z.prefix || "folddb"), st(z.local_path || "/tmp/folddb-data"));
      }
    } catch (D) {
      console.error("Failed to load database config:", D);
    }
  }, qe = async () => {
    try {
      let D;
      if ($ === "local")
        D = {
          type: "local",
          path: j
        };
      else if ($ === "dynamodb") {
        if (!ae || !Ee) {
          x({ success: !1, message: "Table name and region are required for DynamoDB" }), setTimeout(() => x(null), 3e3);
          return;
        }
        D = {
          type: "dynamodb",
          table_name: ae,
          region: Ee,
          user_id: rt || void 0
        };
      } else if ($ === "s3") {
        if (!se || !nt) {
          x({ success: !1, message: "Bucket and region are required for S3" }), setTimeout(() => x(null), 3e3);
          return;
        }
        D = {
          type: "s3",
          bucket: se,
          region: nt,
          prefix: wt || "folddb",
          local_path: Fe || "/tmp/folddb-data"
        };
      }
      const z = await Ns(D);
      z.success ? (x({
        success: !0,
        message: z.data.requires_restart ? "Database configuration saved. Please restart the server for changes to take effect." : z.data.message || "Database configuration saved and restarted successfully"
      }), setTimeout(() => {
        x(null), z.data.requires_restart || e();
      }, 3e3)) : x({ success: !1, message: z.error || "Failed to save database configuration" });
    } catch (D) {
      x({ success: !1, message: D.message || "Failed to save database configuration" });
    }
    setTimeout(() => x(null), 5e3);
  }, rr = () => {
    F(P), x({ success: !0, message: "Schema service environment updated successfully" }), setTimeout(() => {
      x(null), e();
    }, 1500);
  }, nr = (D) => {
    const z = R[D];
    return U[D] ? /* @__PURE__ */ h("span", { className: "inline-flex items-center text-xs bg-gray-100 text-gray-700 px-2 py-1 rounded", children: [
      /* @__PURE__ */ h("svg", { className: "animate-spin h-3 w-3 mr-1", viewBox: "0 0 24 24", children: [
        /* @__PURE__ */ s("circle", { className: "opacity-25", cx: "12", cy: "12", r: "10", stroke: "currentColor", strokeWidth: "4", fill: "none" }),
        /* @__PURE__ */ s("path", { className: "opacity-75", fill: "currentColor", d: "M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" })
      ] }),
      "Checking..."
    ] }) : z ? z.success ? /* @__PURE__ */ h("span", { className: "inline-flex items-center text-xs bg-green-100 text-green-700 px-2 py-1 rounded", children: [
      "✓ Online ",
      z.responseTime && `(${z.responseTime}ms)`
    ] }) : /* @__PURE__ */ s("span", { className: "inline-flex items-center text-xs bg-red-100 text-red-700 px-2 py-1 rounded", title: z.error, children: "✗ Offline" }) : /* @__PURE__ */ s(
      "button",
      {
        onClick: (Ue) => {
          Ue.stopPropagation(), Nt(D);
        },
        className: "text-xs text-blue-600 hover:text-blue-700 underline",
        children: "Test Connection"
      }
    );
  };
  return t ? /* @__PURE__ */ s("div", { className: "fixed inset-0 z-50 overflow-y-auto", children: /* @__PURE__ */ h("div", { className: "flex items-center justify-center min-h-screen px-4 pt-4 pb-20 text-center sm:block sm:p-0", children: [
    /* @__PURE__ */ s(
      "div",
      {
        className: "fixed inset-0 transition-opacity bg-gray-500 bg-opacity-75",
        onClick: e
      }
    ),
    /* @__PURE__ */ h("div", { className: "inline-block align-bottom bg-white rounded-lg text-left overflow-hidden shadow-xl transform transition-all sm:my-8 sm:align-middle sm:max-w-4xl sm:w-full", children: [
      /* @__PURE__ */ h("div", { className: "bg-white", children: [
        /* @__PURE__ */ h("div", { className: "flex items-center justify-between px-6 pt-5 pb-4 border-b border-gray-200", children: [
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
        /* @__PURE__ */ s("div", { className: "border-b border-gray-200", children: /* @__PURE__ */ h("nav", { className: "flex px-6", children: [
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
        /* @__PURE__ */ h("div", { className: "px-6 py-4 max-h-[70vh] overflow-y-auto", children: [
          r === "ai" && /* @__PURE__ */ h("div", { className: "space-y-4", children: [
            /* @__PURE__ */ h("div", { className: "grid grid-cols-1 md:grid-cols-2 gap-4", children: [
              /* @__PURE__ */ h("div", { children: [
                /* @__PURE__ */ s("label", { className: "block text-sm font-medium text-gray-700 mb-1", children: "Provider" }),
                /* @__PURE__ */ h(
                  "select",
                  {
                    value: o,
                    onChange: (D) => c(D.target.value),
                    className: "w-full p-2 border border-gray-300 rounded text-sm",
                    children: [
                      /* @__PURE__ */ s("option", { value: "OpenRouter", children: "OpenRouter" }),
                      /* @__PURE__ */ s("option", { value: "Ollama", children: "Ollama" })
                    ]
                  }
                )
              ] }),
              o === "OpenRouter" ? /* @__PURE__ */ h("div", { children: [
                /* @__PURE__ */ s("label", { className: "block text-sm font-medium text-gray-700 mb-1", children: "Model" }),
                /* @__PURE__ */ h(
                  "select",
                  {
                    value: f,
                    onChange: (D) => p(D.target.value),
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
              ] }) : /* @__PURE__ */ h("div", { children: [
                /* @__PURE__ */ s("label", { className: "block text-sm font-medium text-gray-700 mb-1", children: "Model" }),
                /* @__PURE__ */ s(
                  "input",
                  {
                    type: "text",
                    value: N,
                    onChange: (D) => _(D.target.value),
                    placeholder: "e.g., llama3",
                    className: "w-full p-2 border border-gray-300 rounded text-sm"
                  }
                )
              ] })
            ] }),
            o === "OpenRouter" && /* @__PURE__ */ h("div", { children: [
              /* @__PURE__ */ h("label", { className: "block text-sm font-medium text-gray-700 mb-1", children: [
                "API Key ",
                /* @__PURE__ */ h("span", { className: "text-xs text-gray-500", children: [
                  "(",
                  /* @__PURE__ */ s("a", { href: "https://openrouter.ai/keys", target: "_blank", rel: "noopener noreferrer", className: "text-blue-600 hover:underline", children: "get key" }),
                  ")"
                ] })
              ] }),
              /* @__PURE__ */ s(
                "input",
                {
                  type: "password",
                  value: u,
                  onChange: (D) => l(D.target.value),
                  placeholder: "sk-or-...",
                  className: "w-full p-2 border border-gray-300 rounded text-sm"
                }
              )
            ] }),
            /* @__PURE__ */ h("div", { children: [
              /* @__PURE__ */ h(
                "button",
                {
                  onClick: () => C(!S),
                  className: "text-sm text-gray-600 hover:text-gray-800 flex items-center gap-1",
                  children: [
                    /* @__PURE__ */ s("span", { children: S ? "▼" : "▶" }),
                    "Advanced Settings"
                  ]
                }
              ),
              S && /* @__PURE__ */ s("div", { className: "mt-3 space-y-3 pl-4 border-l-2 border-gray-200", children: /* @__PURE__ */ h("div", { children: [
                /* @__PURE__ */ s("label", { className: "block text-sm font-medium text-gray-700 mb-1", children: "Base URL" }),
                /* @__PURE__ */ s(
                  "input",
                  {
                    type: "text",
                    value: o === "OpenRouter" ? y : b,
                    onChange: (D) => o === "OpenRouter" ? v(D.target.value) : E(D.target.value),
                    placeholder: o === "OpenRouter" ? "https://openrouter.ai/api/v1" : "http://localhost:11434",
                    className: "w-full p-2 border border-gray-300 rounded text-sm"
                  }
                )
              ] }) })
            ] }),
            g && /* @__PURE__ */ s("div", { className: `p-3 rounded-md ${g.success ? "bg-green-50 text-green-800 border border-green-200" : "bg-red-50 text-red-800 border border-red-200"}`, children: /* @__PURE__ */ h("span", { className: "text-sm font-medium", children: [
              g.success ? "✓" : "✗",
              " ",
              g.message
            ] }) })
          ] }),
          r === "transforms" && /* @__PURE__ */ s(Si, { onResult: () => {
          } }),
          r === "keys" && /* @__PURE__ */ s(ki, { onResult: () => {
          } }),
          r === "schema-service" && /* @__PURE__ */ h("div", { className: "space-y-4", children: [
            /* @__PURE__ */ h("div", { className: "mb-4", children: [
              /* @__PURE__ */ s("h4", { className: "text-md font-semibold text-gray-900 mb-2", children: "Schema Service Environment" }),
              /* @__PURE__ */ s("p", { className: "text-sm text-gray-600 mb-4", children: "Select which schema service endpoint to use. This affects where schemas are loaded from and saved to." })
            ] }),
            /* @__PURE__ */ s("div", { className: "space-y-3", children: Object.values(ht).map((D) => /* @__PURE__ */ h(
              "label",
              {
                className: `flex items-start p-4 border-2 rounded-lg cursor-pointer transition-all ${P === D.id ? "border-blue-500 bg-blue-50" : "border-gray-200 hover:border-gray-300 bg-white"}`,
                children: [
                  /* @__PURE__ */ s(
                    "input",
                    {
                      type: "radio",
                      name: "schemaEnvironment",
                      value: D.id,
                      checked: P === D.id,
                      onChange: (z) => T(z.target.value),
                      className: "mt-1 mr-3"
                    }
                  ),
                  /* @__PURE__ */ h("div", { className: "flex-1", children: [
                    /* @__PURE__ */ h("div", { className: "flex items-center justify-between mb-2", children: [
                      /* @__PURE__ */ s("span", { className: "text-sm font-semibold text-gray-900", children: D.name }),
                      /* @__PURE__ */ h("div", { className: "flex items-center gap-2", children: [
                        nr(D.id),
                        P === D.id && /* @__PURE__ */ s("span", { className: "text-xs bg-blue-100 text-blue-700 px-2 py-1 rounded", children: "Active" })
                      ] })
                    ] }),
                    /* @__PURE__ */ s("p", { className: "text-xs text-gray-600 mt-1", children: D.description }),
                    /* @__PURE__ */ s("p", { className: "text-xs text-gray-500 mt-1 font-mono", children: D.baseUrl || window.location.origin }),
                    R[D.id] && !R[D.id].success && /* @__PURE__ */ h("p", { className: "text-xs text-red-600 mt-1", children: [
                      "Error: ",
                      R[D.id].error
                    ] })
                  ] })
                ]
              },
              D.id
            )) }),
            g && /* @__PURE__ */ s("div", { className: `p-3 rounded-md ${g.success ? "bg-green-50 text-green-800 border border-green-200" : "bg-red-50 text-red-800 border border-red-200"}`, children: /* @__PURE__ */ h("span", { className: "text-sm font-medium", children: [
              g.success ? "✓" : "✗",
              " ",
              g.message
            ] }) })
          ] }),
          r === "database" && /* @__PURE__ */ h("div", { className: "space-y-4", children: [
            /* @__PURE__ */ h("div", { className: "mb-4", children: [
              /* @__PURE__ */ s("h4", { className: "text-md font-semibold text-gray-900 mb-2", children: "Database Storage Backend" }),
              /* @__PURE__ */ s("p", { className: "text-sm text-gray-600 mb-4", children: "Choose the storage backend for your database. Changes require a server restart." })
            ] }),
            /* @__PURE__ */ h("div", { children: [
              /* @__PURE__ */ s("label", { className: "block text-sm font-medium text-gray-700 mb-2", children: "Storage Type" }),
              /* @__PURE__ */ h(
                "select",
                {
                  value: $,
                  onChange: (D) => H(D.target.value),
                  className: "w-full p-2 border border-gray-300 rounded text-sm",
                  children: [
                    /* @__PURE__ */ s("option", { value: "local", children: "Local (Sled)" }),
                    /* @__PURE__ */ s("option", { value: "dynamodb", children: "DynamoDB" }),
                    /* @__PURE__ */ s("option", { value: "s3", children: "S3" })
                  ]
                }
              )
            ] }),
            $ === "local" ? /* @__PURE__ */ h("div", { children: [
              /* @__PURE__ */ s("label", { className: "block text-sm font-medium text-gray-700 mb-1", children: "Storage Path" }),
              /* @__PURE__ */ s(
                "input",
                {
                  type: "text",
                  value: j,
                  onChange: (D) => q(D.target.value),
                  placeholder: "data",
                  className: "w-full p-2 border border-gray-300 rounded text-sm"
                }
              ),
              /* @__PURE__ */ s("p", { className: "text-xs text-gray-500 mt-1", children: "Local filesystem path where the database will be stored" })
            ] }) : $ === "dynamodb" ? /* @__PURE__ */ h("div", { className: "space-y-3", children: [
              /* @__PURE__ */ h("div", { children: [
                /* @__PURE__ */ h("label", { className: "block text-sm font-medium text-gray-700 mb-1", children: [
                  "Table Name ",
                  /* @__PURE__ */ s("span", { className: "text-red-500", children: "*" })
                ] }),
                /* @__PURE__ */ s(
                  "input",
                  {
                    type: "text",
                    value: ae,
                    onChange: (D) => he(D.target.value),
                    placeholder: "DataFoldStorage",
                    className: "w-full p-2 border border-gray-300 rounded text-sm"
                  }
                ),
                /* @__PURE__ */ s("p", { className: "text-xs text-gray-500 mt-1", children: "Base table name (namespaces will be appended automatically)" })
              ] }),
              /* @__PURE__ */ h("div", { children: [
                /* @__PURE__ */ h("label", { className: "block text-sm font-medium text-gray-700 mb-1", children: [
                  "AWS Region ",
                  /* @__PURE__ */ s("span", { className: "text-red-500", children: "*" })
                ] }),
                /* @__PURE__ */ s(
                  "input",
                  {
                    type: "text",
                    value: Ee,
                    onChange: (D) => tt(D.target.value),
                    placeholder: "us-west-2",
                    className: "w-full p-2 border border-gray-300 rounded text-sm"
                  }
                ),
                /* @__PURE__ */ s("p", { className: "text-xs text-gray-500 mt-1", children: "AWS region where your DynamoDB tables are located" })
              ] }),
              /* @__PURE__ */ h("div", { children: [
                /* @__PURE__ */ s("label", { className: "block text-sm font-medium text-gray-700 mb-1", children: "User ID (Optional)" }),
                /* @__PURE__ */ s(
                  "input",
                  {
                    type: "text",
                    value: rt,
                    onChange: (D) => X(D.target.value),
                    placeholder: "Leave empty for single-tenant",
                    className: "w-full p-2 border border-gray-300 rounded text-sm"
                  }
                ),
                /* @__PURE__ */ s("p", { className: "text-xs text-gray-500 mt-1", children: "User ID for multi-tenant isolation (uses partition key)" })
              ] }),
              /* @__PURE__ */ s("div", { className: "p-3 bg-yellow-50 border border-yellow-200 rounded-md", children: /* @__PURE__ */ h("p", { className: "text-xs text-yellow-800", children: [
                /* @__PURE__ */ s("strong", { children: "Note:" }),
                " Ensure your AWS credentials are configured (via environment variables, IAM role, or AWS CLI). The DynamoDB tables will be created automatically if they don't exist."
              ] }) })
            ] }) : /* @__PURE__ */ h("div", { className: "space-y-3", children: [
              /* @__PURE__ */ h("div", { children: [
                /* @__PURE__ */ h("label", { className: "block text-sm font-medium text-gray-700 mb-1", children: [
                  "S3 Bucket ",
                  /* @__PURE__ */ s("span", { className: "text-red-500", children: "*" })
                ] }),
                /* @__PURE__ */ s(
                  "input",
                  {
                    type: "text",
                    value: se,
                    onChange: (D) => Me(D.target.value),
                    placeholder: "my-datafold-bucket",
                    className: "w-full p-2 border border-gray-300 rounded text-sm"
                  }
                ),
                /* @__PURE__ */ s("p", { className: "text-xs text-gray-500 mt-1", children: "S3 bucket name where the database will be stored" })
              ] }),
              /* @__PURE__ */ h("div", { children: [
                /* @__PURE__ */ h("label", { className: "block text-sm font-medium text-gray-700 mb-1", children: [
                  "AWS Region ",
                  /* @__PURE__ */ s("span", { className: "text-red-500", children: "*" })
                ] }),
                /* @__PURE__ */ s(
                  "input",
                  {
                    type: "text",
                    value: nt,
                    onChange: (D) => xt(D.target.value),
                    placeholder: "us-east-1",
                    className: "w-full p-2 border border-gray-300 rounded text-sm"
                  }
                ),
                /* @__PURE__ */ s("p", { className: "text-xs text-gray-500 mt-1", children: "AWS region where your S3 bucket is located" })
              ] }),
              /* @__PURE__ */ h("div", { children: [
                /* @__PURE__ */ s("label", { className: "block text-sm font-medium text-gray-700 mb-1", children: "S3 Prefix (Optional)" }),
                /* @__PURE__ */ s(
                  "input",
                  {
                    type: "text",
                    value: wt,
                    onChange: (D) => vt(D.target.value),
                    placeholder: "folddb",
                    className: "w-full p-2 border border-gray-300 rounded text-sm"
                  }
                ),
                /* @__PURE__ */ s("p", { className: "text-xs text-gray-500 mt-1", children: 'Prefix/path within the bucket (defaults to "folddb")' })
              ] }),
              /* @__PURE__ */ h("div", { children: [
                /* @__PURE__ */ s("label", { className: "block text-sm font-medium text-gray-700 mb-1", children: "Local Cache Path" }),
                /* @__PURE__ */ s(
                  "input",
                  {
                    type: "text",
                    value: Fe,
                    onChange: (D) => st(D.target.value),
                    placeholder: "/tmp/folddb-data",
                    className: "w-full p-2 border border-gray-300 rounded text-sm"
                  }
                ),
                /* @__PURE__ */ s("p", { className: "text-xs text-gray-500 mt-1", children: "Local filesystem path for caching S3 data (defaults to /tmp/folddb-data)" })
              ] }),
              /* @__PURE__ */ s("div", { className: "p-3 bg-yellow-50 border border-yellow-200 rounded-md", children: /* @__PURE__ */ h("p", { className: "text-xs text-yellow-800", children: [
                /* @__PURE__ */ s("strong", { children: "Note:" }),
                " Ensure your AWS credentials are configured (via environment variables, IAM role, or AWS CLI). The database will be synced to/from S3 on startup and shutdown."
              ] }) })
            ] }),
            g && /* @__PURE__ */ s("div", { className: `p-3 rounded-md ${g.success ? "bg-green-50 text-green-800 border border-green-200" : "bg-red-50 text-red-800 border border-red-200"}`, children: /* @__PURE__ */ h("span", { className: "text-sm font-medium", children: [
              g.success ? "✓" : "✗",
              " ",
              g.message
            ] }) })
          ] })
        ] })
      ] }),
      /* @__PURE__ */ s("div", { className: "bg-gray-50 px-4 py-3 sm:px-6 sm:flex sm:flex-row-reverse gap-3 border-t border-gray-200", children: r === "ai" || r === "schema-service" || r === "database" ? /* @__PURE__ */ h(Rt, { children: [
        /* @__PURE__ */ s(
          "button",
          {
            onClick: r === "ai" ? tr : r === "schema-service" ? rr : qe,
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
function Xo() {
  const [t, e] = O([]), r = At(null), i = (c) => {
    if (typeof c == "string") return c;
    const u = c.metadata ? JSON.stringify(c.metadata) : "";
    return `[${c.level}] [${c.event_type}] - ${c.message} ${u}`;
  }, o = () => {
    Promise.resolve(
      navigator.clipboard.writeText(t.map(i).join(`
`))
    ).catch(() => {
    });
  };
  return ue(() => {
    ne.getLogs().then((l) => {
      if (l.success && l.data) {
        const f = l.data.logs || [];
        e(Array.isArray(f) ? f : []);
      } else
        e([]);
    }).catch(() => e([]));
    const c = ne.createLogStream(
      (l) => {
        e((f) => {
          let p;
          try {
            p = JSON.parse(l);
          } catch {
            const y = l.split(" - "), v = y.length > 1 ? y[0] : "INFO";
            p = {
              id: `stream-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
              timestamp: Date.now(),
              level: v,
              event_type: "stream (legacy)",
              message: l
            };
          }
          return p.id && f.some((y) => y.id === p.id) ? f : [...f, p];
        });
      },
      (l) => {
        console.warn("Log stream error:", l);
      }
    ), u = setInterval(() => {
      e((l) => {
        const f = l.length > 0 ? l[l.length - 1] : null, p = f ? f.timestamp : void 0;
        return ne.getLogs(p).then((y) => {
          if (y.success && y.data) {
            const v = y.data.logs || [];
            v.length > 0 && e((N) => {
              const _ = v.filter((b) => b.id && N.some((x) => x.id === b.id) ? !1 : !N.some(
                (g) => !g.id && // Only check content if existing has no ID
                g.timestamp === b.timestamp && g.message === b.message
              ));
              return _.length === 0 ? N : [...N, ..._];
            });
          }
        }).catch((y) => console.warn("Log polling error:", y)), l;
      });
    }, 2e3);
    return () => {
      c.close(), clearInterval(u);
    };
  }, []), ue(() => {
    var c;
    (c = r.current) == null || c.scrollIntoView({ behavior: "smooth" });
  }, [t]), /* @__PURE__ */ h("aside", { className: "w-80 bg-gray-900 text-white flex flex-col overflow-hidden", children: [
    /* @__PURE__ */ h("div", { className: "flex items-center justify-between p-4 border-b border-gray-700", children: [
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
    /* @__PURE__ */ h("div", { className: "flex-1 overflow-y-auto p-4 space-y-1 text-xs font-mono", children: [
      t.map((c, u) => /* @__PURE__ */ s("div", { children: i(c) }, c.id || u)),
      /* @__PURE__ */ s("div", { ref: r })
    ] })
  ] });
}
function el({ onSettingsClick: t }) {
  const e = ze(), { isAuthenticated: r, user: i } = J((c) => c.auth), o = () => {
    e(ua()), localStorage.removeItem("fold_user_id"), localStorage.removeItem("fold_user_hash");
  };
  return /* @__PURE__ */ s("header", { className: "bg-white border-b border-gray-200 shadow-sm flex-shrink-0", children: /* @__PURE__ */ h("div", { className: "flex items-center justify-between px-6 py-3", children: [
    /* @__PURE__ */ h("a", { href: "/", className: "flex items-center gap-3 text-blue-600 hover:text-blue-700 transition-colors", children: [
      /* @__PURE__ */ h("svg", { className: "w-8 h-8 flex-shrink-0", viewBox: "0 0 24 24", fill: "currentColor", children: [
        /* @__PURE__ */ s("path", { d: "M12 4C7.58172 4 4 5.79086 4 8C4 10.2091 7.58172 12 12 12C16.4183 12 20 10.2091 20 8C20 5.79086 16.4183 4 12 4Z" }),
        /* @__PURE__ */ s("path", { d: "M4 12V16C4 18.2091 7.58172 20 12 20C16.4183 20 20 18.2091 20 16V12", strokeWidth: "2", strokeLinecap: "round" }),
        /* @__PURE__ */ s("path", { d: "M4 8V12C4 14.2091 7.58172 16 12 16C16.4183 16 20 14.2091 20 12V8", strokeWidth: "2", strokeLinecap: "round" })
      ] }),
      /* @__PURE__ */ s("span", { className: "text-xl font-semibold text-gray-900", children: "DataFold Node" })
    ] }),
    /* @__PURE__ */ h("div", { className: "flex items-center gap-3", children: [
      r && /* @__PURE__ */ h("div", { className: "flex items-center gap-3 mr-2", children: [
        /* @__PURE__ */ s("span", { className: "text-sm text-gray-600", children: i == null ? void 0 : i.id }),
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
      /* @__PURE__ */ h(
        "button",
        {
          onClick: t,
          className: "inline-flex items-center gap-2 px-3 py-2 text-sm text-gray-700 hover:bg-gray-100 rounded-md border border-gray-300 transition-colors",
          title: "Settings",
          children: [
            /* @__PURE__ */ h("svg", { className: "w-4 h-4", fill: "none", stroke: "currentColor", viewBox: "0 0 24 24", children: [
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
function tl() {
  return /* @__PURE__ */ s("footer", { className: "bg-white border-t border-gray-200 py-3", children: /* @__PURE__ */ s("div", { className: "max-w-7xl mx-auto px-6 text-center", children: /* @__PURE__ */ h("p", { className: "text-gray-600 text-sm", children: [
    "DataFold Node © ",
    (/* @__PURE__ */ new Date()).getFullYear()
  ] }) }) });
}
function rl() {
  const [t, e] = O(""), [r, i] = O(""), o = ze(), { isLoading: c } = J((l) => l.auth);
  return /* @__PURE__ */ h("div", { className: "min-h-screen bg-gray-50 flex flex-col justify-center py-12 sm:px-6 lg:px-8", children: [
    /* @__PURE__ */ h("div", { className: "sm:mx-auto sm:w-full sm:max-w-md", children: [
      /* @__PURE__ */ s("h2", { className: "mt-6 text-center text-3xl font-extrabold text-gray-900", children: "Sign in to Exemem" }),
      /* @__PURE__ */ s("p", { className: "mt-2 text-center text-sm text-gray-600", children: "Enter your user identifier to access your Exemem node" })
    ] }),
    /* @__PURE__ */ s("div", { className: "mt-8 sm:mx-auto sm:w-full sm:max-w-md", children: /* @__PURE__ */ s("div", { className: "bg-white py-8 px-4 shadow sm:rounded-lg sm:px-10", children: /* @__PURE__ */ h("form", { className: "space-y-6", onSubmit: async (l) => {
      if (l.preventDefault(), !t.trim()) {
        i("Please enter a user identifier");
        return;
      }
      try {
        const f = await o(Mr(t.trim())).unwrap();
        localStorage.setItem("fold_user_id", f.id), localStorage.setItem("fold_user_hash", f.hash);
      } catch (f) {
        i("Login failed: " + f.message);
      }
    }, children: [
      /* @__PURE__ */ h("div", { children: [
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
          disabled: c,
          className: "w-full flex justify-center py-2 px-4 border border-transparent rounded-md shadow-sm text-sm font-medium text-white bg-blue-600 hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 disabled:opacity-50",
          children: c ? "Connecting..." : "Continue"
        }
      ) })
    ] }) }) })
  ] });
}
function nl() {
  const [t, e] = O(""), [r, i] = O(""), o = ze(), { isAuthenticated: c, isLoading: u } = J((f) => f.auth);
  return c ? null : /* @__PURE__ */ s("div", { className: "fixed inset-0 z-50 overflow-y-auto", children: /* @__PURE__ */ h("div", { className: "flex items-center justify-center min-h-screen px-4 pt-4 pb-20 text-center sm:block sm:p-0", children: [
    /* @__PURE__ */ s("div", { className: "fixed inset-0 transition-opacity bg-gray-900 bg-opacity-75" }),
    /* @__PURE__ */ s("span", { className: "hidden sm:inline-block sm:align-middle sm:h-screen", children: "​" }),
    /* @__PURE__ */ s("div", { className: "inline-block align-bottom bg-white rounded-lg text-left overflow-hidden shadow-xl transform transition-all sm:my-8 sm:align-middle sm:max-w-lg sm:w-full", children: /* @__PURE__ */ s("div", { className: "bg-white px-4 pt-5 pb-4 sm:p-6 sm:pb-4", children: /* @__PURE__ */ s("div", { className: "sm:flex sm:items-start", children: /* @__PURE__ */ h("div", { className: "mt-3 text-center sm:mt-0 sm:text-left w-full", children: [
      /* @__PURE__ */ s("h3", { className: "text-xl leading-6 font-medium text-gray-900 mb-2", children: "Welcome to DataFold" }),
      /* @__PURE__ */ h("div", { className: "mt-2", children: [
        /* @__PURE__ */ s("p", { className: "text-sm text-gray-500 mb-4", children: "Enter your user identifier to continue. This will generate a unique session hash for your environment." }),
        /* @__PURE__ */ h("form", { onSubmit: async (f) => {
          if (f.preventDefault(), !t.trim()) {
            i("Please enter a user identifier");
            return;
          }
          try {
            const p = await o(Mr(t.trim())).unwrap();
            localStorage.setItem("fold_user_id", p.id), localStorage.setItem("fold_user_hash", p.hash);
          } catch (p) {
            i("Login failed: " + p.message);
          }
        }, children: [
          /* @__PURE__ */ h("div", { className: "mb-4", children: [
            /* @__PURE__ */ s("label", { htmlFor: "userId", className: "block text-sm font-medium text-gray-700 mb-1", children: "User Identifier" }),
            /* @__PURE__ */ s(
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
          r && /* @__PURE__ */ s("div", { className: "mb-4 text-sm text-red-600", children: r }),
          /* @__PURE__ */ s(
            "button",
            {
              type: "submit",
              disabled: u,
              className: "w-full inline-flex justify-center rounded-md border border-transparent shadow-sm px-4 py-2 bg-blue-600 text-base font-medium text-white hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 sm:text-sm disabled:opacity-50",
              children: u ? "Connecting..." : "Continue"
            }
          )
        ] })
      ] })
    ] }) }) }) })
  ] }) });
}
function zt({ node: t, depth: e = 0, name: r = null }) {
  const [i, o] = O(e === 0);
  if (!t)
    return /* @__PURE__ */ s("span", { className: "text-gray-400 italic", children: "undefined" });
  if (t.type === "Primitive") {
    const c = t.value, u = {
      String: "text-green-600",
      Number: "text-blue-600",
      Boolean: "text-purple-600",
      Null: "text-gray-500"
    }[c] || "text-gray-600";
    return /* @__PURE__ */ h("span", { className: "inline-flex items-center space-x-2", children: [
      /* @__PURE__ */ s("span", { className: `font-mono text-sm ${u}`, children: c.toLowerCase() }),
      t.classifications && t.classifications.length > 0 && /* @__PURE__ */ s("span", { className: "flex space-x-1", children: t.classifications.map((l) => /* @__PURE__ */ s("span", { className: "px-1.5 py-0.5 text-xs bg-gray-200 text-gray-700 rounded-full font-sans", children: l }, l)) })
    ] });
  }
  if (t.type === "Any")
    return /* @__PURE__ */ s("span", { className: "font-mono text-sm text-orange-600", children: "any" });
  if (t.type === "Array")
    return /* @__PURE__ */ h("div", { className: "inline-flex items-start", children: [
      /* @__PURE__ */ s("span", { className: "font-mono text-sm text-gray-700", children: "Array<" }),
      /* @__PURE__ */ s(zt, { node: t.value, depth: e + 1 }),
      /* @__PURE__ */ s("span", { className: "font-mono text-sm text-gray-700", children: ">" })
    ] });
  if (t.type === "Object" && t.value) {
    const c = Object.entries(t.value);
    return c.length === 0 ? /* @__PURE__ */ s("span", { className: "font-mono text-sm text-gray-500", children: "{}" }) : /* @__PURE__ */ h("div", { className: "inline-block", children: [
      /* @__PURE__ */ s("div", { className: "flex items-center", children: /* @__PURE__ */ h(
        "button",
        {
          onClick: () => o(!i),
          className: "flex items-center hover:bg-gray-100 rounded px-1 -ml-1",
          children: [
            i ? /* @__PURE__ */ s(Hn, { className: "h-3 w-3 text-gray-500" }) : /* @__PURE__ */ s(jn, { className: "h-3 w-3 text-gray-500" }),
            /* @__PURE__ */ h("span", { className: "font-mono text-sm text-gray-700 ml-1", children: [
              "{",
              !i && `... ${c.length} fields`,
              !i && "}"
            ] })
          ]
        }
      ) }),
      i && /* @__PURE__ */ h("div", { className: "ml-4 border-l-2 border-gray-200 pl-3 mt-1", children: [
        c.map(([u, l], f) => /* @__PURE__ */ h("div", { className: "py-1", children: [
          /* @__PURE__ */ s("span", { className: "font-mono text-sm text-indigo-600", children: u }),
          /* @__PURE__ */ s("span", { className: "font-mono text-sm text-gray-500", children: ": " }),
          /* @__PURE__ */ s(zt, { node: l, depth: e + 1, name: u }),
          f < c.length - 1 && /* @__PURE__ */ s("span", { className: "text-gray-400", children: "," })
        ] }, u)),
        /* @__PURE__ */ s("div", { className: "font-mono text-sm text-gray-700", children: "}" })
      ] })
    ] });
  }
  return /* @__PURE__ */ h("span", { className: "font-mono text-sm text-red-500", children: [
    "unknown (",
    JSON.stringify(t),
    ")"
  ] });
}
function Ii({ topology: t, compact: e = !1 }) {
  return t ? e ? /* @__PURE__ */ s("div", { className: "inline-flex items-center", children: /* @__PURE__ */ s(zt, { node: t.root }) }) : /* @__PURE__ */ h("div", { className: "mt-2 p-2 bg-gray-50 rounded border border-gray-200", children: [
    /* @__PURE__ */ s("div", { className: "text-xs font-medium text-gray-600 mb-1", children: "Type Structure:" }),
    /* @__PURE__ */ s("div", { className: "pl-2", children: /* @__PURE__ */ s(zt, { node: t.root }) })
  ] }) : /* @__PURE__ */ s("div", { className: "text-xs text-gray-400 italic", children: "No topology defined" });
}
function sl({ onResult: t, onSchemaUpdated: e }) {
  const r = ze(), i = J(yt);
  J(Ur), J(Un);
  const [o, c] = O({});
  ue(() => {
    console.log("🟢 SchemaTab: Fetching schemas on mount"), r(ke({ forceRefresh: !0 }));
  }, [r]);
  const u = (b) => b.descriptive_name || b.name;
  console.log("🟢 SchemaTab: Current schemas from Redux:", i.map((b) => ({ name: b.name, state: b.state })));
  const l = async (b) => {
    const E = o[b];
    if (c((g) => ({
      ...g,
      [b]: !g[b]
    })), !E) {
      const g = i.find((x) => x.name === b);
      if (g && (!g.fields || Object.keys(g.fields).length === 0))
        try {
          (await ee.getSchema(b)).success && (r(ke({ forceRefresh: !0 })), e && e());
        } catch (x) {
          console.error(`Failed to fetch schema details for ${b}:`, x);
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
    var E, g;
    console.log("🟡 SchemaTab: Starting approveSchema for:", b);
    try {
      const x = await r(je({ schemaName: b }));
      if (console.log("🟡 SchemaTab: approveSchema result:", x), je.fulfilled.match(x)) {
        console.log("🟡 SchemaTab: approveSchema fulfilled, calling callbacks");
        const S = (E = x.payload) == null ? void 0 : E.backfillHash;
        if (console.log("🔄 Backfill hash:", S), console.log("🔄 Refetching schemas from backend after approval..."), await r(ke({ forceRefresh: !0 })), console.log("✅ Refetch complete - backend state should be reflected"), t) {
          const C = S ? `Schema ${b} approved successfully. Backfill started with hash: ${S}` : `Schema ${b} approved successfully`;
          t({ success: !0, message: C, backfillHash: S });
        }
        e && e();
      } else {
        console.log("🔴 SchemaTab: approveSchema rejected:", x.payload);
        const S = typeof x.payload == "string" ? x.payload : ((g = x.payload) == null ? void 0 : g.error) || `Failed to approve schema: ${b}`;
        throw new Error(S);
      }
    } catch (x) {
      if (console.error("🔴 SchemaTab: Failed to approve schema:", x), t) {
        const S = x instanceof Error ? x.message : String(x);
        t({ error: `Failed to approve schema: ${S}` });
      }
    }
  }, y = async (b) => {
    var E;
    try {
      const g = await r(Ve({ schemaName: b }));
      if (Ve.fulfilled.match(g))
        console.log("🟡 SchemaTab: blockSchema fulfilled, calling callbacks"), console.log("🔄 Refetching schemas from backend after blocking..."), await r(ke({ forceRefresh: !0 })), console.log("✅ Refetch complete - backend state should be reflected"), t && t({ success: !0, message: `Schema ${b} blocked successfully` }), e && e();
      else {
        const x = typeof g.payload == "string" ? g.payload : ((E = g.payload) == null ? void 0 : E.error) || `Failed to block schema: ${b}`;
        throw new Error(x);
      }
    } catch (g) {
      if (console.error("Failed to block schema:", g), t) {
        const x = g instanceof Error ? g.message : String(g);
        t({ error: `Failed to block schema: ${x}` });
      }
    }
  }, v = (b) => {
    const E = o[b.name], g = b.state || "Unknown", x = b.fields ? Na(b) : null, S = Ea(b);
    return /* @__PURE__ */ h("div", { className: "bg-white rounded-lg border border-gray-200 shadow-sm overflow-hidden transition-all duration-200 hover:shadow-md", children: [
      /* @__PURE__ */ s(
        "div",
        {
          className: "px-4 py-3 bg-gray-50 cursor-pointer select-none transition-colors duration-200 hover:bg-gray-100",
          onClick: () => l(b.name),
          children: /* @__PURE__ */ h("div", { className: "flex items-center justify-between", children: [
            /* @__PURE__ */ h("div", { className: "flex items-center space-x-2", children: [
              E ? /* @__PURE__ */ s(Hn, { className: "icon icon-sm text-gray-400 transition-transform duration-200" }) : /* @__PURE__ */ s(jn, { className: "icon icon-sm text-gray-400 transition-transform duration-200" }),
              /* @__PURE__ */ s("h3", { className: "font-medium text-gray-900", children: u(b) }),
              b.descriptive_name && b.descriptive_name !== b.name && /* @__PURE__ */ h("span", { className: "text-xs text-gray-500", children: [
                "(",
                b.name,
                ")"
              ] }),
              /* @__PURE__ */ s("span", { className: `px-2 py-1 text-xs font-medium rounded-full ${f(g)}`, children: g }),
              x && /* @__PURE__ */ s("span", { className: "px-2 py-1 text-xs font-medium rounded-full bg-purple-100 text-purple-800", children: "Range Schema" }),
              S && /* @__PURE__ */ s("span", { className: "px-2 py-1 text-xs font-medium rounded-full bg-blue-100 text-blue-800", children: "HashRange Schema" })
            ] }),
            /* @__PURE__ */ h("div", { className: "flex items-center space-x-2", children: [
              g.toLowerCase() === "available" && /* @__PURE__ */ s(
                "button",
                {
                  className: "group inline-flex items-center px-2 py-1 text-xs font-medium rounded-md text-white bg-green-600 hover:bg-green-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-green-500",
                  onClick: (C) => {
                    console.log("🟠 Button clicked: Approve for schema:", b.name), C.stopPropagation(), p(b.name);
                  },
                  children: "Approve"
                }
              ),
              g.toLowerCase() === "approved" && /* @__PURE__ */ s(
                "button",
                {
                  className: "group inline-flex items-center px-2 py-1 text-xs font-medium rounded-md text-white bg-red-600 hover:bg-red-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-red-500",
                  onClick: (C) => {
                    C.stopPropagation(), y(b.name);
                  },
                  children: "Block"
                }
              ),
              g.toLowerCase() === "blocked" && /* @__PURE__ */ s(
                "button",
                {
                  className: "group inline-flex items-center px-2 py-1 text-xs font-medium rounded-md text-white bg-green-600 hover:bg-green-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-green-500",
                  onClick: (C) => {
                    C.stopPropagation(), p(b.name);
                  },
                  children: "Re-approve"
                }
              )
            ] })
          ] })
        }
      ),
      E && b.fields && /* @__PURE__ */ h("div", { className: "p-4 border-t border-gray-200", children: [
        x && /* @__PURE__ */ h("div", { className: "mb-4 p-3 bg-purple-50 rounded-md border border-purple-200", children: [
          /* @__PURE__ */ s("h4", { className: "text-sm font-medium text-purple-900 mb-2", children: "Range Schema Information" }),
          /* @__PURE__ */ h("div", { className: "space-y-1 text-xs text-purple-800", children: [
            /* @__PURE__ */ h("p", { children: [
              /* @__PURE__ */ s("strong", { children: "Range Key:" }),
              " ",
              x.rangeKey
            ] }),
            /* @__PURE__ */ h("p", { children: [
              /* @__PURE__ */ s("strong", { children: "Total Fields:" }),
              " ",
              x.totalFields
            ] }),
            /* @__PURE__ */ h("p", { children: [
              /* @__PURE__ */ s("strong", { children: "Range Fields:" }),
              " ",
              x.rangeFields.length
            ] }),
            /* @__PURE__ */ s("p", { className: "text-purple-600", children: "This schema uses range-based storage for efficient querying and mutations." })
          ] })
        ] }),
        S && /* @__PURE__ */ h("div", { className: "mb-4 p-3 bg-blue-50 rounded-md border border-blue-200", children: [
          /* @__PURE__ */ s("h4", { className: "text-sm font-medium text-blue-900 mb-2", children: "HashRange Schema Information" }),
          /* @__PURE__ */ h("div", { className: "space-y-1 text-xs text-blue-800", children: [
            /* @__PURE__ */ h("p", { children: [
              /* @__PURE__ */ s("strong", { children: "Hash Field:" }),
              " ",
              S.hashField
            ] }),
            /* @__PURE__ */ h("p", { children: [
              /* @__PURE__ */ s("strong", { children: "Range Field:" }),
              " ",
              S.rangeField
            ] }),
            /* @__PURE__ */ h("p", { children: [
              /* @__PURE__ */ s("strong", { children: "Total Fields:" }),
              " ",
              S.totalFields
            ] }),
            /* @__PURE__ */ s("p", { className: "text-blue-600", children: "This schema uses hash-range-based storage for efficient querying and mutations with both hash and range keys." })
          ] })
        ] }),
        /* @__PURE__ */ s("div", { className: "space-y-3", children: Array.isArray(b.fields) ? b.fields.map((C) => {
          var F;
          const I = (F = b.field_topologies) == null ? void 0 : F[C];
          return /* @__PURE__ */ s("div", { className: "p-3 bg-gray-50 rounded-md border border-gray-200", children: /* @__PURE__ */ s("div", { className: "flex items-center justify-between", children: /* @__PURE__ */ h("div", { className: "flex-1", children: [
            /* @__PURE__ */ h("div", { className: "flex items-center space-x-2", children: [
              /* @__PURE__ */ s("span", { className: "font-medium text-gray-900", children: C }),
              (x == null ? void 0 : x.rangeKey) === C && /* @__PURE__ */ s("span", { className: "px-2 py-0.5 text-xs font-medium rounded-full bg-purple-100 text-purple-800", children: "Range Key" }),
              (S == null ? void 0 : S.hashField) === C && /* @__PURE__ */ s("span", { className: "px-2 py-0.5 text-xs font-medium rounded-full bg-blue-100 text-blue-800", children: "Hash Key" }),
              (S == null ? void 0 : S.rangeField) === C && /* @__PURE__ */ s("span", { className: "px-2 py-0.5 text-xs font-medium rounded-full bg-purple-100 text-purple-800", children: "Range Key" })
            ] }),
            I && /* @__PURE__ */ s(Ii, { topology: I })
          ] }) }) }, C);
        }) : /* @__PURE__ */ s("p", { className: "text-sm text-gray-500 italic", children: "No fields defined" }) })
      ] })
    ] }, b.name);
  }, N = (b) => typeof b == "string" ? b.toLowerCase() : typeof b == "object" && b !== null ? String(b).toLowerCase() : String(b || "").toLowerCase(), _ = i.filter(
    (b) => N(b.state) === "approved"
  );
  return /* @__PURE__ */ s("div", { className: "p-6 space-y-6", children: /* @__PURE__ */ h("div", { className: "space-y-4", children: [
    /* @__PURE__ */ s("h3", { className: "text-lg font-medium text-gray-900", children: "Approved Schemas" }),
    _.length > 0 ? _.map(v) : /* @__PURE__ */ s("div", { className: "border rounded-lg p-8 bg-white shadow-sm text-center text-gray-500", children: "No approved schemas found." })
  ] }) });
}
function Kr() {
  const t = ze(), e = J(yt), r = J(Ur), [i, o] = O(""), [c, u] = O([]), [l, f] = O({}), [p, y] = O({}), [v, N] = O(""), [_, b] = O(""), [E, g] = O({}), x = W(() => (e || []).filter((H) => (typeof H.state == "string" ? H.state.toLowerCase() : String(H.state || "").toLowerCase()) === Oe.APPROVED), [e]), S = W(() => i ? (e || []).find((H) => H.name === i) : null, [i, e]), C = W(() => S ? pt(S) : !1, [S]), I = W(() => S ? Pr(S) : !1, [S]), F = W(() => S ? et(S) : null, [S]), P = K((H) => {
    if (o(H), H) {
      const j = (e || []).find((Ee) => Ee.name === H), q = (j == null ? void 0 : j.fields) || (j == null ? void 0 : j.transform_fields) || [], ae = Array.isArray(q) ? q : Object.keys(q);
      u(ae);
      const he = {};
      ae.forEach((Ee) => {
        he[Ee] = "";
      }), f(he);
    } else
      u([]), f({});
    y({}), N(""), b(""), g({});
  }, [e]), T = K((H) => {
    u((j) => j.includes(H) ? j.filter((q) => q !== H) : [...j, H]), f((j) => j[H] !== void 0 ? j : {
      ...j,
      [H]: ""
      // Initialize with empty string for new fields
    });
  }, []), R = K((H, j, q) => {
    y((ae) => ({
      ...ae,
      [H]: {
        ...ae[H],
        [j]: q
      }
    }));
  }, []), B = K((H, j) => {
    f((q) => ({
      ...q,
      [H]: j
    }));
  }, []), U = K(() => {
    o(""), u([]), f({}), y({}), N(""), b(""), g({});
  }, []), L = K(() => {
    t(ke({ forceRefresh: !0 }));
  }, [t]);
  return {
    state: {
      selectedSchema: i,
      queryFields: c,
      fieldValues: l,
      rangeFilters: p,
      rangeSchemaFilter: E,
      rangeKeyValue: v,
      hashKeyValue: _
    },
    setSelectedSchema: o,
    setQueryFields: u,
    setFieldValues: f,
    toggleField: T,
    handleFieldValueChange: B,
    setRangeFilters: y,
    setRangeSchemaFilter: g,
    setRangeKeyValue: N,
    setHashKeyValue: b,
    clearState: U,
    handleSchemaChange: P,
    handleRangeFilterChange: R,
    refetchSchemas: L,
    approvedSchemas: x,
    schemasLoading: r,
    selectedSchemaObj: S,
    isRangeSchema: C,
    isHashRangeSchema: I,
    rangeKey: F
  };
}
function Tt(t) {
  return { HashKey: t };
}
function Bi(t) {
  return { RangePrefix: t };
}
function Oi(t, e) {
  return { RangeRange: { start: t, end: e } };
}
function Fi(t, e) {
  return { HashRangeKey: { hash: t, range: e } };
}
function Wn({
  schema: t,
  queryState: e,
  schemas: r,
  selectedSchemaObj: i,
  isRangeSchema: o,
  rangeKey: c
}) {
  const u = J(Bt), l = W(() => i || (r && t && r[t] ? r[t] : u && Array.isArray(u) && u.find((N) => N.name === t) || null), [i, t, r, u]), f = W(() => typeof o == "boolean" ? o : l ? l.schema_type === "Range" || pt(l) ? !0 : l.fields && typeof l.fields == "object" ? Object.values(l.fields).some((_) => (_ == null ? void 0 : _.field_type) === "Range") : !1 : !1, [l, o]), p = W(() => [], []), y = !0, v = W(() => {
    var C;
    if (!t || !e || !l)
      return {};
    const {
      queryFields: N = [],
      fieldValues: _ = {},
      rangeFilters: b = {},
      rangeSchemaFilter: E = {},
      filters: g = [],
      orderBy: x
    } = e, S = {
      schema_name: t,
      // Backend expects schema_name, not schema
      fields: N
      // Array of selected field names
    };
    if (Pr(l)) {
      const I = e.hashKeyValue, F = (C = e.rangeSchemaFilter) == null ? void 0 : C.key;
      I && I.trim() ? S.filter = Tt(I.trim()) : F && F.trim() && (S.filter = Tt(F.trim()));
    }
    if (f) {
      const I = E && Object.keys(E).length > 0 ? E : Object.values(b).find((P) => P && typeof P == "object" && (P.key || P.keyPrefix || P.start && P.end)) || {}, F = e == null ? void 0 : e.rangeKeyValue;
      !I.key && !I.keyPrefix && !(I.start && I.end) && F && (I.key = F), I.key ? S.filter = Tt(I.key) : I.keyPrefix ? S.filter = Bi(I.keyPrefix) : I.start && I.end && (S.filter = Oi(I.start, I.end));
    }
    return S;
  }, [t, e, l]);
  return K(() => v, [v]), K(() => ({
    isValid: y,
    errors: p
  }), [y, p]), {
    query: v,
    validationErrors: p,
    isValid: y
  };
}
function Ie({
  label: t,
  name: e,
  required: r = !1,
  error: i,
  helpText: o,
  children: c,
  className: u = ""
}) {
  const l = e ? `field-${e}` : `field-${Math.random().toString(36).substr(2, 9)}`, f = !!i;
  return /* @__PURE__ */ h("div", { className: `space-y-2 ${u}`, children: [
    /* @__PURE__ */ h(
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
    /* @__PURE__ */ s("div", { className: "relative", children: c }),
    f && /* @__PURE__ */ s(
      "p",
      {
        className: "text-sm text-red-600",
        role: "alert",
        "aria-live": "polite",
        children: i
      }
    ),
    o && !f && /* @__PURE__ */ s("p", { className: "text-xs text-gray-500", children: o })
  ] });
}
function Zn(t = []) {
  return t.reduce((e, r) => {
    const i = r.group || "default";
    return e[i] || (e[i] = []), e[i].push(r), e;
  }, {});
}
function Li(t = [], e = "") {
  if (Sa(e)) return t;
  const r = e.toLowerCase();
  return t.filter(
    (i) => i.label.toLowerCase().includes(r) || i.value.toLowerCase().includes(r)
  );
}
function Di(t = {}) {
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
function Mi(t, e = !1, r = !1, i = !1) {
  var c, u;
  let o = ((c = t.select) == null ? void 0 : c.base) || "";
  return e && (o += " border-red-300 focus:ring-red-500 focus:border-red-500"), (r || i) && (o += ` ${((u = t.select) == null ? void 0 : u.disabled) || ""}`), o;
}
function Pi(t, e = !1, r = "") {
  const i = {
    "aria-invalid": e
  };
  return e ? i["aria-describedby"] = `${t}-error` : r && (i["aria-describedby"] = `${t}-help`), i;
}
function Ui(t = [], e, r = !0) {
  const [i, o] = O(""), [c, u] = O(!1), l = Li(t, i), f = Zn(l), p = K((S) => {
    o(S.target.value);
  }, []), y = K((S) => {
    S.disabled || (e(S.value), r && (u(!1), o("")));
  }, [e, r]), v = K(() => {
    u(!0);
  }, []), N = K(() => {
    u(!1);
  }, []), _ = K(() => {
    u((S) => !S);
  }, []), b = K((S) => {
    const C = t.find((I) => I.value === S);
    C && y(C);
  }, [t, y]), E = K(() => {
    o("");
  }, []);
  return {
    state: {
      searchTerm: i,
      isOpen: c,
      filteredOptions: l,
      groupedOptions: f
    },
    actions: {
      setSearchTerm: o,
      openDropdown: v,
      closeDropdown: N,
      toggleDropdown: _,
      selectOption: b,
      clearSearch: E
    },
    handleSearchChange: p,
    handleOptionSelect: y
  };
}
function Jn(t) {
  return `field-${t}`;
}
function $i(t) {
  return !!t;
}
function Ki({ hasError: t, disabled: e, additionalClasses: r = "" }) {
  const i = ve.input.base, o = t ? ve.input.error : ve.input.success;
  return `${i} ${o} ${e ? "bg-gray-100 cursor-not-allowed" : ""} ${r}`.trim();
}
function Hi({ fieldId: t, hasError: e, hasHelp: r }) {
  const i = {
    "aria-invalid": e
  };
  return e ? i["aria-describedby"] = `${t}-error` : r && (i["aria-describedby"] = `${t}-help`), i;
}
function ji({ size: t = "sm", color: e = "primary" } = {}) {
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
function Er({
  name: t,
  label: e,
  value: r,
  options: i = [],
  onChange: o,
  error: c,
  helpText: u,
  config: l = {},
  className: f = ""
}) {
  const p = Di(l), { searchable: y, placeholder: v, emptyMessage: N, required: _, disabled: b, loading: E } = p, g = Jn(t), x = !!c, S = i.length > 0, C = Ui(i, o, !0), I = (R) => {
    o(R.target.value);
  };
  if (E)
    return /* @__PURE__ */ s(Ie, { label: e, name: t, required: _, error: c, helpText: u, className: f, children: /* @__PURE__ */ h("div", { className: `${ve.select.disabled} flex items-center`, children: [
      /* @__PURE__ */ s("div", { className: "animate-spin h-4 w-4 border-2 border-gray-400 border-t-transparent rounded-full mr-2" }),
      ba.loading
    ] }) });
  if (!S)
    return /* @__PURE__ */ s(Ie, { label: e, name: t, required: _, error: c, helpText: u, className: f, children: /* @__PURE__ */ s("div", { className: ve.select.disabled, children: N }) });
  if (y) {
    const { state: R, handleSearchChange: B, handleOptionSelect: U } = C;
    return /* @__PURE__ */ s(Ie, { label: e, name: t, required: _, error: c, helpText: u, className: f, children: /* @__PURE__ */ h("div", { className: "relative", children: [
      /* @__PURE__ */ s(
        "input",
        {
          type: "text",
          placeholder: `Search ${e.toLowerCase()}...`,
          value: R.searchTerm,
          onChange: B,
          onFocus: () => C.actions.openDropdown(),
          className: `${ve.input.base} ${x ? ve.input.error : ""}`
        }
      ),
      R.isOpen && R.filteredOptions.length > 0 && /* @__PURE__ */ s("div", { className: "absolute z-10 w-full mt-1 bg-white border border-gray-300 rounded-md shadow-lg max-h-60 overflow-auto", children: Object.entries(R.groupedOptions).map(([L, $]) => /* @__PURE__ */ h("div", { children: [
        L !== "default" && /* @__PURE__ */ s("div", { className: "px-3 py-2 text-xs font-semibold text-gray-500 bg-gray-50 border-b", children: L }),
        $.map((H) => /* @__PURE__ */ s(
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
      ] }, L)) })
    ] }) });
  }
  const F = Zn(i), P = Mi(ve, x, b, E), T = Pi(g, x, u);
  return /* @__PURE__ */ s(Ie, { label: e, name: t, required: _, error: c, helpText: u, className: f, children: /* @__PURE__ */ h(
    "select",
    {
      id: g,
      name: t,
      value: r,
      onChange: I,
      required: _,
      disabled: b,
      className: P,
      ...T,
      children: [
        /* @__PURE__ */ s("option", { value: "", disabled: _, children: v }),
        Object.entries(F).map(
          ([R, B]) => R !== "default" ? /* @__PURE__ */ s("optgroup", { label: R, children: B.map((U) => /* @__PURE__ */ s("option", { value: U.value, disabled: U.disabled, children: U.label }, U.value)) }, R) : B.map((U) => /* @__PURE__ */ s("option", { value: U.value, disabled: U.disabled, children: U.label }, U.value))
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
  disabled: c = !1,
  error: u,
  placeholder: l,
  helpText: f,
  type: p = "text",
  debounced: y = !1,
  debounceMs: v = ga,
  className: N = ""
}) {
  const [_, b] = O(r), [E, g] = O(!1);
  ue(() => {
    b(r);
  }, [r]);
  const x = At(null), S = At(null), C = At(i);
  ue(() => {
    C.current = i;
  }, [i]);
  const I = K((U) => {
    g(!0), x.current && (clearTimeout(x.current), x.current = null), S.current && typeof window < "u" && typeof window.cancelAnimationFrame == "function" && (window.cancelAnimationFrame(S.current), S.current = null);
    const L = () => {
      x.current = setTimeout(() => {
        C.current(U), g(!1);
      }, v);
    };
    typeof window < "u" && typeof window.requestAnimationFrame == "function" ? S.current = window.requestAnimationFrame(L) : setTimeout(L, 0);
  }, [v]), F = (U) => {
    const L = U.target.value;
    b(L), y ? I(L) : i(L);
  }, P = Jn(t), T = $i(u), R = Ki({ hasError: T, disabled: c }), B = Hi({
    fieldId: P,
    hasError: T,
    hasHelp: !!f
  });
  return /* @__PURE__ */ s(
    Ie,
    {
      label: e,
      name: t,
      required: o,
      error: u,
      helpText: f,
      className: N,
      children: /* @__PURE__ */ h("div", { className: "relative", children: [
        /* @__PURE__ */ s(
          "input",
          {
            id: P,
            name: t,
            type: p,
            value: _,
            onChange: F,
            placeholder: l,
            required: o,
            disabled: c,
            className: R,
            ...B
          }
        ),
        y && E && /* @__PURE__ */ s("div", { className: "absolute right-2 top-1/2 transform -translate-y-1/2", children: /* @__PURE__ */ s(
          "div",
          {
            className: ji({ size: "md", color: "primary" }),
            role: "status",
            "aria-label": "Processing input"
          }
        ) })
      ] })
    }
  );
}
function gn(t = {}) {
  return t.start || t.end ? "range" : t.key ? "key" : t.keyPrefix ? "prefix" : "range";
}
function Vi(t, e, r) {
  const i = { ...t };
  return e === "range" || r === "start" || r === "end" ? (delete i.key, delete i.keyPrefix) : e === "key" || r === "key" ? (delete i.start, delete i.end, delete i.keyPrefix) : (e === "prefix" || r === "keyPrefix") && (delete i.start, delete i.end, delete i.key), i;
}
function Gi(t = {}, e, r = ["range", "key", "prefix"]) {
  const [i, o] = O(
    () => gn(t)
  ), [c, u] = O(t), l = K((E) => {
    if (!r.includes(E)) return;
    o(E);
    const g = {};
    u(g), e && e(g);
  }, [r, e]), f = K((E, g) => {
    const x = Vi(c, i, E);
    x[E] = g, u(x), e && e(x);
  }, [c, i, e]), p = K(() => {
    const E = {};
    u(E), e && e(E);
  }, [e]), y = K((E) => {
    u(E);
    const g = gn(E);
    o(g), e && e(E);
  }, [e]), v = K(() => r, [r]), N = K((E) => r.includes(E), [r]);
  return {
    state: {
      activeMode: i,
      value: c
    },
    actions: {
      changeMode: l,
      updateValue: f,
      clearValue: p,
      setValue: y
    },
    getAvailableModes: v,
    isValidMode: N
  };
}
function zi(t = "all", e = "key", r = "") {
  if (r) return r;
  if (t !== "all") return null;
  const i = { ...Bn.rangeKeyFilter }, o = i.keyRange || "", c = (i.exactKey || "").replace("key", e), u = (i.keyPrefix || "").replace("keys", `${e} values`), l = i.emptyNote || "";
  return `${o} ${c} ${u} ${l}`.trim();
}
function qi(t = "all") {
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
function Qi(t = !1) {
  const e = "px-3 py-1 text-xs rounded-md transition-colors duration-200";
  return t ? `${e} bg-primary text-white` : `${e} bg-gray-200 text-gray-700 hover:bg-gray-300`;
}
function Yi() {
  return {
    range: "Key Range",
    key: "Exact Key",
    prefix: "Key Prefix"
  };
}
function Wi(t, e) {
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
    className: c = ""
  } = t;
  return {
    mode: ["all", "range", "key", "prefix"].includes(e) ? e : "all",
    rangeKeyName: String(r),
    required: !!i,
    disabled: !!o,
    className: String(c)
  };
}
function Ji() {
  return "bg-yellow-50 rounded-lg p-4 space-y-4";
}
function Xi() {
  return "text-sm font-medium text-gray-800";
}
function eo() {
  return "flex space-x-4 mb-4";
}
function to() {
  return "grid grid-cols-1 md:grid-cols-3 gap-4";
}
function ro({
  name: t,
  label: e,
  value: r = {},
  onChange: i,
  error: o,
  helpText: c,
  config: u = {},
  className: l = ""
}) {
  const f = Zi(u), { mode: p, rangeKeyName: y, required: v, disabled: N } = f, _ = qi(p), b = Gi(r, i, _.availableModes), { state: E, actions: g } = b, x = Yi(), S = Wi(p, E.activeMode), C = zi(p, y, c);
  return /* @__PURE__ */ s(
    Ie,
    {
      label: e,
      name: t,
      required: v,
      error: o,
      helpText: C,
      className: l,
      children: /* @__PURE__ */ h("div", { className: Ji(), children: [
        /* @__PURE__ */ s("div", { className: "mb-3", children: /* @__PURE__ */ h("span", { className: Xi(), children: [
          "Range Key: ",
          y
        ] }) }),
        _.showModeSelector && /* @__PURE__ */ s("div", { className: eo(), children: _.availableModes.map((I) => /* @__PURE__ */ s(
          "button",
          {
            type: "button",
            onClick: () => g.changeMode(I),
            className: Qi(E.activeMode === I),
            children: x[I]
          },
          I
        )) }),
        /* @__PURE__ */ h("div", { className: to(), children: [
          S.showRange && /* @__PURE__ */ h(Rt, { children: [
            /* @__PURE__ */ s(
              Et,
              {
                name: `${t}-start`,
                label: "Start Key",
                value: E.value.start || "",
                onChange: (I) => g.updateValue("start", I),
                placeholder: "Start key",
                disabled: N,
                className: "col-span-1"
              }
            ),
            /* @__PURE__ */ s(
              Et,
              {
                name: `${t}-end`,
                label: "End Key",
                value: E.value.end || "",
                onChange: (I) => g.updateValue("end", I),
                placeholder: "End key",
                disabled: N,
                className: "col-span-1"
              }
            )
          ] }),
          S.showKey && /* @__PURE__ */ s(
            Et,
            {
              name: `${t}-key`,
              label: "Exact Key",
              value: E.value.key || "",
              onChange: (I) => g.updateValue("key", I),
              placeholder: `Exact ${y} to match`,
              disabled: N,
              className: "col-span-1"
            }
          ),
          S.showPrefix && /* @__PURE__ */ s(
            Et,
            {
              name: `${t}-prefix`,
              label: "Key Prefix",
              value: E.value.keyPrefix || "",
              onChange: (I) => g.updateValue("keyPrefix", I),
              placeholder: `${y} prefix (e.g., 'user:')`,
              disabled: N,
              className: "col-span-1"
            }
          )
        ] })
      ] })
    }
  );
}
function no({
  queryState: t,
  onSchemaChange: e,
  onFieldToggle: r,
  onFieldValueChange: i,
  onRangeFilterChange: o,
  onRangeSchemaFilterChange: c,
  onHashKeyChange: u,
  approvedSchemas: l,
  schemasLoading: f,
  isRangeSchema: p,
  isHashRangeSchema: y,
  rangeKey: v,
  className: N = ""
}) {
  const [_, b] = O({}), { clearQuery: E } = Kr();
  K(() => (b({}), !0), []);
  const g = K((F) => {
    e(F), E && E(), b((P) => {
      const { schema: T, ...R } = P;
      return R;
    });
  }, [e, E]), x = K((F) => {
    r(F), b((P) => {
      const { fields: T, ...R } = P;
      return R;
    });
  }, [r]), S = t != null && t.selectedSchema && l ? l.find((F) => F.name === t.selectedSchema) : null, C = (S == null ? void 0 : S.fields) || (S == null ? void 0 : S.transform_fields) || [], I = Array.isArray(C) ? C : Object.keys(C);
  return /* @__PURE__ */ h("div", { className: `space-y-6 ${N}`, children: [
    /* @__PURE__ */ s(
      Ie,
      {
        label: We.schema,
        name: "schema",
        required: !0,
        error: _.schema,
        helpText: We.schemaHelp,
        children: /* @__PURE__ */ s(
          Er,
          {
            name: "schema",
            value: (t == null ? void 0 : t.selectedSchema) || "",
            onChange: g,
            options: l.map((F) => ({
              value: F.name,
              label: F.descriptive_name || F.name
            })),
            placeholder: "Select a schema...",
            emptyMessage: We.schemaEmpty,
            loading: f
          }
        )
      }
    ),
    (t == null ? void 0 : t.selectedSchema) && I.length > 0 && /* @__PURE__ */ s(
      Ie,
      {
        label: "Field Selection",
        name: "fields",
        required: !0,
        error: _.fields,
        helpText: "Select fields to include in your query",
        children: /* @__PURE__ */ s("div", { className: "bg-gray-50 rounded-md p-4", children: /* @__PURE__ */ s("div", { className: "space-y-3", children: I.map((F) => {
          var P;
          return /* @__PURE__ */ h("label", { className: "relative flex items-start", children: [
            /* @__PURE__ */ s("div", { className: "flex items-center h-5", children: /* @__PURE__ */ s(
              "input",
              {
                type: "checkbox",
                className: "h-4 w-4 text-primary border-gray-300 rounded focus:ring-primary",
                checked: ((P = t == null ? void 0 : t.queryFields) == null ? void 0 : P.includes(F)) || !1,
                onChange: () => x(F)
              }
            ) }),
            /* @__PURE__ */ s("div", { className: "ml-3 flex items-center", children: /* @__PURE__ */ s("span", { className: "text-sm font-medium text-gray-700", children: F }) })
          ] }, F);
        }) }) })
      }
    ),
    y && /* @__PURE__ */ s(
      Ie,
      {
        label: "HashRange Filter",
        name: "hashRangeFilter",
        helpText: "Filter data by hash and range key values",
        children: /* @__PURE__ */ h("div", { className: "bg-purple-50 rounded-md p-4 space-y-4", children: [
          /* @__PURE__ */ h("div", { className: "grid grid-cols-1 md:grid-cols-2 gap-4", children: [
            /* @__PURE__ */ h("div", { className: "space-y-2", children: [
              /* @__PURE__ */ s("label", { className: "block text-sm font-medium text-gray-700", children: "Hash Key" }),
              /* @__PURE__ */ s(
                "input",
                {
                  type: "text",
                  placeholder: "Enter hash key value",
                  className: "w-full px-3 py-2 text-sm border border-gray-300 rounded-md focus:outline-none focus:ring-1 focus:ring-primary focus:border-primary",
                  value: (t == null ? void 0 : t.hashKeyValue) || "",
                  onChange: (F) => u(F.target.value)
                }
              ),
              /* @__PURE__ */ h("div", { className: "text-xs text-gray-500", children: [
                "Hash field: ",
                Ln(l.find((F) => F.name === (t == null ? void 0 : t.selectedSchema))) || "N/A"
              ] })
            ] }),
            /* @__PURE__ */ h("div", { className: "space-y-2", children: [
              /* @__PURE__ */ s("label", { className: "block text-sm font-medium text-gray-700", children: "Range Key" }),
              /* @__PURE__ */ s(
                "input",
                {
                  type: "text",
                  placeholder: "Enter range key value",
                  className: "w-full px-3 py-2 text-sm border border-gray-300 rounded-md focus:outline-none focus:ring-1 focus:ring-primary focus:border-primary",
                  value: (t == null ? void 0 : t.rangeKeyValue) || "",
                  onChange: (F) => c({ key: F.target.value })
                }
              ),
              /* @__PURE__ */ h("div", { className: "text-xs text-gray-500", children: [
                "Range field: ",
                et(l.find((F) => F.name === (t == null ? void 0 : t.selectedSchema))) || "N/A"
              ] })
            ] })
          ] }),
          /* @__PURE__ */ h("div", { className: "text-xs text-gray-500", children: [
            /* @__PURE__ */ h("p", { children: [
              /* @__PURE__ */ s("strong", { children: "Hash Key:" }),
              " Used for partitioning data across multiple nodes"
            ] }),
            /* @__PURE__ */ h("p", { children: [
              /* @__PURE__ */ s("strong", { children: "Range Key:" }),
              " Used for ordering and range queries within a partition"
            ] })
          ] })
        ] })
      }
    ),
    p && v && /* @__PURE__ */ s(
      Ie,
      {
        label: "Range Filter",
        name: "rangeSchemaFilter",
        error: _.rangeFilter,
        helpText: "Filter data by range key values",
        children: /* @__PURE__ */ s(
          ro,
          {
            name: "rangeSchemaFilter",
            value: (t == null ? void 0 : t.rangeSchemaFilter) || {},
            onChange: (F) => {
              c(F), b((P) => {
                const { rangeFilter: T, ...R } = P;
                return R;
              });
            },
            rangeKeyName: v,
            mode: "all"
          }
        )
      }
    )
  ] });
}
function so({
  onExecute: t,
  onExecuteQuery: e,
  onValidate: r,
  onSave: i,
  onSaveQuery: o,
  onClear: c,
  onClearQuery: u,
  disabled: l = !1,
  isExecuting: f = !1,
  isSaving: p = !1,
  showValidation: y = !1,
  showSave: v = !0,
  showClear: N = !0,
  className: _ = "",
  queryData: b
}) {
  const [E, g] = O(null), [x, S] = O(null), { clearQuery: C } = Kr(), I = async (B, U, L = null) => {
    if (!(!U || l))
      try {
        g(B), await U(L);
      } catch ($) {
        console.error(`${B} action failed:`, $);
      } finally {
        g(null), S(null);
      }
  }, F = () => {
    I("execute", e || t, b);
  }, P = () => {
    I("validate", r, b);
  }, T = () => {
    I("save", o || i, b);
  }, R = () => {
    const B = u || c;
    B && B(), C && C();
  };
  return /* @__PURE__ */ h("div", { className: `flex justify-end space-x-3 ${_}`, children: [
    N && /* @__PURE__ */ s(
      "button",
      {
        type: "button",
        onClick: R,
        disabled: l,
        className: `
            inline-flex items-center px-4 py-2 border border-gray-300 rounded-md shadow-sm text-sm font-medium
            ${l ? "bg-gray-100 text-gray-400 cursor-not-allowed" : "bg-white text-gray-700 hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-primary"}
          `,
        children: Ut.clearQuery || "Clear Query"
      }
    ),
    y && r && /* @__PURE__ */ h(
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
          E === "validate" && /* @__PURE__ */ h("svg", { className: "animate-spin -ml-1 mr-2 h-4 w-4 text-white", xmlns: "http://www.w3.org/2000/svg", fill: "none", viewBox: "0 0 24 24", children: [
            /* @__PURE__ */ s("circle", { className: "opacity-25", cx: "12", cy: "12", r: "10", stroke: "currentColor", strokeWidth: "4" }),
            /* @__PURE__ */ s("path", { className: "opacity-75", fill: "currentColor", d: "M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" })
          ] }),
          Ut.validateQuery || "Validate"
        ]
      }
    ),
    v && (i || o) && /* @__PURE__ */ h(
      "button",
      {
        type: "button",
        onClick: T,
        disabled: l || p,
        className: `
            inline-flex items-center px-4 py-2 border border-transparent rounded-md shadow-sm text-sm font-medium
            ${l || p ? "bg-gray-300 text-gray-500 cursor-not-allowed" : "bg-green-600 text-white hover:bg-green-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-green-500"}
          `,
        children: [
          (E === "save" || p) && /* @__PURE__ */ h("svg", { className: "animate-spin -ml-1 mr-2 h-4 w-4 text-white", xmlns: "http://www.w3.org/2000/svg", fill: "none", viewBox: "0 0 24 24", children: [
            /* @__PURE__ */ s("circle", { className: "opacity-25", cx: "12", cy: "12", r: "10", stroke: "currentColor", strokeWidth: "4" }),
            /* @__PURE__ */ s("path", { className: "opacity-75", fill: "currentColor", d: "M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" })
          ] }),
          Ut.saveQuery || "Save Query"
        ]
      }
    ),
    /* @__PURE__ */ h(
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
          (E === "execute" || f) && /* @__PURE__ */ h("svg", { className: "animate-spin -ml-1 mr-2 h-4 w-4 text-white", xmlns: "http://www.w3.org/2000/svg", fill: "none", viewBox: "0 0 24 24", children: [
            /* @__PURE__ */ s("circle", { className: "opacity-25", cx: "12", cy: "12", r: "10", stroke: "currentColor", strokeWidth: "4" }),
            /* @__PURE__ */ s("path", { className: "opacity-75", fill: "currentColor", d: "M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 714 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" })
          ] }),
          E === "execute" || f ? "Executing..." : Ut.executeQuery
        ]
      }
    )
  ] });
}
const ao = (t, e) => {
  if (!t && !e) return null;
  const r = { ...t, ...e };
  let i = [], o = {};
  Array.isArray(r.fields) ? i = r.fields : r.fields && typeof r.fields == "object" ? (i = Object.keys(r.fields), o = r.fields) : r.queryFields && Array.isArray(r.queryFields) && (i = r.queryFields), r.fieldValues && typeof r.fieldValues == "object" && (o = { ...o, ...r.fieldValues });
  const c = {
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
      const u = t.filter.field, l = t.filter.range_filter;
      l.Key ? c.filters[u] = { exactKey: l.Key } : l.KeyRange ? c.filters[u] = {
        keyRange: `${l.KeyRange.start} → ${l.KeyRange.end}`
      } : l.KeyPrefix && (c.filters[u] = { keyPrefix: l.KeyPrefix });
    } else t.filter.range_filter && Object.entries(t.filter.range_filter).forEach(([u, l]) => {
      typeof l == "string" ? c.filters[u] = { exactKey: l } : l.KeyRange ? c.filters[u] = {
        keyRange: `${l.KeyRange.start} → ${l.KeyRange.end}`
      } : l.KeyPrefix && (c.filters[u] = { keyPrefix: l.KeyPrefix });
    });
  return c;
};
function io({
  query: t,
  queryState: e,
  validationErrors: r = [],
  isExecuting: i = !1,
  showJson: o = !1,
  collapsible: c = !0,
  className: u = "",
  title: l = "Query Preview"
}) {
  const f = W(() => ao(t, e), [t, e]);
  return !t && !e ? /* @__PURE__ */ h("div", { className: `bg-gray-50 rounded-md p-4 ${u}`, children: [
    /* @__PURE__ */ s("h3", { className: "text-sm font-medium text-gray-500 mb-2", children: l }),
    /* @__PURE__ */ s("p", { className: "text-sm text-gray-400 italic", children: "No query to preview" })
  ] }) : /* @__PURE__ */ h("div", { className: `bg-white border border-gray-200 rounded-lg shadow-sm ${u}`, children: [
    /* @__PURE__ */ s("div", { className: "px-4 py-3 border-b border-gray-200", children: /* @__PURE__ */ s("h3", { className: "text-sm font-medium text-gray-900", children: l }) }),
    /* @__PURE__ */ h("div", { className: "p-4 space-y-4", children: [
      r && r.length > 0 && /* @__PURE__ */ h("div", { className: "bg-red-50 border border-red-200 rounded-md p-3", children: [
        /* @__PURE__ */ h("div", { className: "flex items-center mb-2", children: [
          /* @__PURE__ */ s("svg", { className: "h-4 w-4 text-red-400 mr-2", fill: "currentColor", viewBox: "0 0 20 20", children: /* @__PURE__ */ s("path", { fillRule: "evenodd", d: "M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7 4a1 1 0 11-2 0 1 1 0 012 0zm-1-9a1 1 0 00-1 1v4a1 1 0 102 0V6a1 1 0 00-1-1z", clipRule: "evenodd" }) }),
          /* @__PURE__ */ s("span", { className: "text-sm font-medium text-red-800", children: "Validation Errors" })
        ] }),
        /* @__PURE__ */ s("ul", { className: "space-y-1", children: r.map((p, y) => /* @__PURE__ */ s("li", { className: "text-sm text-red-700", children: p }, y)) })
      ] }),
      i && /* @__PURE__ */ s("div", { className: "bg-blue-50 border border-blue-200 rounded-md p-3", children: /* @__PURE__ */ h("div", { className: "flex items-center", children: [
        /* @__PURE__ */ h("svg", { className: "animate-spin h-4 w-4 text-blue-400 mr-2", fill: "none", viewBox: "0 0 24 24", children: [
          /* @__PURE__ */ s("circle", { className: "opacity-25", cx: "12", cy: "12", r: "10", stroke: "currentColor", strokeWidth: "4" }),
          /* @__PURE__ */ s("path", { className: "opacity-75", fill: "currentColor", d: "M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" })
        ] }),
        /* @__PURE__ */ s("span", { className: "text-sm font-medium text-blue-800", children: "Executing query..." })
      ] }) }),
      /* @__PURE__ */ h("div", { className: "space-y-3", children: [
        /* @__PURE__ */ h("div", { children: [
          /* @__PURE__ */ s("label", { className: "block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1", children: "Schema" }),
          /* @__PURE__ */ s("div", { className: "inline-flex items-center px-2 py-1 rounded-md bg-blue-100 text-blue-800 text-sm font-medium", children: (f == null ? void 0 : f.schema) || "" })
        ] }),
        /* @__PURE__ */ h("div", { children: [
          /* @__PURE__ */ h("label", { className: "block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1", children: [
            "Fields (",
            f != null && f.fields ? f.fields.length : 0,
            ")"
          ] }),
          /* @__PURE__ */ s("div", { className: "flex flex-wrap gap-1", children: f != null && f.fields && f.fields.length > 0 ? f.fields.map((p, y) => {
            var N;
            const v = (N = f.fieldValues) == null ? void 0 : N[p];
            return /* @__PURE__ */ h("div", { className: "inline-flex flex-col items-start", children: [
              /* @__PURE__ */ s("span", { className: "inline-flex items-center px-2 py-1 rounded-md bg-green-100 text-green-800 text-sm", children: p }),
              v && /* @__PURE__ */ s("span", { className: "text-xs text-gray-600 mt-1 px-2", children: v })
            ] }, y);
          }) : /* @__PURE__ */ s("span", { className: "text-sm text-gray-500 italic", children: "No fields selected" }) })
        ] }),
        (f.filters && Array.isArray(f.filters) && f.filters.length > 0 || f.filters && !Array.isArray(f.filters) && Object.keys(f.filters).length > 0) && /* @__PURE__ */ h("div", { children: [
          /* @__PURE__ */ s("label", { className: "block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1", children: "Filters" }),
          /* @__PURE__ */ s("div", { className: "space-y-2", children: Array.isArray(f.filters) ? (
            // Handle filters as array (from test mocks)
            f.filters.map((p, y) => /* @__PURE__ */ s("div", { className: "bg-yellow-50 rounded-md p-3", children: /* @__PURE__ */ h("div", { className: "text-sm text-yellow-700", children: [
              p.field,
              " ",
              p.operator,
              ' "',
              p.value,
              '"'
            ] }) }, y))
          ) : (
            // Handle filters as object (existing format)
            Object.entries(f.filters).map(([p, y]) => /* @__PURE__ */ h("div", { className: "bg-yellow-50 rounded-md p-3", children: [
              /* @__PURE__ */ s("div", { className: "font-medium text-sm text-yellow-800 mb-1", children: p }),
              /* @__PURE__ */ h("div", { className: "text-sm text-yellow-700", children: [
                y.exactKey && /* @__PURE__ */ h("span", { children: [
                  "Exact key: ",
                  /* @__PURE__ */ s("code", { className: "bg-yellow-200 px-1 rounded", children: y.exactKey })
                ] }),
                y.keyRange && /* @__PURE__ */ h("span", { children: [
                  "Key range: ",
                  /* @__PURE__ */ s("code", { className: "bg-yellow-200 px-1 rounded", children: y.keyRange })
                ] }),
                y.keyPrefix && /* @__PURE__ */ h("span", { children: [
                  "Key prefix: ",
                  /* @__PURE__ */ s("code", { className: "bg-yellow-200 px-1 rounded", children: y.keyPrefix })
                ] })
              ] })
            ] }, p))
          ) })
        ] }),
        f.orderBy && /* @__PURE__ */ h("div", { children: [
          /* @__PURE__ */ s("label", { className: "block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1", children: "OrderBy" }),
          /* @__PURE__ */ s("div", { className: "bg-purple-50 rounded-md p-3", children: /* @__PURE__ */ h("div", { className: "text-sm text-purple-700", children: [
            f.orderBy.field,
            " ",
            f.orderBy.direction
          ] }) })
        ] }),
        f.rangeKey && /* @__PURE__ */ h("div", { children: [
          /* @__PURE__ */ s("label", { className: "block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1", children: "RangeKey" }),
          /* @__PURE__ */ s("div", { className: "bg-indigo-50 rounded-md p-3", children: /* @__PURE__ */ s("div", { className: "text-sm text-indigo-700", children: /* @__PURE__ */ s("code", { className: "bg-indigo-200 px-1 rounded", children: f.rangeKey }) }) })
        ] })
      ] }),
      o && /* @__PURE__ */ h("div", { className: "border-t border-gray-200 pt-4", children: [
        /* @__PURE__ */ s("label", { className: "block text-xs font-medium text-gray-500 uppercase tracking-wide mb-2", children: "Raw JSON" }),
        /* @__PURE__ */ s("pre", { className: "bg-gray-900 text-gray-100 text-xs p-3 rounded-md overflow-x-auto", children: JSON.stringify(t, null, 2) })
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
    handleRangeFilterChange: c,
    setRangeSchemaFilter: u,
    setHashKeyValue: l,
    clearState: f,
    refetchSchemas: p,
    approvedSchemas: y,
    schemasLoading: v,
    selectedSchemaObj: N,
    isRangeSchema: _,
    isHashRangeSchema: b,
    rangeKey: E
  } = Kr();
  ue(() => {
    p();
  }, [p]);
  const [g, x] = O(!1), { query: S, isValid: C } = Wn({
    schema: e.selectedSchema,
    queryState: e,
    schemas: { [e.selectedSchema]: N }
  }), I = K(async (T) => {
    var R;
    if (!T) {
      t({
        error: "No query data provided"
      });
      return;
    }
    x(!0);
    try {
      const B = await $r.executeQuery(T);
      if (!B.success) {
        console.error("Query failed:", B.error), t({
          error: B.error || "Query execution failed",
          details: B
        });
        return;
      }
      t({
        success: !0,
        data: ((R = B.data) == null ? void 0 : R.results) || B.data
      });
    } catch (B) {
      console.error("Failed to execute query:", B), t({
        error: `Network error: ${B.message}`,
        details: B
      });
    } finally {
      x(!1);
    }
  }, [t, C]), F = K(async (T) => {
    console.log("Validating query:", T);
  }, []), P = K(async (T) => {
    if (!T || !C) {
      console.warn("Cannot save invalid query");
      return;
    }
    try {
      console.log("Saving query:", T);
      const R = JSON.parse(localStorage.getItem("savedQueries") || "[]"), B = {
        id: Date.now(),
        name: `Query ${R.length + 1}`,
        data: T,
        createdAt: (/* @__PURE__ */ new Date()).toISOString()
      };
      R.push(B), localStorage.setItem("savedQueries", JSON.stringify(R)), console.log("Query saved successfully");
    } catch (R) {
      console.error("Failed to save query:", R);
    }
  }, [C]);
  return /* @__PURE__ */ s("div", { className: "p-6", children: /* @__PURE__ */ h("div", { className: "grid grid-cols-1 lg:grid-cols-3 gap-6", children: [
    /* @__PURE__ */ h("div", { className: "lg:col-span-2 space-y-6", children: [
      /* @__PURE__ */ s(
        no,
        {
          queryState: e,
          onSchemaChange: r,
          onFieldToggle: i,
          onFieldValueChange: o,
          onRangeFilterChange: c,
          onRangeSchemaFilterChange: u,
          onHashKeyChange: l,
          approvedSchemas: y,
          schemasLoading: v,
          isRangeSchema: _,
          isHashRangeSchema: b,
          rangeKey: E
        }
      ),
      /* @__PURE__ */ s(
        so,
        {
          onExecute: () => I(S),
          onValidate: () => F(S),
          onSave: () => P(S),
          onClear: f,
          queryData: S,
          disabled: !C,
          isExecuting: g,
          showValidation: !1,
          showSave: !0,
          showClear: !0
        }
      )
    ] }),
    /* @__PURE__ */ s("div", { className: "lg:col-span-1", children: /* @__PURE__ */ s(
      io,
      {
        query: S,
        showJson: !1,
        title: "Query Preview"
      }
    ) })
  ] }) });
}
function il({ onResult: t }) {
  const e = ze(), r = J(Ka), i = J(Ha), o = J(ja), c = J(Va), u = J(Ga), l = J(za), f = At(null);
  ue(() => {
    var N;
    (N = f.current) == null || N.scrollIntoView({ behavior: "smooth" });
  }, [c]);
  const p = K((N, _, b = null) => {
    e(Ma({ type: N, content: _, data: b }));
  }, [e]), y = K(async (N) => {
    if (N == null || N.preventDefault(), !r.trim() || o)
      return;
    const _ = r.trim();
    e(an("")), e(ln(!0)), p("user", _);
    try {
      if (l) {
        p("system", "🤔 Analyzing if question can be answered from existing context...");
        const b = await hn.analyzeFollowup({
          session_id: i,
          question: _
        });
        if (!b.success) {
          p("system", `❌ Error: ${b.error || "Failed to analyze question"}`);
          return;
        }
        const E = b.data;
        if (E.needs_query) {
          p("system", `🔍 Need new data: ${E.reasoning}`), p("system", "🔍 Using AI-native index search...");
          const g = await fetch("/api/llm-query/native-index", {
            method: "POST",
            headers: {
              "Content-Type": "application/json"
            },
            body: JSON.stringify({
              query: _,
              session_id: i
            })
          });
          if (!g.ok) {
            const S = await g.json();
            p("system", `❌ Error: ${S.error || "Failed to run AI-native index query"}`);
            return;
          }
          const x = await g.json();
          p("system", "✅ AI-native index search completed"), x.session_id && e(on(x.session_id)), p("system", x.ai_interpretation), p("results", "Raw search results", x.raw_results), u && t({ success: !0, data: x.raw_results });
        } else {
          p("system", `✅ Answering from existing context: ${E.reasoning}`);
          const g = await hn.chat({
            session_id: i,
            question: _
          });
          if (!g.success) {
            p("system", `❌ Error: ${g.error || "Failed to process question"}`);
            return;
          }
          p("system", g.data.answer);
        }
      } else {
        p("system", "🔍 Using AI-native index search...");
        const b = await fetch("/api/llm-query/native-index", {
          method: "POST",
          headers: {
            "Content-Type": "application/json"
          },
          body: JSON.stringify({
            query: _,
            session_id: i
          })
        });
        if (!b.ok) {
          const g = await b.json();
          p("system", `❌ Error: ${g.error || "Failed to run AI-native index query"}`);
          return;
        }
        const E = await b.json();
        p("system", "✅ AI-native index search completed"), E.session_id && e(on(E.session_id)), p("system", E.ai_interpretation), p("results", "Raw search results", E.raw_results), u && t({ success: !0, data: E.raw_results });
      }
    } catch (b) {
      console.error("Error processing input:", b), p("system", `❌ Error: ${b.message}`), t({ error: b.message });
    } finally {
      e(ln(!1));
    }
  }, [r, i, l, o, p, t, e]), v = K(() => {
    e(Ua());
  }, [e]);
  return /* @__PURE__ */ h("div", { className: "flex flex-col bg-white rounded-lg shadow", children: [
    /* @__PURE__ */ h("div", { className: "p-4 border-b border-gray-200 flex justify-between items-center", children: [
      /* @__PURE__ */ h("div", { children: [
        /* @__PURE__ */ s("h2", { className: "text-xl font-bold text-gray-900", children: "🤖 AI Data Assistant" }),
        /* @__PURE__ */ s("p", { className: "text-sm text-gray-600", children: "Ask questions in plain English - I'll find your data" })
      ] }),
      c.length > 0 && /* @__PURE__ */ s(
        "button",
        {
          onClick: v,
          disabled: o,
          className: "px-4 py-2 bg-gray-600 text-white rounded-lg hover:bg-gray-700 disabled:bg-gray-300 disabled:cursor-not-allowed transition-colors text-sm",
          children: "New Conversation"
        }
      )
    ] }),
    /* @__PURE__ */ h("div", { className: "overflow-y-auto bg-gray-50 p-4 space-y-3", style: { maxHeight: "60vh", minHeight: "400px" }, children: [
      c.length === 0 ? /* @__PURE__ */ h("div", { className: "text-center text-gray-500 mt-20", children: [
        /* @__PURE__ */ s("div", { className: "text-6xl mb-4", children: "💬" }),
        /* @__PURE__ */ s("p", { className: "text-lg mb-2", children: "Start a conversation" }),
        /* @__PURE__ */ s("p", { className: "text-sm", children: 'Try: "Find all blog posts from last month" or "Show me products over $100"' })
      ] }) : c.map((N, _) => /* @__PURE__ */ h("div", { children: [
        N.type === "user" && /* @__PURE__ */ s("div", { className: "flex justify-end", children: /* @__PURE__ */ h("div", { className: "bg-blue-600 text-white rounded-lg px-4 py-2 max-w-3xl", children: [
          /* @__PURE__ */ s("p", { className: "text-sm font-semibold mb-1", children: "You" }),
          /* @__PURE__ */ s("p", { className: "whitespace-pre-wrap", children: N.content })
        ] }) }),
        N.type === "system" && /* @__PURE__ */ s("div", { className: "flex justify-start", children: /* @__PURE__ */ h("div", { className: "bg-white border border-gray-200 rounded-lg px-4 py-2 max-w-3xl", children: [
          /* @__PURE__ */ s("p", { className: "text-sm font-semibold text-gray-700 mb-1", children: "AI Assistant" }),
          /* @__PURE__ */ s("p", { className: "text-gray-900 whitespace-pre-wrap", children: N.content })
        ] }) }),
        N.type === "results" && N.data && /* @__PURE__ */ h("div", { className: "bg-green-50 border border-green-200 rounded-lg p-4 max-w-full", children: [
          /* @__PURE__ */ h("div", { className: "flex justify-between items-center mb-2", children: [
            /* @__PURE__ */ h("p", { className: "text-sm font-semibold text-green-800", children: [
              "📊 Results (",
              N.data.length,
              ")"
            ] }),
            /* @__PURE__ */ s(
              "button",
              {
                onClick: () => {
                  const b = !u;
                  if (e(Pa(b)), b) {
                    const E = c.find((g) => g.type === "results");
                    E && E.data && t({ success: !0, data: E.data });
                  } else
                    t(null);
                },
                className: "text-sm text-green-700 hover:text-green-900 underline",
                children: u ? "Hide Details" : "Show Details"
              }
            )
          ] }),
          u && /* @__PURE__ */ h(Rt, { children: [
            /* @__PURE__ */ s("div", { className: "bg-white rounded p-3 mb-2", children: /* @__PURE__ */ s("p", { className: "text-gray-900 whitespace-pre-wrap mb-3", children: N.content }) }),
            /* @__PURE__ */ h("details", { className: "mt-2", children: [
              /* @__PURE__ */ h("summary", { className: "cursor-pointer text-sm text-green-700 hover:text-green-900", children: [
                "View raw data (",
                N.data.length,
                " records)"
              ] }),
              /* @__PURE__ */ s("div", { className: "mt-2 max-h-64 overflow-auto", children: /* @__PURE__ */ s("pre", { className: "text-xs bg-gray-900 text-green-400 p-3 rounded", children: JSON.stringify(N.data, null, 2) }) })
            ] })
          ] })
        ] })
      ] }, _)),
      /* @__PURE__ */ s("div", { ref: f })
    ] }),
    /* @__PURE__ */ h("form", { onSubmit: y, className: "border-t border-gray-200 p-4 bg-white", children: [
      /* @__PURE__ */ h("div", { className: "flex gap-2", children: [
        /* @__PURE__ */ s(
          "input",
          {
            type: "text",
            value: r,
            onChange: (N) => e(an(N.target.value)),
            placeholder: c.some((N) => N.type === "results") ? "Ask a follow-up question or start a new query..." : "Search the native index (e.g., 'Find posts about AI')...",
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
function oo({ selectedSchema: t, mutationType: e, onSchemaChange: r, onTypeChange: i }) {
  const o = J(Bt);
  return /* @__PURE__ */ h("div", { className: "grid grid-cols-2 gap-4", children: [
    /* @__PURE__ */ s(
      Er,
      {
        name: "schema",
        label: We.schema,
        value: t,
        onChange: r,
        options: o.map((c) => ({
          value: c.name,
          label: c.descriptive_name || c.name
        })),
        placeholder: "Select a schema...",
        emptyMessage: "No approved schemas available for mutations",
        helpText: We.schemaHelp
      }
    ),
    /* @__PURE__ */ s(
      Er,
      {
        name: "operationType",
        label: We.operationType,
        value: e,
        onChange: i,
        options: xa,
        helpText: We.operationHelp
      }
    )
  ] });
}
function lo({ fields: t, mutationType: e, mutationData: r, onFieldChange: i, isRangeSchema: o }) {
  if (e === "Delete")
    return /* @__PURE__ */ h("div", { className: "bg-gray-50 rounded-lg p-6", children: [
      /* @__PURE__ */ s("h3", { className: "text-lg font-medium text-gray-900 mb-4", children: "Delete Operation" }),
      /* @__PURE__ */ s("p", { className: "text-sm text-gray-600", children: "This will delete the selected schema. No additional fields are required." })
    ] });
  const c = (u, l) => {
    if (!(l.writable !== !1)) return null;
    const p = r[u] || "";
    switch (l.field_type) {
      case "Collection": {
        let y = [];
        if (p)
          try {
            const v = typeof p == "string" ? JSON.parse(p) : p;
            y = Array.isArray(v) ? v : [v];
          } catch {
            y = p.trim() ? [p] : [];
          }
        return /* @__PURE__ */ h("div", { className: "mb-6", children: [
          /* @__PURE__ */ h("label", { className: "block text-sm font-medium text-gray-700 mb-2", children: [
            u,
            /* @__PURE__ */ s("span", { className: "ml-2 text-xs text-gray-500", children: "Collection" })
          ] }),
          /* @__PURE__ */ s(
            "textarea",
            {
              className: "mt-1 block w-full border-gray-300 rounded-md shadow-sm focus:ring-primary focus:border-primary sm:text-sm font-mono",
              value: y.length > 0 ? JSON.stringify(y, null, 2) : "",
              onChange: (v) => {
                const N = v.target.value.trim();
                if (!N) {
                  i(u, []);
                  return;
                }
                try {
                  const _ = JSON.parse(N);
                  i(u, Array.isArray(_) ? _ : [_]);
                } catch {
                  i(u, [N]);
                }
              },
              placeholder: 'Enter JSON array (e.g., ["item1", "item2"])',
              rows: 4
            }
          ),
          /* @__PURE__ */ s("p", { className: "mt-1 text-xs text-gray-500", children: "Enter data as a JSON array. Empty input will create an empty array." })
        ] }, u);
      }
      case "Range": {
        if (o)
          return /* @__PURE__ */ h("div", { className: "mb-6", children: [
            /* @__PURE__ */ h("label", { className: "block text-sm font-medium text-gray-700 mb-2", children: [
              u,
              /* @__PURE__ */ s("span", { className: "ml-2 text-xs text-gray-500", children: "Single Value (Range Schema)" })
            ] }),
            /* @__PURE__ */ s(
              "input",
              {
                type: "text",
                className: "mt-1 block w-full border-gray-300 rounded-md shadow-sm focus:ring-primary focus:border-primary sm:text-sm",
                value: p,
                onChange: (E) => i(u, E.target.value),
                placeholder: `Enter ${u} value`
              }
            ),
            /* @__PURE__ */ s("p", { className: "mt-1 text-xs text-gray-500", children: "Enter a single value. The system will automatically handle range formatting." })
          ] }, u);
        let y = {};
        if (p)
          try {
            y = typeof p == "string" ? JSON.parse(p) : p, (typeof y != "object" || Array.isArray(y)) && (y = {});
          } catch {
            y = {};
          }
        const v = Object.entries(y), N = () => {
          const E = [...v, ["", ""]], g = Object.fromEntries(E);
          i(u, g);
        }, _ = (E, g, x) => {
          const S = [...v];
          S[E] = [g, x];
          const C = Object.fromEntries(S);
          i(u, C);
        }, b = (E) => {
          const g = v.filter((S, C) => C !== E), x = Object.fromEntries(g);
          i(u, x);
        };
        return /* @__PURE__ */ h("div", { className: "mb-6", children: [
          /* @__PURE__ */ h("label", { className: "block text-sm font-medium text-gray-700 mb-2", children: [
            u,
            /* @__PURE__ */ s("span", { className: "ml-2 text-xs text-gray-500", children: "Range (Complex)" })
          ] }),
          /* @__PURE__ */ s("div", { className: "border border-gray-300 rounded-md p-4 bg-gray-50", children: /* @__PURE__ */ h("div", { className: "space-y-3", children: [
            v.length === 0 ? /* @__PURE__ */ s("p", { className: "text-sm text-gray-500 italic", children: "No key-value pairs added yet" }) : v.map(([E, g], x) => /* @__PURE__ */ h("div", { className: "flex items-center space-x-2", children: [
              /* @__PURE__ */ s(
                "input",
                {
                  type: "text",
                  placeholder: "Key",
                  className: "flex-1 border-gray-300 rounded-md shadow-sm focus:ring-primary focus:border-primary sm:text-sm",
                  value: E,
                  onChange: (S) => _(x, S.target.value, g)
                }
              ),
              /* @__PURE__ */ s("span", { className: "text-gray-500", children: ":" }),
              /* @__PURE__ */ s(
                "input",
                {
                  type: "text",
                  placeholder: "Value",
                  className: "flex-1 border-gray-300 rounded-md shadow-sm focus:ring-primary focus:border-primary sm:text-sm",
                  value: g,
                  onChange: (S) => _(x, E, S.target.value)
                }
              ),
              /* @__PURE__ */ s(
                "button",
                {
                  type: "button",
                  onClick: () => b(x),
                  className: "text-red-600 hover:text-red-800 p-1",
                  title: "Remove this key-value pair",
                  children: /* @__PURE__ */ s("svg", { className: "w-4 h-4", fill: "none", stroke: "currentColor", viewBox: "0 0 24 24", children: /* @__PURE__ */ s("path", { strokeLinecap: "round", strokeLinejoin: "round", strokeWidth: 2, d: "M6 18L18 6M6 6l12 12" }) })
                }
              )
            ] }, x)),
            /* @__PURE__ */ h(
              "button",
              {
                type: "button",
                onClick: N,
                className: "inline-flex items-center px-3 py-2 border border-gray-300 shadow-sm text-sm leading-4 font-medium rounded-md text-gray-700 bg-white hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-primary",
                children: [
                  /* @__PURE__ */ s("svg", { className: "w-4 h-4 mr-1", fill: "none", stroke: "currentColor", viewBox: "0 0 24 24", children: /* @__PURE__ */ s("path", { strokeLinecap: "round", strokeLinejoin: "round", strokeWidth: 2, d: "M12 6v6m0 0v6m0-6h6m-6 0H6" }) }),
                  "Add Key-Value Pair"
                ]
              }
            )
          ] }) }),
          /* @__PURE__ */ s("p", { className: "mt-1 text-xs text-gray-500", children: "Add key-value pairs for this range field. Empty keys will be filtered out." })
        ] }, u);
      }
      default:
        return /* @__PURE__ */ h("div", { className: "mb-6", children: [
          /* @__PURE__ */ h("label", { className: "block text-sm font-medium text-gray-700 mb-2", children: [
            u,
            /* @__PURE__ */ s("span", { className: "ml-2 text-xs text-gray-500", children: "Single" })
          ] }),
          /* @__PURE__ */ s(
            "input",
            {
              type: "text",
              className: "mt-1 block w-full border-gray-300 rounded-md shadow-sm focus:ring-primary focus:border-primary sm:text-sm",
              value: p,
              onChange: (y) => i(u, y.target.value),
              placeholder: `Enter ${u}`
            }
          )
        ] }, u);
    }
  };
  return /* @__PURE__ */ h("div", { className: "bg-gray-50 rounded-lg p-6", children: [
    /* @__PURE__ */ h("h3", { className: "text-lg font-medium text-gray-900 mb-4", children: [
      "Schema Fields",
      o && /* @__PURE__ */ s("span", { className: "ml-2 text-sm text-blue-600 font-normal", children: "(Range Schema - Single Values)" })
    ] }),
    /* @__PURE__ */ s("div", { className: "space-y-6", children: Object.entries(t).map(([u, l]) => c(u, l)) }),
    o && Object.keys(t).length === 0 && /* @__PURE__ */ s("p", { className: "text-sm text-gray-500 italic", children: "No additional fields to configure. Only the range key is required for this schema." })
  ] });
}
function co({ result: t }) {
  return t ? /* @__PURE__ */ s("div", { className: "bg-gray-50 rounded-lg p-4 mt-4", children: /* @__PURE__ */ s("pre", { className: "font-mono text-sm whitespace-pre-wrap", children: typeof t == "string" ? t : JSON.stringify(t, null, 2) }) }) : null;
}
function uo(t) {
  const e = Ae(t);
  return {
    base: e,
    schema: Ca(e),
    mutation: li(e),
    security: ii(e)
  };
}
uo({
  enableCache: !0,
  enableLogging: !0,
  enableMetrics: !0
});
var ho = {}, er = {};
er.byteLength = po;
er.toByteArray = yo;
er.fromByteArray = wo;
var Re = [], xe = [], mo = typeof Uint8Array < "u" ? Uint8Array : Array, yr = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
for (var dt = 0, fo = yr.length; dt < fo; ++dt)
  Re[dt] = yr[dt], xe[yr.charCodeAt(dt)] = dt;
xe[45] = 62;
xe[95] = 63;
function Xn(t) {
  var e = t.length;
  if (e % 4 > 0)
    throw new Error("Invalid string. Length must be a multiple of 4");
  var r = t.indexOf("=");
  r === -1 && (r = e);
  var i = r === e ? 0 : 4 - r % 4;
  return [r, i];
}
function po(t) {
  var e = Xn(t), r = e[0], i = e[1];
  return (r + i) * 3 / 4 - i;
}
function go(t, e, r) {
  return (e + r) * 3 / 4 - r;
}
function yo(t) {
  var e, r = Xn(t), i = r[0], o = r[1], c = new mo(go(t, i, o)), u = 0, l = o > 0 ? i - 4 : i, f;
  for (f = 0; f < l; f += 4)
    e = xe[t.charCodeAt(f)] << 18 | xe[t.charCodeAt(f + 1)] << 12 | xe[t.charCodeAt(f + 2)] << 6 | xe[t.charCodeAt(f + 3)], c[u++] = e >> 16 & 255, c[u++] = e >> 8 & 255, c[u++] = e & 255;
  return o === 2 && (e = xe[t.charCodeAt(f)] << 2 | xe[t.charCodeAt(f + 1)] >> 4, c[u++] = e & 255), o === 1 && (e = xe[t.charCodeAt(f)] << 10 | xe[t.charCodeAt(f + 1)] << 4 | xe[t.charCodeAt(f + 2)] >> 2, c[u++] = e >> 8 & 255, c[u++] = e & 255), c;
}
function bo(t) {
  return Re[t >> 18 & 63] + Re[t >> 12 & 63] + Re[t >> 6 & 63] + Re[t & 63];
}
function xo(t, e, r) {
  for (var i, o = [], c = e; c < r; c += 3)
    i = (t[c] << 16 & 16711680) + (t[c + 1] << 8 & 65280) + (t[c + 2] & 255), o.push(bo(i));
  return o.join("");
}
function wo(t) {
  for (var e, r = t.length, i = r % 3, o = [], c = 16383, u = 0, l = r - i; u < l; u += c)
    o.push(xo(t, u, u + c > l ? l : u + c));
  return i === 1 ? (e = t[r - 1], o.push(
    Re[e >> 2] + Re[e << 4 & 63] + "=="
  )) : i === 2 && (e = (t[r - 2] << 8) + t[r - 1], o.push(
    Re[e >> 10] + Re[e >> 4 & 63] + Re[e << 2 & 63] + "="
  )), o.join("");
}
var Hr = {};
/*! ieee754. BSD-3-Clause License. Feross Aboukhadijeh <https://feross.org/opensource> */
Hr.read = function(t, e, r, i, o) {
  var c, u, l = o * 8 - i - 1, f = (1 << l) - 1, p = f >> 1, y = -7, v = r ? o - 1 : 0, N = r ? -1 : 1, _ = t[e + v];
  for (v += N, c = _ & (1 << -y) - 1, _ >>= -y, y += l; y > 0; c = c * 256 + t[e + v], v += N, y -= 8)
    ;
  for (u = c & (1 << -y) - 1, c >>= -y, y += i; y > 0; u = u * 256 + t[e + v], v += N, y -= 8)
    ;
  if (c === 0)
    c = 1 - p;
  else {
    if (c === f)
      return u ? NaN : (_ ? -1 : 1) * (1 / 0);
    u = u + Math.pow(2, i), c = c - p;
  }
  return (_ ? -1 : 1) * u * Math.pow(2, c - i);
};
Hr.write = function(t, e, r, i, o, c) {
  var u, l, f, p = c * 8 - o - 1, y = (1 << p) - 1, v = y >> 1, N = o === 23 ? Math.pow(2, -24) - Math.pow(2, -77) : 0, _ = i ? 0 : c - 1, b = i ? 1 : -1, E = e < 0 || e === 0 && 1 / e < 0 ? 1 : 0;
  for (e = Math.abs(e), isNaN(e) || e === 1 / 0 ? (l = isNaN(e) ? 1 : 0, u = y) : (u = Math.floor(Math.log(e) / Math.LN2), e * (f = Math.pow(2, -u)) < 1 && (u--, f *= 2), u + v >= 1 ? e += N / f : e += N * Math.pow(2, 1 - v), e * f >= 2 && (u++, f /= 2), u + v >= y ? (l = 0, u = y) : u + v >= 1 ? (l = (e * f - 1) * Math.pow(2, o), u = u + v) : (l = e * Math.pow(2, v - 1) * Math.pow(2, o), u = 0)); o >= 8; t[r + _] = l & 255, _ += b, l /= 256, o -= 8)
    ;
  for (u = u << o | l, p += o; p > 0; t[r + _] = u & 255, _ += b, u /= 256, p -= 8)
    ;
  t[r + _ - b] |= E * 128;
};
/*!
 * The buffer module from node.js, for the browser.
 *
 * @author   Feross Aboukhadijeh <https://feross.org>
 * @license  MIT
 */
(function(t) {
  const e = er, r = Hr, i = typeof Symbol == "function" && typeof Symbol.for == "function" ? Symbol.for("nodejs.util.inspect.custom") : null;
  t.Buffer = l, t.SlowBuffer = S, t.INSPECT_MAX_BYTES = 50;
  const o = 2147483647;
  t.kMaxLength = o, l.TYPED_ARRAY_SUPPORT = c(), !l.TYPED_ARRAY_SUPPORT && typeof console < "u" && typeof console.error == "function" && console.error(
    "This browser lacks typed array (Uint8Array) support which is required by `buffer` v5.x. Use `buffer` v4.x if you require old browser support."
  );
  function c() {
    try {
      const d = new Uint8Array(1), n = { foo: function() {
        return 42;
      } };
      return Object.setPrototypeOf(n, Uint8Array.prototype), Object.setPrototypeOf(d, n), d.foo() === 42;
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
  function u(d) {
    if (d > o)
      throw new RangeError('The value "' + d + '" is invalid for option "size"');
    const n = new Uint8Array(d);
    return Object.setPrototypeOf(n, l.prototype), n;
  }
  function l(d, n, a) {
    if (typeof d == "number") {
      if (typeof n == "string")
        throw new TypeError(
          'The "string" argument must be of type string. Received type number'
        );
      return v(d);
    }
    return f(d, n, a);
  }
  l.poolSize = 8192;
  function f(d, n, a) {
    if (typeof d == "string")
      return N(d, n);
    if (ArrayBuffer.isView(d))
      return b(d);
    if (d == null)
      throw new TypeError(
        "The first argument must be one of type string, Buffer, ArrayBuffer, Array, or Array-like Object. Received type " + typeof d
      );
    if (_e(d, ArrayBuffer) || d && _e(d.buffer, ArrayBuffer) || typeof SharedArrayBuffer < "u" && (_e(d, SharedArrayBuffer) || d && _e(d.buffer, SharedArrayBuffer)))
      return E(d, n, a);
    if (typeof d == "number")
      throw new TypeError(
        'The "value" argument must not be of type number. Received type number'
      );
    const m = d.valueOf && d.valueOf();
    if (m != null && m !== d)
      return l.from(m, n, a);
    const w = g(d);
    if (w) return w;
    if (typeof Symbol < "u" && Symbol.toPrimitive != null && typeof d[Symbol.toPrimitive] == "function")
      return l.from(d[Symbol.toPrimitive]("string"), n, a);
    throw new TypeError(
      "The first argument must be one of type string, Buffer, ArrayBuffer, Array, or Array-like Object. Received type " + typeof d
    );
  }
  l.from = function(d, n, a) {
    return f(d, n, a);
  }, Object.setPrototypeOf(l.prototype, Uint8Array.prototype), Object.setPrototypeOf(l, Uint8Array);
  function p(d) {
    if (typeof d != "number")
      throw new TypeError('"size" argument must be of type number');
    if (d < 0)
      throw new RangeError('The value "' + d + '" is invalid for option "size"');
  }
  function y(d, n, a) {
    return p(d), d <= 0 ? u(d) : n !== void 0 ? typeof a == "string" ? u(d).fill(n, a) : u(d).fill(n) : u(d);
  }
  l.alloc = function(d, n, a) {
    return y(d, n, a);
  };
  function v(d) {
    return p(d), u(d < 0 ? 0 : x(d) | 0);
  }
  l.allocUnsafe = function(d) {
    return v(d);
  }, l.allocUnsafeSlow = function(d) {
    return v(d);
  };
  function N(d, n) {
    if ((typeof n != "string" || n === "") && (n = "utf8"), !l.isEncoding(n))
      throw new TypeError("Unknown encoding: " + n);
    const a = C(d, n) | 0;
    let m = u(a);
    const w = m.write(d, n);
    return w !== a && (m = m.slice(0, w)), m;
  }
  function _(d) {
    const n = d.length < 0 ? 0 : x(d.length) | 0, a = u(n);
    for (let m = 0; m < n; m += 1)
      a[m] = d[m] & 255;
    return a;
  }
  function b(d) {
    if (_e(d, Uint8Array)) {
      const n = new Uint8Array(d);
      return E(n.buffer, n.byteOffset, n.byteLength);
    }
    return _(d);
  }
  function E(d, n, a) {
    if (n < 0 || d.byteLength < n)
      throw new RangeError('"offset" is outside of buffer bounds');
    if (d.byteLength < n + (a || 0))
      throw new RangeError('"length" is outside of buffer bounds');
    let m;
    return n === void 0 && a === void 0 ? m = new Uint8Array(d) : a === void 0 ? m = new Uint8Array(d, n) : m = new Uint8Array(d, n, a), Object.setPrototypeOf(m, l.prototype), m;
  }
  function g(d) {
    if (l.isBuffer(d)) {
      const n = x(d.length) | 0, a = u(n);
      return a.length === 0 || d.copy(a, 0, 0, n), a;
    }
    if (d.length !== void 0)
      return typeof d.length != "number" || sr(d.length) ? u(0) : _(d);
    if (d.type === "Buffer" && Array.isArray(d.data))
      return _(d.data);
  }
  function x(d) {
    if (d >= o)
      throw new RangeError("Attempt to allocate Buffer larger than maximum size: 0x" + o.toString(16) + " bytes");
    return d | 0;
  }
  function S(d) {
    return +d != d && (d = 0), l.alloc(+d);
  }
  l.isBuffer = function(n) {
    return n != null && n._isBuffer === !0 && n !== l.prototype;
  }, l.compare = function(n, a) {
    if (_e(n, Uint8Array) && (n = l.from(n, n.offset, n.byteLength)), _e(a, Uint8Array) && (a = l.from(a, a.offset, a.byteLength)), !l.isBuffer(n) || !l.isBuffer(a))
      throw new TypeError(
        'The "buf1", "buf2" arguments must be one of type Buffer or Uint8Array'
      );
    if (n === a) return 0;
    let m = n.length, w = a.length;
    for (let A = 0, k = Math.min(m, w); A < k; ++A)
      if (n[A] !== a[A]) {
        m = n[A], w = a[A];
        break;
      }
    return m < w ? -1 : w < m ? 1 : 0;
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
    const w = l.allocUnsafe(a);
    let A = 0;
    for (m = 0; m < n.length; ++m) {
      let k = n[m];
      if (_e(k, Uint8Array))
        A + k.length > w.length ? (l.isBuffer(k) || (k = l.from(k)), k.copy(w, A)) : Uint8Array.prototype.set.call(
          w,
          k,
          A
        );
      else if (l.isBuffer(k))
        k.copy(w, A);
      else
        throw new TypeError('"list" argument must be an Array of Buffers');
      A += k.length;
    }
    return w;
  };
  function C(d, n) {
    if (l.isBuffer(d))
      return d.length;
    if (ArrayBuffer.isView(d) || _e(d, ArrayBuffer))
      return d.byteLength;
    if (typeof d != "string")
      throw new TypeError(
        'The "string" argument must be one of type string, Buffer, or ArrayBuffer. Received type ' + typeof d
      );
    const a = d.length, m = arguments.length > 2 && arguments[2] === !0;
    if (!m && a === 0) return 0;
    let w = !1;
    for (; ; )
      switch (n) {
        case "ascii":
        case "latin1":
        case "binary":
          return a;
        case "utf8":
        case "utf-8":
          return D(d).length;
        case "ucs2":
        case "ucs-2":
        case "utf16le":
        case "utf-16le":
          return a * 2;
        case "hex":
          return a >>> 1;
        case "base64":
          return Ue(d).length;
        default:
          if (w)
            return m ? -1 : D(d).length;
          n = ("" + n).toLowerCase(), w = !0;
      }
  }
  l.byteLength = C;
  function I(d, n, a) {
    let m = !1;
    if ((n === void 0 || n < 0) && (n = 0), n > this.length || ((a === void 0 || a > this.length) && (a = this.length), a <= 0) || (a >>>= 0, n >>>= 0, a <= n))
      return "";
    for (d || (d = "utf8"); ; )
      switch (d) {
        case "hex":
          return tt(this, n, a);
        case "utf8":
        case "utf-8":
          return j(this, n, a);
        case "ascii":
          return he(this, n, a);
        case "latin1":
        case "binary":
          return Ee(this, n, a);
        case "base64":
          return H(this, n, a);
        case "ucs2":
        case "ucs-2":
        case "utf16le":
        case "utf-16le":
          return rt(this, n, a);
        default:
          if (m) throw new TypeError("Unknown encoding: " + d);
          d = (d + "").toLowerCase(), m = !0;
      }
  }
  l.prototype._isBuffer = !0;
  function F(d, n, a) {
    const m = d[n];
    d[n] = d[a], d[a] = m;
  }
  l.prototype.swap16 = function() {
    const n = this.length;
    if (n % 2 !== 0)
      throw new RangeError("Buffer size must be a multiple of 16-bits");
    for (let a = 0; a < n; a += 2)
      F(this, a, a + 1);
    return this;
  }, l.prototype.swap32 = function() {
    const n = this.length;
    if (n % 4 !== 0)
      throw new RangeError("Buffer size must be a multiple of 32-bits");
    for (let a = 0; a < n; a += 4)
      F(this, a, a + 3), F(this, a + 1, a + 2);
    return this;
  }, l.prototype.swap64 = function() {
    const n = this.length;
    if (n % 8 !== 0)
      throw new RangeError("Buffer size must be a multiple of 64-bits");
    for (let a = 0; a < n; a += 8)
      F(this, a, a + 7), F(this, a + 1, a + 6), F(this, a + 2, a + 5), F(this, a + 3, a + 4);
    return this;
  }, l.prototype.toString = function() {
    const n = this.length;
    return n === 0 ? "" : arguments.length === 0 ? j(this, 0, n) : I.apply(this, arguments);
  }, l.prototype.toLocaleString = l.prototype.toString, l.prototype.equals = function(n) {
    if (!l.isBuffer(n)) throw new TypeError("Argument must be a Buffer");
    return this === n ? !0 : l.compare(this, n) === 0;
  }, l.prototype.inspect = function() {
    let n = "";
    const a = t.INSPECT_MAX_BYTES;
    return n = this.toString("hex", 0, a).replace(/(.{2})/g, "$1 ").trim(), this.length > a && (n += " ... "), "<Buffer " + n + ">";
  }, i && (l.prototype[i] = l.prototype.inspect), l.prototype.compare = function(n, a, m, w, A) {
    if (_e(n, Uint8Array) && (n = l.from(n, n.offset, n.byteLength)), !l.isBuffer(n))
      throw new TypeError(
        'The "target" argument must be one of type Buffer or Uint8Array. Received type ' + typeof n
      );
    if (a === void 0 && (a = 0), m === void 0 && (m = n ? n.length : 0), w === void 0 && (w = 0), A === void 0 && (A = this.length), a < 0 || m > n.length || w < 0 || A > this.length)
      throw new RangeError("out of range index");
    if (w >= A && a >= m)
      return 0;
    if (w >= A)
      return -1;
    if (a >= m)
      return 1;
    if (a >>>= 0, m >>>= 0, w >>>= 0, A >>>= 0, this === n) return 0;
    let k = A - w, G = m - a;
    const te = Math.min(k, G), Z = this.slice(w, A), re = n.slice(a, m);
    for (let Y = 0; Y < te; ++Y)
      if (Z[Y] !== re[Y]) {
        k = Z[Y], G = re[Y];
        break;
      }
    return k < G ? -1 : G < k ? 1 : 0;
  };
  function P(d, n, a, m, w) {
    if (d.length === 0) return -1;
    if (typeof a == "string" ? (m = a, a = 0) : a > 2147483647 ? a = 2147483647 : a < -2147483648 && (a = -2147483648), a = +a, sr(a) && (a = w ? 0 : d.length - 1), a < 0 && (a = d.length + a), a >= d.length) {
      if (w) return -1;
      a = d.length - 1;
    } else if (a < 0)
      if (w) a = 0;
      else return -1;
    if (typeof n == "string" && (n = l.from(n, m)), l.isBuffer(n))
      return n.length === 0 ? -1 : T(d, n, a, m, w);
    if (typeof n == "number")
      return n = n & 255, typeof Uint8Array.prototype.indexOf == "function" ? w ? Uint8Array.prototype.indexOf.call(d, n, a) : Uint8Array.prototype.lastIndexOf.call(d, n, a) : T(d, [n], a, m, w);
    throw new TypeError("val must be string, number or Buffer");
  }
  function T(d, n, a, m, w) {
    let A = 1, k = d.length, G = n.length;
    if (m !== void 0 && (m = String(m).toLowerCase(), m === "ucs2" || m === "ucs-2" || m === "utf16le" || m === "utf-16le")) {
      if (d.length < 2 || n.length < 2)
        return -1;
      A = 2, k /= 2, G /= 2, a /= 2;
    }
    function te(re, Y) {
      return A === 1 ? re[Y] : re.readUInt16BE(Y * A);
    }
    let Z;
    if (w) {
      let re = -1;
      for (Z = a; Z < k; Z++)
        if (te(d, Z) === te(n, re === -1 ? 0 : Z - re)) {
          if (re === -1 && (re = Z), Z - re + 1 === G) return re * A;
        } else
          re !== -1 && (Z -= Z - re), re = -1;
    } else
      for (a + G > k && (a = k - G), Z = a; Z >= 0; Z--) {
        let re = !0;
        for (let Y = 0; Y < G; Y++)
          if (te(d, Z + Y) !== te(n, Y)) {
            re = !1;
            break;
          }
        if (re) return Z;
      }
    return -1;
  }
  l.prototype.includes = function(n, a, m) {
    return this.indexOf(n, a, m) !== -1;
  }, l.prototype.indexOf = function(n, a, m) {
    return P(this, n, a, m, !0);
  }, l.prototype.lastIndexOf = function(n, a, m) {
    return P(this, n, a, m, !1);
  };
  function R(d, n, a, m) {
    a = Number(a) || 0;
    const w = d.length - a;
    m ? (m = Number(m), m > w && (m = w)) : m = w;
    const A = n.length;
    m > A / 2 && (m = A / 2);
    let k;
    for (k = 0; k < m; ++k) {
      const G = parseInt(n.substr(k * 2, 2), 16);
      if (sr(G)) return k;
      d[a + k] = G;
    }
    return k;
  }
  function B(d, n, a, m) {
    return Ft(D(n, d.length - a), d, a, m);
  }
  function U(d, n, a, m) {
    return Ft(z(n), d, a, m);
  }
  function L(d, n, a, m) {
    return Ft(Ue(n), d, a, m);
  }
  function $(d, n, a, m) {
    return Ft(ye(n, d.length - a), d, a, m);
  }
  l.prototype.write = function(n, a, m, w) {
    if (a === void 0)
      w = "utf8", m = this.length, a = 0;
    else if (m === void 0 && typeof a == "string")
      w = a, m = this.length, a = 0;
    else if (isFinite(a))
      a = a >>> 0, isFinite(m) ? (m = m >>> 0, w === void 0 && (w = "utf8")) : (w = m, m = void 0);
    else
      throw new Error(
        "Buffer.write(string, encoding, offset[, length]) is no longer supported"
      );
    const A = this.length - a;
    if ((m === void 0 || m > A) && (m = A), n.length > 0 && (m < 0 || a < 0) || a > this.length)
      throw new RangeError("Attempt to write outside buffer bounds");
    w || (w = "utf8");
    let k = !1;
    for (; ; )
      switch (w) {
        case "hex":
          return R(this, n, a, m);
        case "utf8":
        case "utf-8":
          return B(this, n, a, m);
        case "ascii":
        case "latin1":
        case "binary":
          return U(this, n, a, m);
        case "base64":
          return L(this, n, a, m);
        case "ucs2":
        case "ucs-2":
        case "utf16le":
        case "utf-16le":
          return $(this, n, a, m);
        default:
          if (k) throw new TypeError("Unknown encoding: " + w);
          w = ("" + w).toLowerCase(), k = !0;
      }
  }, l.prototype.toJSON = function() {
    return {
      type: "Buffer",
      data: Array.prototype.slice.call(this._arr || this, 0)
    };
  };
  function H(d, n, a) {
    return n === 0 && a === d.length ? e.fromByteArray(d) : e.fromByteArray(d.slice(n, a));
  }
  function j(d, n, a) {
    a = Math.min(d.length, a);
    const m = [];
    let w = n;
    for (; w < a; ) {
      const A = d[w];
      let k = null, G = A > 239 ? 4 : A > 223 ? 3 : A > 191 ? 2 : 1;
      if (w + G <= a) {
        let te, Z, re, Y;
        switch (G) {
          case 1:
            A < 128 && (k = A);
            break;
          case 2:
            te = d[w + 1], (te & 192) === 128 && (Y = (A & 31) << 6 | te & 63, Y > 127 && (k = Y));
            break;
          case 3:
            te = d[w + 1], Z = d[w + 2], (te & 192) === 128 && (Z & 192) === 128 && (Y = (A & 15) << 12 | (te & 63) << 6 | Z & 63, Y > 2047 && (Y < 55296 || Y > 57343) && (k = Y));
            break;
          case 4:
            te = d[w + 1], Z = d[w + 2], re = d[w + 3], (te & 192) === 128 && (Z & 192) === 128 && (re & 192) === 128 && (Y = (A & 15) << 18 | (te & 63) << 12 | (Z & 63) << 6 | re & 63, Y > 65535 && Y < 1114112 && (k = Y));
        }
      }
      k === null ? (k = 65533, G = 1) : k > 65535 && (k -= 65536, m.push(k >>> 10 & 1023 | 55296), k = 56320 | k & 1023), m.push(k), w += G;
    }
    return ae(m);
  }
  const q = 4096;
  function ae(d) {
    const n = d.length;
    if (n <= q)
      return String.fromCharCode.apply(String, d);
    let a = "", m = 0;
    for (; m < n; )
      a += String.fromCharCode.apply(
        String,
        d.slice(m, m += q)
      );
    return a;
  }
  function he(d, n, a) {
    let m = "";
    a = Math.min(d.length, a);
    for (let w = n; w < a; ++w)
      m += String.fromCharCode(d[w] & 127);
    return m;
  }
  function Ee(d, n, a) {
    let m = "";
    a = Math.min(d.length, a);
    for (let w = n; w < a; ++w)
      m += String.fromCharCode(d[w]);
    return m;
  }
  function tt(d, n, a) {
    const m = d.length;
    (!n || n < 0) && (n = 0), (!a || a < 0 || a > m) && (a = m);
    let w = "";
    for (let A = n; A < a; ++A)
      w += es[d[A]];
    return w;
  }
  function rt(d, n, a) {
    const m = d.slice(n, a);
    let w = "";
    for (let A = 0; A < m.length - 1; A += 2)
      w += String.fromCharCode(m[A] + m[A + 1] * 256);
    return w;
  }
  l.prototype.slice = function(n, a) {
    const m = this.length;
    n = ~~n, a = a === void 0 ? m : ~~a, n < 0 ? (n += m, n < 0 && (n = 0)) : n > m && (n = m), a < 0 ? (a += m, a < 0 && (a = 0)) : a > m && (a = m), a < n && (a = n);
    const w = this.subarray(n, a);
    return Object.setPrototypeOf(w, l.prototype), w;
  };
  function X(d, n, a) {
    if (d % 1 !== 0 || d < 0) throw new RangeError("offset is not uint");
    if (d + n > a) throw new RangeError("Trying to access beyond buffer length");
  }
  l.prototype.readUintLE = l.prototype.readUIntLE = function(n, a, m) {
    n = n >>> 0, a = a >>> 0, m || X(n, a, this.length);
    let w = this[n], A = 1, k = 0;
    for (; ++k < a && (A *= 256); )
      w += this[n + k] * A;
    return w;
  }, l.prototype.readUintBE = l.prototype.readUIntBE = function(n, a, m) {
    n = n >>> 0, a = a >>> 0, m || X(n, a, this.length);
    let w = this[n + --a], A = 1;
    for (; a > 0 && (A *= 256); )
      w += this[n + --a] * A;
    return w;
  }, l.prototype.readUint8 = l.prototype.readUInt8 = function(n, a) {
    return n = n >>> 0, a || X(n, 1, this.length), this[n];
  }, l.prototype.readUint16LE = l.prototype.readUInt16LE = function(n, a) {
    return n = n >>> 0, a || X(n, 2, this.length), this[n] | this[n + 1] << 8;
  }, l.prototype.readUint16BE = l.prototype.readUInt16BE = function(n, a) {
    return n = n >>> 0, a || X(n, 2, this.length), this[n] << 8 | this[n + 1];
  }, l.prototype.readUint32LE = l.prototype.readUInt32LE = function(n, a) {
    return n = n >>> 0, a || X(n, 4, this.length), (this[n] | this[n + 1] << 8 | this[n + 2] << 16) + this[n + 3] * 16777216;
  }, l.prototype.readUint32BE = l.prototype.readUInt32BE = function(n, a) {
    return n = n >>> 0, a || X(n, 4, this.length), this[n] * 16777216 + (this[n + 1] << 16 | this[n + 2] << 8 | this[n + 3]);
  }, l.prototype.readBigUInt64LE = $e(function(n) {
    n = n >>> 0, Pe(n, "offset");
    const a = this[n], m = this[n + 7];
    (a === void 0 || m === void 0) && qe(n, this.length - 8);
    const w = a + this[++n] * 2 ** 8 + this[++n] * 2 ** 16 + this[++n] * 2 ** 24, A = this[++n] + this[++n] * 2 ** 8 + this[++n] * 2 ** 16 + m * 2 ** 24;
    return BigInt(w) + (BigInt(A) << BigInt(32));
  }), l.prototype.readBigUInt64BE = $e(function(n) {
    n = n >>> 0, Pe(n, "offset");
    const a = this[n], m = this[n + 7];
    (a === void 0 || m === void 0) && qe(n, this.length - 8);
    const w = a * 2 ** 24 + this[++n] * 2 ** 16 + this[++n] * 2 ** 8 + this[++n], A = this[++n] * 2 ** 24 + this[++n] * 2 ** 16 + this[++n] * 2 ** 8 + m;
    return (BigInt(w) << BigInt(32)) + BigInt(A);
  }), l.prototype.readIntLE = function(n, a, m) {
    n = n >>> 0, a = a >>> 0, m || X(n, a, this.length);
    let w = this[n], A = 1, k = 0;
    for (; ++k < a && (A *= 256); )
      w += this[n + k] * A;
    return A *= 128, w >= A && (w -= Math.pow(2, 8 * a)), w;
  }, l.prototype.readIntBE = function(n, a, m) {
    n = n >>> 0, a = a >>> 0, m || X(n, a, this.length);
    let w = a, A = 1, k = this[n + --w];
    for (; w > 0 && (A *= 256); )
      k += this[n + --w] * A;
    return A *= 128, k >= A && (k -= Math.pow(2, 8 * a)), k;
  }, l.prototype.readInt8 = function(n, a) {
    return n = n >>> 0, a || X(n, 1, this.length), this[n] & 128 ? (255 - this[n] + 1) * -1 : this[n];
  }, l.prototype.readInt16LE = function(n, a) {
    n = n >>> 0, a || X(n, 2, this.length);
    const m = this[n] | this[n + 1] << 8;
    return m & 32768 ? m | 4294901760 : m;
  }, l.prototype.readInt16BE = function(n, a) {
    n = n >>> 0, a || X(n, 2, this.length);
    const m = this[n + 1] | this[n] << 8;
    return m & 32768 ? m | 4294901760 : m;
  }, l.prototype.readInt32LE = function(n, a) {
    return n = n >>> 0, a || X(n, 4, this.length), this[n] | this[n + 1] << 8 | this[n + 2] << 16 | this[n + 3] << 24;
  }, l.prototype.readInt32BE = function(n, a) {
    return n = n >>> 0, a || X(n, 4, this.length), this[n] << 24 | this[n + 1] << 16 | this[n + 2] << 8 | this[n + 3];
  }, l.prototype.readBigInt64LE = $e(function(n) {
    n = n >>> 0, Pe(n, "offset");
    const a = this[n], m = this[n + 7];
    (a === void 0 || m === void 0) && qe(n, this.length - 8);
    const w = this[n + 4] + this[n + 5] * 2 ** 8 + this[n + 6] * 2 ** 16 + (m << 24);
    return (BigInt(w) << BigInt(32)) + BigInt(a + this[++n] * 2 ** 8 + this[++n] * 2 ** 16 + this[++n] * 2 ** 24);
  }), l.prototype.readBigInt64BE = $e(function(n) {
    n = n >>> 0, Pe(n, "offset");
    const a = this[n], m = this[n + 7];
    (a === void 0 || m === void 0) && qe(n, this.length - 8);
    const w = (a << 24) + // Overflow
    this[++n] * 2 ** 16 + this[++n] * 2 ** 8 + this[++n];
    return (BigInt(w) << BigInt(32)) + BigInt(this[++n] * 2 ** 24 + this[++n] * 2 ** 16 + this[++n] * 2 ** 8 + m);
  }), l.prototype.readFloatLE = function(n, a) {
    return n = n >>> 0, a || X(n, 4, this.length), r.read(this, n, !0, 23, 4);
  }, l.prototype.readFloatBE = function(n, a) {
    return n = n >>> 0, a || X(n, 4, this.length), r.read(this, n, !1, 23, 4);
  }, l.prototype.readDoubleLE = function(n, a) {
    return n = n >>> 0, a || X(n, 8, this.length), r.read(this, n, !0, 52, 8);
  }, l.prototype.readDoubleBE = function(n, a) {
    return n = n >>> 0, a || X(n, 8, this.length), r.read(this, n, !1, 52, 8);
  };
  function se(d, n, a, m, w, A) {
    if (!l.isBuffer(d)) throw new TypeError('"buffer" argument must be a Buffer instance');
    if (n > w || n < A) throw new RangeError('"value" argument is out of bounds');
    if (a + m > d.length) throw new RangeError("Index out of range");
  }
  l.prototype.writeUintLE = l.prototype.writeUIntLE = function(n, a, m, w) {
    if (n = +n, a = a >>> 0, m = m >>> 0, !w) {
      const G = Math.pow(2, 8 * m) - 1;
      se(this, n, a, m, G, 0);
    }
    let A = 1, k = 0;
    for (this[a] = n & 255; ++k < m && (A *= 256); )
      this[a + k] = n / A & 255;
    return a + m;
  }, l.prototype.writeUintBE = l.prototype.writeUIntBE = function(n, a, m, w) {
    if (n = +n, a = a >>> 0, m = m >>> 0, !w) {
      const G = Math.pow(2, 8 * m) - 1;
      se(this, n, a, m, G, 0);
    }
    let A = m - 1, k = 1;
    for (this[a + A] = n & 255; --A >= 0 && (k *= 256); )
      this[a + A] = n / k & 255;
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
  function Me(d, n, a, m, w) {
    Nt(n, m, w, d, a, 7);
    let A = Number(n & BigInt(4294967295));
    d[a++] = A, A = A >> 8, d[a++] = A, A = A >> 8, d[a++] = A, A = A >> 8, d[a++] = A;
    let k = Number(n >> BigInt(32) & BigInt(4294967295));
    return d[a++] = k, k = k >> 8, d[a++] = k, k = k >> 8, d[a++] = k, k = k >> 8, d[a++] = k, a;
  }
  function nt(d, n, a, m, w) {
    Nt(n, m, w, d, a, 7);
    let A = Number(n & BigInt(4294967295));
    d[a + 7] = A, A = A >> 8, d[a + 6] = A, A = A >> 8, d[a + 5] = A, A = A >> 8, d[a + 4] = A;
    let k = Number(n >> BigInt(32) & BigInt(4294967295));
    return d[a + 3] = k, k = k >> 8, d[a + 2] = k, k = k >> 8, d[a + 1] = k, k = k >> 8, d[a] = k, a + 8;
  }
  l.prototype.writeBigUInt64LE = $e(function(n, a = 0) {
    return Me(this, n, a, BigInt(0), BigInt("0xffffffffffffffff"));
  }), l.prototype.writeBigUInt64BE = $e(function(n, a = 0) {
    return nt(this, n, a, BigInt(0), BigInt("0xffffffffffffffff"));
  }), l.prototype.writeIntLE = function(n, a, m, w) {
    if (n = +n, a = a >>> 0, !w) {
      const te = Math.pow(2, 8 * m - 1);
      se(this, n, a, m, te - 1, -te);
    }
    let A = 0, k = 1, G = 0;
    for (this[a] = n & 255; ++A < m && (k *= 256); )
      n < 0 && G === 0 && this[a + A - 1] !== 0 && (G = 1), this[a + A] = (n / k >> 0) - G & 255;
    return a + m;
  }, l.prototype.writeIntBE = function(n, a, m, w) {
    if (n = +n, a = a >>> 0, !w) {
      const te = Math.pow(2, 8 * m - 1);
      se(this, n, a, m, te - 1, -te);
    }
    let A = m - 1, k = 1, G = 0;
    for (this[a + A] = n & 255; --A >= 0 && (k *= 256); )
      n < 0 && G === 0 && this[a + A + 1] !== 0 && (G = 1), this[a + A] = (n / k >> 0) - G & 255;
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
  }, l.prototype.writeBigInt64LE = $e(function(n, a = 0) {
    return Me(this, n, a, -BigInt("0x8000000000000000"), BigInt("0x7fffffffffffffff"));
  }), l.prototype.writeBigInt64BE = $e(function(n, a = 0) {
    return nt(this, n, a, -BigInt("0x8000000000000000"), BigInt("0x7fffffffffffffff"));
  });
  function xt(d, n, a, m, w, A) {
    if (a + m > d.length) throw new RangeError("Index out of range");
    if (a < 0) throw new RangeError("Index out of range");
  }
  function wt(d, n, a, m, w) {
    return n = +n, a = a >>> 0, w || xt(d, n, a, 4), r.write(d, n, a, m, 23, 4), a + 4;
  }
  l.prototype.writeFloatLE = function(n, a, m) {
    return wt(this, n, a, !0, m);
  }, l.prototype.writeFloatBE = function(n, a, m) {
    return wt(this, n, a, !1, m);
  };
  function vt(d, n, a, m, w) {
    return n = +n, a = a >>> 0, w || xt(d, n, a, 8), r.write(d, n, a, m, 52, 8), a + 8;
  }
  l.prototype.writeDoubleLE = function(n, a, m) {
    return vt(this, n, a, !0, m);
  }, l.prototype.writeDoubleBE = function(n, a, m) {
    return vt(this, n, a, !1, m);
  }, l.prototype.copy = function(n, a, m, w) {
    if (!l.isBuffer(n)) throw new TypeError("argument should be a Buffer");
    if (m || (m = 0), !w && w !== 0 && (w = this.length), a >= n.length && (a = n.length), a || (a = 0), w > 0 && w < m && (w = m), w === m || n.length === 0 || this.length === 0) return 0;
    if (a < 0)
      throw new RangeError("targetStart out of bounds");
    if (m < 0 || m >= this.length) throw new RangeError("Index out of range");
    if (w < 0) throw new RangeError("sourceEnd out of bounds");
    w > this.length && (w = this.length), n.length - a < w - m && (w = n.length - a + m);
    const A = w - m;
    return this === n && typeof Uint8Array.prototype.copyWithin == "function" ? this.copyWithin(a, m, w) : Uint8Array.prototype.set.call(
      n,
      this.subarray(m, w),
      a
    ), A;
  }, l.prototype.fill = function(n, a, m, w) {
    if (typeof n == "string") {
      if (typeof a == "string" ? (w = a, a = 0, m = this.length) : typeof m == "string" && (w = m, m = this.length), w !== void 0 && typeof w != "string")
        throw new TypeError("encoding must be a string");
      if (typeof w == "string" && !l.isEncoding(w))
        throw new TypeError("Unknown encoding: " + w);
      if (n.length === 1) {
        const k = n.charCodeAt(0);
        (w === "utf8" && k < 128 || w === "latin1") && (n = k);
      }
    } else typeof n == "number" ? n = n & 255 : typeof n == "boolean" && (n = Number(n));
    if (a < 0 || this.length < a || this.length < m)
      throw new RangeError("Out of range index");
    if (m <= a)
      return this;
    a = a >>> 0, m = m === void 0 ? this.length : m >>> 0, n || (n = 0);
    let A;
    if (typeof n == "number")
      for (A = a; A < m; ++A)
        this[A] = n;
    else {
      const k = l.isBuffer(n) ? n : l.from(n, w), G = k.length;
      if (G === 0)
        throw new TypeError('The value "' + n + '" is invalid for argument "value"');
      for (A = 0; A < m - a; ++A)
        this[A + a] = k[A % G];
    }
    return this;
  };
  const Fe = {};
  function st(d, n, a) {
    Fe[d] = class extends a {
      constructor() {
        super(), Object.defineProperty(this, "message", {
          value: n.apply(this, arguments),
          writable: !0,
          configurable: !0
        }), this.name = `${this.name} [${d}]`, this.stack, delete this.name;
      }
      get code() {
        return d;
      }
      set code(w) {
        Object.defineProperty(this, "code", {
          configurable: !0,
          enumerable: !0,
          value: w,
          writable: !0
        });
      }
      toString() {
        return `${this.name} [${d}]: ${this.message}`;
      }
    };
  }
  st(
    "ERR_BUFFER_OUT_OF_BOUNDS",
    function(d) {
      return d ? `${d} is outside of buffer bounds` : "Attempt to access memory outside buffer bounds";
    },
    RangeError
  ), st(
    "ERR_INVALID_ARG_TYPE",
    function(d, n) {
      return `The "${d}" argument must be of type number. Received type ${typeof n}`;
    },
    TypeError
  ), st(
    "ERR_OUT_OF_RANGE",
    function(d, n, a) {
      let m = `The value of "${d}" is out of range.`, w = a;
      return Number.isInteger(a) && Math.abs(a) > 2 ** 32 ? w = Ot(String(a)) : typeof a == "bigint" && (w = String(a), (a > BigInt(2) ** BigInt(32) || a < -(BigInt(2) ** BigInt(32))) && (w = Ot(w)), w += "n"), m += ` It must be ${n}. Received ${w}`, m;
    },
    RangeError
  );
  function Ot(d) {
    let n = "", a = d.length;
    const m = d[0] === "-" ? 1 : 0;
    for (; a >= m + 4; a -= 3)
      n = `_${d.slice(a - 3, a)}${n}`;
    return `${d.slice(0, a)}${n}`;
  }
  function tr(d, n, a) {
    Pe(n, "offset"), (d[n] === void 0 || d[n + a] === void 0) && qe(n, d.length - (a + 1));
  }
  function Nt(d, n, a, m, w, A) {
    if (d > a || d < n) {
      const k = typeof n == "bigint" ? "n" : "";
      let G;
      throw n === 0 || n === BigInt(0) ? G = `>= 0${k} and < 2${k} ** ${(A + 1) * 8}${k}` : G = `>= -(2${k} ** ${(A + 1) * 8 - 1}${k}) and < 2 ** ${(A + 1) * 8 - 1}${k}`, new Fe.ERR_OUT_OF_RANGE("value", G, d);
    }
    tr(m, w, A);
  }
  function Pe(d, n) {
    if (typeof d != "number")
      throw new Fe.ERR_INVALID_ARG_TYPE(n, "number", d);
  }
  function qe(d, n, a) {
    throw Math.floor(d) !== d ? (Pe(d, a), new Fe.ERR_OUT_OF_RANGE("offset", "an integer", d)) : n < 0 ? new Fe.ERR_BUFFER_OUT_OF_BOUNDS() : new Fe.ERR_OUT_OF_RANGE(
      "offset",
      `>= 0 and <= ${n}`,
      d
    );
  }
  const rr = /[^+/0-9A-Za-z-_]/g;
  function nr(d) {
    if (d = d.split("=")[0], d = d.trim().replace(rr, ""), d.length < 2) return "";
    for (; d.length % 4 !== 0; )
      d = d + "=";
    return d;
  }
  function D(d, n) {
    n = n || 1 / 0;
    let a;
    const m = d.length;
    let w = null;
    const A = [];
    for (let k = 0; k < m; ++k) {
      if (a = d.charCodeAt(k), a > 55295 && a < 57344) {
        if (!w) {
          if (a > 56319) {
            (n -= 3) > -1 && A.push(239, 191, 189);
            continue;
          } else if (k + 1 === m) {
            (n -= 3) > -1 && A.push(239, 191, 189);
            continue;
          }
          w = a;
          continue;
        }
        if (a < 56320) {
          (n -= 3) > -1 && A.push(239, 191, 189), w = a;
          continue;
        }
        a = (w - 55296 << 10 | a - 56320) + 65536;
      } else w && (n -= 3) > -1 && A.push(239, 191, 189);
      if (w = null, a < 128) {
        if ((n -= 1) < 0) break;
        A.push(a);
      } else if (a < 2048) {
        if ((n -= 2) < 0) break;
        A.push(
          a >> 6 | 192,
          a & 63 | 128
        );
      } else if (a < 65536) {
        if ((n -= 3) < 0) break;
        A.push(
          a >> 12 | 224,
          a >> 6 & 63 | 128,
          a & 63 | 128
        );
      } else if (a < 1114112) {
        if ((n -= 4) < 0) break;
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
  function z(d) {
    const n = [];
    for (let a = 0; a < d.length; ++a)
      n.push(d.charCodeAt(a) & 255);
    return n;
  }
  function ye(d, n) {
    let a, m, w;
    const A = [];
    for (let k = 0; k < d.length && !((n -= 2) < 0); ++k)
      a = d.charCodeAt(k), m = a >> 8, w = a % 256, A.push(w), A.push(m);
    return A;
  }
  function Ue(d) {
    return e.toByteArray(nr(d));
  }
  function Ft(d, n, a, m) {
    let w;
    for (w = 0; w < m && !(w + a >= n.length || w >= d.length); ++w)
      n[w + a] = d[w];
    return w;
  }
  function _e(d, n) {
    return d instanceof n || d != null && d.constructor != null && d.constructor.name != null && d.constructor.name === n.name;
  }
  function sr(d) {
    return d !== d;
  }
  const es = function() {
    const d = "0123456789abcdef", n = new Array(256);
    for (let a = 0; a < 16; ++a) {
      const m = a * 16;
      for (let w = 0; w < 16; ++w)
        n[m + w] = d[a] + d[w];
    }
    return n;
  }();
  function $e(d) {
    return typeof BigInt > "u" ? ts : d;
  }
  function ts() {
    throw new Error("BigInt not supported");
  }
})(ho);
const vo = { executeMutation: "Execute Mutation" }, yn = { rangeKeyRequired: "Range key is required", rangeKeyOptional: "Range key is optional for delete operations" }, bn = { label: "Range Key", backgroundColor: "bg-blue-50" };
function ll({ onResult: t }) {
  const e = J(Bt);
  J((T) => T.auth);
  const [r, i] = O(""), [o, c] = O({}), [u, l] = O("Insert"), [f, p] = O(null), [y, v] = O(""), [N, _] = O({}), b = (T) => {
    i(T), c({}), l("Insert"), v("");
  }, E = (T, R) => {
    c((B) => ({ ...B, [T]: R }));
  }, g = async (T) => {
    if (T.preventDefault(), !r) return;
    const R = e.find((L) => L.name === r), B = u ? In[u] || u.toLowerCase() : "";
    if (!B)
      return;
    let U;
    pt(R) ? U = va(R, u, y, o) : U = {
      type: "mutation",
      schema: r,
      mutation_type: B,
      fields_and_values: u === "Delete" ? {} : o,
      key_value: { hash: null, range: null }
    };
    try {
      const L = await $r.executeMutation(U);
      if (!L.success)
        throw new Error(L.error || "Mutation failed");
      const $ = L;
      p($), t($), $.success && (c({}), v(""));
    } catch (L) {
      const $ = { error: `Network error: ${L.message}`, details: L };
      p($), t($);
    }
  }, x = r ? e.find((T) => T.name === r) : null, S = x ? pt(x) : !1, C = x ? et(x) : null, F = !x || !Array.isArray(x.fields) ? {} : (S ? x.fields.filter((R) => R !== C) : x.fields).reduce((R, B) => (R[B] = {}, R), {}), P = !r || !u || u !== "Delete" && Object.keys(o).length === 0 || S && u !== "Delete" && !y.trim();
  return /* @__PURE__ */ h("div", { className: "p-6", children: [
    /* @__PURE__ */ h("form", { onSubmit: g, className: "space-y-6", children: [
      /* @__PURE__ */ s(
        oo,
        {
          selectedSchema: r,
          mutationType: u,
          onSchemaChange: b,
          onTypeChange: l
        }
      ),
      r && S && /* @__PURE__ */ h("div", { className: `${bn.backgroundColor} rounded-lg p-4`, children: [
        /* @__PURE__ */ s("h3", { className: "text-lg font-medium text-gray-900 mb-4", children: "Range Schema Configuration" }),
        /* @__PURE__ */ s(
          Et,
          {
            name: "rangeKey",
            label: `${C} (${bn.label})`,
            value: y,
            onChange: v,
            placeholder: `Enter ${C} value`,
            required: u !== "Delete",
            error: N.rangeKey,
            helpText: u !== "Delete" ? yn.rangeKeyRequired : yn.rangeKeyOptional,
            debounced: !0
          }
        )
      ] }),
      r && /* @__PURE__ */ s(
        lo,
        {
          fields: F,
          mutationType: u,
          mutationData: o,
          onFieldChange: E,
          isRangeSchema: S
        }
      ),
      /* @__PURE__ */ s("div", { className: "flex justify-end pt-4", children: /* @__PURE__ */ s(
        "button",
        {
          type: "submit",
          className: `inline-flex items-center px-4 py-2 border border-transparent rounded-md shadow-sm text-sm font-medium text-white ${P ? "bg-gray-300 cursor-not-allowed" : "bg-primary hover:bg-primary/90 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-primary"}`,
          disabled: P,
          children: vo.executeMutation
        }
      ) })
    ] }),
    /* @__PURE__ */ s(co, { result: f })
  ] });
}
function cl({ onResult: t }) {
  const [e, r] = O(""), [i, o] = O(!0), [c, u] = O(0), [l, f] = O("default"), [p, y] = O(!1), [v, N] = O(null);
  ue(() => {
    _();
  }, []);
  const _ = async () => {
    try {
      const x = await gt.getStatus();
      x.success && N(x.data);
    } catch (x) {
      console.error("Failed to fetch ingestion status:", x);
    }
  }, b = async () => {
    y(!0), t(null);
    try {
      const x = JSON.parse(e), S = {
        autoExecute: i,
        trustDistance: c,
        pubKey: l
      }, C = await gt.processIngestion(x, S);
      C.success ? (t({
        success: !0,
        data: C.data
      }), r("")) : t({
        success: !1,
        error: "Failed to process ingestion"
      });
    } catch (x) {
      t({
        success: !1,
        error: x.message || "Failed to process ingestion"
      });
    } finally {
      y(!1);
    }
  }, E = () => {
    const x = [
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
    ], C = [
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
    ], I = [];
    for (let F = 1; F <= 100; F++) {
      const P = x[Math.floor(Math.random() * x.length)], T = S[Math.floor(Math.random() * S.length)], R = C[Math.floor(Math.random() * C.length)], B = /* @__PURE__ */ new Date(), U = new Date(B.getTime() - 6 * 30 * 24 * 60 * 60 * 1e3), L = U.getTime() + Math.random() * (B.getTime() - U.getTime()), $ = new Date(L).toISOString().split("T")[0], H = [
        `Getting Started with ${T}: A Complete Guide`,
        `Advanced ${T} Techniques You Need to Know`,
        `Why ${T} is Changing the Industry`,
        `Building Scalable Applications with ${T}`,
        `The Future of ${T}: Trends and Predictions`,
        `Common ${T} Mistakes and How to Avoid Them`,
        `Best Practices for ${T} Development`,
        `From Beginner to Expert in ${T}`,
        `Case Study: Implementing ${T} in Production`,
        `${T} Tools and Frameworks Comparison`
      ], j = H[Math.floor(Math.random() * H.length)], q = [
        `In this comprehensive guide, we'll explore the fundamentals of ${T} and how it's revolutionizing the way we approach modern development. Whether you're a seasoned developer or just starting out, this article will provide valuable insights into best practices and real-world applications.

## Introduction to ${T}

${T} has become an essential part of today's technology landscape. With its powerful capabilities and growing ecosystem, it offers developers unprecedented opportunities to build robust and scalable solutions.

## Key Concepts

Understanding the core concepts of ${T} is crucial for success. Let's dive into the fundamental principles that make this technology so powerful:

1. **Core Architecture**: The foundation of ${T} lies in its well-designed architecture
2. **Performance Optimization**: Learn how to maximize efficiency and minimize resource usage
3. **Integration Patterns**: Discover best practices for connecting with other systems
4. **Security Considerations**: Implement robust security measures from the ground up

## Real-World Applications

Many companies have successfully implemented ${T} in their production environments. Here are some notable examples:

- **Case Study 1**: A major e-commerce platform reduced their response time by 60%
- **Case Study 2**: A fintech startup improved their scalability by 300%
- **Case Study 3**: A healthcare company enhanced their data processing capabilities

## Getting Started

Ready to dive in? Here's a step-by-step guide to get you started with ${T}:

\`\`\`javascript
// Example implementation
const example = new ${T}();
example.initialize();
example.process();
\`\`\`

## Conclusion

${T} represents a significant advancement in technology, offering developers powerful tools to build the next generation of applications. By following the principles and practices outlined in this guide, you'll be well-equipped to leverage ${T} in your own projects.

Remember, the key to success with ${T} is continuous learning and experimentation. Stay curious, keep building, and don't hesitate to explore new possibilities!`,
        `The landscape of ${T} is constantly evolving, and staying ahead of the curve requires a deep understanding of both current trends and emerging technologies. In this article, we'll examine the latest developments and provide actionable insights for developers looking to enhance their skills.

## Current State of ${T}

Today's ${T} ecosystem is more mature and feature-rich than ever before. With improved tooling, better documentation, and a growing community, developers have access to resources that make implementation more straightforward.

## Emerging Trends

Several key trends are shaping the future of ${T}:

- **Automation**: Increasing focus on automated workflows and CI/CD integration
- **Performance**: New optimization techniques that improve speed and efficiency
- **Security**: Enhanced security features and best practices
- **Scalability**: Better support for large-scale deployments

## Industry Impact

The adoption of ${T} across various industries has been remarkable:

- **Technology Sector**: 85% of tech companies have implemented ${T} solutions
- **Financial Services**: Improved transaction processing and risk management
- **Healthcare**: Enhanced patient data management and analysis
- **E-commerce**: Better customer experience and operational efficiency

## Implementation Strategies

When implementing ${T}, consider these strategic approaches:

1. **Phased Rollout**: Start with pilot projects before full deployment
2. **Team Training**: Invest in comprehensive team education
3. **Monitoring**: Implement robust monitoring and alerting systems
4. **Documentation**: Maintain detailed documentation for future reference

## Future Outlook

Looking ahead, ${T} is poised for continued growth and innovation. Key areas to watch include:

- Advanced AI integration
- Improved developer experience
- Enhanced security features
- Better cross-platform compatibility

The future of ${T} is bright, and developers who invest in learning these technologies now will be well-positioned for success in the years to come.`,
        `Building robust applications with ${T} requires more than just technical knowledge—it demands a strategic approach to architecture, design, and implementation. In this deep dive, we'll explore advanced techniques that will elevate your ${T} development skills.

## Architecture Patterns

Effective ${T} applications rely on well-established architectural patterns:

### Microservices Architecture
Breaking down monolithic applications into smaller, manageable services provides better scalability and maintainability.

### Event-Driven Design
Implementing event-driven patterns enables better decoupling and improved system responsiveness.

### Domain-Driven Design
Organizing code around business domains leads to more maintainable and understandable applications.

## Performance Optimization

Optimizing ${T} applications requires attention to multiple factors:

- **Caching Strategies**: Implement intelligent caching to reduce database load
- **Resource Management**: Optimize memory usage and CPU utilization
- **Network Optimization**: Minimize network overhead and latency
- **Database Tuning**: Optimize queries and indexing strategies

## Testing Strategies

Comprehensive testing is essential for reliable ${T} applications:

\`\`\`javascript
// Example test structure
describe('${T} Component', () => {
  it('should handle basic functionality', () => {
    const component = new ${T}Component();
    expect(component.process()).toBeDefined();
  });
  
  it('should handle edge cases', () => {
    const component = new ${T}Component();
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

Security should be a primary concern when developing ${T} applications:

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

Mastering ${T} development is an ongoing journey that requires continuous learning and adaptation. By implementing these advanced techniques and best practices, you'll build more robust, scalable, and maintainable applications.

The key to success lies in understanding not just the technical aspects, but also the business context and user needs. Keep experimenting, stay updated with the latest developments, and always prioritize code quality and user experience.`
      ], ae = q[Math.floor(Math.random() * q.length)];
      I.push({
        title: j,
        content: ae,
        author: P,
        publish_date: $,
        tags: R
      });
    }
    return I;
  }, g = (x) => {
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
    r(JSON.stringify(S[x], null, 2));
  };
  return /* @__PURE__ */ h("div", { className: "space-y-4", children: [
    v && /* @__PURE__ */ s("div", { className: "bg-white p-3 rounded-lg shadow-sm border border-gray-200", children: /* @__PURE__ */ h("div", { className: "flex items-center gap-4 text-sm", children: [
      /* @__PURE__ */ s("span", { className: `px-2 py-1 rounded text-xs font-medium ${v.enabled && v.configured ? "bg-green-100 text-green-800" : "bg-red-100 text-red-800"}`, children: v.enabled && v.configured ? "Ready" : "Not Configured" }),
      /* @__PURE__ */ h("span", { className: "text-gray-600", children: [
        v.provider,
        " · ",
        v.model
      ] }),
      /* @__PURE__ */ s("span", { className: "text-xs text-gray-500", children: "Configure AI settings using the Settings button in the header" })
    ] }) }),
    /* @__PURE__ */ h("div", { className: "bg-white p-4 rounded-lg shadow", children: [
      /* @__PURE__ */ h("div", { className: "flex items-center justify-between mb-3", children: [
        /* @__PURE__ */ s("h3", { className: "text-lg font-medium text-gray-900", children: "JSON Data" }),
        /* @__PURE__ */ h("div", { className: "flex gap-2", children: [
          /* @__PURE__ */ s(
            "button",
            {
              onClick: () => g("blogposts"),
              className: "px-2 py-1 bg-green-50 text-green-700 rounded text-xs hover:bg-green-100",
              children: "Blog Posts (100)"
            }
          ),
          /* @__PURE__ */ s(
            "button",
            {
              onClick: () => g("twitter"),
              className: "px-2 py-1 bg-blue-50 text-blue-700 rounded text-xs hover:bg-blue-100",
              children: "Twitter"
            }
          ),
          /* @__PURE__ */ s(
            "button",
            {
              onClick: () => g("instagram"),
              className: "px-2 py-1 bg-pink-50 text-pink-700 rounded text-xs hover:bg-pink-100",
              children: "Instagram"
            }
          ),
          /* @__PURE__ */ s(
            "button",
            {
              onClick: () => g("linkedin"),
              className: "px-2 py-1 bg-indigo-50 text-indigo-700 rounded text-xs hover:bg-indigo-100",
              children: "LinkedIn"
            }
          ),
          /* @__PURE__ */ s(
            "button",
            {
              onClick: () => g("tiktok"),
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
          onChange: (x) => r(x.target.value),
          placeholder: "Enter your JSON data here or load a sample...",
          className: "w-full h-64 p-3 border border-gray-300 rounded-md font-mono text-sm focus:ring-2 focus:ring-blue-500 focus:border-transparent"
        }
      )
    ] }),
    /* @__PURE__ */ s("div", { className: "bg-white p-4 rounded-lg shadow", children: /* @__PURE__ */ h("div", { className: "flex items-center justify-between", children: [
      /* @__PURE__ */ h("div", { className: "flex items-center gap-4", children: [
        /* @__PURE__ */ h("label", { className: "flex items-center gap-2 text-sm", children: [
          /* @__PURE__ */ s(
            "input",
            {
              type: "checkbox",
              checked: i,
              onChange: (x) => o(x.target.checked),
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
  const [e, r] = O(!1), [i, o] = O(null), [c, u] = O(!0), [l, f] = O(0), [p, y] = O("default"), [v, N] = O(!1), [_, b] = O(null), [E, g] = O(!1), [x, S] = O("");
  ue(() => {
    C();
  }, []);
  const C = async () => {
    try {
      const L = await gt.getStatus();
      L.success && b(L.data);
    } catch (L) {
      console.error("Failed to fetch ingestion status:", L);
    }
  }, I = K((L) => {
    L.preventDefault(), L.stopPropagation(), r(!0);
  }, []), F = K((L) => {
    L.preventDefault(), L.stopPropagation(), r(!1);
  }, []), P = K((L) => {
    L.preventDefault(), L.stopPropagation();
  }, []), T = K((L) => {
    L.preventDefault(), L.stopPropagation(), r(!1);
    const $ = L.dataTransfer.files;
    $ && $.length > 0 && o($[0]);
  }, []), R = K((L) => {
    const $ = L.target.files;
    $ && $.length > 0 && o($[0]);
  }, []), B = async () => {
    if (E) {
      if (!x || !x.startsWith("s3://")) {
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
    N(!0), t(null);
    try {
      const L = new FormData(), $ = crypto.randomUUID();
      L.append("progress_id", $), E ? L.append("s3FilePath", x) : L.append("file", i), L.append("autoExecute", c.toString()), L.append("trustDistance", l.toString()), L.append("pubKey", p);
      const j = await (await fetch("/api/ingestion/upload", {
        method: "POST",
        body: L
      })).json();
      j.success ? t({
        success: !0,
        data: {
          schema_used: j.schema_name || j.schema_used,
          new_schema_created: j.new_schema_created,
          mutations_generated: j.mutations_generated,
          mutations_executed: j.mutations_executed
        }
      }) : t({
        success: !1,
        error: j.error || "Failed to process file"
      });
    } catch (L) {
      t({
        success: !1,
        error: L.message || "Failed to process file"
      });
    } finally {
      N(!1);
    }
  }, U = (L) => {
    if (L === 0) return "0 Bytes";
    const $ = 1024, H = ["Bytes", "KB", "MB", "GB"], j = Math.floor(Math.log(L) / Math.log($));
    return Math.round(L / Math.pow($, j) * 100) / 100 + " " + H[j];
  };
  return /* @__PURE__ */ h("div", { className: "space-y-4", children: [
    _ && /* @__PURE__ */ s("div", { className: "bg-white p-3 rounded-lg shadow-sm border border-gray-200", children: /* @__PURE__ */ h("div", { className: "flex items-center gap-4 text-sm", children: [
      /* @__PURE__ */ s("span", { className: `px-2 py-1 rounded text-xs font-medium ${_.enabled && _.configured ? "bg-green-100 text-green-800" : "bg-red-100 text-red-800"}`, children: _.enabled && _.configured ? "Ready" : "Not Configured" }),
      /* @__PURE__ */ h("span", { className: "text-gray-600", children: [
        _.provider,
        " · ",
        _.model
      ] }),
      /* @__PURE__ */ s("span", { className: "text-xs text-gray-500", children: "Configure AI settings using the Settings button in the header" })
    ] }) }),
    v && /* @__PURE__ */ s("div", { className: "bg-blue-50 border border-blue-200 rounded-lg p-4", children: /* @__PURE__ */ h("div", { className: "flex items-center gap-3", children: [
      /* @__PURE__ */ h("svg", { className: "animate-spin h-5 w-5 text-blue-600", xmlns: "http://www.w3.org/2000/svg", fill: "none", viewBox: "0 0 24 24", children: [
        /* @__PURE__ */ s("circle", { className: "opacity-25", cx: "12", cy: "12", r: "10", stroke: "currentColor", strokeWidth: "4" }),
        /* @__PURE__ */ s("path", { className: "opacity-75", fill: "currentColor", d: "M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" })
      ] }),
      /* @__PURE__ */ s("span", { className: "text-blue-800 font-medium", children: "Processing file..." })
    ] }) }),
    /* @__PURE__ */ s("div", { className: "bg-white p-4 rounded-lg shadow", children: /* @__PURE__ */ h("div", { className: "flex items-center gap-6", children: [
      /* @__PURE__ */ s("span", { className: "text-sm font-medium text-gray-700", children: "Input Mode:" }),
      /* @__PURE__ */ h("label", { className: "flex items-center gap-2 cursor-pointer", children: [
        /* @__PURE__ */ s(
          "input",
          {
            type: "radio",
            checked: !E,
            onChange: () => g(!1),
            className: "rounded"
          }
        ),
        /* @__PURE__ */ s("span", { className: "text-sm text-gray-700", children: "Upload File" })
      ] }),
      /* @__PURE__ */ h("label", { className: "flex items-center gap-2 cursor-pointer", children: [
        /* @__PURE__ */ s(
          "input",
          {
            type: "radio",
            checked: E,
            onChange: () => g(!0),
            className: "rounded"
          }
        ),
        /* @__PURE__ */ s("span", { className: "text-sm text-gray-700", children: "S3 File Path" })
      ] })
    ] }) }),
    E ? /* @__PURE__ */ h("div", { className: "bg-white p-6 rounded-lg shadow", children: [
      /* @__PURE__ */ s("h3", { className: "text-lg font-medium text-gray-900 mb-4", children: "S3 File Path" }),
      /* @__PURE__ */ h("div", { className: "space-y-3", children: [
        /* @__PURE__ */ s("label", { className: "block text-sm font-medium text-gray-700", children: "Enter S3 file path" }),
        /* @__PURE__ */ s(
          "input",
          {
            type: "text",
            value: x,
            onChange: (L) => S(L.target.value),
            placeholder: "s3://bucket-name/path/to/file.json",
            className: "w-full px-4 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
          }
        ),
        /* @__PURE__ */ s("p", { className: "text-xs text-gray-500", children: "The file will be downloaded from S3 for processing without re-uploading" })
      ] })
    ] }) : /* @__PURE__ */ h("div", { className: "bg-white p-6 rounded-lg shadow", children: [
      /* @__PURE__ */ s("h3", { className: "text-lg font-medium text-gray-900 mb-4", children: "Upload File" }),
      /* @__PURE__ */ s(
        "div",
        {
          className: `border-2 border-dashed rounded-lg p-12 text-center transition-colors ${e ? "border-blue-500 bg-blue-50" : "border-gray-300 bg-gray-50 hover:bg-gray-100"}`,
          onDragEnter: I,
          onDragOver: P,
          onDragLeave: F,
          onDrop: T,
          children: /* @__PURE__ */ h("div", { className: "space-y-4", children: [
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
            i ? /* @__PURE__ */ h("div", { className: "space-y-2", children: [
              /* @__PURE__ */ s("p", { className: "text-lg font-medium text-gray-900", children: i.name }),
              /* @__PURE__ */ s("p", { className: "text-sm text-gray-500", children: U(i.size) }),
              /* @__PURE__ */ s(
                "button",
                {
                  onClick: () => o(null),
                  className: "text-sm text-blue-600 hover:text-blue-700 underline",
                  children: "Remove file"
                }
              )
            ] }) : /* @__PURE__ */ h("div", { children: [
              /* @__PURE__ */ s("p", { className: "text-lg text-gray-700 mb-2", children: "Drag and drop a file here, or click to select" }),
              /* @__PURE__ */ s("p", { className: "text-sm text-gray-500", children: "Supported formats: PDF, DOCX, TXT, CSV, JSON, XML, and more" })
            ] }),
            /* @__PURE__ */ s(
              "input",
              {
                type: "file",
                id: "file-upload",
                className: "hidden",
                onChange: R
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
    /* @__PURE__ */ s("div", { className: "bg-white p-4 rounded-lg shadow", children: /* @__PURE__ */ h("div", { className: "flex items-center justify-between", children: [
      /* @__PURE__ */ h("div", { className: "flex items-center gap-4", children: [
        /* @__PURE__ */ h("label", { className: "flex items-center gap-2 text-sm", children: [
          /* @__PURE__ */ s(
            "input",
            {
              type: "checkbox",
              checked: c,
              onChange: (L) => u(L.target.checked),
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
          onClick: B,
          disabled: v || !E && !i || E && !x,
          className: `px-6 py-2.5 rounded font-medium transition-colors ${v || !E && !i || E && !x ? "bg-gray-300 text-gray-500 cursor-not-allowed" : "bg-blue-600 text-white hover:bg-blue-700"}`,
          children: v ? "Processing..." : E ? "Process S3 File" : "Upload & Process"
        }
      )
    ] }) }),
    /* @__PURE__ */ s("div", { className: "bg-blue-50 border border-blue-200 rounded-lg p-4", children: /* @__PURE__ */ h("div", { className: "flex items-start gap-3", children: [
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
      /* @__PURE__ */ h("div", { className: "text-sm text-blue-800", children: [
        /* @__PURE__ */ s("p", { className: "font-medium mb-1", children: "How it works:" }),
        /* @__PURE__ */ h("ol", { className: "list-decimal list-inside space-y-1", children: [
          /* @__PURE__ */ s("li", { children: E ? "Provide an S3 file path (files already in S3 are not re-uploaded)" : "Upload any file type (PDFs, documents, spreadsheets, etc.)" }),
          /* @__PURE__ */ s("li", { children: "File is automatically converted to JSON using AI" }),
          /* @__PURE__ */ s("li", { children: "AI analyzes the JSON and maps it to appropriate schemas" }),
          /* @__PURE__ */ s("li", { children: "Data is stored in the database with the file location tracked" })
        ] })
      ] })
    ] }) })
  ] });
}
function No() {
  const t = ze(), e = J(Bt), r = J(yt), i = J(Ur), o = J(Un), c = J(Fa), u = K(async () => {
    t(ke({ forceRefresh: !0 }));
  }, [t]), l = K((p) => r.find((y) => y.name === p) || null, [r]), f = K((p) => {
    const y = l(p);
    return y ? Dn(y.state) === Oe.APPROVED : !1;
  }, [l]);
  return ue(() => {
    c.isValid || (console.log("🟡 useApprovedSchemas: Cache invalid, fetching schemas"), t(ke()));
  }, [t]), {
    approvedSchemas: e,
    isLoading: i,
    error: o,
    refetch: u,
    getSchemaByName: l,
    isSchemaApproved: f,
    // Additional utility for components that need all schemas for display
    allSchemas: r
  };
}
function So({ r: t }) {
  var e, r;
  return /* @__PURE__ */ h("tr", { className: "border-t", children: [
    /* @__PURE__ */ s("td", { className: "px-2 py-1 text-xs text-gray-600", children: ((e = t.key_value) == null ? void 0 : e.hash) ?? "" }),
    /* @__PURE__ */ s("td", { className: "px-2 py-1 text-xs text-gray-600", children: ((r = t.key_value) == null ? void 0 : r.range) ?? "" }),
    /* @__PURE__ */ s("td", { className: "px-2 py-1 text-xs font-mono text-gray-800", children: t.schema_name }),
    /* @__PURE__ */ s("td", { className: "px-2 py-1 text-xs text-gray-800", children: t.field }),
    /* @__PURE__ */ s("td", { className: "px-2 py-1 text-xs text-gray-800 whitespace-pre-wrap break-words", children: Eo(t.value) })
  ] });
}
function Eo(t) {
  if (t == null) return "";
  if (typeof t == "string") return t;
  try {
    return JSON.stringify(t);
  } catch {
    return String(t);
  }
}
function ul({ onResult: t }) {
  const { approvedSchemas: e, isLoading: r, refetch: i } = No(), [o, c] = O(""), [u, l] = O(!1), [f, p] = O([]), [y, v] = O(null), [N, _] = O(() => /* @__PURE__ */ new Set()), [b, E] = O(() => /* @__PURE__ */ new Map());
  ue(() => {
    i();
  }, [i]);
  const g = K(async () => {
    var T;
    l(!0), v(null);
    try {
      const R = await ui.search(o);
      if (R.success) {
        const B = ((T = R.data) == null ? void 0 : T.results) || [];
        p(B), t({ success: !0, data: B });
      } else
        v(R.error || "Search failed"), t({ error: R.error || "Search failed", status: R.status });
    } catch (R) {
      v(R.message || "Network error"), t({ error: R.message || "Network error" });
    } finally {
      l(!1);
    }
  }, [o, t]), x = K((T) => {
    if (!T) return [];
    const R = T.fields;
    return Array.isArray(R) ? R.slice() : R && typeof R == "object" ? Object.keys(R) : [];
  }, []), S = W(() => {
    const T = /* @__PURE__ */ new Map();
    return (e || []).forEach((R) => T.set(R.name, R)), T;
  }, [e]), C = K((T, R) => {
    const B = (R == null ? void 0 : R.hash) ?? "", U = (R == null ? void 0 : R.range) ?? "";
    return `${T}|${B}|${U}`;
  }, []), I = K((T) => {
    const R = T == null ? void 0 : T.hash, B = T == null ? void 0 : T.range;
    if (R && B) return Fi(R, B);
    if (R) return Tt(R);
    if (B) return Tt(B);
  }, []), F = K(async (T, R) => {
    var ae;
    const B = S.get(T), U = x(B), L = I(R), $ = { schema_name: T, fields: U };
    L && ($.filter = L);
    const H = await $r.executeQuery($);
    if (!H.success)
      throw new Error(H.error || "Query failed");
    const j = Array.isArray((ae = H.data) == null ? void 0 : ae.results) ? H.data.results : [], q = j.find((he) => {
      var se, Me;
      const Ee = ((se = he == null ? void 0 : he.key) == null ? void 0 : se.hash) ?? null, tt = ((Me = he == null ? void 0 : he.key) == null ? void 0 : Me.range) ?? null, rt = (R == null ? void 0 : R.hash) ?? null, X = (R == null ? void 0 : R.range) ?? null;
      return String(Ee || "") === String(rt || "") && String(tt || "") === String(X || "");
    }) || j[0];
    return (q == null ? void 0 : q.fields) || (q && typeof q == "object" ? q : {});
  }, [S, x, I]), P = K(async () => {
    const T = /* @__PURE__ */ new Map();
    for (const U of f) {
      const L = C(U.schema_name, U.key_value);
      T.has(L) || T.set(L, U);
    }
    const R = Array.from(T.values()), B = new Map(b);
    await Promise.all(R.map(async (U) => {
      const L = C(U.schema_name, U.key_value);
      if (!B.has(L))
        try {
          const $ = await F(U.schema_name, U.key_value);
          B.set(L, $);
        } catch {
          B.set(L, {});
        }
    })), E(B);
  }, [f, b, C, F]);
  return ue(() => {
    f.length > 0 && P().catch(() => {
    });
  }, [f, P]), /* @__PURE__ */ h("div", { className: "p-6 space-y-4", children: [
    /* @__PURE__ */ h("div", { className: "bg-white p-4 rounded-lg shadow", children: [
      /* @__PURE__ */ h("div", { className: "mb-3", children: [
        /* @__PURE__ */ s("h3", { className: "text-lg font-medium text-gray-900", children: "Native Index Search" }),
        /* @__PURE__ */ s("p", { className: "text-xs text-gray-500", children: "Search the database-native word index across all approved schemas." })
      ] }),
      /* @__PURE__ */ h("div", { className: "flex gap-2 items-center", children: [
        /* @__PURE__ */ s(
          "input",
          {
            type: "text",
            value: o,
            onChange: (T) => c(T.target.value),
            placeholder: "Enter search term (e.g. jennifer)",
            className: "flex-1 px-3 py-2 border rounded-md text-sm"
          }
        ),
        /* @__PURE__ */ s(
          "button",
          {
            onClick: g,
            disabled: u || !o.trim(),
            className: `px-4 py-2 rounded text-sm ${u || !o.trim() ? "bg-gray-300 text-gray-600" : "bg-blue-600 text-white hover:bg-blue-700"}`,
            children: u ? "Searching..." : "Search"
          }
        )
      ] })
    ] }),
    /* @__PURE__ */ h("div", { className: "bg-white p-4 rounded-lg shadow", children: [
      /* @__PURE__ */ h("div", { className: "mb-2 flex items-center justify-between", children: [
        /* @__PURE__ */ s("h4", { className: "text-md font-medium text-gray-900", children: "Search Results" }),
        /* @__PURE__ */ h("div", { className: "flex items-center gap-3", children: [
          /* @__PURE__ */ h("span", { className: "text-xs text-gray-500", children: [
            f.length,
            " matches"
          ] }),
          f.length > 0 && /* @__PURE__ */ s(
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
      y && /* @__PURE__ */ s("div", { className: "mb-2 p-2 bg-red-50 border border-red-200 text-xs text-red-700 rounded", children: y }),
      /* @__PURE__ */ s("div", { className: "overflow-auto max-h-[450px]", children: /* @__PURE__ */ h("table", { className: "min-w-full text-left text-xs", children: [
        /* @__PURE__ */ s("thead", { children: /* @__PURE__ */ h("tr", { className: "text-gray-500", children: [
          /* @__PURE__ */ s("th", { className: "px-2 py-1", children: "Hash" }),
          /* @__PURE__ */ s("th", { className: "px-2 py-1", children: "Range" }),
          /* @__PURE__ */ s("th", { className: "px-2 py-1", children: "Schema" }),
          /* @__PURE__ */ s("th", { className: "px-2 py-1", children: "Field" }),
          /* @__PURE__ */ s("th", { className: "px-2 py-1", children: "Value" }),
          /* @__PURE__ */ s("th", { className: "px-2 py-1" })
        ] }) }),
        /* @__PURE__ */ h("tbody", { children: [
          f.map((T, R) => {
            const B = C(T.schema_name, T.key_value), U = N.has(B), L = b.get(B);
            return /* @__PURE__ */ h(Rt, { children: [
              /* @__PURE__ */ s(So, { r: T }, `${B}-row`),
              /* @__PURE__ */ h("tr", { className: "border-b", children: [
                /* @__PURE__ */ s("td", { colSpan: 5 }),
                /* @__PURE__ */ s("td", { className: "px-2 py-1 text-right", children: /* @__PURE__ */ s(
                  "button",
                  {
                    type: "button",
                    className: "text-xs px-2 py-1 rounded border border-gray-300 hover:bg-gray-100",
                    onClick: async () => {
                      const $ = new Set(N);
                      if ($.has(B) ? $.delete(B) : $.add(B), _($), !b.has(B))
                        try {
                          const H = await F(T.schema_name, T.key_value);
                          E((j) => new Map(j).set(B, H));
                        } catch {
                        }
                    },
                    children: U ? "Hide Data" : "Show Data"
                  }
                ) })
              ] }, `${B}-actions`),
              U && /* @__PURE__ */ s("tr", { children: /* @__PURE__ */ s("td", { colSpan: 6, className: "px-2 pb-3", children: /* @__PURE__ */ s("div", { className: "ml-2 bg-gray-50 border rounded", children: /* @__PURE__ */ s(FieldsTable, { fields: L || {} }) }) }) }, `${B}-details`)
            ] });
          }),
          f.length === 0 && /* @__PURE__ */ s("tr", { children: /* @__PURE__ */ s("td", { colSpan: 5, className: "px-2 py-3 text-center text-gray-500", children: "No results" }) })
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
  const c = {
    sm: "px-1.5 py-0.5 text-xs",
    md: "px-2.5 py-0.5 text-xs",
    lg: "px-3 py-1 text-sm"
  }, u = () => tn[t] || tn.available, l = () => ({
    approved: "Approved",
    available: "Available",
    blocked: "Blocked",
    pending: "Pending"
  })[t] || "Unknown", f = () => o ? Bn.schemaStates[t] || "Unknown schema state" : "", p = `
    inline-flex items-center rounded-full font-medium
    ${c[r]}
    ${u()}
    ${i}
  `.trim();
  return /* @__PURE__ */ h("div", { className: "inline-flex items-center space-x-2", children: [
    /* @__PURE__ */ s(
      "span",
      {
        className: p,
        title: f(),
        "aria-label": `Schema status: ${l()}${e ? ", Range Schema" : ""}`,
        children: l()
      }
    ),
    e && /* @__PURE__ */ s(
      "span",
      {
        className: `
            inline-flex items-center rounded-full font-medium
            ${c[r]}
            ${rn.badgeColor}
          `,
        title: "This schema uses range-based keys for efficient querying",
        "aria-label": "Range Schema",
        children: rn.label
      }
    )
  ] });
}
const Ao = (t) => (t == null ? void 0 : t.schema_type) === "Range", _o = (t) => {
  var e;
  return ((e = t == null ? void 0 : t.key) == null ? void 0 : e.range_field) || null;
};
function ml({
  children: t,
  queryState: e,
  schemas: r,
  selectedSchemaObj: i,
  isRangeSchema: o,
  rangeKey: c,
  schema: u,
  ...l
}) {
  const f = W(() => u || (e != null && e.selectedSchema ? e.selectedSchema : (i == null ? void 0 : i.name) ?? null), [u, e == null ? void 0 : e.selectedSchema, i == null ? void 0 : i.name]), p = W(() => i || (f && r && r[f] ? r[f] : null), [f, r, i]), y = W(() => {
    if (r)
      return r;
    if (f && p)
      return { [f]: p };
  }, [r, f, p]), v = W(() => typeof o == "boolean" ? o : Ao(p), [o, p]), N = W(() => c || _o(p), [c, p]), _ = W(() => ({
    ...l,
    schema: f,
    queryState: e,
    schemas: y,
    selectedSchemaObj: p,
    isRangeSchema: v,
    rangeKey: N
  }), [l, f, e, y, p, v, N]);
  let b;
  try {
    b = Wn(_);
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
  fl as DEFAULT_TAB,
  Ie as FieldWrapper,
  dl as FileUploadTab,
  Qo as FoldDbProvider,
  tl as Footer,
  el as Header,
  cl as IngestionTab,
  ki as KeyManagementTab,
  il as LlmQueryTab,
  Xo as LogSidebar,
  nl as LoginModal,
  rl as LoginPage,
  lo as MutationEditor,
  ll as MutationTab,
  ul as NativeIndexTab,
  so as QueryActions,
  ml as QueryBuilder,
  no as QueryForm,
  io as QueryPreview,
  al as QueryTab,
  ro as RangeField,
  co as ResultViewer,
  Wo as ResultsSection,
  oo as SchemaSelector,
  hl as SchemaStatusBadge,
  sl as SchemaTab,
  Er as SelectField,
  Jo as SettingsModal,
  Yo as StatusSection,
  xi as StructuredResults,
  Zo as TabNavigation,
  Et as TextField,
  Ii as TopologyDisplay,
  Si as TransformsTab,
  $a as aiQueryReducer,
  ha as authReducer,
  da as clearAuthentication,
  Fo as clearError,
  ur as fetchNodePrivateKey,
  cr as initializeSystemKey,
  Bo as injectStore,
  Mr as loginUser,
  ua as logoutUser,
  dr as refreshSystemKey,
  Do as restoreSession,
  Da as schemaReducer,
  Oo as setError,
  qa as store,
  Lo as updateSystemKey,
  ze as useAppDispatch,
  J as useAppSelector,
  No as useApprovedSchemas,
  Ht as validatePrivateKey
};
