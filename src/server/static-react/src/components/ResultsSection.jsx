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
    <div className="card-terminal mt-6">
      <div className="card-terminal-header">
        <div className="flex items-center gap-3">
          <span className={`${isError ? 'text-terminal-red' : 'text-terminal-green'}`}>
            {isError ? '✖' : '✔'}
          </span>
          <span className={`font-medium ${isError ? 'text-terminal-red' : 'text-terminal-green'}`}>
            {isError ? 'ERROR' : 'OUTPUT'}
          </span>
          <span className="text-xs text-terminal-dim">
            [{typeof results === 'string' ? 'text' : structured ? 'structured' : 'json'}]
          </span>
          {results.status && (
            <span className={`badge-terminal ${
              results.status >= 400
                ? 'badge-terminal-error'
                : 'badge-terminal-success'
            }`}>
              status: {results.status}
            </span>
          )}
        </div>
        {!isError && typeof results !== 'string' && (
          <button
            type="button"
            className="btn-terminal text-xs py-1 px-3"
            onClick={() => setStructured((v) => !v)}
          >
            {structured ? '$ view --json' : '$ view --structured'}
          </button>
        )}
      </div>
      
      <div className="card-terminal-body">
        {isError && (
          <div className="mb-4 p-4 bg-terminal border-l-4 border-terminal-red rounded">
            <div className="flex items-start gap-3">
              <span className="text-terminal-red text-lg">!</span>
              <div>
                <h4 className="text-sm font-medium text-terminal-red mb-1">
                  Execution Failed
                </h4>
                <p className="text-sm text-terminal-dim">
                  <span className="text-terminal-red">→</span> {results.error || 'An unknown error occurred'}
                </p>
              </div>
            </div>
          </div>
        )}
        
        {structured && !isError && typeof results !== 'string' ? (
          <div className="rounded bg-terminal border border-terminal overflow-auto max-h-[500px] p-4">
            <StructuredResults results={results} />
          </div>
        ) : (
          <div className={`code-block overflow-auto max-h-[500px]`}>
            <div className="code-block-header">
              <span>{isError ? 'error.log' : 'output.json'}</span>
              <span className="text-terminal-dim">
                {new Date().toLocaleTimeString()}
              </span>
            </div>
            <div className="code-block-body">
              <pre className={`terminal-output ${isError ? 'output-error' : 'output-success'}`}>
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
