import { useMemo, useState } from 'react'
import StructuredResults from './StructuredResults'
import { isHashRangeFieldsShape } from '../utils/hashRangeResults'

function ResultsSection({ results }) {
  const hasResults = results != null
  const isError = hasResults && (Boolean(results.error) || (results.status && results.status >= 400))
  const hasData = hasResults && results.data !== undefined
  const defaultStructured = useMemo(() => hasResults && !isError && isHashRangeFieldsShape(hasData ? results.data : results), [hasResults, results, isError, hasData])
  const [structured, setStructured] = useState(defaultStructured)

  if (!hasResults) {
    return null
  }

  return (
    <div className="minimal-card mt-6">
      <div className="minimal-card-header">
        <div className="flex items-center gap-3">
          <span className={`${isError ? 'text-error' : 'text-success'}`}>
            {isError ? '✖' : '✔'}
          </span>
          <span className={`font-medium ${isError ? 'text-error' : 'text-success'}`}>
            {isError ? 'ERROR' : 'OUTPUT'}
          </span>
          <span className="text-xs text-secondary">
            [{typeof results === 'string' ? 'text' : structured ? 'structured' : 'json'}]
          </span>
          {results.status && (
            <span className={`minimal-badge ${
              results.status >= 400
                ? 'minimal-badge-error'
                : 'minimal-badge-success'
            }`}>
              status: {results.status}
            </span>
          )}
        </div>
        {!isError && typeof results !== 'string' && (
          <button
            type="button"
            className="minimal-btn-secondary minimal-btn-sm"
            onClick={() => setStructured((v) => !v)}
          >
            {structured ? 'view json' : 'view structured'}
          </button>
        )}
      </div>
      
      <div className="minimal-card-body">
        {isError && (
          <div className="minimal-error-block mb-4">
            <div className="flex items-start gap-3">
              <span className="text-error text-lg">!</span>
              <div>
                <h4 className="text-sm font-medium text-error mb-1">
                  Execution Failed
                </h4>
                <p className="text-sm text-secondary">
                  <span className="text-error">→</span> {results.error || 'An unknown error occurred'}
                </p>
              </div>
            </div>
          </div>
        )}
        
        {structured && !isError && typeof results !== 'string' ? (
          <div className="overflow-auto max-h-[500px] p-4 border border-solid" style={{ borderColor: 'var(--color-border)' }}>
            <StructuredResults results={results} />
          </div>
        ) : (
          <div className="minimal-minimal-code-block overflow-auto max-h-[500px]">
            <div className="minimal-code-header">
              <span>{isError ? 'error.log' : 'output.json'}</span>
              <span className="text-tertiary">
                {new Date().toLocaleTimeString()}
              </span>
            </div>
            <div className="minimal-code-body">
              <pre className={`${isError ? 'text-error' : 'text-success'}`}>
                {typeof results === 'string'
                  ? results
                  : JSON.stringify(hasData ? results.data : results, null, 2)
                }
              </pre>
            </div>
          </div>
        )}
      </div>
    </div>
  )
}

export default ResultsSection
