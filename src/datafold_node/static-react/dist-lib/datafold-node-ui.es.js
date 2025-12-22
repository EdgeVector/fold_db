var Ai = Object.defineProperty;
var Ti = (e, t, r) => t in e ? Ai(e, t, { enumerable: !0, configurable: !0, writable: !0, value: r }) : e[t] = r;
var lt = (e, t, r) => Ti(e, typeof t != "symbol" ? t + "" : t, r);
import * as Y from "react";
import ha, { createContext as Ci, useState as D, useContext as Ri, useEffect as xe, useMemo as ye, useCallback as H, useRef as or } from "react";
import { Provider as ki, useSelector as Ii, useDispatch as Oi } from "react-redux";
var _s = { exports: {} }, rr = {};
/**
 * @license React
 * react-jsx-runtime.production.min.js
 *
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */
var vn;
function Di() {
  if (vn) return rr;
  vn = 1;
  var e = ha, t = Symbol.for("react.element"), r = Symbol.for("react.fragment"), n = Object.prototype.hasOwnProperty, a = e.__SECRET_INTERNALS_DO_NOT_USE_OR_YOU_WILL_BE_FIRED.ReactCurrentOwner, l = { key: !0, ref: !0, __self: !0, __source: !0 };
  function d(c, f, m) {
    var h, y = {}, x = null, N = null;
    m !== void 0 && (x = "" + m), f.key !== void 0 && (x = "" + f.key), f.ref !== void 0 && (N = f.ref);
    for (h in f) n.call(f, h) && !l.hasOwnProperty(h) && (y[h] = f[h]);
    if (c && c.defaultProps) for (h in f = c.defaultProps, f) y[h] === void 0 && (y[h] = f[h]);
    return { $$typeof: t, type: c, key: x, ref: N, props: y, _owner: a.current };
  }
  return rr.Fragment = r, rr.jsx = d, rr.jsxs = d, rr;
}
var sr = {};
/**
 * @license React
 * react-jsx-runtime.development.js
 *
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */
var wn;
function Fi() {
  return wn || (wn = 1, process.env.NODE_ENV !== "production" && function() {
    var e = ha, t = Symbol.for("react.element"), r = Symbol.for("react.portal"), n = Symbol.for("react.fragment"), a = Symbol.for("react.strict_mode"), l = Symbol.for("react.profiler"), d = Symbol.for("react.provider"), c = Symbol.for("react.context"), f = Symbol.for("react.forward_ref"), m = Symbol.for("react.suspense"), h = Symbol.for("react.suspense_list"), y = Symbol.for("react.memo"), x = Symbol.for("react.lazy"), N = Symbol.for("react.offscreen"), S = Symbol.iterator, E = "@@iterator";
    function p(b) {
      if (b === null || typeof b != "object")
        return null;
      var P = S && b[S] || b[E];
      return typeof P == "function" ? P : null;
    }
    var v = e.__SECRET_INTERNALS_DO_NOT_USE_OR_YOU_WILL_BE_FIRED;
    function w(b) {
      {
        for (var P = arguments.length, U = new Array(P > 1 ? P - 1 : 0), q = 1; q < P; q++)
          U[q - 1] = arguments[q];
        A("error", b, U);
      }
    }
    function A(b, P, U) {
      {
        var q = v.ReactDebugCurrentFrame, se = q.getStackAddendum();
        se !== "" && (P += "%s", U = U.concat([se]));
        var ae = U.map(function(ee) {
          return String(ee);
        });
        ae.unshift("Warning: " + P), Function.prototype.apply.call(console[b], console, ae);
      }
    }
    var _ = !1, T = !1, M = !1, R = !1, k = !1, I;
    I = Symbol.for("react.module.reference");
    function $(b) {
      return !!(typeof b == "string" || typeof b == "function" || b === n || b === l || k || b === a || b === m || b === h || R || b === N || _ || T || M || typeof b == "object" && b !== null && (b.$$typeof === x || b.$$typeof === y || b.$$typeof === d || b.$$typeof === c || b.$$typeof === f || // This needs to include all possible module reference object
      // types supported by any Flight configuration anywhere since
      // we don't know which Flight build this will end up being used
      // with.
      b.$$typeof === I || b.getModuleId !== void 0));
    }
    function F(b, P, U) {
      var q = b.displayName;
      if (q)
        return q;
      var se = P.displayName || P.name || "";
      return se !== "" ? U + "(" + se + ")" : U;
    }
    function z(b) {
      return b.displayName || "Context";
    }
    function V(b) {
      if (b == null)
        return null;
      if (typeof b.tag == "number" && w("Received an unexpected object in getComponentNameFromType(). This is likely a bug in React. Please file an issue."), typeof b == "function")
        return b.displayName || b.name || null;
      if (typeof b == "string")
        return b;
      switch (b) {
        case n:
          return "Fragment";
        case r:
          return "Portal";
        case l:
          return "Profiler";
        case a:
          return "StrictMode";
        case m:
          return "Suspense";
        case h:
          return "SuspenseList";
      }
      if (typeof b == "object")
        switch (b.$$typeof) {
          case c:
            var P = b;
            return z(P) + ".Consumer";
          case d:
            var U = b;
            return z(U._context) + ".Provider";
          case f:
            return F(b, b.render, "ForwardRef");
          case y:
            var q = b.displayName || null;
            return q !== null ? q : V(b.type) || "Memo";
          case x: {
            var se = b, ae = se._payload, ee = se._init;
            try {
              return V(ee(ae));
            } catch {
              return null;
            }
          }
        }
      return null;
    }
    var G = Object.assign, L = 0, J, Q, ge, Me, ze, ne, ce;
    function mt() {
    }
    mt.__reactDisabledLog = !0;
    function pt() {
      {
        if (L === 0) {
          J = console.log, Q = console.info, ge = console.warn, Me = console.error, ze = console.group, ne = console.groupCollapsed, ce = console.groupEnd;
          var b = {
            configurable: !0,
            enumerable: !0,
            value: mt,
            writable: !0
          };
          Object.defineProperties(console, {
            info: b,
            log: b,
            warn: b,
            error: b,
            group: b,
            groupCollapsed: b,
            groupEnd: b
          });
        }
        L++;
      }
    }
    function At() {
      {
        if (L--, L === 0) {
          var b = {
            configurable: !0,
            enumerable: !0,
            writable: !0
          };
          Object.defineProperties(console, {
            log: G({}, b, {
              value: J
            }),
            info: G({}, b, {
              value: Q
            }),
            warn: G({}, b, {
              value: ge
            }),
            error: G({}, b, {
              value: Me
            }),
            group: G({}, b, {
              value: ze
            }),
            groupCollapsed: G({}, b, {
              value: ne
            }),
            groupEnd: G({}, b, {
              value: ce
            })
          });
        }
        L < 0 && w("disabledDepth fell below zero. This is a bug in React. Please file an issue.");
      }
    }
    var nt = v.ReactCurrentDispatcher, at;
    function Ce(b, P, U) {
      {
        if (at === void 0)
          try {
            throw Error();
          } catch (se) {
            var q = se.stack.trim().match(/\n( *(at )?)/);
            at = q && q[1] || "";
          }
        return `
` + at + b;
      }
    }
    var Ye = !1, it;
    {
      var er = typeof WeakMap == "function" ? WeakMap : Map;
      it = new er();
    }
    function gt(b, P) {
      if (!b || Ye)
        return "";
      {
        var U = it.get(b);
        if (U !== void 0)
          return U;
      }
      var q;
      Ye = !0;
      var se = Error.prepareStackTrace;
      Error.prepareStackTrace = void 0;
      var ae;
      ae = nt.current, nt.current = null, pt();
      try {
        if (P) {
          var ee = function() {
            throw Error();
          };
          if (Object.defineProperty(ee.prototype, "props", {
            set: function() {
              throw Error();
            }
          }), typeof Reflect == "object" && Reflect.construct) {
            try {
              Reflect.construct(ee, []);
            } catch (ke) {
              q = ke;
            }
            Reflect.construct(b, [], ee);
          } else {
            try {
              ee.call();
            } catch (ke) {
              q = ke;
            }
            b.call(ee.prototype);
          }
        } else {
          try {
            throw Error();
          } catch (ke) {
            q = ke;
          }
          b();
        }
      } catch (ke) {
        if (ke && q && typeof ke.stack == "string") {
          for (var Z = ke.stack.split(`
`), _e = q.stack.split(`
`), fe = Z.length - 1, pe = _e.length - 1; fe >= 1 && pe >= 0 && Z[fe] !== _e[pe]; )
            pe--;
          for (; fe >= 1 && pe >= 0; fe--, pe--)
            if (Z[fe] !== _e[pe]) {
              if (fe !== 1 || pe !== 1)
                do
                  if (fe--, pe--, pe < 0 || Z[fe] !== _e[pe]) {
                    var Be = `
` + Z[fe].replace(" at new ", " at ");
                    return b.displayName && Be.includes("<anonymous>") && (Be = Be.replace("<anonymous>", b.displayName)), typeof b == "function" && it.set(b, Be), Be;
                  }
                while (fe >= 1 && pe >= 0);
              break;
            }
        }
      } finally {
        Ye = !1, nt.current = ae, At(), Error.prepareStackTrace = se;
      }
      var Mt = b ? b.displayName || b.name : "", Tt = Mt ? Ce(Mt) : "";
      return typeof b == "function" && it.set(b, Tt), Tt;
    }
    function Qe(b, P, U) {
      return gt(b, !1);
    }
    function ot(b) {
      var P = b.prototype;
      return !!(P && P.isReactComponent);
    }
    function yt(b, P, U) {
      if (b == null)
        return "";
      if (typeof b == "function")
        return gt(b, ot(b));
      if (typeof b == "string")
        return Ce(b);
      switch (b) {
        case m:
          return Ce("Suspense");
        case h:
          return Ce("SuspenseList");
      }
      if (typeof b == "object")
        switch (b.$$typeof) {
          case f:
            return Qe(b.render);
          case y:
            return yt(b.type, P, U);
          case x: {
            var q = b, se = q._payload, ae = q._init;
            try {
              return yt(ae(se), P, U);
            } catch {
            }
          }
        }
      return "";
    }
    var ct = Object.prototype.hasOwnProperty, B = {}, X = v.ReactDebugCurrentFrame;
    function be(b) {
      if (b) {
        var P = b._owner, U = yt(b.type, b._source, P ? P.type : null);
        X.setExtraStackFrame(U);
      } else
        X.setExtraStackFrame(null);
    }
    function Je(b, P, U, q, se) {
      {
        var ae = Function.call.bind(ct);
        for (var ee in b)
          if (ae(b, ee)) {
            var Z = void 0;
            try {
              if (typeof b[ee] != "function") {
                var _e = Error((q || "React class") + ": " + U + " type `" + ee + "` is invalid; it must be a function, usually from the `prop-types` package, but received `" + typeof b[ee] + "`.This often happens because of typos such as `PropTypes.function` instead of `PropTypes.func`.");
                throw _e.name = "Invariant Violation", _e;
              }
              Z = b[ee](P, ee, q, U, null, "SECRET_DO_NOT_PASS_THIS_OR_YOU_WILL_BE_FIRED");
            } catch (fe) {
              Z = fe;
            }
            Z && !(Z instanceof Error) && (be(se), w("%s: type specification of %s `%s` is invalid; the type checker function must return `null` or an `Error` but returned a %s. You may have forgotten to pass an argument to the type checker creator (arrayOf, instanceOf, objectOf, oneOf, oneOfType, and shape all require an argument).", q || "React class", U, ee, typeof Z), be(null)), Z instanceof Error && !(Z.message in B) && (B[Z.message] = !0, be(se), w("Failed %s type: %s", U, Z.message), be(null));
          }
      }
    }
    var Ft = Array.isArray;
    function Re(b) {
      return Ft(b);
    }
    function tr(b) {
      {
        var P = typeof Symbol == "function" && Symbol.toStringTag, U = P && b[Symbol.toStringTag] || b.constructor.name || "Object";
        return U;
      }
    }
    function os(b) {
      try {
        return Ge(b), !1;
      } catch {
        return !0;
      }
    }
    function Ge(b) {
      return "" + b;
    }
    function Er(b) {
      if (os(b))
        return w("The provided key is an unsupported type %s. This value must be coerced to a string before before using it here.", tr(b)), Ge(b);
    }
    var u = v.ReactCurrentOwner, i = {
      key: !0,
      ref: !0,
      __self: !0,
      __source: !0
    }, o, g;
    function j(b) {
      if (ct.call(b, "ref")) {
        var P = Object.getOwnPropertyDescriptor(b, "ref").get;
        if (P && P.isReactWarning)
          return !1;
      }
      return b.ref !== void 0;
    }
    function C(b) {
      if (ct.call(b, "key")) {
        var P = Object.getOwnPropertyDescriptor(b, "key").get;
        if (P && P.isReactWarning)
          return !1;
      }
      return b.key !== void 0;
    }
    function O(b, P) {
      typeof b.ref == "string" && u.current;
    }
    function W(b, P) {
      {
        var U = function() {
          o || (o = !0, w("%s: `key` is not a prop. Trying to access it will result in `undefined` being returned. If you need to access the same value within the child component, you should pass it as a different prop. (https://reactjs.org/link/special-props)", P));
        };
        U.isReactWarning = !0, Object.defineProperty(b, "key", {
          get: U,
          configurable: !0
        });
      }
    }
    function le(b, P) {
      {
        var U = function() {
          g || (g = !0, w("%s: `ref` is not a prop. Trying to access it will result in `undefined` being returned. If you need to access the same value within the child component, you should pass it as a different prop. (https://reactjs.org/link/special-props)", P));
        };
        U.isReactWarning = !0, Object.defineProperty(b, "ref", {
          get: U,
          configurable: !0
        });
      }
    }
    var oe = function(b, P, U, q, se, ae, ee) {
      var Z = {
        // This tag allows us to uniquely identify this as a React Element
        $$typeof: t,
        // Built-in properties that belong on the element
        type: b,
        key: P,
        ref: U,
        props: ee,
        // Record the component responsible for creating this element.
        _owner: ae
      };
      return Z._store = {}, Object.defineProperty(Z._store, "validated", {
        configurable: !1,
        enumerable: !1,
        writable: !0,
        value: !1
      }), Object.defineProperty(Z, "_self", {
        configurable: !1,
        enumerable: !1,
        writable: !1,
        value: q
      }), Object.defineProperty(Z, "_source", {
        configurable: !1,
        enumerable: !1,
        writable: !1,
        value: se
      }), Object.freeze && (Object.freeze(Z.props), Object.freeze(Z)), Z;
    };
    function de(b, P, U, q, se) {
      {
        var ae, ee = {}, Z = null, _e = null;
        U !== void 0 && (Er(U), Z = "" + U), C(P) && (Er(P.key), Z = "" + P.key), j(P) && (_e = P.ref, O(P, se));
        for (ae in P)
          ct.call(P, ae) && !i.hasOwnProperty(ae) && (ee[ae] = P[ae]);
        if (b && b.defaultProps) {
          var fe = b.defaultProps;
          for (ae in fe)
            ee[ae] === void 0 && (ee[ae] = fe[ae]);
        }
        if (Z || _e) {
          var pe = typeof b == "function" ? b.displayName || b.name || "Unknown" : b;
          Z && W(ee, pe), _e && le(ee, pe);
        }
        return oe(b, Z, _e, se, q, u.current, ee);
      }
    }
    var re = v.ReactCurrentOwner, hn = v.ReactDebugCurrentFrame;
    function Pt(b) {
      if (b) {
        var P = b._owner, U = yt(b.type, b._source, P ? P.type : null);
        hn.setExtraStackFrame(U);
      } else
        hn.setExtraStackFrame(null);
    }
    var cs;
    cs = !1;
    function ls(b) {
      return typeof b == "object" && b !== null && b.$$typeof === t;
    }
    function mn() {
      {
        if (re.current) {
          var b = V(re.current.type);
          if (b)
            return `

Check the render method of \`` + b + "`.";
        }
        return "";
      }
    }
    function yi(b) {
      return "";
    }
    var pn = {};
    function xi(b) {
      {
        var P = mn();
        if (!P) {
          var U = typeof b == "string" ? b : b.displayName || b.name;
          U && (P = `

Check the top-level render call using <` + U + ">.");
        }
        return P;
      }
    }
    function gn(b, P) {
      {
        if (!b._store || b._store.validated || b.key != null)
          return;
        b._store.validated = !0;
        var U = xi(P);
        if (pn[U])
          return;
        pn[U] = !0;
        var q = "";
        b && b._owner && b._owner !== re.current && (q = " It was passed a child from " + V(b._owner.type) + "."), Pt(b), w('Each child in a list should have a unique "key" prop.%s%s See https://reactjs.org/link/warning-keys for more information.', U, q), Pt(null);
      }
    }
    function yn(b, P) {
      {
        if (typeof b != "object")
          return;
        if (Re(b))
          for (var U = 0; U < b.length; U++) {
            var q = b[U];
            ls(q) && gn(q, P);
          }
        else if (ls(b))
          b._store && (b._store.validated = !0);
        else if (b) {
          var se = p(b);
          if (typeof se == "function" && se !== b.entries)
            for (var ae = se.call(b), ee; !(ee = ae.next()).done; )
              ls(ee.value) && gn(ee.value, P);
        }
      }
    }
    function bi(b) {
      {
        var P = b.type;
        if (P == null || typeof P == "string")
          return;
        var U;
        if (typeof P == "function")
          U = P.propTypes;
        else if (typeof P == "object" && (P.$$typeof === f || // Note: Memo only checks outer props here.
        // Inner props are checked in the reconciler.
        P.$$typeof === y))
          U = P.propTypes;
        else
          return;
        if (U) {
          var q = V(P);
          Je(U, b.props, "prop", q, b);
        } else if (P.PropTypes !== void 0 && !cs) {
          cs = !0;
          var se = V(P);
          w("Component %s declared `PropTypes` instead of `propTypes`. Did you misspell the property assignment?", se || "Unknown");
        }
        typeof P.getDefaultProps == "function" && !P.getDefaultProps.isReactClassApproved && w("getDefaultProps is only used on classic React.createClass definitions. Use a static property named `defaultProps` instead.");
      }
    }
    function vi(b) {
      {
        for (var P = Object.keys(b.props), U = 0; U < P.length; U++) {
          var q = P[U];
          if (q !== "children" && q !== "key") {
            Pt(b), w("Invalid prop `%s` supplied to `React.Fragment`. React.Fragment can only have `key` and `children` props.", q), Pt(null);
            break;
          }
        }
        b.ref !== null && (Pt(b), w("Invalid attribute `ref` supplied to `React.Fragment`."), Pt(null));
      }
    }
    var xn = {};
    function bn(b, P, U, q, se, ae) {
      {
        var ee = $(b);
        if (!ee) {
          var Z = "";
          (b === void 0 || typeof b == "object" && b !== null && Object.keys(b).length === 0) && (Z += " You likely forgot to export your component from the file it's defined in, or you might have mixed up default and named imports.");
          var _e = yi();
          _e ? Z += _e : Z += mn();
          var fe;
          b === null ? fe = "null" : Re(b) ? fe = "array" : b !== void 0 && b.$$typeof === t ? (fe = "<" + (V(b.type) || "Unknown") + " />", Z = " Did you accidentally export a JSX literal instead of a component?") : fe = typeof b, w("React.jsx: type is invalid -- expected a string (for built-in components) or a class/function (for composite components) but got: %s.%s", fe, Z);
        }
        var pe = de(b, P, U, se, ae);
        if (pe == null)
          return pe;
        if (ee) {
          var Be = P.children;
          if (Be !== void 0)
            if (q)
              if (Re(Be)) {
                for (var Mt = 0; Mt < Be.length; Mt++)
                  yn(Be[Mt], b);
                Object.freeze && Object.freeze(Be);
              } else
                w("React.jsx: Static children should always be an array. You are likely explicitly calling React.jsxs or React.jsxDEV. Use the Babel transform instead.");
            else
              yn(Be, b);
        }
        if (ct.call(P, "key")) {
          var Tt = V(b), ke = Object.keys(P).filter(function(_i) {
            return _i !== "key";
          }), ds = ke.length > 0 ? "{key: someKey, " + ke.join(": ..., ") + ": ...}" : "{key: someKey}";
          if (!xn[Tt + ds]) {
            var Si = ke.length > 0 ? "{" + ke.join(": ..., ") + ": ...}" : "{}";
            w(`A props object containing a "key" prop is being spread into JSX:
  let props = %s;
  <%s {...props} />
React keys must be passed directly to JSX without using spread:
  let props = %s;
  <%s key={someKey} {...props} />`, ds, Tt, Si, Tt), xn[Tt + ds] = !0;
          }
        }
        return b === n ? vi(pe) : bi(pe), pe;
      }
    }
    function wi(b, P, U) {
      return bn(b, P, U, !0);
    }
    function Ei(b, P, U) {
      return bn(b, P, U, !1);
    }
    var Ni = Ei, ji = wi;
    sr.Fragment = n, sr.jsx = Ni, sr.jsxs = ji;
  }()), sr;
}
process.env.NODE_ENV === "production" ? _s.exports = Di() : _s.exports = Fi();
var s = _s.exports;
function ve(e) {
  return `Minified Redux error #${e}; visit https://redux.js.org/Errors?code=${e} for the full message or use the non-minified dev environment for full errors. `;
}
var Pi = typeof Symbol == "function" && Symbol.observable || "@@observable", En = Pi, us = () => Math.random().toString(36).substring(7).split("").join("."), Mi = {
  INIT: `@@redux/INIT${/* @__PURE__ */ us()}`,
  REPLACE: `@@redux/REPLACE${/* @__PURE__ */ us()}`,
  PROBE_UNKNOWN_ACTION: () => `@@redux/PROBE_UNKNOWN_ACTION${us()}`
}, It = Mi;
function pr(e) {
  if (typeof e != "object" || e === null)
    return !1;
  let t = e;
  for (; Object.getPrototypeOf(t) !== null; )
    t = Object.getPrototypeOf(t);
  return Object.getPrototypeOf(e) === t || Object.getPrototypeOf(e) === null;
}
function Bi(e) {
  if (e === void 0)
    return "undefined";
  if (e === null)
    return "null";
  const t = typeof e;
  switch (t) {
    case "boolean":
    case "string":
    case "number":
    case "symbol":
    case "function":
      return t;
  }
  if (Array.isArray(e))
    return "array";
  if (Ui(e))
    return "date";
  if ($i(e))
    return "error";
  const r = Li(e);
  switch (r) {
    case "Symbol":
    case "Promise":
    case "WeakMap":
    case "WeakSet":
    case "Map":
    case "Set":
      return r;
  }
  return Object.prototype.toString.call(e).slice(8, -1).toLowerCase().replace(/\s/g, "");
}
function Li(e) {
  return typeof e.constructor == "function" ? e.constructor.name : null;
}
function $i(e) {
  return e instanceof Error || typeof e.message == "string" && e.constructor && typeof e.constructor.stackTraceLimit == "number";
}
function Ui(e) {
  return e instanceof Date ? !0 : typeof e.toDateString == "function" && typeof e.getDate == "function" && typeof e.setDate == "function";
}
function vt(e) {
  let t = typeof e;
  return process.env.NODE_ENV !== "production" && (t = Bi(e)), t;
}
function ma(e, t, r) {
  if (typeof e != "function")
    throw new Error(process.env.NODE_ENV === "production" ? ve(2) : `Expected the root reducer to be a function. Instead, received: '${vt(e)}'`);
  if (typeof t == "function" && typeof r == "function" || typeof r == "function" && typeof arguments[3] == "function")
    throw new Error(process.env.NODE_ENV === "production" ? ve(0) : "It looks like you are passing several store enhancers to createStore(). This is not supported. Instead, compose them together to a single function. See https://redux.js.org/tutorials/fundamentals/part-4-store#creating-a-store-with-enhancers for an example.");
  if (typeof t == "function" && typeof r > "u" && (r = t, t = void 0), typeof r < "u") {
    if (typeof r != "function")
      throw new Error(process.env.NODE_ENV === "production" ? ve(1) : `Expected the enhancer to be a function. Instead, received: '${vt(r)}'`);
    return r(ma)(e, t);
  }
  let n = e, a = t, l = /* @__PURE__ */ new Map(), d = l, c = 0, f = !1;
  function m() {
    d === l && (d = /* @__PURE__ */ new Map(), l.forEach((p, v) => {
      d.set(v, p);
    }));
  }
  function h() {
    if (f)
      throw new Error(process.env.NODE_ENV === "production" ? ve(3) : "You may not call store.getState() while the reducer is executing. The reducer has already received the state as an argument. Pass it down from the top reducer instead of reading it from the store.");
    return a;
  }
  function y(p) {
    if (typeof p != "function")
      throw new Error(process.env.NODE_ENV === "production" ? ve(4) : `Expected the listener to be a function. Instead, received: '${vt(p)}'`);
    if (f)
      throw new Error(process.env.NODE_ENV === "production" ? ve(5) : "You may not call store.subscribe() while the reducer is executing. If you would like to be notified after the store has been updated, subscribe from a component and invoke store.getState() in the callback to access the latest state. See https://redux.js.org/api/store#subscribelistener for more details.");
    let v = !0;
    m();
    const w = c++;
    return d.set(w, p), function() {
      if (v) {
        if (f)
          throw new Error(process.env.NODE_ENV === "production" ? ve(6) : "You may not unsubscribe from a store listener while the reducer is executing. See https://redux.js.org/api/store#subscribelistener for more details.");
        v = !1, m(), d.delete(w), l = null;
      }
    };
  }
  function x(p) {
    if (!pr(p))
      throw new Error(process.env.NODE_ENV === "production" ? ve(7) : `Actions must be plain objects. Instead, the actual type was: '${vt(p)}'. You may need to add middleware to your store setup to handle dispatching other values, such as 'redux-thunk' to handle dispatching functions. See https://redux.js.org/tutorials/fundamentals/part-4-store#middleware and https://redux.js.org/tutorials/fundamentals/part-6-async-logic#using-the-redux-thunk-middleware for examples.`);
    if (typeof p.type > "u")
      throw new Error(process.env.NODE_ENV === "production" ? ve(8) : 'Actions may not have an undefined "type" property. You may have misspelled an action type string constant.');
    if (typeof p.type != "string")
      throw new Error(process.env.NODE_ENV === "production" ? ve(17) : `Action "type" property must be a string. Instead, the actual type was: '${vt(p.type)}'. Value was: '${p.type}' (stringified)`);
    if (f)
      throw new Error(process.env.NODE_ENV === "production" ? ve(9) : "Reducers may not dispatch actions.");
    try {
      f = !0, a = n(a, p);
    } finally {
      f = !1;
    }
    return (l = d).forEach((w) => {
      w();
    }), p;
  }
  function N(p) {
    if (typeof p != "function")
      throw new Error(process.env.NODE_ENV === "production" ? ve(10) : `Expected the nextReducer to be a function. Instead, received: '${vt(p)}`);
    n = p, x({
      type: It.REPLACE
    });
  }
  function S() {
    const p = y;
    return {
      /**
       * The minimal observable subscription method.
       * @param observer Any object that can be used as an observer.
       * The observer object should have a `next` method.
       * @returns An object with an `unsubscribe` method that can
       * be used to unsubscribe the observable from the store, and prevent further
       * emission of values from the observable.
       */
      subscribe(v) {
        if (typeof v != "object" || v === null)
          throw new Error(process.env.NODE_ENV === "production" ? ve(11) : `Expected the observer to be an object. Instead, received: '${vt(v)}'`);
        function w() {
          const _ = v;
          _.next && _.next(h());
        }
        return w(), {
          unsubscribe: p(w)
        };
      },
      [En]() {
        return this;
      }
    };
  }
  return x({
    type: It.INIT
  }), {
    dispatch: x,
    subscribe: y,
    getState: h,
    replaceReducer: N,
    [En]: S
  };
}
function Nn(e) {
  typeof console < "u" && typeof console.error == "function" && console.error(e);
  try {
    throw new Error(e);
  } catch {
  }
}
function Ki(e, t, r, n) {
  const a = Object.keys(t), l = r && r.type === It.INIT ? "preloadedState argument passed to createStore" : "previous state received by the reducer";
  if (a.length === 0)
    return "Store does not have a valid reducer. Make sure the argument passed to combineReducers is an object whose values are reducers.";
  if (!pr(e))
    return `The ${l} has unexpected type of "${vt(e)}". Expected argument to be an object with the following keys: "${a.join('", "')}"`;
  const d = Object.keys(e).filter((c) => !t.hasOwnProperty(c) && !n[c]);
  if (d.forEach((c) => {
    n[c] = !0;
  }), !(r && r.type === It.REPLACE) && d.length > 0)
    return `Unexpected ${d.length > 1 ? "keys" : "key"} "${d.join('", "')}" found in ${l}. Expected to find one of the known reducer keys instead: "${a.join('", "')}". Unexpected keys will be ignored.`;
}
function Vi(e) {
  Object.keys(e).forEach((t) => {
    const r = e[t];
    if (typeof r(void 0, {
      type: It.INIT
    }) > "u")
      throw new Error(process.env.NODE_ENV === "production" ? ve(12) : `The slice reducer for key "${t}" returned undefined during initialization. If the state passed to the reducer is undefined, you must explicitly return the initial state. The initial state may not be undefined. If you don't want to set a value for this reducer, you can use null instead of undefined.`);
    if (typeof r(void 0, {
      type: It.PROBE_UNKNOWN_ACTION()
    }) > "u")
      throw new Error(process.env.NODE_ENV === "production" ? ve(13) : `The slice reducer for key "${t}" returned undefined when probed with a random type. Don't try to handle '${It.INIT}' or other actions in "redux/*" namespace. They are considered private. Instead, you must return the current state for any unknown actions, unless it is undefined, in which case you must return the initial state, regardless of the action type. The initial state may not be undefined, but can be null.`);
  });
}
function Hi(e) {
  const t = Object.keys(e), r = {};
  for (let d = 0; d < t.length; d++) {
    const c = t[d];
    process.env.NODE_ENV !== "production" && typeof e[c] > "u" && Nn(`No reducer provided for key "${c}"`), typeof e[c] == "function" && (r[c] = e[c]);
  }
  const n = Object.keys(r);
  let a;
  process.env.NODE_ENV !== "production" && (a = {});
  let l;
  try {
    Vi(r);
  } catch (d) {
    l = d;
  }
  return function(c = {}, f) {
    if (l)
      throw l;
    if (process.env.NODE_ENV !== "production") {
      const y = Ki(c, r, f, a);
      y && Nn(y);
    }
    let m = !1;
    const h = {};
    for (let y = 0; y < n.length; y++) {
      const x = n[y], N = r[x], S = c[x], E = N(S, f);
      if (typeof E > "u") {
        const p = f && f.type;
        throw new Error(process.env.NODE_ENV === "production" ? ve(14) : `When called with an action of type ${p ? `"${String(p)}"` : "(unknown type)"}, the slice reducer for key "${x}" returned undefined. To ignore an action, you must explicitly return the previous state. If you want this reducer to hold no value, you can return null instead of undefined.`);
      }
      h[x] = E, m = m || E !== S;
    }
    return m = m || n.length !== Object.keys(c).length, m ? h : c;
  };
}
function Br(...e) {
  return e.length === 0 ? (t) => t : e.length === 1 ? e[0] : e.reduce((t, r) => (...n) => t(r(...n)));
}
function zi(...e) {
  return (t) => (r, n) => {
    const a = t(r, n);
    let l = () => {
      throw new Error(process.env.NODE_ENV === "production" ? ve(15) : "Dispatching while constructing your middleware is not allowed. Other middleware would not be applied to this dispatch.");
    };
    const d = {
      getState: a.getState,
      dispatch: (f, ...m) => l(f, ...m)
    }, c = e.map((f) => f(d));
    return l = Br(...c)(a.dispatch), {
      ...a,
      dispatch: l
    };
  };
}
function pa(e) {
  return pr(e) && "type" in e && typeof e.type == "string";
}
var ga = Symbol.for("immer-nothing"), jn = Symbol.for("immer-draftable"), Te = Symbol.for("immer-state"), Gi = process.env.NODE_ENV !== "production" ? [
  // All error codes, starting by 0:
  function(e) {
    return `The plugin for '${e}' has not been loaded into Immer. To enable the plugin, import and call \`enable${e}()\` when initializing your application.`;
  },
  function(e) {
    return `produce can only be called on things that are draftable: plain objects, arrays, Map, Set or classes that are marked with '[immerable]: true'. Got '${e}'`;
  },
  "This object has been frozen and should not be mutated",
  function(e) {
    return "Cannot use a proxy that has been revoked. Did you pass an object from inside an immer function to an async process? " + e;
  },
  "An immer producer returned a new value *and* modified its draft. Either return a new value *or* modify the draft.",
  "Immer forbids circular references",
  "The first or second argument to `produce` must be a function",
  "The third argument to `produce` must be a function or undefined",
  "First argument to `createDraft` must be a plain object, an array, or an immerable object",
  "First argument to `finishDraft` must be a draft returned by `createDraft`",
  function(e) {
    return `'current' expects a draft, got: ${e}`;
  },
  "Object.defineProperty() cannot be used on an Immer draft",
  "Object.setPrototypeOf() cannot be used on an Immer draft",
  "Immer only supports deleting array indices",
  "Immer only supports setting array indices and the 'length' property",
  function(e) {
    return `'original' expects a draft, got: ${e}`;
  }
  // Note: if more errors are added, the errorOffset in Patches.ts should be increased
  // See Patches.ts for additional errors
] : [];
function Fe(e, ...t) {
  if (process.env.NODE_ENV !== "production") {
    const r = Gi[e], n = Rt(r) ? r.apply(null, t) : r;
    throw new Error(`[Immer] ${n}`);
  }
  throw new Error(
    `[Immer] minified error nr: ${e}. Full error at: https://bit.ly/3cXEKWf`
  );
}
var Pe = Object, Gt = Pe.getPrototypeOf, Lr = "constructor", qr = "prototype", As = "configurable", $r = "enumerable", Dr = "writable", ur = "value", ht = (e) => !!e && !!e[Te];
function qe(e) {
  var t;
  return e ? ya(e) || Wr(e) || !!e[jn] || !!((t = e[Lr]) != null && t[jn]) || Yr(e) || Qr(e) : !1;
}
var qi = Pe[qr][Lr].toString(), Sn = /* @__PURE__ */ new WeakMap();
function ya(e) {
  if (!e || !Vs(e))
    return !1;
  const t = Gt(e);
  if (t === null || t === Pe[qr])
    return !0;
  const r = Pe.hasOwnProperty.call(t, Lr) && t[Lr];
  if (r === Object)
    return !0;
  if (!Rt(r))
    return !1;
  let n = Sn.get(r);
  return n === void 0 && (n = Function.toString.call(r), Sn.set(r, n)), n === qi;
}
function gr(e, t, r = !0) {
  yr(e) === 0 ? (r ? Reflect.ownKeys(e) : Pe.keys(e)).forEach((a) => {
    t(a, e[a], e);
  }) : e.forEach((n, a) => t(a, n, e));
}
function yr(e) {
  const t = e[Te];
  return t ? t.type_ : Wr(e) ? 1 : Yr(e) ? 2 : Qr(e) ? 3 : 0;
}
var _n = (e, t, r = yr(e)) => r === 2 ? e.has(t) : Pe[qr].hasOwnProperty.call(e, t), Ts = (e, t, r = yr(e)) => (
  // @ts-ignore
  r === 2 ? e.get(t) : e[t]
), Ur = (e, t, r, n = yr(e)) => {
  n === 2 ? e.set(t, r) : n === 3 ? e.add(r) : e[t] = r;
};
function Wi(e, t) {
  return e === t ? e !== 0 || 1 / e === 1 / t : e !== e && t !== t;
}
var Wr = Array.isArray, Yr = (e) => e instanceof Map, Qr = (e) => e instanceof Set, Vs = (e) => typeof e == "object", Rt = (e) => typeof e == "function", fs = (e) => typeof e == "boolean", ft = (e) => e.copy_ || e.base_, Hs = (e) => e.modified_ ? e.copy_ : e.base_;
function Cs(e, t) {
  if (Yr(e))
    return new Map(e);
  if (Qr(e))
    return new Set(e);
  if (Wr(e))
    return Array[qr].slice.call(e);
  const r = ya(e);
  if (t === !0 || t === "class_only" && !r) {
    const n = Pe.getOwnPropertyDescriptors(e);
    delete n[Te];
    let a = Reflect.ownKeys(n);
    for (let l = 0; l < a.length; l++) {
      const d = a[l], c = n[d];
      c[Dr] === !1 && (c[Dr] = !0, c[As] = !0), (c.get || c.set) && (n[d] = {
        [As]: !0,
        [Dr]: !0,
        // could live with !!desc.set as well here...
        [$r]: c[$r],
        [ur]: e[d]
      });
    }
    return Pe.create(Gt(e), n);
  } else {
    const n = Gt(e);
    if (n !== null && r)
      return { ...e };
    const a = Pe.create(n);
    return Pe.assign(a, e);
  }
}
function zs(e, t = !1) {
  return Jr(e) || ht(e) || !qe(e) || (yr(e) > 1 && Pe.defineProperties(e, {
    set: Nr,
    add: Nr,
    clear: Nr,
    delete: Nr
  }), Pe.freeze(e), t && gr(
    e,
    (r, n) => {
      zs(n, !0);
    },
    !1
  )), e;
}
function Yi() {
  Fe(2);
}
var Nr = {
  [ur]: Yi
};
function Jr(e) {
  return e === null || !Vs(e) ? !0 : Pe.isFrozen(e);
}
var Kr = "MapSet", Rs = "Patches", xa = {};
function qt(e) {
  const t = xa[e];
  return t || Fe(0, e), t;
}
var Qi = (e) => !!xa[e], fr, ba = () => fr, Ji = (e, t) => ({
  drafts_: [],
  parent_: e,
  immer_: t,
  // Whenever the modified draft contains a draft from another scope, we
  // need to prevent auto-freezing so the unowned draft can be finalized.
  canAutoFreeze_: !0,
  unfinalizedDrafts_: 0,
  handledSet_: /* @__PURE__ */ new Set(),
  processedForPatches_: /* @__PURE__ */ new Set(),
  mapSetPlugin_: Qi(Kr) ? qt(Kr) : void 0
});
function An(e, t) {
  t && (e.patchPlugin_ = qt(Rs), e.patches_ = [], e.inversePatches_ = [], e.patchListener_ = t);
}
function ks(e) {
  Is(e), e.drafts_.forEach(Zi), e.drafts_ = null;
}
function Is(e) {
  e === fr && (fr = e.parent_);
}
var Tn = (e) => fr = Ji(fr, e);
function Zi(e) {
  const t = e[Te];
  t.type_ === 0 || t.type_ === 1 ? t.revoke_() : t.revoked_ = !0;
}
function Cn(e, t) {
  t.unfinalizedDrafts_ = t.drafts_.length;
  const r = t.drafts_[0];
  if (e !== void 0 && e !== r) {
    r[Te].modified_ && (ks(t), Fe(4)), qe(e) && (e = Rn(t, e));
    const { patchPlugin_: a } = t;
    a && a.generateReplacementPatches_(
      r[Te].base_,
      e,
      t
    );
  } else
    e = Rn(t, r);
  return Xi(t, e, !0), ks(t), t.patches_ && t.patchListener_(t.patches_, t.inversePatches_), e !== ga ? e : void 0;
}
function Rn(e, t) {
  if (Jr(t))
    return t;
  const r = t[Te];
  if (!r)
    return Gs(t, e.handledSet_, e);
  if (!Zr(r, e))
    return t;
  if (!r.modified_)
    return r.base_;
  if (!r.finalized_) {
    const { callbacks_: n } = r;
    if (n)
      for (; n.length > 0; )
        n.pop()(e);
    Ea(r, e);
  }
  return r.copy_;
}
function Xi(e, t, r = !1) {
  !e.parent_ && e.immer_.autoFreeze_ && e.canAutoFreeze_ && zs(t, r);
}
function va(e) {
  e.finalized_ = !0, e.scope_.unfinalizedDrafts_--;
}
var Zr = (e, t) => e.scope_ === t, eo = [];
function wa(e, t, r, n) {
  const a = ft(e), l = e.type_;
  if (n !== void 0 && Ts(a, n, l) === t) {
    Ur(a, n, r, l);
    return;
  }
  if (!e.draftLocations_) {
    const c = e.draftLocations_ = /* @__PURE__ */ new Map();
    gr(a, (f, m) => {
      if (ht(m)) {
        const h = c.get(m) || [];
        h.push(f), c.set(m, h);
      }
    });
  }
  const d = e.draftLocations_.get(t) ?? eo;
  for (const c of d)
    Ur(a, c, r, l);
}
function to(e, t, r) {
  e.callbacks_.push(function(a) {
    var c;
    const l = t;
    if (!l || !Zr(l, a))
      return;
    (c = a.mapSetPlugin_) == null || c.fixSetContents(l);
    const d = Hs(l);
    wa(e, l.draft_ ?? l, d, r), Ea(l, a);
  });
}
function Ea(e, t) {
  var n;
  if (e.modified_ && !e.finalized_ && (e.type_ === 3 || (((n = e.assigned_) == null ? void 0 : n.size) ?? 0) > 0)) {
    const { patchPlugin_: a } = t;
    if (a) {
      const l = a.getPath(e);
      l && a.generatePatches_(e, l, t);
    }
    va(e);
  }
}
function ro(e, t, r) {
  const { scope_: n } = e;
  if (ht(r)) {
    const a = r[Te];
    Zr(a, n) && a.callbacks_.push(function() {
      Fr(e);
      const d = Hs(a);
      wa(e, r, d, t);
    });
  } else qe(r) && e.callbacks_.push(function() {
    const l = ft(e);
    Ts(l, t, e.type_) === r && n.drafts_.length > 1 && (e.assigned_.get(t) ?? !1) === !0 && e.copy_ && Gs(
      Ts(e.copy_, t, e.type_),
      n.handledSet_,
      n
    );
  });
}
function Gs(e, t, r) {
  return !r.immer_.autoFreeze_ && r.unfinalizedDrafts_ < 1 || ht(e) || t.has(e) || !qe(e) || Jr(e) || (t.add(e), gr(e, (n, a) => {
    if (ht(a)) {
      const l = a[Te];
      if (Zr(l, r)) {
        const d = Hs(l);
        Ur(e, n, d, e.type_), va(l);
      }
    } else qe(a) && Gs(a, t, r);
  })), e;
}
function so(e, t) {
  const r = Wr(e), n = {
    type_: r ? 1 : 0,
    // Track which produce call this is associated with.
    scope_: t ? t.scope_ : ba(),
    // True for both shallow and deep changes.
    modified_: !1,
    // Used during finalization.
    finalized_: !1,
    // Track which properties have been assigned (true) or deleted (false).
    // actually instantiated in `prepareCopy()`
    assigned_: void 0,
    // The parent draft state.
    parent_: t,
    // The base state.
    base_: e,
    // The base proxy.
    draft_: null,
    // set below
    // The base copy with any updated values.
    copy_: null,
    // Called by the `produce` function.
    revoke_: null,
    isManual_: !1,
    // `callbacks` actually gets assigned in `createProxy`
    callbacks_: void 0
  };
  let a = n, l = qs;
  r && (a = [n], l = hr);
  const { revoke: d, proxy: c } = Proxy.revocable(a, l);
  return n.draft_ = c, n.revoke_ = d, [c, n];
}
var qs = {
  get(e, t) {
    if (t === Te)
      return e;
    const r = ft(e);
    if (!_n(r, t, e.type_))
      return no(e, r, t);
    const n = r[t];
    if (e.finalized_ || !qe(n))
      return n;
    if (n === hs(e.base_, t)) {
      Fr(e);
      const a = e.type_ === 1 ? +t : t, l = Ds(e.scope_, n, e, a);
      return e.copy_[a] = l;
    }
    return n;
  },
  has(e, t) {
    return t in ft(e);
  },
  ownKeys(e) {
    return Reflect.ownKeys(ft(e));
  },
  set(e, t, r) {
    const n = Na(ft(e), t);
    if (n != null && n.set)
      return n.set.call(e.draft_, r), !0;
    if (!e.modified_) {
      const a = hs(ft(e), t), l = a == null ? void 0 : a[Te];
      if (l && l.base_ === r)
        return e.copy_[t] = r, e.assigned_.set(t, !1), !0;
      if (Wi(r, a) && (r !== void 0 || _n(e.base_, t, e.type_)))
        return !0;
      Fr(e), Os(e);
    }
    return e.copy_[t] === r && // special case: handle new props with value 'undefined'
    (r !== void 0 || t in e.copy_) || // special case: NaN
    Number.isNaN(r) && Number.isNaN(e.copy_[t]) || (e.copy_[t] = r, e.assigned_.set(t, !0), ro(e, t, r)), !0;
  },
  deleteProperty(e, t) {
    return Fr(e), hs(e.base_, t) !== void 0 || t in e.base_ ? (e.assigned_.set(t, !1), Os(e)) : e.assigned_.delete(t), e.copy_ && delete e.copy_[t], !0;
  },
  // Note: We never coerce `desc.value` into an Immer draft, because we can't make
  // the same guarantee in ES5 mode.
  getOwnPropertyDescriptor(e, t) {
    const r = ft(e), n = Reflect.getOwnPropertyDescriptor(r, t);
    return n && {
      [Dr]: !0,
      [As]: e.type_ !== 1 || t !== "length",
      [$r]: n[$r],
      [ur]: r[t]
    };
  },
  defineProperty() {
    Fe(11);
  },
  getPrototypeOf(e) {
    return Gt(e.base_);
  },
  setPrototypeOf() {
    Fe(12);
  }
}, hr = {};
gr(qs, (e, t) => {
  hr[e] = function() {
    const r = arguments;
    return r[0] = r[0][0], t.apply(this, r);
  };
});
hr.deleteProperty = function(e, t) {
  return process.env.NODE_ENV !== "production" && isNaN(parseInt(t)) && Fe(13), hr.set.call(this, e, t, void 0);
};
hr.set = function(e, t, r) {
  return process.env.NODE_ENV !== "production" && t !== "length" && isNaN(parseInt(t)) && Fe(14), qs.set.call(this, e[0], t, r, e[0]);
};
function hs(e, t) {
  const r = e[Te];
  return (r ? ft(r) : e)[t];
}
function no(e, t, r) {
  var a;
  const n = Na(t, r);
  return n ? ur in n ? n[ur] : (
    // This is a very special case, if the prop is a getter defined by the
    // prototype, we should invoke it with the draft as context!
    (a = n.get) == null ? void 0 : a.call(e.draft_)
  ) : void 0;
}
function Na(e, t) {
  if (!(t in e))
    return;
  let r = Gt(e);
  for (; r; ) {
    const n = Object.getOwnPropertyDescriptor(r, t);
    if (n)
      return n;
    r = Gt(r);
  }
}
function Os(e) {
  e.modified_ || (e.modified_ = !0, e.parent_ && Os(e.parent_));
}
function Fr(e) {
  e.copy_ || (e.assigned_ = /* @__PURE__ */ new Map(), e.copy_ = Cs(
    e.base_,
    e.scope_.immer_.useStrictShallowCopy_
  ));
}
var ao = class {
  constructor(e) {
    this.autoFreeze_ = !0, this.useStrictShallowCopy_ = !1, this.useStrictIteration_ = !1, this.produce = (t, r, n) => {
      if (Rt(t) && !Rt(r)) {
        const l = r;
        r = t;
        const d = this;
        return function(f = l, ...m) {
          return d.produce(f, (h) => r.call(this, h, ...m));
        };
      }
      Rt(r) || Fe(6), n !== void 0 && !Rt(n) && Fe(7);
      let a;
      if (qe(t)) {
        const l = Tn(this), d = Ds(l, t, void 0);
        let c = !0;
        try {
          a = r(d), c = !1;
        } finally {
          c ? ks(l) : Is(l);
        }
        return An(l, n), Cn(a, l);
      } else if (!t || !Vs(t)) {
        if (a = r(t), a === void 0 && (a = t), a === ga && (a = void 0), this.autoFreeze_ && zs(a, !0), n) {
          const l = [], d = [];
          qt(Rs).generateReplacementPatches_(t, a, {
            patches_: l,
            inversePatches_: d
          }), n(l, d);
        }
        return a;
      } else
        Fe(1, t);
    }, this.produceWithPatches = (t, r) => {
      if (Rt(t))
        return (d, ...c) => this.produceWithPatches(d, (f) => t(f, ...c));
      let n, a;
      return [this.produce(t, r, (d, c) => {
        n = d, a = c;
      }), n, a];
    }, fs(e == null ? void 0 : e.autoFreeze) && this.setAutoFreeze(e.autoFreeze), fs(e == null ? void 0 : e.useStrictShallowCopy) && this.setUseStrictShallowCopy(e.useStrictShallowCopy), fs(e == null ? void 0 : e.useStrictIteration) && this.setUseStrictIteration(e.useStrictIteration);
  }
  createDraft(e) {
    qe(e) || Fe(8), ht(e) && (e = io(e));
    const t = Tn(this), r = Ds(t, e, void 0);
    return r[Te].isManual_ = !0, Is(t), r;
  }
  finishDraft(e, t) {
    const r = e && e[Te];
    (!r || !r.isManual_) && Fe(9);
    const { scope_: n } = r;
    return An(n, t), Cn(void 0, n);
  }
  /**
   * Pass true to automatically freeze all copies created by Immer.
   *
   * By default, auto-freezing is enabled.
   */
  setAutoFreeze(e) {
    this.autoFreeze_ = e;
  }
  /**
   * Pass true to enable strict shallow copy.
   *
   * By default, immer does not copy the object descriptors such as getter, setter and non-enumrable properties.
   */
  setUseStrictShallowCopy(e) {
    this.useStrictShallowCopy_ = e;
  }
  /**
   * Pass false to use faster iteration that skips non-enumerable properties
   * but still handles symbols for compatibility.
   *
   * By default, strict iteration is enabled (includes all own properties).
   */
  setUseStrictIteration(e) {
    this.useStrictIteration_ = e;
  }
  shouldUseStrictIteration() {
    return this.useStrictIteration_;
  }
  applyPatches(e, t) {
    let r;
    for (r = t.length - 1; r >= 0; r--) {
      const a = t[r];
      if (a.path.length === 0 && a.op === "replace") {
        e = a.value;
        break;
      }
    }
    r > -1 && (t = t.slice(r + 1));
    const n = qt(Rs).applyPatches_;
    return ht(e) ? n(e, t) : this.produce(
      e,
      (a) => n(a, t)
    );
  }
};
function Ds(e, t, r, n) {
  const [a, l] = Yr(t) ? qt(Kr).proxyMap_(t, r) : Qr(t) ? qt(Kr).proxySet_(t, r) : so(t, r);
  return ((r == null ? void 0 : r.scope_) ?? ba()).drafts_.push(a), l.callbacks_ = (r == null ? void 0 : r.callbacks_) ?? [], l.key_ = n, r && n !== void 0 ? to(r, l, n) : l.callbacks_.push(function(f) {
    var h;
    (h = f.mapSetPlugin_) == null || h.fixSetContents(l);
    const { patchPlugin_: m } = f;
    l.modified_ && m && m.generatePatches_(l, [], f);
  }), a;
}
function io(e) {
  return ht(e) || Fe(10, e), ja(e);
}
function ja(e) {
  if (!qe(e) || Jr(e))
    return e;
  const t = e[Te];
  let r, n = !0;
  if (t) {
    if (!t.modified_)
      return t.base_;
    t.finalized_ = !0, r = Cs(e, t.scope_.immer_.useStrictShallowCopy_), n = t.scope_.immer_.shouldUseStrictIteration();
  } else
    r = Cs(e, !0);
  return gr(
    r,
    (a, l) => {
      Ur(r, a, ja(l));
    },
    n
  ), t && (t.finalized_ = !1), r;
}
var oo = new ao(), Sa = oo.produce, co = (e, t, r) => {
  if (t.length === 1 && t[0] === r) {
    let n = !1;
    try {
      const a = {};
      e(a) === a && (n = !0);
    } catch {
    }
    if (n) {
      let a;
      try {
        throw new Error();
      } catch (l) {
        ({ stack: a } = l);
      }
      console.warn(
        `The result function returned its own inputs without modification. e.g
\`createSelector([state => state.todos], todos => todos)\`
This could lead to inefficient memoization and unnecessary re-renders.
Ensure transformation logic is in the result function, and extraction logic is in the input selectors.`,
        { stack: a }
      );
    }
  }
}, lo = (e, t, r) => {
  const { memoize: n, memoizeOptions: a } = t, { inputSelectorResults: l, inputSelectorResultsCopy: d } = e, c = n(() => ({}), ...a);
  if (!(c.apply(null, l) === c.apply(null, d))) {
    let m;
    try {
      throw new Error();
    } catch (h) {
      ({ stack: m } = h);
    }
    console.warn(
      `An input selector returned a different result when passed same arguments.
This means your output selector will likely run more frequently than intended.
Avoid returning a new reference inside your input selector, e.g.
\`createSelector([state => state.todos.map(todo => todo.id)], todoIds => todoIds.length)\``,
      {
        arguments: r,
        firstInputs: l,
        secondInputs: d,
        stack: m
      }
    );
  }
}, uo = {
  inputStabilityCheck: "once",
  identityFunctionCheck: "once"
};
function fo(e, t = `expected a function, instead received ${typeof e}`) {
  if (typeof e != "function")
    throw new TypeError(t);
}
function ho(e, t = `expected an object, instead received ${typeof e}`) {
  if (typeof e != "object")
    throw new TypeError(t);
}
function mo(e, t = "expected all items to be functions, instead received the following types: ") {
  if (!e.every((r) => typeof r == "function")) {
    const r = e.map(
      (n) => typeof n == "function" ? `function ${n.name || "unnamed"}()` : typeof n
    ).join(", ");
    throw new TypeError(`${t}[${r}]`);
  }
}
var kn = (e) => Array.isArray(e) ? e : [e];
function po(e) {
  const t = Array.isArray(e[0]) ? e[0] : e;
  return mo(
    t,
    "createSelector expects all input-selectors to be functions, but received the following types: "
  ), t;
}
function In(e, t) {
  const r = [], { length: n } = e;
  for (let a = 0; a < n; a++)
    r.push(e[a].apply(null, t));
  return r;
}
var go = (e, t) => {
  const { identityFunctionCheck: r, inputStabilityCheck: n } = {
    ...uo,
    ...t
  };
  return {
    identityFunctionCheck: {
      shouldRun: r === "always" || r === "once" && e,
      run: co
    },
    inputStabilityCheck: {
      shouldRun: n === "always" || n === "once" && e,
      run: lo
    }
  };
}, yo = class {
  constructor(e) {
    this.value = e;
  }
  deref() {
    return this.value;
  }
}, xo = typeof WeakRef < "u" ? WeakRef : yo, bo = 0, On = 1;
function jr() {
  return {
    s: bo,
    v: void 0,
    o: null,
    p: null
  };
}
function _a(e, t = {}) {
  let r = jr();
  const { resultEqualityCheck: n } = t;
  let a, l = 0;
  function d() {
    var y;
    let c = r;
    const { length: f } = arguments;
    for (let x = 0, N = f; x < N; x++) {
      const S = arguments[x];
      if (typeof S == "function" || typeof S == "object" && S !== null) {
        let E = c.o;
        E === null && (c.o = E = /* @__PURE__ */ new WeakMap());
        const p = E.get(S);
        p === void 0 ? (c = jr(), E.set(S, c)) : c = p;
      } else {
        let E = c.p;
        E === null && (c.p = E = /* @__PURE__ */ new Map());
        const p = E.get(S);
        p === void 0 ? (c = jr(), E.set(S, c)) : c = p;
      }
    }
    const m = c;
    let h;
    if (c.s === On)
      h = c.v;
    else if (h = e.apply(null, arguments), l++, n) {
      const x = ((y = a == null ? void 0 : a.deref) == null ? void 0 : y.call(a)) ?? a;
      x != null && n(x, h) && (h = x, l !== 0 && l--), a = typeof h == "object" && h !== null || typeof h == "function" ? new xo(h) : h;
    }
    return m.s = On, m.v = h, h;
  }
  return d.clearCache = () => {
    r = jr(), d.resetResultsCount();
  }, d.resultsCount = () => l, d.resetResultsCount = () => {
    l = 0;
  }, d;
}
function vo(e, ...t) {
  const r = typeof e == "function" ? {
    memoize: e,
    memoizeOptions: t
  } : e, n = (...a) => {
    let l = 0, d = 0, c, f = {}, m = a.pop();
    typeof m == "object" && (f = m, m = a.pop()), fo(
      m,
      `createSelector expects an output function after the inputs, but received: [${typeof m}]`
    );
    const h = {
      ...r,
      ...f
    }, {
      memoize: y,
      memoizeOptions: x = [],
      argsMemoize: N = _a,
      argsMemoizeOptions: S = [],
      devModeChecks: E = {}
    } = h, p = kn(x), v = kn(S), w = po(a), A = y(function() {
      return l++, m.apply(
        null,
        arguments
      );
    }, ...p);
    let _ = !0;
    const T = N(function() {
      d++;
      const R = In(
        w,
        arguments
      );
      if (c = A.apply(null, R), process.env.NODE_ENV !== "production") {
        const { identityFunctionCheck: k, inputStabilityCheck: I } = go(_, E);
        if (k.shouldRun && k.run(
          m,
          R,
          c
        ), I.shouldRun) {
          const $ = In(
            w,
            arguments
          );
          I.run(
            { inputSelectorResults: R, inputSelectorResultsCopy: $ },
            { memoize: y, memoizeOptions: p },
            arguments
          );
        }
        _ && (_ = !1);
      }
      return c;
    }, ...v);
    return Object.assign(T, {
      resultFunc: m,
      memoizedResultFunc: A,
      dependencies: w,
      dependencyRecomputations: () => d,
      resetDependencyRecomputations: () => {
        d = 0;
      },
      lastResult: () => c,
      recomputations: () => l,
      resetRecomputations: () => {
        l = 0;
      },
      memoize: y,
      argsMemoize: N
    });
  };
  return Object.assign(n, {
    withTypes: () => n
  }), n;
}
var _t = /* @__PURE__ */ vo(_a), wo = Object.assign(
  (e, t = _t) => {
    ho(
      e,
      `createStructuredSelector expects first argument to be an object where each property is a selector, instead received a ${typeof e}`
    );
    const r = Object.keys(e), n = r.map(
      (l) => e[l]
    );
    return t(
      n,
      (...l) => l.reduce((d, c, f) => (d[r[f]] = c, d), {})
    );
  },
  { withTypes: () => wo }
);
function Aa(e) {
  return ({ dispatch: r, getState: n }) => (a) => (l) => typeof l == "function" ? l(r, n, e) : a(l);
}
var Eo = Aa(), No = Aa, jo = typeof window < "u" && window.__REDUX_DEVTOOLS_EXTENSION_COMPOSE__ ? window.__REDUX_DEVTOOLS_EXTENSION_COMPOSE__ : function() {
  if (arguments.length !== 0)
    return typeof arguments[0] == "object" ? Br : Br.apply(null, arguments);
}, Ta = (e) => e && typeof e.match == "function";
function cr(e, t) {
  function r(...n) {
    if (t) {
      let a = t(...n);
      if (!a)
        throw new Error(process.env.NODE_ENV === "production" ? ie(0) : "prepareAction did not return an object");
      return {
        type: e,
        payload: a.payload,
        ..."meta" in a && {
          meta: a.meta
        },
        ..."error" in a && {
          error: a.error
        }
      };
    }
    return {
      type: e,
      payload: n[0]
    };
  }
  return r.toString = () => `${e}`, r.type = e, r.match = (n) => pa(n) && n.type === e, r;
}
function So(e) {
  return typeof e == "function" && "type" in e && // hasMatchFunction only wants Matchers but I don't see the point in rewriting it
  Ta(e);
}
function _o(e) {
  const t = e ? `${e}`.split("/") : [], r = t[t.length - 1] || "actionCreator";
  return `Detected an action creator with type "${e || "unknown"}" being dispatched. 
Make sure you're calling the action creator before dispatching, i.e. \`dispatch(${r}())\` instead of \`dispatch(${r})\`. This is necessary even if the action has no payload.`;
}
function Ao(e = {}) {
  if (process.env.NODE_ENV === "production")
    return () => (r) => (n) => r(n);
  const {
    isActionCreator: t = So
  } = e;
  return () => (r) => (n) => (t(n) && console.warn(_o(n.type)), r(n));
}
function Ca(e, t) {
  let r = 0;
  return {
    measureTime(n) {
      const a = Date.now();
      try {
        return n();
      } finally {
        const l = Date.now();
        r += l - a;
      }
    },
    warnIfExceeded() {
      r > e && console.warn(`${t} took ${r}ms, which is more than the warning threshold of ${e}ms. 
If your state or actions are very large, you may want to disable the middleware as it might cause too much of a slowdown in development mode. See https://redux-toolkit.js.org/api/getDefaultMiddleware for instructions.
It is disabled in production builds, so you don't need to worry about that.`);
    }
  };
}
var Ra = class ar extends Array {
  constructor(...t) {
    super(...t), Object.setPrototypeOf(this, ar.prototype);
  }
  static get [Symbol.species]() {
    return ar;
  }
  concat(...t) {
    return super.concat.apply(this, t);
  }
  prepend(...t) {
    return t.length === 1 && Array.isArray(t[0]) ? new ar(...t[0].concat(this)) : new ar(...t.concat(this));
  }
};
function Dn(e) {
  return qe(e) ? Sa(e, () => {
  }) : e;
}
function Sr(e, t, r) {
  return e.has(t) ? e.get(t) : e.set(t, r(t)).get(t);
}
function To(e) {
  return typeof e != "object" || e == null || Object.isFrozen(e);
}
function Co(e, t, r) {
  const n = ka(e, t, r);
  return {
    detectMutations() {
      return Ia(e, t, n, r);
    }
  };
}
function ka(e, t = [], r, n = "", a = /* @__PURE__ */ new Set()) {
  const l = {
    value: r
  };
  if (!e(r) && !a.has(r)) {
    a.add(r), l.children = {};
    const d = t.length > 0;
    for (const c in r) {
      const f = n ? n + "." + c : c;
      d && t.some((h) => h instanceof RegExp ? h.test(f) : f === h) || (l.children[c] = ka(e, t, r[c], f));
    }
  }
  return l;
}
function Ia(e, t = [], r, n, a = !1, l = "") {
  const d = r ? r.value : void 0, c = d === n;
  if (a && !c && !Number.isNaN(n))
    return {
      wasMutated: !0,
      path: l
    };
  if (e(d) || e(n))
    return {
      wasMutated: !1
    };
  const f = {};
  for (let h in r.children)
    f[h] = !0;
  for (let h in n)
    f[h] = !0;
  const m = t.length > 0;
  for (let h in f) {
    const y = l ? l + "." + h : h;
    if (m && t.some((S) => S instanceof RegExp ? S.test(y) : y === S))
      continue;
    const x = Ia(e, t, r.children[h], n[h], c, y);
    if (x.wasMutated)
      return x;
  }
  return {
    wasMutated: !1
  };
}
function Ro(e = {}) {
  if (process.env.NODE_ENV === "production")
    return () => (t) => (r) => t(r);
  {
    let t = function(c, f, m, h) {
      return JSON.stringify(c, r(f, h), m);
    }, r = function(c, f) {
      let m = [], h = [];
      return f || (f = function(y, x) {
        return m[0] === x ? "[Circular ~]" : "[Circular ~." + h.slice(0, m.indexOf(x)).join(".") + "]";
      }), function(y, x) {
        if (m.length > 0) {
          var N = m.indexOf(this);
          ~N ? m.splice(N + 1) : m.push(this), ~N ? h.splice(N, 1 / 0, y) : h.push(y), ~m.indexOf(x) && (x = f.call(this, y, x));
        } else m.push(x);
        return c == null ? x : c.call(this, y, x);
      };
    }, {
      isImmutable: n = To,
      ignoredPaths: a,
      warnAfter: l = 32
    } = e;
    const d = Co.bind(null, n, a);
    return ({
      getState: c
    }) => {
      let f = c(), m = d(f), h;
      return (y) => (x) => {
        const N = Ca(l, "ImmutableStateInvariantMiddleware");
        N.measureTime(() => {
          if (f = c(), h = m.detectMutations(), m = d(f), h.wasMutated)
            throw new Error(process.env.NODE_ENV === "production" ? ie(19) : `A state mutation was detected between dispatches, in the path '${h.path || ""}'.  This may cause incorrect behavior. (https://redux.js.org/style-guide/style-guide#do-not-mutate-state)`);
        });
        const S = y(x);
        return N.measureTime(() => {
          if (f = c(), h = m.detectMutations(), m = d(f), h.wasMutated)
            throw new Error(process.env.NODE_ENV === "production" ? ie(20) : `A state mutation was detected inside a dispatch, in the path: ${h.path || ""}. Take a look at the reducer(s) handling the action ${t(x)}. (https://redux.js.org/style-guide/style-guide#do-not-mutate-state)`);
        }), N.warnIfExceeded(), S;
      };
    };
  }
}
function Oa(e) {
  const t = typeof e;
  return e == null || t === "string" || t === "boolean" || t === "number" || Array.isArray(e) || pr(e);
}
function Fs(e, t = "", r = Oa, n, a = [], l) {
  let d;
  if (!r(e))
    return {
      keyPath: t || "<root>",
      value: e
    };
  if (typeof e != "object" || e === null || l != null && l.has(e)) return !1;
  const c = n != null ? n(e) : Object.entries(e), f = a.length > 0;
  for (const [m, h] of c) {
    const y = t ? t + "." + m : m;
    if (!(f && a.some((N) => N instanceof RegExp ? N.test(y) : y === N))) {
      if (!r(h))
        return {
          keyPath: y,
          value: h
        };
      if (typeof h == "object" && (d = Fs(h, y, r, n, a, l), d))
        return d;
    }
  }
  return l && Da(e) && l.add(e), !1;
}
function Da(e) {
  if (!Object.isFrozen(e)) return !1;
  for (const t of Object.values(e))
    if (!(typeof t != "object" || t === null) && !Da(t))
      return !1;
  return !0;
}
function ko(e = {}) {
  if (process.env.NODE_ENV === "production")
    return () => (t) => (r) => t(r);
  {
    const {
      isSerializable: t = Oa,
      getEntries: r,
      ignoredActions: n = [],
      ignoredActionPaths: a = ["meta.arg", "meta.baseQueryMeta"],
      ignoredPaths: l = [],
      warnAfter: d = 32,
      ignoreState: c = !1,
      ignoreActions: f = !1,
      disableCache: m = !1
    } = e, h = !m && WeakSet ? /* @__PURE__ */ new WeakSet() : void 0;
    return (y) => (x) => (N) => {
      if (!pa(N))
        return x(N);
      const S = x(N), E = Ca(d, "SerializableStateInvariantMiddleware");
      return !f && !(n.length && n.indexOf(N.type) !== -1) && E.measureTime(() => {
        const p = Fs(N, "", t, r, a, h);
        if (p) {
          const {
            keyPath: v,
            value: w
          } = p;
          console.error(`A non-serializable value was detected in an action, in the path: \`${v}\`. Value:`, w, `
Take a look at the logic that dispatched this action: `, N, `
(See https://redux.js.org/faq/actions#why-should-type-be-a-string-or-at-least-serializable-why-should-my-action-types-be-constants)`, `
(To allow non-serializable values see: https://redux-toolkit.js.org/usage/usage-guide#working-with-non-serializable-data)`);
        }
      }), c || (E.measureTime(() => {
        const p = y.getState(), v = Fs(p, "", t, r, l, h);
        if (v) {
          const {
            keyPath: w,
            value: A
          } = v;
          console.error(`A non-serializable value was detected in the state, in the path: \`${w}\`. Value:`, A, `
Take a look at the reducer(s) handling this action type: ${N.type}.
(See https://redux.js.org/faq/organizing-state#can-i-put-functions-promises-or-other-non-serializable-items-in-my-store-state)`);
        }
      }), E.warnIfExceeded()), S;
    };
  }
}
function _r(e) {
  return typeof e == "boolean";
}
var Io = () => function(t) {
  const {
    thunk: r = !0,
    immutableCheck: n = !0,
    serializableCheck: a = !0,
    actionCreatorCheck: l = !0
  } = t ?? {};
  let d = new Ra();
  if (r && (_r(r) ? d.push(Eo) : d.push(No(r.extraArgument))), process.env.NODE_ENV !== "production") {
    if (n) {
      let c = {};
      _r(n) || (c = n), d.unshift(Ro(c));
    }
    if (a) {
      let c = {};
      _r(a) || (c = a), d.push(ko(c));
    }
    if (l) {
      let c = {};
      _r(l) || (c = l), d.unshift(Ao(c));
    }
  }
  return d;
}, Oo = "RTK_autoBatch", Fn = (e) => (t) => {
  setTimeout(t, e);
}, Do = (e = {
  type: "raf"
}) => (t) => (...r) => {
  const n = t(...r);
  let a = !0, l = !1, d = !1;
  const c = /* @__PURE__ */ new Set(), f = e.type === "tick" ? queueMicrotask : e.type === "raf" ? (
    // requestAnimationFrame won't exist in SSR environments. Fall back to a vague approximation just to keep from erroring.
    typeof window < "u" && window.requestAnimationFrame ? window.requestAnimationFrame : Fn(10)
  ) : e.type === "callback" ? e.queueNotification : Fn(e.timeout), m = () => {
    d = !1, l && (l = !1, c.forEach((h) => h()));
  };
  return Object.assign({}, n, {
    // Override the base `store.subscribe` method to keep original listeners
    // from running if we're delaying notifications
    subscribe(h) {
      const y = () => a && h(), x = n.subscribe(y);
      return c.add(h), () => {
        x(), c.delete(h);
      };
    },
    // Override the base `store.dispatch` method so that we can check actions
    // for the `shouldAutoBatch` flag and determine if batching is active
    dispatch(h) {
      var y;
      try {
        return a = !((y = h == null ? void 0 : h.meta) != null && y[Oo]), l = !a, l && (d || (d = !0, f(m))), n.dispatch(h);
      } finally {
        a = !0;
      }
    }
  });
}, Fo = (e) => function(r) {
  const {
    autoBatch: n = !0
  } = r ?? {};
  let a = new Ra(e);
  return n && a.push(Do(typeof n == "object" ? n : void 0)), a;
};
function Po(e) {
  const t = Io(), {
    reducer: r = void 0,
    middleware: n,
    devTools: a = !0,
    duplicateMiddlewareCheck: l = !0,
    preloadedState: d = void 0,
    enhancers: c = void 0
  } = e || {};
  let f;
  if (typeof r == "function")
    f = r;
  else if (pr(r))
    f = Hi(r);
  else
    throw new Error(process.env.NODE_ENV === "production" ? ie(1) : "`reducer` is a required argument, and must be a function or an object of functions that can be passed to combineReducers");
  if (process.env.NODE_ENV !== "production" && n && typeof n != "function")
    throw new Error(process.env.NODE_ENV === "production" ? ie(2) : "`middleware` field must be a callback");
  let m;
  if (typeof n == "function") {
    if (m = n(t), process.env.NODE_ENV !== "production" && !Array.isArray(m))
      throw new Error(process.env.NODE_ENV === "production" ? ie(3) : "when using a middleware builder function, an array of middleware must be returned");
  } else
    m = t();
  if (process.env.NODE_ENV !== "production" && m.some((E) => typeof E != "function"))
    throw new Error(process.env.NODE_ENV === "production" ? ie(4) : "each middleware provided to configureStore must be a function");
  if (process.env.NODE_ENV !== "production" && l) {
    let E = /* @__PURE__ */ new Set();
    m.forEach((p) => {
      if (E.has(p))
        throw new Error(process.env.NODE_ENV === "production" ? ie(42) : "Duplicate middleware references found when creating the store. Ensure that each middleware is only included once.");
      E.add(p);
    });
  }
  let h = Br;
  a && (h = jo({
    // Enable capture of stack traces for dispatched Redux actions
    trace: process.env.NODE_ENV !== "production",
    ...typeof a == "object" && a
  }));
  const y = zi(...m), x = Fo(y);
  if (process.env.NODE_ENV !== "production" && c && typeof c != "function")
    throw new Error(process.env.NODE_ENV === "production" ? ie(5) : "`enhancers` field must be a callback");
  let N = typeof c == "function" ? c(x) : x();
  if (process.env.NODE_ENV !== "production" && !Array.isArray(N))
    throw new Error(process.env.NODE_ENV === "production" ? ie(6) : "`enhancers` callback must return an array");
  if (process.env.NODE_ENV !== "production" && N.some((E) => typeof E != "function"))
    throw new Error(process.env.NODE_ENV === "production" ? ie(7) : "each enhancer provided to configureStore must be a function");
  process.env.NODE_ENV !== "production" && m.length && !N.includes(y) && console.error("middlewares were provided, but middleware enhancer was not included in final enhancers - make sure to call `getDefaultEnhancers`");
  const S = h(...N);
  return ma(f, d, S);
}
function Fa(e) {
  const t = {}, r = [];
  let n;
  const a = {
    addCase(l, d) {
      if (process.env.NODE_ENV !== "production") {
        if (r.length > 0)
          throw new Error(process.env.NODE_ENV === "production" ? ie(26) : "`builder.addCase` should only be called before calling `builder.addMatcher`");
        if (n)
          throw new Error(process.env.NODE_ENV === "production" ? ie(27) : "`builder.addCase` should only be called before calling `builder.addDefaultCase`");
      }
      const c = typeof l == "string" ? l : l.type;
      if (!c)
        throw new Error(process.env.NODE_ENV === "production" ? ie(28) : "`builder.addCase` cannot be called with an empty action type");
      if (c in t)
        throw new Error(process.env.NODE_ENV === "production" ? ie(29) : `\`builder.addCase\` cannot be called with two reducers for the same action type '${c}'`);
      return t[c] = d, a;
    },
    addAsyncThunk(l, d) {
      if (process.env.NODE_ENV !== "production" && n)
        throw new Error(process.env.NODE_ENV === "production" ? ie(43) : "`builder.addAsyncThunk` should only be called before calling `builder.addDefaultCase`");
      return d.pending && (t[l.pending.type] = d.pending), d.rejected && (t[l.rejected.type] = d.rejected), d.fulfilled && (t[l.fulfilled.type] = d.fulfilled), d.settled && r.push({
        matcher: l.settled,
        reducer: d.settled
      }), a;
    },
    addMatcher(l, d) {
      if (process.env.NODE_ENV !== "production" && n)
        throw new Error(process.env.NODE_ENV === "production" ? ie(30) : "`builder.addMatcher` should only be called before calling `builder.addDefaultCase`");
      return r.push({
        matcher: l,
        reducer: d
      }), a;
    },
    addDefaultCase(l) {
      if (process.env.NODE_ENV !== "production" && n)
        throw new Error(process.env.NODE_ENV === "production" ? ie(31) : "`builder.addDefaultCase` can only be called once");
      return n = l, a;
    }
  };
  return e(a), [t, r, n];
}
function Mo(e) {
  return typeof e == "function";
}
function Bo(e, t) {
  if (process.env.NODE_ENV !== "production" && typeof t == "object")
    throw new Error(process.env.NODE_ENV === "production" ? ie(8) : "The object notation for `createReducer` has been removed. Please use the 'builder callback' notation instead: https://redux-toolkit.js.org/api/createReducer");
  let [r, n, a] = Fa(t), l;
  if (Mo(e))
    l = () => Dn(e());
  else {
    const c = Dn(e);
    l = () => c;
  }
  function d(c = l(), f) {
    let m = [r[f.type], ...n.filter(({
      matcher: h
    }) => h(f)).map(({
      reducer: h
    }) => h)];
    return m.filter((h) => !!h).length === 0 && (m = [a]), m.reduce((h, y) => {
      if (y)
        if (ht(h)) {
          const N = y(h, f);
          return N === void 0 ? h : N;
        } else {
          if (qe(h))
            return Sa(h, (x) => y(x, f));
          {
            const x = y(h, f);
            if (x === void 0) {
              if (h === null)
                return h;
              throw Error("A case reducer on a non-draftable value must not return undefined");
            }
            return x;
          }
        }
      return h;
    }, c);
  }
  return d.getInitialState = l, d;
}
var Lo = (e, t) => Ta(e) ? e.match(t) : e(t);
function $o(...e) {
  return (t) => e.some((r) => Lo(r, t));
}
var Uo = "ModuleSymbhasOwnPr-0123456789ABCDEFGHNRVfgctiUvz_KqYTJkLxpZXIjQW", Ko = (e = 21) => {
  let t = "", r = e;
  for (; r--; )
    t += Uo[Math.random() * 64 | 0];
  return t;
}, Vo = ["name", "message", "stack", "code"], ms = class {
  constructor(e, t) {
    /*
    type-only property to distinguish between RejectWithValue and FulfillWithMeta
    does not exist at runtime
    */
    lt(this, "_type");
    this.payload = e, this.meta = t;
  }
}, Pn = class {
  constructor(e, t) {
    /*
    type-only property to distinguish between RejectWithValue and FulfillWithMeta
    does not exist at runtime
    */
    lt(this, "_type");
    this.payload = e, this.meta = t;
  }
}, Ho = (e) => {
  if (typeof e == "object" && e !== null) {
    const t = {};
    for (const r of Vo)
      typeof e[r] == "string" && (t[r] = e[r]);
    return t;
  }
  return {
    message: String(e)
  };
}, Mn = "External signal was aborted", Jt = /* @__PURE__ */ (() => {
  function e(t, r, n) {
    const a = cr(t + "/fulfilled", (f, m, h, y) => ({
      payload: f,
      meta: {
        ...y || {},
        arg: h,
        requestId: m,
        requestStatus: "fulfilled"
      }
    })), l = cr(t + "/pending", (f, m, h) => ({
      payload: void 0,
      meta: {
        ...h || {},
        arg: m,
        requestId: f,
        requestStatus: "pending"
      }
    })), d = cr(t + "/rejected", (f, m, h, y, x) => ({
      payload: y,
      error: (n && n.serializeError || Ho)(f || "Rejected"),
      meta: {
        ...x || {},
        arg: h,
        requestId: m,
        rejectedWithValue: !!y,
        requestStatus: "rejected",
        aborted: (f == null ? void 0 : f.name) === "AbortError",
        condition: (f == null ? void 0 : f.name) === "ConditionError"
      }
    }));
    function c(f, {
      signal: m
    } = {}) {
      return (h, y, x) => {
        const N = n != null && n.idGenerator ? n.idGenerator(f) : Ko(), S = new AbortController();
        let E, p;
        function v(A) {
          p = A, S.abort();
        }
        m && (m.aborted ? v(Mn) : m.addEventListener("abort", () => v(Mn), {
          once: !0
        }));
        const w = async function() {
          var T, M;
          let A;
          try {
            let R = (T = n == null ? void 0 : n.condition) == null ? void 0 : T.call(n, f, {
              getState: y,
              extra: x
            });
            if (Go(R) && (R = await R), R === !1 || S.signal.aborted)
              throw {
                name: "ConditionError",
                message: "Aborted due to condition callback returning false."
              };
            const k = new Promise((I, $) => {
              E = () => {
                $({
                  name: "AbortError",
                  message: p || "Aborted"
                });
              }, S.signal.addEventListener("abort", E, {
                once: !0
              });
            });
            h(l(N, f, (M = n == null ? void 0 : n.getPendingMeta) == null ? void 0 : M.call(n, {
              requestId: N,
              arg: f
            }, {
              getState: y,
              extra: x
            }))), A = await Promise.race([k, Promise.resolve(r(f, {
              dispatch: h,
              getState: y,
              extra: x,
              requestId: N,
              signal: S.signal,
              abort: v,
              rejectWithValue: (I, $) => new ms(I, $),
              fulfillWithValue: (I, $) => new Pn(I, $)
            })).then((I) => {
              if (I instanceof ms)
                throw I;
              return I instanceof Pn ? a(I.payload, N, f, I.meta) : a(I, N, f);
            })]);
          } catch (R) {
            A = R instanceof ms ? d(null, N, f, R.payload, R.meta) : d(R, N, f);
          } finally {
            E && S.signal.removeEventListener("abort", E);
          }
          return n && !n.dispatchConditionRejection && d.match(A) && A.meta.condition || h(A), A;
        }();
        return Object.assign(w, {
          abort: v,
          requestId: N,
          arg: f,
          unwrap() {
            return w.then(zo);
          }
        });
      };
    }
    return Object.assign(c, {
      pending: l,
      rejected: d,
      fulfilled: a,
      settled: $o(d, a),
      typePrefix: t
    });
  }
  return e.withTypes = () => e, e;
})();
function zo(e) {
  if (e.meta && e.meta.rejectedWithValue)
    throw e.payload;
  if (e.error)
    throw e.error;
  return e.payload;
}
function Go(e) {
  return e !== null && typeof e == "object" && typeof e.then == "function";
}
var qo = /* @__PURE__ */ Symbol.for("rtk-slice-createasyncthunk");
function Wo(e, t) {
  return `${e}/${t}`;
}
function Yo({
  creators: e
} = {}) {
  var r;
  const t = (r = e == null ? void 0 : e.asyncThunk) == null ? void 0 : r[qo];
  return function(a) {
    const {
      name: l,
      reducerPath: d = l
    } = a;
    if (!l)
      throw new Error(process.env.NODE_ENV === "production" ? ie(11) : "`name` is a required option for createSlice");
    typeof process < "u" && process.env.NODE_ENV === "development" && a.initialState === void 0 && console.error("You must provide an `initialState` value that is not `undefined`. You may have misspelled `initialState`");
    const c = (typeof a.reducers == "function" ? a.reducers(Jo()) : a.reducers) || {}, f = Object.keys(c), m = {
      sliceCaseReducersByName: {},
      sliceCaseReducersByType: {},
      actionCreators: {},
      sliceMatchers: []
    }, h = {
      addCase(_, T) {
        const M = typeof _ == "string" ? _ : _.type;
        if (!M)
          throw new Error(process.env.NODE_ENV === "production" ? ie(12) : "`context.addCase` cannot be called with an empty action type");
        if (M in m.sliceCaseReducersByType)
          throw new Error(process.env.NODE_ENV === "production" ? ie(13) : "`context.addCase` cannot be called with two reducers for the same action type: " + M);
        return m.sliceCaseReducersByType[M] = T, h;
      },
      addMatcher(_, T) {
        return m.sliceMatchers.push({
          matcher: _,
          reducer: T
        }), h;
      },
      exposeAction(_, T) {
        return m.actionCreators[_] = T, h;
      },
      exposeCaseReducer(_, T) {
        return m.sliceCaseReducersByName[_] = T, h;
      }
    };
    f.forEach((_) => {
      const T = c[_], M = {
        reducerName: _,
        type: Wo(l, _),
        createNotation: typeof a.reducers == "function"
      };
      Xo(T) ? tc(M, T, h, t) : Zo(M, T, h);
    });
    function y() {
      if (process.env.NODE_ENV !== "production" && typeof a.extraReducers == "object")
        throw new Error(process.env.NODE_ENV === "production" ? ie(14) : "The object notation for `createSlice.extraReducers` has been removed. Please use the 'builder callback' notation instead: https://redux-toolkit.js.org/api/createSlice");
      const [_ = {}, T = [], M = void 0] = typeof a.extraReducers == "function" ? Fa(a.extraReducers) : [a.extraReducers], R = {
        ..._,
        ...m.sliceCaseReducersByType
      };
      return Bo(a.initialState, (k) => {
        for (let I in R)
          k.addCase(I, R[I]);
        for (let I of m.sliceMatchers)
          k.addMatcher(I.matcher, I.reducer);
        for (let I of T)
          k.addMatcher(I.matcher, I.reducer);
        M && k.addDefaultCase(M);
      });
    }
    const x = (_) => _, N = /* @__PURE__ */ new Map(), S = /* @__PURE__ */ new WeakMap();
    let E;
    function p(_, T) {
      return E || (E = y()), E(_, T);
    }
    function v() {
      return E || (E = y()), E.getInitialState();
    }
    function w(_, T = !1) {
      function M(k) {
        let I = k[_];
        if (typeof I > "u") {
          if (T)
            I = Sr(S, M, v);
          else if (process.env.NODE_ENV !== "production")
            throw new Error(process.env.NODE_ENV === "production" ? ie(15) : "selectSlice returned undefined for an uninjected slice reducer");
        }
        return I;
      }
      function R(k = x) {
        const I = Sr(N, T, () => /* @__PURE__ */ new WeakMap());
        return Sr(I, k, () => {
          const $ = {};
          for (const [F, z] of Object.entries(a.selectors ?? {}))
            $[F] = Qo(z, k, () => Sr(S, k, v), T);
          return $;
        });
      }
      return {
        reducerPath: _,
        getSelectors: R,
        get selectors() {
          return R(M);
        },
        selectSlice: M
      };
    }
    const A = {
      name: l,
      reducer: p,
      actions: m.actionCreators,
      caseReducers: m.sliceCaseReducersByName,
      getInitialState: v,
      ...w(d),
      injectInto(_, {
        reducerPath: T,
        ...M
      } = {}) {
        const R = T ?? d;
        return _.inject({
          reducerPath: R,
          reducer: p
        }, M), {
          ...A,
          ...w(R, !0)
        };
      }
    };
    return A;
  };
}
function Qo(e, t, r, n) {
  function a(l, ...d) {
    let c = t(l);
    if (typeof c > "u") {
      if (n)
        c = r();
      else if (process.env.NODE_ENV !== "production")
        throw new Error(process.env.NODE_ENV === "production" ? ie(16) : "selectState returned undefined for an uninjected slice reducer");
    }
    return e(c, ...d);
  }
  return a.unwrapped = e, a;
}
var Ws = /* @__PURE__ */ Yo();
function Jo() {
  function e(t, r) {
    return {
      _reducerDefinitionType: "asyncThunk",
      payloadCreator: t,
      ...r
    };
  }
  return e.withTypes = () => e, {
    reducer(t) {
      return Object.assign({
        // hack so the wrapping function has the same name as the original
        // we need to create a wrapper so the `reducerDefinitionType` is not assigned to the original
        [t.name](...r) {
          return t(...r);
        }
      }[t.name], {
        _reducerDefinitionType: "reducer"
        /* reducer */
      });
    },
    preparedReducer(t, r) {
      return {
        _reducerDefinitionType: "reducerWithPrepare",
        prepare: t,
        reducer: r
      };
    },
    asyncThunk: e
  };
}
function Zo({
  type: e,
  reducerName: t,
  createNotation: r
}, n, a) {
  let l, d;
  if ("reducer" in n) {
    if (r && !ec(n))
      throw new Error(process.env.NODE_ENV === "production" ? ie(17) : "Please use the `create.preparedReducer` notation for prepared action creators with the `create` notation.");
    l = n.reducer, d = n.prepare;
  } else
    l = n;
  a.addCase(e, l).exposeCaseReducer(t, l).exposeAction(t, d ? cr(e, d) : cr(e));
}
function Xo(e) {
  return e._reducerDefinitionType === "asyncThunk";
}
function ec(e) {
  return e._reducerDefinitionType === "reducerWithPrepare";
}
function tc({
  type: e,
  reducerName: t
}, r, n, a) {
  if (!a)
    throw new Error(process.env.NODE_ENV === "production" ? ie(18) : "Cannot use `create.asyncThunk` in the built-in `createSlice`. Use `buildCreateSlice({ creators: { asyncThunk: asyncThunkCreator } })` to create a customised version of `createSlice`.");
  const {
    payloadCreator: l,
    fulfilled: d,
    pending: c,
    rejected: f,
    settled: m,
    options: h
  } = r, y = a(e, l, h);
  n.exposeAction(t, y), d && n.addCase(y.fulfilled, d), c && n.addCase(y.pending, c), f && n.addCase(y.rejected, f), m && n.addMatcher(y.settled, m), n.exposeCaseReducer(t, {
    fulfilled: d || Ar,
    pending: c || Ar,
    rejected: f || Ar,
    settled: m || Ar
  });
}
function Ar() {
}
function ie(e) {
  return `Minified Redux Toolkit error #${e}; visit https://redux-toolkit.js.org/Errors?code=${e} for the full message or use the non-minified dev environment for full errors. `;
}
const rc = {
  GET_INDEXING_STATUS: "/indexing/status",
  GET_INGESTION_CONFIG: "/ingestion/config",
  HEALTH_CHECK: "/ingestion/health",
  PROCESS_JSON: "/ingestion/process",
  GET_STATUS: "/ingestion/status",
  VALIDATE_JSON: "/ingestion/validate",
  ANALYZE_QUERY: "/llm-query/analyze",
  GET_BACKFILL_STATUS: (e) => `/llm-query/backfill/${e}`,
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
  GET_SCHEMA: (e) => `/schema/${e}`,
  APPROVE_SCHEMA: (e) => `/schema/${e}/approve`,
  BLOCK_SCHEMA: (e) => `/schema/${e}/block`,
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
  GET_BACKFILL: (e) => `/transforms/backfills/${e}`,
  GET_TRANSFORM_QUEUE: "/transforms/queue",
  ADD_TO_TRANSFORM_QUEUE: (e) => `/transforms/queue/${e}`,
  GET_TRANSFORM_STATISTICS: "/transforms/statistics"
}, sc = {
  ROOT: "/api"
}, te = rc, nc = 3e4, ac = 3, ic = 1e3, we = {
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
}, Ee = {
  NONE: 0,
  // Mutations, destructive operations
  LIMITED: 1,
  // State changes, config operations, registrations
  STANDARD: 2,
  // Most read operations, network issues
  CRITICAL: 3
}, Nt = {
  // 3 minutes - schema state, transforms
  STANDARD: 3e5,
  // 1 hour - system public key
  // Semantic aliases
  SYSTEM_STATUS: 3e4,
  SCHEMA_DATA: 3e5,
  SECURITY_STATUS: 6e4,
  SYSTEM_PUBLIC_KEY: 36e5
}, Ue = {
  BAD_REQUEST: 400,
  UNAUTHORIZED: 401,
  FORBIDDEN: 403,
  NOT_FOUND: 404,
  INTERNAL_SERVER_ERROR: 500,
  BAD_GATEWAY: 502,
  SERVICE_UNAVAILABLE: 503
}, ps = {
  JSON: "application/json",
  FORM_DATA: "multipart/form-data",
  URL_ENCODED: "application/x-www-form-urlencoded",
  TEXT: "text/plain"
}, Tr = {
  CONTENT_TYPE: "Content-Type",
  AUTHORIZATION: "Authorization",
  SIGNED_REQUEST: "X-Signed-Request",
  REQUEST_ID: "X-Request-ID",
  AUTHENTICATED: "X-Authenticated"
}, Oe = {
  NETWORK_ERROR: "Network connection failed. Please check your internet connection.",
  TIMEOUT_ERROR: "Request timed out. Please try again.",
  AUTHENTICATION_ERROR: "Authentication required. Please ensure you are properly authenticated.",
  SCHEMA_STATE_ERROR: "Schema operation not allowed. Only approved schemas can be accessed.",
  SERVER_ERROR: "Server error occurred. Please try again later.",
  VALIDATION_ERROR: "Request validation failed. Please check your input.",
  NOT_FOUND_ERROR: "Requested resource not found.",
  PERMISSION_ERROR: "Permission denied. You do not have access to this resource.",
  RATE_LIMIT_ERROR: "Too many requests. Please wait before trying again."
}, lr = {
  DEFAULT_TTL_MS: Nt.STANDARD,
  MAX_CACHE_SIZE: 100,
  SCHEMA_CACHE_TTL_MS: Nt.SCHEMA_DATA,
  SYSTEM_STATUS_CACHE_TTL_MS: Nt.SYSTEM_STATUS
}, Ps = {
  RETRYABLE_STATUS_CODES: [408, 429, 500, 502, 503, 504],
  EXPONENTIAL_BACKOFF_MULTIPLIER: 2,
  MAX_RETRY_DELAY_MS: 1e4
}, Xr = {
  // Use relative path for CloudFront compatibility
  BASE_URL: "api"
}, De = {
  AVAILABLE: "available",
  APPROVED: "approved",
  BLOCKED: "blocked"
}, oc = {
  MUTATION: "mutation"
}, Vr = {
  SYSTEM_STATUS: "system-status",
  SECURITY_STATUS: "security-status",
  SYSTEM_PUBLIC_KEY: "system-public-key"
};
class Se extends Error {
  constructor(t, r = 0, n = {}) {
    super(t), this.name = "ApiError", this.status = r, this.response = n.response, this.isNetworkError = n.isNetworkError || !1, this.isTimeoutError = n.isTimeoutError || !1, this.isRetryable = this.determineRetryability(r, n.isNetworkError, n.isTimeoutError), this.requestId = n.requestId, this.timestamp = Date.now(), this.code = n.code, this.details = n.details, Object.setPrototypeOf(this, Se.prototype);
  }
  /**
   * Determines if an error is retryable based on status code and error type
   */
  determineRetryability(t, r, n) {
    return r || n ? !0 : Ps.RETRYABLE_STATUS_CODES.includes(t);
  }
  /**
   * Convert error to a user-friendly message
   */
  toUserMessage() {
    if (this.isNetworkError)
      return Oe.NETWORK_ERROR;
    if (this.isTimeoutError)
      return Oe.TIMEOUT_ERROR;
    switch (this.status) {
      case Ue.UNAUTHORIZED:
        return Oe.AUTHENTICATION_ERROR;
      case Ue.FORBIDDEN:
        return Oe.PERMISSION_ERROR;
      case Ue.NOT_FOUND:
        return Oe.NOT_FOUND_ERROR;
      case Ue.BAD_REQUEST:
        return Oe.VALIDATION_ERROR;
      case Ue.INTERNAL_SERVER_ERROR:
      case Ue.BAD_GATEWAY:
      case Ue.SERVICE_UNAVAILABLE:
        return Oe.SERVER_ERROR;
      case 429:
        return Oe.RATE_LIMIT_ERROR;
      default:
        return this.message || Oe.SERVER_ERROR;
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
class Ys extends Se {
  constructor(t = Oe.AUTHENTICATION_ERROR, r) {
    super(t, Ue.UNAUTHORIZED, {
      code: "AUTH_ERROR",
      requestId: r
    }), this.name = "AuthenticationError", Object.setPrototypeOf(this, Ys.prototype);
  }
}
class es extends Se {
  constructor(t, r, n, a = Oe.SCHEMA_STATE_ERROR) {
    super(a, Ue.FORBIDDEN, {
      code: "SCHEMA_STATE_ERROR",
      details: { schemaName: t, currentState: r, operation: n }
    }), this.name = "SchemaStateError", this.schemaName = t, this.currentState = r, this.operation = n, Object.setPrototypeOf(this, es.prototype);
  }
}
class Qs extends Se {
  constructor(t = Oe.NETWORK_ERROR, r) {
    super(t, 0, {
      isNetworkError: !0,
      code: "NETWORK_ERROR",
      requestId: r
    }), this.name = "NetworkError", Object.setPrototypeOf(this, Qs.prototype);
  }
}
class Js extends Se {
  constructor(t, r) {
    super(`Request timed out after ${t}ms`, 408, {
      isTimeoutError: !0,
      code: "TIMEOUT_ERROR",
      requestId: r,
      details: { timeoutMs: t }
    }), this.name = "TimeoutError", this.timeoutMs = t, Object.setPrototypeOf(this, Js.prototype);
  }
}
class Zs extends Se {
  constructor(t, r) {
    super("Request validation failed", Ue.BAD_REQUEST, {
      code: "VALIDATION_ERROR",
      requestId: r,
      details: { validationErrors: t }
    }), this.name = "ValidationError", this.validationErrors = t, Object.setPrototypeOf(this, Zs.prototype);
  }
}
class Xs extends Se {
  constructor(t, r) {
    const n = t ? `Rate limit exceeded. Retry after ${t} seconds.` : Oe.RATE_LIMIT_ERROR;
    super(n, 429, {
      code: "RATE_LIMIT_ERROR",
      requestId: r,
      details: { retryAfter: t }
    }), this.name = "RateLimitError", this.retryAfter = t, Object.setPrototypeOf(this, Xs.prototype);
  }
}
class nr {
  /**
   * Create an ApiError from a fetch response
   */
  static async fromResponse(t, r) {
    let n = {};
    try {
      const l = await t.text();
      l && (n = JSON.parse(l));
    } catch {
    }
    const a = typeof n.error == "string" ? n.error : typeof n.message == "string" ? n.message : `HTTP ${t.status}`;
    if (t.status === Ue.UNAUTHORIZED)
      return new Ys(a, r || "");
    if (t.status === 429) {
      const l = t.headers.get("Retry-After");
      return new Xs(l ? parseInt(l) : void 0, r);
    }
    return t.status === Ue.BAD_REQUEST && n.validationErrors ? new Zs(n.validationErrors, r || "") : new Se(a, t.status, {
      response: n,
      requestId: r,
      code: typeof n.code == "string" ? n.code : void 0,
      details: typeof n.details == "object" && n.details !== null ? n.details : void 0
    });
  }
  /**
   * Create an ApiError from a network error
   */
  static fromNetworkError(t, r) {
    return new Qs(t.message, r);
  }
  /**
   * Create an ApiError from a timeout
   */
  static fromTimeout(t, r) {
    return new Js(t, r);
  }
  /**
   * Create a schema state error
   */
  static fromSchemaState(t, r, n) {
    return new es(t, r, n);
  }
}
function cc(e) {
  return e instanceof Se;
}
function lc(e) {
  return cc(e) && e.isRetryable;
}
class dc {
  constructor(t = lr.MAX_CACHE_SIZE) {
    this.cache = /* @__PURE__ */ new Map(), this.maxSize = t;
  }
  get(t) {
    const r = this.cache.get(t);
    return r ? Date.now() > r.timestamp + r.ttl ? (this.cache.delete(t), null) : r.data : null;
  }
  set(t, r, n = lr.DEFAULT_TTL_MS) {
    if (this.cache.size >= this.maxSize) {
      const a = this.cache.keys().next().value;
      this.cache.delete(a);
    }
    this.cache.set(t, {
      data: r,
      timestamp: Date.now(),
      ttl: n,
      key: t
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
class uc {
  constructor() {
    this.queue = /* @__PURE__ */ new Map();
  }
  /**
   * Get or create a request promise to prevent duplicate requests
   */
  getOrCreate(t, r) {
    if (this.queue.has(t))
      return this.queue.get(t);
    const n = r().finally(() => {
      this.queue.delete(t);
    });
    return this.queue.set(t, n), n;
  }
  clear() {
    this.queue.clear();
  }
}
class Pa {
  constructor(t = {}) {
    this.requestInterceptors = [], this.responseInterceptors = [], this.errorInterceptors = [], this.metrics = [], this.config = {
      baseUrl: t.baseUrl || Xr.BASE_URL,
      timeout: t.timeout || nc,
      retryAttempts: t.retryAttempts || ac,
      retryDelay: t.retryDelay || ic,
      defaultHeaders: t.defaultHeaders || {},
      enableCache: t.enableCache !== !1,
      enableLogging: t.enableLogging !== !1,
      enableMetrics: t.enableMetrics !== !1
    }, this.cache = new dc(), this.requestQueue = new uc();
  }
  /**
   * HTTP GET method
   */
  async get(t, r = {}) {
    return this.request("GET", t, void 0, r);
  }
  /**
   * HTTP POST method
   */
  async post(t, r, n = {}) {
    return this.request("POST", t, r, n);
  }
  /**
   * HTTP PUT method
   */
  async put(t, r, n = {}) {
    return this.request("PUT", t, r, n);
  }
  /**
   * HTTP DELETE method
   */
  async delete(t, r = {}) {
    return this.request("DELETE", t, void 0, r);
  }
  /**
   * HTTP PATCH method
   */
  async patch(t, r, n = {}) {
    return this.request("PATCH", t, r, n);
  }
  /**
   * Batch request processing
   */
  async batch(t) {
    if (t.length > lr.MAX_CACHE_SIZE)
      throw new Se(`Batch size exceeds limit of ${lr.MAX_CACHE_SIZE}`);
    const r = t.map(async (n) => {
      try {
        const a = await this.request(
          n.method,
          n.url,
          n.body,
          n.options
        );
        return {
          id: n.id,
          success: a.success,
          data: a.data,
          status: a.status
        };
      } catch (a) {
        const l = a instanceof Se ? a : new Se(a.message);
        return {
          id: n.id,
          success: !1,
          error: l.message,
          status: l.status
        };
      }
    });
    return Promise.all(r);
  }
  /**
   * Core request method with all functionality
   */
  async request(t, r, n, a = {}) {
    var f, m;
    const l = a.requestId || this.generateRequestId(), d = Date.now();
    let c = {
      url: this.buildUrl(r),
      method: t,
      headers: { ...this.config.defaultHeaders },
      body: n,
      timeout: a.timeout || this.config.timeout,
      retries: a.retries !== void 0 ? a.retries : this.config.retryAttempts,
      validateSchema: !!a.validateSchema,
      requiresAuth: !1,
      abortSignal: a.abortSignal,
      metadata: {
        requestId: l,
        timestamp: d,
        priority: a.priority || "normal"
      }
    };
    try {
      for (const N of this.requestInterceptors)
        c = await N(c);
      if (c.validateSchema && await this.validateSchemaAccess(r, t, a.validateSchema || !0), t === "GET" && this.config.enableCache && a.cacheable !== !1) {
        const N = this.generateCacheKey(c.url, c.headers), S = this.cache.get(N);
        if (S)
          return {
            ...S,
            meta: {
              ...S.meta,
              cached: !0,
              fromCache: !0,
              requestId: l,
              timestamp: ((f = S.meta) == null ? void 0 : f.timestamp) || Date.now()
            }
          };
      }
      const h = `${t}:${c.url}:${JSON.stringify(n)}`, y = await this.requestQueue.getOrCreate(
        h,
        () => this.executeRequest(c)
      );
      if (t === "GET" && this.config.enableCache && a.cacheable !== !1 && y.success) {
        const N = this.generateCacheKey(c.url, c.headers), S = a.cacheTtl || lr.DEFAULT_TTL_MS;
        this.cache.set(N, y, S);
      }
      let x = y;
      for (const N of this.responseInterceptors)
        x = await N(x);
      return this.config.enableMetrics && this.recordMetrics({
        requestId: l,
        url: c.url,
        method: t,
        startTime: d,
        endTime: Date.now(),
        duration: Date.now() - d,
        status: y.status,
        cached: ((m = y.meta) == null ? void 0 : m.cached) || !1
      }), x;
    } catch (h) {
      let y = h instanceof Se ? h : nr.fromNetworkError(h, l);
      for (const x of this.errorInterceptors)
        y = await x(y);
      throw this.config.enableMetrics && this.recordMetrics({
        requestId: l,
        url: c.url,
        method: t,
        startTime: d,
        endTime: Date.now(),
        duration: Date.now() - d,
        error: y.message
      }), y;
    }
  }
  /**
   * Execute the actual HTTP request with retry logic
   */
  async executeRequest(t) {
    let r;
    for (let n = 0; n <= t.retries; n++)
      try {
        return await this.performRequest(t);
      } catch (a) {
        if (r = a instanceof Se ? a : nr.fromNetworkError(a, t.metadata.requestId), n === t.retries || !lc(r))
          break;
        const l = Math.min(
          this.config.retryDelay * Math.pow(Ps.EXPONENTIAL_BACKOFF_MULTIPLIER, n),
          Ps.MAX_RETRY_DELAY_MS
        );
        await this.sleep(l);
      }
    throw r;
  }
  /**
   * Perform the actual HTTP request
   */
  async performRequest(t) {
    const r = new AbortController(), n = setTimeout(() => r.abort(), t.timeout);
    try {
      const a = { ...t.headers };
      t.body && !a[Tr.CONTENT_TYPE] && (a[Tr.CONTENT_TYPE] = ps.JSON), a[Tr.REQUEST_ID] = t.metadata.requestId;
      const l = {
        method: t.method,
        headers: a,
        signal: t.abortSignal || r.signal
      };
      t.body && t.method !== "GET" && (l.body = this.serializeBody(t.body, a[Tr.CONTENT_TYPE]));
      const d = await fetch(t.url, l);
      return clearTimeout(n), await this.handleResponse(d, t.metadata.requestId);
    } catch (a) {
      throw clearTimeout(n), a.name === "AbortError" ? nr.fromTimeout(t.timeout, t.metadata.requestId) : nr.fromNetworkError(a, t.metadata.requestId);
    }
  }
  /**
   * Handle HTTP response and convert to standardized format
   */
  async handleResponse(t, r) {
    if (!t.ok)
      throw await nr.fromResponse(t, r);
    let n;
    const a = t.headers.get("content-type");
    try {
      a != null && a.includes("application/json") ? n = await t.json() : n = await t.text();
    } catch {
      throw new Se("Failed to parse response", t.status, { requestId: r });
    }
    return {
      success: !0,
      data: n,
      status: t.status,
      headers: this.extractHeaders(t.headers),
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
  async addAuthHeaders(t, r) {
  }
  /**
   * Validate schema access according to SCHEMA-002 rules
   */
  async validateSchemaAccess(t, r, n) {
    const a = t.match(/\/schemas\/([^\/]+)/);
    if (!a) return;
    const l = a[1], d = typeof n == "boolean" ? {} : n;
    if ((t.includes("/mutation") || t.includes("/query")) && d.requiresApproved !== !1) {
      const c = ni.getState().schemas, m = Object.values(c.schemas || {}).find((h) => h.name === l);
      if (!m || m.state !== De.APPROVED)
        throw new es(
          l,
          (m == null ? void 0 : m.state) || "unknown",
          oc.MUTATION
        );
    }
  }
  /**
   * Serialize request body based on content type
   */
  serializeBody(t, r) {
    return r === ps.JSON ? JSON.stringify(t) : r === ps.FORM_DATA ? t : String(t);
  }
  /**
   * Extract response headers as plain object
   */
  extractHeaders(t) {
    const r = {};
    return t.forEach((n, a) => {
      r[a] = n;
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
  generateCacheKey(t, r) {
    const n = Object.keys(r).filter((a) => !a.startsWith("X-Request")).sort().map((a) => `${a}:${r[a]}`).join(";");
    return `${t}|${n}`;
  }
  /**
   * Build full URL from endpoint
   */
  buildUrl(t) {
    return t.startsWith("http") ? t : `${this.config.baseUrl}${t.startsWith("/") ? "" : "/"}${t}`;
  }
  /**
   * Sleep utility for retry delays
   */
  sleep(t) {
    return new Promise((r) => setTimeout(r, t));
  }
  /**
   * Record request metrics
   */
  recordMetrics(t) {
    this.metrics.push(t), this.metrics.length > 1e3 && this.metrics.splice(0, this.metrics.length - 1e3);
  }
  // Interceptor management methods
  addRequestInterceptor(t) {
    this.requestInterceptors.push(t);
  }
  addResponseInterceptor(t) {
    this.responseInterceptors.push(t);
  }
  addErrorInterceptor(t) {
    this.errorInterceptors.push(t);
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
const fc = new Pa();
function We(e) {
  return new Pa(e);
}
class hc {
  constructor(t) {
    this.client = t || We({
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
  async getLogs() {
    return this.client.get(te.LIST_LOGS, {
      requiresAuth: !1,
      // Logs are public for monitoring
      timeout: we.STANDARD,
      retries: Ee.STANDARD,
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
  async resetDatabase(t = !1) {
    if (!t)
      throw new Error("Database reset requires explicit confirmation");
    const r = { confirm: t };
    return this.client.post(
      te.RESET_DATABASE,
      r,
      {
        timeout: we.DESTRUCTIVE_OPERATIONS,
        // Longer timeout for database operations
        retries: Ee.NONE,
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
    return this.client.get(te.GET_SYSTEM_STATUS, {
      requiresAuth: !1,
      // Status is public for monitoring
      timeout: we.QUICK,
      retries: Ee.CRITICAL,
      // Multiple retries for critical system data
      cacheable: !0,
      cacheTtl: Nt.SYSTEM_STATUS,
      // Cache for 30 seconds
      cacheKey: Vr.SYSTEM_STATUS
    });
  }
  /**
   * Get the node's private key
   * UNPROTECTED - No authentication required for UI access
   * 
   * @returns Promise resolving to private key response
   */
  async getNodePrivateKey() {
    return this.client.get(te.GET_NODE_PRIVATE_KEY, {
      requiresAuth: !1,
      // No authentication required for UI access
      timeout: we.STANDARD,
      retries: Ee.STANDARD,
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
    return this.client.get(te.GET_NODE_PUBLIC_KEY, {
      requiresAuth: !1,
      // Public key is safe to share
      timeout: we.QUICK,
      retries: Ee.STANDARD,
      cacheable: !0,
      cacheTtl: Nt.SYSTEM_STATUS,
      // Cache for 30 seconds
      cacheKey: Vr.SYSTEM_PUBLIC_KEY
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
  createLogStream(t, r) {
    const n = te.STREAM_LOGS, a = n.startsWith("http") ? n : `${Xr.BASE_URL}${n.startsWith("/") ? "" : "/"}${n}`, l = new EventSource(a);
    return l.onmessage = (d) => {
      t(d.data);
    }, r && (l.onerror = r), l;
  }
  /**
   * Validate reset database request
   * Client-side validation helper
   * 
   * @param request - Reset request to validate
   * @returns Validation result
   */
  validateResetRequest(t) {
    const r = [];
    return typeof t != "object" || t === null ? (r.push("Request must be an object"), { isValid: !1, errors: r }) : (typeof t.confirm != "boolean" ? r.push("Confirm must be a boolean value") : t.confirm || r.push("Confirm must be true to proceed with database reset"), {
      isValid: r.length === 0,
      errors: r
    });
  }
  /**
   * Get API metrics for system operations
   */
  getMetrics() {
    return this.client.getMetrics().filter(
      (t) => t.url.includes("/system") || t.url.includes("/logs")
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
      timeout: we.STANDARD,
      retries: Ee.STANDARD,
      cacheable: !0,
      cacheTtl: Nt.SYSTEM_STATUS,
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
  async updateDatabaseConfig(t) {
    const r = { database: t };
    return this.client.post(
      "/system/database-config",
      r,
      {
        timeout: we.STANDARD,
        retries: Ee.NONE,
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
const me = new hc();
me.getLogs.bind(me);
me.resetDatabase.bind(me);
me.getSystemStatus.bind(me);
const en = me.getNodePrivateKey.bind(me);
me.getNodePublicKey.bind(me);
const mc = me.getDatabaseConfig.bind(me), pc = me.updateDatabaseConfig.bind(me);
me.createLogStream.bind(me);
me.validateResetRequest.bind(me);
/*! noble-ed25519 - MIT License (c) 2019 Paul Miller (paulmillr.com) */
const gc = {
  p: 0x7fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffedn,
  n: 0x1000000000000000000000000000000014def9dea2f79cd65812631a5cf5d3edn,
  a: 0x7fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffecn,
  d: 0x52036cee2b6ffe738cc740797779e89800700a4d4141d8ab75eb4dca135978a3n,
  Gx: 0x216936d3cd6e53fec0a4e231fdd6dc5c692cc7609525a7b2c9562d608f25d51an,
  Gy: 0x6666666666666666666666666666666666666666666666666666666666666658n
}, { p: je, n: Pr, Gx: Bn, Gy: Ln, a: gs, d: ys } = gc, yc = 8n, mr = 32, Ma = 64, Ve = (e = "") => {
  throw new Error(e);
}, xc = (e) => typeof e == "bigint", Ba = (e) => typeof e == "string", bc = (e) => e instanceof Uint8Array || ArrayBuffer.isView(e) && e.constructor.name === "Uint8Array", Wt = (e, t) => !bc(e) || typeof t == "number" && t > 0 && e.length !== t ? Ve("Uint8Array expected") : e, ts = (e) => new Uint8Array(e), tn = (e) => Uint8Array.from(e), La = (e, t) => e.toString(16).padStart(t, "0"), rn = (e) => Array.from(Wt(e)).map((t) => La(t, 2)).join(""), dt = { _0: 48, _9: 57, A: 65, F: 70, a: 97, f: 102 }, $n = (e) => {
  if (e >= dt._0 && e <= dt._9)
    return e - dt._0;
  if (e >= dt.A && e <= dt.F)
    return e - (dt.A - 10);
  if (e >= dt.a && e <= dt.f)
    return e - (dt.a - 10);
}, sn = (e) => {
  const t = "hex invalid";
  if (!Ba(e))
    return Ve(t);
  const r = e.length, n = r / 2;
  if (r % 2)
    return Ve(t);
  const a = ts(n);
  for (let l = 0, d = 0; l < n; l++, d += 2) {
    const c = $n(e.charCodeAt(d)), f = $n(e.charCodeAt(d + 1));
    if (c === void 0 || f === void 0)
      return Ve(t);
    a[l] = c * 16 + f;
  }
  return a;
}, $a = (e, t) => Wt(Ba(e) ? sn(e) : tn(Wt(e)), t), Ua = () => globalThis == null ? void 0 : globalThis.crypto, vc = () => {
  var e;
  return ((e = Ua()) == null ? void 0 : e.subtle) ?? Ve("crypto.subtle must be defined");
}, Un = (...e) => {
  const t = ts(e.reduce((n, a) => n + Wt(a).length, 0));
  let r = 0;
  return e.forEach((n) => {
    t.set(n, r), r += n.length;
  }), t;
}, wc = (e = mr) => Ua().getRandomValues(ts(e)), Hr = BigInt, Ct = (e, t, r, n = "bad number: out of range") => xc(e) && t <= e && e < r ? e : Ve(n), K = (e, t = je) => {
  const r = e % t;
  return r >= 0n ? r : t + r;
}, Ec = (e) => K(e, Pr), Ka = (e, t) => {
  (e === 0n || t <= 0n) && Ve("no inverse n=" + e + " mod=" + t);
  let r = K(e, t), n = t, a = 0n, l = 1n;
  for (; r !== 0n; ) {
    const d = n / r, c = n % r, f = a - l * d;
    n = r, r = c, a = l, l = f;
  }
  return n === 1n ? K(a, t) : Ve("no inverse");
}, Kn = (e) => e instanceof Ot ? e : Ve("Point expected"), Ms = 2n ** 256n, Xe = class Xe {
  constructor(t, r, n, a) {
    lt(this, "ex");
    lt(this, "ey");
    lt(this, "ez");
    lt(this, "et");
    const l = Ms;
    this.ex = Ct(t, 0n, l), this.ey = Ct(r, 0n, l), this.ez = Ct(n, 1n, l), this.et = Ct(a, 0n, l), Object.freeze(this);
  }
  static fromAffine(t) {
    return new Xe(t.x, t.y, 1n, K(t.x * t.y));
  }
  /** RFC8032 5.1.3: Uint8Array to Point. */
  static fromBytes(t, r = !1) {
    const n = ys, a = tn(Wt(t, mr)), l = t[31];
    a[31] = l & -129;
    const d = Va(a);
    Ct(d, 0n, r ? Ms : je);
    const f = K(d * d), m = K(f - 1n), h = K(n * f + 1n);
    let { isValid: y, value: x } = Sc(m, h);
    y || Ve("bad point: y not sqrt");
    const N = (x & 1n) === 1n, S = (l & 128) !== 0;
    return !r && x === 0n && S && Ve("bad point: x==0, isLastByteOdd"), S !== N && (x = K(-x)), new Xe(x, d, 1n, K(x * d));
  }
  /** Checks if the point is valid and on-curve. */
  assertValidity() {
    const t = gs, r = ys, n = this;
    if (n.is0())
      throw new Error("bad point: ZERO");
    const { ex: a, ey: l, ez: d, et: c } = n, f = K(a * a), m = K(l * l), h = K(d * d), y = K(h * h), x = K(f * t), N = K(h * K(x + m)), S = K(y + K(r * K(f * m)));
    if (N !== S)
      throw new Error("bad point: equation left != right (1)");
    const E = K(a * l), p = K(d * c);
    if (E !== p)
      throw new Error("bad point: equation left != right (2)");
    return this;
  }
  /** Equality check: compare points P&Q. */
  equals(t) {
    const { ex: r, ey: n, ez: a } = this, { ex: l, ey: d, ez: c } = Kn(t), f = K(r * c), m = K(l * a), h = K(n * c), y = K(d * a);
    return f === m && h === y;
  }
  is0() {
    return this.equals(Ht);
  }
  /** Flip point over y coordinate. */
  negate() {
    return new Xe(K(-this.ex), this.ey, this.ez, K(-this.et));
  }
  /** Point doubling. Complete formula. Cost: `4M + 4S + 1*a + 6add + 1*2`. */
  double() {
    const { ex: t, ey: r, ez: n } = this, a = gs, l = K(t * t), d = K(r * r), c = K(2n * K(n * n)), f = K(a * l), m = t + r, h = K(K(m * m) - l - d), y = f + d, x = y - c, N = f - d, S = K(h * x), E = K(y * N), p = K(h * N), v = K(x * y);
    return new Xe(S, E, v, p);
  }
  /** Point addition. Complete formula. Cost: `8M + 1*k + 8add + 1*2`. */
  add(t) {
    const { ex: r, ey: n, ez: a, et: l } = this, { ex: d, ey: c, ez: f, et: m } = Kn(t), h = gs, y = ys, x = K(r * d), N = K(n * c), S = K(l * y * m), E = K(a * f), p = K((r + n) * (d + c) - x - N), v = K(E - S), w = K(E + S), A = K(N - h * x), _ = K(p * v), T = K(w * A), M = K(p * A), R = K(v * w);
    return new Xe(_, T, R, M);
  }
  /**
   * Point-by-scalar multiplication. Scalar must be in range 1 <= n < CURVE.n.
   * Uses {@link wNAF} for base point.
   * Uses fake point to mitigate side-channel leakage.
   * @param n scalar by which point is multiplied
   * @param safe safe mode guards against timing attacks; unsafe mode is faster
   */
  multiply(t, r = !0) {
    if (!r && (t === 0n || this.is0()))
      return Ht;
    if (Ct(t, 1n, Pr), t === 1n)
      return this;
    if (this.equals(Yt))
      return Ic(t).p;
    let n = Ht, a = Yt;
    for (let l = this; t > 0n; l = l.double(), t >>= 1n)
      t & 1n ? n = n.add(l) : r && (a = a.add(l));
    return n;
  }
  /** Convert point to 2d xy affine point. (X, Y, Z) ∋ (x=X/Z, y=Y/Z) */
  toAffine() {
    const { ex: t, ey: r, ez: n } = this;
    if (this.equals(Ht))
      return { x: 0n, y: 1n };
    const a = Ka(n, je);
    return K(n * a) !== 1n && Ve("invalid inverse"), { x: K(t * a), y: K(r * a) };
  }
  toBytes() {
    const { x: t, y: r } = this.assertValidity().toAffine(), n = Nc(r);
    return n[31] |= t & 1n ? 128 : 0, n;
  }
  toHex() {
    return rn(this.toBytes());
  }
  // encode to hex string
  clearCofactor() {
    return this.multiply(Hr(yc), !1);
  }
  isSmallOrder() {
    return this.clearCofactor().is0();
  }
  isTorsionFree() {
    let t = this.multiply(Pr / 2n, !1).double();
    return Pr % 2n && (t = t.add(this)), t.is0();
  }
  static fromHex(t, r) {
    return Xe.fromBytes($a(t), r);
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
lt(Xe, "BASE"), lt(Xe, "ZERO");
let Ot = Xe;
const Yt = new Ot(Bn, Ln, 1n, K(Bn * Ln)), Ht = new Ot(0n, 1n, 1n, 0n);
Ot.BASE = Yt;
Ot.ZERO = Ht;
const Nc = (e) => sn(La(Ct(e, 0n, Ms), Ma)).reverse(), Va = (e) => Hr("0x" + rn(tn(Wt(e)).reverse())), Ze = (e, t) => {
  let r = e;
  for (; t-- > 0n; )
    r *= r, r %= je;
  return r;
}, jc = (e) => {
  const r = e * e % je * e % je, n = Ze(r, 2n) * r % je, a = Ze(n, 1n) * e % je, l = Ze(a, 5n) * a % je, d = Ze(l, 10n) * l % je, c = Ze(d, 20n) * d % je, f = Ze(c, 40n) * c % je, m = Ze(f, 80n) * f % je, h = Ze(m, 80n) * f % je, y = Ze(h, 10n) * l % je;
  return { pow_p_5_8: Ze(y, 2n) * e % je, b2: r };
}, Vn = 0x2b8324804fc1df0b2b4d00993dfbd7a72f431806ad2fe478c4ee1b274a0ea0b0n, Sc = (e, t) => {
  const r = K(t * t * t), n = K(r * r * t), a = jc(e * n).pow_p_5_8;
  let l = K(e * r * a);
  const d = K(t * l * l), c = l, f = K(l * Vn), m = d === e, h = d === K(-e), y = d === K(-e * Vn);
  return m && (l = c), (h || y) && (l = f), (K(l) & 1n) === 1n && (l = K(-l)), { isValid: m || h, value: l };
}, _c = (e) => Ec(Va(e)), Ac = (...e) => Bs.sha512Async(...e), Tc = (e) => {
  const t = e.slice(0, mr);
  t[0] &= 248, t[31] &= 127, t[31] |= 64;
  const r = e.slice(mr, Ma), n = _c(t), a = Yt.multiply(n), l = a.toBytes();
  return { head: t, prefix: r, scalar: n, point: a, pointBytes: l };
}, Cc = (e) => Ac($a(e, mr)).then(Tc), nn = (e) => Cc(e).then((t) => t.pointBytes), Bs = {
  sha512Async: async (...e) => {
    const t = vc(), r = Un(...e);
    return ts(await t.digest("SHA-512", r.buffer));
  },
  sha512Sync: void 0,
  bytesToHex: rn,
  hexToBytes: sn,
  concatBytes: Un,
  mod: K,
  invert: Ka,
  randomBytes: wc
}, zr = 8, Rc = 256, Ha = Math.ceil(Rc / zr) + 1, Ls = 2 ** (zr - 1), kc = () => {
  const e = [];
  let t = Yt, r = t;
  for (let n = 0; n < Ha; n++) {
    r = t, e.push(r);
    for (let a = 1; a < Ls; a++)
      r = r.add(t), e.push(r);
    t = r.double();
  }
  return e;
};
let Hn;
const zn = (e, t) => {
  const r = t.negate();
  return e ? r : t;
}, Ic = (e) => {
  const t = Hn || (Hn = kc());
  let r = Ht, n = Yt;
  const a = 2 ** zr, l = a, d = Hr(a - 1), c = Hr(zr);
  for (let f = 0; f < Ha; f++) {
    let m = Number(e & d);
    e >>= c, m > Ls && (m -= l, e += 1n);
    const h = f * Ls, y = h, x = h + Math.abs(m) - 1, N = f % 2 !== 0, S = m < 0;
    m === 0 ? n = n.add(zn(N, t[y])) : r = r.add(zn(S, t[x]));
  }
  return { p: r, f: n };
};
/*! noble-hashes - MIT License (c) 2022 Paul Miller (paulmillr.com) */
function Oc(e) {
  return e instanceof Uint8Array || ArrayBuffer.isView(e) && e.constructor.name === "Uint8Array";
}
function an(e, ...t) {
  if (!Oc(e))
    throw new Error("Uint8Array expected");
  if (t.length > 0 && !t.includes(e.length))
    throw new Error("Uint8Array expected of length " + t + ", got length=" + e.length);
}
function Gn(e, t = !0) {
  if (e.destroyed)
    throw new Error("Hash instance has been destroyed");
  if (t && e.finished)
    throw new Error("Hash#digest() has already been called");
}
function Dc(e, t) {
  an(e);
  const r = t.outputLen;
  if (e.length < r)
    throw new Error("digestInto() expects output buffer of length at least " + r);
}
function $s(...e) {
  for (let t = 0; t < e.length; t++)
    e[t].fill(0);
}
function xs(e) {
  return new DataView(e.buffer, e.byteOffset, e.byteLength);
}
function Fc(e) {
  if (typeof e != "string")
    throw new Error("string expected");
  return new Uint8Array(new TextEncoder().encode(e));
}
function za(e) {
  return typeof e == "string" && (e = Fc(e)), an(e), e;
}
class Pc {
}
function Mc(e) {
  const t = (n) => e().update(za(n)).digest(), r = e();
  return t.outputLen = r.outputLen, t.blockLen = r.blockLen, t.create = () => e(), t;
}
function Bc(e, t, r, n) {
  if (typeof e.setBigUint64 == "function")
    return e.setBigUint64(t, r, n);
  const a = BigInt(32), l = BigInt(4294967295), d = Number(r >> a & l), c = Number(r & l), f = n ? 4 : 0, m = n ? 0 : 4;
  e.setUint32(t + f, d, n), e.setUint32(t + m, c, n);
}
class Lc extends Pc {
  constructor(t, r, n, a) {
    super(), this.finished = !1, this.length = 0, this.pos = 0, this.destroyed = !1, this.blockLen = t, this.outputLen = r, this.padOffset = n, this.isLE = a, this.buffer = new Uint8Array(t), this.view = xs(this.buffer);
  }
  update(t) {
    Gn(this), t = za(t), an(t);
    const { view: r, buffer: n, blockLen: a } = this, l = t.length;
    for (let d = 0; d < l; ) {
      const c = Math.min(a - this.pos, l - d);
      if (c === a) {
        const f = xs(t);
        for (; a <= l - d; d += a)
          this.process(f, d);
        continue;
      }
      n.set(t.subarray(d, d + c), this.pos), this.pos += c, d += c, this.pos === a && (this.process(r, 0), this.pos = 0);
    }
    return this.length += t.length, this.roundClean(), this;
  }
  digestInto(t) {
    Gn(this), Dc(t, this), this.finished = !0;
    const { buffer: r, view: n, blockLen: a, isLE: l } = this;
    let { pos: d } = this;
    r[d++] = 128, $s(this.buffer.subarray(d)), this.padOffset > a - d && (this.process(n, 0), d = 0);
    for (let y = d; y < a; y++)
      r[y] = 0;
    Bc(n, a - 8, BigInt(this.length * 8), l), this.process(n, 0);
    const c = xs(t), f = this.outputLen;
    if (f % 4)
      throw new Error("_sha2: outputLen should be aligned to 32bit");
    const m = f / 4, h = this.get();
    if (m > h.length)
      throw new Error("_sha2: outputLen bigger than state");
    for (let y = 0; y < m; y++)
      c.setUint32(4 * y, h[y], l);
  }
  digest() {
    const { buffer: t, outputLen: r } = this;
    this.digestInto(t);
    const n = t.slice(0, r);
    return this.destroy(), n;
  }
  _cloneInto(t) {
    t || (t = new this.constructor()), t.set(...this.get());
    const { blockLen: r, buffer: n, length: a, finished: l, destroyed: d, pos: c } = this;
    return t.destroyed = d, t.finished = l, t.length = a, t.pos = c, a % r && t.buffer.set(n), t;
  }
  clone() {
    return this._cloneInto();
  }
}
const Ne = /* @__PURE__ */ Uint32Array.from([
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
]), Cr = /* @__PURE__ */ BigInt(2 ** 32 - 1), qn = /* @__PURE__ */ BigInt(32);
function $c(e, t = !1) {
  return t ? { h: Number(e & Cr), l: Number(e >> qn & Cr) } : { h: Number(e >> qn & Cr) | 0, l: Number(e & Cr) | 0 };
}
function Uc(e, t = !1) {
  const r = e.length;
  let n = new Uint32Array(r), a = new Uint32Array(r);
  for (let l = 0; l < r; l++) {
    const { h: d, l: c } = $c(e[l], t);
    [n[l], a[l]] = [d, c];
  }
  return [n, a];
}
const Wn = (e, t, r) => e >>> r, Yn = (e, t, r) => e << 32 - r | t >>> r, Bt = (e, t, r) => e >>> r | t << 32 - r, Lt = (e, t, r) => e << 32 - r | t >>> r, Rr = (e, t, r) => e << 64 - r | t >>> r - 32, kr = (e, t, r) => e >>> r - 32 | t << 64 - r;
function ut(e, t, r, n) {
  const a = (t >>> 0) + (n >>> 0);
  return { h: e + r + (a / 2 ** 32 | 0) | 0, l: a | 0 };
}
const Kc = (e, t, r) => (e >>> 0) + (t >>> 0) + (r >>> 0), Vc = (e, t, r, n) => t + r + n + (e / 2 ** 32 | 0) | 0, Hc = (e, t, r, n) => (e >>> 0) + (t >>> 0) + (r >>> 0) + (n >>> 0), zc = (e, t, r, n, a) => t + r + n + a + (e / 2 ** 32 | 0) | 0, Gc = (e, t, r, n, a) => (e >>> 0) + (t >>> 0) + (r >>> 0) + (n >>> 0) + (a >>> 0), qc = (e, t, r, n, a, l) => t + r + n + a + l + (e / 2 ** 32 | 0) | 0, Ga = Uc([
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
].map((e) => BigInt(e))), Wc = Ga[0], Yc = Ga[1], xt = /* @__PURE__ */ new Uint32Array(80), bt = /* @__PURE__ */ new Uint32Array(80);
class Qc extends Lc {
  constructor(t = 64) {
    super(128, t, 16, !1), this.Ah = Ne[0] | 0, this.Al = Ne[1] | 0, this.Bh = Ne[2] | 0, this.Bl = Ne[3] | 0, this.Ch = Ne[4] | 0, this.Cl = Ne[5] | 0, this.Dh = Ne[6] | 0, this.Dl = Ne[7] | 0, this.Eh = Ne[8] | 0, this.El = Ne[9] | 0, this.Fh = Ne[10] | 0, this.Fl = Ne[11] | 0, this.Gh = Ne[12] | 0, this.Gl = Ne[13] | 0, this.Hh = Ne[14] | 0, this.Hl = Ne[15] | 0;
  }
  // prettier-ignore
  get() {
    const { Ah: t, Al: r, Bh: n, Bl: a, Ch: l, Cl: d, Dh: c, Dl: f, Eh: m, El: h, Fh: y, Fl: x, Gh: N, Gl: S, Hh: E, Hl: p } = this;
    return [t, r, n, a, l, d, c, f, m, h, y, x, N, S, E, p];
  }
  // prettier-ignore
  set(t, r, n, a, l, d, c, f, m, h, y, x, N, S, E, p) {
    this.Ah = t | 0, this.Al = r | 0, this.Bh = n | 0, this.Bl = a | 0, this.Ch = l | 0, this.Cl = d | 0, this.Dh = c | 0, this.Dl = f | 0, this.Eh = m | 0, this.El = h | 0, this.Fh = y | 0, this.Fl = x | 0, this.Gh = N | 0, this.Gl = S | 0, this.Hh = E | 0, this.Hl = p | 0;
  }
  process(t, r) {
    for (let A = 0; A < 16; A++, r += 4)
      xt[A] = t.getUint32(r), bt[A] = t.getUint32(r += 4);
    for (let A = 16; A < 80; A++) {
      const _ = xt[A - 15] | 0, T = bt[A - 15] | 0, M = Bt(_, T, 1) ^ Bt(_, T, 8) ^ Wn(_, T, 7), R = Lt(_, T, 1) ^ Lt(_, T, 8) ^ Yn(_, T, 7), k = xt[A - 2] | 0, I = bt[A - 2] | 0, $ = Bt(k, I, 19) ^ Rr(k, I, 61) ^ Wn(k, I, 6), F = Lt(k, I, 19) ^ kr(k, I, 61) ^ Yn(k, I, 6), z = Hc(R, F, bt[A - 7], bt[A - 16]), V = zc(z, M, $, xt[A - 7], xt[A - 16]);
      xt[A] = V | 0, bt[A] = z | 0;
    }
    let { Ah: n, Al: a, Bh: l, Bl: d, Ch: c, Cl: f, Dh: m, Dl: h, Eh: y, El: x, Fh: N, Fl: S, Gh: E, Gl: p, Hh: v, Hl: w } = this;
    for (let A = 0; A < 80; A++) {
      const _ = Bt(y, x, 14) ^ Bt(y, x, 18) ^ Rr(y, x, 41), T = Lt(y, x, 14) ^ Lt(y, x, 18) ^ kr(y, x, 41), M = y & N ^ ~y & E, R = x & S ^ ~x & p, k = Gc(w, T, R, Yc[A], bt[A]), I = qc(k, v, _, M, Wc[A], xt[A]), $ = k | 0, F = Bt(n, a, 28) ^ Rr(n, a, 34) ^ Rr(n, a, 39), z = Lt(n, a, 28) ^ kr(n, a, 34) ^ kr(n, a, 39), V = n & l ^ n & c ^ l & c, G = a & d ^ a & f ^ d & f;
      v = E | 0, w = p | 0, E = N | 0, p = S | 0, N = y | 0, S = x | 0, { h: y, l: x } = ut(m | 0, h | 0, I | 0, $ | 0), m = c | 0, h = f | 0, c = l | 0, f = d | 0, l = n | 0, d = a | 0;
      const L = Kc($, z, G);
      n = Vc(L, I, F, V), a = L | 0;
    }
    ({ h: n, l: a } = ut(this.Ah | 0, this.Al | 0, n | 0, a | 0)), { h: l, l: d } = ut(this.Bh | 0, this.Bl | 0, l | 0, d | 0), { h: c, l: f } = ut(this.Ch | 0, this.Cl | 0, c | 0, f | 0), { h: m, l: h } = ut(this.Dh | 0, this.Dl | 0, m | 0, h | 0), { h: y, l: x } = ut(this.Eh | 0, this.El | 0, y | 0, x | 0), { h: N, l: S } = ut(this.Fh | 0, this.Fl | 0, N | 0, S | 0), { h: E, l: p } = ut(this.Gh | 0, this.Gl | 0, E | 0, p | 0), { h: v, l: w } = ut(this.Hh | 0, this.Hl | 0, v | 0, w | 0), this.set(n, a, l, d, c, f, m, h, y, x, N, S, E, p, v, w);
  }
  roundClean() {
    $s(xt, bt);
  }
  destroy() {
    $s(this.buffer), this.set(0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0);
  }
}
const Jc = /* @__PURE__ */ Mc(() => new Qc()), Zc = Jc;
var on = {}, rs = {};
rs.byteLength = tl;
rs.toByteArray = sl;
rs.fromByteArray = il;
var et = [], $e = [], Xc = typeof Uint8Array < "u" ? Uint8Array : Array, bs = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
for (var $t = 0, el = bs.length; $t < el; ++$t)
  et[$t] = bs[$t], $e[bs.charCodeAt($t)] = $t;
$e[45] = 62;
$e[95] = 63;
function qa(e) {
  var t = e.length;
  if (t % 4 > 0)
    throw new Error("Invalid string. Length must be a multiple of 4");
  var r = e.indexOf("=");
  r === -1 && (r = t);
  var n = r === t ? 0 : 4 - r % 4;
  return [r, n];
}
function tl(e) {
  var t = qa(e), r = t[0], n = t[1];
  return (r + n) * 3 / 4 - n;
}
function rl(e, t, r) {
  return (t + r) * 3 / 4 - r;
}
function sl(e) {
  var t, r = qa(e), n = r[0], a = r[1], l = new Xc(rl(e, n, a)), d = 0, c = a > 0 ? n - 4 : n, f;
  for (f = 0; f < c; f += 4)
    t = $e[e.charCodeAt(f)] << 18 | $e[e.charCodeAt(f + 1)] << 12 | $e[e.charCodeAt(f + 2)] << 6 | $e[e.charCodeAt(f + 3)], l[d++] = t >> 16 & 255, l[d++] = t >> 8 & 255, l[d++] = t & 255;
  return a === 2 && (t = $e[e.charCodeAt(f)] << 2 | $e[e.charCodeAt(f + 1)] >> 4, l[d++] = t & 255), a === 1 && (t = $e[e.charCodeAt(f)] << 10 | $e[e.charCodeAt(f + 1)] << 4 | $e[e.charCodeAt(f + 2)] >> 2, l[d++] = t >> 8 & 255, l[d++] = t & 255), l;
}
function nl(e) {
  return et[e >> 18 & 63] + et[e >> 12 & 63] + et[e >> 6 & 63] + et[e & 63];
}
function al(e, t, r) {
  for (var n, a = [], l = t; l < r; l += 3)
    n = (e[l] << 16 & 16711680) + (e[l + 1] << 8 & 65280) + (e[l + 2] & 255), a.push(nl(n));
  return a.join("");
}
function il(e) {
  for (var t, r = e.length, n = r % 3, a = [], l = 16383, d = 0, c = r - n; d < c; d += l)
    a.push(al(e, d, d + l > c ? c : d + l));
  return n === 1 ? (t = e[r - 1], a.push(
    et[t >> 2] + et[t << 4 & 63] + "=="
  )) : n === 2 && (t = (e[r - 2] << 8) + e[r - 1], a.push(
    et[t >> 10] + et[t >> 4 & 63] + et[t << 2 & 63] + "="
  )), a.join("");
}
var cn = {};
/*! ieee754. BSD-3-Clause License. Feross Aboukhadijeh <https://feross.org/opensource> */
cn.read = function(e, t, r, n, a) {
  var l, d, c = a * 8 - n - 1, f = (1 << c) - 1, m = f >> 1, h = -7, y = r ? a - 1 : 0, x = r ? -1 : 1, N = e[t + y];
  for (y += x, l = N & (1 << -h) - 1, N >>= -h, h += c; h > 0; l = l * 256 + e[t + y], y += x, h -= 8)
    ;
  for (d = l & (1 << -h) - 1, l >>= -h, h += n; h > 0; d = d * 256 + e[t + y], y += x, h -= 8)
    ;
  if (l === 0)
    l = 1 - m;
  else {
    if (l === f)
      return d ? NaN : (N ? -1 : 1) * (1 / 0);
    d = d + Math.pow(2, n), l = l - m;
  }
  return (N ? -1 : 1) * d * Math.pow(2, l - n);
};
cn.write = function(e, t, r, n, a, l) {
  var d, c, f, m = l * 8 - a - 1, h = (1 << m) - 1, y = h >> 1, x = a === 23 ? Math.pow(2, -24) - Math.pow(2, -77) : 0, N = n ? 0 : l - 1, S = n ? 1 : -1, E = t < 0 || t === 0 && 1 / t < 0 ? 1 : 0;
  for (t = Math.abs(t), isNaN(t) || t === 1 / 0 ? (c = isNaN(t) ? 1 : 0, d = h) : (d = Math.floor(Math.log(t) / Math.LN2), t * (f = Math.pow(2, -d)) < 1 && (d--, f *= 2), d + y >= 1 ? t += x / f : t += x * Math.pow(2, 1 - y), t * f >= 2 && (d++, f /= 2), d + y >= h ? (c = 0, d = h) : d + y >= 1 ? (c = (t * f - 1) * Math.pow(2, a), d = d + y) : (c = t * Math.pow(2, y - 1) * Math.pow(2, a), d = 0)); a >= 8; e[r + N] = c & 255, N += S, c /= 256, a -= 8)
    ;
  for (d = d << a | c, m += a; m > 0; e[r + N] = d & 255, N += S, d /= 256, m -= 8)
    ;
  e[r + N - S] |= E * 128;
};
/*!
 * The buffer module from node.js, for the browser.
 *
 * @author   Feross Aboukhadijeh <https://feross.org>
 * @license  MIT
 */
(function(e) {
  const t = rs, r = cn, n = typeof Symbol == "function" && typeof Symbol.for == "function" ? Symbol.for("nodejs.util.inspect.custom") : null;
  e.Buffer = c, e.SlowBuffer = w, e.INSPECT_MAX_BYTES = 50;
  const a = 2147483647;
  e.kMaxLength = a, c.TYPED_ARRAY_SUPPORT = l(), !c.TYPED_ARRAY_SUPPORT && typeof console < "u" && typeof console.error == "function" && console.error(
    "This browser lacks typed array (Uint8Array) support which is required by `buffer` v5.x. Use `buffer` v4.x if you require old browser support."
  );
  function l() {
    try {
      const u = new Uint8Array(1), i = { foo: function() {
        return 42;
      } };
      return Object.setPrototypeOf(i, Uint8Array.prototype), Object.setPrototypeOf(u, i), u.foo() === 42;
    } catch {
      return !1;
    }
  }
  Object.defineProperty(c.prototype, "parent", {
    enumerable: !0,
    get: function() {
      if (c.isBuffer(this))
        return this.buffer;
    }
  }), Object.defineProperty(c.prototype, "offset", {
    enumerable: !0,
    get: function() {
      if (c.isBuffer(this))
        return this.byteOffset;
    }
  });
  function d(u) {
    if (u > a)
      throw new RangeError('The value "' + u + '" is invalid for option "size"');
    const i = new Uint8Array(u);
    return Object.setPrototypeOf(i, c.prototype), i;
  }
  function c(u, i, o) {
    if (typeof u == "number") {
      if (typeof i == "string")
        throw new TypeError(
          'The "string" argument must be of type string. Received type number'
        );
      return y(u);
    }
    return f(u, i, o);
  }
  c.poolSize = 8192;
  function f(u, i, o) {
    if (typeof u == "string")
      return x(u, i);
    if (ArrayBuffer.isView(u))
      return S(u);
    if (u == null)
      throw new TypeError(
        "The first argument must be one of type string, Buffer, ArrayBuffer, Array, or Array-like Object. Received type " + typeof u
      );
    if (Re(u, ArrayBuffer) || u && Re(u.buffer, ArrayBuffer) || typeof SharedArrayBuffer < "u" && (Re(u, SharedArrayBuffer) || u && Re(u.buffer, SharedArrayBuffer)))
      return E(u, i, o);
    if (typeof u == "number")
      throw new TypeError(
        'The "value" argument must not be of type number. Received type number'
      );
    const g = u.valueOf && u.valueOf();
    if (g != null && g !== u)
      return c.from(g, i, o);
    const j = p(u);
    if (j) return j;
    if (typeof Symbol < "u" && Symbol.toPrimitive != null && typeof u[Symbol.toPrimitive] == "function")
      return c.from(u[Symbol.toPrimitive]("string"), i, o);
    throw new TypeError(
      "The first argument must be one of type string, Buffer, ArrayBuffer, Array, or Array-like Object. Received type " + typeof u
    );
  }
  c.from = function(u, i, o) {
    return f(u, i, o);
  }, Object.setPrototypeOf(c.prototype, Uint8Array.prototype), Object.setPrototypeOf(c, Uint8Array);
  function m(u) {
    if (typeof u != "number")
      throw new TypeError('"size" argument must be of type number');
    if (u < 0)
      throw new RangeError('The value "' + u + '" is invalid for option "size"');
  }
  function h(u, i, o) {
    return m(u), u <= 0 ? d(u) : i !== void 0 ? typeof o == "string" ? d(u).fill(i, o) : d(u).fill(i) : d(u);
  }
  c.alloc = function(u, i, o) {
    return h(u, i, o);
  };
  function y(u) {
    return m(u), d(u < 0 ? 0 : v(u) | 0);
  }
  c.allocUnsafe = function(u) {
    return y(u);
  }, c.allocUnsafeSlow = function(u) {
    return y(u);
  };
  function x(u, i) {
    if ((typeof i != "string" || i === "") && (i = "utf8"), !c.isEncoding(i))
      throw new TypeError("Unknown encoding: " + i);
    const o = A(u, i) | 0;
    let g = d(o);
    const j = g.write(u, i);
    return j !== o && (g = g.slice(0, j)), g;
  }
  function N(u) {
    const i = u.length < 0 ? 0 : v(u.length) | 0, o = d(i);
    for (let g = 0; g < i; g += 1)
      o[g] = u[g] & 255;
    return o;
  }
  function S(u) {
    if (Re(u, Uint8Array)) {
      const i = new Uint8Array(u);
      return E(i.buffer, i.byteOffset, i.byteLength);
    }
    return N(u);
  }
  function E(u, i, o) {
    if (i < 0 || u.byteLength < i)
      throw new RangeError('"offset" is outside of buffer bounds');
    if (u.byteLength < i + (o || 0))
      throw new RangeError('"length" is outside of buffer bounds');
    let g;
    return i === void 0 && o === void 0 ? g = new Uint8Array(u) : o === void 0 ? g = new Uint8Array(u, i) : g = new Uint8Array(u, i, o), Object.setPrototypeOf(g, c.prototype), g;
  }
  function p(u) {
    if (c.isBuffer(u)) {
      const i = v(u.length) | 0, o = d(i);
      return o.length === 0 || u.copy(o, 0, 0, i), o;
    }
    if (u.length !== void 0)
      return typeof u.length != "number" || tr(u.length) ? d(0) : N(u);
    if (u.type === "Buffer" && Array.isArray(u.data))
      return N(u.data);
  }
  function v(u) {
    if (u >= a)
      throw new RangeError("Attempt to allocate Buffer larger than maximum size: 0x" + a.toString(16) + " bytes");
    return u | 0;
  }
  function w(u) {
    return +u != u && (u = 0), c.alloc(+u);
  }
  c.isBuffer = function(i) {
    return i != null && i._isBuffer === !0 && i !== c.prototype;
  }, c.compare = function(i, o) {
    if (Re(i, Uint8Array) && (i = c.from(i, i.offset, i.byteLength)), Re(o, Uint8Array) && (o = c.from(o, o.offset, o.byteLength)), !c.isBuffer(i) || !c.isBuffer(o))
      throw new TypeError(
        'The "buf1", "buf2" arguments must be one of type Buffer or Uint8Array'
      );
    if (i === o) return 0;
    let g = i.length, j = o.length;
    for (let C = 0, O = Math.min(g, j); C < O; ++C)
      if (i[C] !== o[C]) {
        g = i[C], j = o[C];
        break;
      }
    return g < j ? -1 : j < g ? 1 : 0;
  }, c.isEncoding = function(i) {
    switch (String(i).toLowerCase()) {
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
  }, c.concat = function(i, o) {
    if (!Array.isArray(i))
      throw new TypeError('"list" argument must be an Array of Buffers');
    if (i.length === 0)
      return c.alloc(0);
    let g;
    if (o === void 0)
      for (o = 0, g = 0; g < i.length; ++g)
        o += i[g].length;
    const j = c.allocUnsafe(o);
    let C = 0;
    for (g = 0; g < i.length; ++g) {
      let O = i[g];
      if (Re(O, Uint8Array))
        C + O.length > j.length ? (c.isBuffer(O) || (O = c.from(O)), O.copy(j, C)) : Uint8Array.prototype.set.call(
          j,
          O,
          C
        );
      else if (c.isBuffer(O))
        O.copy(j, C);
      else
        throw new TypeError('"list" argument must be an Array of Buffers');
      C += O.length;
    }
    return j;
  };
  function A(u, i) {
    if (c.isBuffer(u))
      return u.length;
    if (ArrayBuffer.isView(u) || Re(u, ArrayBuffer))
      return u.byteLength;
    if (typeof u != "string")
      throw new TypeError(
        'The "string" argument must be one of type string, Buffer, or ArrayBuffer. Received type ' + typeof u
      );
    const o = u.length, g = arguments.length > 2 && arguments[2] === !0;
    if (!g && o === 0) return 0;
    let j = !1;
    for (; ; )
      switch (i) {
        case "ascii":
        case "latin1":
        case "binary":
          return o;
        case "utf8":
        case "utf-8":
          return B(u).length;
        case "ucs2":
        case "ucs-2":
        case "utf16le":
        case "utf-16le":
          return o * 2;
        case "hex":
          return o >>> 1;
        case "base64":
          return Je(u).length;
        default:
          if (j)
            return g ? -1 : B(u).length;
          i = ("" + i).toLowerCase(), j = !0;
      }
  }
  c.byteLength = A;
  function _(u, i, o) {
    let g = !1;
    if ((i === void 0 || i < 0) && (i = 0), i > this.length || ((o === void 0 || o > this.length) && (o = this.length), o <= 0) || (o >>>= 0, i >>>= 0, o <= i))
      return "";
    for (u || (u = "utf8"); ; )
      switch (u) {
        case "hex":
          return Me(this, i, o);
        case "utf8":
        case "utf-8":
          return G(this, i, o);
        case "ascii":
          return Q(this, i, o);
        case "latin1":
        case "binary":
          return ge(this, i, o);
        case "base64":
          return V(this, i, o);
        case "ucs2":
        case "ucs-2":
        case "utf16le":
        case "utf-16le":
          return ze(this, i, o);
        default:
          if (g) throw new TypeError("Unknown encoding: " + u);
          u = (u + "").toLowerCase(), g = !0;
      }
  }
  c.prototype._isBuffer = !0;
  function T(u, i, o) {
    const g = u[i];
    u[i] = u[o], u[o] = g;
  }
  c.prototype.swap16 = function() {
    const i = this.length;
    if (i % 2 !== 0)
      throw new RangeError("Buffer size must be a multiple of 16-bits");
    for (let o = 0; o < i; o += 2)
      T(this, o, o + 1);
    return this;
  }, c.prototype.swap32 = function() {
    const i = this.length;
    if (i % 4 !== 0)
      throw new RangeError("Buffer size must be a multiple of 32-bits");
    for (let o = 0; o < i; o += 4)
      T(this, o, o + 3), T(this, o + 1, o + 2);
    return this;
  }, c.prototype.swap64 = function() {
    const i = this.length;
    if (i % 8 !== 0)
      throw new RangeError("Buffer size must be a multiple of 64-bits");
    for (let o = 0; o < i; o += 8)
      T(this, o, o + 7), T(this, o + 1, o + 6), T(this, o + 2, o + 5), T(this, o + 3, o + 4);
    return this;
  }, c.prototype.toString = function() {
    const i = this.length;
    return i === 0 ? "" : arguments.length === 0 ? G(this, 0, i) : _.apply(this, arguments);
  }, c.prototype.toLocaleString = c.prototype.toString, c.prototype.equals = function(i) {
    if (!c.isBuffer(i)) throw new TypeError("Argument must be a Buffer");
    return this === i ? !0 : c.compare(this, i) === 0;
  }, c.prototype.inspect = function() {
    let i = "";
    const o = e.INSPECT_MAX_BYTES;
    return i = this.toString("hex", 0, o).replace(/(.{2})/g, "$1 ").trim(), this.length > o && (i += " ... "), "<Buffer " + i + ">";
  }, n && (c.prototype[n] = c.prototype.inspect), c.prototype.compare = function(i, o, g, j, C) {
    if (Re(i, Uint8Array) && (i = c.from(i, i.offset, i.byteLength)), !c.isBuffer(i))
      throw new TypeError(
        'The "target" argument must be one of type Buffer or Uint8Array. Received type ' + typeof i
      );
    if (o === void 0 && (o = 0), g === void 0 && (g = i ? i.length : 0), j === void 0 && (j = 0), C === void 0 && (C = this.length), o < 0 || g > i.length || j < 0 || C > this.length)
      throw new RangeError("out of range index");
    if (j >= C && o >= g)
      return 0;
    if (j >= C)
      return -1;
    if (o >= g)
      return 1;
    if (o >>>= 0, g >>>= 0, j >>>= 0, C >>>= 0, this === i) return 0;
    let O = C - j, W = g - o;
    const le = Math.min(O, W), oe = this.slice(j, C), de = i.slice(o, g);
    for (let re = 0; re < le; ++re)
      if (oe[re] !== de[re]) {
        O = oe[re], W = de[re];
        break;
      }
    return O < W ? -1 : W < O ? 1 : 0;
  };
  function M(u, i, o, g, j) {
    if (u.length === 0) return -1;
    if (typeof o == "string" ? (g = o, o = 0) : o > 2147483647 ? o = 2147483647 : o < -2147483648 && (o = -2147483648), o = +o, tr(o) && (o = j ? 0 : u.length - 1), o < 0 && (o = u.length + o), o >= u.length) {
      if (j) return -1;
      o = u.length - 1;
    } else if (o < 0)
      if (j) o = 0;
      else return -1;
    if (typeof i == "string" && (i = c.from(i, g)), c.isBuffer(i))
      return i.length === 0 ? -1 : R(u, i, o, g, j);
    if (typeof i == "number")
      return i = i & 255, typeof Uint8Array.prototype.indexOf == "function" ? j ? Uint8Array.prototype.indexOf.call(u, i, o) : Uint8Array.prototype.lastIndexOf.call(u, i, o) : R(u, [i], o, g, j);
    throw new TypeError("val must be string, number or Buffer");
  }
  function R(u, i, o, g, j) {
    let C = 1, O = u.length, W = i.length;
    if (g !== void 0 && (g = String(g).toLowerCase(), g === "ucs2" || g === "ucs-2" || g === "utf16le" || g === "utf-16le")) {
      if (u.length < 2 || i.length < 2)
        return -1;
      C = 2, O /= 2, W /= 2, o /= 2;
    }
    function le(de, re) {
      return C === 1 ? de[re] : de.readUInt16BE(re * C);
    }
    let oe;
    if (j) {
      let de = -1;
      for (oe = o; oe < O; oe++)
        if (le(u, oe) === le(i, de === -1 ? 0 : oe - de)) {
          if (de === -1 && (de = oe), oe - de + 1 === W) return de * C;
        } else
          de !== -1 && (oe -= oe - de), de = -1;
    } else
      for (o + W > O && (o = O - W), oe = o; oe >= 0; oe--) {
        let de = !0;
        for (let re = 0; re < W; re++)
          if (le(u, oe + re) !== le(i, re)) {
            de = !1;
            break;
          }
        if (de) return oe;
      }
    return -1;
  }
  c.prototype.includes = function(i, o, g) {
    return this.indexOf(i, o, g) !== -1;
  }, c.prototype.indexOf = function(i, o, g) {
    return M(this, i, o, g, !0);
  }, c.prototype.lastIndexOf = function(i, o, g) {
    return M(this, i, o, g, !1);
  };
  function k(u, i, o, g) {
    o = Number(o) || 0;
    const j = u.length - o;
    g ? (g = Number(g), g > j && (g = j)) : g = j;
    const C = i.length;
    g > C / 2 && (g = C / 2);
    let O;
    for (O = 0; O < g; ++O) {
      const W = parseInt(i.substr(O * 2, 2), 16);
      if (tr(W)) return O;
      u[o + O] = W;
    }
    return O;
  }
  function I(u, i, o, g) {
    return Ft(B(i, u.length - o), u, o, g);
  }
  function $(u, i, o, g) {
    return Ft(X(i), u, o, g);
  }
  function F(u, i, o, g) {
    return Ft(Je(i), u, o, g);
  }
  function z(u, i, o, g) {
    return Ft(be(i, u.length - o), u, o, g);
  }
  c.prototype.write = function(i, o, g, j) {
    if (o === void 0)
      j = "utf8", g = this.length, o = 0;
    else if (g === void 0 && typeof o == "string")
      j = o, g = this.length, o = 0;
    else if (isFinite(o))
      o = o >>> 0, isFinite(g) ? (g = g >>> 0, j === void 0 && (j = "utf8")) : (j = g, g = void 0);
    else
      throw new Error(
        "Buffer.write(string, encoding, offset[, length]) is no longer supported"
      );
    const C = this.length - o;
    if ((g === void 0 || g > C) && (g = C), i.length > 0 && (g < 0 || o < 0) || o > this.length)
      throw new RangeError("Attempt to write outside buffer bounds");
    j || (j = "utf8");
    let O = !1;
    for (; ; )
      switch (j) {
        case "hex":
          return k(this, i, o, g);
        case "utf8":
        case "utf-8":
          return I(this, i, o, g);
        case "ascii":
        case "latin1":
        case "binary":
          return $(this, i, o, g);
        case "base64":
          return F(this, i, o, g);
        case "ucs2":
        case "ucs-2":
        case "utf16le":
        case "utf-16le":
          return z(this, i, o, g);
        default:
          if (O) throw new TypeError("Unknown encoding: " + j);
          j = ("" + j).toLowerCase(), O = !0;
      }
  }, c.prototype.toJSON = function() {
    return {
      type: "Buffer",
      data: Array.prototype.slice.call(this._arr || this, 0)
    };
  };
  function V(u, i, o) {
    return i === 0 && o === u.length ? t.fromByteArray(u) : t.fromByteArray(u.slice(i, o));
  }
  function G(u, i, o) {
    o = Math.min(u.length, o);
    const g = [];
    let j = i;
    for (; j < o; ) {
      const C = u[j];
      let O = null, W = C > 239 ? 4 : C > 223 ? 3 : C > 191 ? 2 : 1;
      if (j + W <= o) {
        let le, oe, de, re;
        switch (W) {
          case 1:
            C < 128 && (O = C);
            break;
          case 2:
            le = u[j + 1], (le & 192) === 128 && (re = (C & 31) << 6 | le & 63, re > 127 && (O = re));
            break;
          case 3:
            le = u[j + 1], oe = u[j + 2], (le & 192) === 128 && (oe & 192) === 128 && (re = (C & 15) << 12 | (le & 63) << 6 | oe & 63, re > 2047 && (re < 55296 || re > 57343) && (O = re));
            break;
          case 4:
            le = u[j + 1], oe = u[j + 2], de = u[j + 3], (le & 192) === 128 && (oe & 192) === 128 && (de & 192) === 128 && (re = (C & 15) << 18 | (le & 63) << 12 | (oe & 63) << 6 | de & 63, re > 65535 && re < 1114112 && (O = re));
        }
      }
      O === null ? (O = 65533, W = 1) : O > 65535 && (O -= 65536, g.push(O >>> 10 & 1023 | 55296), O = 56320 | O & 1023), g.push(O), j += W;
    }
    return J(g);
  }
  const L = 4096;
  function J(u) {
    const i = u.length;
    if (i <= L)
      return String.fromCharCode.apply(String, u);
    let o = "", g = 0;
    for (; g < i; )
      o += String.fromCharCode.apply(
        String,
        u.slice(g, g += L)
      );
    return o;
  }
  function Q(u, i, o) {
    let g = "";
    o = Math.min(u.length, o);
    for (let j = i; j < o; ++j)
      g += String.fromCharCode(u[j] & 127);
    return g;
  }
  function ge(u, i, o) {
    let g = "";
    o = Math.min(u.length, o);
    for (let j = i; j < o; ++j)
      g += String.fromCharCode(u[j]);
    return g;
  }
  function Me(u, i, o) {
    const g = u.length;
    (!i || i < 0) && (i = 0), (!o || o < 0 || o > g) && (o = g);
    let j = "";
    for (let C = i; C < o; ++C)
      j += os[u[C]];
    return j;
  }
  function ze(u, i, o) {
    const g = u.slice(i, o);
    let j = "";
    for (let C = 0; C < g.length - 1; C += 2)
      j += String.fromCharCode(g[C] + g[C + 1] * 256);
    return j;
  }
  c.prototype.slice = function(i, o) {
    const g = this.length;
    i = ~~i, o = o === void 0 ? g : ~~o, i < 0 ? (i += g, i < 0 && (i = 0)) : i > g && (i = g), o < 0 ? (o += g, o < 0 && (o = 0)) : o > g && (o = g), o < i && (o = i);
    const j = this.subarray(i, o);
    return Object.setPrototypeOf(j, c.prototype), j;
  };
  function ne(u, i, o) {
    if (u % 1 !== 0 || u < 0) throw new RangeError("offset is not uint");
    if (u + i > o) throw new RangeError("Trying to access beyond buffer length");
  }
  c.prototype.readUintLE = c.prototype.readUIntLE = function(i, o, g) {
    i = i >>> 0, o = o >>> 0, g || ne(i, o, this.length);
    let j = this[i], C = 1, O = 0;
    for (; ++O < o && (C *= 256); )
      j += this[i + O] * C;
    return j;
  }, c.prototype.readUintBE = c.prototype.readUIntBE = function(i, o, g) {
    i = i >>> 0, o = o >>> 0, g || ne(i, o, this.length);
    let j = this[i + --o], C = 1;
    for (; o > 0 && (C *= 256); )
      j += this[i + --o] * C;
    return j;
  }, c.prototype.readUint8 = c.prototype.readUInt8 = function(i, o) {
    return i = i >>> 0, o || ne(i, 1, this.length), this[i];
  }, c.prototype.readUint16LE = c.prototype.readUInt16LE = function(i, o) {
    return i = i >>> 0, o || ne(i, 2, this.length), this[i] | this[i + 1] << 8;
  }, c.prototype.readUint16BE = c.prototype.readUInt16BE = function(i, o) {
    return i = i >>> 0, o || ne(i, 2, this.length), this[i] << 8 | this[i + 1];
  }, c.prototype.readUint32LE = c.prototype.readUInt32LE = function(i, o) {
    return i = i >>> 0, o || ne(i, 4, this.length), (this[i] | this[i + 1] << 8 | this[i + 2] << 16) + this[i + 3] * 16777216;
  }, c.prototype.readUint32BE = c.prototype.readUInt32BE = function(i, o) {
    return i = i >>> 0, o || ne(i, 4, this.length), this[i] * 16777216 + (this[i + 1] << 16 | this[i + 2] << 8 | this[i + 3]);
  }, c.prototype.readBigUInt64LE = Ge(function(i) {
    i = i >>> 0, Qe(i, "offset");
    const o = this[i], g = this[i + 7];
    (o === void 0 || g === void 0) && ot(i, this.length - 8);
    const j = o + this[++i] * 2 ** 8 + this[++i] * 2 ** 16 + this[++i] * 2 ** 24, C = this[++i] + this[++i] * 2 ** 8 + this[++i] * 2 ** 16 + g * 2 ** 24;
    return BigInt(j) + (BigInt(C) << BigInt(32));
  }), c.prototype.readBigUInt64BE = Ge(function(i) {
    i = i >>> 0, Qe(i, "offset");
    const o = this[i], g = this[i + 7];
    (o === void 0 || g === void 0) && ot(i, this.length - 8);
    const j = o * 2 ** 24 + this[++i] * 2 ** 16 + this[++i] * 2 ** 8 + this[++i], C = this[++i] * 2 ** 24 + this[++i] * 2 ** 16 + this[++i] * 2 ** 8 + g;
    return (BigInt(j) << BigInt(32)) + BigInt(C);
  }), c.prototype.readIntLE = function(i, o, g) {
    i = i >>> 0, o = o >>> 0, g || ne(i, o, this.length);
    let j = this[i], C = 1, O = 0;
    for (; ++O < o && (C *= 256); )
      j += this[i + O] * C;
    return C *= 128, j >= C && (j -= Math.pow(2, 8 * o)), j;
  }, c.prototype.readIntBE = function(i, o, g) {
    i = i >>> 0, o = o >>> 0, g || ne(i, o, this.length);
    let j = o, C = 1, O = this[i + --j];
    for (; j > 0 && (C *= 256); )
      O += this[i + --j] * C;
    return C *= 128, O >= C && (O -= Math.pow(2, 8 * o)), O;
  }, c.prototype.readInt8 = function(i, o) {
    return i = i >>> 0, o || ne(i, 1, this.length), this[i] & 128 ? (255 - this[i] + 1) * -1 : this[i];
  }, c.prototype.readInt16LE = function(i, o) {
    i = i >>> 0, o || ne(i, 2, this.length);
    const g = this[i] | this[i + 1] << 8;
    return g & 32768 ? g | 4294901760 : g;
  }, c.prototype.readInt16BE = function(i, o) {
    i = i >>> 0, o || ne(i, 2, this.length);
    const g = this[i + 1] | this[i] << 8;
    return g & 32768 ? g | 4294901760 : g;
  }, c.prototype.readInt32LE = function(i, o) {
    return i = i >>> 0, o || ne(i, 4, this.length), this[i] | this[i + 1] << 8 | this[i + 2] << 16 | this[i + 3] << 24;
  }, c.prototype.readInt32BE = function(i, o) {
    return i = i >>> 0, o || ne(i, 4, this.length), this[i] << 24 | this[i + 1] << 16 | this[i + 2] << 8 | this[i + 3];
  }, c.prototype.readBigInt64LE = Ge(function(i) {
    i = i >>> 0, Qe(i, "offset");
    const o = this[i], g = this[i + 7];
    (o === void 0 || g === void 0) && ot(i, this.length - 8);
    const j = this[i + 4] + this[i + 5] * 2 ** 8 + this[i + 6] * 2 ** 16 + (g << 24);
    return (BigInt(j) << BigInt(32)) + BigInt(o + this[++i] * 2 ** 8 + this[++i] * 2 ** 16 + this[++i] * 2 ** 24);
  }), c.prototype.readBigInt64BE = Ge(function(i) {
    i = i >>> 0, Qe(i, "offset");
    const o = this[i], g = this[i + 7];
    (o === void 0 || g === void 0) && ot(i, this.length - 8);
    const j = (o << 24) + // Overflow
    this[++i] * 2 ** 16 + this[++i] * 2 ** 8 + this[++i];
    return (BigInt(j) << BigInt(32)) + BigInt(this[++i] * 2 ** 24 + this[++i] * 2 ** 16 + this[++i] * 2 ** 8 + g);
  }), c.prototype.readFloatLE = function(i, o) {
    return i = i >>> 0, o || ne(i, 4, this.length), r.read(this, i, !0, 23, 4);
  }, c.prototype.readFloatBE = function(i, o) {
    return i = i >>> 0, o || ne(i, 4, this.length), r.read(this, i, !1, 23, 4);
  }, c.prototype.readDoubleLE = function(i, o) {
    return i = i >>> 0, o || ne(i, 8, this.length), r.read(this, i, !0, 52, 8);
  }, c.prototype.readDoubleBE = function(i, o) {
    return i = i >>> 0, o || ne(i, 8, this.length), r.read(this, i, !1, 52, 8);
  };
  function ce(u, i, o, g, j, C) {
    if (!c.isBuffer(u)) throw new TypeError('"buffer" argument must be a Buffer instance');
    if (i > j || i < C) throw new RangeError('"value" argument is out of bounds');
    if (o + g > u.length) throw new RangeError("Index out of range");
  }
  c.prototype.writeUintLE = c.prototype.writeUIntLE = function(i, o, g, j) {
    if (i = +i, o = o >>> 0, g = g >>> 0, !j) {
      const W = Math.pow(2, 8 * g) - 1;
      ce(this, i, o, g, W, 0);
    }
    let C = 1, O = 0;
    for (this[o] = i & 255; ++O < g && (C *= 256); )
      this[o + O] = i / C & 255;
    return o + g;
  }, c.prototype.writeUintBE = c.prototype.writeUIntBE = function(i, o, g, j) {
    if (i = +i, o = o >>> 0, g = g >>> 0, !j) {
      const W = Math.pow(2, 8 * g) - 1;
      ce(this, i, o, g, W, 0);
    }
    let C = g - 1, O = 1;
    for (this[o + C] = i & 255; --C >= 0 && (O *= 256); )
      this[o + C] = i / O & 255;
    return o + g;
  }, c.prototype.writeUint8 = c.prototype.writeUInt8 = function(i, o, g) {
    return i = +i, o = o >>> 0, g || ce(this, i, o, 1, 255, 0), this[o] = i & 255, o + 1;
  }, c.prototype.writeUint16LE = c.prototype.writeUInt16LE = function(i, o, g) {
    return i = +i, o = o >>> 0, g || ce(this, i, o, 2, 65535, 0), this[o] = i & 255, this[o + 1] = i >>> 8, o + 2;
  }, c.prototype.writeUint16BE = c.prototype.writeUInt16BE = function(i, o, g) {
    return i = +i, o = o >>> 0, g || ce(this, i, o, 2, 65535, 0), this[o] = i >>> 8, this[o + 1] = i & 255, o + 2;
  }, c.prototype.writeUint32LE = c.prototype.writeUInt32LE = function(i, o, g) {
    return i = +i, o = o >>> 0, g || ce(this, i, o, 4, 4294967295, 0), this[o + 3] = i >>> 24, this[o + 2] = i >>> 16, this[o + 1] = i >>> 8, this[o] = i & 255, o + 4;
  }, c.prototype.writeUint32BE = c.prototype.writeUInt32BE = function(i, o, g) {
    return i = +i, o = o >>> 0, g || ce(this, i, o, 4, 4294967295, 0), this[o] = i >>> 24, this[o + 1] = i >>> 16, this[o + 2] = i >>> 8, this[o + 3] = i & 255, o + 4;
  };
  function mt(u, i, o, g, j) {
    gt(i, g, j, u, o, 7);
    let C = Number(i & BigInt(4294967295));
    u[o++] = C, C = C >> 8, u[o++] = C, C = C >> 8, u[o++] = C, C = C >> 8, u[o++] = C;
    let O = Number(i >> BigInt(32) & BigInt(4294967295));
    return u[o++] = O, O = O >> 8, u[o++] = O, O = O >> 8, u[o++] = O, O = O >> 8, u[o++] = O, o;
  }
  function pt(u, i, o, g, j) {
    gt(i, g, j, u, o, 7);
    let C = Number(i & BigInt(4294967295));
    u[o + 7] = C, C = C >> 8, u[o + 6] = C, C = C >> 8, u[o + 5] = C, C = C >> 8, u[o + 4] = C;
    let O = Number(i >> BigInt(32) & BigInt(4294967295));
    return u[o + 3] = O, O = O >> 8, u[o + 2] = O, O = O >> 8, u[o + 1] = O, O = O >> 8, u[o] = O, o + 8;
  }
  c.prototype.writeBigUInt64LE = Ge(function(i, o = 0) {
    return mt(this, i, o, BigInt(0), BigInt("0xffffffffffffffff"));
  }), c.prototype.writeBigUInt64BE = Ge(function(i, o = 0) {
    return pt(this, i, o, BigInt(0), BigInt("0xffffffffffffffff"));
  }), c.prototype.writeIntLE = function(i, o, g, j) {
    if (i = +i, o = o >>> 0, !j) {
      const le = Math.pow(2, 8 * g - 1);
      ce(this, i, o, g, le - 1, -le);
    }
    let C = 0, O = 1, W = 0;
    for (this[o] = i & 255; ++C < g && (O *= 256); )
      i < 0 && W === 0 && this[o + C - 1] !== 0 && (W = 1), this[o + C] = (i / O >> 0) - W & 255;
    return o + g;
  }, c.prototype.writeIntBE = function(i, o, g, j) {
    if (i = +i, o = o >>> 0, !j) {
      const le = Math.pow(2, 8 * g - 1);
      ce(this, i, o, g, le - 1, -le);
    }
    let C = g - 1, O = 1, W = 0;
    for (this[o + C] = i & 255; --C >= 0 && (O *= 256); )
      i < 0 && W === 0 && this[o + C + 1] !== 0 && (W = 1), this[o + C] = (i / O >> 0) - W & 255;
    return o + g;
  }, c.prototype.writeInt8 = function(i, o, g) {
    return i = +i, o = o >>> 0, g || ce(this, i, o, 1, 127, -128), i < 0 && (i = 255 + i + 1), this[o] = i & 255, o + 1;
  }, c.prototype.writeInt16LE = function(i, o, g) {
    return i = +i, o = o >>> 0, g || ce(this, i, o, 2, 32767, -32768), this[o] = i & 255, this[o + 1] = i >>> 8, o + 2;
  }, c.prototype.writeInt16BE = function(i, o, g) {
    return i = +i, o = o >>> 0, g || ce(this, i, o, 2, 32767, -32768), this[o] = i >>> 8, this[o + 1] = i & 255, o + 2;
  }, c.prototype.writeInt32LE = function(i, o, g) {
    return i = +i, o = o >>> 0, g || ce(this, i, o, 4, 2147483647, -2147483648), this[o] = i & 255, this[o + 1] = i >>> 8, this[o + 2] = i >>> 16, this[o + 3] = i >>> 24, o + 4;
  }, c.prototype.writeInt32BE = function(i, o, g) {
    return i = +i, o = o >>> 0, g || ce(this, i, o, 4, 2147483647, -2147483648), i < 0 && (i = 4294967295 + i + 1), this[o] = i >>> 24, this[o + 1] = i >>> 16, this[o + 2] = i >>> 8, this[o + 3] = i & 255, o + 4;
  }, c.prototype.writeBigInt64LE = Ge(function(i, o = 0) {
    return mt(this, i, o, -BigInt("0x8000000000000000"), BigInt("0x7fffffffffffffff"));
  }), c.prototype.writeBigInt64BE = Ge(function(i, o = 0) {
    return pt(this, i, o, -BigInt("0x8000000000000000"), BigInt("0x7fffffffffffffff"));
  });
  function At(u, i, o, g, j, C) {
    if (o + g > u.length) throw new RangeError("Index out of range");
    if (o < 0) throw new RangeError("Index out of range");
  }
  function nt(u, i, o, g, j) {
    return i = +i, o = o >>> 0, j || At(u, i, o, 4), r.write(u, i, o, g, 23, 4), o + 4;
  }
  c.prototype.writeFloatLE = function(i, o, g) {
    return nt(this, i, o, !0, g);
  }, c.prototype.writeFloatBE = function(i, o, g) {
    return nt(this, i, o, !1, g);
  };
  function at(u, i, o, g, j) {
    return i = +i, o = o >>> 0, j || At(u, i, o, 8), r.write(u, i, o, g, 52, 8), o + 8;
  }
  c.prototype.writeDoubleLE = function(i, o, g) {
    return at(this, i, o, !0, g);
  }, c.prototype.writeDoubleBE = function(i, o, g) {
    return at(this, i, o, !1, g);
  }, c.prototype.copy = function(i, o, g, j) {
    if (!c.isBuffer(i)) throw new TypeError("argument should be a Buffer");
    if (g || (g = 0), !j && j !== 0 && (j = this.length), o >= i.length && (o = i.length), o || (o = 0), j > 0 && j < g && (j = g), j === g || i.length === 0 || this.length === 0) return 0;
    if (o < 0)
      throw new RangeError("targetStart out of bounds");
    if (g < 0 || g >= this.length) throw new RangeError("Index out of range");
    if (j < 0) throw new RangeError("sourceEnd out of bounds");
    j > this.length && (j = this.length), i.length - o < j - g && (j = i.length - o + g);
    const C = j - g;
    return this === i && typeof Uint8Array.prototype.copyWithin == "function" ? this.copyWithin(o, g, j) : Uint8Array.prototype.set.call(
      i,
      this.subarray(g, j),
      o
    ), C;
  }, c.prototype.fill = function(i, o, g, j) {
    if (typeof i == "string") {
      if (typeof o == "string" ? (j = o, o = 0, g = this.length) : typeof g == "string" && (j = g, g = this.length), j !== void 0 && typeof j != "string")
        throw new TypeError("encoding must be a string");
      if (typeof j == "string" && !c.isEncoding(j))
        throw new TypeError("Unknown encoding: " + j);
      if (i.length === 1) {
        const O = i.charCodeAt(0);
        (j === "utf8" && O < 128 || j === "latin1") && (i = O);
      }
    } else typeof i == "number" ? i = i & 255 : typeof i == "boolean" && (i = Number(i));
    if (o < 0 || this.length < o || this.length < g)
      throw new RangeError("Out of range index");
    if (g <= o)
      return this;
    o = o >>> 0, g = g === void 0 ? this.length : g >>> 0, i || (i = 0);
    let C;
    if (typeof i == "number")
      for (C = o; C < g; ++C)
        this[C] = i;
    else {
      const O = c.isBuffer(i) ? i : c.from(i, j), W = O.length;
      if (W === 0)
        throw new TypeError('The value "' + i + '" is invalid for argument "value"');
      for (C = 0; C < g - o; ++C)
        this[C + o] = O[C % W];
    }
    return this;
  };
  const Ce = {};
  function Ye(u, i, o) {
    Ce[u] = class extends o {
      constructor() {
        super(), Object.defineProperty(this, "message", {
          value: i.apply(this, arguments),
          writable: !0,
          configurable: !0
        }), this.name = `${this.name} [${u}]`, this.stack, delete this.name;
      }
      get code() {
        return u;
      }
      set code(j) {
        Object.defineProperty(this, "code", {
          configurable: !0,
          enumerable: !0,
          value: j,
          writable: !0
        });
      }
      toString() {
        return `${this.name} [${u}]: ${this.message}`;
      }
    };
  }
  Ye(
    "ERR_BUFFER_OUT_OF_BOUNDS",
    function(u) {
      return u ? `${u} is outside of buffer bounds` : "Attempt to access memory outside buffer bounds";
    },
    RangeError
  ), Ye(
    "ERR_INVALID_ARG_TYPE",
    function(u, i) {
      return `The "${u}" argument must be of type number. Received type ${typeof i}`;
    },
    TypeError
  ), Ye(
    "ERR_OUT_OF_RANGE",
    function(u, i, o) {
      let g = `The value of "${u}" is out of range.`, j = o;
      return Number.isInteger(o) && Math.abs(o) > 2 ** 32 ? j = it(String(o)) : typeof o == "bigint" && (j = String(o), (o > BigInt(2) ** BigInt(32) || o < -(BigInt(2) ** BigInt(32))) && (j = it(j)), j += "n"), g += ` It must be ${i}. Received ${j}`, g;
    },
    RangeError
  );
  function it(u) {
    let i = "", o = u.length;
    const g = u[0] === "-" ? 1 : 0;
    for (; o >= g + 4; o -= 3)
      i = `_${u.slice(o - 3, o)}${i}`;
    return `${u.slice(0, o)}${i}`;
  }
  function er(u, i, o) {
    Qe(i, "offset"), (u[i] === void 0 || u[i + o] === void 0) && ot(i, u.length - (o + 1));
  }
  function gt(u, i, o, g, j, C) {
    if (u > o || u < i) {
      const O = typeof i == "bigint" ? "n" : "";
      let W;
      throw i === 0 || i === BigInt(0) ? W = `>= 0${O} and < 2${O} ** ${(C + 1) * 8}${O}` : W = `>= -(2${O} ** ${(C + 1) * 8 - 1}${O}) and < 2 ** ${(C + 1) * 8 - 1}${O}`, new Ce.ERR_OUT_OF_RANGE("value", W, u);
    }
    er(g, j, C);
  }
  function Qe(u, i) {
    if (typeof u != "number")
      throw new Ce.ERR_INVALID_ARG_TYPE(i, "number", u);
  }
  function ot(u, i, o) {
    throw Math.floor(u) !== u ? (Qe(u, o), new Ce.ERR_OUT_OF_RANGE("offset", "an integer", u)) : i < 0 ? new Ce.ERR_BUFFER_OUT_OF_BOUNDS() : new Ce.ERR_OUT_OF_RANGE(
      "offset",
      `>= 0 and <= ${i}`,
      u
    );
  }
  const yt = /[^+/0-9A-Za-z-_]/g;
  function ct(u) {
    if (u = u.split("=")[0], u = u.trim().replace(yt, ""), u.length < 2) return "";
    for (; u.length % 4 !== 0; )
      u = u + "=";
    return u;
  }
  function B(u, i) {
    i = i || 1 / 0;
    let o;
    const g = u.length;
    let j = null;
    const C = [];
    for (let O = 0; O < g; ++O) {
      if (o = u.charCodeAt(O), o > 55295 && o < 57344) {
        if (!j) {
          if (o > 56319) {
            (i -= 3) > -1 && C.push(239, 191, 189);
            continue;
          } else if (O + 1 === g) {
            (i -= 3) > -1 && C.push(239, 191, 189);
            continue;
          }
          j = o;
          continue;
        }
        if (o < 56320) {
          (i -= 3) > -1 && C.push(239, 191, 189), j = o;
          continue;
        }
        o = (j - 55296 << 10 | o - 56320) + 65536;
      } else j && (i -= 3) > -1 && C.push(239, 191, 189);
      if (j = null, o < 128) {
        if ((i -= 1) < 0) break;
        C.push(o);
      } else if (o < 2048) {
        if ((i -= 2) < 0) break;
        C.push(
          o >> 6 | 192,
          o & 63 | 128
        );
      } else if (o < 65536) {
        if ((i -= 3) < 0) break;
        C.push(
          o >> 12 | 224,
          o >> 6 & 63 | 128,
          o & 63 | 128
        );
      } else if (o < 1114112) {
        if ((i -= 4) < 0) break;
        C.push(
          o >> 18 | 240,
          o >> 12 & 63 | 128,
          o >> 6 & 63 | 128,
          o & 63 | 128
        );
      } else
        throw new Error("Invalid code point");
    }
    return C;
  }
  function X(u) {
    const i = [];
    for (let o = 0; o < u.length; ++o)
      i.push(u.charCodeAt(o) & 255);
    return i;
  }
  function be(u, i) {
    let o, g, j;
    const C = [];
    for (let O = 0; O < u.length && !((i -= 2) < 0); ++O)
      o = u.charCodeAt(O), g = o >> 8, j = o % 256, C.push(j), C.push(g);
    return C;
  }
  function Je(u) {
    return t.toByteArray(ct(u));
  }
  function Ft(u, i, o, g) {
    let j;
    for (j = 0; j < g && !(j + o >= i.length || j >= u.length); ++j)
      i[j + o] = u[j];
    return j;
  }
  function Re(u, i) {
    return u instanceof i || u != null && u.constructor != null && u.constructor.name != null && u.constructor.name === i.name;
  }
  function tr(u) {
    return u !== u;
  }
  const os = function() {
    const u = "0123456789abcdef", i = new Array(256);
    for (let o = 0; o < 16; ++o) {
      const g = o * 16;
      for (let j = 0; j < 16; ++j)
        i[g + j] = u[o] + u[j];
    }
    return i;
  }();
  function Ge(u) {
    return typeof BigInt > "u" ? Er : u;
  }
  function Er() {
    throw new Error("BigInt not supported");
  }
})(on);
function ss(e) {
  return on.Buffer.from(e, "base64");
}
function ol(e) {
  return on.Buffer.from(e).toString("base64");
}
Bs.sha512Sync = (...e) => Zc(Bs.concatBytes(...e));
const cl = {
  isAuthenticated: !1,
  systemPublicKey: null,
  systemKeyId: null,
  privateKey: null,
  publicKeyId: null,
  isLoading: !1,
  error: null
}, vs = Jt(
  "auth/initializeSystemKey",
  async (e, { rejectWithValue: t }) => {
    try {
      const r = await en();
      if (console.log("initializeSystemKey thunk response:", r), r.success && r.data && r.data.private_key) {
        const n = ss(r.data.private_key), a = await nn(n);
        return {
          systemPublicKey: btoa(String.fromCharCode(...a)),
          systemKeyId: "node-private-key",
          privateKey: n,
          isAuthenticated: !0
        };
      } else
        return {
          systemPublicKey: null,
          systemKeyId: null,
          privateKey: null,
          isAuthenticated: !1
        };
    } catch (r) {
      return console.error("Failed to fetch node private key:", r), t(r instanceof Error ? r.message : "Failed to fetch node private key");
    }
  }
), Mr = Jt(
  "auth/validatePrivateKey",
  async (e, { getState: t, rejectWithValue: r }) => {
    const n = t(), { systemPublicKey: a, systemKeyId: l } = n.auth;
    if (!a || !l)
      return r("System public key not available");
    try {
      console.log("🔑 Converting private key from base64...");
      const d = ss(e);
      console.log("🔑 Generating public key from private key...");
      const c = await nn(d), f = btoa(String.fromCharCode(...c)), m = f === a;
      return console.log("🔑 Key comparison:", {
        derived: f,
        system: a,
        matches: m
      }), m ? {
        privateKey: d,
        publicKeyId: l,
        isAuthenticated: !0
      } : r("Private key does not match system public key");
    } catch (d) {
      return console.error("Private key validation failed:", d), r(d instanceof Error ? d.message : "Private key validation failed");
    }
  }
), ws = Jt(
  "auth/refreshSystemKey",
  async (e, { rejectWithValue: t }) => {
    for (let a = 1; a <= 5; a++)
      try {
        const l = await en();
        if (l.success && l.data && l.data.private_key) {
          const d = ss(l.data.private_key), c = await nn(d);
          return {
            systemPublicKey: btoa(String.fromCharCode(...c)),
            systemKeyId: "node-private-key",
            privateKey: d,
            isAuthenticated: !0
          };
        } else if (a < 5) {
          const d = 200 * a;
          await new Promise((c) => setTimeout(c, d));
        }
      } catch (l) {
        if (a === 5)
          return t(l instanceof Error ? l.message : "Failed to fetch node private key");
        {
          const d = 200 * a;
          await new Promise((c) => setTimeout(c, d));
        }
      }
    return t("Failed to fetch node private key after multiple attempts");
  }
), Es = Jt(
  "auth/fetchNodePrivateKey",
  async (e, { rejectWithValue: t }) => {
    try {
      const r = await en();
      return console.log("fetchNodePrivateKey thunk response:", r), r.success && r.data && r.data.private_key ? {
        privateKey: ss(r.data.private_key),
        publicKeyId: "node-private-key",
        // Use a consistent identifier
        isAuthenticated: !0
      } : t("Failed to fetch private key from backend");
    } catch (r) {
      return console.error("Failed to fetch node private key:", r), t(r instanceof Error ? r.message : "Failed to fetch node private key");
    }
  }
), Wa = Ws({
  name: "auth",
  initialState: cl,
  reducers: {
    clearAuthentication: (e) => {
      e.isAuthenticated = !1, e.privateKey = null, e.publicKeyId = null, e.error = null;
    },
    setError: (e, t) => {
      e.error = t.payload;
    },
    clearError: (e) => {
      e.error = null;
    },
    updateSystemKey: (e, t) => {
      e.systemPublicKey = t.payload.systemPublicKey, e.systemKeyId = t.payload.systemKeyId, e.error = null;
    }
  },
  extraReducers: (e) => {
    e.addCase(vs.pending, (t) => {
      t.isLoading = !0, t.error = null;
    }).addCase(vs.fulfilled, (t, r) => {
      t.isLoading = !1, t.systemPublicKey = r.payload.systemPublicKey, t.systemKeyId = r.payload.systemKeyId, t.privateKey = r.payload.privateKey, t.isAuthenticated = r.payload.isAuthenticated, t.error = null;
    }).addCase(vs.rejected, (t, r) => {
      t.isLoading = !1, t.error = r.payload;
    }).addCase(Mr.pending, (t) => {
      t.isLoading = !0, t.error = null;
    }).addCase(Mr.fulfilled, (t, r) => {
      t.isLoading = !1, t.isAuthenticated = r.payload.isAuthenticated, t.privateKey = r.payload.privateKey, t.publicKeyId = r.payload.publicKeyId, t.error = null;
    }).addCase(Mr.rejected, (t, r) => {
      t.isLoading = !1, t.isAuthenticated = !1, t.privateKey = null, t.publicKeyId = null, t.error = r.payload;
    }).addCase(ws.pending, (t) => {
      t.isLoading = !0, t.error = null;
    }).addCase(ws.fulfilled, (t, r) => {
      t.isLoading = !1, t.systemPublicKey = r.payload.systemPublicKey, t.systemKeyId = r.payload.systemKeyId, t.privateKey = r.payload.privateKey, t.isAuthenticated = r.payload.isAuthenticated, t.error = null;
    }).addCase(ws.rejected, (t, r) => {
      t.isLoading = !1, t.systemPublicKey = null, t.systemKeyId = null, t.error = r.payload;
    }).addCase(Es.pending, (t) => {
      t.isLoading = !0, t.error = null;
    }).addCase(Es.fulfilled, (t, r) => {
      t.isLoading = !1, t.isAuthenticated = r.payload.isAuthenticated, t.privateKey = r.payload.privateKey, t.publicKeyId = r.payload.publicKeyId, t.error = null;
    }).addCase(Es.rejected, (t, r) => {
      t.isLoading = !1, t.isAuthenticated = !1, t.privateKey = null, t.publicKeyId = null, t.error = r.payload;
    });
  }
}), { clearAuthentication: ll, setError: wu, clearError: Eu, updateSystemKey: Nu } = Wa.actions, dl = Wa.reducer, ul = 3e5, Ns = 3, xr = {
  // Async thunk action types
  FETCH_SCHEMAS: "schemas/fetchSchemas",
  APPROVE_SCHEMA: "schemas/approveSchema",
  BLOCK_SCHEMA: "schemas/blockSchema",
  UNLOAD_SCHEMA: "schemas/unloadSchema",
  LOAD_SCHEMA: "schemas/loadSchema"
}, br = {
  // Network and API errors
  FETCH_FAILED: "Failed to fetch schemas from server",
  // Schema operation errors
  APPROVE_FAILED: "Failed to approve schema",
  BLOCK_FAILED: "Failed to block schema",
  UNLOAD_FAILED: "Failed to unload schema",
  LOAD_FAILED: "Failed to load schema"
}, st = {
  AVAILABLE: "available",
  APPROVED: "approved",
  BLOCKED: "blocked",
  LOADING: "loading",
  ERROR: "error"
};
process.env.NODE_ENV, process.env.NODE_ENV;
process.env.NODE_ENV, process.env.NODE_ENV;
const fl = {
  MUTATION_WRAPPER_KEY: "value"
}, hl = 200, ml = 300, pl = [
  // Main features
  { id: "ingestion", label: "Ingestion", icon: "📥", group: "main" },
  { id: "file-upload", label: "File Upload", icon: "📄", group: "main" },
  { id: "llm-query", label: "AI Query", icon: "🤖", group: "main" },
  // Developer/Advanced features
  { id: "schemas", label: "Schemas", icon: "📊", group: "advanced" },
  { id: "query", label: "Query", icon: "🔍", group: "advanced" },
  { id: "mutation", label: "Mutation", icon: "✏️", group: "advanced" },
  { id: "native-index", label: "Native Index Query", icon: "🧭", group: "advanced" }
], Ir = {
  executeQuery: "Execute Query"
}, kt = {
  schema: "Schema",
  schemaEmpty: "No schemas available",
  schemaHelp: "Select a schema to work with",
  operationType: "Operation Type",
  operationHelp: "Select the type of operation to perform"
}, gl = {
  loading: "Loading..."
}, yl = [
  { value: "Insert", label: "Insert" },
  { value: "Update", label: "Update" },
  { value: "Delete", label: "Delete" }
], Ya = {
  Insert: "create",
  Create: "create",
  Update: "update",
  Delete: "delete"
}, xl = {};
function Qa(e) {
  if (!e || typeof e != "object") return null;
  const t = e.schema_type;
  if (t === "Single")
    return "Single";
  if (t === "Range")
    return "Range";
  if (t === "HashRange")
    return "HashRange";
  if (typeof t == "object" && t !== null) {
    if ("HashRange" in t)
      return "HashRange";
    if ("Range" in t)
      return "Range";
  }
  return null;
}
function ln(e) {
  return !e || typeof e != "object" ? !1 : Qa(e) === "HashRange";
}
function Ja(e) {
  if (typeof e != "string") return null;
  const t = e.split(".");
  return t[t.length - 1] || e;
}
function Qt(e) {
  return !e || typeof e != "object" ? !1 : Qa(e) === "Range";
}
function Dt(e) {
  var r;
  if (!e || typeof e != "object") return null;
  const t = (r = e == null ? void 0 : e.key) == null ? void 0 : r.range_field;
  return typeof t == "string" && t.trim() ? Ja(t) : null;
}
function Za(e) {
  var r;
  if (!e || typeof e != "object") return null;
  const t = (r = e == null ? void 0 : e.key) == null ? void 0 : r.hash_field;
  return t && typeof t == "string" && t.trim() ? Ja(t) : null;
}
function bl(e) {
  if (!Qt(e))
    return {};
  const t = Dt(e);
  if (!Array.isArray(e.fields))
    throw new Error(`Expected schema.fields to be an array for range schema "${e.name}", got ${typeof e.fields}`);
  return e.fields.reduce((r, n) => (n !== t && (r[n] = {}), r), {});
}
function vl(e, t, r, n) {
  const a = typeof t == "string" ? Ya[t] || t.toLowerCase() : "", l = a === "delete", d = {
    type: "mutation",
    schema: e.name,
    mutation_type: a
  }, c = Dt(e);
  if (l)
    d.fields_and_values = {}, d.key_value = { hash: null, range: null }, r && r.trim() && c && (d.fields_and_values[c] = r.trim(), d.key_value.range = r.trim());
  else {
    const f = {};
    r && r.trim() && c && (f[c] = r.trim()), Object.entries(n).forEach(([m, h]) => {
      if (m !== c) {
        const y = fl.MUTATION_WRAPPER_KEY;
        typeof h == "string" || typeof h == "number" || typeof h == "boolean" ? f[m] = { [y]: h } : typeof h == "object" && h !== null ? f[m] = h : f[m] = { [y]: h };
      }
    }), d.fields_and_values = f, d.key_value = {
      hash: null,
      range: r && r.trim() ? r.trim() : null
    };
  }
  return d;
}
function Qn(e) {
  return Qt(e) ? {
    isRangeSchema: !0,
    rangeKey: Dt(e),
    rangeFields: [],
    // Declarative schemas don't store field types
    nonRangeKeyFields: bl(e),
    totalFields: Array.isArray(e.fields) ? e.fields.length : 0
  } : null;
}
function Xa(e) {
  return typeof e == "string" ? e.toLowerCase() : typeof e == "object" && e !== null ? e.state ? String(e.state).toLowerCase() : String(e).toLowerCase() : String(e || "").toLowerCase();
}
function wl(e) {
  return e == null;
}
function Jn(e) {
  return ln(e) ? {
    isHashRangeSchema: !0,
    hashField: Za(e),
    rangeField: Dt(e),
    totalFields: Array.isArray(e.fields) ? e.fields.length : 0
  } : null;
}
const Or = De.AVAILABLE, El = /* @__PURE__ */ new Set([
  De.AVAILABLE,
  De.APPROVED,
  De.BLOCKED,
  "loading",
  "error"
]);
function Nl(e) {
  if (!e || typeof e != "object")
    return null;
  const t = e.name;
  if (typeof t == "string" && t.trim().length > 0)
    return t;
  const r = e.schema;
  if (r && typeof r == "object") {
    const n = r.name;
    if (typeof n == "string" && n.trim().length > 0)
      return n;
  }
  return null;
}
function jl(e) {
  var r;
  return !e || typeof e != "object" ? void 0 : [
    e.state,
    e.schema_state,
    e.schemaState,
    e.status,
    e.current_state,
    (r = e.schema) == null ? void 0 : r.state
  ].find((n) => n !== void 0);
}
class ns {
  constructor(t) {
    this.client = t || We({
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
    const t = await this.client.get(te.LIST_SCHEMAS, {
      cacheable: !0,
      cacheKey: "schemas:all",
      cacheTtl: 3e5
      // 5 minutes
    });
    if (!t.success)
      return { ...t, data: [] };
    const r = t.data;
    let n = [];
    return Array.isArray(r) ? n = r : r && typeof r == "object" ? n = Object.values(r) : (typeof console < "u" && console.warn && console.warn("[schemaClient.getSchemas] Unexpected response shape; normalizing to empty array", r), n = []), { ...t, data: n };
  }
  /**
   * Get a specific schema by name
   * UNPROTECTED - No authentication required
   */
  async getSchema(t) {
    return this.client.get(te.GET_SCHEMA(t), {
      validateSchema: {
        schemaName: t,
        operation: "read",
        requiresApproved: !1
        // Allow reading any schema for inspection
      },
      cacheable: !0,
      cacheKey: `schema:${t}`,
      cacheTtl: 3e5
      // 5 minutes
    });
  }
  /**
   * Get schemas filtered by state (computed client-side)
   * UNPROTECTED - No authentication required
   */
  async getSchemasByState(t) {
    var a;
    if (!Object.values(De).includes(t))
      throw new Error(`Invalid schema state: ${t}. Must be one of: ${Object.values(De).join(", ")}`);
    const r = await this.getSchemas();
    return !r.success || !r.data ? { success: !1, error: "Failed to fetch schemas", status: r.status, data: { data: [], state: t } } : {
      success: !0,
      data: { data: r.data.filter((l) => l.state === t).map((l) => l.name), state: t },
      status: 200,
      meta: { timestamp: Date.now(), cached: ((a = r.meta) == null ? void 0 : a.cached) || !1 }
    };
  }
  /**
   * Get all schemas with their state mappings (computed client-side)
   * UNPROTECTED - No authentication required
   */
  async getAllSchemasWithState() {
    var a;
    const t = await this.getSchemas();
    if (!t.success || !t.data)
      return {
        success: !1,
        error: "Failed to fetch schemas",
        status: t.status,
        data: {}
      };
    const r = Array.isArray(t.data) ? t.data : [], n = {};
    return r.forEach((l) => {
      const d = Nl(l);
      if (!d) {
        typeof console < "u" && console.warn && console.warn("[schemaClient.getAllSchemasWithState] Encountered schema entry without a name, skipping entry.");
        return;
      }
      const c = jl(l), f = Xa(c);
      if (!c || f.length === 0) {
        typeof console < "u" && console.warn && console.warn(
          `[schemaClient.getAllSchemasWithState] Missing schema state for '${d}', defaulting to '${Or}'.`
        ), n[d] = Or;
        return;
      }
      if (!El.has(f)) {
        typeof console < "u" && console.warn && console.warn(
          `[schemaClient.getAllSchemasWithState] Unrecognized schema state '${String(c)}' for '${d}', defaulting to '${Or}'.`
        ), n[d] = Or;
        return;
      }
      n[d] = f;
    }), {
      success: !0,
      data: n,
      status: t.status ?? 200,
      meta: {
        ...t.meta,
        timestamp: Date.now(),
        cached: ((a = t.meta) == null ? void 0 : a.cached) ?? !1
      }
    };
  }
  /**
   * Get schema status summary (computed client-side)
   * UNPROTECTED - No authentication required
   */
  async getSchemaStatus() {
    var a;
    const t = await this.getSchemas();
    if (!t.success || !t.data)
      return { success: !1, error: "Failed to fetch schemas", status: t.status, data: { available: 0, approved: 0, blocked: 0, total: 0 } };
    const r = t.data;
    return { success: !0, data: {
      available: r.filter((l) => l.state === De.AVAILABLE).length,
      approved: r.filter((l) => l.state === De.APPROVED).length,
      blocked: r.filter((l) => l.state === De.BLOCKED).length,
      total: r.length
    }, status: 200, meta: { timestamp: Date.now(), cached: ((a = t.meta) == null ? void 0 : a.cached) || !1 } };
  }
  /**
   * Approve a schema (transition to approved state)
   * UNPROTECTED - No authentication required
   * SCHEMA-002 Compliance: Only available schemas can be approved
   */
  async approveSchema(t) {
    return this.client.post(
      te.APPROVE_SCHEMA(t),
      {},
      // Empty body, schema name is in URL
      {
        validateSchema: {
          schemaName: t,
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
  async blockSchema(t) {
    return this.client.post(
      te.BLOCK_SCHEMA(t),
      {},
      // Empty body, schema name is in URL
      {
        validateSchema: {
          schemaName: t,
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
    var t;
    try {
      const r = await this.getSchemas();
      return !r.success || !r.data ? { success: !1, error: "Failed to fetch schemas", status: r.status, data: [] } : { success: !0, data: r.data.filter((a) => a.state === De.APPROVED), status: 200, meta: { timestamp: Date.now(), cached: (t = r.meta) == null ? void 0 : t.cached } };
    } catch (r) {
      return { success: !1, error: r.message || "Failed to fetch approved schemas", status: r.status || 500, data: [] };
    }
  }
  /**
   * Load a schema into memory (no-op client-side; server has no endpoint)
   */
  async loadSchema(t) {
    return { success: !0, status: 200 };
  }
  /**
   * Unload a schema from memory (no-op client-side; server has no endpoint)
   */
  async unloadSchema(t) {
    return { success: !0, status: 200 };
  }
  /**
   * Validate if a schema can be used for mutations/queries (SCHEMA-002 compliance)
   */
  async validateSchemaForOperation(t, r) {
    try {
      const n = await this.getSchema(t);
      if (!n.success || !n.data)
        return {
          isValid: !1,
          error: `Schema '${t}' not found`
        };
      const a = n.data;
      return a.state !== De.APPROVED ? {
        isValid: !1,
        error: `Schema '${t}' is not approved. Current state: ${a.state}. Only approved schemas can be used for ${r}s.`,
        schema: a
      } : {
        isValid: !0,
        schema: a
      };
    } catch (n) {
      return {
        isValid: !1,
        error: `Failed to validate schema '${t}': ${n.message}`
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
  async getBackfillStatus(t) {
    return this.client.get(`/api/backfill/${t}`, {
      cacheable: !1,
      // Don't cache backfill status as it changes frequently
      timeout: 5e3
    });
  }
}
const ue = new ns();
function Sl(e) {
  return new ns(e);
}
ue.getSchemasByState.bind(ue);
ue.getAllSchemasWithState.bind(ue);
ue.getSchemaStatus.bind(ue);
ue.getSchema.bind(ue);
ue.approveSchema.bind(ue);
ue.blockSchema.bind(ue);
ue.loadSchema.bind(ue);
ue.unloadSchema.bind(ue);
ue.getApprovedSchemas.bind(ue);
ue.validateSchemaForOperation.bind(ue);
ue.getBackfillStatus.bind(ue);
const Ie = {
  APPROVE: "approve",
  BLOCK: "block",
  UNLOAD: "unload",
  LOAD: "load"
}, ei = (e, t) => e ? Date.now() - e < t : !1, _l = (e, t, r = Date.now()) => ({
  schemaName: e,
  error: t,
  timestamp: r
}), Al = (e, t, r, n) => ({
  schemaName: e,
  newState: t,
  timestamp: Date.now(),
  updatedSchema: r,
  backfillHash: n
}), as = (e, t, r, n) => Jt(
  e,
  async ({ schemaName: a, options: l = {} }, { getState: d, rejectWithValue: c }) => {
    var m;
    d().schemas.schemas[a];
    try {
      const h = await t(a);
      if (!h.success)
        throw new Error(h.error || n);
      const y = (m = h.data) == null ? void 0 : m.backfill_hash;
      return Al(a, r, void 0, y);
    } catch (h) {
      return c(
        _l(
          a,
          h instanceof Error ? h.message : n
        )
      );
    }
  }
), Le = (e, t) => ({
  pending: (r, n) => {
    const a = n.meta.arg.schemaName;
    r.loading.operations[a] = !0, delete r.errors.operations[a];
  },
  fulfilled: (r, n) => {
    const { schemaName: a, newState: l, updatedSchema: d } = n.payload;
    r.loading.operations[a] = !1, r.schemas[a] && (r.schemas[a].state = l, d && Object.assign(r.schemas[a], d), r.schemas[a].lastOperation = {
      type: t,
      timestamp: Date.now(),
      success: !0
    });
  },
  rejected: (r, n) => {
    const { schemaName: a, error: l } = n.payload;
    r.loading.operations[a] = !1, r.errors.operations[a] = l, r.schemas[a] && (r.schemas[a].lastOperation = {
      type: t,
      timestamp: Date.now(),
      success: !1,
      error: l
    });
  }
}), Zn = {
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
    ttl: ul,
    version: "1.0.0",
    lastUpdated: null
  },
  activeSchema: null
}, tt = Jt(
  xr.FETCH_SCHEMAS,
  async (e = {}, { getState: t, rejectWithValue: r }) => {
    const n = t(), { lastFetched: a, cache: l } = n.schemas;
    if (!e.forceRefresh && ei(a, l.ttl))
      return {
        schemas: Object.values(n.schemas.schemas),
        timestamp: a
      };
    const d = new ns(
      We({
        baseUrl: Xr.BASE_URL,
        // Use main API base URL (/api)
        enableCache: !0,
        enableLogging: !0,
        enableMetrics: !0
      })
    );
    e.forceRefresh && (console.log("🔄 Force refresh requested - clearing API client cache"), d.clearCache());
    let c = null;
    for (let m = 1; m <= Ns; m++)
      try {
        const h = await d.getSchemas();
        if (!h.success)
          throw new Error(`Failed to fetch schemas: ${h.error || "Unknown error"}`);
        console.log("📁 Raw schemas response:", h.data);
        const y = h.data || [];
        if (!Array.isArray(y))
          throw new Error(`Schemas response is not an array: ${typeof y}`);
        const x = y.map((S) => {
          if (!S.name)
            if (console.warn("⚠️ Schema missing name field:", S), S.schema && S.schema.name)
              S.name = S.schema.name;
            else
              return console.error("❌ Schema has no name field and cannot be displayed:", S), null;
          let E = st.AVAILABLE;
          return S.state && (typeof S.state == "string" ? E = S.state.toLowerCase() : typeof S.state == "object" && S.state.state ? E = String(S.state.state).toLowerCase() : E = String(S.state).toLowerCase()), console.log("🟢 fetchSchemas: Using backend schema for", S.name, "with state:", E), {
            ...S,
            state: E
          };
        }).filter((S) => S !== null);
        console.log("✅ Using backend schemas directly:", x.map((S) => ({ name: S.name, state: S.state })));
        const N = Date.now();
        return {
          schemas: x,
          timestamp: N
        };
      } catch (h) {
        if (c = h instanceof Error ? h : new Error("Unknown error"), m < Ns) {
          const x = typeof window < "u" && window.__TEST_ENV__ === !0 ? 10 : 1e3 * m;
          await new Promise((N) => setTimeout(N, x));
        }
      }
    const f = `Failed to fetch schemas after ${Ns} attempts: ${(c == null ? void 0 : c.message) || "Unknown error"}`;
    return r(f);
  }
), is = () => new ns(
  We({
    baseUrl: Xr.BASE_URL,
    // Use main API base URL (/api)
    enableCache: !0,
    enableLogging: !0,
    enableMetrics: !0
  })
), wt = as(
  xr.APPROVE_SCHEMA,
  (e) => is().approveSchema(e),
  st.APPROVED,
  br.APPROVE_FAILED
), Et = as(
  xr.BLOCK_SCHEMA,
  (e) => is().blockSchema(e),
  st.BLOCKED,
  br.BLOCK_FAILED
), Ut = as(
  xr.UNLOAD_SCHEMA,
  (e) => is().unloadSchema(e),
  st.AVAILABLE,
  br.UNLOAD_FAILED
), Kt = as(
  xr.LOAD_SCHEMA,
  (e) => is().loadSchema(e),
  st.APPROVED,
  br.LOAD_FAILED
), ti = Ws({
  name: "schemas",
  initialState: Zn,
  reducers: {
    /**
     * Set the currently active schema
     */
    setActiveSchema: (e, t) => {
      e.activeSchema = t.payload;
    },
    /**
     * Update a specific schema's status
     */
    updateSchemaStatus: (e, t) => {
      const { schemaName: r, newState: n } = t.payload;
      e.schemas[r] && (e.schemas[r].state = n, e.schemas[r].lastOperation = {
        type: Ie.APPROVE,
        timestamp: Date.now(),
        success: !0
      });
    },
    /**
     * Set loading state for operations
     */
    setLoading: (e, t) => {
      const { operation: r, isLoading: n, schemaName: a } = t.payload;
      r === "fetch" ? e.loading.fetch = n : a && (e.loading.operations[a] = n);
    },
    /**
     * Set error state for operations
     */
    setError: (e, t) => {
      const { operation: r, error: n, schemaName: a } = t.payload;
      r === "fetch" ? e.errors.fetch = n : a && (e.errors.operations[a] = n || "");
    },
    /**
     * Clear all errors
     */
    clearError: (e) => {
      e.errors.fetch = null, e.errors.operations = {};
    },
    /**
     * Clear error for specific operation
     */
    clearOperationError: (e, t) => {
      const r = t.payload;
      delete e.errors.operations[r];
    },
    /**
     * Invalidate cache to force next fetch
     */
    invalidateCache: (e) => {
      e.lastFetched = null, e.cache.lastUpdated = null;
    },
    /**
     * Reset all schema state
     */
    resetSchemas: (e) => {
      Object.assign(e, Zn);
    }
  },
  extraReducers: (e) => {
    e.addCase(tt.pending, (t) => {
      t.loading.fetch = !0, t.errors.fetch = null;
    }).addCase(tt.fulfilled, (t, r) => {
      t.loading.fetch = !1, t.errors.fetch = null;
      const n = {};
      r.payload.schemas.forEach((a) => {
        n[a.name] = a;
      }), t.schemas = n, t.lastFetched = r.payload.timestamp, t.cache.lastUpdated = r.payload.timestamp;
    }).addCase(tt.rejected, (t, r) => {
      t.loading.fetch = !1, t.errors.fetch = r.payload || br.FETCH_FAILED;
    }).addCase(wt.pending, Le(wt, Ie.APPROVE).pending).addCase(wt.fulfilled, Le(wt, Ie.APPROVE).fulfilled).addCase(wt.rejected, Le(wt, Ie.APPROVE).rejected).addCase(Et.pending, Le(Et, Ie.BLOCK).pending).addCase(Et.fulfilled, Le(Et, Ie.BLOCK).fulfilled).addCase(Et.rejected, Le(Et, Ie.BLOCK).rejected).addCase(Ut.pending, Le(Ut, Ie.UNLOAD).pending).addCase(Ut.fulfilled, Le(Ut, Ie.UNLOAD).fulfilled).addCase(Ut.rejected, Le(Ut, Ie.UNLOAD).rejected).addCase(Kt.pending, Le(Kt, Ie.LOAD).pending).addCase(Kt.fulfilled, Le(Kt, Ie.LOAD).fulfilled).addCase(Kt.rejected, Le(Kt, Ie.LOAD).rejected);
  }
}), Tl = (e) => e.schemas, Zt = (e) => Object.values(e.schemas.schemas), Cl = (e) => e.schemas.schemas, vr = _t(
  [Zt],
  (e) => e.filter((t) => (typeof t.state == "string" ? t.state.toLowerCase() : typeof t.state == "object" && t.state !== null && t.state.state ? String(t.state.state).toLowerCase() : String(t.state || "").toLowerCase()) === st.APPROVED)
), Rl = _t(
  [Zt],
  (e) => e.filter((t) => t.state === st.AVAILABLE)
);
_t(
  [Zt],
  (e) => e.filter((t) => t.state === st.BLOCKED)
);
_t(
  [vr],
  (e) => e.filter((t) => {
    var r;
    return ((r = t.rangeInfo) == null ? void 0 : r.isRangeSchema) === !0;
  })
);
_t(
  [Rl],
  (e) => e.filter((t) => {
    var r;
    return ((r = t.rangeInfo) == null ? void 0 : r.isRangeSchema) === !0;
  })
);
const dn = (e) => e.schemas.loading.fetch, ri = (e) => e.schemas.errors.fetch, kl = _t(
  [Tl],
  (e) => ({
    isValid: ei(e.lastFetched, e.cache.ttl),
    lastFetched: e.lastFetched,
    ttl: e.cache.ttl
  })
), Il = (e) => e.schemas.activeSchema;
_t(
  [Il, Cl],
  (e, t) => e && t[e] || null
);
const {
  setActiveSchema: ju,
  updateSchemaStatus: Su,
  setLoading: _u,
  setError: Au,
  clearError: Tu,
  clearOperationError: Cu,
  invalidateCache: Ru,
  resetSchemas: ku
} = ti.actions, Ol = ti.reducer, Xn = {
  inputText: "",
  sessionId: null,
  isProcessing: !1,
  conversationLog: [],
  showResults: !1
}, si = Ws({
  name: "aiQuery",
  initialState: Xn,
  reducers: {
    // Input management
    setInputText: (e, t) => {
      e.inputText = t.payload;
    },
    clearInputText: (e) => {
      e.inputText = "";
    },
    // Session management
    setSessionId: (e, t) => {
      e.sessionId = t.payload;
    },
    // Processing state
    setIsProcessing: (e, t) => {
      e.isProcessing = t.payload;
    },
    // Conversation management
    addMessage: (e, t) => {
      const r = {
        ...t.payload,
        timestamp: (/* @__PURE__ */ new Date()).toISOString()
      };
      e.conversationLog.push(r);
    },
    clearConversation: (e) => {
      e.conversationLog = [];
    },
    // UI state
    setShowResults: (e, t) => {
      e.showResults = t.payload;
    },
    // Combined actions
    startNewConversation: (e) => {
      e.sessionId = null, e.conversationLog = [], e.inputText = "", e.isProcessing = !1, e.showResults = !1;
    },
    // Reset all state
    resetAIQueryState: () => Xn
  }
}), {
  setInputText: ea,
  clearInputText: Iu,
  setSessionId: ta,
  setIsProcessing: ra,
  addMessage: Dl,
  clearConversation: Ou,
  setShowResults: Fl,
  startNewConversation: Pl,
  resetAIQueryState: Du
} = si.actions, Ml = si.reducer, Bl = (e) => e.aiQuery.inputText, Ll = (e) => e.aiQuery.sessionId, $l = (e) => e.aiQuery.isProcessing, Ul = (e) => e.aiQuery.conversationLog, Kl = (e) => e.aiQuery.showResults, Vl = (e) => e.aiQuery.sessionId && e.aiQuery.conversationLog.some((t) => t.type === "results"), ni = Po({
  reducer: {
    auth: dl,
    schemas: Ol,
    aiQuery: Ml
  },
  middleware: (e) => e({
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
      ignoredActionsPaths: ["payload.privateKey", "payload.schemas.definition"],
      // Ignore these paths in the state
      ignoredPaths: ["auth.privateKey", "schemas.schemas.*.definition"]
    }
  }),
  devTools: !0
  // Enable Redux DevTools for debugging
});
function Hl() {
  console.log("🔄 Schema client reset - will use new configuration on next request");
}
async function zl(e) {
  const t = Date.now(), r = e.includes("127.0.0.1") || e.includes("localhost"), n = r ? `${e}/health` : `${e}/schema`;
  try {
    const a = {
      method: r ? "GET" : "POST",
      headers: {
        "Content-Type": "application/json"
      },
      signal: AbortSignal.timeout(5e3)
      // 5 second timeout
    };
    r || (a.body = JSON.stringify({ action: "status" }));
    const l = await fetch(n, a), d = Date.now() - t;
    return l.ok ? {
      success: !0,
      status: (await l.json()).status || "online",
      responseTime: d
    } : {
      success: !1,
      error: `HTTP ${l.status}: ${l.statusText}`,
      responseTime: d
    };
  } catch (a) {
    const l = Date.now() - t;
    return {
      success: !1,
      error: a.name === "TimeoutError" ? "Connection timeout" : a.message,
      responseTime: l
    };
  }
}
const zt = {
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
}, sa = "schemaServiceEnvironment", ai = Ci({
  environment: zt.LOCAL,
  setEnvironment: () => {
  },
  getSchemaServiceBaseUrl: () => ""
});
function Gl({ children: e }) {
  const [t, r] = D(() => {
    const l = localStorage.getItem(sa);
    if (l) {
      const d = Object.values(zt).find((c) => c.id === l);
      if (d) return d;
    }
    return zt.LOCAL;
  }), n = (l) => {
    const d = Object.values(zt).find((c) => c.id === l);
    d && (r(d), localStorage.setItem(sa, l), Hl(), console.log(`Schema service environment changed to: ${d.name} (${d.baseUrl || "same origin"})`), console.log("🔄 Schema client has been reset - next request will use new endpoint"));
  }, a = () => t.baseUrl || "";
  return /* @__PURE__ */ s.jsx(ai.Provider, { value: { environment: t, setEnvironment: n, getSchemaServiceBaseUrl: a }, children: e });
}
function ql() {
  const e = Ri(ai);
  if (!e)
    throw new Error("useSchemaServiceConfig must be used within SchemaServiceConfigProvider");
  return e;
}
const Fu = ({ children: e, store: t }) => /* @__PURE__ */ s.jsx(ki, { store: t || ni, children: /* @__PURE__ */ s.jsx(Gl, { children: e }) });
function Wl({
  title: e,
  titleId: t,
  ...r
}, n) {
  return /* @__PURE__ */ Y.createElement("svg", Object.assign({
    xmlns: "http://www.w3.org/2000/svg",
    viewBox: "0 0 24 24",
    fill: "currentColor",
    "aria-hidden": "true",
    "data-slot": "icon",
    ref: n,
    "aria-labelledby": t
  }, r), e ? /* @__PURE__ */ Y.createElement("title", {
    id: t
  }, e) : null, /* @__PURE__ */ Y.createElement("path", {
    fillRule: "evenodd",
    d: "M2.25 12c0-5.385 4.365-9.75 9.75-9.75s9.75 4.365 9.75 9.75-4.365 9.75-9.75 9.75S2.25 17.385 2.25 12Zm13.36-1.814a.75.75 0 1 0-1.22-.872l-3.236 4.53L9.53 12.22a.75.75 0 0 0-1.06 1.06l2.25 2.25a.75.75 0 0 0 1.14-.094l3.75-5.25Z",
    clipRule: "evenodd"
  }));
}
const Yl = /* @__PURE__ */ Y.forwardRef(Wl);
function Ql({
  title: e,
  titleId: t,
  ...r
}, n) {
  return /* @__PURE__ */ Y.createElement("svg", Object.assign({
    xmlns: "http://www.w3.org/2000/svg",
    viewBox: "0 0 24 24",
    fill: "currentColor",
    "aria-hidden": "true",
    "data-slot": "icon",
    ref: n,
    "aria-labelledby": t
  }, r), e ? /* @__PURE__ */ Y.createElement("title", {
    id: t
  }, e) : null, /* @__PURE__ */ Y.createElement("path", {
    fillRule: "evenodd",
    d: "M12.53 16.28a.75.75 0 0 1-1.06 0l-7.5-7.5a.75.75 0 0 1 1.06-1.06L12 14.69l6.97-6.97a.75.75 0 1 1 1.06 1.06l-7.5 7.5Z",
    clipRule: "evenodd"
  }));
}
const ii = /* @__PURE__ */ Y.forwardRef(Ql);
function Jl({
  title: e,
  titleId: t,
  ...r
}, n) {
  return /* @__PURE__ */ Y.createElement("svg", Object.assign({
    xmlns: "http://www.w3.org/2000/svg",
    viewBox: "0 0 24 24",
    fill: "currentColor",
    "aria-hidden": "true",
    "data-slot": "icon",
    ref: n,
    "aria-labelledby": t
  }, r), e ? /* @__PURE__ */ Y.createElement("title", {
    id: t
  }, e) : null, /* @__PURE__ */ Y.createElement("path", {
    fillRule: "evenodd",
    d: "M16.28 11.47a.75.75 0 0 1 0 1.06l-7.5 7.5a.75.75 0 0 1-1.06-1.06L14.69 12 7.72 5.03a.75.75 0 0 1 1.06-1.06l7.5 7.5Z",
    clipRule: "evenodd"
  }));
}
const Us = /* @__PURE__ */ Y.forwardRef(Jl);
function Zl({
  title: e,
  titleId: t,
  ...r
}, n) {
  return /* @__PURE__ */ Y.createElement("svg", Object.assign({
    xmlns: "http://www.w3.org/2000/svg",
    viewBox: "0 0 24 24",
    fill: "currentColor",
    "aria-hidden": "true",
    "data-slot": "icon",
    ref: n,
    "aria-labelledby": t
  }, r), e ? /* @__PURE__ */ Y.createElement("title", {
    id: t
  }, e) : null, /* @__PURE__ */ Y.createElement("path", {
    fillRule: "evenodd",
    d: "M16.5 4.478v.227a48.816 48.816 0 0 1 3.878.512.75.75 0 1 1-.256 1.478l-.209-.035-1.005 13.07a3 3 0 0 1-2.991 2.77H8.084a3 3 0 0 1-2.991-2.77L4.087 6.66l-.209.035a.75.75 0 0 1-.256-1.478A48.567 48.567 0 0 1 7.5 4.705v-.227c0-1.564 1.213-2.9 2.816-2.951a52.662 52.662 0 0 1 3.369 0c1.603.051 2.815 1.387 2.815 2.951Zm-6.136-1.452a51.196 51.196 0 0 1 3.273 0C14.39 3.05 15 3.684 15 4.478v.113a49.488 49.488 0 0 0-6 0v-.113c0-.794.609-1.428 1.364-1.452Zm-.355 5.945a.75.75 0 1 0-1.5.058l.347 9a.75.75 0 1 0 1.499-.058l-.346-9Zm5.48.058a.75.75 0 1 0-1.498-.058l-.347 9a.75.75 0 0 0 1.5.058l.345-9Z",
    clipRule: "evenodd"
  }));
}
const na = /* @__PURE__ */ Y.forwardRef(Zl);
class oi {
  constructor(t) {
    this.client = t || We({
      enableCache: !0,
      // Cache public keys and verification results
      enableLogging: !0,
      enableMetrics: !0
    });
  }
  // Removed verifyMessage: Single-developer mono-repo; no server verify endpoint yet.
  /**
  * Get the system's public key
  * UNPROTECTED - UI never uses authentication
    * 
    * @returns Promise resolving to system public key
    */
  async getSystemPublicKey() {
    return this.client.get(
      te.GET_SYSTEM_PUBLIC_KEY,
      {
        requiresAuth: !1,
        // System public key is public
        timeout: we.QUICK,
        retries: Ee.CRITICAL,
        // Multiple retries for critical system data
        cacheable: !0,
        // Cache system public key
        cacheTtl: Nt.SYSTEM_PUBLIC_KEY,
        // Cache for 1 hour (system key doesn't change often)
        cacheKey: Vr.SYSTEM_PUBLIC_KEY
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
  validatePublicKeyFormat(t) {
    try {
      if (!t || typeof t != "string")
        return {
          isValid: !1,
          error: "Public key must be a non-empty string"
        };
      const r = t.trim();
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
    return this.client.get(
      te.GET_SYSTEM_STATUS,
      {
        timeout: we.QUICK,
        retries: Ee.STANDARD,
        cacheable: !0,
        cacheTtl: Nt.SECURITY_STATUS,
        // Cache for 1 minute
        cacheKey: Vr.SECURITY_STATUS
      }
    );
  }
  /**
   * Validate a signed message's structure before sending for verification
   * This is a client-side validation helper
   * 
   * @param signedMessage The signed message to validate
   * @returns Validation result
   */
  validateSignedMessage(t) {
    const r = [];
    if (!t || typeof t != "object")
      return r.push("Signed message must be an object"), { isValid: !1, errors: r };
    if ((!t.payload || typeof t.payload != "string") && r.push("Payload must be a non-empty base64 string"), (!t.signature || typeof t.signature != "string") && r.push("Signature must be a non-empty base64 string"), (!t.public_key_id || typeof t.public_key_id != "string") && r.push("Public key ID must be a non-empty string"), !t.timestamp || typeof t.timestamp != "number")
      r.push("Timestamp must be a Unix timestamp number");
    else {
      const a = Math.floor(Date.now() / 1e3) - t.timestamp;
      a > 300 && r.push("Message is too old (timestamp more than 5 minutes ago)"), a < -60 && r.push("Message timestamp is too far in the future");
    }
    return t.nonce && typeof t.nonce != "string" && r.push("Nonce must be a string if provided"), {
      isValid: r.length === 0,
      errors: r
    };
  }
  /**
   * Get API metrics for security operations
   */
  getMetrics() {
    return this.client.getMetrics().filter(
      (t) => t.url.includes("/security")
    );
  }
  /**
   * Clear security-related cache
   */
  clearCache() {
    this.client.clearCache();
  }
}
const St = new oi();
function Xl(e) {
  return new oi(e);
}
St.getSystemPublicKey.bind(St);
St.validatePublicKeyFormat.bind(St);
St.validateSignedMessage.bind(St);
St.getSecurityStatus.bind(St);
class ed {
  constructor(t) {
    this.client = t || We({
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
    return this.client.get(te.LIST_TRANSFORMS, {
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
    return this.client.get(te.GET_TRANSFORM_QUEUE, {
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
  async addToQueue(t) {
    if (!t || typeof t != "string")
      throw new Error("Transform ID is required and must be a string");
    return this.client.post(
      te.ADD_TO_TRANSFORM_QUEUE(t),
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
    return this.client.get(te.GET_ALL_BACKFILLS, {
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
    return this.client.get(te.GET_ACTIVE_BACKFILLS, {
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
  async getBackfill(t) {
    if (!t || typeof t != "string")
      throw new Error("Transform ID is required and must be a string");
    return this.client.get(te.GET_BACKFILL(t), {
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
    return this.client.get(te.GET_TRANSFORM_STATISTICS, {
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
    return this.client.get(te.GET_BACKFILL_STATISTICS, {
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
  async getTransform(t) {
    if (!t || typeof t != "string")
      throw new Error("Transform ID is required and must be a string");
    const r = await this.getTransforms();
    if (r.success && r.data) {
      const n = r.data[t] || null;
      return {
        ...r,
        data: n
      };
    }
    return r;
  }
  /**
   * Get API metrics for transform operations
   */
  getMetrics() {
    return this.client.getMetrics().filter(
      (t) => t.url.includes("/transforms") || t.url.includes("/queue")
    );
  }
  /**
   * Clear transform-related cache
   */
  clearCache() {
    this.client.clearCache();
  }
}
const Ae = new ed();
Ae.getTransforms.bind(Ae);
Ae.getQueue.bind(Ae);
Ae.addToQueue.bind(Ae);
Ae.refreshQueue.bind(Ae);
Ae.getTransform.bind(Ae);
class ci {
  constructor(t) {
    this.client = t || We({
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
  async executeMutation(t) {
    return this.client.post(
      te.EXECUTE_MUTATION,
      t,
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
  async executeMutationsBatch(t) {
    return this.client.post(
      te.EXECUTE_MUTATIONS_BATCH,
      t,
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
  async executeQuery(t) {
    return this.client.post(
      te.EXECUTE_QUERY,
      t,
      {
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
      }
    );
  }
  /**
   * Validate a mutation before execution
   * This checks schema compliance, field validation, and business rules
   * 
   * @param mutation The mutation object to validate
   * @returns Promise resolving to validation result
   */
  async validateMutation(t) {
    return Promise.resolve({ success: !0, data: { isValid: !0 }, status: 200 });
  }
  /**
   * Execute a batch of mutations as a single transaction
   * All mutations must target approved schemas
   * 
   * @param mutations Array of mutation objects
   * @returns Promise resolving to batch execution results
   */
  async executeBatchMutations(t) {
    return { success: !1, error: "Batch mutations not supported", status: 501, data: [] };
  }
  /**
   * Execute a parameterized query with filters and pagination
   * Provides enhanced query capabilities beyond basic executeQuery
   * 
   * @param queryParams Query parameters including schema, filters, pagination
   * @returns Promise resolving to enhanced query results
   */
  async executeParameterizedQuery(t) {
    return this.client.post(te.EXECUTE_QUERY, t, {
      validateSchema: {
        schemaName: t.schema,
        operation: "read",
        requiresApproved: !0
      },
      timeout: 15e3,
      retries: 2,
      cacheable: !0,
      cacheTtl: 12e4,
      cacheKey: `parameterized-query:${JSON.stringify(t)}`
    });
  }
  /**
   * Get mutation history for a specific record or schema
   * Useful for auditing and tracking changes
   * 
   * @param params History query parameters
   * @returns Promise resolving to mutation history
   */
  async getMutationHistory(t) {
    return { success: !1, error: "Mutation history not supported", status: 501, data: [] };
  }
  /**
   * Check if a schema is available for mutations (SCHEMA-002 compliance)
   * 
   * @param schemaName The name of the schema to check
   * @returns Promise resolving to schema availability info
   */
  async validateSchemaForMutation(t) {
    try {
      const r = await this.client.get(te.GET_SCHEMA(t), {
        timeout: 5e3,
        retries: 1,
        cacheable: !0,
        cacheTtl: 18e4
        // Cache schema state for 3 minutes
      });
      if (!r.success || !r.data)
        return {
          isValid: !1,
          schemaState: "unknown",
          canMutate: !1,
          canQuery: !1,
          error: `Schema '${t}' not found`
        };
      const n = r.data, a = n.state === De.APPROVED;
      return {
        isValid: !0,
        schemaState: n.state,
        canMutate: a,
        canQuery: a,
        error: a ? void 0 : `Schema '${t}' is not approved (current state: ${n.state})`
      };
    } catch (r) {
      return {
        isValid: !1,
        schemaState: "error",
        canMutate: !1,
        canQuery: !1,
        error: `Failed to validate schema '${t}': ${r.message}`
      };
    }
  }
  /**
   * Get API metrics for mutation operations
   */
  getMetrics() {
    return this.client.getMetrics().filter(
      (t) => t.url.includes("/mutation") || t.url.includes("/query")
    );
  }
  /**
   * Clear any cached query results
   */
  clearCache() {
    this.client.clearCache();
  }
}
const un = new ci();
function td(e) {
  return new ci(e);
}
class rd {
  constructor(t) {
    this.client = t || We({
      baseUrl: sc.ROOT,
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
    return this.client.get(
      te.GET_STATUS,
      {
        requiresAuth: !1,
        // Status endpoint is public
        timeout: we.QUICK,
        retries: Ee.STANDARD,
        cacheable: !1
        // Status should always be fresh
      }
    );
  }
  /**
   * Get ingestion configuration
   * UNPROTECTED - No authentication required
   * 
   * @returns Promise resolving to general ingestion configuration
   */
  async getConfig() {
    return this.client.get(
      te.GET_INGESTION_CONFIG,
      {
        timeout: we.QUICK,
        retries: Ee.STANDARD,
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
  async saveConfig(t) {
    return this.client.post(
      te.GET_INGESTION_CONFIG,
      t,
      {
        timeout: we.CONFIG,
        // Longer timeout for config operations
        retries: Ee.LIMITED,
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
  async validateData(t) {
    return this.client.post(
      te.VALIDATE_JSON,
      t,
      {
        requiresAuth: !1,
        // Validation is a utility operation
        timeout: we.MUTATION,
        // Longer timeout for AI analysis
        retries: Ee.STANDARD,
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
  async processIngestion(t, r = {}) {
    const n = {
      data: t,
      auto_execute: r.autoExecute ?? !0,
      trust_distance: r.trustDistance ?? 0,
      pub_key: r.pubKey ?? "default"
    }, a = this.validateIngestionRequest(n);
    if (!a.isValid)
      throw new Error(`Invalid ingestion request: ${a.errors.join(", ")}`);
    return this.client.post(
      te.PROCESS_JSON,
      n,
      {
        timeout: we.AI_PROCESSING,
        // Extended timeout for AI processing (60 seconds)
        retries: Ee.LIMITED,
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
  validateIngestionRequest(t) {
    const r = [], n = [];
    return !t.data || typeof t.data != "object" ? r.push("Data must be a valid object") : Object.keys(t.data).length === 0 && r.push("Data cannot be empty"), typeof t.trust_distance != "number" || t.trust_distance < 0 ? r.push("Trust distance must be a non-negative number") : t.trust_distance > 10 && n.push("Trust distance is unusually high"), (!t.pub_key || t.pub_key.trim().length === 0) && r.push("Public key is required"), typeof t.auto_execute != "boolean" && r.push("Auto execute must be a boolean value"), {
      isValid: r.length === 0,
      errors: r,
      warnings: n
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
  createIngestionRequest(t, r = {}) {
    return {
      data: { ...t },
      // Create a copy
      auto_execute: r.autoExecute ?? !0,
      trust_distance: r.trustDistance ?? 0,
      pub_key: r.pubKey ?? "default"
    };
  }
  /**
   * Get API metrics for ingestion operations
   */
  getMetrics() {
    return this.client.getMetrics().filter(
      (t) => t.url.includes("/ingestion")
    );
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
  async getProgress(t) {
    return this.client.get(
      `/ingestion/progress/${t}`,
      {
        requiresAuth: !1,
        timeout: we.QUICK,
        retries: Ee.STANDARD,
        cacheable: !1
      }
    );
  }
  /**
   * Get all active ingestion progress
   * 
   * @returns Promise resolving to all active progress
   */
  async getAllProgress() {
    return this.client.get(
      "/ingestion/progress",
      {
        requiresAuth: !1,
        timeout: we.QUICK,
        retries: Ee.STANDARD,
        cacheable: !1
      }
    );
  }
  clearCache() {
    this.client.clearCache();
  }
}
const jt = new rd(), Vt = We({
  timeout: we.AI_PROCESSING,
  retries: Ee.LIMITED
}), aa = {
  /**
   * Run a query in a single step (analyze + execute with internal polling loop)
   */
  async runQuery(e) {
    return Vt.post("/llm-query/run", e);
  },
  /**
   * Analyze a natural language query
   */
  async analyzeQuery(e) {
    return Vt.post("/llm-query/analyze", e);
  },
  /**
   * Execute a query plan
   */
  async executeQueryPlan(e) {
    return Vt.post("/llm-query/execute", e);
  },
  /**
   * Ask a follow-up question about results
   */
  async chat(e) {
    return Vt.post("/llm-query/chat", e);
  },
  /**
   * Analyze if a follow-up question can be answered from existing context
   */
  async analyzeFollowup(e) {
    return Vt.post("/llm-query/analyze-followup", e);
  },
  /**
   * Get backfill status by hash
   */
  async getBackfillStatus(e) {
    return Vt.get(`/llm-query/backfill/${e}`);
  }
};
class sd {
  constructor(t) {
    this.client = t || We({ enableCache: !0, enableLogging: !0 });
  }
  async search(t) {
    const r = `${te.NATIVE_INDEX_SEARCH}?term=${encodeURIComponent(t)}`;
    return this.client.get(r, {
      timeout: 8e3,
      retries: 2,
      cacheable: !0,
      cacheTtl: 6e4
    });
  }
}
const nd = new sd();
async function ad() {
  return (await fc.get(
    te.GET_INDEXING_STATUS,
    { cacheable: !1 }
    // Don't cache - we need real-time status updates
  )).data;
}
function id(e = 1e3) {
  const [t, r] = Y.useState(null), [n, a] = Y.useState(null);
  return Y.useEffect(() => {
    let l = !0, d;
    const c = async () => {
      try {
        const m = await ad();
        l && (r(m), a(null));
      } catch (m) {
        l && a(m instanceof Error ? m : new Error("Failed to fetch indexing status"));
      }
    };
    c();
    const f = async () => {
      if (await c(), l) {
        const m = (t == null ? void 0 : t.state) === "Indexing" ? e : e * 5;
        d = setTimeout(f, m);
      }
    };
    return d = setTimeout(f, e), () => {
      l = !1, d && clearTimeout(d);
    };
  }, [e, t == null ? void 0 : t.state]), { status: t, error: n };
}
function Pu() {
  const [e, t] = D(!1), [r, n] = D(!1), [a, l] = D(null), [d, c] = D(null), [f, m] = D(null);
  xe(() => {
    const v = (w) => {
      console.log("🔵 StatusSection: Received ingestion-started event", w.detail), m(w.detail.progressId), c({
        progress_percentage: 0,
        status_message: "Starting ingestion...",
        is_complete: !1
      }), console.log("🔵 StatusSection: Set initial ingestion progress");
    };
    return window.addEventListener("ingestion-started", v), console.log("🔵 StatusSection: Listening for ingestion-started events"), () => window.removeEventListener("ingestion-started", v);
  }, []), xe(() => {
    if (!f) return;
    let v = !0, w;
    const A = async () => {
      try {
        const _ = await jt.getProgress(f);
        v && _.success && _.data ? (c(_.data), _.data.is_complete ? m(null) : w = setTimeout(A, 200)) : v && (w = setTimeout(A, 200));
      } catch (_) {
        console.error("Error polling ingestion:", _), v && (w = setTimeout(A, 500));
      }
    };
    return A(), () => {
      v = !1, w && clearTimeout(w);
    };
  }, [f]);
  const { status: h } = id(1e3), y = async () => {
    n(!0), l(null);
    try {
      const v = await me.resetDatabase(!0);
      v.success && v.data ? (l({ type: "success", message: v.data.message }), setTimeout(() => {
        window.location.reload();
      }, 2e3)) : l({ type: "error", message: v.error || "Reset failed" });
    } catch (v) {
      l({ type: "error", message: `Network error: ${v.message}` });
    } finally {
      n(!1), t(!1);
    }
  }, x = () => e ? /* @__PURE__ */ s.jsx("div", { className: "fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50", children: /* @__PURE__ */ s.jsxs("div", { className: "bg-white rounded-lg p-6 max-w-md w-full mx-4", children: [
    /* @__PURE__ */ s.jsxs("div", { className: "flex items-center gap-3 mb-4", children: [
      /* @__PURE__ */ s.jsx(na, { className: "w-6 h-6 text-red-500" }),
      /* @__PURE__ */ s.jsx("h3", { className: "text-lg font-semibold text-gray-900", children: "Reset Database" })
    ] }),
    /* @__PURE__ */ s.jsxs("div", { className: "mb-6", children: [
      /* @__PURE__ */ s.jsx("p", { className: "text-gray-700 mb-2", children: "This will permanently delete all data and restart the node:" }),
      /* @__PURE__ */ s.jsxs("ul", { className: "list-disc list-inside text-sm text-gray-600 space-y-1", children: [
        /* @__PURE__ */ s.jsx("li", { children: "All schemas will be removed" }),
        /* @__PURE__ */ s.jsx("li", { children: "All stored data will be deleted" }),
        /* @__PURE__ */ s.jsx("li", { children: "Network connections will be reset" }),
        /* @__PURE__ */ s.jsx("li", { children: "This action cannot be undone" })
      ] })
    ] }),
    /* @__PURE__ */ s.jsxs("div", { className: "flex gap-3 justify-end", children: [
      /* @__PURE__ */ s.jsx(
        "button",
        {
          onClick: () => t(!1),
          className: "px-4 py-2 text-sm font-medium text-gray-700 bg-gray-100 rounded-md hover:bg-gray-200 transition-colors",
          disabled: r,
          children: "Cancel"
        }
      ),
      /* @__PURE__ */ s.jsx(
        "button",
        {
          onClick: y,
          disabled: r,
          className: "px-4 py-2 text-sm font-medium text-white bg-red-600 rounded-md hover:bg-red-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors",
          children: r ? "Resetting..." : "Reset Database"
        }
      )
    ] })
  ] }) }) : null, N = () => {
    if (console.log("🟡 StatusSection getIngestionStatus:", {
      hasProgress: !!d,
      percentage: d == null ? void 0 : d.progress_percentage,
      isComplete: d == null ? void 0 : d.is_complete,
      results: d == null ? void 0 : d.results
    }), d && !d.is_complete) {
      const v = d.started_at ? Math.floor((/* @__PURE__ */ new Date() - new Date(d.started_at)) / 1e3) : 0;
      return {
        state: "active",
        title: "Ingesting Data",
        detail: d.status_message,
        percentage: d.progress_percentage,
        metrics: v > 0 ? [`${v}s elapsed`] : [],
        color: "blue"
      };
    }
    if (d != null && d.is_complete && (d != null && d.results)) {
      const v = d.started_at && d.completed_at ? Math.floor((new Date(d.completed_at) - new Date(d.started_at)) / 1e3) : 0;
      return {
        state: "completed",
        title: "Ingestion",
        detail: "Last ingestion completed",
        metrics: [
          `${d.results.mutations_executed || 0} items ingested`,
          v > 0 ? `${v}s duration` : null
        ].filter(Boolean),
        color: "green"
      };
    }
    return {
      state: "idle",
      title: "Ingestion",
      detail: "No active ingestion",
      metrics: [],
      color: "gray"
    };
  }, S = () => (console.log("🟡 StatusSection getIndexingStatus:", {
    indexingState: h == null ? void 0 : h.state,
    totalOps: h == null ? void 0 : h.total_operations_processed
  }), (h == null ? void 0 : h.state) === "Indexing" ? {
    state: "active",
    title: "Background Indexing",
    detail: "Actively processing index operations",
    metrics: [
      `${h.total_operations_processed.toLocaleString()} ops processed`,
      `${h.operations_per_second.toFixed(0)} ops/sec`
    ],
    color: "indigo"
  } : (h == null ? void 0 : h.total_operations_processed) > 0 ? {
    state: "completed",
    title: "Indexing",
    detail: "All operations indexed",
    metrics: [`${h.total_operations_processed.toLocaleString()} total operations`],
    color: "green"
  } : {
    state: "idle",
    title: "Indexing",
    detail: "No indexing activity",
    metrics: [],
    color: "gray"
  }), E = N(), p = S();
  return /* @__PURE__ */ s.jsxs(s.Fragment, { children: [
    /* @__PURE__ */ s.jsxs("div", { className: "bg-white rounded-lg shadow-sm p-4 mb-6", children: [
      /* @__PURE__ */ s.jsxs("div", { className: "flex items-center justify-between mb-4", children: [
        /* @__PURE__ */ s.jsxs("div", { className: "flex items-center gap-2", children: [
          /* @__PURE__ */ s.jsx(Yl, { className: "w-5 h-5 text-green-500" }),
          /* @__PURE__ */ s.jsx("h2", { className: "text-lg font-semibold text-gray-900", children: "System Status" })
        ] }),
        /* @__PURE__ */ s.jsxs(
          "button",
          {
            onClick: () => t(!0),
            className: "flex items-center gap-2 px-3 py-1.5 text-sm font-medium text-red-600 border border-red-200 rounded-md hover:bg-red-50 hover:border-red-300 transition-colors",
            disabled: r,
            children: [
              /* @__PURE__ */ s.jsx(na, { className: "w-4 h-4" }),
              "Reset Database"
            ]
          }
        )
      ] }),
      /* @__PURE__ */ s.jsxs("div", { className: "grid grid-cols-1 md:grid-cols-2 gap-4", children: [
        /* @__PURE__ */ s.jsxs("div", { className: `p-4 rounded-lg border-2 ${E.state === "active" ? "border-blue-200 bg-blue-50" : E.state === "completed" ? "border-green-200 bg-green-50" : "border-gray-200 bg-gray-50"}`, children: [
          /* @__PURE__ */ s.jsxs("div", { className: "flex items-center justify-between mb-2", children: [
            /* @__PURE__ */ s.jsxs("div", { className: "flex items-center gap-2", children: [
              /* @__PURE__ */ s.jsx("div", { className: `w-2.5 h-2.5 rounded-full ${E.state === "active" ? "bg-blue-500 animate-pulse" : E.state === "completed" ? "bg-green-500" : "bg-gray-400"}` }),
              /* @__PURE__ */ s.jsx("h3", { className: `font-semibold ${E.state === "active" ? "text-blue-900" : E.state === "completed" ? "text-green-900" : "text-gray-700"}`, children: E.title })
            ] }),
            /* @__PURE__ */ s.jsx("span", { className: `text-xs font-medium px-2 py-1 rounded ${E.state === "active" ? "bg-blue-100 text-blue-700" : E.state === "completed" ? "bg-green-100 text-green-700" : "bg-gray-200 text-gray-600"}`, children: E.state === "active" ? "Active" : E.state === "completed" ? "Complete" : "Idle" })
          ] }),
          /* @__PURE__ */ s.jsx("p", { className: `text-sm ${E.state === "active" ? "text-blue-700" : E.state === "completed" ? "text-green-700" : "text-gray-500"}`, children: E.detail }),
          E.metrics && E.metrics.length > 0 && /* @__PURE__ */ s.jsx("div", { className: "mt-2 flex flex-wrap gap-2", children: E.metrics.map((v, w) => /* @__PURE__ */ s.jsx("span", { className: `text-xs font-medium px-2 py-1 rounded ${E.state === "active" ? "bg-blue-100 text-blue-800" : E.state === "completed" ? "bg-green-100 text-green-800" : "bg-gray-100 text-gray-600"}`, children: v }, w)) }),
          E.percentage !== void 0 && /* @__PURE__ */ s.jsxs("div", { className: "mt-3", children: [
            /* @__PURE__ */ s.jsxs("div", { className: "flex items-center justify-between mb-1", children: [
              /* @__PURE__ */ s.jsx("span", { className: "text-xs font-medium text-blue-700", children: "Progress" }),
              /* @__PURE__ */ s.jsxs("span", { className: "text-xs font-semibold text-blue-900", children: [
                E.percentage,
                "%"
              ] })
            ] }),
            /* @__PURE__ */ s.jsx("div", { className: "w-full bg-blue-200 rounded-full h-2", children: /* @__PURE__ */ s.jsx(
              "div",
              {
                className: "bg-blue-600 h-2 rounded-full transition-all duration-300",
                style: { width: `${E.percentage}%` }
              }
            ) })
          ] })
        ] }),
        /* @__PURE__ */ s.jsxs("div", { className: `p-4 rounded-lg border-2 ${p.state === "active" ? "border-indigo-200 bg-indigo-50" : p.state === "completed" ? "border-green-200 bg-green-50" : "border-gray-200 bg-gray-50"}`, children: [
          /* @__PURE__ */ s.jsxs("div", { className: "flex items-center justify-between mb-2", children: [
            /* @__PURE__ */ s.jsxs("div", { className: "flex items-center gap-2", children: [
              /* @__PURE__ */ s.jsx("div", { className: `w-2.5 h-2.5 rounded-full ${p.state === "active" ? "bg-indigo-500 animate-pulse" : p.state === "completed" ? "bg-green-500" : "bg-gray-400"}` }),
              /* @__PURE__ */ s.jsx("h3", { className: `font-semibold ${p.state === "active" ? "text-indigo-900" : p.state === "completed" ? "text-green-900" : "text-gray-700"}`, children: p.title })
            ] }),
            /* @__PURE__ */ s.jsx("span", { className: `text-xs font-medium px-2 py-1 rounded ${p.state === "active" ? "bg-indigo-100 text-indigo-700" : p.state === "completed" ? "bg-green-100 text-green-700" : "bg-gray-200 text-gray-600"}`, children: p.state === "active" ? "Active" : p.state === "completed" ? "Complete" : "Idle" })
          ] }),
          /* @__PURE__ */ s.jsx("p", { className: `text-sm ${p.state === "active" ? "text-indigo-700" : p.state === "completed" ? "text-green-700" : "text-gray-500"}`, children: p.detail }),
          p.metrics && p.metrics.length > 0 && /* @__PURE__ */ s.jsx("div", { className: "mt-2 flex flex-wrap gap-2", children: p.metrics.map((v, w) => /* @__PURE__ */ s.jsx("span", { className: `text-xs font-medium px-2 py-1 rounded ${p.state === "active" ? "bg-indigo-100 text-indigo-800" : p.state === "completed" ? "bg-green-100 text-green-800" : "bg-gray-100 text-gray-600"}`, children: v }, w)) })
        ] })
      ] }),
      a && /* @__PURE__ */ s.jsx("div", { className: `mt-3 p-3 rounded-md text-sm ${a.type === "success" ? "bg-green-50 text-green-800 border border-green-200" : "bg-red-50 text-red-800 border border-red-200"}`, children: a.message })
    ] }),
    /* @__PURE__ */ s.jsx(x, {})
  ] });
}
function He(e) {
  return e !== null && typeof e == "object" && !Array.isArray(e);
}
function od(e) {
  const t = Xt(e);
  if (!He(t)) return !1;
  const r = Object.keys(t);
  if (r.length === 0) return !1;
  for (let n = 0; n < Math.min(3, r.length); n++) {
    const a = t[r[n]];
    if (!He(a)) return !1;
    const l = Object.keys(a);
    if (l.length !== 0)
      for (let d = 0; d < Math.min(3, l.length); d++) {
        const c = a[l[d]];
        if (!He(c)) return !1;
        Object.keys(c).length;
      }
  }
  return !0;
}
function Xt(e) {
  return e && He(e) && Object.prototype.hasOwnProperty.call(e, "data") ? e.data : e;
}
function cd(e) {
  const t = Xt(e) || {};
  if (!He(t)) return { hashes: 0, ranges: 0 };
  const r = Object.keys(t).length;
  let n = 0;
  for (const a of Object.keys(t)) {
    const l = t[a];
    He(l) && (n += Object.keys(l).length);
  }
  return { hashes: r, ranges: n };
}
function ld(e) {
  const t = Xt(e) || {};
  return He(t) ? Object.keys(t).sort(di) : [];
}
function li(e, t) {
  const r = Xt(e) || {}, n = He(r) && He(r[t]) ? r[t] : {};
  return Object.keys(n).sort(di);
}
function di(e, t) {
  const r = ia(e), n = ia(t);
  return !Number.isNaN(r) && !Number.isNaN(n) ? r - n : String(e).localeCompare(String(t));
}
function ia(e) {
  const t = Number(e);
  return Number.isFinite(t) ? t : Number.NaN;
}
function dd(e, t, r) {
  const n = Xt(e) || {};
  if (!He(n)) return null;
  const a = n[t];
  if (!He(a)) return null;
  const l = a[r];
  return He(l) ? l : null;
}
function ui(e, t, r) {
  return e.slice(t, Math.min(t + r, e.length));
}
const ud = 50;
function fi({ isOpen: e, onClick: t, label: r }) {
  return /* @__PURE__ */ s.jsxs(
    "button",
    {
      type: "button",
      className: "text-left w-full flex items-center justify-between px-3 py-2 hover:bg-gray-100 rounded",
      onClick: t,
      "aria-expanded": e,
      children: [
        /* @__PURE__ */ s.jsx("span", { className: "font-mono text-sm text-gray-800 truncate", children: r }),
        /* @__PURE__ */ s.jsx("span", { className: "ml-2 text-gray-500 text-xs", children: e ? "▼" : "▶" })
      ]
    }
  );
}
function fd({ fields: e }) {
  const t = ye(() => Object.entries(e || {}), [e]);
  return t.length === 0 ? /* @__PURE__ */ s.jsx("div", { className: "text-xs text-gray-500 italic px-3 py-2", children: "No fields" }) : /* @__PURE__ */ s.jsx("div", { className: "px-3 py-2 overflow-x-auto", children: /* @__PURE__ */ s.jsx("table", { className: "min-w-full border-separate border-spacing-y-1", children: /* @__PURE__ */ s.jsx("tbody", { children: t.map(([r, n]) => /* @__PURE__ */ s.jsxs("tr", { className: "bg-white", children: [
    /* @__PURE__ */ s.jsx("td", { className: "align-top text-xs font-medium text-gray-700 pr-4 whitespace-nowrap", children: r }),
    /* @__PURE__ */ s.jsx("td", { className: "align-top text-xs text-gray-700", children: /* @__PURE__ */ s.jsx("pre", { className: "font-mono whitespace-pre-wrap break-words", children: hd(n) }) })
  ] }, r)) }) }) });
}
function hd(e) {
  if (e === null) return "null";
  if (typeof e == "string") return e;
  if (typeof e == "number" || typeof e == "boolean") return String(e);
  try {
    return JSON.stringify(e, null, 2);
  } catch {
    return String(e);
  }
}
function md({ results: e, pageSize: t = ud }) {
  const r = ye(() => Xt(e) || {}, [e]), n = ye(() => cd(e), [e]), a = ye(() => ld(e), [e]), [l, d] = D(() => /* @__PURE__ */ new Set()), [c, f] = D(() => /* @__PURE__ */ new Set()), [m, h] = D({ start: 0, count: t }), [y, x] = D(() => /* @__PURE__ */ new Map()), N = H((v) => {
    d((w) => {
      const A = new Set(w);
      return A.has(v) ? A.delete(v) : A.add(v), A;
    }), x((w) => {
      if (!l.has(v)) {
        const A = li(r, v).length, _ = new Map(w);
        return _.set(v, { start: 0, count: Math.min(t, A) }), _;
      }
      return w;
    });
  }, [r, l, t]), S = H((v, w) => {
    const A = v + "||" + w;
    f((_) => {
      const T = new Set(_);
      return T.has(A) ? T.delete(A) : T.add(A), T;
    });
  }, []), E = H(() => {
    const v = Math.min(a.length, m.count + t);
    h((w) => ({ start: 0, count: v }));
  }, [a, m.count, t]), p = ye(() => ui(a, m.start, m.count), [a, m]);
  return /* @__PURE__ */ s.jsxs("div", { className: "space-y-2", children: [
    /* @__PURE__ */ s.jsxs("div", { className: "text-xs text-gray-600", children: [
      /* @__PURE__ */ s.jsxs("span", { className: "mr-4", children: [
        "Hashes: ",
        /* @__PURE__ */ s.jsx("strong", { children: n.hashes })
      ] }),
      /* @__PURE__ */ s.jsxs("span", { children: [
        "Ranges: ",
        /* @__PURE__ */ s.jsx("strong", { children: n.ranges })
      ] })
    ] }),
    /* @__PURE__ */ s.jsx("div", { className: "border rounded-md divide-y divide-gray-200 bg-gray-50", children: p.map((v) => /* @__PURE__ */ s.jsxs("div", { className: "p-2", children: [
      /* @__PURE__ */ s.jsx(
        fi,
        {
          isOpen: l.has(v),
          onClick: () => N(v),
          label: `hash: ${String(v)}`
        }
      ),
      l.has(v) && /* @__PURE__ */ s.jsx(
        pd,
        {
          data: r,
          hashKey: v,
          rangeOpen: c,
          onToggleRange: S,
          pageSize: t,
          rangeWindow: y.get(v),
          setRangeWindow: (w) => x((A) => new Map(A).set(v, w))
        }
      )
    ] }, v)) }),
    m.count < a.length && /* @__PURE__ */ s.jsx("div", { className: "pt-2", children: /* @__PURE__ */ s.jsxs(
      "button",
      {
        type: "button",
        className: "text-xs px-3 py-1 rounded bg-gray-200 hover:bg-gray-300",
        onClick: E,
        children: [
          "Show more hashes (",
          m.count,
          "/",
          a.length,
          ")"
        ]
      }
    ) })
  ] });
}
function pd({ data: e, hashKey: t, rangeOpen: r, onToggleRange: n, pageSize: a, rangeWindow: l, setRangeWindow: d }) {
  const c = ye(() => li(e, t), [e, t]), f = l || { start: 0, count: Math.min(a, c.length) }, m = ye(() => ui(c, f.start, f.count), [c, f]), h = H(() => {
    const y = Math.min(c.length, f.count + a);
    d({ start: 0, count: y });
  }, [c.length, f.count, a, d]);
  return /* @__PURE__ */ s.jsxs("div", { className: "ml-4 mt-1 border-l pl-3", children: [
    m.map((y) => /* @__PURE__ */ s.jsxs("div", { className: "py-1", children: [
      /* @__PURE__ */ s.jsx(
        fi,
        {
          isOpen: r.has(t + "||" + y),
          onClick: () => n(t, y),
          label: `range: ${String(y)}`
        }
      ),
      r.has(t + "||" + y) && /* @__PURE__ */ s.jsx("div", { className: "ml-4 mt-1", children: /* @__PURE__ */ s.jsx(fd, { fields: dd(e, t, y) || {} }) })
    ] }, y)),
    f.count < c.length && /* @__PURE__ */ s.jsx("div", { className: "pt-1", children: /* @__PURE__ */ s.jsxs(
      "button",
      {
        type: "button",
        className: "text-xs px-3 py-1 rounded bg-gray-200 hover:bg-gray-300",
        onClick: h,
        children: [
          "Show more ranges (",
          f.count,
          "/",
          c.length,
          ")"
        ]
      }
    ) })
  ] });
}
function Mu({ results: e }) {
  const t = e != null, r = t && (!!e.error || e.status && e.status >= 400), n = t && e.data !== void 0, a = ye(() => t && !r && od(n ? e.data : e), [t, e, r, n]), [l, d] = D(a);
  return t ? /* @__PURE__ */ s.jsxs("div", { className: "bg-white rounded-lg shadow-sm p-6 mt-6", children: [
    /* @__PURE__ */ s.jsxs("h3", { className: "text-lg font-semibold mb-4 flex items-center", children: [
      /* @__PURE__ */ s.jsx("span", { className: `mr-2 ${r ? "text-red-600" : "text-gray-900"}`, children: r ? "Error" : "Results" }),
      /* @__PURE__ */ s.jsxs("span", { className: "text-xs font-normal text-gray-500", children: [
        "(",
        typeof e == "string" ? "Text" : l ? "Structured" : "JSON",
        ")"
      ] }),
      e.status && /* @__PURE__ */ s.jsxs("span", { className: `ml-2 px-2 py-1 text-xs rounded-full ${e.status >= 400 ? "bg-red-100 text-red-800" : "bg-green-100 text-green-800"}`, children: [
        "Status: ",
        e.status
      ] }),
      !r && typeof e != "string" && /* @__PURE__ */ s.jsx("div", { className: "ml-auto", children: /* @__PURE__ */ s.jsx(
        "button",
        {
          type: "button",
          className: "text-xs px-2 py-1 rounded border border-gray-300 hover:bg-gray-100",
          onClick: () => d((c) => !c),
          children: l ? "View JSON" : "View Structured"
        }
      ) })
    ] }),
    r && /* @__PURE__ */ s.jsx("div", { className: "mb-4 p-4 bg-red-50 border border-red-200 rounded-md", children: /* @__PURE__ */ s.jsxs("div", { className: "flex", children: [
      /* @__PURE__ */ s.jsx("div", { className: "flex-shrink-0", children: /* @__PURE__ */ s.jsx("svg", { className: "h-5 w-5 text-red-400", viewBox: "0 0 20 20", fill: "currentColor", children: /* @__PURE__ */ s.jsx("path", { fillRule: "evenodd", d: "M10 18a8 8 0 100-16 8 8 0 000 16zM8.707 7.293a1 1 0 00-1.414 1.414L8.586 10l-1.293 1.293a1 1 0 101.414 1.414L10 11.414l1.293 1.293a1 1 0 001.414-1.414L11.414 10l1.293-1.293a1 1 0 00-1.414-1.414L10 8.586 8.707 7.293z", clipRule: "evenodd" }) }) }),
      /* @__PURE__ */ s.jsxs("div", { className: "ml-3", children: [
        /* @__PURE__ */ s.jsx("h4", { className: "text-sm font-medium text-red-800", children: "Query Execution Failed" }),
        /* @__PURE__ */ s.jsx("div", { className: "mt-2 text-sm text-red-700", children: /* @__PURE__ */ s.jsx("p", { children: e.error || "An unknown error occurred" }) })
      ] })
    ] }) }),
    l && !r && typeof e != "string" ? /* @__PURE__ */ s.jsx("div", { className: "rounded-md p-2 bg-gray-50 border overflow-auto max-h-[500px]", children: /* @__PURE__ */ s.jsx(md, { results: e }) }) : /* @__PURE__ */ s.jsx("div", { className: `rounded-md p-4 overflow-auto max-h-[500px] ${r ? "bg-red-50 border border-red-200" : "bg-gray-50"}`, children: /* @__PURE__ */ s.jsx("pre", { className: `font-mono text-sm whitespace-pre-wrap ${r ? "text-red-700" : "text-gray-700"}`, children: typeof e == "string" ? e : JSON.stringify(n ? e.data : e, null, 2) }) })
  ] }) : null;
}
const Ke = {
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
function Bu({
  tabs: e = pl,
  activeTab: t,
  onTabChange: r,
  className: n = ""
}) {
  const a = (m, h) => {
    r(m);
  }, l = (m) => {
    const h = t === m.id, y = m.disabled || !1;
    let x = Ke.tab.base;
    return h ? x += ` ${Ke.tab.active}` : y ? x += ` ${Ke.tab.disabled}` : x += ` ${Ke.tab.inactive}`, x;
  }, d = e.filter((m) => m.group === "main"), c = e.filter((m) => m.group === "advanced"), f = (m) => {
    const h = m.disabled || !1;
    return /* @__PURE__ */ s.jsxs(
      "button",
      {
        className: l(m),
        onClick: () => a(m.id, m.requiresAuth),
        disabled: h,
        "aria-current": t === m.id ? "page" : void 0,
        "aria-label": `${m.label} tab`,
        style: {
          transitionDuration: `${hl}ms`
        },
        children: [
          m.icon && /* @__PURE__ */ s.jsx("span", { className: "mr-2", "aria-hidden": "true", children: m.icon }),
          /* @__PURE__ */ s.jsx("span", { children: m.label })
        ]
      },
      m.id
    );
  };
  return /* @__PURE__ */ s.jsx("div", { className: `border-b border-gray-200 ${n}`, children: /* @__PURE__ */ s.jsxs("div", { className: "flex items-center", children: [
    /* @__PURE__ */ s.jsx("div", { className: "flex space-x-8", children: d.map(f) }),
    c.length > 0 && /* @__PURE__ */ s.jsx("div", { className: "mx-6 h-6 w-px bg-gray-300", "aria-hidden": "true" }),
    c.length > 0 && /* @__PURE__ */ s.jsxs("div", { className: "flex items-center space-x-6", children: [
      /* @__PURE__ */ s.jsx("span", { className: "text-xs text-gray-500 font-medium uppercase tracking-wider", children: "Advanced" }),
      /* @__PURE__ */ s.jsx("div", { className: "flex space-x-6", children: c.map(f) })
    ] })
  ] }) });
}
const wr = () => Oi(), he = Ii;
function Gr({ node: e, depth: t = 0, name: r = null }) {
  const [n, a] = D(t === 0);
  if (!e)
    return /* @__PURE__ */ s.jsx("span", { className: "text-gray-400 italic", children: "undefined" });
  if (e.type === "Primitive") {
    const l = e.value, d = {
      String: "text-green-600",
      Number: "text-blue-600",
      Boolean: "text-purple-600",
      Null: "text-gray-500"
    }[l] || "text-gray-600";
    return /* @__PURE__ */ s.jsxs("span", { className: "inline-flex items-center space-x-2", children: [
      /* @__PURE__ */ s.jsx("span", { className: `font-mono text-sm ${d}`, children: l.toLowerCase() }),
      e.classifications && e.classifications.length > 0 && /* @__PURE__ */ s.jsx("span", { className: "flex space-x-1", children: e.classifications.map((c) => /* @__PURE__ */ s.jsx("span", { className: "px-1.5 py-0.5 text-xs bg-gray-200 text-gray-700 rounded-full font-sans", children: c }, c)) })
    ] });
  }
  if (e.type === "Any")
    return /* @__PURE__ */ s.jsx("span", { className: "font-mono text-sm text-orange-600", children: "any" });
  if (e.type === "Array")
    return /* @__PURE__ */ s.jsxs("div", { className: "inline-flex items-start", children: [
      /* @__PURE__ */ s.jsx("span", { className: "font-mono text-sm text-gray-700", children: "Array<" }),
      /* @__PURE__ */ s.jsx(Gr, { node: e.value, depth: t + 1 }),
      /* @__PURE__ */ s.jsx("span", { className: "font-mono text-sm text-gray-700", children: ">" })
    ] });
  if (e.type === "Object" && e.value) {
    const l = Object.entries(e.value);
    return l.length === 0 ? /* @__PURE__ */ s.jsx("span", { className: "font-mono text-sm text-gray-500", children: "{}" }) : /* @__PURE__ */ s.jsxs("div", { className: "inline-block", children: [
      /* @__PURE__ */ s.jsx("div", { className: "flex items-center", children: /* @__PURE__ */ s.jsxs(
        "button",
        {
          onClick: () => a(!n),
          className: "flex items-center hover:bg-gray-100 rounded px-1 -ml-1",
          children: [
            n ? /* @__PURE__ */ s.jsx(ii, { className: "h-3 w-3 text-gray-500" }) : /* @__PURE__ */ s.jsx(Us, { className: "h-3 w-3 text-gray-500" }),
            /* @__PURE__ */ s.jsxs("span", { className: "font-mono text-sm text-gray-700 ml-1", children: [
              "{",
              !n && `... ${l.length} fields`,
              !n && "}"
            ] })
          ]
        }
      ) }),
      n && /* @__PURE__ */ s.jsxs("div", { className: "ml-4 border-l-2 border-gray-200 pl-3 mt-1", children: [
        l.map(([d, c], f) => /* @__PURE__ */ s.jsxs("div", { className: "py-1", children: [
          /* @__PURE__ */ s.jsx("span", { className: "font-mono text-sm text-indigo-600", children: d }),
          /* @__PURE__ */ s.jsx("span", { className: "font-mono text-sm text-gray-500", children: ": " }),
          /* @__PURE__ */ s.jsx(Gr, { node: c, depth: t + 1, name: d }),
          f < l.length - 1 && /* @__PURE__ */ s.jsx("span", { className: "text-gray-400", children: "," })
        ] }, d)),
        /* @__PURE__ */ s.jsx("div", { className: "font-mono text-sm text-gray-700", children: "}" })
      ] })
    ] });
  }
  return /* @__PURE__ */ s.jsxs("span", { className: "font-mono text-sm text-red-500", children: [
    "unknown (",
    JSON.stringify(e),
    ")"
  ] });
}
function gd({ topology: e, compact: t = !1 }) {
  return e ? t ? /* @__PURE__ */ s.jsx("div", { className: "inline-flex items-center", children: /* @__PURE__ */ s.jsx(Gr, { node: e.root }) }) : /* @__PURE__ */ s.jsxs("div", { className: "mt-2 p-2 bg-gray-50 rounded border border-gray-200", children: [
    /* @__PURE__ */ s.jsx("div", { className: "text-xs font-medium text-gray-600 mb-1", children: "Type Structure:" }),
    /* @__PURE__ */ s.jsx("div", { className: "pl-2", children: /* @__PURE__ */ s.jsx(Gr, { node: e.root }) })
  ] }) : /* @__PURE__ */ s.jsx("div", { className: "text-xs text-gray-400 italic", children: "No topology defined" });
}
function Lu({ onResult: e, onSchemaUpdated: t }) {
  const r = wr(), n = he(Zt);
  he(dn), he(ri);
  const [a, l] = D({});
  xe(() => {
    console.log("🟢 SchemaTab: Fetching schemas on mount"), r(tt({ forceRefresh: !0 }));
  }, [r]);
  const d = (p) => p.descriptive_name || p.name;
  console.log("🟢 SchemaTab: Current schemas from Redux:", n.map((p) => ({ name: p.name, state: p.state })));
  const c = async (p) => {
    const v = a[p];
    if (l((w) => ({
      ...w,
      [p]: !w[p]
    })), !v) {
      const w = n.find((A) => A.name === p);
      if (w && (!w.fields || Object.keys(w.fields).length === 0))
        try {
          (await ue.getSchema(p)).success && (r(tt({ forceRefresh: !0 })), t && t());
        } catch (A) {
          console.error(`Failed to fetch schema details for ${p}:`, A);
        }
    }
  }, f = (p) => {
    switch (p == null ? void 0 : p.toLowerCase()) {
      case "approved":
        return "bg-green-100 text-green-800";
      case "available":
        return "bg-blue-100 text-blue-800";
      case "blocked":
        return "bg-red-100 text-red-800";
      default:
        return "bg-gray-100 text-gray-800";
    }
  }, m = async (p) => {
    var v, w;
    console.log("🟡 SchemaTab: Starting approveSchema for:", p);
    try {
      const A = await r(wt({ schemaName: p }));
      if (console.log("🟡 SchemaTab: approveSchema result:", A), wt.fulfilled.match(A)) {
        console.log("🟡 SchemaTab: approveSchema fulfilled, calling callbacks");
        const _ = (v = A.payload) == null ? void 0 : v.backfillHash;
        if (console.log("🔄 Backfill hash:", _), console.log("🔄 Refetching schemas from backend after approval..."), await r(tt({ forceRefresh: !0 })), console.log("✅ Refetch complete - backend state should be reflected"), e) {
          const T = _ ? `Schema ${p} approved successfully. Backfill started with hash: ${_}` : `Schema ${p} approved successfully`;
          e({ success: !0, message: T, backfillHash: _ });
        }
        t && t();
      } else {
        console.log("🔴 SchemaTab: approveSchema rejected:", A.payload);
        const _ = typeof A.payload == "string" ? A.payload : ((w = A.payload) == null ? void 0 : w.error) || `Failed to approve schema: ${p}`;
        throw new Error(_);
      }
    } catch (A) {
      if (console.error("🔴 SchemaTab: Failed to approve schema:", A), e) {
        const _ = A instanceof Error ? A.message : String(A);
        e({ error: `Failed to approve schema: ${_}` });
      }
    }
  }, h = async (p) => {
    var v;
    try {
      const w = await r(Et({ schemaName: p }));
      if (Et.fulfilled.match(w))
        console.log("🟡 SchemaTab: blockSchema fulfilled, calling callbacks"), console.log("🔄 Refetching schemas from backend after blocking..."), await r(tt({ forceRefresh: !0 })), console.log("✅ Refetch complete - backend state should be reflected"), e && e({ success: !0, message: `Schema ${p} blocked successfully` }), t && t();
      else {
        const A = typeof w.payload == "string" ? w.payload : ((v = w.payload) == null ? void 0 : v.error) || `Failed to block schema: ${p}`;
        throw new Error(A);
      }
    } catch (w) {
      if (console.error("Failed to block schema:", w), e) {
        const A = w instanceof Error ? w.message : String(w);
        e({ error: `Failed to block schema: ${A}` });
      }
    }
  }, y = (p) => {
    const v = a[p.name], w = p.state || "Unknown", A = p.fields ? Qn(p) : null, _ = Jn(p);
    return /* @__PURE__ */ s.jsxs("div", { className: "bg-white rounded-lg border border-gray-200 shadow-sm overflow-hidden transition-all duration-200 hover:shadow-md", children: [
      /* @__PURE__ */ s.jsx(
        "div",
        {
          className: "px-4 py-3 bg-gray-50 cursor-pointer select-none transition-colors duration-200 hover:bg-gray-100",
          onClick: () => c(p.name),
          children: /* @__PURE__ */ s.jsxs("div", { className: "flex items-center justify-between", children: [
            /* @__PURE__ */ s.jsxs("div", { className: "flex items-center space-x-2", children: [
              v ? /* @__PURE__ */ s.jsx(ii, { className: "icon icon-sm text-gray-400 transition-transform duration-200" }) : /* @__PURE__ */ s.jsx(Us, { className: "icon icon-sm text-gray-400 transition-transform duration-200" }),
              /* @__PURE__ */ s.jsx("h3", { className: "font-medium text-gray-900", children: d(p) }),
              p.descriptive_name && p.descriptive_name !== p.name && /* @__PURE__ */ s.jsxs("span", { className: "text-xs text-gray-500", children: [
                "(",
                p.name,
                ")"
              ] }),
              /* @__PURE__ */ s.jsx("span", { className: `px-2 py-1 text-xs font-medium rounded-full ${f(w)}`, children: w }),
              A && /* @__PURE__ */ s.jsx("span", { className: "px-2 py-1 text-xs font-medium rounded-full bg-purple-100 text-purple-800", children: "Range Schema" }),
              _ && /* @__PURE__ */ s.jsx("span", { className: "px-2 py-1 text-xs font-medium rounded-full bg-blue-100 text-blue-800", children: "HashRange Schema" })
            ] }),
            /* @__PURE__ */ s.jsxs("div", { className: "flex items-center space-x-2", children: [
              w.toLowerCase() === "available" && /* @__PURE__ */ s.jsx(
                "button",
                {
                  className: "group inline-flex items-center px-2 py-1 text-xs font-medium rounded-md text-white bg-green-600 hover:bg-green-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-green-500",
                  onClick: (T) => {
                    console.log("🟠 Button clicked: Approve for schema:", p.name), T.stopPropagation(), m(p.name);
                  },
                  children: "Approve"
                }
              ),
              w.toLowerCase() === "approved" && /* @__PURE__ */ s.jsx(
                "button",
                {
                  className: "group inline-flex items-center px-2 py-1 text-xs font-medium rounded-md text-white bg-red-600 hover:bg-red-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-red-500",
                  onClick: (T) => {
                    T.stopPropagation(), h(p.name);
                  },
                  children: "Block"
                }
              ),
              w.toLowerCase() === "blocked" && /* @__PURE__ */ s.jsx(
                "button",
                {
                  className: "group inline-flex items-center px-2 py-1 text-xs font-medium rounded-md text-white bg-green-600 hover:bg-green-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-green-500",
                  onClick: (T) => {
                    T.stopPropagation(), m(p.name);
                  },
                  children: "Re-approve"
                }
              )
            ] })
          ] })
        }
      ),
      v && p.fields && /* @__PURE__ */ s.jsxs("div", { className: "p-4 border-t border-gray-200", children: [
        A && /* @__PURE__ */ s.jsxs("div", { className: "mb-4 p-3 bg-purple-50 rounded-md border border-purple-200", children: [
          /* @__PURE__ */ s.jsx("h4", { className: "text-sm font-medium text-purple-900 mb-2", children: "Range Schema Information" }),
          /* @__PURE__ */ s.jsxs("div", { className: "space-y-1 text-xs text-purple-800", children: [
            /* @__PURE__ */ s.jsxs("p", { children: [
              /* @__PURE__ */ s.jsx("strong", { children: "Range Key:" }),
              " ",
              A.rangeKey
            ] }),
            /* @__PURE__ */ s.jsxs("p", { children: [
              /* @__PURE__ */ s.jsx("strong", { children: "Total Fields:" }),
              " ",
              A.totalFields
            ] }),
            /* @__PURE__ */ s.jsxs("p", { children: [
              /* @__PURE__ */ s.jsx("strong", { children: "Range Fields:" }),
              " ",
              A.rangeFields.length
            ] }),
            /* @__PURE__ */ s.jsx("p", { className: "text-purple-600", children: "This schema uses range-based storage for efficient querying and mutations." })
          ] })
        ] }),
        _ && /* @__PURE__ */ s.jsxs("div", { className: "mb-4 p-3 bg-blue-50 rounded-md border border-blue-200", children: [
          /* @__PURE__ */ s.jsx("h4", { className: "text-sm font-medium text-blue-900 mb-2", children: "HashRange Schema Information" }),
          /* @__PURE__ */ s.jsxs("div", { className: "space-y-1 text-xs text-blue-800", children: [
            /* @__PURE__ */ s.jsxs("p", { children: [
              /* @__PURE__ */ s.jsx("strong", { children: "Hash Field:" }),
              " ",
              _.hashField
            ] }),
            /* @__PURE__ */ s.jsxs("p", { children: [
              /* @__PURE__ */ s.jsx("strong", { children: "Range Field:" }),
              " ",
              _.rangeField
            ] }),
            /* @__PURE__ */ s.jsxs("p", { children: [
              /* @__PURE__ */ s.jsx("strong", { children: "Total Fields:" }),
              " ",
              _.totalFields
            ] }),
            /* @__PURE__ */ s.jsx("p", { className: "text-blue-600", children: "This schema uses hash-range-based storage for efficient querying and mutations with both hash and range keys." })
          ] })
        ] }),
        /* @__PURE__ */ s.jsx("div", { className: "space-y-3", children: Array.isArray(p.fields) ? p.fields.map((T) => {
          var R;
          const M = (R = p.field_topologies) == null ? void 0 : R[T];
          return /* @__PURE__ */ s.jsx("div", { className: "p-3 bg-gray-50 rounded-md border border-gray-200", children: /* @__PURE__ */ s.jsx("div", { className: "flex items-center justify-between", children: /* @__PURE__ */ s.jsxs("div", { className: "flex-1", children: [
            /* @__PURE__ */ s.jsxs("div", { className: "flex items-center space-x-2", children: [
              /* @__PURE__ */ s.jsx("span", { className: "font-medium text-gray-900", children: T }),
              (A == null ? void 0 : A.rangeKey) === T && /* @__PURE__ */ s.jsx("span", { className: "px-2 py-0.5 text-xs font-medium rounded-full bg-purple-100 text-purple-800", children: "Range Key" }),
              (_ == null ? void 0 : _.hashField) === T && /* @__PURE__ */ s.jsx("span", { className: "px-2 py-0.5 text-xs font-medium rounded-full bg-blue-100 text-blue-800", children: "Hash Key" }),
              (_ == null ? void 0 : _.rangeField) === T && /* @__PURE__ */ s.jsx("span", { className: "px-2 py-0.5 text-xs font-medium rounded-full bg-purple-100 text-purple-800", children: "Range Key" })
            ] }),
            M && /* @__PURE__ */ s.jsx(gd, { topology: M })
          ] }) }) }, T);
        }) : /* @__PURE__ */ s.jsx("p", { className: "text-sm text-gray-500 italic", children: "No fields defined" }) })
      ] })
    ] }, p.name);
  }, x = (p) => typeof p == "string" ? p.toLowerCase() : typeof p == "object" && p !== null ? String(p).toLowerCase() : String(p || "").toLowerCase(), N = n.filter(
    (p) => x(p.state) === "available"
  ), S = n.filter(
    (p) => x(p.state) === "approved"
  ), E = n.filter(
    (p) => x(p.state) === "blocked"
  );
  return /* @__PURE__ */ s.jsxs("div", { className: "p-6 space-y-6", children: [
    /* @__PURE__ */ s.jsxs("div", { children: [
      /* @__PURE__ */ s.jsx("h3", { className: "text-lg font-medium text-gray-900 mb-4", children: "Available Schemas" }),
      /* @__PURE__ */ s.jsx("div", { className: "border rounded-lg bg-white shadow-sm", children: /* @__PURE__ */ s.jsxs("details", { className: "group", children: [
        /* @__PURE__ */ s.jsxs("summary", { className: "flex items-center justify-between p-4 cursor-pointer hover:bg-gray-50", children: [
          /* @__PURE__ */ s.jsxs("span", { className: "font-medium text-gray-900", children: [
            "Available Schemas (",
            N.length,
            ")"
          ] }),
          /* @__PURE__ */ s.jsx(Us, { className: "h-5 w-5 text-gray-400 group-open:rotate-90 transition-transform" })
        ] }),
        /* @__PURE__ */ s.jsx("div", { className: "border-t bg-gray-50", children: N.length === 0 ? /* @__PURE__ */ s.jsx("div", { className: "p-4 text-gray-500 text-center", children: "No available schemas" }) : /* @__PURE__ */ s.jsx("div", { className: "space-y-2 p-4", children: N.map((p) => {
          const v = p.fields ? Qn(p) : null, w = Jn(p);
          return /* @__PURE__ */ s.jsxs("div", { className: "flex items-center justify-between p-3 bg-white rounded border", children: [
            /* @__PURE__ */ s.jsxs("div", { className: "flex items-center space-x-3", children: [
              /* @__PURE__ */ s.jsxs("div", { className: "flex items-center space-x-2", children: [
                /* @__PURE__ */ s.jsx("h4", { className: "font-medium text-gray-900", children: d(p) }),
                p.descriptive_name && p.descriptive_name !== p.name && /* @__PURE__ */ s.jsxs("span", { className: "text-xs text-gray-500", children: [
                  "(",
                  p.name,
                  ")"
                ] })
              ] }),
              /* @__PURE__ */ s.jsx("span", { className: `px-2 py-1 rounded-full text-xs font-medium ${f(p.state)}`, children: p.state }),
              v && /* @__PURE__ */ s.jsx("span", { className: "px-2 py-1 text-xs font-medium rounded-full bg-purple-100 text-purple-800", children: "Range Schema" }),
              w && /* @__PURE__ */ s.jsx("span", { className: "px-2 py-1 text-xs font-medium rounded-full bg-blue-100 text-blue-800", children: "HashRange Schema" })
            ] }),
            /* @__PURE__ */ s.jsx("div", { className: "flex space-x-2", children: /* @__PURE__ */ s.jsx(
              "button",
              {
                onClick: () => m(p.name),
                className: "px-3 py-1 bg-green-500 text-white rounded text-sm hover:bg-green-600",
                children: "Approve"
              }
            ) })
          ] }, p.name);
        }) }) })
      ] }) })
    ] }),
    /* @__PURE__ */ s.jsxs("div", { className: "space-y-4", children: [
      /* @__PURE__ */ s.jsx("h3", { className: "text-lg font-medium text-gray-900", children: "Approved Schemas" }),
      S.length > 0 ? S.map(y) : /* @__PURE__ */ s.jsx("div", { className: "border rounded-lg p-8 bg-white shadow-sm text-center text-gray-500", children: "No approved schemas. Approve schemas from the available list above to see them here." })
    ] }),
    E.length > 0 && /* @__PURE__ */ s.jsxs("div", { className: "space-y-4", children: [
      /* @__PURE__ */ s.jsx("h3", { className: "text-lg font-medium text-gray-900", children: "Blocked Schemas" }),
      E.map(y)
    ] })
  ] });
}
function fn() {
  const e = wr(), t = he(Zt), r = he(dn), [n, a] = D(""), [l, d] = D([]), [c, f] = D({}), [m, h] = D({}), [y, x] = D(""), [N, S] = D(""), [E, p] = D({}), v = ye(() => (t || []).filter((V) => (typeof V.state == "string" ? V.state.toLowerCase() : String(V.state || "").toLowerCase()) === st.APPROVED), [t]), w = ye(() => n ? (t || []).find((V) => V.name === n) : null, [n, t]), A = ye(() => w ? Qt(w) : !1, [w]), _ = ye(() => w ? ln(w) : !1, [w]), T = ye(() => w ? Dt(w) : null, [w]), M = H((V) => {
    if (a(V), V) {
      const G = (t || []).find((ge) => ge.name === V), L = (G == null ? void 0 : G.fields) || (G == null ? void 0 : G.transform_fields) || [], J = Array.isArray(L) ? L : Object.keys(L);
      d(J);
      const Q = {};
      J.forEach((ge) => {
        Q[ge] = "";
      }), f(Q);
    } else
      d([]), f({});
    h({}), x(""), S(""), p({});
  }, [t]), R = H((V) => {
    d((G) => G.includes(V) ? G.filter((L) => L !== V) : [...G, V]), f((G) => G[V] !== void 0 ? G : {
      ...G,
      [V]: ""
      // Initialize with empty string for new fields
    });
  }, []), k = H((V, G, L) => {
    h((J) => ({
      ...J,
      [V]: {
        ...J[V],
        [G]: L
      }
    }));
  }, []), I = H((V, G) => {
    f((L) => ({
      ...L,
      [V]: G
    }));
  }, []), $ = H(() => {
    a(""), d([]), f({}), h({}), x(""), S(""), p({});
  }, []), F = H(() => {
    e(tt({ forceRefresh: !0 }));
  }, [e]);
  return {
    state: {
      selectedSchema: n,
      queryFields: l,
      fieldValues: c,
      rangeFilters: m,
      rangeSchemaFilter: E,
      rangeKeyValue: y,
      hashKeyValue: N
    },
    setSelectedSchema: a,
    setQueryFields: d,
    setFieldValues: f,
    toggleField: R,
    handleFieldValueChange: I,
    setRangeFilters: h,
    setRangeSchemaFilter: p,
    setRangeKeyValue: x,
    setHashKeyValue: S,
    clearState: $,
    handleSchemaChange: M,
    handleRangeFilterChange: k,
    refetchSchemas: F,
    approvedSchemas: v,
    schemasLoading: r,
    selectedSchemaObj: w,
    isRangeSchema: A,
    isHashRangeSchema: _,
    rangeKey: T
  };
}
function dr(e) {
  return { HashKey: e };
}
function yd(e) {
  return { RangePrefix: e };
}
function xd(e, t) {
  return { RangeRange: { start: e, end: t } };
}
function bd(e, t) {
  return { HashRangeKey: { hash: e, range: t } };
}
function vd({
  schema: e,
  queryState: t,
  schemas: r,
  selectedSchemaObj: n,
  isRangeSchema: a,
  rangeKey: l
}) {
  const d = he(vr), c = ye(() => n || (r && e && r[e] ? r[e] : d && Array.isArray(d) && d.find((x) => x.name === e) || null), [n, e, r, d]), f = ye(() => typeof a == "boolean" ? a : c ? c.schema_type === "Range" || Qt(c) ? !0 : c.fields && typeof c.fields == "object" ? Object.values(c.fields).some((N) => (N == null ? void 0 : N.field_type) === "Range") : !1 : !1, [c, a]), m = ye(() => [], []), h = !0, y = ye(() => {
    var A;
    if (!e || !t || !c)
      return {};
    const {
      queryFields: x = [],
      fieldValues: N = {},
      rangeFilters: S = {},
      rangeSchemaFilter: E = {},
      filters: p = [],
      orderBy: v
    } = t, w = {
      schema_name: e,
      // Backend expects schema_name, not schema
      fields: x
      // Array of selected field names
    };
    if (ln(c)) {
      const _ = t.hashKeyValue, T = (A = t.rangeSchemaFilter) == null ? void 0 : A.key;
      _ && _.trim() ? w.filter = dr(_.trim()) : T && T.trim() && (w.filter = dr(T.trim()));
    }
    if (f) {
      const _ = E && Object.keys(E).length > 0 ? E : Object.values(S).find((M) => M && typeof M == "object" && (M.key || M.keyPrefix || M.start && M.end)) || {}, T = t == null ? void 0 : t.rangeKeyValue;
      !_.key && !_.keyPrefix && !(_.start && _.end) && T && (_.key = T), _.key ? w.filter = dr(_.key) : _.keyPrefix ? w.filter = yd(_.keyPrefix) : _.start && _.end && (w.filter = xd(_.start, _.end));
    }
    return w;
  }, [e, t, c]);
  return H(() => y, [y]), H(() => ({
    isValid: h,
    errors: m
  }), [h, m]), {
    query: y,
    validationErrors: m,
    isValid: h
  };
}
function rt({
  label: e,
  name: t,
  required: r = !1,
  error: n,
  helpText: a,
  children: l,
  className: d = ""
}) {
  const c = t ? `field-${t}` : `field-${Math.random().toString(36).substr(2, 9)}`, f = !!n;
  return /* @__PURE__ */ s.jsxs("div", { className: `space-y-2 ${d}`, children: [
    /* @__PURE__ */ s.jsxs(
      "label",
      {
        htmlFor: c,
        className: "block text-sm font-medium text-gray-700",
        children: [
          e,
          r && /* @__PURE__ */ s.jsx("span", { className: "ml-1 text-red-500", "aria-label": "required", children: "*" })
        ]
      }
    ),
    /* @__PURE__ */ s.jsx("div", { className: "relative", children: l }),
    f && /* @__PURE__ */ s.jsx(
      "p",
      {
        className: "text-sm text-red-600",
        role: "alert",
        "aria-live": "polite",
        children: n
      }
    ),
    a && !f && /* @__PURE__ */ s.jsx("p", { className: "text-xs text-gray-500", children: a })
  ] });
}
function hi(e = []) {
  return e.reduce((t, r) => {
    const n = r.group || "default";
    return t[n] || (t[n] = []), t[n].push(r), t;
  }, {});
}
function wd(e = [], t = "") {
  if (wl(t)) return e;
  const r = t.toLowerCase();
  return e.filter(
    (n) => n.label.toLowerCase().includes(r) || n.value.toLowerCase().includes(r)
  );
}
function Ed(e = {}) {
  return {
    placeholder: "Select an option...",
    emptyMessage: "No options available",
    searchable: !1,
    required: !1,
    disabled: !1,
    loading: !1,
    showConfirmation: !1,
    ...e
  };
}
function Nd(e, t = !1, r = !1, n = !1) {
  var l, d;
  let a = ((l = e.select) == null ? void 0 : l.base) || "";
  return t && (a += " border-red-300 focus:ring-red-500 focus:border-red-500"), (r || n) && (a += ` ${((d = e.select) == null ? void 0 : d.disabled) || ""}`), a;
}
function jd(e, t = !1, r = "") {
  const n = {
    "aria-invalid": t
  };
  return t ? n["aria-describedby"] = `${e}-error` : r && (n["aria-describedby"] = `${e}-help`), n;
}
function Sd(e = [], t, r = !0) {
  const [n, a] = D(""), [l, d] = D(!1), c = wd(e, n), f = hi(c), m = H((w) => {
    a(w.target.value);
  }, []), h = H((w) => {
    w.disabled || (t(w.value), r && (d(!1), a("")));
  }, [t, r]), y = H(() => {
    d(!0);
  }, []), x = H(() => {
    d(!1);
  }, []), N = H(() => {
    d((w) => !w);
  }, []), S = H((w) => {
    const A = e.find((_) => _.value === w);
    A && h(A);
  }, [e, h]), E = H(() => {
    a("");
  }, []);
  return {
    state: {
      searchTerm: n,
      isOpen: l,
      filteredOptions: c,
      groupedOptions: f
    },
    actions: {
      setSearchTerm: a,
      openDropdown: y,
      closeDropdown: x,
      toggleDropdown: N,
      selectOption: S,
      clearSearch: E
    },
    handleSearchChange: m,
    handleOptionSelect: h
  };
}
function mi(e) {
  return `field-${e}`;
}
function _d(e) {
  return !!e;
}
function Ad({ hasError: e, disabled: t, additionalClasses: r = "" }) {
  const n = Ke.input.base, a = e ? Ke.input.error : Ke.input.success;
  return `${n} ${a} ${t ? "bg-gray-100 cursor-not-allowed" : ""} ${r}`.trim();
}
function Td({ fieldId: e, hasError: t, hasHelp: r }) {
  const n = {
    "aria-invalid": t
  };
  return t ? n["aria-describedby"] = `${e}-error` : r && (n["aria-describedby"] = `${e}-help`), n;
}
function Cd({ size: e = "sm", color: t = "primary" } = {}) {
  const r = {
    sm: "h-3 w-3",
    md: "h-4 w-4",
    lg: "h-5 w-5"
  }, n = {
    primary: "border-primary border-t-transparent",
    gray: "border-gray-400 border-t-transparent",
    white: "border-white border-t-transparent"
  };
  return `animate-spin ${r[e]} border-2 ${n[t]} rounded-full`;
}
function Ks({
  name: e,
  label: t,
  value: r,
  options: n = [],
  onChange: a,
  error: l,
  helpText: d,
  config: c = {},
  className: f = ""
}) {
  const m = Ed(c), { searchable: h, placeholder: y, emptyMessage: x, required: N, disabled: S, loading: E } = m, p = mi(e), v = !!l, w = n.length > 0, A = Sd(n, a, !0), _ = (k) => {
    a(k.target.value);
  };
  if (E)
    return /* @__PURE__ */ s.jsx(rt, { label: t, name: e, required: N, error: l, helpText: d, className: f, children: /* @__PURE__ */ s.jsxs("div", { className: `${Ke.select.disabled} flex items-center`, children: [
      /* @__PURE__ */ s.jsx("div", { className: "animate-spin h-4 w-4 border-2 border-gray-400 border-t-transparent rounded-full mr-2" }),
      gl.loading
    ] }) });
  if (!w)
    return /* @__PURE__ */ s.jsx(rt, { label: t, name: e, required: N, error: l, helpText: d, className: f, children: /* @__PURE__ */ s.jsx("div", { className: Ke.select.disabled, children: x }) });
  if (h) {
    const { state: k, handleSearchChange: I, handleOptionSelect: $ } = A;
    return /* @__PURE__ */ s.jsx(rt, { label: t, name: e, required: N, error: l, helpText: d, className: f, children: /* @__PURE__ */ s.jsxs("div", { className: "relative", children: [
      /* @__PURE__ */ s.jsx(
        "input",
        {
          type: "text",
          placeholder: `Search ${t.toLowerCase()}...`,
          value: k.searchTerm,
          onChange: I,
          onFocus: () => A.actions.openDropdown(),
          className: `${Ke.input.base} ${v ? Ke.input.error : ""}`
        }
      ),
      k.isOpen && k.filteredOptions.length > 0 && /* @__PURE__ */ s.jsx("div", { className: "absolute z-10 w-full mt-1 bg-white border border-gray-300 rounded-md shadow-lg max-h-60 overflow-auto", children: Object.entries(k.groupedOptions).map(([F, z]) => /* @__PURE__ */ s.jsxs("div", { children: [
        F !== "default" && /* @__PURE__ */ s.jsx("div", { className: "px-3 py-2 text-xs font-semibold text-gray-500 bg-gray-50 border-b", children: F }),
        z.map((V) => /* @__PURE__ */ s.jsx(
          "button",
          {
            type: "button",
            onClick: () => $(V),
            disabled: V.disabled,
            className: `w-full text-left px-3 py-2 hover:bg-gray-100 focus:bg-gray-100 focus:outline-none ${V.disabled ? "text-gray-400 cursor-not-allowed" : "text-gray-900"} ${r === V.value ? "bg-primary text-white" : ""}`,
            children: V.label
          },
          V.value
        ))
      ] }, F)) })
    ] }) });
  }
  const T = hi(n), M = Nd(Ke, v, S, E), R = jd(p, v, d);
  return /* @__PURE__ */ s.jsx(rt, { label: t, name: e, required: N, error: l, helpText: d, className: f, children: /* @__PURE__ */ s.jsxs(
    "select",
    {
      id: p,
      name: e,
      value: r,
      onChange: _,
      required: N,
      disabled: S,
      className: M,
      ...R,
      children: [
        /* @__PURE__ */ s.jsx("option", { value: "", disabled: N, children: y }),
        Object.entries(T).map(
          ([k, I]) => k !== "default" ? /* @__PURE__ */ s.jsx("optgroup", { label: k, children: I.map(($) => /* @__PURE__ */ s.jsx("option", { value: $.value, disabled: $.disabled, children: $.label }, $.value)) }, k) : I.map(($) => /* @__PURE__ */ s.jsx("option", { value: $.value, disabled: $.disabled, children: $.label }, $.value))
        )
      ]
    }
  ) });
}
function ir({
  name: e,
  label: t,
  value: r,
  onChange: n,
  required: a = !1,
  disabled: l = !1,
  error: d,
  placeholder: c,
  helpText: f,
  type: m = "text",
  debounced: h = !1,
  debounceMs: y = ml,
  className: x = ""
}) {
  const [N, S] = D(r), [E, p] = D(!1);
  xe(() => {
    S(r);
  }, [r]);
  const v = or(null), w = or(null), A = or(n);
  xe(() => {
    A.current = n;
  }, [n]);
  const _ = H(($) => {
    p(!0), v.current && (clearTimeout(v.current), v.current = null), w.current && typeof window < "u" && typeof window.cancelAnimationFrame == "function" && (window.cancelAnimationFrame(w.current), w.current = null);
    const F = () => {
      v.current = setTimeout(() => {
        A.current($), p(!1);
      }, y);
    };
    typeof window < "u" && typeof window.requestAnimationFrame == "function" ? w.current = window.requestAnimationFrame(F) : setTimeout(F, 0);
  }, [y]), T = ($) => {
    const F = $.target.value;
    S(F), h ? _(F) : n(F);
  }, M = mi(e), R = _d(d), k = Ad({ hasError: R, disabled: l }), I = Td({
    fieldId: M,
    hasError: R,
    hasHelp: !!f
  });
  return /* @__PURE__ */ s.jsx(
    rt,
    {
      label: t,
      name: e,
      required: a,
      error: d,
      helpText: f,
      className: x,
      children: /* @__PURE__ */ s.jsxs("div", { className: "relative", children: [
        /* @__PURE__ */ s.jsx(
          "input",
          {
            id: M,
            name: e,
            type: m,
            value: N,
            onChange: T,
            placeholder: c,
            required: a,
            disabled: l,
            className: k,
            ...I
          }
        ),
        h && E && /* @__PURE__ */ s.jsx("div", { className: "absolute right-2 top-1/2 transform -translate-y-1/2", children: /* @__PURE__ */ s.jsx(
          "div",
          {
            className: Cd({ size: "md", color: "primary" }),
            role: "status",
            "aria-label": "Processing input"
          }
        ) })
      ] })
    }
  );
}
function oa(e = {}) {
  return e.start || e.end ? "range" : e.key ? "key" : e.keyPrefix ? "prefix" : "range";
}
function Rd(e, t, r) {
  const n = { ...e };
  return t === "range" || r === "start" || r === "end" ? (delete n.key, delete n.keyPrefix) : t === "key" || r === "key" ? (delete n.start, delete n.end, delete n.keyPrefix) : (t === "prefix" || r === "keyPrefix") && (delete n.start, delete n.end, delete n.key), n;
}
function kd(e = {}, t, r = ["range", "key", "prefix"]) {
  const [n, a] = D(
    () => oa(e)
  ), [l, d] = D(e), c = H((E) => {
    if (!r.includes(E)) return;
    a(E);
    const p = {};
    d(p), t && t(p);
  }, [r, t]), f = H((E, p) => {
    const v = Rd(l, n, E);
    v[E] = p, d(v), t && t(v);
  }, [l, n, t]), m = H(() => {
    const E = {};
    d(E), t && t(E);
  }, [t]), h = H((E) => {
    d(E);
    const p = oa(E);
    a(p), t && t(E);
  }, [t]), y = H(() => r, [r]), x = H((E) => r.includes(E), [r]);
  return {
    state: {
      activeMode: n,
      value: l
    },
    actions: {
      changeMode: c,
      updateValue: f,
      clearValue: m,
      setValue: h
    },
    getAvailableModes: y,
    isValidMode: x
  };
}
function Id(e = "all", t = "key", r = "") {
  if (r) return r;
  if (e !== "all") return null;
  const n = { ...xl.rangeKeyFilter }, a = n.keyRange || "", l = (n.exactKey || "").replace("key", t), d = (n.keyPrefix || "").replace("keys", `${t} values`), c = n.emptyNote || "";
  return `${a} ${l} ${d} ${c}`.trim();
}
function Od(e = "all") {
  const t = {
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
  return t[e] || t.all;
}
function Dd(e = !1) {
  const t = "px-3 py-1 text-xs rounded-md transition-colors duration-200";
  return e ? `${t} bg-primary text-white` : `${t} bg-gray-200 text-gray-700 hover:bg-gray-300`;
}
function Fd() {
  return {
    range: "Key Range",
    key: "Exact Key",
    prefix: "Key Prefix"
  };
}
function Pd(e, t) {
  return e === "all" ? {
    showRange: t === "range",
    showKey: t === "key",
    showPrefix: t === "prefix"
  } : {
    showRange: e === "range",
    showKey: e === "key",
    showPrefix: e === "prefix"
  };
}
function Md(e = {}) {
  const {
    mode: t = "all",
    rangeKeyName: r = "key",
    required: n = !1,
    disabled: a = !1,
    className: l = ""
  } = e;
  return {
    mode: ["all", "range", "key", "prefix"].includes(t) ? t : "all",
    rangeKeyName: String(r),
    required: !!n,
    disabled: !!a,
    className: String(l)
  };
}
function Bd() {
  return "bg-yellow-50 rounded-lg p-4 space-y-4";
}
function Ld() {
  return "text-sm font-medium text-gray-800";
}
function $d() {
  return "flex space-x-4 mb-4";
}
function Ud() {
  return "grid grid-cols-1 md:grid-cols-3 gap-4";
}
function Kd({
  name: e,
  label: t,
  value: r = {},
  onChange: n,
  error: a,
  helpText: l,
  config: d = {},
  className: c = ""
}) {
  const f = Md(d), { mode: m, rangeKeyName: h, required: y, disabled: x } = f, N = Od(m), S = kd(r, n, N.availableModes), { state: E, actions: p } = S, v = Fd(), w = Pd(m, E.activeMode), A = Id(m, h, l);
  return /* @__PURE__ */ s.jsx(
    rt,
    {
      label: t,
      name: e,
      required: y,
      error: a,
      helpText: A,
      className: c,
      children: /* @__PURE__ */ s.jsxs("div", { className: Bd(), children: [
        /* @__PURE__ */ s.jsx("div", { className: "mb-3", children: /* @__PURE__ */ s.jsxs("span", { className: Ld(), children: [
          "Range Key: ",
          h
        ] }) }),
        N.showModeSelector && /* @__PURE__ */ s.jsx("div", { className: $d(), children: N.availableModes.map((_) => /* @__PURE__ */ s.jsx(
          "button",
          {
            type: "button",
            onClick: () => p.changeMode(_),
            className: Dd(E.activeMode === _),
            children: v[_]
          },
          _
        )) }),
        /* @__PURE__ */ s.jsxs("div", { className: Ud(), children: [
          w.showRange && /* @__PURE__ */ s.jsxs(s.Fragment, { children: [
            /* @__PURE__ */ s.jsx(
              ir,
              {
                name: `${e}-start`,
                label: "Start Key",
                value: E.value.start || "",
                onChange: (_) => p.updateValue("start", _),
                placeholder: "Start key",
                disabled: x,
                className: "col-span-1"
              }
            ),
            /* @__PURE__ */ s.jsx(
              ir,
              {
                name: `${e}-end`,
                label: "End Key",
                value: E.value.end || "",
                onChange: (_) => p.updateValue("end", _),
                placeholder: "End key",
                disabled: x,
                className: "col-span-1"
              }
            )
          ] }),
          w.showKey && /* @__PURE__ */ s.jsx(
            ir,
            {
              name: `${e}-key`,
              label: "Exact Key",
              value: E.value.key || "",
              onChange: (_) => p.updateValue("key", _),
              placeholder: `Exact ${h} to match`,
              disabled: x,
              className: "col-span-1"
            }
          ),
          w.showPrefix && /* @__PURE__ */ s.jsx(
            ir,
            {
              name: `${e}-prefix`,
              label: "Key Prefix",
              value: E.value.keyPrefix || "",
              onChange: (_) => p.updateValue("keyPrefix", _),
              placeholder: `${h} prefix (e.g., 'user:')`,
              disabled: x,
              className: "col-span-1"
            }
          )
        ] })
      ] })
    }
  );
}
function Vd({
  queryState: e,
  onSchemaChange: t,
  onFieldToggle: r,
  onFieldValueChange: n,
  onRangeFilterChange: a,
  onRangeSchemaFilterChange: l,
  onHashKeyChange: d,
  approvedSchemas: c,
  schemasLoading: f,
  isRangeSchema: m,
  isHashRangeSchema: h,
  rangeKey: y,
  className: x = ""
}) {
  const [N, S] = D({}), { clearQuery: E } = fn();
  H(() => (S({}), !0), []);
  const p = H((T) => {
    t(T), E && E(), S((M) => {
      const { schema: R, ...k } = M;
      return k;
    });
  }, [t, E]), v = H((T) => {
    r(T), S((M) => {
      const { fields: R, ...k } = M;
      return k;
    });
  }, [r]), w = e != null && e.selectedSchema && c ? c.find((T) => T.name === e.selectedSchema) : null, A = (w == null ? void 0 : w.fields) || (w == null ? void 0 : w.transform_fields) || [], _ = Array.isArray(A) ? A : Object.keys(A);
  return /* @__PURE__ */ s.jsxs("div", { className: `space-y-6 ${x}`, children: [
    /* @__PURE__ */ s.jsx(
      rt,
      {
        label: kt.schema,
        name: "schema",
        required: !0,
        error: N.schema,
        helpText: kt.schemaHelp,
        children: /* @__PURE__ */ s.jsx(
          Ks,
          {
            name: "schema",
            value: (e == null ? void 0 : e.selectedSchema) || "",
            onChange: p,
            options: c.map((T) => ({
              value: T.name,
              label: T.descriptive_name || T.name
            })),
            placeholder: "Select a schema...",
            emptyMessage: kt.schemaEmpty,
            loading: f
          }
        )
      }
    ),
    (e == null ? void 0 : e.selectedSchema) && _.length > 0 && /* @__PURE__ */ s.jsx(
      rt,
      {
        label: "Field Selection",
        name: "fields",
        required: !0,
        error: N.fields,
        helpText: "Select fields to include in your query",
        children: /* @__PURE__ */ s.jsx("div", { className: "bg-gray-50 rounded-md p-4", children: /* @__PURE__ */ s.jsx("div", { className: "space-y-3", children: _.map((T) => {
          var M;
          return /* @__PURE__ */ s.jsxs("label", { className: "relative flex items-start", children: [
            /* @__PURE__ */ s.jsx("div", { className: "flex items-center h-5", children: /* @__PURE__ */ s.jsx(
              "input",
              {
                type: "checkbox",
                className: "h-4 w-4 text-primary border-gray-300 rounded focus:ring-primary",
                checked: ((M = e == null ? void 0 : e.queryFields) == null ? void 0 : M.includes(T)) || !1,
                onChange: () => v(T)
              }
            ) }),
            /* @__PURE__ */ s.jsx("div", { className: "ml-3 flex items-center", children: /* @__PURE__ */ s.jsx("span", { className: "text-sm font-medium text-gray-700", children: T }) })
          ] }, T);
        }) }) })
      }
    ),
    h && /* @__PURE__ */ s.jsx(
      rt,
      {
        label: "HashRange Filter",
        name: "hashRangeFilter",
        helpText: "Filter data by hash and range key values",
        children: /* @__PURE__ */ s.jsxs("div", { className: "bg-purple-50 rounded-md p-4 space-y-4", children: [
          /* @__PURE__ */ s.jsxs("div", { className: "grid grid-cols-1 md:grid-cols-2 gap-4", children: [
            /* @__PURE__ */ s.jsxs("div", { className: "space-y-2", children: [
              /* @__PURE__ */ s.jsx("label", { className: "block text-sm font-medium text-gray-700", children: "Hash Key" }),
              /* @__PURE__ */ s.jsx(
                "input",
                {
                  type: "text",
                  placeholder: "Enter hash key value",
                  className: "w-full px-3 py-2 text-sm border border-gray-300 rounded-md focus:outline-none focus:ring-1 focus:ring-primary focus:border-primary",
                  value: (e == null ? void 0 : e.hashKeyValue) || "",
                  onChange: (T) => d(T.target.value)
                }
              ),
              /* @__PURE__ */ s.jsxs("div", { className: "text-xs text-gray-500", children: [
                "Hash field: ",
                Za(c.find((T) => T.name === (e == null ? void 0 : e.selectedSchema))) || "N/A"
              ] })
            ] }),
            /* @__PURE__ */ s.jsxs("div", { className: "space-y-2", children: [
              /* @__PURE__ */ s.jsx("label", { className: "block text-sm font-medium text-gray-700", children: "Range Key" }),
              /* @__PURE__ */ s.jsx(
                "input",
                {
                  type: "text",
                  placeholder: "Enter range key value",
                  className: "w-full px-3 py-2 text-sm border border-gray-300 rounded-md focus:outline-none focus:ring-1 focus:ring-primary focus:border-primary",
                  value: (e == null ? void 0 : e.rangeKeyValue) || "",
                  onChange: (T) => l({ key: T.target.value })
                }
              ),
              /* @__PURE__ */ s.jsxs("div", { className: "text-xs text-gray-500", children: [
                "Range field: ",
                Dt(c.find((T) => T.name === (e == null ? void 0 : e.selectedSchema))) || "N/A"
              ] })
            ] })
          ] }),
          /* @__PURE__ */ s.jsxs("div", { className: "text-xs text-gray-500", children: [
            /* @__PURE__ */ s.jsxs("p", { children: [
              /* @__PURE__ */ s.jsx("strong", { children: "Hash Key:" }),
              " Used for partitioning data across multiple nodes"
            ] }),
            /* @__PURE__ */ s.jsxs("p", { children: [
              /* @__PURE__ */ s.jsx("strong", { children: "Range Key:" }),
              " Used for ordering and range queries within a partition"
            ] })
          ] })
        ] })
      }
    ),
    m && y && /* @__PURE__ */ s.jsx(
      rt,
      {
        label: "Range Filter",
        name: "rangeSchemaFilter",
        error: N.rangeFilter,
        helpText: "Filter data by range key values",
        children: /* @__PURE__ */ s.jsx(
          Kd,
          {
            name: "rangeSchemaFilter",
            value: (e == null ? void 0 : e.rangeSchemaFilter) || {},
            onChange: (T) => {
              l(T), S((M) => {
                const { rangeFilter: R, ...k } = M;
                return k;
              });
            },
            rangeKeyName: y,
            mode: "all"
          }
        )
      }
    )
  ] });
}
function Hd({
  onExecute: e,
  onExecuteQuery: t,
  onValidate: r,
  onSave: n,
  onSaveQuery: a,
  onClear: l,
  onClearQuery: d,
  disabled: c = !1,
  isExecuting: f = !1,
  isSaving: m = !1,
  showValidation: h = !1,
  showSave: y = !0,
  showClear: x = !0,
  className: N = "",
  queryData: S
}) {
  const [E, p] = D(null), [v, w] = D(null), { clearQuery: A } = fn(), _ = async (I, $, F = null) => {
    if (!(!$ || c))
      try {
        p(I), await $(F);
      } catch (z) {
        console.error(`${I} action failed:`, z);
      } finally {
        p(null), w(null);
      }
  }, T = () => {
    _("execute", t || e, S);
  }, M = () => {
    _("validate", r, S);
  }, R = () => {
    _("save", a || n, S);
  }, k = () => {
    const I = d || l;
    I && I(), A && A();
  };
  return /* @__PURE__ */ s.jsxs("div", { className: `flex justify-end space-x-3 ${N}`, children: [
    x && /* @__PURE__ */ s.jsx(
      "button",
      {
        type: "button",
        onClick: k,
        disabled: c,
        className: `
            inline-flex items-center px-4 py-2 border border-gray-300 rounded-md shadow-sm text-sm font-medium
            ${c ? "bg-gray-100 text-gray-400 cursor-not-allowed" : "bg-white text-gray-700 hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-primary"}
          `,
        children: Ir.clearQuery || "Clear Query"
      }
    ),
    h && r && /* @__PURE__ */ s.jsxs(
      "button",
      {
        type: "button",
        onClick: M,
        disabled: c,
        className: `
            inline-flex items-center px-4 py-2 border border-transparent rounded-md shadow-sm text-sm font-medium
            ${c ? "bg-gray-300 text-gray-500 cursor-not-allowed" : "bg-blue-600 text-white hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500"}
          `,
        children: [
          E === "validate" && /* @__PURE__ */ s.jsxs("svg", { className: "animate-spin -ml-1 mr-2 h-4 w-4 text-white", xmlns: "http://www.w3.org/2000/svg", fill: "none", viewBox: "0 0 24 24", children: [
            /* @__PURE__ */ s.jsx("circle", { className: "opacity-25", cx: "12", cy: "12", r: "10", stroke: "currentColor", strokeWidth: "4" }),
            /* @__PURE__ */ s.jsx("path", { className: "opacity-75", fill: "currentColor", d: "M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" })
          ] }),
          Ir.validateQuery || "Validate"
        ]
      }
    ),
    y && (n || a) && /* @__PURE__ */ s.jsxs(
      "button",
      {
        type: "button",
        onClick: R,
        disabled: c || m,
        className: `
            inline-flex items-center px-4 py-2 border border-transparent rounded-md shadow-sm text-sm font-medium
            ${c || m ? "bg-gray-300 text-gray-500 cursor-not-allowed" : "bg-green-600 text-white hover:bg-green-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-green-500"}
          `,
        children: [
          (E === "save" || m) && /* @__PURE__ */ s.jsxs("svg", { className: "animate-spin -ml-1 mr-2 h-4 w-4 text-white", xmlns: "http://www.w3.org/2000/svg", fill: "none", viewBox: "0 0 24 24", children: [
            /* @__PURE__ */ s.jsx("circle", { className: "opacity-25", cx: "12", cy: "12", r: "10", stroke: "currentColor", strokeWidth: "4" }),
            /* @__PURE__ */ s.jsx("path", { className: "opacity-75", fill: "currentColor", d: "M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" })
          ] }),
          Ir.saveQuery || "Save Query"
        ]
      }
    ),
    /* @__PURE__ */ s.jsxs(
      "button",
      {
        type: "button",
        onClick: T,
        disabled: c || f,
        className: `
          inline-flex items-center px-4 py-2 border border-transparent rounded-md shadow-sm text-sm font-medium text-white
          ${c || f ? "bg-gray-300 cursor-not-allowed" : "bg-primary hover:bg-primary/90 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-primary"}
        `,
        children: [
          (E === "execute" || f) && /* @__PURE__ */ s.jsxs("svg", { className: "animate-spin -ml-1 mr-2 h-4 w-4 text-white", xmlns: "http://www.w3.org/2000/svg", fill: "none", viewBox: "0 0 24 24", children: [
            /* @__PURE__ */ s.jsx("circle", { className: "opacity-25", cx: "12", cy: "12", r: "10", stroke: "currentColor", strokeWidth: "4" }),
            /* @__PURE__ */ s.jsx("path", { className: "opacity-75", fill: "currentColor", d: "M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 714 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" })
          ] }),
          E === "execute" || f ? "Executing..." : Ir.executeQuery
        ]
      }
    )
  ] });
}
const zd = (e, t) => {
  if (!e && !t) return null;
  const r = { ...e, ...t };
  let n = [], a = {};
  Array.isArray(r.fields) ? n = r.fields : r.fields && typeof r.fields == "object" ? (n = Object.keys(r.fields), a = r.fields) : r.queryFields && Array.isArray(r.queryFields) && (n = r.queryFields), r.fieldValues && typeof r.fieldValues == "object" && (a = { ...a, ...r.fieldValues });
  const l = {
    schema: r.schema || r.selectedSchema,
    fields: n,
    fieldValues: a,
    filters: r.filters || {},
    // Include filters from test mocks
    orderBy: r.orderBy,
    // Include orderBy from test mocks
    rangeKey: r.rangeKey
    // Include rangeKey from test mocks
  };
  if (e && e.filter)
    if (e.filter.field && e.filter.range_filter) {
      const d = e.filter.field, c = e.filter.range_filter;
      c.Key ? l.filters[d] = { exactKey: c.Key } : c.KeyRange ? l.filters[d] = {
        keyRange: `${c.KeyRange.start} → ${c.KeyRange.end}`
      } : c.KeyPrefix && (l.filters[d] = { keyPrefix: c.KeyPrefix });
    } else e.filter.range_filter && Object.entries(e.filter.range_filter).forEach(([d, c]) => {
      typeof c == "string" ? l.filters[d] = { exactKey: c } : c.KeyRange ? l.filters[d] = {
        keyRange: `${c.KeyRange.start} → ${c.KeyRange.end}`
      } : c.KeyPrefix && (l.filters[d] = { keyPrefix: c.KeyPrefix });
    });
  return l;
};
function Gd({
  query: e,
  queryState: t,
  validationErrors: r = [],
  isExecuting: n = !1,
  showJson: a = !1,
  collapsible: l = !0,
  className: d = "",
  title: c = "Query Preview"
}) {
  const f = ye(() => zd(e, t), [e, t]);
  return !e && !t ? /* @__PURE__ */ s.jsxs("div", { className: `bg-gray-50 rounded-md p-4 ${d}`, children: [
    /* @__PURE__ */ s.jsx("h3", { className: "text-sm font-medium text-gray-500 mb-2", children: c }),
    /* @__PURE__ */ s.jsx("p", { className: "text-sm text-gray-400 italic", children: "No query to preview" })
  ] }) : /* @__PURE__ */ s.jsxs("div", { className: `bg-white border border-gray-200 rounded-lg shadow-sm ${d}`, children: [
    /* @__PURE__ */ s.jsx("div", { className: "px-4 py-3 border-b border-gray-200", children: /* @__PURE__ */ s.jsx("h3", { className: "text-sm font-medium text-gray-900", children: c }) }),
    /* @__PURE__ */ s.jsxs("div", { className: "p-4 space-y-4", children: [
      r && r.length > 0 && /* @__PURE__ */ s.jsxs("div", { className: "bg-red-50 border border-red-200 rounded-md p-3", children: [
        /* @__PURE__ */ s.jsxs("div", { className: "flex items-center mb-2", children: [
          /* @__PURE__ */ s.jsx("svg", { className: "h-4 w-4 text-red-400 mr-2", fill: "currentColor", viewBox: "0 0 20 20", children: /* @__PURE__ */ s.jsx("path", { fillRule: "evenodd", d: "M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7 4a1 1 0 11-2 0 1 1 0 012 0zm-1-9a1 1 0 00-1 1v4a1 1 0 102 0V6a1 1 0 00-1-1z", clipRule: "evenodd" }) }),
          /* @__PURE__ */ s.jsx("span", { className: "text-sm font-medium text-red-800", children: "Validation Errors" })
        ] }),
        /* @__PURE__ */ s.jsx("ul", { className: "space-y-1", children: r.map((m, h) => /* @__PURE__ */ s.jsx("li", { className: "text-sm text-red-700", children: m }, h)) })
      ] }),
      n && /* @__PURE__ */ s.jsx("div", { className: "bg-blue-50 border border-blue-200 rounded-md p-3", children: /* @__PURE__ */ s.jsxs("div", { className: "flex items-center", children: [
        /* @__PURE__ */ s.jsxs("svg", { className: "animate-spin h-4 w-4 text-blue-400 mr-2", fill: "none", viewBox: "0 0 24 24", children: [
          /* @__PURE__ */ s.jsx("circle", { className: "opacity-25", cx: "12", cy: "12", r: "10", stroke: "currentColor", strokeWidth: "4" }),
          /* @__PURE__ */ s.jsx("path", { className: "opacity-75", fill: "currentColor", d: "M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" })
        ] }),
        /* @__PURE__ */ s.jsx("span", { className: "text-sm font-medium text-blue-800", children: "Executing query..." })
      ] }) }),
      /* @__PURE__ */ s.jsxs("div", { className: "space-y-3", children: [
        /* @__PURE__ */ s.jsxs("div", { children: [
          /* @__PURE__ */ s.jsx("label", { className: "block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1", children: "Schema" }),
          /* @__PURE__ */ s.jsx("div", { className: "inline-flex items-center px-2 py-1 rounded-md bg-blue-100 text-blue-800 text-sm font-medium", children: (f == null ? void 0 : f.schema) || "" })
        ] }),
        /* @__PURE__ */ s.jsxs("div", { children: [
          /* @__PURE__ */ s.jsxs("label", { className: "block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1", children: [
            "Fields (",
            f != null && f.fields ? f.fields.length : 0,
            ")"
          ] }),
          /* @__PURE__ */ s.jsx("div", { className: "flex flex-wrap gap-1", children: f != null && f.fields && f.fields.length > 0 ? f.fields.map((m, h) => {
            var x;
            const y = (x = f.fieldValues) == null ? void 0 : x[m];
            return /* @__PURE__ */ s.jsxs("div", { className: "inline-flex flex-col items-start", children: [
              /* @__PURE__ */ s.jsx("span", { className: "inline-flex items-center px-2 py-1 rounded-md bg-green-100 text-green-800 text-sm", children: m }),
              y && /* @__PURE__ */ s.jsx("span", { className: "text-xs text-gray-600 mt-1 px-2", children: y })
            ] }, h);
          }) : /* @__PURE__ */ s.jsx("span", { className: "text-sm text-gray-500 italic", children: "No fields selected" }) })
        ] }),
        (f.filters && Array.isArray(f.filters) && f.filters.length > 0 || f.filters && !Array.isArray(f.filters) && Object.keys(f.filters).length > 0) && /* @__PURE__ */ s.jsxs("div", { children: [
          /* @__PURE__ */ s.jsx("label", { className: "block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1", children: "Filters" }),
          /* @__PURE__ */ s.jsx("div", { className: "space-y-2", children: Array.isArray(f.filters) ? (
            // Handle filters as array (from test mocks)
            f.filters.map((m, h) => /* @__PURE__ */ s.jsx("div", { className: "bg-yellow-50 rounded-md p-3", children: /* @__PURE__ */ s.jsxs("div", { className: "text-sm text-yellow-700", children: [
              m.field,
              " ",
              m.operator,
              ' "',
              m.value,
              '"'
            ] }) }, h))
          ) : (
            // Handle filters as object (existing format)
            Object.entries(f.filters).map(([m, h]) => /* @__PURE__ */ s.jsxs("div", { className: "bg-yellow-50 rounded-md p-3", children: [
              /* @__PURE__ */ s.jsx("div", { className: "font-medium text-sm text-yellow-800 mb-1", children: m }),
              /* @__PURE__ */ s.jsxs("div", { className: "text-sm text-yellow-700", children: [
                h.exactKey && /* @__PURE__ */ s.jsxs("span", { children: [
                  "Exact key: ",
                  /* @__PURE__ */ s.jsx("code", { className: "bg-yellow-200 px-1 rounded", children: h.exactKey })
                ] }),
                h.keyRange && /* @__PURE__ */ s.jsxs("span", { children: [
                  "Key range: ",
                  /* @__PURE__ */ s.jsx("code", { className: "bg-yellow-200 px-1 rounded", children: h.keyRange })
                ] }),
                h.keyPrefix && /* @__PURE__ */ s.jsxs("span", { children: [
                  "Key prefix: ",
                  /* @__PURE__ */ s.jsx("code", { className: "bg-yellow-200 px-1 rounded", children: h.keyPrefix })
                ] })
              ] })
            ] }, m))
          ) })
        ] }),
        f.orderBy && /* @__PURE__ */ s.jsxs("div", { children: [
          /* @__PURE__ */ s.jsx("label", { className: "block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1", children: "OrderBy" }),
          /* @__PURE__ */ s.jsx("div", { className: "bg-purple-50 rounded-md p-3", children: /* @__PURE__ */ s.jsxs("div", { className: "text-sm text-purple-700", children: [
            f.orderBy.field,
            " ",
            f.orderBy.direction
          ] }) })
        ] }),
        f.rangeKey && /* @__PURE__ */ s.jsxs("div", { children: [
          /* @__PURE__ */ s.jsx("label", { className: "block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1", children: "RangeKey" }),
          /* @__PURE__ */ s.jsx("div", { className: "bg-indigo-50 rounded-md p-3", children: /* @__PURE__ */ s.jsx("div", { className: "text-sm text-indigo-700", children: /* @__PURE__ */ s.jsx("code", { className: "bg-indigo-200 px-1 rounded", children: f.rangeKey }) }) })
        ] })
      ] }),
      a && /* @__PURE__ */ s.jsxs("div", { className: "border-t border-gray-200 pt-4", children: [
        /* @__PURE__ */ s.jsx("label", { className: "block text-xs font-medium text-gray-500 uppercase tracking-wide mb-2", children: "Raw JSON" }),
        /* @__PURE__ */ s.jsx("pre", { className: "bg-gray-900 text-gray-100 text-xs p-3 rounded-md overflow-x-auto", children: JSON.stringify(e, null, 2) })
      ] })
    ] })
  ] });
}
function $u({ onResult: e }) {
  const {
    state: t,
    handleSchemaChange: r,
    toggleField: n,
    handleFieldValueChange: a,
    handleRangeFilterChange: l,
    setRangeSchemaFilter: d,
    setHashKeyValue: c,
    clearState: f,
    refetchSchemas: m,
    approvedSchemas: h,
    schemasLoading: y,
    selectedSchemaObj: x,
    isRangeSchema: N,
    isHashRangeSchema: S,
    rangeKey: E
  } = fn();
  xe(() => {
    m();
  }, [m]);
  const [p, v] = D(!1), { query: w, isValid: A } = vd({
    schema: t.selectedSchema,
    queryState: t,
    schemas: { [t.selectedSchema]: x }
  }), _ = H(async (R) => {
    if (!R) {
      e({
        error: "No query data provided"
      });
      return;
    }
    v(!0);
    try {
      const k = await un.executeQuery(R);
      if (!k.success) {
        console.error("Query failed:", k.error), e({
          error: k.error || "Query execution failed",
          details: k
        });
        return;
      }
      e({
        success: !0,
        data: k.data
        // The actual query results are directly in response.data
      });
    } catch (k) {
      console.error("Failed to execute query:", k), e({
        error: `Network error: ${k.message}`,
        details: k
      });
    } finally {
      v(!1);
    }
  }, [e, A]), T = H(async (R) => {
    console.log("Validating query:", R);
  }, []), M = H(async (R) => {
    if (!R || !A) {
      console.warn("Cannot save invalid query");
      return;
    }
    try {
      console.log("Saving query:", R);
      const k = JSON.parse(localStorage.getItem("savedQueries") || "[]"), I = {
        id: Date.now(),
        name: `Query ${k.length + 1}`,
        data: R,
        createdAt: (/* @__PURE__ */ new Date()).toISOString()
      };
      k.push(I), localStorage.setItem("savedQueries", JSON.stringify(k)), console.log("Query saved successfully");
    } catch (k) {
      console.error("Failed to save query:", k);
    }
  }, [A]);
  return /* @__PURE__ */ s.jsx("div", { className: "p-6", children: /* @__PURE__ */ s.jsxs("div", { className: "grid grid-cols-1 lg:grid-cols-3 gap-6", children: [
    /* @__PURE__ */ s.jsxs("div", { className: "lg:col-span-2 space-y-6", children: [
      /* @__PURE__ */ s.jsx(
        Vd,
        {
          queryState: t,
          onSchemaChange: r,
          onFieldToggle: n,
          onFieldValueChange: a,
          onRangeFilterChange: l,
          onRangeSchemaFilterChange: d,
          onHashKeyChange: c,
          approvedSchemas: h,
          schemasLoading: y,
          isRangeSchema: N,
          isHashRangeSchema: S,
          rangeKey: E
        }
      ),
      /* @__PURE__ */ s.jsx(
        Hd,
        {
          onExecute: () => _(w),
          onValidate: () => T(w),
          onSave: () => M(w),
          onClear: f,
          queryData: w,
          disabled: !A,
          isExecuting: p,
          showValidation: !1,
          showSave: !0,
          showClear: !0
        }
      )
    ] }),
    /* @__PURE__ */ s.jsx("div", { className: "lg:col-span-1", children: /* @__PURE__ */ s.jsx(
      Gd,
      {
        query: w,
        showJson: !1,
        title: "Query Preview"
      }
    ) })
  ] }) });
}
function Uu({ onResult: e }) {
  const t = wr(), r = he(Bl), n = he(Ll), a = he($l), l = he(Ul), d = he(Kl), c = he(Vl), f = or(null);
  xe(() => {
    var x;
    (x = f.current) == null || x.scrollIntoView({ behavior: "smooth" });
  }, [l]);
  const m = H((x, N, S = null) => {
    t(Dl({ type: x, content: N, data: S }));
  }, [t]), h = H(async (x) => {
    if (x == null || x.preventDefault(), !r.trim() || a)
      return;
    const N = r.trim();
    t(ea("")), t(ra(!0)), m("user", N);
    try {
      if (c) {
        m("system", "🤔 Analyzing if question can be answered from existing context...");
        const S = await aa.analyzeFollowup({
          session_id: n,
          question: N
        });
        if (!S.success) {
          m("system", `❌ Error: ${S.error || "Failed to analyze question"}`);
          return;
        }
        const E = S.data;
        if (E.needs_query) {
          m("system", `🔍 Need new data: ${E.reasoning}`), m("system", "🔍 Using AI-native index search...");
          const p = await fetch("/api/llm-query/native-index", {
            method: "POST",
            headers: {
              "Content-Type": "application/json"
            },
            body: JSON.stringify({
              query: N,
              session_id: n
            })
          });
          if (!p.ok) {
            const w = await p.json();
            m("system", `❌ Error: ${w.error || "Failed to run AI-native index query"}`);
            return;
          }
          const v = await p.json();
          m("system", "✅ AI-native index search completed"), v.session_id && t(ta(v.session_id)), m("system", v.ai_interpretation), m("results", "Raw search results", v.raw_results), d && e({ success: !0, data: v.raw_results });
        } else {
          m("system", `✅ Answering from existing context: ${E.reasoning}`);
          const p = await aa.chat({
            session_id: n,
            question: N
          });
          if (!p.success) {
            m("system", `❌ Error: ${p.error || "Failed to process question"}`);
            return;
          }
          m("system", p.data.answer);
        }
      } else {
        m("system", "🔍 Using AI-native index search...");
        const S = await fetch("/api/llm-query/native-index", {
          method: "POST",
          headers: {
            "Content-Type": "application/json"
          },
          body: JSON.stringify({
            query: N,
            session_id: n
          })
        });
        if (!S.ok) {
          const p = await S.json();
          m("system", `❌ Error: ${p.error || "Failed to run AI-native index query"}`);
          return;
        }
        const E = await S.json();
        m("system", "✅ AI-native index search completed"), E.session_id && t(ta(E.session_id)), m("system", E.ai_interpretation), m("results", "Raw search results", E.raw_results), d && e({ success: !0, data: E.raw_results });
      }
    } catch (S) {
      console.error("Error processing input:", S), m("system", `❌ Error: ${S.message}`), e({ error: S.message });
    } finally {
      t(ra(!1));
    }
  }, [r, n, c, a, m, e, t]), y = H(() => {
    t(Pl());
  }, [t]);
  return /* @__PURE__ */ s.jsxs("div", { className: "flex flex-col bg-white rounded-lg shadow", children: [
    /* @__PURE__ */ s.jsxs("div", { className: "p-4 border-b border-gray-200 flex justify-between items-center", children: [
      /* @__PURE__ */ s.jsxs("div", { children: [
        /* @__PURE__ */ s.jsx("h2", { className: "text-xl font-bold text-gray-900", children: "🤖 AI Data Assistant" }),
        /* @__PURE__ */ s.jsx("p", { className: "text-sm text-gray-600", children: "Ask questions in plain English - I'll find your data" })
      ] }),
      l.length > 0 && /* @__PURE__ */ s.jsx(
        "button",
        {
          onClick: y,
          disabled: a,
          className: "px-4 py-2 bg-gray-600 text-white rounded-lg hover:bg-gray-700 disabled:bg-gray-300 disabled:cursor-not-allowed transition-colors text-sm",
          children: "New Conversation"
        }
      )
    ] }),
    /* @__PURE__ */ s.jsxs("div", { className: "overflow-y-auto bg-gray-50 p-4 space-y-3", style: { maxHeight: "60vh", minHeight: "400px" }, children: [
      l.length === 0 ? /* @__PURE__ */ s.jsxs("div", { className: "text-center text-gray-500 mt-20", children: [
        /* @__PURE__ */ s.jsx("div", { className: "text-6xl mb-4", children: "💬" }),
        /* @__PURE__ */ s.jsx("p", { className: "text-lg mb-2", children: "Start a conversation" }),
        /* @__PURE__ */ s.jsx("p", { className: "text-sm", children: 'Try: "Find all blog posts from last month" or "Show me products over $100"' })
      ] }) : l.map((x, N) => /* @__PURE__ */ s.jsxs("div", { children: [
        x.type === "user" && /* @__PURE__ */ s.jsx("div", { className: "flex justify-end", children: /* @__PURE__ */ s.jsxs("div", { className: "bg-blue-600 text-white rounded-lg px-4 py-2 max-w-3xl", children: [
          /* @__PURE__ */ s.jsx("p", { className: "text-sm font-semibold mb-1", children: "You" }),
          /* @__PURE__ */ s.jsx("p", { className: "whitespace-pre-wrap", children: x.content })
        ] }) }),
        x.type === "system" && /* @__PURE__ */ s.jsx("div", { className: "flex justify-start", children: /* @__PURE__ */ s.jsxs("div", { className: "bg-white border border-gray-200 rounded-lg px-4 py-2 max-w-3xl", children: [
          /* @__PURE__ */ s.jsx("p", { className: "text-sm font-semibold text-gray-700 mb-1", children: "AI Assistant" }),
          /* @__PURE__ */ s.jsx("p", { className: "text-gray-900 whitespace-pre-wrap", children: x.content })
        ] }) }),
        x.type === "results" && x.data && /* @__PURE__ */ s.jsxs("div", { className: "bg-green-50 border border-green-200 rounded-lg p-4 max-w-full", children: [
          /* @__PURE__ */ s.jsxs("div", { className: "flex justify-between items-center mb-2", children: [
            /* @__PURE__ */ s.jsxs("p", { className: "text-sm font-semibold text-green-800", children: [
              "📊 Results (",
              x.data.length,
              ")"
            ] }),
            /* @__PURE__ */ s.jsx(
              "button",
              {
                onClick: () => {
                  const S = !d;
                  if (t(Fl(S)), S) {
                    const E = l.find((p) => p.type === "results");
                    E && E.data && e({ success: !0, data: E.data });
                  } else
                    e(null);
                },
                className: "text-sm text-green-700 hover:text-green-900 underline",
                children: d ? "Hide Details" : "Show Details"
              }
            )
          ] }),
          d && /* @__PURE__ */ s.jsxs(s.Fragment, { children: [
            /* @__PURE__ */ s.jsx("div", { className: "bg-white rounded p-3 mb-2", children: /* @__PURE__ */ s.jsx("p", { className: "text-gray-900 whitespace-pre-wrap mb-3", children: x.content }) }),
            /* @__PURE__ */ s.jsxs("details", { className: "mt-2", children: [
              /* @__PURE__ */ s.jsxs("summary", { className: "cursor-pointer text-sm text-green-700 hover:text-green-900", children: [
                "View raw data (",
                x.data.length,
                " records)"
              ] }),
              /* @__PURE__ */ s.jsx("div", { className: "mt-2 max-h-64 overflow-auto", children: /* @__PURE__ */ s.jsx("pre", { className: "text-xs bg-gray-900 text-green-400 p-3 rounded", children: JSON.stringify(x.data, null, 2) }) })
            ] })
          ] })
        ] })
      ] }, N)),
      /* @__PURE__ */ s.jsx("div", { ref: f })
    ] }),
    /* @__PURE__ */ s.jsxs("form", { onSubmit: h, className: "border-t border-gray-200 p-4 bg-white", children: [
      /* @__PURE__ */ s.jsxs("div", { className: "flex gap-2", children: [
        /* @__PURE__ */ s.jsx(
          "input",
          {
            type: "text",
            value: r,
            onChange: (x) => t(ea(x.target.value)),
            placeholder: l.some((x) => x.type === "results") ? "Ask a follow-up question or start a new query..." : "Search the native index (e.g., 'Find posts about AI')...",
            disabled: a,
            className: "flex-1 px-4 py-3 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent disabled:bg-gray-100",
            autoFocus: !0
          }
        ),
        /* @__PURE__ */ s.jsx(
          "button",
          {
            type: "submit",
            disabled: !r.trim() || a,
            className: "px-6 py-3 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:bg-gray-300 disabled:cursor-not-allowed transition-colors font-semibold",
            children: a ? "⏳ Processing..." : "Send"
          }
        )
      ] }),
      a && /* @__PURE__ */ s.jsx("p", { className: "text-center text-sm text-gray-500 mt-2", children: "AI is analyzing and searching..." })
    ] })
  ] });
}
function qd({ selectedSchema: e, mutationType: t, onSchemaChange: r, onTypeChange: n }) {
  const a = he(vr);
  return /* @__PURE__ */ s.jsxs("div", { className: "grid grid-cols-2 gap-4", children: [
    /* @__PURE__ */ s.jsx(
      Ks,
      {
        name: "schema",
        label: kt.schema,
        value: e,
        onChange: r,
        options: a.map((l) => ({
          value: l.name,
          label: l.descriptive_name || l.name
        })),
        placeholder: "Select a schema...",
        emptyMessage: "No approved schemas available for mutations",
        helpText: kt.schemaHelp
      }
    ),
    /* @__PURE__ */ s.jsx(
      Ks,
      {
        name: "operationType",
        label: kt.operationType,
        value: t,
        onChange: n,
        options: yl,
        helpText: kt.operationHelp
      }
    )
  ] });
}
function Wd({ fields: e, mutationType: t, mutationData: r, onFieldChange: n, isRangeSchema: a }) {
  if (t === "Delete")
    return /* @__PURE__ */ s.jsxs("div", { className: "bg-gray-50 rounded-lg p-6", children: [
      /* @__PURE__ */ s.jsx("h3", { className: "text-lg font-medium text-gray-900 mb-4", children: "Delete Operation" }),
      /* @__PURE__ */ s.jsx("p", { className: "text-sm text-gray-600", children: "This will delete the selected schema. No additional fields are required." })
    ] });
  const l = (d, c) => {
    if (!(c.writable !== !1)) return null;
    const m = r[d] || "";
    switch (c.field_type) {
      case "Collection": {
        let h = [];
        if (m)
          try {
            const y = typeof m == "string" ? JSON.parse(m) : m;
            h = Array.isArray(y) ? y : [y];
          } catch {
            h = m.trim() ? [m] : [];
          }
        return /* @__PURE__ */ s.jsxs("div", { className: "mb-6", children: [
          /* @__PURE__ */ s.jsxs("label", { className: "block text-sm font-medium text-gray-700 mb-2", children: [
            d,
            /* @__PURE__ */ s.jsx("span", { className: "ml-2 text-xs text-gray-500", children: "Collection" })
          ] }),
          /* @__PURE__ */ s.jsx(
            "textarea",
            {
              className: "mt-1 block w-full border-gray-300 rounded-md shadow-sm focus:ring-primary focus:border-primary sm:text-sm font-mono",
              value: h.length > 0 ? JSON.stringify(h, null, 2) : "",
              onChange: (y) => {
                const x = y.target.value.trim();
                if (!x) {
                  n(d, []);
                  return;
                }
                try {
                  const N = JSON.parse(x);
                  n(d, Array.isArray(N) ? N : [N]);
                } catch {
                  n(d, [x]);
                }
              },
              placeholder: 'Enter JSON array (e.g., ["item1", "item2"])',
              rows: 4
            }
          ),
          /* @__PURE__ */ s.jsx("p", { className: "mt-1 text-xs text-gray-500", children: "Enter data as a JSON array. Empty input will create an empty array." })
        ] }, d);
      }
      case "Range": {
        if (a)
          return /* @__PURE__ */ s.jsxs("div", { className: "mb-6", children: [
            /* @__PURE__ */ s.jsxs("label", { className: "block text-sm font-medium text-gray-700 mb-2", children: [
              d,
              /* @__PURE__ */ s.jsx("span", { className: "ml-2 text-xs text-gray-500", children: "Single Value (Range Schema)" })
            ] }),
            /* @__PURE__ */ s.jsx(
              "input",
              {
                type: "text",
                className: "mt-1 block w-full border-gray-300 rounded-md shadow-sm focus:ring-primary focus:border-primary sm:text-sm",
                value: m,
                onChange: (E) => n(d, E.target.value),
                placeholder: `Enter ${d} value`
              }
            ),
            /* @__PURE__ */ s.jsx("p", { className: "mt-1 text-xs text-gray-500", children: "Enter a single value. The system will automatically handle range formatting." })
          ] }, d);
        let h = {};
        if (m)
          try {
            h = typeof m == "string" ? JSON.parse(m) : m, (typeof h != "object" || Array.isArray(h)) && (h = {});
          } catch {
            h = {};
          }
        const y = Object.entries(h), x = () => {
          const E = [...y, ["", ""]], p = Object.fromEntries(E);
          n(d, p);
        }, N = (E, p, v) => {
          const w = [...y];
          w[E] = [p, v];
          const A = Object.fromEntries(w);
          n(d, A);
        }, S = (E) => {
          const p = y.filter((w, A) => A !== E), v = Object.fromEntries(p);
          n(d, v);
        };
        return /* @__PURE__ */ s.jsxs("div", { className: "mb-6", children: [
          /* @__PURE__ */ s.jsxs("label", { className: "block text-sm font-medium text-gray-700 mb-2", children: [
            d,
            /* @__PURE__ */ s.jsx("span", { className: "ml-2 text-xs text-gray-500", children: "Range (Complex)" })
          ] }),
          /* @__PURE__ */ s.jsx("div", { className: "border border-gray-300 rounded-md p-4 bg-gray-50", children: /* @__PURE__ */ s.jsxs("div", { className: "space-y-3", children: [
            y.length === 0 ? /* @__PURE__ */ s.jsx("p", { className: "text-sm text-gray-500 italic", children: "No key-value pairs added yet" }) : y.map(([E, p], v) => /* @__PURE__ */ s.jsxs("div", { className: "flex items-center space-x-2", children: [
              /* @__PURE__ */ s.jsx(
                "input",
                {
                  type: "text",
                  placeholder: "Key",
                  className: "flex-1 border-gray-300 rounded-md shadow-sm focus:ring-primary focus:border-primary sm:text-sm",
                  value: E,
                  onChange: (w) => N(v, w.target.value, p)
                }
              ),
              /* @__PURE__ */ s.jsx("span", { className: "text-gray-500", children: ":" }),
              /* @__PURE__ */ s.jsx(
                "input",
                {
                  type: "text",
                  placeholder: "Value",
                  className: "flex-1 border-gray-300 rounded-md shadow-sm focus:ring-primary focus:border-primary sm:text-sm",
                  value: p,
                  onChange: (w) => N(v, E, w.target.value)
                }
              ),
              /* @__PURE__ */ s.jsx(
                "button",
                {
                  type: "button",
                  onClick: () => S(v),
                  className: "text-red-600 hover:text-red-800 p-1",
                  title: "Remove this key-value pair",
                  children: /* @__PURE__ */ s.jsx("svg", { className: "w-4 h-4", fill: "none", stroke: "currentColor", viewBox: "0 0 24 24", children: /* @__PURE__ */ s.jsx("path", { strokeLinecap: "round", strokeLinejoin: "round", strokeWidth: 2, d: "M6 18L18 6M6 6l12 12" }) })
                }
              )
            ] }, v)),
            /* @__PURE__ */ s.jsxs(
              "button",
              {
                type: "button",
                onClick: x,
                className: "inline-flex items-center px-3 py-2 border border-gray-300 shadow-sm text-sm leading-4 font-medium rounded-md text-gray-700 bg-white hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-primary",
                children: [
                  /* @__PURE__ */ s.jsx("svg", { className: "w-4 h-4 mr-1", fill: "none", stroke: "currentColor", viewBox: "0 0 24 24", children: /* @__PURE__ */ s.jsx("path", { strokeLinecap: "round", strokeLinejoin: "round", strokeWidth: 2, d: "M12 6v6m0 0v6m0-6h6m-6 0H6" }) }),
                  "Add Key-Value Pair"
                ]
              }
            )
          ] }) }),
          /* @__PURE__ */ s.jsx("p", { className: "mt-1 text-xs text-gray-500", children: "Add key-value pairs for this range field. Empty keys will be filtered out." })
        ] }, d);
      }
      default:
        return /* @__PURE__ */ s.jsxs("div", { className: "mb-6", children: [
          /* @__PURE__ */ s.jsxs("label", { className: "block text-sm font-medium text-gray-700 mb-2", children: [
            d,
            /* @__PURE__ */ s.jsx("span", { className: "ml-2 text-xs text-gray-500", children: "Single" })
          ] }),
          /* @__PURE__ */ s.jsx(
            "input",
            {
              type: "text",
              className: "mt-1 block w-full border-gray-300 rounded-md shadow-sm focus:ring-primary focus:border-primary sm:text-sm",
              value: m,
              onChange: (h) => n(d, h.target.value),
              placeholder: `Enter ${d}`
            }
          )
        ] }, d);
    }
  };
  return /* @__PURE__ */ s.jsxs("div", { className: "bg-gray-50 rounded-lg p-6", children: [
    /* @__PURE__ */ s.jsxs("h3", { className: "text-lg font-medium text-gray-900 mb-4", children: [
      "Schema Fields",
      a && /* @__PURE__ */ s.jsx("span", { className: "ml-2 text-sm text-blue-600 font-normal", children: "(Range Schema - Single Values)" })
    ] }),
    /* @__PURE__ */ s.jsx("div", { className: "space-y-6", children: Object.entries(e).map(([d, c]) => l(d, c)) }),
    a && Object.keys(e).length === 0 && /* @__PURE__ */ s.jsx("p", { className: "text-sm text-gray-500 italic", children: "No additional fields to configure. Only the range key is required for this schema." })
  ] });
}
function Yd({ result: e }) {
  return e ? /* @__PURE__ */ s.jsx("div", { className: "bg-gray-50 rounded-lg p-4 mt-4", children: /* @__PURE__ */ s.jsx("pre", { className: "font-mono text-sm whitespace-pre-wrap", children: typeof e == "string" ? e : JSON.stringify(e, null, 2) }) }) : null;
}
function Qd(e) {
  const t = We(e);
  return {
    base: t,
    schema: Sl(t),
    mutation: td(t),
    security: Xl(t)
  };
}
Qd({
  enableCache: !0,
  enableLogging: !0,
  enableMetrics: !0
});
const Jd = { executeMutation: "Execute Mutation" }, ca = { rangeKeyRequired: "Range key is required", rangeKeyOptional: "Range key is optional for delete operations" }, la = { label: "Range Key", backgroundColor: "bg-blue-50" };
function Ku({ onResult: e }) {
  const t = he(vr);
  he((R) => R.auth);
  const [r, n] = D(""), [a, l] = D({}), [d, c] = D("Insert"), [f, m] = D(null), [h, y] = D(""), [x, N] = D({}), S = (R) => {
    n(R), l({}), c("Insert"), y("");
  }, E = (R, k) => {
    l((I) => ({ ...I, [R]: k }));
  }, p = async (R) => {
    if (R.preventDefault(), !r) return;
    const k = t.find((F) => F.name === r), I = d ? Ya[d] || d.toLowerCase() : "";
    if (!I)
      return;
    let $;
    Qt(k) ? $ = vl(k, d, h, a) : $ = {
      type: "mutation",
      schema: r,
      mutation_type: I,
      fields_and_values: d === "Delete" ? {} : a,
      key_value: { hash: null, range: null }
    };
    try {
      const F = await un.executeMutation($);
      if (!F.success)
        throw new Error(F.error || "Mutation failed");
      const z = F;
      m(z), e(z), z.success && (l({}), y(""));
    } catch (F) {
      const z = { error: `Network error: ${F.message}`, details: F };
      m(z), e(z);
    }
  }, v = r ? t.find((R) => R.name === r) : null, w = v ? Qt(v) : !1, A = v ? Dt(v) : null, T = !v || !Array.isArray(v.fields) ? {} : (w ? v.fields.filter((k) => k !== A) : v.fields).reduce((k, I) => (k[I] = {}, k), {}), M = !r || !d || d !== "Delete" && Object.keys(a).length === 0 || w && d !== "Delete" && !h.trim();
  return /* @__PURE__ */ s.jsxs("div", { className: "p-6", children: [
    /* @__PURE__ */ s.jsxs("form", { onSubmit: p, className: "space-y-6", children: [
      /* @__PURE__ */ s.jsx(
        qd,
        {
          selectedSchema: r,
          mutationType: d,
          onSchemaChange: S,
          onTypeChange: c
        }
      ),
      r && w && /* @__PURE__ */ s.jsxs("div", { className: `${la.backgroundColor} rounded-lg p-4`, children: [
        /* @__PURE__ */ s.jsx("h3", { className: "text-lg font-medium text-gray-900 mb-4", children: "Range Schema Configuration" }),
        /* @__PURE__ */ s.jsx(
          ir,
          {
            name: "rangeKey",
            label: `${A} (${la.label})`,
            value: h,
            onChange: y,
            placeholder: `Enter ${A} value`,
            required: d !== "Delete",
            error: x.rangeKey,
            helpText: d !== "Delete" ? ca.rangeKeyRequired : ca.rangeKeyOptional,
            debounced: !0
          }
        )
      ] }),
      r && /* @__PURE__ */ s.jsx(
        Wd,
        {
          fields: T,
          mutationType: d,
          mutationData: a,
          onFieldChange: E,
          isRangeSchema: w
        }
      ),
      /* @__PURE__ */ s.jsx("div", { className: "flex justify-end pt-4", children: /* @__PURE__ */ s.jsx(
        "button",
        {
          type: "submit",
          className: `inline-flex items-center px-4 py-2 border border-transparent rounded-md shadow-sm text-sm font-medium text-white ${M ? "bg-gray-300 cursor-not-allowed" : "bg-primary hover:bg-primary/90 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-primary"}`,
          disabled: M,
          children: Jd.executeMutation
        }
      ) })
    ] }),
    /* @__PURE__ */ s.jsx(Yd, { result: f })
  ] });
}
function pi({ progress: e, className: t = "" }) {
  if (!e)
    return null;
  const r = (l) => {
    switch (l) {
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
  }, n = (l) => {
    switch (l) {
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
        return l;
    }
  }, a = (l, d) => {
    const c = new Date(l), f = d ? new Date(d) : /* @__PURE__ */ new Date(), m = Math.round((f - c) / 1e3);
    if (m < 60)
      return `${m}s`;
    {
      const h = Math.floor(m / 60), y = m % 60;
      return `${h}m ${y}s`;
    }
  };
  return /* @__PURE__ */ s.jsxs("div", { className: `bg-white p-4 rounded-lg shadow border ${t}`, children: [
    /* @__PURE__ */ s.jsxs("div", { className: "flex items-center justify-between mb-3", children: [
      /* @__PURE__ */ s.jsxs("div", { className: "flex items-center gap-2", children: [
        /* @__PURE__ */ s.jsx("div", { className: `w-3 h-3 rounded-full ${r(e.current_step)}` }),
        /* @__PURE__ */ s.jsx("h3", { className: "text-sm font-medium text-gray-900", children: n(e.current_step) })
      ] }),
      /* @__PURE__ */ s.jsx("div", { className: "text-xs text-gray-500", children: a(e.started_at, e.completed_at) })
    ] }),
    /* @__PURE__ */ s.jsxs("div", { className: "mb-3", children: [
      /* @__PURE__ */ s.jsxs("div", { className: "flex justify-between text-xs text-gray-600 mb-1", children: [
        /* @__PURE__ */ s.jsxs("span", { children: [
          e.progress_percentage,
          "%"
        ] }),
        /* @__PURE__ */ s.jsx("span", { children: e.status_message })
      ] }),
      /* @__PURE__ */ s.jsx("div", { className: "w-full bg-gray-200 rounded-full h-2", children: /* @__PURE__ */ s.jsx(
        "div",
        {
          className: `h-2 rounded-full transition-all duration-300 ${r(e.current_step)}`,
          style: { width: `${e.progress_percentage}%` }
        }
      ) })
    ] }),
    e.results && /* @__PURE__ */ s.jsx("div", { className: "mt-3 p-3 bg-green-50 rounded-md", children: /* @__PURE__ */ s.jsxs("div", { className: "text-sm text-green-800", children: [
      /* @__PURE__ */ s.jsx("div", { className: "font-medium mb-1", children: "Ingestion Complete!" }),
      /* @__PURE__ */ s.jsxs("div", { className: "text-xs space-y-1", children: [
        /* @__PURE__ */ s.jsxs("div", { children: [
          "Schema: ",
          e.results.schema_name
        ] }),
        /* @__PURE__ */ s.jsxs("div", { children: [
          "New Schema: ",
          e.results.new_schema_created ? "Yes" : "No"
        ] }),
        /* @__PURE__ */ s.jsxs("div", { children: [
          "Mutations Generated: ",
          e.results.mutations_generated
        ] }),
        /* @__PURE__ */ s.jsxs("div", { children: [
          "Mutations Executed: ",
          e.results.mutations_executed
        ] })
      ] })
    ] }) }),
    e.error_message && /* @__PURE__ */ s.jsx("div", { className: "mt-3 p-3 bg-red-50 rounded-md", children: /* @__PURE__ */ s.jsxs("div", { className: "text-sm text-red-800", children: [
      /* @__PURE__ */ s.jsx("div", { className: "font-medium mb-1", children: "Ingestion Failed" }),
      /* @__PURE__ */ s.jsx("div", { className: "text-xs", children: e.error_message })
    ] }) }),
    /* @__PURE__ */ s.jsx("div", { className: "mt-4", children: /* @__PURE__ */ s.jsx("div", { className: "flex justify-between text-xs text-gray-500", children: [
      "ValidatingConfig",
      "PreparingSchemas",
      "FlatteningData",
      "GettingAIRecommendation",
      "SettingUpSchema",
      "GeneratingMutations",
      "ExecutingMutations"
    ].map((l, d) => {
      const c = e.current_step === l, f = e.progress_percentage > (d + 1) * 12.5;
      return /* @__PURE__ */ s.jsxs("div", { className: "flex flex-col items-center", children: [
        /* @__PURE__ */ s.jsx(
          "div",
          {
            className: `w-2 h-2 rounded-full mb-1 ${c || f ? r(l) : "bg-gray-300"}`
          }
        ),
        /* @__PURE__ */ s.jsx("span", { className: "text-xs text-center max-w-16 leading-tight", children: n(l).split(" ")[0] })
      ] }, l);
    }) }) })
  ] });
}
function Vu({ onResult: e }) {
  const [t, r] = D(""), [n, a] = D(!0), [l, d] = D(0), [c, f] = D("default"), [m, h] = D(!1), [y, x] = D(null), [N, S] = D(null), [E, p] = D(null);
  xe(() => {
    v();
  }, []), xe(() => {
    if (!E) return;
    const T = async () => {
      try {
        const R = await jt.getProgress(E);
        R.success && R.data && (S(R.data), R.data.is_complete && (h(!1), p(null), R.data.results ? e({
          success: !0,
          data: {
            schema_used: R.data.results.schema_name,
            new_schema_created: R.data.results.new_schema_created,
            mutations_generated: R.data.results.mutations_generated,
            mutations_executed: R.data.results.mutations_executed
          }
        }) : R.data.error_message && e({
          success: !1,
          error: R.data.error_message
        })));
      } catch (R) {
        console.error("Failed to fetch progress:", R);
      }
    };
    T();
    const M = setInterval(T, 200);
    return () => clearInterval(M);
  }, [E, e]);
  const v = async () => {
    try {
      const T = await jt.getStatus();
      T.success && x(T.data);
    } catch (T) {
      console.error("Failed to fetch ingestion status:", T);
    }
  }, w = async () => {
    h(!0), p(null), e(null), S({
      progress_percentage: 0,
      status_message: "Starting ingestion...",
      current_step: "ValidatingConfig",
      is_complete: !1,
      started_at: (/* @__PURE__ */ new Date()).toISOString()
    }), await new Promise((T) => setTimeout(T, 100));
    try {
      const T = JSON.parse(t), M = {
        autoExecute: n,
        trustDistance: l,
        pubKey: c
      }, R = await jt.processIngestion(T, M);
      R.success ? R.data.progress_id ? (p(R.data.progress_id), console.log("🟢 IngestionTab: Dispatching ingestion-started event", R.data.progress_id), window.dispatchEvent(new CustomEvent("ingestion-started", {
        detail: { progressId: R.data.progress_id }
      })), console.log("🟢 IngestionTab: Event dispatched")) : (e(R.data), r(""), h(!1)) : (e({
        success: !1,
        error: "Failed to process ingestion"
      }), h(!1), S(null));
    } catch (T) {
      e({
        success: !1,
        error: T.message || "Failed to process ingestion"
      }), h(!1), S(null);
    }
  }, A = () => {
    const T = [
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
    ], M = [
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
    ], R = [
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
    ], k = [];
    for (let I = 1; I <= 100; I++) {
      const $ = T[Math.floor(Math.random() * T.length)], F = M[Math.floor(Math.random() * M.length)], z = R[Math.floor(Math.random() * R.length)], V = /* @__PURE__ */ new Date(), G = new Date(V.getTime() - 6 * 30 * 24 * 60 * 60 * 1e3), L = G.getTime() + Math.random() * (V.getTime() - G.getTime()), J = new Date(L).toISOString().split("T")[0], Q = [
        `Getting Started with ${F}: A Complete Guide`,
        `Advanced ${F} Techniques You Need to Know`,
        `Why ${F} is Changing the Industry`,
        `Building Scalable Applications with ${F}`,
        `The Future of ${F}: Trends and Predictions`,
        `Common ${F} Mistakes and How to Avoid Them`,
        `Best Practices for ${F} Development`,
        `From Beginner to Expert in ${F}`,
        `Case Study: Implementing ${F} in Production`,
        `${F} Tools and Frameworks Comparison`
      ], ge = Q[Math.floor(Math.random() * Q.length)], Me = [
        `In this comprehensive guide, we'll explore the fundamentals of ${F} and how it's revolutionizing the way we approach modern development. Whether you're a seasoned developer or just starting out, this article will provide valuable insights into best practices and real-world applications.

## Introduction to ${F}

${F} has become an essential part of today's technology landscape. With its powerful capabilities and growing ecosystem, it offers developers unprecedented opportunities to build robust and scalable solutions.

## Key Concepts

Understanding the core concepts of ${F} is crucial for success. Let's dive into the fundamental principles that make this technology so powerful:

1. **Core Architecture**: The foundation of ${F} lies in its well-designed architecture
2. **Performance Optimization**: Learn how to maximize efficiency and minimize resource usage
3. **Integration Patterns**: Discover best practices for connecting with other systems
4. **Security Considerations**: Implement robust security measures from the ground up

## Real-World Applications

Many companies have successfully implemented ${F} in their production environments. Here are some notable examples:

- **Case Study 1**: A major e-commerce platform reduced their response time by 60%
- **Case Study 2**: A fintech startup improved their scalability by 300%
- **Case Study 3**: A healthcare company enhanced their data processing capabilities

## Getting Started

Ready to dive in? Here's a step-by-step guide to get you started with ${F}:

\`\`\`javascript
// Example implementation
const example = new ${F}();
example.initialize();
example.process();
\`\`\`

## Conclusion

${F} represents a significant advancement in technology, offering developers powerful tools to build the next generation of applications. By following the principles and practices outlined in this guide, you'll be well-equipped to leverage ${F} in your own projects.

Remember, the key to success with ${F} is continuous learning and experimentation. Stay curious, keep building, and don't hesitate to explore new possibilities!`,
        `The landscape of ${F} is constantly evolving, and staying ahead of the curve requires a deep understanding of both current trends and emerging technologies. In this article, we'll examine the latest developments and provide actionable insights for developers looking to enhance their skills.

## Current State of ${F}

Today's ${F} ecosystem is more mature and feature-rich than ever before. With improved tooling, better documentation, and a growing community, developers have access to resources that make implementation more straightforward.

## Emerging Trends

Several key trends are shaping the future of ${F}:

- **Automation**: Increasing focus on automated workflows and CI/CD integration
- **Performance**: New optimization techniques that improve speed and efficiency
- **Security**: Enhanced security features and best practices
- **Scalability**: Better support for large-scale deployments

## Industry Impact

The adoption of ${F} across various industries has been remarkable:

- **Technology Sector**: 85% of tech companies have implemented ${F} solutions
- **Financial Services**: Improved transaction processing and risk management
- **Healthcare**: Enhanced patient data management and analysis
- **E-commerce**: Better customer experience and operational efficiency

## Implementation Strategies

When implementing ${F}, consider these strategic approaches:

1. **Phased Rollout**: Start with pilot projects before full deployment
2. **Team Training**: Invest in comprehensive team education
3. **Monitoring**: Implement robust monitoring and alerting systems
4. **Documentation**: Maintain detailed documentation for future reference

## Future Outlook

Looking ahead, ${F} is poised for continued growth and innovation. Key areas to watch include:

- Advanced AI integration
- Improved developer experience
- Enhanced security features
- Better cross-platform compatibility

The future of ${F} is bright, and developers who invest in learning these technologies now will be well-positioned for success in the years to come.`,
        `Building robust applications with ${F} requires more than just technical knowledge—it demands a strategic approach to architecture, design, and implementation. In this deep dive, we'll explore advanced techniques that will elevate your ${F} development skills.

## Architecture Patterns

Effective ${F} applications rely on well-established architectural patterns:

### Microservices Architecture
Breaking down monolithic applications into smaller, manageable services provides better scalability and maintainability.

### Event-Driven Design
Implementing event-driven patterns enables better decoupling and improved system responsiveness.

### Domain-Driven Design
Organizing code around business domains leads to more maintainable and understandable applications.

## Performance Optimization

Optimizing ${F} applications requires attention to multiple factors:

- **Caching Strategies**: Implement intelligent caching to reduce database load
- **Resource Management**: Optimize memory usage and CPU utilization
- **Network Optimization**: Minimize network overhead and latency
- **Database Tuning**: Optimize queries and indexing strategies

## Testing Strategies

Comprehensive testing is essential for reliable ${F} applications:

\`\`\`javascript
// Example test structure
describe('${F} Component', () => {
  it('should handle basic functionality', () => {
    const component = new ${F}Component();
    expect(component.process()).toBeDefined();
  });
  
  it('should handle edge cases', () => {
    const component = new ${F}Component();
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

Security should be a primary concern when developing ${F} applications:

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

Mastering ${F} development is an ongoing journey that requires continuous learning and adaptation. By implementing these advanced techniques and best practices, you'll build more robust, scalable, and maintainable applications.

The key to success lies in understanding not just the technical aspects, but also the business context and user needs. Keep experimenting, stay updated with the latest developments, and always prioritize code quality and user experience.`
      ], ze = Me[Math.floor(Math.random() * Me.length)];
      k.push({
        title: ge,
        content: ze,
        author: $,
        publish_date: J,
        tags: z
      });
    }
    return k;
  }, _ = (T) => {
    const M = {
      blogposts: A(),
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
    r(JSON.stringify(M[T], null, 2));
  };
  return /* @__PURE__ */ s.jsxs("div", { className: "space-y-4", children: [
    y && /* @__PURE__ */ s.jsx("div", { className: "bg-white p-3 rounded-lg shadow-sm border border-gray-200", children: /* @__PURE__ */ s.jsxs("div", { className: "flex items-center gap-4 text-sm", children: [
      /* @__PURE__ */ s.jsx("span", { className: `px-2 py-1 rounded text-xs font-medium ${y.enabled && y.configured ? "bg-green-100 text-green-800" : "bg-red-100 text-red-800"}`, children: y.enabled && y.configured ? "Ready" : "Not Configured" }),
      /* @__PURE__ */ s.jsxs("span", { className: "text-gray-600", children: [
        y.provider,
        " · ",
        y.model
      ] }),
      /* @__PURE__ */ s.jsx("span", { className: "text-xs text-gray-500", children: "Configure AI settings using the Settings button in the header" })
    ] }) }),
    N && /* @__PURE__ */ s.jsx(pi, { progress: N }),
    /* @__PURE__ */ s.jsxs("div", { className: "bg-white p-4 rounded-lg shadow", children: [
      /* @__PURE__ */ s.jsxs("div", { className: "flex items-center justify-between mb-3", children: [
        /* @__PURE__ */ s.jsx("h3", { className: "text-lg font-medium text-gray-900", children: "JSON Data" }),
        /* @__PURE__ */ s.jsxs("div", { className: "flex gap-2", children: [
          /* @__PURE__ */ s.jsx(
            "button",
            {
              onClick: () => _("blogposts"),
              className: "px-2 py-1 bg-green-50 text-green-700 rounded text-xs hover:bg-green-100",
              children: "Blog Posts (100)"
            }
          ),
          /* @__PURE__ */ s.jsx(
            "button",
            {
              onClick: () => _("twitter"),
              className: "px-2 py-1 bg-blue-50 text-blue-700 rounded text-xs hover:bg-blue-100",
              children: "Twitter"
            }
          ),
          /* @__PURE__ */ s.jsx(
            "button",
            {
              onClick: () => _("instagram"),
              className: "px-2 py-1 bg-pink-50 text-pink-700 rounded text-xs hover:bg-pink-100",
              children: "Instagram"
            }
          ),
          /* @__PURE__ */ s.jsx(
            "button",
            {
              onClick: () => _("linkedin"),
              className: "px-2 py-1 bg-indigo-50 text-indigo-700 rounded text-xs hover:bg-indigo-100",
              children: "LinkedIn"
            }
          ),
          /* @__PURE__ */ s.jsx(
            "button",
            {
              onClick: () => _("tiktok"),
              className: "px-2 py-1 bg-purple-50 text-purple-700 rounded text-xs hover:bg-purple-100",
              children: "TikTok"
            }
          )
        ] })
      ] }),
      /* @__PURE__ */ s.jsx(
        "textarea",
        {
          id: "jsonData",
          value: t,
          onChange: (T) => r(T.target.value),
          placeholder: "Enter your JSON data here or load a sample...",
          className: "w-full h-64 p-3 border border-gray-300 rounded-md font-mono text-sm focus:ring-2 focus:ring-blue-500 focus:border-transparent"
        }
      )
    ] }),
    /* @__PURE__ */ s.jsx("div", { className: "bg-white p-4 rounded-lg shadow", children: /* @__PURE__ */ s.jsxs("div", { className: "flex items-center justify-between", children: [
      /* @__PURE__ */ s.jsxs("div", { className: "flex items-center gap-4", children: [
        /* @__PURE__ */ s.jsxs("label", { className: "flex items-center gap-2 text-sm", children: [
          /* @__PURE__ */ s.jsx(
            "input",
            {
              type: "checkbox",
              checked: n,
              onChange: (T) => a(T.target.checked),
              className: "rounded"
            }
          ),
          /* @__PURE__ */ s.jsx("span", { className: "text-gray-700", children: "Auto-execute mutations" })
        ] }),
        /* @__PURE__ */ s.jsx("span", { className: "text-xs text-gray-500", children: "AI will analyze and automatically map data to schemas" })
      ] }),
      /* @__PURE__ */ s.jsx(
        "button",
        {
          onClick: w,
          disabled: m || !t.trim(),
          className: `px-6 py-2.5 rounded font-medium transition-colors ${m || !t.trim() ? "bg-gray-300 text-gray-500 cursor-not-allowed" : "bg-blue-600 text-white hover:bg-blue-700"}`,
          children: m ? "Processing..." : "Process Data"
        }
      )
    ] }) })
  ] });
}
function Hu({ onResult: e }) {
  const [t, r] = D(!1), [n, a] = D(null), [l, d] = D(!0), [c, f] = D(0), [m, h] = D("default"), [y, x] = D(!1), [N, S] = D(null), [E, p] = D(null), [v, w] = D(null), [A, _] = D(!1), [T, M] = D("");
  xe(() => {
    R();
  }, []), xe(() => {
    if (!v) return;
    const L = async () => {
      try {
        const Q = await jt.getProgress(v);
        Q.success && Q.data && (p(Q.data), Q.data.is_complete && (x(!1), w(null), Q.data.results ? e({
          success: !0,
          data: {
            schema_used: Q.data.results.schema_name,
            new_schema_created: Q.data.results.new_schema_created,
            mutations_generated: Q.data.results.mutations_generated,
            mutations_executed: Q.data.results.mutations_executed
          }
        }) : Q.data.error_message && e({
          success: !1,
          error: Q.data.error_message
        })));
      } catch (Q) {
        console.error("Failed to fetch progress:", Q);
      }
    };
    L();
    const J = setInterval(L, 200);
    return () => clearInterval(J);
  }, [v, e]);
  const R = async () => {
    try {
      const L = await jt.getStatus();
      L.success && S(L.data);
    } catch (L) {
      console.error("Failed to fetch ingestion status:", L);
    }
  }, k = H((L) => {
    L.preventDefault(), L.stopPropagation(), r(!0);
  }, []), I = H((L) => {
    L.preventDefault(), L.stopPropagation(), r(!1);
  }, []), $ = H((L) => {
    L.preventDefault(), L.stopPropagation();
  }, []), F = H((L) => {
    L.preventDefault(), L.stopPropagation(), r(!1);
    const J = L.dataTransfer.files;
    J && J.length > 0 && a(J[0]);
  }, []), z = H((L) => {
    const J = L.target.files;
    J && J.length > 0 && a(J[0]);
  }, []), V = async () => {
    if (A) {
      if (!T || !T.startsWith("s3://")) {
        e({
          success: !1,
          error: "Please provide a valid S3 path (e.g., s3://bucket/path/to/file.json)"
        });
        return;
      }
    } else if (!n) {
      e({
        success: !1,
        error: "Please select a file to upload"
      });
      return;
    }
    x(!0), w(null), e(null), p({
      progress_percentage: 0,
      status_message: A ? "Processing S3 file..." : "Uploading file...",
      current_step: "ValidatingConfig",
      is_complete: !1,
      started_at: (/* @__PURE__ */ new Date()).toISOString()
    }), await new Promise((L) => setTimeout(L, 100));
    try {
      const L = new FormData();
      A ? L.append("s3FilePath", T) : L.append("file", n), L.append("autoExecute", l.toString()), L.append("trustDistance", c.toString()), L.append("pubKey", m);
      const Q = await (await fetch("/api/ingestion/upload", {
        method: "POST",
        body: L
      })).json();
      Q.success && Q.progress_id ? (w(Q.progress_id), console.log("🟢 FileUploadTab: Dispatching ingestion-started event", Q.progress_id), window.dispatchEvent(new CustomEvent("ingestion-started", {
        detail: { progressId: Q.progress_id }
      })), console.log("🟢 FileUploadTab: Event dispatched")) : (e({
        success: !1,
        error: Q.error || "Failed to process file"
      }), x(!1), p(null));
    } catch (L) {
      e({
        success: !1,
        error: L.message || "Failed to process file"
      }), x(!1), p(null);
    }
  }, G = (L) => {
    if (L === 0) return "0 Bytes";
    const J = 1024, Q = ["Bytes", "KB", "MB", "GB"], ge = Math.floor(Math.log(L) / Math.log(J));
    return Math.round(L / Math.pow(J, ge) * 100) / 100 + " " + Q[ge];
  };
  return /* @__PURE__ */ s.jsxs("div", { className: "space-y-4", children: [
    N && /* @__PURE__ */ s.jsx("div", { className: "bg-white p-3 rounded-lg shadow-sm border border-gray-200", children: /* @__PURE__ */ s.jsxs("div", { className: "flex items-center gap-4 text-sm", children: [
      /* @__PURE__ */ s.jsx("span", { className: `px-2 py-1 rounded text-xs font-medium ${N.enabled && N.configured ? "bg-green-100 text-green-800" : "bg-red-100 text-red-800"}`, children: N.enabled && N.configured ? "Ready" : "Not Configured" }),
      /* @__PURE__ */ s.jsxs("span", { className: "text-gray-600", children: [
        N.provider,
        " · ",
        N.model
      ] }),
      /* @__PURE__ */ s.jsx("span", { className: "text-xs text-gray-500", children: "Configure AI settings using the Settings button in the header" })
    ] }) }),
    E && /* @__PURE__ */ s.jsx(pi, { progress: E }),
    /* @__PURE__ */ s.jsx("div", { className: "bg-white p-4 rounded-lg shadow", children: /* @__PURE__ */ s.jsxs("div", { className: "flex items-center gap-6", children: [
      /* @__PURE__ */ s.jsx("span", { className: "text-sm font-medium text-gray-700", children: "Input Mode:" }),
      /* @__PURE__ */ s.jsxs("label", { className: "flex items-center gap-2 cursor-pointer", children: [
        /* @__PURE__ */ s.jsx(
          "input",
          {
            type: "radio",
            checked: !A,
            onChange: () => _(!1),
            className: "rounded"
          }
        ),
        /* @__PURE__ */ s.jsx("span", { className: "text-sm text-gray-700", children: "Upload File" })
      ] }),
      /* @__PURE__ */ s.jsxs("label", { className: "flex items-center gap-2 cursor-pointer", children: [
        /* @__PURE__ */ s.jsx(
          "input",
          {
            type: "radio",
            checked: A,
            onChange: () => _(!0),
            className: "rounded"
          }
        ),
        /* @__PURE__ */ s.jsx("span", { className: "text-sm text-gray-700", children: "S3 File Path" })
      ] })
    ] }) }),
    A ? /* @__PURE__ */ s.jsxs("div", { className: "bg-white p-6 rounded-lg shadow", children: [
      /* @__PURE__ */ s.jsx("h3", { className: "text-lg font-medium text-gray-900 mb-4", children: "S3 File Path" }),
      /* @__PURE__ */ s.jsxs("div", { className: "space-y-3", children: [
        /* @__PURE__ */ s.jsx("label", { className: "block text-sm font-medium text-gray-700", children: "Enter S3 file path" }),
        /* @__PURE__ */ s.jsx(
          "input",
          {
            type: "text",
            value: T,
            onChange: (L) => M(L.target.value),
            placeholder: "s3://bucket-name/path/to/file.json",
            className: "w-full px-4 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
          }
        ),
        /* @__PURE__ */ s.jsx("p", { className: "text-xs text-gray-500", children: "The file will be downloaded from S3 for processing without re-uploading" })
      ] })
    ] }) : /* @__PURE__ */ s.jsxs("div", { className: "bg-white p-6 rounded-lg shadow", children: [
      /* @__PURE__ */ s.jsx("h3", { className: "text-lg font-medium text-gray-900 mb-4", children: "Upload File" }),
      /* @__PURE__ */ s.jsx(
        "div",
        {
          className: `border-2 border-dashed rounded-lg p-12 text-center transition-colors ${t ? "border-blue-500 bg-blue-50" : "border-gray-300 bg-gray-50 hover:bg-gray-100"}`,
          onDragEnter: k,
          onDragOver: $,
          onDragLeave: I,
          onDrop: F,
          children: /* @__PURE__ */ s.jsxs("div", { className: "space-y-4", children: [
            /* @__PURE__ */ s.jsx("div", { className: "flex justify-center", children: /* @__PURE__ */ s.jsx(
              "svg",
              {
                className: "w-16 h-16 text-gray-400",
                fill: "none",
                stroke: "currentColor",
                viewBox: "0 0 24 24",
                xmlns: "http://www.w3.org/2000/svg",
                children: /* @__PURE__ */ s.jsx(
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
            n ? /* @__PURE__ */ s.jsxs("div", { className: "space-y-2", children: [
              /* @__PURE__ */ s.jsx("p", { className: "text-lg font-medium text-gray-900", children: n.name }),
              /* @__PURE__ */ s.jsx("p", { className: "text-sm text-gray-500", children: G(n.size) }),
              /* @__PURE__ */ s.jsx(
                "button",
                {
                  onClick: () => a(null),
                  className: "text-sm text-blue-600 hover:text-blue-700 underline",
                  children: "Remove file"
                }
              )
            ] }) : /* @__PURE__ */ s.jsxs("div", { children: [
              /* @__PURE__ */ s.jsx("p", { className: "text-lg text-gray-700 mb-2", children: "Drag and drop a file here, or click to select" }),
              /* @__PURE__ */ s.jsx("p", { className: "text-sm text-gray-500", children: "Supported formats: PDF, DOCX, TXT, CSV, JSON, XML, and more" })
            ] }),
            /* @__PURE__ */ s.jsx(
              "input",
              {
                type: "file",
                id: "file-upload",
                className: "hidden",
                onChange: z
              }
            ),
            !n && /* @__PURE__ */ s.jsx(
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
    /* @__PURE__ */ s.jsx("div", { className: "bg-white p-4 rounded-lg shadow", children: /* @__PURE__ */ s.jsxs("div", { className: "flex items-center justify-between", children: [
      /* @__PURE__ */ s.jsxs("div", { className: "flex items-center gap-4", children: [
        /* @__PURE__ */ s.jsxs("label", { className: "flex items-center gap-2 text-sm", children: [
          /* @__PURE__ */ s.jsx(
            "input",
            {
              type: "checkbox",
              checked: l,
              onChange: (L) => d(L.target.checked),
              className: "rounded"
            }
          ),
          /* @__PURE__ */ s.jsx("span", { className: "text-gray-700", children: "Auto-execute mutations" })
        ] }),
        /* @__PURE__ */ s.jsx("span", { className: "text-xs text-gray-500", children: "File will be converted to JSON and processed by AI" })
      ] }),
      /* @__PURE__ */ s.jsx(
        "button",
        {
          onClick: V,
          disabled: y || !A && !n || A && !T,
          className: `px-6 py-2.5 rounded font-medium transition-colors ${y || !A && !n || A && !T ? "bg-gray-300 text-gray-500 cursor-not-allowed" : "bg-blue-600 text-white hover:bg-blue-700"}`,
          children: y ? "Processing..." : A ? "Process S3 File" : "Upload & Process"
        }
      )
    ] }) }),
    /* @__PURE__ */ s.jsx("div", { className: "bg-blue-50 border border-blue-200 rounded-lg p-4", children: /* @__PURE__ */ s.jsxs("div", { className: "flex items-start gap-3", children: [
      /* @__PURE__ */ s.jsx(
        "svg",
        {
          className: "w-6 h-6 text-blue-600 flex-shrink-0 mt-0.5",
          fill: "none",
          stroke: "currentColor",
          viewBox: "0 0 24 24",
          children: /* @__PURE__ */ s.jsx(
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
      /* @__PURE__ */ s.jsxs("div", { className: "text-sm text-blue-800", children: [
        /* @__PURE__ */ s.jsx("p", { className: "font-medium mb-1", children: "How it works:" }),
        /* @__PURE__ */ s.jsxs("ol", { className: "list-decimal list-inside space-y-1", children: [
          /* @__PURE__ */ s.jsx("li", { children: A ? "Provide an S3 file path (files already in S3 are not re-uploaded)" : "Upload any file type (PDFs, documents, spreadsheets, etc.)" }),
          /* @__PURE__ */ s.jsx("li", { children: "File is automatically converted to JSON using AI" }),
          /* @__PURE__ */ s.jsx("li", { children: "AI analyzes the JSON and maps it to appropriate schemas" }),
          /* @__PURE__ */ s.jsx("li", { children: "Data is stored in the database with the file location tracked" })
        ] })
      ] })
    ] }) })
  ] });
}
function Zd() {
  const e = wr(), t = he(vr), r = he(Zt), n = he(dn), a = he(ri), l = he(kl), d = H(async () => {
    e(tt({ forceRefresh: !0 }));
  }, [e]), c = H((m) => r.find((h) => h.name === m) || null, [r]), f = H((m) => {
    const h = c(m);
    return h ? Xa(h.state) === st.APPROVED : !1;
  }, [c]);
  return xe(() => {
    l.isValid || (console.log("🟡 useApprovedSchemas: Cache invalid, fetching schemas"), e(tt()));
  }, [e]), {
    approvedSchemas: t,
    isLoading: n,
    error: a,
    refetch: d,
    getSchemaByName: c,
    isSchemaApproved: f,
    // Additional utility for components that need all schemas for display
    allSchemas: r
  };
}
function Xd({ r: e }) {
  var t, r;
  return /* @__PURE__ */ s.jsxs("tr", { className: "border-t", children: [
    /* @__PURE__ */ s.jsx("td", { className: "px-2 py-1 text-xs text-gray-600", children: ((t = e.key_value) == null ? void 0 : t.hash) ?? "" }),
    /* @__PURE__ */ s.jsx("td", { className: "px-2 py-1 text-xs text-gray-600", children: ((r = e.key_value) == null ? void 0 : r.range) ?? "" }),
    /* @__PURE__ */ s.jsx("td", { className: "px-2 py-1 text-xs font-mono text-gray-800", children: e.schema_name }),
    /* @__PURE__ */ s.jsx("td", { className: "px-2 py-1 text-xs text-gray-800", children: e.field }),
    /* @__PURE__ */ s.jsx("td", { className: "px-2 py-1 text-xs text-gray-800 whitespace-pre-wrap break-words", children: eu(e.value) })
  ] });
}
function eu(e) {
  if (e == null) return "";
  if (typeof e == "string") return e;
  try {
    return JSON.stringify(e);
  } catch {
    return String(e);
  }
}
function zu({ onResult: e }) {
  const { approvedSchemas: t, isLoading: r, refetch: n } = Zd(), [a, l] = D(""), [d, c] = D(!1), [f, m] = D([]), [h, y] = D(null), [x, N] = D(() => /* @__PURE__ */ new Set()), [S, E] = D(() => /* @__PURE__ */ new Map());
  xe(() => {
    n();
  }, [n]);
  const p = H(async () => {
    c(!0), y(null);
    try {
      const R = await nd.search(a);
      R.success ? (m(R.data || []), e({ success: !0, data: R.data || [] })) : (y(R.error || "Search failed"), e({ error: R.error || "Search failed", status: R.status }));
    } catch (R) {
      y(R.message || "Network error"), e({ error: R.message || "Network error" });
    } finally {
      c(!1);
    }
  }, [a, e]), v = H((R) => {
    if (!R) return [];
    const k = R.fields;
    return Array.isArray(k) ? k.slice() : k && typeof k == "object" ? Object.keys(k) : [];
  }, []), w = ye(() => {
    const R = /* @__PURE__ */ new Map();
    return (t || []).forEach((k) => R.set(k.name, k)), R;
  }, [t]), A = H((R, k) => {
    const I = (k == null ? void 0 : k.hash) ?? "", $ = (k == null ? void 0 : k.range) ?? "";
    return `${R}|${I}|${$}`;
  }, []), _ = H((R) => {
    const k = R == null ? void 0 : R.hash, I = R == null ? void 0 : R.range;
    if (k && I) return bd(k, I);
    if (k) return dr(k);
    if (I) return dr(I);
  }, []), T = H(async (R, k) => {
    const I = w.get(R), $ = v(I), F = _(k), z = { schema_name: R, fields: $ };
    F && (z.filter = F);
    const V = await un.executeQuery(z);
    if (!V.success)
      throw new Error(V.error || "Query failed");
    const G = Array.isArray(V.data) ? V.data : [], L = G.find((J) => {
      var ne, ce;
      const Q = ((ne = J == null ? void 0 : J.key) == null ? void 0 : ne.hash) ?? null, ge = ((ce = J == null ? void 0 : J.key) == null ? void 0 : ce.range) ?? null, Me = (k == null ? void 0 : k.hash) ?? null, ze = (k == null ? void 0 : k.range) ?? null;
      return String(Q || "") === String(Me || "") && String(ge || "") === String(ze || "");
    }) || G[0];
    return (L == null ? void 0 : L.fields) || (L && typeof L == "object" ? L : {});
  }, [w, v, _]), M = H(async () => {
    const R = /* @__PURE__ */ new Map();
    for (const $ of f) {
      const F = A($.schema_name, $.key_value);
      R.has(F) || R.set(F, $);
    }
    const k = Array.from(R.values()), I = new Map(S);
    await Promise.all(k.map(async ($) => {
      const F = A($.schema_name, $.key_value);
      if (!I.has(F))
        try {
          const z = await T($.schema_name, $.key_value);
          I.set(F, z);
        } catch {
          I.set(F, {});
        }
    })), E(I);
  }, [f, S, A, T]);
  return xe(() => {
    f.length > 0 && M().catch(() => {
    });
  }, [f, M]), /* @__PURE__ */ s.jsxs("div", { className: "p-6 space-y-4", children: [
    /* @__PURE__ */ s.jsxs("div", { className: "bg-white p-4 rounded-lg shadow", children: [
      /* @__PURE__ */ s.jsxs("div", { className: "mb-3", children: [
        /* @__PURE__ */ s.jsx("h3", { className: "text-lg font-medium text-gray-900", children: "Native Index Search" }),
        /* @__PURE__ */ s.jsx("p", { className: "text-xs text-gray-500", children: "Search the database-native word index across all approved schemas." })
      ] }),
      /* @__PURE__ */ s.jsxs("div", { className: "flex gap-2 items-center", children: [
        /* @__PURE__ */ s.jsx(
          "input",
          {
            type: "text",
            value: a,
            onChange: (R) => l(R.target.value),
            placeholder: "Enter search term (e.g. jennifer)",
            className: "flex-1 px-3 py-2 border rounded-md text-sm"
          }
        ),
        /* @__PURE__ */ s.jsx(
          "button",
          {
            onClick: p,
            disabled: d || !a.trim(),
            className: `px-4 py-2 rounded text-sm ${d || !a.trim() ? "bg-gray-300 text-gray-600" : "bg-blue-600 text-white hover:bg-blue-700"}`,
            children: d ? "Searching..." : "Search"
          }
        )
      ] })
    ] }),
    /* @__PURE__ */ s.jsxs("div", { className: "bg-white p-4 rounded-lg shadow", children: [
      /* @__PURE__ */ s.jsxs("div", { className: "mb-2 flex items-center justify-between", children: [
        /* @__PURE__ */ s.jsx("h4", { className: "text-md font-medium text-gray-900", children: "Search Results" }),
        /* @__PURE__ */ s.jsxs("div", { className: "flex items-center gap-3", children: [
          /* @__PURE__ */ s.jsxs("span", { className: "text-xs text-gray-500", children: [
            f.length,
            " matches"
          ] }),
          f.length > 0 && /* @__PURE__ */ s.jsx(
            "button",
            {
              type: "button",
              className: "text-xs px-2 py-1 rounded border border-gray-300 hover:bg-gray-100",
              onClick: () => M(),
              children: "Refresh Details"
            }
          )
        ] })
      ] }),
      h && /* @__PURE__ */ s.jsx("div", { className: "mb-2 p-2 bg-red-50 border border-red-200 text-xs text-red-700 rounded", children: h }),
      /* @__PURE__ */ s.jsx("div", { className: "overflow-auto max-h-[450px]", children: /* @__PURE__ */ s.jsxs("table", { className: "min-w-full text-left text-xs", children: [
        /* @__PURE__ */ s.jsx("thead", { children: /* @__PURE__ */ s.jsxs("tr", { className: "text-gray-500", children: [
          /* @__PURE__ */ s.jsx("th", { className: "px-2 py-1", children: "Hash" }),
          /* @__PURE__ */ s.jsx("th", { className: "px-2 py-1", children: "Range" }),
          /* @__PURE__ */ s.jsx("th", { className: "px-2 py-1", children: "Schema" }),
          /* @__PURE__ */ s.jsx("th", { className: "px-2 py-1", children: "Field" }),
          /* @__PURE__ */ s.jsx("th", { className: "px-2 py-1", children: "Value" }),
          /* @__PURE__ */ s.jsx("th", { className: "px-2 py-1" })
        ] }) }),
        /* @__PURE__ */ s.jsxs("tbody", { children: [
          f.map((R, k) => {
            const I = A(R.schema_name, R.key_value), $ = x.has(I), F = S.get(I);
            return /* @__PURE__ */ s.jsxs(s.Fragment, { children: [
              /* @__PURE__ */ s.jsx(Xd, { r: R }, `${I}-row`),
              /* @__PURE__ */ s.jsxs("tr", { className: "border-b", children: [
                /* @__PURE__ */ s.jsx("td", { colSpan: 5 }),
                /* @__PURE__ */ s.jsx("td", { className: "px-2 py-1 text-right", children: /* @__PURE__ */ s.jsx(
                  "button",
                  {
                    type: "button",
                    className: "text-xs px-2 py-1 rounded border border-gray-300 hover:bg-gray-100",
                    onClick: async () => {
                      const z = new Set(x);
                      if (z.has(I) ? z.delete(I) : z.add(I), N(z), !S.has(I))
                        try {
                          const V = await T(R.schema_name, R.key_value);
                          E((G) => new Map(G).set(I, V));
                        } catch {
                        }
                    },
                    children: $ ? "Hide Data" : "Show Data"
                  }
                ) })
              ] }, `${I}-actions`),
              $ && /* @__PURE__ */ s.jsx("tr", { children: /* @__PURE__ */ s.jsx("td", { colSpan: 6, className: "px-2 pb-3", children: /* @__PURE__ */ s.jsx("div", { className: "ml-2 bg-gray-50 border rounded", children: /* @__PURE__ */ s.jsx(FieldsTable, { fields: F || {} }) }) }) }, `${I}-details`)
            ] });
          }),
          f.length === 0 && /* @__PURE__ */ s.jsx("tr", { children: /* @__PURE__ */ s.jsx("td", { colSpan: 5, className: "px-2 py-3 text-center text-gray-500", children: "No results" }) })
        ] })
      ] }) })
    ] })
  ] });
}
const da = {
  InProgress: { color: "text-blue-700 bg-blue-50", icon: "⏳" },
  Completed: { color: "text-green-700 bg-green-50", icon: "✅" },
  Failed: { color: "text-red-700 bg-red-50", icon: "❌" },
  default: { color: "text-gray-700 bg-gray-50", icon: "❓" }
}, gi = (e) => new Date(e * 1e3).toLocaleString(), tu = (e, t) => {
  const r = (t || Math.floor(Date.now() / 1e3)) - e;
  return r < 60 ? `${r}s` : r < 3600 ? `${Math.floor(r / 60)}m ${r % 60}s` : `${Math.floor(r / 3600)}h ${Math.floor(r % 3600 / 60)}m`;
}, ru = (e, t) => {
  const r = e + t;
  return r === 0 ? "N/A" : `${Math.round(e / r * 100)}%`;
}, su = ({ backfill: e }) => {
  const t = da[e.status] || da.default;
  return /* @__PURE__ */ s.jsxs("div", { className: `p-3 rounded-lg border ${t.color}`, children: [
    /* @__PURE__ */ s.jsxs("div", { className: "flex justify-between items-start mb-2", children: [
      /* @__PURE__ */ s.jsxs("div", { className: "flex items-center gap-2", children: [
        /* @__PURE__ */ s.jsx("span", { className: "text-xl", children: t.icon }),
        /* @__PURE__ */ s.jsxs("div", { children: [
          /* @__PURE__ */ s.jsx("div", { className: "font-semibold", children: e.transform_id }),
          /* @__PURE__ */ s.jsxs("div", { className: "text-xs opacity-80", children: [
            "Source: ",
            e.schema_name
          ] })
        ] })
      ] }),
      /* @__PURE__ */ s.jsxs("div", { className: "text-xs text-right", children: [
        /* @__PURE__ */ s.jsxs("div", { children: [
          "Started: ",
          gi(e.start_time)
        ] }),
        /* @__PURE__ */ s.jsxs("div", { children: [
          "Duration: ",
          tu(e.start_time, e.end_time)
        ] })
      ] })
    ] }),
    /* @__PURE__ */ s.jsx(nu, { backfill: e }),
    e.status === "InProgress" && e.mutations_expected > 0 && /* @__PURE__ */ s.jsx(au, { backfill: e })
  ] });
}, nu = ({ backfill: e }) => {
  const { status: t } = e;
  return t === "InProgress" ? /* @__PURE__ */ s.jsxs("div", { className: "grid grid-cols-2 md:grid-cols-3 gap-2 text-sm mt-2", children: [
    /* @__PURE__ */ s.jsxs("div", { children: [
      /* @__PURE__ */ s.jsx("span", { className: "font-medium", children: "Mutations:" }),
      " ",
      e.mutations_completed,
      " / ",
      e.mutations_expected
    ] }),
    e.mutations_failed > 0 && /* @__PURE__ */ s.jsxs("div", { className: "text-red-600", children: [
      /* @__PURE__ */ s.jsx("span", { className: "font-medium", children: "Failed:" }),
      " ",
      e.mutations_failed
    ] })
  ] }) : t === "Completed" ? /* @__PURE__ */ s.jsxs("div", { className: "grid grid-cols-2 md:grid-cols-3 gap-2 text-sm mt-2", children: [
    /* @__PURE__ */ s.jsxs("div", { children: [
      /* @__PURE__ */ s.jsx("span", { className: "font-medium", children: "Mutations:" }),
      " ",
      e.mutations_completed
    ] }),
    /* @__PURE__ */ s.jsxs("div", { children: [
      /* @__PURE__ */ s.jsx("span", { className: "font-medium", children: "Records:" }),
      " ",
      e.records_produced
    ] }),
    /* @__PURE__ */ s.jsxs("div", { children: [
      /* @__PURE__ */ s.jsx("span", { className: "font-medium", children: "Completed:" }),
      " ",
      e.end_time && gi(e.end_time)
    ] })
  ] }) : t === "Failed" && e.error ? /* @__PURE__ */ s.jsx("div", { className: "grid grid-cols-2 md:grid-cols-3 gap-2 text-sm mt-2", children: /* @__PURE__ */ s.jsxs("div", { className: "col-span-2 md:col-span-3", children: [
    /* @__PURE__ */ s.jsx("span", { className: "font-medium", children: "Error:" }),
    " ",
    e.error
  ] }) }) : null;
}, au = ({ backfill: e }) => {
  const t = Math.round(e.mutations_completed / e.mutations_expected * 100);
  return /* @__PURE__ */ s.jsxs("div", { className: "mt-2", children: [
    /* @__PURE__ */ s.jsx("div", { className: "w-full bg-gray-200 rounded-full h-2", children: /* @__PURE__ */ s.jsx(
      "div",
      {
        className: "bg-blue-600 h-2 rounded-full transition-all duration-300",
        style: { width: `${t}%` }
      }
    ) }),
    /* @__PURE__ */ s.jsxs("div", { className: "text-xs text-right mt-1", children: [
      t,
      "% complete"
    ] })
  ] });
}, iu = () => {
  const [e, t] = D([]), [r, n] = D(null), [a, l] = D(!0), [d, c] = D(null), [f, m] = D(!1), h = H(async () => {
    try {
      const p = await Ae.getAllBackfills();
      if (!(p != null && p.success) || !p.data)
        throw new Error((p == null ? void 0 : p.error) || "Failed to fetch backfills - invalid response");
      t(p.data), c(null);
    } catch (p) {
      throw console.error("Failed to fetch backfills:", p), c(p.message || "Failed to load backfills"), p;
    }
  }, []), y = H(async () => {
    try {
      const p = await Ae.getBackfillStatistics();
      if (!(p != null && p.success) || !p.data)
        throw new Error((p == null ? void 0 : p.error) || "Failed to fetch backfill statistics - invalid response");
      n(p.data), c(null);
    } catch (p) {
      throw console.error("Failed to fetch backfill statistics:", p), c(p.message || "Failed to load statistics"), p;
    } finally {
      l(!1);
    }
  }, []);
  xe(() => {
    h(), y();
    const p = setInterval(() => {
      h(), y();
    }, 3e3);
    return () => clearInterval(p);
  }, [h, y]);
  const x = e.filter((p) => p.status === "InProgress"), N = e.filter((p) => p.status === "Completed"), S = e.filter((p) => p.status === "Failed"), E = f ? e : x;
  return a ? /* @__PURE__ */ s.jsx("div", { className: "bg-gray-50 p-4 rounded-lg", children: /* @__PURE__ */ s.jsxs("div", { className: "flex items-center", children: [
    /* @__PURE__ */ s.jsx("div", { className: "animate-spin rounded-full h-4 w-4 border-b-2 border-gray-600 mr-2" }),
    /* @__PURE__ */ s.jsx("span", { className: "text-gray-800", children: "Loading backfill information..." })
  ] }) }) : d ? /* @__PURE__ */ s.jsx("div", { className: "bg-red-50 p-4 rounded-lg", role: "alert", children: /* @__PURE__ */ s.jsxs("span", { className: "text-red-800", children: [
    "Error: ",
    d
  ] }) }) : /* @__PURE__ */ s.jsxs("div", { className: "space-y-4", children: [
    r && /* @__PURE__ */ s.jsxs("div", { className: "bg-gray-50 p-4 rounded-lg", children: [
      /* @__PURE__ */ s.jsx("h3", { className: "text-md font-medium text-gray-800 mb-3", children: "Backfill Statistics" }),
      /* @__PURE__ */ s.jsxs("div", { className: "grid grid-cols-2 md:grid-cols-4 gap-4 text-sm", children: [
        /* @__PURE__ */ s.jsxs("div", { children: [
          /* @__PURE__ */ s.jsx("div", { className: "text-gray-600", children: "Total Mutations" }),
          /* @__PURE__ */ s.jsx("div", { className: "text-lg font-semibold text-gray-900", children: r.total_mutations_completed })
        ] }),
        /* @__PURE__ */ s.jsxs("div", { children: [
          /* @__PURE__ */ s.jsx("div", { className: "text-gray-600", children: "Success Rate" }),
          /* @__PURE__ */ s.jsx("div", { className: "text-lg font-semibold text-green-700", children: ru(r.total_mutations_completed, r.total_mutations_failed) })
        ] }),
        /* @__PURE__ */ s.jsxs("div", { children: [
          /* @__PURE__ */ s.jsx("div", { className: "text-gray-600", children: "Backfills" }),
          /* @__PURE__ */ s.jsx("div", { className: "text-lg font-semibold text-blue-700", children: r.total_backfills })
        ] }),
        /* @__PURE__ */ s.jsxs("div", { children: [
          /* @__PURE__ */ s.jsx("div", { className: "text-gray-600", children: "Failures" }),
          /* @__PURE__ */ s.jsx("div", { className: "text-lg font-semibold text-red-700", children: r.total_mutations_failed })
        ] })
      ] })
    ] }),
    /* @__PURE__ */ s.jsxs("div", { className: "bg-gray-50 p-4 rounded-lg", children: [
      /* @__PURE__ */ s.jsxs("div", { className: "flex justify-between items-center mb-3", children: [
        /* @__PURE__ */ s.jsx("h3", { className: "text-md font-medium text-gray-800", children: "Backfills" }),
        /* @__PURE__ */ s.jsxs("div", { className: "flex items-center gap-4", children: [
          /* @__PURE__ */ s.jsxs("div", { className: "text-sm text-gray-600", children: [
            "Active: ",
            x.length,
            " | Completed: ",
            N.length,
            " | Failed: ",
            S.length
          ] }),
          /* @__PURE__ */ s.jsx(
            "button",
            {
              onClick: () => m(!f),
              className: "px-3 py-1 text-sm bg-gray-200 text-gray-800 rounded hover:bg-gray-300",
              children: f ? "Show Active Only" : "Show All"
            }
          )
        ] })
      ] }),
      E.length === 0 ? /* @__PURE__ */ s.jsx("div", { className: "text-gray-600 text-sm", children: f ? "No backfills recorded" : "No active backfills" }) : /* @__PURE__ */ s.jsx("div", { className: "space-y-3", children: E.map((p) => /* @__PURE__ */ s.jsx(
        su,
        {
          backfill: p
        },
        `${p.transform_id}-${p.start_time}`
      )) })
    ] })
  ] });
}, ou = {
  queue: [],
  length: 0,
  isEmpty: !0
}, cu = (e = {}) => {
  const t = Array.isArray(e.queue) ? e.queue : [], r = typeof e.length == "number" ? e.length : t.length, n = typeof e.isEmpty == "boolean" ? e.isEmpty : t.length === 0;
  return { queue: t, length: r, isEmpty: n };
}, lu = ({ onResult: e }) => {
  const [t, r] = D(ou), [n, a] = D({}), [l, d] = D({}), [c, f] = D(!1), [m, h] = D(null), [y, x] = D([]), N = H(async () => {
    f(!0), h(null);
    try {
      const p = await Ae.getTransforms();
      if (p != null && p.success && p.data) {
        const v = p.data, w = v && typeof v == "object" && !Array.isArray(v) ? Object.entries(v).map(([A, _]) => ({
          transform_id: A,
          ..._
        })) : Array.isArray(v) ? v : [];
        x(w);
      } else {
        const v = (p == null ? void 0 : p.error) || "Failed to load transforms";
        h(v), x([]);
      }
    } catch (p) {
      console.error("Failed to fetch transforms:", p), h(p.message || "Failed to load transforms"), x([]);
    } finally {
      f(!1);
    }
  }, []), S = H(async () => {
    try {
      const p = await Ae.getQueue();
      p != null && p.success && p.data && r(cu(p.data));
    } catch (p) {
      console.error("Failed to fetch transform queue info:", p);
    }
  }, []);
  xe(() => {
    N(), S();
    const p = setInterval(S, 5e3);
    return () => clearInterval(p);
  }, [N, S]);
  const E = H(async (p, v) => {
    var A;
    const w = v ? `${p}.${v}` : p;
    a((_) => ({ ..._, [w]: !0 })), d((_) => ({ ..._, [w]: null }));
    try {
      const _ = await Ae.addToQueue(w);
      if (!(_ != null && _.success)) {
        const T = ((A = _ == null ? void 0 : _.data) == null ? void 0 : A.message) || (_ == null ? void 0 : _.error) || "Failed to add transform to queue";
        throw new Error(T);
      }
      typeof e == "function" && e({ success: !0, transformId: w }), await S();
    } catch (_) {
      console.error("Failed to add transform to queue:", _), d((T) => ({ ...T, [w]: _.message || "Failed to add transform to queue" }));
    } finally {
      a((_) => ({ ..._, [w]: !1 }));
    }
  }, [S, e]);
  return /* @__PURE__ */ s.jsxs("div", { className: "space-y-4", children: [
    /* @__PURE__ */ s.jsxs("div", { className: "flex justify-between items-center", children: [
      /* @__PURE__ */ s.jsx("h2", { className: "text-xl font-semibold text-gray-800", children: "Transforms" }),
      /* @__PURE__ */ s.jsxs("div", { className: "text-sm text-gray-600", children: [
        "Queue Status: ",
        t.isEmpty ? "Empty" : `${t.length} transform(s) queued`
      ] })
    ] }),
    /* @__PURE__ */ s.jsx(iu, {}),
    !t.isEmpty && /* @__PURE__ */ s.jsxs("div", { className: "bg-blue-50 p-4 rounded-lg", "data-testid": "transform-queue", children: [
      /* @__PURE__ */ s.jsx("h3", { className: "text-md font-medium text-blue-800 mb-2", children: "Transform Queue" }),
      /* @__PURE__ */ s.jsx("ul", { className: "list-disc list-inside space-y-1", children: t.queue.map((p, v) => /* @__PURE__ */ s.jsx("li", { className: "text-blue-700", children: p }, `${p}-${v}`)) })
    ] }),
    c && /* @__PURE__ */ s.jsx("div", { className: "bg-blue-50 p-4 rounded-lg", role: "status", children: /* @__PURE__ */ s.jsxs("div", { className: "flex items-center", children: [
      /* @__PURE__ */ s.jsx("div", { className: "animate-spin rounded-full h-4 w-4 border-b-2 border-blue-600 mr-2" }),
      /* @__PURE__ */ s.jsx("span", { className: "text-blue-800", children: "Loading transforms..." })
    ] }) }),
    m && /* @__PURE__ */ s.jsx("div", { className: "bg-red-50 p-4 rounded-lg", role: "alert", children: /* @__PURE__ */ s.jsxs("div", { className: "flex items-center", children: [
      /* @__PURE__ */ s.jsxs("span", { className: "text-red-800", children: [
        "Error loading transforms: ",
        m
      ] }),
      /* @__PURE__ */ s.jsx(
        "button",
        {
          onClick: N,
          className: "ml-4 px-3 py-1 text-sm bg-red-500 text-white rounded hover:bg-red-600",
          children: "Retry"
        }
      )
    ] }) }),
    !c && !m && y.length > 0 && /* @__PURE__ */ s.jsx("div", { className: "space-y-4", children: y.map((p, v) => {
      var V;
      const w = p.transform_id || `transform-${v}`, A = n[w], _ = l[w], T = p.name || ((V = p.transform_id) == null ? void 0 : V.split(".")[0]) || "Unknown", M = p.schema_type;
      let R = "Single", k = "bg-gray-100 text-gray-800";
      M != null && M.Range ? (R = "Range", k = "bg-blue-100 text-blue-800") : M != null && M.HashRange && (R = "HashRange", k = "bg-purple-100 text-purple-800");
      const I = p.key, $ = p.transform_fields || {}, F = Object.keys($).length, z = Object.keys($);
      return /* @__PURE__ */ s.jsxs("div", { className: "bg-white p-4 rounded-lg shadow border-l-4 border-blue-500", children: [
        /* @__PURE__ */ s.jsx("div", { className: "flex justify-between items-start mb-3", children: /* @__PURE__ */ s.jsxs("div", { className: "flex-1", children: [
          /* @__PURE__ */ s.jsx("h3", { className: "text-lg font-semibold text-gray-900", children: T }),
          /* @__PURE__ */ s.jsxs("div", { className: "flex gap-2 mt-2 flex-wrap", children: [
            /* @__PURE__ */ s.jsx("span", { className: `inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium ${k}`, children: R }),
            F > 0 && /* @__PURE__ */ s.jsxs("span", { className: "inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-green-100 text-green-800", children: [
              F,
              " field",
              F !== 1 ? "s" : ""
            ] })
          ] }),
          z.length > 0 && /* @__PURE__ */ s.jsxs("div", { className: "mt-2 text-sm text-gray-600", children: [
            /* @__PURE__ */ s.jsx("span", { className: "font-medium", children: "Fields:" }),
            " ",
            z.join(", ")
          ] })
        ] }) }),
        /* @__PURE__ */ s.jsxs("div", { className: "mt-3 space-y-3", children: [
          I && /* @__PURE__ */ s.jsxs("div", { className: "bg-blue-50 rounded p-3", children: [
            /* @__PURE__ */ s.jsx("div", { className: "text-sm font-medium text-blue-900 mb-1", children: "Key Configuration:" }),
            /* @__PURE__ */ s.jsxs("div", { className: "text-sm text-blue-800 space-y-1", children: [
              I.hash_field && /* @__PURE__ */ s.jsxs("div", { children: [
                /* @__PURE__ */ s.jsx("span", { className: "font-medium", children: "Hash Key:" }),
                " ",
                I.hash_field
              ] }),
              I.range_field && /* @__PURE__ */ s.jsxs("div", { children: [
                /* @__PURE__ */ s.jsx("span", { className: "font-medium", children: "Range Key:" }),
                " ",
                I.range_field
              ] }),
              !I.hash_field && !I.range_field && I.key_field && /* @__PURE__ */ s.jsxs("div", { children: [
                /* @__PURE__ */ s.jsx("span", { className: "font-medium", children: "Key:" }),
                " ",
                I.key_field
              ] })
            ] })
          ] }),
          F > 0 && /* @__PURE__ */ s.jsxs("div", { children: [
            /* @__PURE__ */ s.jsx("div", { className: "text-sm font-medium text-gray-700 mb-2", children: "Transform Fields:" }),
            /* @__PURE__ */ s.jsx("div", { className: "bg-gray-50 rounded p-3 space-y-2", children: Object.entries($).map(([G, L]) => /* @__PURE__ */ s.jsxs("div", { className: "border-l-2 border-gray-300 pl-3", children: [
              /* @__PURE__ */ s.jsx("div", { className: "font-medium text-gray-900 text-sm", children: G }),
              /* @__PURE__ */ s.jsx("div", { className: "text-gray-600 font-mono text-xs mt-1 break-all", children: L })
            ] }, G)) })
          ] })
        ] }),
        /* @__PURE__ */ s.jsxs("div", { className: "mt-4 flex items-center gap-3", children: [
          /* @__PURE__ */ s.jsx(
            "button",
            {
              onClick: () => E(w, null),
              disabled: A,
              className: `px-4 py-2 text-sm font-medium rounded-md text-white ${A ? "bg-blue-300 cursor-not-allowed" : "bg-blue-600 hover:bg-blue-700"}`,
              children: A ? "Adding..." : "Add to Queue"
            }
          ),
          _ && /* @__PURE__ */ s.jsxs("span", { className: "text-sm text-red-600", children: [
            "Error: ",
            _
          ] })
        ] })
      ] }, w);
    }) }),
    !c && !m && y.length === 0 && /* @__PURE__ */ s.jsxs("div", { className: "bg-gray-50 p-4 rounded-lg", children: [
      /* @__PURE__ */ s.jsx("p", { className: "text-gray-600", children: "No transforms registered" }),
      /* @__PURE__ */ s.jsx("p", { className: "text-sm text-gray-500 mt-1", children: "Register a transform in a schema to view it here and add it to the processing queue." })
    ] })
  ] });
};
function du({
  title: e,
  titleId: t,
  ...r
}, n) {
  return /* @__PURE__ */ Y.createElement("svg", Object.assign({
    xmlns: "http://www.w3.org/2000/svg",
    fill: "none",
    viewBox: "0 0 24 24",
    strokeWidth: 1.5,
    stroke: "currentColor",
    "aria-hidden": "true",
    "data-slot": "icon",
    ref: n,
    "aria-labelledby": t
  }, r), e ? /* @__PURE__ */ Y.createElement("title", {
    id: t
  }, e) : null, /* @__PURE__ */ Y.createElement("path", {
    strokeLinecap: "round",
    strokeLinejoin: "round",
    d: "m4.5 12.75 6 6 9-13.5"
  }));
}
const js = /* @__PURE__ */ Y.forwardRef(du);
function uu({
  title: e,
  titleId: t,
  ...r
}, n) {
  return /* @__PURE__ */ Y.createElement("svg", Object.assign({
    xmlns: "http://www.w3.org/2000/svg",
    fill: "none",
    viewBox: "0 0 24 24",
    strokeWidth: 1.5,
    stroke: "currentColor",
    "aria-hidden": "true",
    "data-slot": "icon",
    ref: n,
    "aria-labelledby": t
  }, r), e ? /* @__PURE__ */ Y.createElement("title", {
    id: t
  }, e) : null, /* @__PURE__ */ Y.createElement("path", {
    strokeLinecap: "round",
    strokeLinejoin: "round",
    d: "M15.666 3.888A2.25 2.25 0 0 0 13.5 2.25h-3c-1.03 0-1.9.693-2.166 1.638m7.332 0c.055.194.084.4.084.612v0a.75.75 0 0 1-.75.75H9a.75.75 0 0 1-.75-.75v0c0-.212.03-.418.084-.612m7.332 0c.646.049 1.288.11 1.927.184 1.1.128 1.907 1.077 1.907 2.185V19.5a2.25 2.25 0 0 1-2.25 2.25H6.75A2.25 2.25 0 0 1 4.5 19.5V6.257c0-1.108.806-2.057 1.907-2.185a48.208 48.208 0 0 1 1.927-.184"
  }));
}
const ua = /* @__PURE__ */ Y.forwardRef(uu);
function fu({
  title: e,
  titleId: t,
  ...r
}, n) {
  return /* @__PURE__ */ Y.createElement("svg", Object.assign({
    xmlns: "http://www.w3.org/2000/svg",
    fill: "none",
    viewBox: "0 0 24 24",
    strokeWidth: 1.5,
    stroke: "currentColor",
    "aria-hidden": "true",
    "data-slot": "icon",
    ref: n,
    "aria-labelledby": t
  }, r), e ? /* @__PURE__ */ Y.createElement("title", {
    id: t
  }, e) : null, /* @__PURE__ */ Y.createElement("path", {
    strokeLinecap: "round",
    strokeLinejoin: "round",
    d: "M12 9v3.75m-9.303 3.376c-.866 1.5.217 3.374 1.948 3.374h14.71c1.73 0 2.813-1.874 1.948-3.374L13.949 3.378c-.866-1.5-3.032-1.5-3.898 0L2.697 16.126ZM12 15.75h.007v.008H12v-.008Z"
  }));
}
const fa = /* @__PURE__ */ Y.forwardRef(fu);
function hu({
  title: e,
  titleId: t,
  ...r
}, n) {
  return /* @__PURE__ */ Y.createElement("svg", Object.assign({
    xmlns: "http://www.w3.org/2000/svg",
    fill: "none",
    viewBox: "0 0 24 24",
    strokeWidth: 1.5,
    stroke: "currentColor",
    "aria-hidden": "true",
    "data-slot": "icon",
    ref: n,
    "aria-labelledby": t
  }, r), e ? /* @__PURE__ */ Y.createElement("title", {
    id: t
  }, e) : null, /* @__PURE__ */ Y.createElement("path", {
    strokeLinecap: "round",
    strokeLinejoin: "round",
    d: "M15.75 5.25a3 3 0 0 1 3 3m3 0a6 6 0 0 1-7.029 5.912c-.563-.097-1.159.026-1.563.43L10.5 17.25H8.25v2.25H6v2.25H2.25v-2.818c0-.597.237-1.17.659-1.591l6.499-6.499c.404-.404.527-1 .43-1.563A6 6 0 1 1 21.75 8.25Z"
  }));
}
const Ss = /* @__PURE__ */ Y.forwardRef(hu);
function mu({
  title: e,
  titleId: t,
  ...r
}, n) {
  return /* @__PURE__ */ Y.createElement("svg", Object.assign({
    xmlns: "http://www.w3.org/2000/svg",
    fill: "none",
    viewBox: "0 0 24 24",
    strokeWidth: 1.5,
    stroke: "currentColor",
    "aria-hidden": "true",
    "data-slot": "icon",
    ref: n,
    "aria-labelledby": t
  }, r), e ? /* @__PURE__ */ Y.createElement("title", {
    id: t
  }, e) : null, /* @__PURE__ */ Y.createElement("path", {
    strokeLinecap: "round",
    strokeLinejoin: "round",
    d: "M9 12.75 11.25 15 15 9.75m-3-7.036A11.959 11.959 0 0 1 3.598 6 11.99 11.99 0 0 0 3 9.749c0 5.592 3.824 10.29 9 11.623 5.176-1.332 9-6.03 9-11.622 0-1.31-.21-2.571-.598-3.751h-.152c-3.196 0-6.1-1.248-8.25-3.285Z"
  }));
}
const pu = /* @__PURE__ */ Y.forwardRef(mu);
function gu({ onResult: e }) {
  const t = wr(), r = he((k) => k.auth), { isAuthenticated: n, systemPublicKey: a, systemKeyId: l, privateKey: d, isLoading: c, error: f } = r, m = d ? ol(d) : null, [h, y] = D(null), [x, N] = D(""), [S, E] = D(!1), [p, v] = D(null), [w, A] = D(!1), _ = async (k, I) => {
    try {
      await navigator.clipboard.writeText(k), y(I), setTimeout(() => y(null), 2e3);
    } catch ($) {
      console.error("Failed to copy:", $);
    }
  }, T = async () => {
    if (!x.trim()) {
      v({ valid: !1, error: "Please enter a private key" });
      return;
    }
    E(!0);
    try {
      const I = (await t(Mr(x.trim())).unwrap()).isAuthenticated;
      v({
        valid: I,
        error: I ? null : "Private key does not match the system public key"
      }), I && console.log("Private key validation successful");
    } catch (k) {
      v({
        valid: !1,
        error: `Validation failed: ${k.message}`
      });
    } finally {
      E(!1);
    }
  }, M = () => {
    N(""), v(null), A(!1);
  }, R = () => {
    M(), t(ll());
  };
  return /* @__PURE__ */ s.jsxs("div", { className: "p-4 bg-white rounded-lg shadow", children: [
    /* @__PURE__ */ s.jsx("h2", { className: "text-xl font-semibold mb-4", children: "Key Management" }),
    /* @__PURE__ */ s.jsx("div", { className: "bg-blue-50 border border-blue-200 rounded-md p-4 mb-6", children: /* @__PURE__ */ s.jsxs("div", { className: "flex items-start", children: [
      /* @__PURE__ */ s.jsx(pu, { className: "h-5 w-5 text-blue-400 mr-2 flex-shrink-0 mt-0.5" }),
      /* @__PURE__ */ s.jsxs("div", { className: "text-sm text-blue-700 flex-1", children: [
        /* @__PURE__ */ s.jsx("p", { className: "font-medium", children: "Current System Public Key:" }),
        c ? /* @__PURE__ */ s.jsx("p", { className: "text-blue-600", children: "Loading..." }) : a ? /* @__PURE__ */ s.jsxs("div", { className: "mt-2", children: [
          /* @__PURE__ */ s.jsxs("div", { className: "flex", children: [
            /* @__PURE__ */ s.jsx(
              "input",
              {
                type: "text",
                value: a && a !== "null" ? a : "",
                readOnly: !0,
                className: "flex-1 px-2 py-1 border border-blue-300 rounded-l-md bg-blue-50 text-xs font-mono"
              }
            ),
            /* @__PURE__ */ s.jsx(
              "button",
              {
                onClick: () => _(a, "system"),
                className: "px-2 py-1 border border-l-0 border-blue-300 rounded-r-md bg-white hover:bg-blue-50 focus:outline-none focus:ring-2 focus:ring-blue-500",
                children: h === "system" ? /* @__PURE__ */ s.jsx(js, { className: "h-3 w-3 text-green-600" }) : /* @__PURE__ */ s.jsx(ua, { className: "h-3 w-3 text-blue-500" })
              }
            )
          ] }),
          l && /* @__PURE__ */ s.jsxs("p", { className: "text-xs text-blue-600 mt-1", children: [
            "Key ID: ",
            l
          ] }),
          n && /* @__PURE__ */ s.jsx("p", { className: "text-xs text-green-600 mt-1", children: "🔓 Authenticated - Private key loaded!" })
        ] }) : /* @__PURE__ */ s.jsx("p", { className: "text-blue-600 mt-1", children: "No system public key available." })
      ] })
    ] }) }),
    n && m && /* @__PURE__ */ s.jsx("div", { className: "bg-green-50 border border-green-200 rounded-md p-4 mb-6", children: /* @__PURE__ */ s.jsxs("div", { className: "flex items-start", children: [
      /* @__PURE__ */ s.jsx(Ss, { className: "h-5 w-5 text-green-400 mr-2 flex-shrink-0 mt-0.5" }),
      /* @__PURE__ */ s.jsxs("div", { className: "text-sm text-green-700 flex-1", children: [
        /* @__PURE__ */ s.jsx("p", { className: "font-medium", children: "Current Private Key (Auto-loaded from Node)" }),
        /* @__PURE__ */ s.jsx("p", { className: "mt-1", children: "Your private key has been automatically loaded from the backend node." }),
        /* @__PURE__ */ s.jsxs("div", { className: "mt-3", children: [
          /* @__PURE__ */ s.jsxs("div", { className: "flex", children: [
            /* @__PURE__ */ s.jsx(
              "textarea",
              {
                value: m,
                readOnly: !0,
                className: "flex-1 px-3 py-2 border border-green-300 rounded-l-md bg-green-50 text-xs font-mono resize-none",
                rows: 3,
                placeholder: "Private key will appear here..."
              }
            ),
            /* @__PURE__ */ s.jsx(
              "button",
              {
                onClick: () => _(m, "private"),
                className: "px-3 py-2 border border-l-0 border-green-300 rounded-r-md bg-white hover:bg-green-50 focus:outline-none focus:ring-2 focus:ring-green-500",
                title: "Copy private key",
                children: h === "private" ? /* @__PURE__ */ s.jsx(js, { className: "h-3 w-3 text-green-600" }) : /* @__PURE__ */ s.jsx(ua, { className: "h-3 w-3 text-green-500" })
              }
            )
          ] }),
          /* @__PURE__ */ s.jsx("p", { className: "text-xs text-green-600 mt-1", children: "🔓 Authenticated - Private key loaded from node!" })
        ] })
      ] })
    ] }) }),
    a && !n && !m && /* @__PURE__ */ s.jsx("div", { className: "bg-yellow-50 border border-yellow-200 rounded-md p-4 mb-6", children: /* @__PURE__ */ s.jsxs("div", { className: "flex items-start", children: [
      /* @__PURE__ */ s.jsx(Ss, { className: "h-5 w-5 text-yellow-400 mr-2 flex-shrink-0 mt-0.5" }),
      /* @__PURE__ */ s.jsxs("div", { className: "text-sm text-yellow-700 flex-1", children: [
        /* @__PURE__ */ s.jsx("p", { className: "font-medium", children: "Import Private Key" }),
        /* @__PURE__ */ s.jsx("p", { className: "mt-1", children: "You have a registered public key but no local private key. Enter your private key to restore access." }),
        w ? /* @__PURE__ */ s.jsxs("div", { className: "mt-3 space-y-3", children: [
          /* @__PURE__ */ s.jsxs("div", { children: [
            /* @__PURE__ */ s.jsx("label", { className: "block text-xs font-medium text-yellow-700 mb-1", children: "Private Key (Base64)" }),
            /* @__PURE__ */ s.jsx(
              "textarea",
              {
                value: x,
                onChange: (k) => N(k.target.value),
                placeholder: "Enter your private key here...",
                className: "w-full px-3 py-2 border border-yellow-300 rounded-md focus:outline-none focus:ring-2 focus:ring-yellow-500 text-xs font-mono",
                rows: 3
              }
            )
          ] }),
          p && /* @__PURE__ */ s.jsx("div", { className: `p-2 rounded-md text-xs ${p.valid ? "bg-green-50 border border-green-200 text-green-700" : "bg-red-50 border border-red-200 text-red-700"}`, children: p.valid ? /* @__PURE__ */ s.jsxs("div", { className: "flex items-center", children: [
            /* @__PURE__ */ s.jsx(js, { className: "h-4 w-4 text-green-600 mr-1" }),
            /* @__PURE__ */ s.jsx("span", { children: "Private key matches system public key!" })
          ] }) : /* @__PURE__ */ s.jsxs("div", { className: "flex items-center", children: [
            /* @__PURE__ */ s.jsx(fa, { className: "h-4 w-4 text-red-600 mr-1" }),
            /* @__PURE__ */ s.jsx("span", { children: p.error })
          ] }) }),
          /* @__PURE__ */ s.jsxs("div", { className: "flex gap-2", children: [
            /* @__PURE__ */ s.jsx(
              "button",
              {
                onClick: T,
                disabled: S || !x.trim(),
                className: "inline-flex items-center px-3 py-2 border border-transparent text-xs font-medium rounded-md shadow-sm text-white bg-green-600 hover:bg-green-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-green-500 disabled:opacity-50",
                children: S ? "Validating..." : "Validate & Import"
              }
            ),
            /* @__PURE__ */ s.jsx(
              "button",
              {
                onClick: R,
                className: "inline-flex items-center px-3 py-2 border border-gray-300 text-xs font-medium rounded-md shadow-sm text-gray-700 bg-white hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-yellow-500",
                children: "Cancel"
              }
            )
          ] }),
          /* @__PURE__ */ s.jsx("div", { className: "bg-red-50 border border-red-200 rounded-md p-2", children: /* @__PURE__ */ s.jsxs("div", { className: "flex", children: [
            /* @__PURE__ */ s.jsx(fa, { className: "h-4 w-4 text-red-400 mr-1 flex-shrink-0" }),
            /* @__PURE__ */ s.jsxs("div", { className: "text-xs text-red-700", children: [
              /* @__PURE__ */ s.jsx("p", { className: "font-medium", children: "Security Warning:" }),
              /* @__PURE__ */ s.jsx("p", { children: "Only enter your private key on trusted devices. Never share or store private keys in plain text." })
            ] })
          ] }) })
        ] }) : /* @__PURE__ */ s.jsxs(
          "button",
          {
            onClick: () => A(!0),
            className: "mt-3 inline-flex items-center px-3 py-2 border border-transparent text-sm font-medium rounded-md shadow-sm text-white bg-yellow-600 hover:bg-yellow-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-yellow-500",
            children: [
              /* @__PURE__ */ s.jsx(Ss, { className: "h-4 w-4 mr-1" }),
              "Import Private Key"
            ]
          }
        )
      ] })
    ] }) })
  ] });
}
function Gu({ isOpen: e, onClose: t }) {
  const [r, n] = D("ai"), [a, l] = D("OpenRouter"), [d, c] = D(""), [f, m] = D("anthropic/claude-3.5-sonnet"), [h, y] = D("https://openrouter.ai/api/v1"), [x, N] = D("llama3"), [S, E] = D("http://localhost:11434"), [p, v] = D(null), [w, A] = D(!1), { environment: _, setEnvironment: T } = ql(), [M, R] = D(_.id), [k, I] = D({}), [$, F] = D({}), [z, V] = D("local"), [G, L] = D("data"), [J, Q] = D("DataFoldStorage"), [ge, Me] = D("us-west-2"), [ze, ne] = D(""), [ce, mt] = D(""), [pt, At] = D("us-east-1"), [nt, at] = D("folddb"), [Ce, Ye] = D("/tmp/folddb-data");
  xe(() => {
    e && (it(), Qe(), R(_.id), r === "schema-service" && gt(_.id));
  }, [e, _.id, r]);
  const it = async () => {
    try {
      const B = await jt.getConfig();
      B.success && (c(B.data.openrouter.api_key || ""), m(B.data.openrouter.model || "anthropic/claude-3.5-sonnet"), y(B.data.openrouter.base_url || "https://openrouter.ai/api/v1"), N(B.data.ollama.model || "llama3"), E(B.data.ollama.base_url || "http://localhost:11434"), l(B.data.provider || "OpenRouter"));
    } catch (B) {
      console.error("Failed to load AI config:", B);
    }
  }, er = async () => {
    try {
      const B = {
        provider: a,
        openrouter: {
          api_key: d,
          model: f,
          base_url: h
        },
        ollama: {
          model: x,
          base_url: S
        }
      };
      (await jt.saveConfig(B)).success ? (v({ success: !0, message: "Configuration saved successfully" }), setTimeout(() => {
        v(null), t();
      }, 1500)) : v({ success: !1, message: "Failed to save configuration" });
    } catch (B) {
      v({ success: !1, message: B.message || "Failed to save configuration" });
    }
    setTimeout(() => v(null), 3e3);
  }, gt = async (B) => {
    const X = Object.values(zt).find((be) => be.id === B);
    if (X) {
      F((be) => ({ ...be, [B]: !0 }));
      try {
        const be = await zl(X.baseUrl);
        I((Je) => ({
          ...Je,
          [B]: be
        }));
      } catch (be) {
        I((Je) => ({
          ...Je,
          [B]: { success: !1, error: be.message }
        }));
      } finally {
        F((be) => ({ ...be, [B]: !1 }));
      }
    }
  }, Qe = async () => {
    try {
      const B = await mc();
      if (B.success && B.data) {
        const X = B.data;
        V(X.type), X.type === "local" ? L(X.path || "data") : X.type === "dynamodb" ? (Q(X.table_name || "DataFoldStorage"), Me(X.region || "us-west-2"), ne(X.user_id || "")) : X.type === "s3" && (mt(X.bucket || ""), At(X.region || "us-east-1"), at(X.prefix || "folddb"), Ye(X.local_path || "/tmp/folddb-data"));
      }
    } catch (B) {
      console.error("Failed to load database config:", B);
    }
  }, ot = async () => {
    try {
      let B;
      if (z === "local")
        B = {
          type: "local",
          path: G
        };
      else if (z === "dynamodb") {
        if (!J || !ge) {
          v({ success: !1, message: "Table name and region are required for DynamoDB" }), setTimeout(() => v(null), 3e3);
          return;
        }
        B = {
          type: "dynamodb",
          table_name: J,
          region: ge,
          user_id: ze || void 0
        };
      } else if (z === "s3") {
        if (!ce || !pt) {
          v({ success: !1, message: "Bucket and region are required for S3" }), setTimeout(() => v(null), 3e3);
          return;
        }
        B = {
          type: "s3",
          bucket: ce,
          region: pt,
          prefix: nt || "folddb",
          local_path: Ce || "/tmp/folddb-data"
        };
      }
      const X = await pc(B);
      X.success ? (v({
        success: !0,
        message: X.data.requires_restart ? "Database configuration saved. Please restart the server for changes to take effect." : X.data.message || "Database configuration saved and restarted successfully"
      }), setTimeout(() => {
        v(null), X.data.requires_restart || t();
      }, 3e3)) : v({ success: !1, message: X.error || "Failed to save database configuration" });
    } catch (B) {
      v({ success: !1, message: B.message || "Failed to save database configuration" });
    }
    setTimeout(() => v(null), 5e3);
  }, yt = () => {
    T(M), v({ success: !0, message: "Schema service environment updated successfully" }), setTimeout(() => {
      v(null), t();
    }, 1500);
  }, ct = (B) => {
    const X = k[B];
    return $[B] ? /* @__PURE__ */ s.jsxs("span", { className: "inline-flex items-center text-xs bg-gray-100 text-gray-700 px-2 py-1 rounded", children: [
      /* @__PURE__ */ s.jsxs("svg", { className: "animate-spin h-3 w-3 mr-1", viewBox: "0 0 24 24", children: [
        /* @__PURE__ */ s.jsx("circle", { className: "opacity-25", cx: "12", cy: "12", r: "10", stroke: "currentColor", strokeWidth: "4", fill: "none" }),
        /* @__PURE__ */ s.jsx("path", { className: "opacity-75", fill: "currentColor", d: "M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" })
      ] }),
      "Checking..."
    ] }) : X ? X.success ? /* @__PURE__ */ s.jsxs("span", { className: "inline-flex items-center text-xs bg-green-100 text-green-700 px-2 py-1 rounded", children: [
      "✓ Online ",
      X.responseTime && `(${X.responseTime}ms)`
    ] }) : /* @__PURE__ */ s.jsx("span", { className: "inline-flex items-center text-xs bg-red-100 text-red-700 px-2 py-1 rounded", title: X.error, children: "✗ Offline" }) : /* @__PURE__ */ s.jsx(
      "button",
      {
        onClick: (Je) => {
          Je.stopPropagation(), gt(B);
        },
        className: "text-xs text-blue-600 hover:text-blue-700 underline",
        children: "Test Connection"
      }
    );
  };
  return e ? /* @__PURE__ */ s.jsx("div", { className: "fixed inset-0 z-50 overflow-y-auto", children: /* @__PURE__ */ s.jsxs("div", { className: "flex items-center justify-center min-h-screen px-4 pt-4 pb-20 text-center sm:block sm:p-0", children: [
    /* @__PURE__ */ s.jsx(
      "div",
      {
        className: "fixed inset-0 transition-opacity bg-gray-500 bg-opacity-75",
        onClick: t
      }
    ),
    /* @__PURE__ */ s.jsxs("div", { className: "inline-block align-bottom bg-white rounded-lg text-left overflow-hidden shadow-xl transform transition-all sm:my-8 sm:align-middle sm:max-w-4xl sm:w-full", children: [
      /* @__PURE__ */ s.jsxs("div", { className: "bg-white", children: [
        /* @__PURE__ */ s.jsxs("div", { className: "flex items-center justify-between px-6 pt-5 pb-4 border-b border-gray-200", children: [
          /* @__PURE__ */ s.jsx("h3", { className: "text-lg font-medium text-gray-900", children: "Settings" }),
          /* @__PURE__ */ s.jsx(
            "button",
            {
              onClick: t,
              className: "text-gray-400 hover:text-gray-600 transition-colors",
              children: /* @__PURE__ */ s.jsx("svg", { className: "w-6 h-6", fill: "none", stroke: "currentColor", viewBox: "0 0 24 24", children: /* @__PURE__ */ s.jsx("path", { strokeLinecap: "round", strokeLinejoin: "round", strokeWidth: 2, d: "M6 18L18 6M6 6l12 12" }) })
            }
          )
        ] }),
        /* @__PURE__ */ s.jsx("div", { className: "border-b border-gray-200", children: /* @__PURE__ */ s.jsxs("nav", { className: "flex px-6", children: [
          /* @__PURE__ */ s.jsx(
            "button",
            {
              onClick: () => n("ai"),
              className: `py-3 px-4 text-sm font-medium border-b-2 transition-colors ${r === "ai" ? "border-blue-500 text-blue-600" : "border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300"}`,
              children: "AI Configuration"
            }
          ),
          /* @__PURE__ */ s.jsx(
            "button",
            {
              onClick: () => n("transforms"),
              className: `py-3 px-4 text-sm font-medium border-b-2 transition-colors ${r === "transforms" ? "border-blue-500 text-blue-600" : "border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300"}`,
              children: "Transforms"
            }
          ),
          /* @__PURE__ */ s.jsx(
            "button",
            {
              onClick: () => n("keys"),
              className: `py-3 px-4 text-sm font-medium border-b-2 transition-colors ${r === "keys" ? "border-blue-500 text-blue-600" : "border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300"}`,
              children: "Key Management"
            }
          ),
          /* @__PURE__ */ s.jsx(
            "button",
            {
              onClick: () => n("schema-service"),
              className: `py-3 px-4 text-sm font-medium border-b-2 transition-colors ${r === "schema-service" ? "border-blue-500 text-blue-600" : "border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300"}`,
              children: "Schema Service"
            }
          ),
          /* @__PURE__ */ s.jsx(
            "button",
            {
              onClick: () => n("database"),
              className: `py-3 px-4 text-sm font-medium border-b-2 transition-colors ${r === "database" ? "border-blue-500 text-blue-600" : "border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300"}`,
              children: "Database"
            }
          )
        ] }) }),
        /* @__PURE__ */ s.jsxs("div", { className: "px-6 py-4 max-h-[70vh] overflow-y-auto", children: [
          r === "ai" && /* @__PURE__ */ s.jsxs("div", { className: "space-y-4", children: [
            /* @__PURE__ */ s.jsxs("div", { className: "grid grid-cols-1 md:grid-cols-2 gap-4", children: [
              /* @__PURE__ */ s.jsxs("div", { children: [
                /* @__PURE__ */ s.jsx("label", { className: "block text-sm font-medium text-gray-700 mb-1", children: "Provider" }),
                /* @__PURE__ */ s.jsxs(
                  "select",
                  {
                    value: a,
                    onChange: (B) => l(B.target.value),
                    className: "w-full p-2 border border-gray-300 rounded text-sm",
                    children: [
                      /* @__PURE__ */ s.jsx("option", { value: "OpenRouter", children: "OpenRouter" }),
                      /* @__PURE__ */ s.jsx("option", { value: "Ollama", children: "Ollama" })
                    ]
                  }
                )
              ] }),
              a === "OpenRouter" ? /* @__PURE__ */ s.jsxs("div", { children: [
                /* @__PURE__ */ s.jsx("label", { className: "block text-sm font-medium text-gray-700 mb-1", children: "Model" }),
                /* @__PURE__ */ s.jsxs(
                  "select",
                  {
                    value: f,
                    onChange: (B) => m(B.target.value),
                    className: "w-full p-2 border border-gray-300 rounded text-sm",
                    children: [
                      /* @__PURE__ */ s.jsx("option", { value: "anthropic/claude-3.5-sonnet", children: "Claude 3.5 Sonnet" }),
                      /* @__PURE__ */ s.jsx("option", { value: "anthropic/claude-3.5-haiku", children: "Claude 3.5 Haiku" }),
                      /* @__PURE__ */ s.jsx("option", { value: "openai/gpt-4o", children: "GPT-4o" }),
                      /* @__PURE__ */ s.jsx("option", { value: "openai/gpt-4o-mini", children: "GPT-4o Mini" }),
                      /* @__PURE__ */ s.jsx("option", { value: "openai/o1", children: "OpenAI o1" }),
                      /* @__PURE__ */ s.jsx("option", { value: "openai/o1-mini", children: "OpenAI o1-mini" }),
                      /* @__PURE__ */ s.jsx("option", { value: "google/gemini-2.0-flash-exp", children: "Gemini 2.0 Flash" }),
                      /* @__PURE__ */ s.jsx("option", { value: "google/gemini-pro-1.5", children: "Gemini 1.5 Pro" }),
                      /* @__PURE__ */ s.jsx("option", { value: "meta-llama/llama-3.3-70b-instruct", children: "Llama 3.3 70B" }),
                      /* @__PURE__ */ s.jsx("option", { value: "meta-llama/llama-3.1-405b-instruct", children: "Llama 3.1 405B" }),
                      /* @__PURE__ */ s.jsx("option", { value: "deepseek/deepseek-chat", children: "DeepSeek Chat" }),
                      /* @__PURE__ */ s.jsx("option", { value: "qwen/qwen-2.5-72b-instruct", children: "Qwen 2.5 72B" })
                    ]
                  }
                )
              ] }) : /* @__PURE__ */ s.jsxs("div", { children: [
                /* @__PURE__ */ s.jsx("label", { className: "block text-sm font-medium text-gray-700 mb-1", children: "Model" }),
                /* @__PURE__ */ s.jsx(
                  "input",
                  {
                    type: "text",
                    value: x,
                    onChange: (B) => N(B.target.value),
                    placeholder: "e.g., llama3",
                    className: "w-full p-2 border border-gray-300 rounded text-sm"
                  }
                )
              ] })
            ] }),
            a === "OpenRouter" && /* @__PURE__ */ s.jsxs("div", { children: [
              /* @__PURE__ */ s.jsxs("label", { className: "block text-sm font-medium text-gray-700 mb-1", children: [
                "API Key ",
                /* @__PURE__ */ s.jsxs("span", { className: "text-xs text-gray-500", children: [
                  "(",
                  /* @__PURE__ */ s.jsx("a", { href: "https://openrouter.ai/keys", target: "_blank", rel: "noopener noreferrer", className: "text-blue-600 hover:underline", children: "get key" }),
                  ")"
                ] })
              ] }),
              /* @__PURE__ */ s.jsx(
                "input",
                {
                  type: "password",
                  value: d,
                  onChange: (B) => c(B.target.value),
                  placeholder: "sk-or-...",
                  className: "w-full p-2 border border-gray-300 rounded text-sm"
                }
              )
            ] }),
            /* @__PURE__ */ s.jsxs("div", { children: [
              /* @__PURE__ */ s.jsxs(
                "button",
                {
                  onClick: () => A(!w),
                  className: "text-sm text-gray-600 hover:text-gray-800 flex items-center gap-1",
                  children: [
                    /* @__PURE__ */ s.jsx("span", { children: w ? "▼" : "▶" }),
                    "Advanced Settings"
                  ]
                }
              ),
              w && /* @__PURE__ */ s.jsx("div", { className: "mt-3 space-y-3 pl-4 border-l-2 border-gray-200", children: /* @__PURE__ */ s.jsxs("div", { children: [
                /* @__PURE__ */ s.jsx("label", { className: "block text-sm font-medium text-gray-700 mb-1", children: "Base URL" }),
                /* @__PURE__ */ s.jsx(
                  "input",
                  {
                    type: "text",
                    value: a === "OpenRouter" ? h : S,
                    onChange: (B) => a === "OpenRouter" ? y(B.target.value) : E(B.target.value),
                    placeholder: a === "OpenRouter" ? "https://openrouter.ai/api/v1" : "http://localhost:11434",
                    className: "w-full p-2 border border-gray-300 rounded text-sm"
                  }
                )
              ] }) })
            ] }),
            p && /* @__PURE__ */ s.jsx("div", { className: `p-3 rounded-md ${p.success ? "bg-green-50 text-green-800 border border-green-200" : "bg-red-50 text-red-800 border border-red-200"}`, children: /* @__PURE__ */ s.jsxs("span", { className: "text-sm font-medium", children: [
              p.success ? "✓" : "✗",
              " ",
              p.message
            ] }) })
          ] }),
          r === "transforms" && /* @__PURE__ */ s.jsx(lu, { onResult: () => {
          } }),
          r === "keys" && /* @__PURE__ */ s.jsx(gu, { onResult: () => {
          } }),
          r === "schema-service" && /* @__PURE__ */ s.jsxs("div", { className: "space-y-4", children: [
            /* @__PURE__ */ s.jsxs("div", { className: "mb-4", children: [
              /* @__PURE__ */ s.jsx("h4", { className: "text-md font-semibold text-gray-900 mb-2", children: "Schema Service Environment" }),
              /* @__PURE__ */ s.jsx("p", { className: "text-sm text-gray-600 mb-4", children: "Select which schema service endpoint to use. This affects where schemas are loaded from and saved to." })
            ] }),
            /* @__PURE__ */ s.jsx("div", { className: "space-y-3", children: Object.values(zt).map((B) => /* @__PURE__ */ s.jsxs(
              "label",
              {
                className: `flex items-start p-4 border-2 rounded-lg cursor-pointer transition-all ${M === B.id ? "border-blue-500 bg-blue-50" : "border-gray-200 hover:border-gray-300 bg-white"}`,
                children: [
                  /* @__PURE__ */ s.jsx(
                    "input",
                    {
                      type: "radio",
                      name: "schemaEnvironment",
                      value: B.id,
                      checked: M === B.id,
                      onChange: (X) => R(X.target.value),
                      className: "mt-1 mr-3"
                    }
                  ),
                  /* @__PURE__ */ s.jsxs("div", { className: "flex-1", children: [
                    /* @__PURE__ */ s.jsxs("div", { className: "flex items-center justify-between mb-2", children: [
                      /* @__PURE__ */ s.jsx("span", { className: "text-sm font-semibold text-gray-900", children: B.name }),
                      /* @__PURE__ */ s.jsxs("div", { className: "flex items-center gap-2", children: [
                        ct(B.id),
                        M === B.id && /* @__PURE__ */ s.jsx("span", { className: "text-xs bg-blue-100 text-blue-700 px-2 py-1 rounded", children: "Active" })
                      ] })
                    ] }),
                    /* @__PURE__ */ s.jsx("p", { className: "text-xs text-gray-600 mt-1", children: B.description }),
                    /* @__PURE__ */ s.jsx("p", { className: "text-xs text-gray-500 mt-1 font-mono", children: B.baseUrl || window.location.origin }),
                    k[B.id] && !k[B.id].success && /* @__PURE__ */ s.jsxs("p", { className: "text-xs text-red-600 mt-1", children: [
                      "Error: ",
                      k[B.id].error
                    ] })
                  ] })
                ]
              },
              B.id
            )) }),
            p && /* @__PURE__ */ s.jsx("div", { className: `p-3 rounded-md ${p.success ? "bg-green-50 text-green-800 border border-green-200" : "bg-red-50 text-red-800 border border-red-200"}`, children: /* @__PURE__ */ s.jsxs("span", { className: "text-sm font-medium", children: [
              p.success ? "✓" : "✗",
              " ",
              p.message
            ] }) })
          ] }),
          r === "database" && /* @__PURE__ */ s.jsxs("div", { className: "space-y-4", children: [
            /* @__PURE__ */ s.jsxs("div", { className: "mb-4", children: [
              /* @__PURE__ */ s.jsx("h4", { className: "text-md font-semibold text-gray-900 mb-2", children: "Database Storage Backend" }),
              /* @__PURE__ */ s.jsx("p", { className: "text-sm text-gray-600 mb-4", children: "Choose the storage backend for your database. Changes require a server restart." })
            ] }),
            /* @__PURE__ */ s.jsxs("div", { children: [
              /* @__PURE__ */ s.jsx("label", { className: "block text-sm font-medium text-gray-700 mb-2", children: "Storage Type" }),
              /* @__PURE__ */ s.jsxs(
                "select",
                {
                  value: z,
                  onChange: (B) => V(B.target.value),
                  className: "w-full p-2 border border-gray-300 rounded text-sm",
                  children: [
                    /* @__PURE__ */ s.jsx("option", { value: "local", children: "Local (Sled)" }),
                    /* @__PURE__ */ s.jsx("option", { value: "dynamodb", children: "DynamoDB" }),
                    /* @__PURE__ */ s.jsx("option", { value: "s3", children: "S3" })
                  ]
                }
              )
            ] }),
            z === "local" ? /* @__PURE__ */ s.jsxs("div", { children: [
              /* @__PURE__ */ s.jsx("label", { className: "block text-sm font-medium text-gray-700 mb-1", children: "Storage Path" }),
              /* @__PURE__ */ s.jsx(
                "input",
                {
                  type: "text",
                  value: G,
                  onChange: (B) => L(B.target.value),
                  placeholder: "data",
                  className: "w-full p-2 border border-gray-300 rounded text-sm"
                }
              ),
              /* @__PURE__ */ s.jsx("p", { className: "text-xs text-gray-500 mt-1", children: "Local filesystem path where the database will be stored" })
            ] }) : z === "dynamodb" ? /* @__PURE__ */ s.jsxs("div", { className: "space-y-3", children: [
              /* @__PURE__ */ s.jsxs("div", { children: [
                /* @__PURE__ */ s.jsxs("label", { className: "block text-sm font-medium text-gray-700 mb-1", children: [
                  "Table Name ",
                  /* @__PURE__ */ s.jsx("span", { className: "text-red-500", children: "*" })
                ] }),
                /* @__PURE__ */ s.jsx(
                  "input",
                  {
                    type: "text",
                    value: J,
                    onChange: (B) => Q(B.target.value),
                    placeholder: "DataFoldStorage",
                    className: "w-full p-2 border border-gray-300 rounded text-sm"
                  }
                ),
                /* @__PURE__ */ s.jsx("p", { className: "text-xs text-gray-500 mt-1", children: "Base table name (namespaces will be appended automatically)" })
              ] }),
              /* @__PURE__ */ s.jsxs("div", { children: [
                /* @__PURE__ */ s.jsxs("label", { className: "block text-sm font-medium text-gray-700 mb-1", children: [
                  "AWS Region ",
                  /* @__PURE__ */ s.jsx("span", { className: "text-red-500", children: "*" })
                ] }),
                /* @__PURE__ */ s.jsx(
                  "input",
                  {
                    type: "text",
                    value: ge,
                    onChange: (B) => Me(B.target.value),
                    placeholder: "us-west-2",
                    className: "w-full p-2 border border-gray-300 rounded text-sm"
                  }
                ),
                /* @__PURE__ */ s.jsx("p", { className: "text-xs text-gray-500 mt-1", children: "AWS region where your DynamoDB tables are located" })
              ] }),
              /* @__PURE__ */ s.jsxs("div", { children: [
                /* @__PURE__ */ s.jsx("label", { className: "block text-sm font-medium text-gray-700 mb-1", children: "User ID (Optional)" }),
                /* @__PURE__ */ s.jsx(
                  "input",
                  {
                    type: "text",
                    value: ze,
                    onChange: (B) => ne(B.target.value),
                    placeholder: "Leave empty for single-tenant",
                    className: "w-full p-2 border border-gray-300 rounded text-sm"
                  }
                ),
                /* @__PURE__ */ s.jsx("p", { className: "text-xs text-gray-500 mt-1", children: "User ID for multi-tenant isolation (uses partition key)" })
              ] }),
              /* @__PURE__ */ s.jsx("div", { className: "p-3 bg-yellow-50 border border-yellow-200 rounded-md", children: /* @__PURE__ */ s.jsxs("p", { className: "text-xs text-yellow-800", children: [
                /* @__PURE__ */ s.jsx("strong", { children: "Note:" }),
                " Ensure your AWS credentials are configured (via environment variables, IAM role, or AWS CLI). The DynamoDB tables will be created automatically if they don't exist."
              ] }) })
            ] }) : /* @__PURE__ */ s.jsxs("div", { className: "space-y-3", children: [
              /* @__PURE__ */ s.jsxs("div", { children: [
                /* @__PURE__ */ s.jsxs("label", { className: "block text-sm font-medium text-gray-700 mb-1", children: [
                  "S3 Bucket ",
                  /* @__PURE__ */ s.jsx("span", { className: "text-red-500", children: "*" })
                ] }),
                /* @__PURE__ */ s.jsx(
                  "input",
                  {
                    type: "text",
                    value: ce,
                    onChange: (B) => mt(B.target.value),
                    placeholder: "my-datafold-bucket",
                    className: "w-full p-2 border border-gray-300 rounded text-sm"
                  }
                ),
                /* @__PURE__ */ s.jsx("p", { className: "text-xs text-gray-500 mt-1", children: "S3 bucket name where the database will be stored" })
              ] }),
              /* @__PURE__ */ s.jsxs("div", { children: [
                /* @__PURE__ */ s.jsxs("label", { className: "block text-sm font-medium text-gray-700 mb-1", children: [
                  "AWS Region ",
                  /* @__PURE__ */ s.jsx("span", { className: "text-red-500", children: "*" })
                ] }),
                /* @__PURE__ */ s.jsx(
                  "input",
                  {
                    type: "text",
                    value: pt,
                    onChange: (B) => At(B.target.value),
                    placeholder: "us-east-1",
                    className: "w-full p-2 border border-gray-300 rounded text-sm"
                  }
                ),
                /* @__PURE__ */ s.jsx("p", { className: "text-xs text-gray-500 mt-1", children: "AWS region where your S3 bucket is located" })
              ] }),
              /* @__PURE__ */ s.jsxs("div", { children: [
                /* @__PURE__ */ s.jsx("label", { className: "block text-sm font-medium text-gray-700 mb-1", children: "S3 Prefix (Optional)" }),
                /* @__PURE__ */ s.jsx(
                  "input",
                  {
                    type: "text",
                    value: nt,
                    onChange: (B) => at(B.target.value),
                    placeholder: "folddb",
                    className: "w-full p-2 border border-gray-300 rounded text-sm"
                  }
                ),
                /* @__PURE__ */ s.jsx("p", { className: "text-xs text-gray-500 mt-1", children: 'Prefix/path within the bucket (defaults to "folddb")' })
              ] }),
              /* @__PURE__ */ s.jsxs("div", { children: [
                /* @__PURE__ */ s.jsx("label", { className: "block text-sm font-medium text-gray-700 mb-1", children: "Local Cache Path" }),
                /* @__PURE__ */ s.jsx(
                  "input",
                  {
                    type: "text",
                    value: Ce,
                    onChange: (B) => Ye(B.target.value),
                    placeholder: "/tmp/folddb-data",
                    className: "w-full p-2 border border-gray-300 rounded text-sm"
                  }
                ),
                /* @__PURE__ */ s.jsx("p", { className: "text-xs text-gray-500 mt-1", children: "Local filesystem path for caching S3 data (defaults to /tmp/folddb-data)" })
              ] }),
              /* @__PURE__ */ s.jsx("div", { className: "p-3 bg-yellow-50 border border-yellow-200 rounded-md", children: /* @__PURE__ */ s.jsxs("p", { className: "text-xs text-yellow-800", children: [
                /* @__PURE__ */ s.jsx("strong", { children: "Note:" }),
                " Ensure your AWS credentials are configured (via environment variables, IAM role, or AWS CLI). The database will be synced to/from S3 on startup and shutdown."
              ] }) })
            ] }),
            p && /* @__PURE__ */ s.jsx("div", { className: `p-3 rounded-md ${p.success ? "bg-green-50 text-green-800 border border-green-200" : "bg-red-50 text-red-800 border border-red-200"}`, children: /* @__PURE__ */ s.jsxs("span", { className: "text-sm font-medium", children: [
              p.success ? "✓" : "✗",
              " ",
              p.message
            ] }) })
          ] })
        ] })
      ] }),
      /* @__PURE__ */ s.jsx("div", { className: "bg-gray-50 px-4 py-3 sm:px-6 sm:flex sm:flex-row-reverse gap-3 border-t border-gray-200", children: r === "ai" || r === "schema-service" || r === "database" ? /* @__PURE__ */ s.jsxs(s.Fragment, { children: [
        /* @__PURE__ */ s.jsx(
          "button",
          {
            onClick: r === "ai" ? er : r === "schema-service" ? yt : ot,
            className: "w-full inline-flex justify-center rounded-md border border-transparent shadow-sm px-4 py-2 bg-blue-600 text-base font-medium text-white hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 sm:ml-3 sm:w-auto sm:text-sm",
            children: r === "database" ? "Save and Restart DB" : "Save Configuration"
          }
        ),
        /* @__PURE__ */ s.jsx(
          "button",
          {
            onClick: t,
            className: "mt-3 w-full inline-flex justify-center rounded-md border border-gray-300 shadow-sm px-4 py-2 bg-white text-base font-medium text-gray-700 hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 sm:mt-0 sm:w-auto sm:text-sm",
            children: "Cancel"
          }
        )
      ] }) : /* @__PURE__ */ s.jsx(
        "button",
        {
          onClick: t,
          className: "w-full inline-flex justify-center rounded-md border border-gray-300 shadow-sm px-4 py-2 bg-white text-base font-medium text-gray-700 hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 sm:w-auto sm:text-sm",
          children: "Close"
        }
      ) })
    ] })
  ] }) }) : null;
}
function qu() {
  const [e, t] = D([]), r = or(null), n = () => {
    Promise.resolve(
      navigator.clipboard.writeText(e.join(`
`))
    ).catch(() => {
    });
  };
  return xe(() => {
    me.getLogs().then((l) => {
      if (l.success && l.data) {
        const d = l.data.logs || [];
        t(Array.isArray(d) ? d : []);
      } else
        t([]);
    }).catch(() => t([]));
    const a = me.createLogStream(
      (l) => {
        t((d) => [...d, l]);
      },
      (l) => {
        console.warn("Log stream error:", l);
      }
    );
    return () => a.close();
  }, []), xe(() => {
    var a;
    (a = r.current) == null || a.scrollIntoView({ behavior: "smooth" });
  }, [e]), /* @__PURE__ */ s.jsxs("aside", { className: "w-80 bg-gray-900 text-white flex flex-col overflow-hidden", children: [
    /* @__PURE__ */ s.jsxs("div", { className: "flex items-center justify-between p-4 border-b border-gray-700", children: [
      /* @__PURE__ */ s.jsx("h2", { className: "text-lg font-semibold", children: "Logs" }),
      /* @__PURE__ */ s.jsx(
        "button",
        {
          onClick: n,
          className: "text-xs text-blue-300 hover:underline",
          children: "Copy"
        }
      )
    ] }),
    /* @__PURE__ */ s.jsxs("div", { className: "flex-1 overflow-y-auto p-4 space-y-1 text-xs font-mono", children: [
      e.map((a, l) => /* @__PURE__ */ s.jsx("div", { children: a }, l)),
      /* @__PURE__ */ s.jsx("div", { ref: r })
    ] })
  ] });
}
export {
  Hu as FileUploadTab,
  Fu as FoldDbProvider,
  Vu as IngestionTab,
  Uu as LlmQueryTab,
  qu as LogSidebar,
  Ku as MutationTab,
  zu as NativeIndexTab,
  $u as QueryTab,
  Mu as ResultsSection,
  Lu as SchemaTab,
  Gu as SettingsModal,
  Pu as StatusSection,
  Bu as TabNavigation,
  wr as useAppDispatch,
  he as useAppSelector,
  Zd as useApprovedSchemas
};
