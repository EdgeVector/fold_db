/**
 * Displays a schema's descriptive name with an optional truncated hash suffix.
 *
 * Used across DataBrowserTab, IngestionReport, SchemaTab, etc.
 */
import { getSchemaDisplayName, truncateHash } from '../../utils/schemaUtils'

export default function SchemaName({ schema, name, className = 'font-mono text-sm text-primary font-medium' }) {
  const schemaName = name || schema?.name || ''
  const displayName = schema ? getSchemaDisplayName(schema) : schemaName
  const descriptive = schema?.descriptive_name
  const differs = descriptive && descriptive !== schemaName

  return (
    <>
      <span className={className}>{displayName}</span>
      {differs && (
        <span className="text-xs text-tertiary" title={schemaName}>
          ({truncateHash(schemaName)})
        </span>
      )}
    </>
  )
}
