import { useMemo, useState, useCallback } from 'react'
import {
  extractData,
  summarizeCounts,
  getSortedHashKeys,
  getSortedRangeKeys,
  getFieldsAt,
  sliceKeys
} from '../utils/hashRangeResults'
import { executeQuery } from '../api/clients/mutationClient'
import { getSchema } from '../api/clients/schemaClient'
import { toggleSetItem, getFieldNames } from '../utils/schemaUtils'

// Simple, dependency-free lazy list windowing.
const DEFAULT_PAGE_SIZE = 50

function isReference(value) {
  return value && typeof value === 'object' && !Array.isArray(value)
    && typeof value.schema === 'string' && value.key && typeof value.key === 'object'
}

function isReferenceArray(value) {
  return Array.isArray(value) && value.length > 0 && value.some(isReference)
}

function buildFilterFromKey(key) {
  if (key.hash && key.range) {
    return { HashRangeKey: { hash: key.hash, range: key.range } }
  }
  if (key.hash) {
    return { HashKey: key.hash }
  }
  if (key.range) {
    return { RangeKey: key.range }
  }
  return undefined
}

function truncateHash(s) {
  if (typeof s === 'string' && s.length > 16) return s.slice(0, 8) + '...' + s.slice(-8)
  return s
}

function referenceLabel(ref) {
  const parts = []
  if (ref.key.hash) parts.push(`hash:${truncateHash(ref.key.hash)}`)
  if (ref.key.range) parts.push(`range:${ref.key.range}`)
  const name = truncateHash(ref.schema)
  return `${name} (${parts.join(', ')})`
}

function ReferenceValue({ reference }) {
  const [fetched, setFetched] = useState(null)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState(null)
  const [expanded, setExpanded] = useState(false)
  const [displayName, setDisplayName] = useState(null)

  const handleFetch = async () => {
    if (fetched) {
      setExpanded(!expanded)
      return
    }
    setLoading(true)
    setError(null)
    try {
      const schemaRes = await getSchema(reference.schema)
      const schema = schemaRes.data?.schema || schemaRes.data
      if (schema?.descriptive_name) {
        setDisplayName(schema.descriptive_name)
      }
      const fieldNames = getFieldNames(schema)
      if (fieldNames.length === 0) {
        throw new Error(`No fields found for schema "${reference.schema}"`)
      }

      const filter = buildFilterFromKey(reference.key)
      const query = { schema_name: reference.schema, fields: fieldNames }
      if (filter) query.filter = filter

      let queryRes = await executeQuery(query)
      if (!queryRes.success) {
        throw new Error(queryRes.error || 'Query failed')
      }
      let results = queryRes.data?.results || queryRes.data
      // If filtered query returned no results, retry without filter
      if (Array.isArray(results) && results.length === 0 && filter) {
        const retryRes = await executeQuery({ schema_name: reference.schema, fields: fieldNames })
        if (retryRes.success) {
          results = retryRes.data?.results || retryRes.data
        }
      }
      setFetched(results)
      setExpanded(true)
    } catch (e) {
      setError(e.message)
    } finally {
      setLoading(false)
    }
  }

  return (
    <div className="space-y-1">
      <div className="flex items-center gap-2">
        <span className="font-mono text-xs text-secondary">{'\u2192'} {displayName || referenceLabel(reference)}</span>
        <button
          type="button"
          className="btn-secondary btn-sm text-xs px-2 py-0.5"
          onClick={handleFetch}
          disabled={loading}
        >
          {loading ? 'Loading...' : fetched ? (expanded ? 'Hide' : 'Show') : 'Fetch'}
        </button>
      </div>
      {error && <div className="text-xs text-red-500">{error}</div>}
      {fetched && expanded && (
        <div className="ml-4 border-l border-border pl-2">
          {Array.isArray(fetched) ? fetched.map((item, i) => (
            <FieldsTable key={i} fields={item.fields || item} />
          )) : (
            <FieldsTable fields={fetched.fields || fetched} />
          )}
        </div>
      )}
    </div>
  )
}

function ReferenceArrayValue({ references }) {
  return (
    <div className="space-y-2">
      <span className="text-xs text-secondary">{references.length} reference{references.length !== 1 ? 's' : ''}</span>
      {references.map((ref, i) => (
        <ReferenceValue key={`${ref.schema}-${ref.key?.hash || ''}-${ref.key?.range || ''}-${i}`} reference={ref} />
      ))}
    </div>
  )
}

function renderFieldValue(value) {
  if (isReference(value)) {
    return <ReferenceValue reference={value} />
  }
  if (isReferenceArray(value)) {
    return <ReferenceArrayValue references={value.filter(isReference)} />
  }
  return <pre className="font-mono whitespace-pre-wrap break-words">{formatValue(value)}</pre>
}

function ToggleButton({ isOpen, onClick, label }) {
  return (
    <button
      type="button"
      className="text-left w-full flex items-center justify-between px-3 py-2 hover:bg-surface-secondary rounded"
      onClick={onClick}
      aria-expanded={isOpen}
    >
      <span className="font-mono text-sm text-primary truncate">{label}</span>
      <span className="ml-2 text-secondary text-xs">{isOpen ? '▼' : '▶'}</span>
    </button>
  )
}

export function FieldsTable({ fields }) {
  const entries = useMemo(() => Object.entries(fields || {}), [fields])
  if (entries.length === 0) {
    return (
      <div className="text-xs text-secondary italic px-3 py-2">No fields</div>
    )
  }

  return (
    <div className="px-3 py-2 overflow-x-auto">
      <table className="min-w-full border-separate border-spacing-y-1">
        <tbody>
          {entries.map(([k, v]) => (
            <tr key={k} className="bg-surface">
              <td className="align-top text-xs font-medium text-primary pr-4 whitespace-nowrap">{k}</td>
              <td className="align-top text-xs text-primary">
                {renderFieldValue(v)}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  )
}

function formatValue(value) {
  if (value === null) return 'null'
  if (typeof value === 'string') return value
  if (typeof value === 'number' || typeof value === 'boolean') return String(value)
  try {
    return JSON.stringify(value, null, 2)
  } catch {
    return String(value)
  }
}

export default function StructuredResults({ results, pageSize = DEFAULT_PAGE_SIZE }) {
  const data = useMemo(() => extractData(results) || {}, [results])
  const counts = useMemo(() => summarizeCounts(results), [results])
  const allHashes = useMemo(() => getSortedHashKeys(results), [results])

  const [hashOpen, setHashOpen] = useState(() => new Set())
  const [rangeOpen, setRangeOpen] = useState(() => new Set())
  const [hashWindow, setHashWindow] = useState({ start: 0, count: pageSize })
  const [rangeWindows, setRangeWindows] = useState(() => new Map())

  const toggleHash = useCallback((h) => {
    setHashOpen((prev) => toggleSetItem(prev, h))
    setRangeWindows((prev) => {
      if (!hashOpen.has(h)) {
        const total = getSortedRangeKeys(data, h).length
        const next = new Map(prev)
        next.set(h, { start: 0, count: Math.min(pageSize, total) })
        return next
      }
      return prev
    })
  }, [data, hashOpen, pageSize])

  const toggleRange = useCallback((h, r) => {
    const key = h + '||' + r
    setRangeOpen((prev) => toggleSetItem(prev, key))
  }, [])

const showMoreHashes = useCallback(() => {
  const nextCount = Math.min(allHashes.length, hashWindow.count + pageSize)
  setHashWindow((_w) => ({ start: 0, count: nextCount }))
}, [allHashes, hashWindow.count, pageSize])

  const visibleHashes = useMemo(() => sliceKeys(allHashes, hashWindow.start, hashWindow.count), [allHashes, hashWindow])

  return (
    <div className="space-y-2">
      <div className="text-xs text-secondary">
        <span className="mr-4">Hashes: <strong>{counts.hashes}</strong></span>
        <span>Ranges: <strong>{counts.ranges}</strong></span>
      </div>

      <div className="border border-border divide-y divide-border bg-surface-secondary">
        {visibleHashes.map((h) => (
          <div key={h} className="p-2">
            <ToggleButton
              isOpen={hashOpen.has(h)}
              onClick={() => toggleHash(h)}
              label={`hash: ${String(h)}`}
            />

            {hashOpen.has(h) && (
              <HashRanges
                data={data}
                hashKey={h}
                rangeOpen={rangeOpen}
                onToggleRange={toggleRange}
                pageSize={pageSize}
                rangeWindow={rangeWindows.get(h)}
                setRangeWindow={(w) => setRangeWindows((prev) => new Map(prev).set(h, w))}
              />
            )}
          </div>
        ))}
      </div>

      {hashWindow.count < allHashes.length && (
        <div className="pt-2">
          <button type="button" onClick={showMoreHashes} className="btn-secondary btn-sm">
            Show more hashes ({hashWindow.count}/{allHashes.length})
          </button>
        </div>
      )}
    </div>
  )
}

function HashRanges({ data, hashKey, rangeOpen, onToggleRange, pageSize, rangeWindow, setRangeWindow }) {
  const allRanges = useMemo(() => getSortedRangeKeys(data, hashKey), [data, hashKey])
  const effectiveWindow = useMemo(() => rangeWindow || { start: 0, count: Math.min(pageSize, allRanges.length) }, [rangeWindow, pageSize, allRanges.length])
  const visibleRanges = useMemo(() => sliceKeys(allRanges, effectiveWindow.start, effectiveWindow.count), [allRanges, effectiveWindow])

  const showMoreRanges = useCallback(() => {
    const next = Math.min(allRanges.length, effectiveWindow.count + pageSize)
    setRangeWindow({ start: 0, count: next })
  }, [allRanges.length, effectiveWindow.count, pageSize, setRangeWindow])

  return (
    <div className="ml-4 mt-1 border-l pl-3">
      {visibleRanges.map((r) => (
        <div key={r} className="py-1">
          <ToggleButton
            isOpen={rangeOpen.has(hashKey + '||' + r)}
            onClick={() => onToggleRange(hashKey, r)}
            label={`range: ${String(r)}`}
          />
          {rangeOpen.has(hashKey + '||' + r) && (
            <div className="ml-4 mt-1">
              <FieldsTable fields={getFieldsAt(data, hashKey, r) || {}} />
            </div>
          )}
        </div>
      ))}

      {effectiveWindow.count < allRanges.length && (
        <div className="pt-1">
          <button type="button" onClick={showMoreRanges} className="btn-secondary btn-sm">
            Show more ranges ({effectiveWindow.count}/{allRanges.length})
          </button>
        </div>
      )}
    </div>
  )
}


