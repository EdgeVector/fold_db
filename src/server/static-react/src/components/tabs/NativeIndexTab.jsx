import { useCallback, useEffect, useMemo, useState } from 'react'
import { useApprovedSchemas } from '../../hooks/useApprovedSchemas.js'
import { nativeIndexClient, mutationClient } from '../../api/clients'
import { 
  createHashRangeKeyFilter,
  createHashKeyFilter
} from '../../utils/filterUtils'

function ResultRow({ r }) {
  return (
    <tr className="border-t">
      <td className="px-2 py-1 text-xs text-gray-600">
        {r.key_value?.hash ?? ''}
      </td>
      <td className="px-2 py-1 text-xs text-gray-600">
        {r.key_value?.range ?? ''}
      </td>
      <td className="px-2 py-1 text-xs font-mono text-gray-800">
        {r.schema_name}
      </td>
      <td className="px-2 py-1 text-xs text-gray-800">
        {r.field}
      </td>
      <td className="px-2 py-1 text-xs text-gray-800 whitespace-pre-wrap break-words">
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
  const { approvedSchemas, isLoading: schemasLoading, refetch: refetchSchemas } = useApprovedSchemas()
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
      setError(e.message || 'Network error')
      onResult({ error: e.message || 'Network error' })
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
    if (r) return createHashKeyFilter(r)
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
      } catch (_e) {
        // store empty to avoid refetch loop
        updates.set(id, {})
      }
    }))
    setRecordDetails(updates)
  }, [results, recordDetails, buildKeyId, fetchRecordFor])

  useEffect(() => {
    if (results.length > 0) {
      fetchAllDetails().catch(() => {})
    }
  }, [results, fetchAllDetails])

  return (
    <div className="p-6 space-y-4">
      <div className="bg-white p-4 rounded-lg shadow">
        <div className="mb-3">
          <h3 className="text-lg font-medium text-gray-900">Native Index Search</h3>
          <p className="text-xs text-gray-500">Search the database-native word index across all approved schemas.</p>
        </div>
        <div className="flex gap-2 items-center">
          <input
            type="text"
            value={term}
            onChange={(e) => setTerm(e.target.value)}
            placeholder="Enter search term (e.g. jennifer)"
            className="flex-1 px-3 py-2 border rounded-md text-sm"
          />
          <button
            onClick={handleSearch}
            disabled={isSearching || !term.trim()}
            className={`btn-terminal px-6 py-2 text-sm font-medium ${isSearching || !term.trim() ? 'opacity-50 cursor-not-allowed' : 'btn-terminal-primary'}`}
          >
            {isSearching ? 'Searching...' : '→ Search'}
          </button>
        </div>
      </div>


      <div className="bg-white p-4 rounded-lg shadow">
        <div className="mb-2 flex items-center justify-between">
          <h4 className="text-md font-medium text-gray-900">Search Results</h4>
          <div className="flex items-center gap-3">
            <span className="text-xs text-gray-500">{results.length} matches</span>
            {results.length > 0 && (
              <button
                type="button"
                className="text-xs px-2 py-1 rounded border border-gray-300 hover:bg-gray-100"
                onClick={() => fetchAllDetails()}
              >
                Refresh Details
              </button>
            )}
          </div>
        </div>
        {error && (
          <div className="mb-2 p-2 bg-red-50 border border-red-200 text-xs text-red-700 rounded">{error}</div>
        )}
        <div className="overflow-auto max-h-[450px]">
          <table className="min-w-full text-left text-xs">
            <thead>
              <tr className="bg-gray-50">
                <th className="px-2 py-2 text-left text-xs font-semibold text-gray-600 uppercase tracking-wide border-b-2 border-gray-300">Hash</th>
                <th className="px-2 py-2 text-left text-xs font-semibold text-gray-600 uppercase tracking-wide border-b-2 border-gray-300">Range</th>
                <th className="px-2 py-2 text-left text-xs font-semibold text-gray-600 uppercase tracking-wide border-b-2 border-gray-300">Schema</th>
                <th className="px-2 py-2 text-left text-xs font-semibold text-gray-600 uppercase tracking-wide border-b-2 border-gray-300">Field</th>
                <th className="px-2 py-2 text-left text-xs font-semibold text-gray-600 uppercase tracking-wide border-b-2 border-gray-300">Value</th>
                <th className="px-2 py-2 border-b-2 border-gray-300"></th>
              </tr>
            </thead>
            <tbody>
              {results.map((r, i) => {
                const id = buildKeyId(r.schema_name, r.key_value)
                const isOpen = expanded.has(id)
                const details = recordDetails.get(id)
                return (
                  <>
                    <ResultRow key={`${id}-row`} r={r} />
                    <tr key={`${id}-actions`} className="border-b">
                      <td colSpan={5}></td>
                      <td className="px-2 py-1 text-right">
                        <button
                          type="button"
                          className="text-xs px-2 py-1 rounded border border-gray-300 hover:bg-gray-100"
                          onClick={async () => {
                            const next = new Set(expanded)
                            if (next.has(id)) next.delete(id); else next.add(id)
                            setExpanded(next)
                            if (!recordDetails.has(id)) {
                              try {
                                const fields = await fetchRecordFor(r.schema_name, r.key_value)
                                setRecordDetails(prev => new Map(prev).set(id, fields))
                              } catch {}
                            }
                          }}
                        >
                          {isOpen ? 'Hide Data' : 'Show Data'}
                        </button>
                      </td>
                    </tr>
                    {isOpen && (
                      <tr key={`${id}-details`}>
                        <td colSpan={6} className="px-2 pb-3">
                          <div className="ml-2 bg-gray-50 border rounded">
                            <FieldsTable fields={details || {}} />
                          </div>
                        </td>
                      </tr>
                    )}
                  </>
                )
              })}
              {results.length === 0 && (
                <tr>
                  <td colSpan={5} className="px-2 py-3 text-center text-gray-500">No results</td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  )
}

