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
    <div className="bg-white rounded-lg shadow-sm p-6 mt-6">
      <h3 className="text-lg font-semibold mb-4 flex items-center">
        <span className={`mr-2 ${isError ? 'text-red-600' : 'text-gray-900'}`}>
          {isError ? 'Error' : 'Results'}
        </span>
        <span className="text-xs font-normal text-gray-500">
          ({typeof results === 'string' ? 'Text' : structured ? 'Structured' : 'JSON'})
        </span>
        {results.status && (
          <span className={`ml-2 px-2 py-1 text-xs rounded-full ${
            results.status >= 400
              ? 'bg-red-100 text-red-800'
              : 'bg-green-100 text-green-800'
          }`}>
            Status: {results.status}
          </span>
        )}
        {!isError && typeof results !== 'string' && (
          <div className="ml-auto">
            <button
              type="button"
              className="text-xs px-2 py-1 rounded border border-gray-300 hover:bg-gray-100"
              onClick={() => setStructured((v) => !v)}
            >
              {structured ? 'View JSON' : 'View Structured'}
            </button>
          </div>
        )}
      </h3>
      
      {isError && (
        <div className="mb-4 p-4 bg-red-50 border border-red-200 rounded-md">
          <div className="flex">
            <div className="flex-shrink-0">
              <svg className="h-5 w-5 text-red-400" viewBox="0 0 20 20" fill="currentColor">
                <path fillRule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zM8.707 7.293a1 1 0 00-1.414 1.414L8.586 10l-1.293 1.293a1 1 0 101.414 1.414L10 11.414l1.293 1.293a1 1 0 001.414-1.414L11.414 10l1.293-1.293a1 1 0 00-1.414-1.414L10 8.586 8.707 7.293z" clipRule="evenodd" />
              </svg>
            </div>
            <div className="ml-3">
              <h4 className="text-sm font-medium text-red-800">
                Query Execution Failed
              </h4>
              <div className="mt-2 text-sm text-red-700">
                <p>{results.error || 'An unknown error occurred'}</p>
              </div>
            </div>
          </div>
        </div>
      )}
      
      {structured && !isError && typeof results !== 'string' ? (
        <div className="rounded-md p-2 bg-gray-50 border overflow-auto max-h-[500px]">
          <StructuredResults results={results} />
        </div>
      ) : (
        <div className={`rounded-md p-4 overflow-auto max-h-[500px] ${
          isError ? 'bg-red-50 border border-red-200' : 'bg-gray-50'
        }`}>
          <pre className={`font-mono text-sm whitespace-pre-wrap ${
            isError ? 'text-red-700' : 'text-gray-700'
          }`}>
            {typeof results === 'string'
              ? results
              : JSON.stringify(hasData ? results.data : results, null, 2)
            }
          </pre>
        </div>
      )}
    </div>
  )
}

export default ResultsSection
