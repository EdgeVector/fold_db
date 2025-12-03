import { useState } from 'react'
import { Provider } from 'react-redux'
import { store } from './store/store'
import Header from './components/Header'
import Footer from './components/Footer'
import StatusSection from './components/StatusSection'
import ResultsSection from './components/ResultsSection'
import TabNavigation from './components/TabNavigation'
import SchemaTab from './components/tabs/SchemaTab'
import QueryTab from './components/tabs/QueryTab'
import LlmQueryTab from './components/tabs/LlmQueryTab'
import MutationTab from './components/tabs/MutationTab'
import IngestionTab from './components/tabs/IngestionTab'
import FileUploadTab from './components/tabs/FileUploadTab'
import NativeIndexTab from './components/tabs/NativeIndexTab'
import SettingsModal from './components/SettingsModal'
import LogSidebar from './components/LogSidebar'
import { useApprovedSchemas } from './hooks/useApprovedSchemas.js'
import { useAppSelector, useAppDispatch } from './store/hooks'
import { initializeSystemKey, fetchNodePrivateKey } from './store/authSlice'
import { useEffect } from 'react'
import { DEFAULT_TAB } from './constants'
import { SchemaServiceConfigProvider } from './contexts/SchemaServiceConfigContext'

export function AppContent() {
  // Initialize activeTab from URL hash if present, otherwise use DEFAULT_TAB
  const getInitialTab = () => {
    if (typeof window !== 'undefined' && window.location.hash) {
      const hash = window.location.hash.slice(1); // Remove '#'
      // Map common hash values to tab IDs
      if (hash === 'schemas' || hash === 'schema') return 'schemas';
      if (hash === 'query') return 'query';
      if (hash === 'mutation') return 'mutation';
      if (hash === 'ingestion') return 'ingestion';
      if (hash === 'file-upload') return 'file-upload';
      if (hash === 'native-index') return 'native-index';
      if (hash === 'llm-query' || hash === 'ai-query') return 'llm-query';
    }
    return DEFAULT_TAB;
  };
  
  const [activeTab, setActiveTab] = useState(getInitialTab())
  const [isSettingsOpen, setIsSettingsOpen] = useState(false)
  const [results, setResults] = useState(null)
  
  // Sync activeTab with URL hash changes
  useEffect(() => {
    const handleHashChange = () => {
      const hash = window.location.hash.slice(1);
      if (hash && hash !== activeTab) {
        // Map hash to tab ID
        if (hash === 'schemas' || hash === 'schema') setActiveTab('schemas');
        else if (hash === 'query') setActiveTab('query');
        else if (hash === 'mutation') setActiveTab('mutation');
        else if (hash === 'ingestion') setActiveTab('ingestion');
        else if (hash === 'file-upload') setActiveTab('file-upload');
        else if (hash === 'native-index') setActiveTab('native-index');
        else if (hash === 'llm-query' || hash === 'ai-query') setActiveTab('llm-query');
      }
    };
    
    window.addEventListener('hashchange', handleHashChange);
    // Check initial hash
    handleHashChange();
    
    return () => window.removeEventListener('hashchange', handleHashChange);
  }, [activeTab]);
  
  // Use the new useApprovedSchemas hook (TASK-001)
  const {
    approvedSchemas: _approvedSchemas,
    allSchemas: _allSchemas,
    isLoading: schemasLoading,
    error: schemasError,
    refetch: refetchSchemas
  } = useApprovedSchemas()
  
  // Redux state and dispatch
  const dispatch = useAppDispatch()
  const authState = useAppSelector(state => state.auth)
  const { isAuthenticated: _isAuthenticated, systemPublicKey: _systemPublicKey, systemKeyId: _systemKeyId, isLoading: _isLoading, error: _error } = authState
  

  // Initialize system key on mount
  useEffect(() => {
    dispatch(initializeSystemKey())
  }, [dispatch])

  // Fetch node private key on mount
  useEffect(() => {
    dispatch(fetchNodePrivateKey())
  }, [dispatch])

  const handleTabChange = (tab) => {
    setActiveTab(tab)
    setResults(null)
    // Update URL hash to match active tab
    if (typeof window !== 'undefined') {
      window.location.hash = tab;
    }
  }

  const handleOperationResult = (result) => {
    setResults(result)
  }

  const handleSchemaUpdated = () => {
    // Use the hook's refetch method instead of manual fetchSchemas (TASK-001)
    refetchSchemas()
  }

  const renderActiveTab = () => {
    switch (activeTab) {
      case 'schemas':
        return (
          <SchemaTab
            onResult={handleOperationResult}
            onSchemaUpdated={handleSchemaUpdated}
          />
        )
      case 'query':
        return <QueryTab onResult={handleOperationResult} />
      case 'llm-query':
        return <LlmQueryTab onResult={handleOperationResult} />
      case 'mutation':
        return (
          <div className="tab-content">
            <MutationTab
              onResult={handleOperationResult}
            />
          </div>
        )
      case 'ingestion':
        return <IngestionTab onResult={handleOperationResult} />
      case 'file-upload':
        return <FileUploadTab onResult={handleOperationResult} />
      case 'native-index':
        return <NativeIndexTab onResult={handleOperationResult} />
      default:
        return null
    }
  }

  return (
    <div className="h-screen flex flex-col bg-gray-50 overflow-hidden">
      <Header onSettingsClick={() => setIsSettingsOpen(true)} />
      <SettingsModal isOpen={isSettingsOpen} onClose={() => setIsSettingsOpen(false)} />
      
      <div className="flex flex-1 overflow-hidden">
        <main className="flex-1 overflow-y-auto">
          <div className="container mx-auto px-4 py-6">
            <StatusSection />

            <div className="mt-6">
            {/* Schema Loading/Error States */}
            {schemasError && (
              <div className="mb-4 p-4 bg-red-50 border border-red-200 rounded-lg">
                <div className="flex items-center">
                  <div className="flex-shrink-0">
                    <svg className="h-5 w-5 text-red-400" viewBox="0 0 20 20" fill="currentColor">
                      <path fillRule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zM8.707 7.293a1 1 0 00-1.414 1.414L8.586 10l-1.293 1.293a1 1 0 101.414 1.414L10 11.414l1.293 1.293a1 1 0 001.414-1.414L11.414 10l1.293-1.293a1 1 0 00-1.414-1.414L10 8.586 8.707 7.293z" clipRule="evenodd" />
                    </svg>
                  </div>
                  <div className="ml-3">
                    <h3 className="text-sm font-medium text-red-800">
                      Schema Loading Error
                    </h3>
                    <div className="mt-2 text-sm text-red-700">
                      <p>{schemasError}</p>
                    </div>
                  </div>
                </div>
              </div>
            )}

            {schemasLoading && (
              <div className="mb-4 p-4 bg-blue-50 border border-blue-200 rounded-lg">
                <div className="flex items-center">
                  <div className="flex-shrink-0">
                    <svg className="animate-spin h-5 w-5 text-blue-400" viewBox="0 0 24 24">
                      <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" fill="none" />
                      <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" />
                    </svg>
                  </div>
                  <div className="ml-3">
                    <h3 className="text-sm font-medium text-blue-800">
                      Loading Schemas...
                    </h3>
                    <div className="mt-2 text-sm text-blue-700">
                      <p>Fetching schema information from the server.</p>
                    </div>
                  </div>
                </div>
              </div>
            )}

            {/* Tab Navigation Component (TASK-002) */}
            <TabNavigation
              activeTab={activeTab}
              onTabChange={handleTabChange}
            />

              <div className="mt-4">
                {renderActiveTab()}
              </div>
            </div>

            {results && <ResultsSection results={results} />}
          </div>
        </main>
        
        <LogSidebar />
      </div>
      
      <Footer />
    </div>
  )
}

function App() {
  return (
    <Provider store={store}>
      <SchemaServiceConfigProvider>
        <AppContent />
      </SchemaServiceConfigProvider>
    </Provider>
  )
}

export default App
