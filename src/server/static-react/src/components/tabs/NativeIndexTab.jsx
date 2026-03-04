import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { useApprovedSchemas } from '../../hooks/useApprovedSchemas.js'
import { nativeIndexClient, mutationClient } from '../../api/clients'
import { FieldsTable } from '../StructuredResults'
import {
  createHashRangeKeyFilter,
  createHashKeyFilter,
  createRangeKeyFilter,
} from '../../utils/filterUtils'
import { getSchemaDisplayName, getFieldNames, toErrorMessage } from '../../utils/schemaUtils'

function formatValue(v) {
  if (v == null) return ''
  if (typeof v === 'string') return v
  try { return JSON.stringify(v) } catch { return String(v) }
}

function RecordRow({ result, schemaByName, fetchRecordFor }) {
  const [expanded, setExpanded] = useState(false)
  const [details, setDetails] = useState(null)
  const [loading, setLoading] = useState(false)

  const schema = schemaByName?.get(result.schema_name)
  const displayName = getSchemaDisplayName(schema) || result.schema_name
  const hash = result.key_value?.hash ?? ''
  const range = result.key_value?.range ?? ''
  const truncatedHash = hash.length > 10 ? hash.slice(0, 10) + '...' : hash

  const handleShowRecord = useCallback(async () => {
    if (expanded) {
      setExpanded(false)
      return
    }
    setExpanded(true)
    if (details) return
    setLoading(true)
    try {
      const fields = await fetchRecordFor(result.schema_name, result.key_value)
      setDetails(fields)
    } catch {
      setDetails({})
    } finally {
      setLoading(false)
    }
  }, [expanded, details, fetchRecordFor, result.schema_name, result.key_value])

  return (
    <div className="ml-4 border-l-2 border-border pl-3 py-1">
      <div className="flex items-center gap-2 text-xs">
        <span className="font-mono text-primary" title={result.schema_name}>{displayName}</span>
        <span className="text-secondary" title={hash}>{truncatedHash}</span>
        {range && <span className="text-secondary">{range}</span>}
        <span className="text-secondary">field:{result.field}</span>
        <button
          type="button"
          className="btn-secondary btn-sm ml-auto"
          onClick={handleShowRecord}
          disabled={loading}
        >
          {loading ? 'Loading...' : expanded ? 'Hide Record' : 'Show Record'}
        </button>
      </div>
      {expanded && details && (
        <div className="mt-1 ml-2 bg-surface-secondary p-2 rounded">
          <FieldsTable fields={details} />
        </div>
      )}
    </div>
  )
}

function WordGroup({ value, records, schemaByName, fetchRecordFor, buildKeyId, isOpen, onToggle }) {
  return (
    <div className="border border-border rounded mb-2">
      <button
        type="button"
        className="w-full text-left px-3 py-2 flex items-center gap-2 hover:bg-surface-secondary transition-colors"
        onClick={onToggle}
      >
        <span className="text-xs text-secondary">{isOpen ? '▼' : '▶'}</span>
        <span className="font-mono text-sm text-primary font-semibold">{formatValue(value)}</span>
        <span className="text-xs text-secondary">({records.length} record{records.length !== 1 ? 's' : ''})</span>
      </button>
      {isOpen && (
        <div className="px-2 pb-2">
          {records.map((r) => {
            const id = buildKeyId(r.schema_name, r.key_value)
            return (
              <RecordRow
                key={`${id}|${r.field}`}
                result={r}
                schemaByName={schemaByName}
                fetchRecordFor={fetchRecordFor}
              />
            )
          })}
        </div>
      )}
    </div>
  )
}

export default function NativeIndexTab({ onResult }) {
  const { approvedSchemas, refetch: refetchSchemas } = useApprovedSchemas()
  const [term, setTerm] = useState('')
  const [isSearching, setIsSearching] = useState(false)
  const [results, setResults] = useState([])
  const [error, setError] = useState(null)
  const [expandedWords, setExpandedWords] = useState(() => new Set())
  const [visibleCount, setVisibleCount] = useState(10)
  const sentinelRef = useRef(null)

  useEffect(() => { refetchSchemas() }, [refetchSchemas])

  const handleSearch = useCallback(async () => {
    setIsSearching(true)
    setError(null)
    try {
      const res = await nativeIndexClient.search(term)
      if (res.success) {
        const resultsArray = res.data?.results || []
        setResults(resultsArray)
        setExpandedWords(new Set())
        setVisibleCount(10)
        onResult({ success: true, data: resultsArray })
      } else {
        setError(res.error || 'Search failed')
        onResult({ error: res.error || 'Search failed', status: res.status })
      }
    } catch (e) {
      const msg = toErrorMessage(e)
      setError(msg || 'Network error')
      onResult({ error: msg || 'Network error' })
    } finally {
      setIsSearching(false)
    }
  }, [term, onResult])

  const schemaByName = useMemo(() => {
    const map = new Map()
    ;(approvedSchemas || []).forEach(s => map.set(s.name, s))
    return map
  }, [approvedSchemas])

  const buildKeyId = useCallback((schema, kv) => {
    const h = kv?.hash ?? ''
    const r = kv?.range ?? ''
    return `${schema}|${h}|${r}`
  }, [])

  const buildFilterForKey = useCallback((kv) => {
    const h = kv?.hash
    const r = kv?.range
    if (h && r) return createHashRangeKeyFilter(h, r)
    if (h) return createHashKeyFilter(h)
    if (r) return createRangeKeyFilter(r)
    return undefined
  }, [])

  const fetchRecordFor = useCallback(async (schema, kv) => {
    const schemaObj = schemaByName.get(schema)
    const fields = getFieldNames(schemaObj)
    const filter = buildFilterForKey(kv)
    const query = { schema_name: schema, fields }
    if (filter) query.filter = filter
    const res = await mutationClient.executeQuery(query)
    if (!res.success) {
      throw new Error(res.error || 'Query failed')
    }
    const arr = Array.isArray(res.data?.results) ? res.data.results : []
    const match = arr.find(x => {
      return String(x?.key?.hash || '') === String(kv?.hash || '') &&
             String(x?.key?.range || '') === String(kv?.range || '')
    }) || arr[0]
    return match?.fields || (match && typeof match === 'object' ? match : {})
  }, [schemaByName, buildFilterForKey])

  const groupedResults = useMemo(() => {
    const map = new Map()
    for (const r of results) {
      const key = formatValue(r.value)
      if (!map.has(key)) {
        map.set(key, { value: r.value, records: [] })
      }
      map.get(key).records.push(r)
    }
    return Array.from(map.values())
  }, [results])

  const visibleGroups = useMemo(
    () => groupedResults.slice(0, visibleCount),
    [groupedResults, visibleCount],
  )

  // IntersectionObserver for scroll-to-load-more
  useEffect(() => {
    const sentinel = sentinelRef.current
    if (!sentinel) return
    // eslint-disable-next-line no-undef
    const observer = new IntersectionObserver(
      (entries) => {
        if (entries[0].isIntersecting) {
          setVisibleCount(prev => prev + 10)
        }
      },
      { threshold: 0.1 },
    )
    observer.observe(sentinel)
    return () => observer.disconnect()
  }, [groupedResults.length])

  const toggleWord = useCallback((value) => {
    setExpandedWords(prev => {
      const next = new Set(prev)
      if (next.has(value)) {
        next.delete(value)
      } else {
        next.add(value)
      }
      return next
    })
  }, [])

  return (
    <div className="space-y-4">
      <div className="flex gap-2 items-center">
        <input
          type="text"
          value={term}
          onChange={(e) => setTerm(e.target.value)}
          placeholder="Search across all schemas..."
          className="input flex-1"
        />
        <button onClick={handleSearch} disabled={isSearching || !term.trim()} className="btn-primary">
          {isSearching ? 'Searching...' : '→ Search'}
        </button>
      </div>

      <div className="text-sm text-secondary">
        {results.length} matches across {groupedResults.length} terms
      </div>

      {error && (
        <div className="text-sm text-gruvbox-red">{error}</div>
      )}

      <div className="overflow-auto max-h-[500px]">
        {visibleGroups.map((group) => {
          const key = formatValue(group.value)
          return (
            <WordGroup
              key={key}
              value={group.value}
              records={group.records}
              schemaByName={schemaByName}
              fetchRecordFor={fetchRecordFor}
              buildKeyId={buildKeyId}
              isOpen={expandedWords.has(key)}
              onToggle={() => toggleWord(key)}
            />
          )
        })}
        {results.length === 0 && (
          <div className="px-2 py-3 text-center text-secondary text-sm">No results</div>
        )}
        {visibleCount < groupedResults.length && (
          <div ref={sentinelRef} className="py-2 text-center text-xs text-secondary">
            Loading more...
          </div>
        )}
      </div>
    </div>
  )
}
