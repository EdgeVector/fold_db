import { useState } from 'react'
import { FoldDbProvider } from './components/FoldDbProvider'
import Header from './components/Header'
import Footer from './components/Footer'
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
import { initializeSystemKey, fetchNodePrivateKey, restoreSession } from './store/authSlice'
import LoginPage from './components/LoginPage'
import { useEffect } from 'react'
import { DEFAULT_TAB } from './constants'
import { store } from './store/store'
import { injectStore } from './api/core/client'

// Inject store into ApiClient to handle circular dependency safely
injectStore(store)

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
  const { isAuthenticated, systemPublicKey: _systemPublicKey, systemKeyId: _systemKeyId, isLoading: _isLoading, error: _error } = authState

  console.log('AppContent render:', { isAuthenticated, activeTab });


  // Initialize system key on mount
  useEffect(() => {
    dispatch(initializeSystemKey())
  }, [dispatch])

  // Fetch node private key on mount
  useEffect(() => {
    dispatch(fetchNodePrivateKey())
  }, [dispatch])

  // Restore session on mount
  useEffect(() => {
    const userId = localStorage.getItem('fold_user_id')
    const userHash = localStorage.getItem('fold_user_hash')
    if (userId && userHash) {
      dispatch(restoreSession({ id: userId, hash: userHash }))
    }
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

  // Show login page if not authenticated
  if (!isAuthenticated && !_isLoading) {
    return <LoginPage />;
  }

  // Show loading spinner while restoring session or checking auth
  if (_isLoading) {
    return (
      <div className="h-screen flex items-center justify-center bg-terminal">
        <div className="text-center">
          <div className="mb-4">
            <span className="spinner-terminal w-8 h-8"></span>
          </div>
          <p className="text-terminal-green font-mono text-sm">
            <span className="text-terminal-dim">$ </span>
            initializing...
            <span className="cursor"></span>
          </p>
        </div>
      </div>
    );
  }

  return (
    <div className="h-screen flex flex-col bg-terminal overflow-hidden">
      <Header onSettingsClick={() => setIsSettingsOpen(true)} />
      <SettingsModal isOpen={isSettingsOpen} onClose={() => setIsSettingsOpen(false)} />

      <div className="flex flex-1 overflow-hidden">
        <main className="flex-1 overflow-y-auto">
          <div className="container mx-auto px-4 py-4">
            <div className="mt-4">
              {/* Schema Loading/Error States */}
              {schemasError && (
                <div className="mb-4 p-4 card-terminal border-l-4 border-terminal-red">
                  <div className="flex items-center">
                    <div className="flex-shrink-0">
                      <span className="text-terminal-red text-lg">✖</span>
                    </div>
                    <div className="ml-3">
                      <h3 className="text-sm font-medium text-terminal-red">
                        ERROR: Schema Loading Failed
                      </h3>
                      <div className="mt-2 text-sm text-terminal-dim">
                        <p><span className="text-terminal-red">→</span> {schemasError}</p>
                      </div>
                    </div>
                  </div>
                </div>
              )}

              {schemasLoading && (
                <div className="mb-4 p-4 card-terminal border-l-4 border-terminal-blue">
                  <div className="flex items-center">
                    <div className="flex-shrink-0">
                      <span className="spinner-terminal"></span>
                    </div>
                    <div className="ml-3">
                      <h3 className="text-sm font-medium text-terminal-blue">
                        Loading Schemas...
                      </h3>
                      <div className="mt-1 text-sm text-terminal-dim">
                        <p><span className="text-terminal-blue">→</span> Fetching schema data from server</p>
                      </div>
                    </div>
                  </div>
                </div>
              )}

              {/* Tab Navigation Component */}
              <TabNavigation
                activeTab={activeTab}
                onTabChange={handleTabChange}
              />

              <div className="mt-4 card-terminal">
                <div className="card-terminal-header">
                  <span className="text-terminal-green text-sm font-medium">
                    <span className="text-terminal-dim">$</span> {activeTab}
                  </span>
                  <span className="text-xs text-terminal-dim">
                    {new Date().toLocaleTimeString()}
                  </span>
                </div>
                <div className="card-terminal-body">
                  {renderActiveTab()}
                </div>
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
    <FoldDbProvider>
      <AppContent />
    </FoldDbProvider>
  )
}

export default App
