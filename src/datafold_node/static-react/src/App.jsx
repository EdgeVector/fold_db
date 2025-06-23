import { useState, useEffect } from 'react'
import { Provider } from 'react-redux'
import { store } from './store/store'
import Header from './components/Header'
import Footer from './components/Footer'
import StatusSection from './components/StatusSection'
import ResultsSection from './components/ResultsSection'
import SchemaTab from './components/tabs/SchemaTab'
import QueryTab from './components/tabs/QueryTab'
import MutationTab from './components/tabs/MutationTab'
import TransformsTab from './components/tabs/TransformsTab'
import SchemaDependenciesTab from './components/tabs/SchemaDependenciesTab'
import IngestionTab from './components/tabs/IngestionTab'
import KeyManagementTab from './components/tabs/KeyManagementTab'
import LogSidebar from './components/LogSidebar'
import { useKeyGeneration } from './hooks/useKeyGeneration'
import { useAppSelector, useAppDispatch } from './store/hooks'
import { initializeSystemKey } from './store/authSlice'

export function AppContent() {
  const [activeTab, setActiveTab] = useState('keys') // Default to keys tab
  const [results, setResults] = useState(null)
  const [schemas, setSchemas] = useState([])
  const keyGenerationResult = useKeyGeneration()
  
  // Redux state and dispatch
  const dispatch = useAppDispatch()
  const authState = useAppSelector(state => state.auth)
  const { isAuthenticated, systemPublicKey: _systemPublicKey, systemKeyId: _systemKeyId, isLoading: _isLoading, error: _error } = authState
  

  // Initialize system key on mount
  useEffect(() => {
    dispatch(initializeSystemKey())
  }, [dispatch])


  useEffect(() => {
    fetchSchemas()
  }, [])

  const fetchSchemas = async () => {
    try {
      const response = await fetch('/api/schemas')
      const data = await response.json()
      console.log('Schemas API response:', data)
      
      // Convert the state map to an array of schema objects with states
      const schemasWithStates = Object.entries(data.data || {}).map(([name, state]) => ({
        name,
        state,
        fields: {} // Will be populated below for approved schemas
      }))
      
      // Filter for approved schemas only (these are the ones available for mutations)
      const approvedSchemas = schemasWithStates.filter(
        (s) => s.state && s.state.toLowerCase() === 'approved'
      )
      console.log('Approved schemas:', approvedSchemas)
      
      // Fetch detailed schema information for approved schemas
      const schemasWithDetails = await Promise.all(
        approvedSchemas.map(async (schema) => {
          try {
            console.log(`Fetching details for schema: ${schema.name}`)
            const schemaResponse = await fetch(`/api/schema/${schema.name}`)
            console.log(`Schema ${schema.name} response status:`, schemaResponse.status)
            if (schemaResponse.ok) {
              const schemaData = await schemaResponse.json()
              console.log(`Schema ${schema.name} data:`, schemaData)
              return {
                ...schema,
                ...schemaData, // Include the full schema data including schema_type
                fields: schemaData.fields || {}
              }
            } else {
              console.error(`Failed to fetch schema ${schema.name}: ${schemaResponse.status}`)
            }
          } catch (err) {
            console.error(`Failed to fetch details for schema ${schema.name}:`, err)
          }
          return schema // Return original if fetch fails
        })
      )
      
      console.log('Final schemas with details:', schemasWithDetails)
      setSchemas(schemasWithDetails)
    } catch (error) {
      console.error('Failed to fetch schemas:', error)
    }
  }

  const handleTabChange = (tab) => {
    // If not authenticated, only allow Keys tab
    if (!isAuthenticated && tab !== 'keys') {
      return
    }
    setActiveTab(tab)
    setResults(null)
  }

  const handleOperationResult = (result) => {
    setResults(result)
  }

  const handleSchemaUpdated = () => {
    fetchSchemas()
  }

  const renderActiveTab = () => {
    switch (activeTab) {
      case 'schemas':
        return (
          <SchemaTab
            schemas={schemas}
            onResult={handleOperationResult}
            onSchemaUpdated={handleSchemaUpdated}
          />
        )
      case 'query':
        return <QueryTab schemas={schemas} onResult={handleOperationResult} />
      case 'mutation':
        return (
          <div className="tab-content">
            <MutationTab
              schemas={schemas}
              onResult={handleOperationResult}
            />
          </div>
        )
      case 'ingestion':
        return <IngestionTab onResult={handleOperationResult} />
      case 'transforms':
        return <TransformsTab schemas={schemas} onResult={handleOperationResult} />
      case 'dependencies':
        return <SchemaDependenciesTab schemas={schemas} />
      case 'keys':
        return (
          <KeyManagementTab
            onResult={handleOperationResult}
            keyGenerationResult={keyGenerationResult}
          />
        )
      default:
        return null
    }
  }

  return (
    <div className="min-h-screen flex bg-gray-50">
      <div className="flex flex-col flex-1">
        <Header />
        <main className="container mx-auto px-4 py-6 flex-1">
          <StatusSection />

          <div className="mt-6">
            {/* Authentication Warning */}
            {!isAuthenticated && (
              <div className="mb-4 p-4 bg-yellow-50 border border-yellow-200 rounded-lg">
                <div className="flex items-center">
                  <div className="flex-shrink-0">
                    <svg className="h-5 w-5 text-yellow-400" viewBox="0 0 20 20" fill="currentColor">
                      <path fillRule="evenodd" d="M8.257 3.099c.765-1.36 2.722-1.36 3.486 0l5.58 9.92c.75 1.334-.213 2.98-1.742 2.98H4.42c-1.53 0-2.493-1.646-1.743-2.98l5.58-9.92zM11 13a1 1 0 11-2 0 1 1 0 012 0zm-1-8a1 1 0 00-1 1v3a1 1 0 002 0V6a1 1 0 00-1-1z" clipRule="evenodd" />
                    </svg>
                  </div>
                  <div className="ml-3">
                    <h3 className="text-sm font-medium text-yellow-800">
                      Authentication Required
                    </h3>
                    <div className="mt-2 text-sm text-yellow-700">
                      <p>Please set up your private key in the Keys tab to access other features.</p>
                    </div>
                  </div>
                </div>
              </div>
            )}

            <div className="flex border-b border-gray-200">
              <button
              className={`px-4 py-2 text-sm font-medium ${
                activeTab === 'schemas'
                  ? 'text-primary border-b-2 border-primary'
                  : isAuthenticated
                    ? 'text-gray-500 hover:text-gray-700 hover:border-gray-300'
                    : 'text-gray-300 cursor-not-allowed'
              }`}
              onClick={() => handleTabChange('schemas')}
              disabled={!isAuthenticated}
            >
              Schemas
              {!isAuthenticated && <span className="ml-1 text-xs">🔒</span>}
            </button>
            <button
              className={`px-4 py-2 text-sm font-medium ${
                activeTab === 'query'
                  ? 'text-primary border-b-2 border-primary'
                  : isAuthenticated
                    ? 'text-gray-500 hover:text-gray-700 hover:border-gray-300'
                    : 'text-gray-300 cursor-not-allowed'
              }`}
              onClick={() => handleTabChange('query')}
              disabled={!isAuthenticated}
            >
              Query
              {!isAuthenticated && <span className="ml-1 text-xs">🔒</span>}
            </button>
            <button
              className={`px-4 py-2 text-sm font-medium ${
                activeTab === 'mutation'
                  ? 'text-primary border-b-2 border-primary'
                  : isAuthenticated
                    ? 'text-gray-500 hover:text-gray-700 hover:border-gray-300'
                    : 'text-gray-300 cursor-not-allowed'
              }`}
              onClick={() => handleTabChange('mutation')}
              disabled={!isAuthenticated}
            >
              Mutation
              {!isAuthenticated && <span className="ml-1 text-xs">🔒</span>}
            </button>
            <button
              className={`px-4 py-2 text-sm font-medium ${
                activeTab === 'ingestion'
                  ? 'text-primary border-b-2 border-primary'
                  : isAuthenticated
                    ? 'text-gray-500 hover:text-gray-700 hover:border-gray-300'
                    : 'text-gray-300 cursor-not-allowed'
              }`}
              onClick={() => handleTabChange('ingestion')}
              disabled={!isAuthenticated}
            >
              Ingestion
              {!isAuthenticated && <span className="ml-1 text-xs">🔒</span>}
            </button>
            <button
              className={`px-4 py-2 text-sm font-medium ${
                activeTab === 'transforms'
                  ? 'text-primary border-b-2 border-primary'
                  : isAuthenticated
                    ? 'text-gray-500 hover:text-gray-700 hover:border-gray-300'
                    : 'text-gray-300 cursor-not-allowed'
              }`}
              onClick={() => handleTabChange('transforms')}
              disabled={!isAuthenticated}
            >
              Transforms
              {!isAuthenticated && <span className="ml-1 text-xs">🔒</span>}
            </button>
            <button
              className={`px-4 py-2 text-sm font-medium ${
                activeTab === 'dependencies'
                  ? 'text-primary border-b-2 border-primary'
                  : isAuthenticated
                    ? 'text-gray-500 hover:text-gray-700 hover:border-gray-300'
                    : 'text-gray-300 cursor-not-allowed'
              }`}
              onClick={() => handleTabChange('dependencies')}
              disabled={!isAuthenticated}
            >
              Dependencies
              {!isAuthenticated && <span className="ml-1 text-xs">🔒</span>}
            </button>
            <button
              className={`px-4 py-2 text-sm font-medium ${
                activeTab === 'keys'
                  ? 'text-primary border-b-2 border-primary'
                  : 'text-gray-500 hover:text-gray-700 hover:border-gray-300'
              }`}
              onClick={() => handleTabChange('keys')}
            >
              Keys
              {isAuthenticated && <span className="ml-1 text-xs">✓</span>}
            </button>
            </div>

            <div className="mt-4">
              {renderActiveTab()}
            </div>
          </div>

          {results && <ResultsSection results={results} />}
        </main>
        <Footer />
      </div>
      <LogSidebar />
    </div>
  )
}

function App() {
  return (
    <Provider store={store}>
      <AppContent />
    </Provider>
  )
}

export default App
