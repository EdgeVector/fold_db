import { useCallback, useEffect, useMemo, useState, Fragment } from 'react'
import { useAppSelector } from '../../store/hooks'
import { selectAllSchemas } from '../../store/schemaSlice'
import { schemaClient } from '../../api/clients/schemaClient'
import { mutationClient } from '../../api/clients'
import { FieldsTable } from '../StructuredResults'
import { SCHEMA_BADGE_COLORS } from '../../constants/ui'
import {
  createHashRangeKeyFilter,
  createHashKeyFilter,
  createRangeKeyFilter,
} from '../../utils/filterUtils'

const PAGE_SIZE = 50

function keyId(schemaName, kv) {
  return `${schemaName}|${kv?.hash ?? ''}|${kv?.range ?? ''}`
}

function keyLabel(kv) {
  const parts = []
  if (kv?.hash) parts.push(kv.hash)
  if (kv?.range) parts.push(kv.range)
  return parts.join(' / ') || '(default)'
}

function StateBadge({ state }) {
  const cls = SCHEMA_BADGE_COLORS[state] || 'badge badge-warning'
  return <span className={cls}>{state}</span>
}

const IMAGE_EXTENSIONS = /\.(jpe?g|png|gif|webp|svg)$/i

function RecordMetadata({ metadata }) {
  const [expanded, setExpanded] = useState(false)
  const [blobUrl, setBlobUrl] = useState(null)

  if (!metadata || typeof metadata !== 'object') return null

  // Pick the first field entry that has a source_file_name
  const entries = Object.entries(metadata)
  const representative = entries.find(([, v]) => v?.source_file_name)?.[1] || entries[0]?.[1]
  if (!representative) return null

  const sourceFile = representative.source_file_name
  const fileHash = representative.metadata?.file_hash
  if (!sourceFile && !fileHash) return null

  const isImage = sourceFile && IMAGE_EXTENSIONS.test(sourceFile)
  const fileUrl = fileHash ? `/api/file/${fileHash}?name=${encodeURIComponent(sourceFile || '')}` : null

  // Fetch image with auth headers and create blob URL
  useEffect(() => {
    if (!expanded || !isImage || !fileUrl) return
    let revoked = false
    const userHash = localStorage.getItem('fold_user_hash')
    const headers = {}
    if (userHash) {
      headers['x-user-hash'] = userHash
      headers['x-user-id'] = userHash
    }
    fetch(fileUrl, { headers })
      .then((res) => {
        if (!res.ok) throw new Error(res.statusText)
        return res.blob()
      })
      .then((blob) => {
        if (revoked) return
        setBlobUrl(URL.createObjectURL(blob))
      })
      .catch(() => setBlobUrl(null))
    return () => {
      revoked = true
      setBlobUrl((prev) => { if (prev) URL.revokeObjectURL(prev); return null })
    }
  }, [expanded, isImage, fileUrl])

  return (
    <div className="mb-1">
      <button
        type="button"
        className="flex items-center gap-1 text-xs text-tertiary hover:text-secondary transition-colors"
        onClick={() => setExpanded((v) => !v)}
      >
        <span>{expanded ? '▾' : '▸'}</span>
        <span>Source info</span>
        {sourceFile && !expanded && (
          <span className="font-mono text-secondary ml-1 truncate max-w-[300px]">{sourceFile}</span>
        )}
      </button>
      {expanded && (
        <div className="pl-4 pt-1 space-y-1 text-xs text-secondary font-mono">
          {sourceFile && <div>File: {sourceFile}</div>}
          {fileHash && <div>Hash: {fileHash.length > 16 ? fileHash.slice(0, 16) + '…' : fileHash}</div>}
          {isImage && blobUrl && (
            <div className="mt-2">
              <img
                src={blobUrl}
                alt={sourceFile}
                className="max-w-xs max-h-64 rounded border border-border object-contain bg-surface-secondary"
              />
            </div>
          )}
        </div>
      )}
    </div>
  )
}

export default function DataBrowserTab() {
  const schemas = useAppSelector(selectAllSchemas)

  // Schema-level expand state + cached keys
  const [expandedSchemas, setExpandedSchemas] = useState(() => new Set())
  const [schemaKeys, setSchemaKeys] = useState({})       // { name: { keys, total_count } }
  const [schemaLoading, setSchemaLoading] = useState({})  // { name: bool }
  const [schemaErrors, setSchemaErrors] = useState({})    // { name: string }

  // Key-level expand state + cached records
  const [expandedKeys, setExpandedKeys] = useState(() => new Set())
  const [keyRecords, setKeyRecords] = useState({})        // { compositeId: { fields, metadata } }
  const [keyLoading, setKeyLoading] = useState({})        // { compositeId: bool }

  const schemaList = useMemo(() => {
    if (!Array.isArray(schemas)) return []
    return [...schemas].sort((a, b) => (a.name || '').localeCompare(b.name || ''))
  }, [schemas])

  const getFieldNames = useCallback((schemaObj) => {
    if (!schemaObj) return []
    const f = schemaObj.fields
    if (Array.isArray(f)) return f.slice()
    if (f && typeof f === 'object') return Object.keys(f)
    return []
  }, [])

  const fieldCount = useCallback((schema) => {
    const f = schema?.fields
    if (Array.isArray(f)) return f.length
    if (f && typeof f === 'object') return Object.keys(f).length
    return 0
  }, [])

  // -- Schema expansion: fetch keys --
  const toggleSchema = useCallback(async (name) => {
    setExpandedSchemas((prev) => {
      const next = new Set(prev)
      if (next.has(name)) {
        next.delete(name)
      } else {
        next.add(name)
      }
      return next
    })

    // Fetch keys on first expand (or if not already loaded)
    if (!schemaKeys[name] && !schemaLoading[name]) {
      setSchemaLoading((p) => ({ ...p, [name]: true }))
      setSchemaErrors((p) => ({ ...p, [name]: null }))
      try {
        const res = await schemaClient.listSchemaKeys(name, 0, PAGE_SIZE)
        if (res.success && res.data) {
          setSchemaKeys((p) => ({ ...p, [name]: { keys: res.data.keys || [], total_count: res.data.total_count || 0 } }))
        } else {
          setSchemaErrors((p) => ({ ...p, [name]: res.error || 'Failed to fetch keys' }))
        }
      } catch (e) {
        setSchemaErrors((p) => ({ ...p, [name]: (e instanceof Error ? e.message : String(e)) || 'Network error' }))
      } finally {
        setSchemaLoading((p) => ({ ...p, [name]: false }))
      }
    }
  }, [schemaKeys, schemaLoading])

  // -- Load more keys --
  const loadMoreKeys = useCallback(async (name) => {
    const current = schemaKeys[name]
    if (!current) return
    const offset = current.keys.length
    setSchemaLoading((p) => ({ ...p, [name]: true }))
    try {
      const res = await schemaClient.listSchemaKeys(name, offset, PAGE_SIZE)
      if (res.success && res.data) {
        setSchemaKeys((p) => ({
          ...p,
          [name]: {
            keys: [...(p[name]?.keys || []), ...(res.data.keys || [])],
            total_count: res.data.total_count || p[name]?.total_count || 0,
          },
        }))
      }
    } catch {
      // silent - user can retry
    } finally {
      setSchemaLoading((p) => ({ ...p, [name]: false }))
    }
  }, [schemaKeys])

  // -- Key expansion: fetch field values --
  const toggleKey = useCallback(async (schemaName, kv, schema) => {
    const id = keyId(schemaName, kv)
    setExpandedKeys((prev) => {
      const next = new Set(prev)
      if (next.has(id)) next.delete(id); else next.add(id)
      return next
    })

    if (!keyRecords[id] && !keyLoading[id]) {
      setKeyLoading((p) => ({ ...p, [id]: true }))
      try {
        const fields = getFieldNames(schema)
        const filter = buildFilter(kv)
        const query = { schema_name: schemaName, fields }
        if (filter) query.filter = filter
        const res = await mutationClient.executeQuery(query)
        if (res.success) {
          const arr = Array.isArray(res.data?.results) ? res.data.results : []
          const match = arr.find((x) => {
            return String(x?.key?.hash || '') === String(kv?.hash || '') &&
                   String(x?.key?.range || '') === String(kv?.range || '')
          }) || arr[0]
          setKeyRecords((p) => ({ ...p, [id]: { fields: match?.fields || {}, metadata: match?.metadata || {} } }))
        } else {
          setKeyRecords((p) => ({ ...p, [id]: { fields: {}, metadata: {} } }))
        }
      } catch { /* show empty fields on error - user can re-expand */
        setKeyRecords((p) => ({ ...p, [id]: { fields: {}, metadata: {} } }))
      } finally {
        setKeyLoading((p) => ({ ...p, [id]: false }))
      }
    }
  }, [keyRecords, keyLoading, getFieldNames])

  if (schemaList.length === 0) {
    return (
      <div className="text-secondary text-sm py-6 text-center">
        No schemas loaded. Ingest some data first.
      </div>
    )
  }

  return (
    <div className="space-y-1">
      {schemaList.map((schema) => {
        const name = schema.name
        const isOpen = expandedSchemas.has(name)
        const data = schemaKeys[name]
        const loading = schemaLoading[name]
        const error = schemaErrors[name]

        return (
          <div key={name} className="border border-border">
            {/* Schema row */}
            <button
              type="button"
              className="w-full flex items-center gap-2 px-3 py-2 text-left bg-surface hover:bg-surface-secondary transition-colors"
              onClick={() => toggleSchema(name)}
            >
              <span className="text-xs text-secondary">{isOpen ? '▾' : '▸'}</span>
              <span className="font-mono text-sm text-primary font-medium">{schema.descriptive_name || name}</span>
              {schema.descriptive_name && schema.descriptive_name !== name && (
                <span className="text-xs text-tertiary" title={name}>({name.length > 16 ? name.slice(0, 12) + '…' : name})</span>
              )}
              <span className="text-xs text-tertiary">({fieldCount(schema)} fields)</span>
              <StateBadge state={schema.state || 'available'} />
            </button>

            {/* Keys list */}
            {isOpen && (
              <div className="pl-6 pr-3 pb-2 bg-surface-secondary">
                {loading && !data && (
                  <div className="text-xs text-secondary py-2">Loading keys...</div>
                )}
                {error && (
                  <div className="text-xs text-gruvbox-red py-1">{error}</div>
                )}
                {data && data.keys.length === 0 && (
                  <div className="text-xs text-secondary py-2 italic">No keys found</div>
                )}
                {data && data.keys.map((kv) => {
                  const id = keyId(name, kv)
                  const isKeyOpen = expandedKeys.has(id)
                  const record = keyRecords[id]
                  const kLoading = keyLoading[id]

                  return (
                    <div key={id} className="border-b border-border last:border-b-0">
                      <button
                        type="button"
                        className="w-full flex items-center gap-2 px-2 py-1.5 text-left hover:bg-surface transition-colors"
                        onClick={() => toggleKey(name, kv, schema)}
                      >
                        <span className="text-xs text-secondary">{isKeyOpen ? '▾' : '▸'}</span>
                        <span className="text-xs font-mono text-primary">{keyLabel(kv)}</span>
                        {kLoading && <span className="text-xs text-secondary">(loading...)</span>}
                      </button>

                      {isKeyOpen && (
                        <div className="pl-6 pb-2">
                          {record ? (
                            <Fragment>
                              <RecordMetadata metadata={record.metadata} />
                              <FieldsTable fields={record.fields} />
                            </Fragment>
                          ) : (
                            <div className="text-xs text-secondary italic">Loading...</div>
                          )}
                        </div>
                      )}
                    </div>
                  )
                })}

                {/* Show more */}
                {data && data.keys.length < data.total_count && (
                  <div className="pt-2">
                    <button
                      type="button"
                      className="btn-secondary btn-sm"
                      onClick={() => loadMoreKeys(name)}
                      disabled={loading}
                    >
                      {loading ? 'Loading...' : `Show more keys (${data.keys.length}/${data.total_count})`}
                    </button>
                  </div>
                )}
              </div>
            )}
          </div>
        )
      })}
    </div>
  )
}

function buildFilter(kv) {
  const h = kv?.hash
  const r = kv?.range
  if (h && r) return createHashRangeKeyFilter(h, r)
  if (h) return createHashKeyFilter(h)
  if (r) return createRangeKeyFilter(r)
  return undefined
}
