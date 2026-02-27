import { useEffect, useMemo, useState } from 'react'
import StructuredResults from './StructuredResults'
import { isHashRangeFieldsShape } from '../utils/hashRangeResults'

function ResultsSection({ results }) {
  const hasResults = results != null
  const isError = hasResults && (Boolean(results.error) || (results.status && results.status >= 400))
  const hasData = hasResults && results.data !== undefined
  const defaultStructured = useMemo(() => hasResults && !isError && isHashRangeFieldsShape(hasData ? results.data : results), [hasResults, results, isError, hasData])
  const [structured, setStructured] = useState(defaultStructured)
  useEffect(() => { setStructured(defaultStructured) }, [defaultStructured])

  if (!hasResults) return null

  return (
    <div className="mt-6 card">
      <div className="flex items-center justify-between px-4 py-3 border-b border-border bg-surface-secondary">
        <div className="flex items-center gap-3">
          <span className={isError ? 'text-gruvbox-red' : ''}>{isError ? '✖' : '✔'}</span>
          <span className={`font-medium ${isError ? 'text-gruvbox-red' : ''}`}>
            {isError ? 'ERROR' : 'OUTPUT'}
          </span>
          <span className="text-xs text-secondary">
            [{typeof results === 'string' ? 'text' : structured ? 'structured' : 'json'}]
          </span>
          {results.status && (
            <span className={`badge ${results.status >= 400 ? 'badge-error' : 'badge-success'}`}>
              status: {results.status}
            </span>
          )}
        </div>
        {!isError && typeof results !== 'string' && (
          <button className="btn-secondary btn-sm" onClick={() => setStructured(v => !v)}>
            {structured ? 'view json' : 'view structured'}
          </button>
        )}
      </div>

      <div className="p-4">
        {isError && (
          <div className="mb-4 p-4 card card-error">
            <div className="flex items-start gap-3">
              <span className="text-gruvbox-red text-lg">!</span>
              <div>
                <h4 className="text-sm font-medium text-gruvbox-red mb-1">Execution Failed</h4>
                <p className="text-sm text-secondary">
                  <span className="text-gruvbox-red">→</span> {results.error || 'An unknown error occurred'}
                </p>
              </div>
            </div>
          </div>
        )}

        {structured && !isError && typeof results !== 'string' ? (
          <div className="overflow-auto max-h-[500px]">
            <StructuredResults results={results} />
          </div>
        ) : (
          <div className="overflow-auto max-h-[500px]">
            <pre className={`text-sm font-mono whitespace-pre-wrap ${isError ? 'text-gruvbox-red' : ''}`}>
              {typeof results === 'string' ? results : JSON.stringify(hasData ? results.data : results, null, 2)}
            </pre>
          </div>
        )}
      </div>
    </div>
  )
}

export default ResultsSection
