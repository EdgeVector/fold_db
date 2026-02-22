import { useCallback, useEffect, useMemo, useState } from 'react'
import { useApprovedSchemas } from '../../hooks/useApprovedSchemas.js'
import { nativeIndexClient, mutationClient } from '../../api/clients'
import { FieldsTable } from '../StructuredResults'
import {
  createHashRangeKeyFilter,
  createHashKeyFilter,
  createRangeKeyFilter,
} from '../../utils/filterUtils'

function ResultRow({ r, schemaByName }) {
  const schema = schemaByName?.get(r.schema_name)
  const displayName = schema?.descriptive_name || r.schema_name
  return (
    <tr className="border-t">
      <td className="px-2 py-1 text-xs text-secondary">
        {r.key_value?.hash ?? ''}
      </td>
      <td className="px-2 py-1 text-xs text-secondary">
        {r.key_value?.range ?? ''}
      </td>
      <td className="px-2 py-1 text-xs font-mono text-primary" title={r.schema_name}>
        {displayName}
      </td>
      <td className="px-2 py-1 text-xs text-primary">
        {r.field}
      </td>
      <td className="px-2 py-1 text-xs text-primary whitespace-pre-wrap break-words">
        {formatValue(r.value)}
      </td>
    </tr>
  )
}

function formatValue(v) {
  if (v == null) return ''
  if (typeof v === 'string') return v
  try { return JSON.stringify(v) } catch { return String(v) }
}

export default function NativeIndexTab({ onResult }) {
  const { approvedSchemas, refetch: refetchSchemas } = useApprovedSchemas()
  const [term, setTerm] = useState('')
  const [isSearching, setIsSearching] = useState(false)
  const [results, setResults] = useState([])
  const [error, setError] = useState(null)
  const [expanded, setExpanded] = useState(() => new Set())
  const [recordDetails, setRecordDetails] = useState(() => new Map())
  

  useEffect(() => { refetchSchemas() }, [refetchSchemas])

  const handleSearch = useCallback(async () => {
    setIsSearching(true)
    setError(null)
    try {
      const res = await nativeIndexClient.search(term)
      if (res.success) {
        // API returns { ok: true, results: [...] } in data, extract results array
        const resultsArray = res.data?.results || []
        setResults(resultsArray)
        onResult({ success: true, data: resultsArray })
      } else {
        setError(res.error || 'Search failed')
        onResult({ error: res.error || 'Search failed', status: res.status })
      }
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e)
      setError(msg || 'Network error')
      onResult({ error: msg || 'Network error' })
    } finally {
      setIsSearching(false)
    }
  }, [term, onResult])


  const getFieldNames = useCallback((schemaObj) => {
    if (!schemaObj) return []
    const f = schemaObj.fields
    if (Array.isArray(f)) return f.slice()
    if (f && typeof f === 'object') return Object.keys(f)
    return []
  }, [])

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
    // Server returns { ok, results, user_hash } in data - extract results array
    const arr = Array.isArray(res.data?.results) ? res.data.results : []
    // Prefer exact key match if present
    const match = arr.find(x => {
      const kh = x?.key?.hash ?? null
      const kr = x?.key?.range ?? null
      const h = kv?.hash ?? null
      const r = kv?.range ?? null
      return String(kh || '') === String(h || '') && String(kr || '') === String(r || '')
    }) || arr[0]
    return match?.fields || (match && typeof match === 'object' ? match : {})
  }, [schemaByName, getFieldNames, buildFilterForKey])

  const fetchAllDetails = useCallback(async () => {
    const unique = new Map()
    for (const r of results) {
      const id = buildKeyId(r.schema_name, r.key_value)
      if (!unique.has(id)) unique.set(id, r)
    }
    const entries = Array.from(unique.values())
    const updates = new Map(recordDetails)
    await Promise.all(entries.map(async (r) => {
      const id = buildKeyId(r.schema_name, r.key_value)
      if (updates.has(id)) return
      try {
        const fields = await fetchRecordFor(r.schema_name, r.key_value)
        updates.set(id, fields)
      } catch {
        // store empty to avoid refetch loop
        updates.set(id, {})
      }
    }))
    setRecordDetails(updates)
  }, [results, recordDetails, buildKeyId, fetchRecordFor])

  useEffect(() => {
    if (results.length > 0) {
      fetchAllDetails().catch(() => { /* detail fetch is best-effort */ })
    }
  }, [results, fetchAllDetails])

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

      <div className="flex items-center justify-between text-sm">
        <span className="text-secondary">{results.length} matches</span>
        {results.length > 0 && (
          <button type="button" className="btn-secondary btn-sm" onClick={() => fetchAllDetails()}>
            Refresh
          </button>
        )}
      </div>

      {error && (
        <div className="text-sm text-gruvbox-red">{error}</div>
      )}

      <div className="overflow-auto max-h-[450px] border border-border">
          <table className="min-w-full text-left text-xs">
            <thead>
              <tr className="bg-surface-secondary">
                <th className="px-2 py-2 text-left text-xs font-semibold text-secondary uppercase tracking-wide border-b-2 border-border">Hash</th>
                <th className="px-2 py-2 text-left text-xs font-semibold text-secondary uppercase tracking-wide border-b-2 border-border">Range</th>
                <th className="px-2 py-2 text-left text-xs font-semibold text-secondary uppercase tracking-wide border-b-2 border-border">Schema</th>
                <th className="px-2 py-2 text-left text-xs font-semibold text-secondary uppercase tracking-wide border-b-2 border-border">Field</th>
                <th className="px-2 py-2 text-left text-xs font-semibold text-secondary uppercase tracking-wide border-b-2 border-border">Value</th>
                <th className="px-2 py-2 border-b-2 border-border"></th>
              </tr>
            </thead>
            <tbody>
              {results.map((r) => {
                const id = buildKeyId(r.schema_name, r.key_value)
                const isOpen = expanded.has(id)
                const details = recordDetails.get(id)
                return (
                  <>
                    <ResultRow key={`${id}-row`} r={r} schemaByName={schemaByName} />
                    <tr key={`${id}-actions`} className="border-b">
                      <td colSpan={5}></td>
                      <td className="px-2 py-1 text-right">
                        <button
                          type="button"
                          className="btn-secondary btn-sm"
                          onClick={async () => {
                            const next = new Set(expanded)
                            if (next.has(id)) next.delete(id); else next.add(id)
                            setExpanded(next)
                            if (!recordDetails.has(id)) {
                              try {
                                const fields = await fetchRecordFor(r.schema_name, r.key_value)
                                setRecordDetails(prev => new Map(prev).set(id, fields))
                              } catch { /* ignore */ }
                            }
                          }}
                        >
                          {isOpen ? 'Hide Data' : 'Show Data'}
                        </button>
                      </td>
                    </tr>
                    {isOpen && (
                      <tr key={`${id}-details`}>
                        <td colSpan={6} className="px-2 pb-3 bg-surface-secondary">
                          <FieldsTable fields={details || {}} />
                        </td>
                      </tr>
                    )}
                  </>
                )
              })}
              {results.length === 0 && (
                <tr>
                  <td colSpan={5} className="px-2 py-3 text-center text-secondary">No results</td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
    </div>
  )
}

