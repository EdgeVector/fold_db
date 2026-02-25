import { useCallback, useEffect, useMemo, useState, Fragment } from 'react'
import { useAppDispatch, useAppSelector } from '../store/hooks'
import { fetchSchemas, selectAllSchemas } from '../store/schemaSlice'
import { mutationClient } from '../api/clients'
import { schemaClient } from '../api/clients/schemaClient'
import { FieldsTable } from './StructuredResults'
import SchemaName from './shared/SchemaName'
import { getFieldNames as getFieldNamesUtil, toggleSetItem } from '../utils/schemaUtils'
import {
  keyId,
  keyLabel,
  StateBadge,
  getMaxVersion,
  getFirstMoleculeUuid,
  VersionBadge,
  VersionHistory,
  RecordMetadata,
  buildFilter,
} from './data-browser/shared'

export default function IngestionReport({ ingestionResult, onDismiss }) {
  const dispatch = useAppDispatch()
  const allSchemas = useAppSelector(selectAllSchemas)

  const data = ingestionResult?.data || ingestionResult
  const schemasWritten = data?.schemas_written || []
  const mutationsGenerated = data?.mutations_generated ?? 0
  const mutationsExecuted = data?.mutations_executed ?? 0
  const newSchemaCreated = data?.new_schema_created ?? false

  // Track whether the initial schema fetch has completed so we can distinguish
  // "schema still loading" from "schema no longer exists in the database".
  const [schemasLoaded, setSchemasLoaded] = useState(false)

  // Refresh schemas on mount so new ingested schemas are available
  useEffect(() => {
    dispatch(fetchSchemas({ forceRefresh: true })).finally(() => setSchemasLoaded(true))
  }, [dispatch])

  // Build a lookup of schema name -> schema object from Redux store
  const schemaLookup = useMemo(() => {
    const map = {}
    if (Array.isArray(allSchemas)) {
      for (const s of allSchemas) {
        if (s?.name) map[s.name] = s
      }
    }
    return map
  }, [allSchemas])

  // Schema-level expand state
  const [expandedSchemas, setExpandedSchemas] = useState(() => new Set())

  // Key-level expand + cached records
  const [expandedKeys, setExpandedKeys] = useState(() => new Set())
  const [keyRecords, setKeyRecords] = useState({})
  const [keyLoading, setKeyLoading] = useState({})

  // Field names cache for schemas not yet in Redux
  const [fieldNamesCache, setFieldNamesCache] = useState({})

  const getFieldNames = useCallback((schemaName) => {
    const schema = schemaLookup[schemaName]
    if (schema) return getFieldNamesUtil(schema)
    return fieldNamesCache[schemaName] || []
  }, [schemaLookup, fieldNamesCache])

  const fieldCount = useCallback((schemaName) => {
    return getFieldNames(schemaName).length
  }, [getFieldNames])

  const toggleSchema = useCallback(async (schemaName) => {
    setExpandedSchemas((prev) => toggleSetItem(prev, schemaName))

    // If we don't have field names yet, fetch the schema
    if (!schemaLookup[schemaName] && !fieldNamesCache[schemaName]) {
      try {
        const res = await schemaClient.getSchema(schemaName)
        const schema = res.data?.schema || res.data
        if (schema) {
          const f = schema.fields
          const names = Array.isArray(f) ? f : (f && typeof f === 'object') ? Object.keys(f) : []
          setFieldNamesCache((p) => ({ ...p, [schemaName]: names }))
        }
      } catch { /* will show empty fields */ }
    }
  }, [schemaLookup, fieldNamesCache])

  const toggleKey = useCallback(async (schemaName, kv) => {
    const id = keyId(schemaName, kv)
    setExpandedKeys((prev) => toggleSetItem(prev, id))

    if (!keyRecords[id] && !keyLoading[id]) {
      setKeyLoading((p) => ({ ...p, [id]: true }))
      try {
        let fields = getFieldNames(schemaName)
        if (fields.length === 0) {
          try {
            const res = await schemaClient.getSchema(schemaName)
            const schema = res.data?.schema || res.data
            if (schema) {
              const f = schema.fields
              const names = Array.isArray(f) ? f : (f && typeof f === 'object') ? Object.keys(f) : []
              setFieldNamesCache((p) => ({ ...p, [schemaName]: names }))
              fields = names
            }
          } catch { /* proceed with empty */ }
        }
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
          const notFound = !match
          setKeyRecords((p) => ({ ...p, [id]: { fields: match?.fields || {}, metadata: match?.metadata || {}, notFound } }))
        } else {
          setKeyRecords((p) => ({ ...p, [id]: { fields: {}, metadata: {}, notFound: true } }))
        }
      } catch {
        setKeyRecords((p) => ({ ...p, [id]: { fields: {}, metadata: {}, notFound: true } }))
      } finally {
        setKeyLoading((p) => ({ ...p, [id]: false }))
      }
    }
  }, [keyRecords, keyLoading, getFieldNames])

  if (schemasWritten.length === 0) {
    return null
  }

  return (
    <div className="mt-6 card">
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-3 border-b border-border bg-surface-secondary">
        <div className="flex items-center gap-3">
          <span className="text-gruvbox-green">&#10004;</span>
          <span className="font-medium">INGESTION REPORT</span>
          <span className="text-xs text-secondary">
            {mutationsGenerated} generated, {mutationsExecuted} executed
          </span>
          {newSchemaCreated && (
            <span className="inline-flex items-center px-1.5 py-0.5 text-[10px] font-semibold rounded bg-gruvbox-green/20 text-gruvbox-green">
              new
            </span>
          )}
        </div>
        {onDismiss && (
          <button
            type="button"
            className="btn-secondary btn-sm text-xs"
            onClick={onDismiss}
          >
            Dismiss
          </button>
        )}
      </div>

      {/* Schema list */}
      <div className="p-4 space-y-1">
        {schemasWritten.map((sw) => {
          const name = sw.schema_name
          const keys = sw.keys_written || []
          const isOpen = expandedSchemas.has(name)
          const schema = schemaLookup[name]

          return (
            <div key={name} className="border border-border">
              {/* Schema row */}
              <button
                type="button"
                className="w-full flex items-center gap-2 px-3 py-2 text-left bg-surface hover:bg-surface-secondary transition-colors"
                onClick={() => toggleSchema(name)}
              >
                <span className="text-xs text-secondary">{isOpen ? '▾' : '▸'}</span>
                <SchemaName schema={schema} name={name} />
                <span className="text-xs text-tertiary">
                  ({keys.length} record{keys.length !== 1 ? 's' : ''})
                </span>
                {fieldCount(name) > 0 && (
                  <span className="text-xs text-tertiary">({fieldCount(name)} fields)</span>
                )}
                {schema && <StateBadge state={schema.state || 'available'} />}
                {!schema && schemasLoaded && (
                  <span className="inline-flex items-center px-1.5 py-0.5 text-[10px] font-semibold rounded bg-gruvbox-red/20 text-gruvbox-red">
                    removed
                  </span>
                )}
              </button>

              {/* Keys list */}
              {isOpen && (
                <div className="pl-6 pr-3 pb-2 bg-surface-secondary">
                  {keys.length === 0 && (
                    <div className="text-xs text-secondary py-2 italic">No keys</div>
                  )}
                  {keys.map((kv) => {
                    const id = keyId(name, kv)
                    const isKeyOpen = expandedKeys.has(id)
                    const record = keyRecords[id]
                    const kLoading = keyLoading[id]

                    const maxVersion = record ? getMaxVersion(record.metadata) : 0
                    const molUuid = record ? getFirstMoleculeUuid(record.metadata) : null

                    return (
                      <div key={id} className="border-b border-border last:border-b-0">
                        <button
                          type="button"
                          className="w-full flex items-center gap-2 px-2 py-1.5 text-left hover:bg-surface transition-colors"
                          onClick={() => toggleKey(name, kv)}
                        >
                          <span className="text-xs text-secondary">{isKeyOpen ? '▾' : '▸'}</span>
                          <span className="text-xs font-mono text-primary">{keyLabel(kv)}</span>
                          <VersionBadge version={maxVersion} />
                          {kLoading && <span className="text-xs text-secondary">(loading...)</span>}
                        </button>

                        {isKeyOpen && (
                          <div className="pl-6 pb-2">
                            {record ? (
                              record.notFound ? (
                                <div className="text-xs text-gruvbox-red italic px-3 py-2">Record not found in database</div>
                              ) : (
                                <Fragment>
                                  <RecordMetadata metadata={record.metadata} />
                                  {maxVersion > 1 && <VersionHistory moleculeUuid={molUuid} />}
                                  <FieldsTable fields={record.fields} />
                                </Fragment>
                              )
                            ) : (
                              <div className="text-xs text-secondary italic">Loading...</div>
                            )}
                          </div>
                        )}
                      </div>
                    )
                  })}
                </div>
              )}
            </div>
          )
        })}
      </div>
    </div>
  )
}
