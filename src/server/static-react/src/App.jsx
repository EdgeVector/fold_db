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
import SmartFolderTab from './components/tabs/SmartFolderTab'
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
      if (hash === 'smart-folder') return 'smart-folder';
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
        else if (hash === 'smart-folder') setActiveTab('smart-folder');
      }
    };

    window.addEventListener('hashchange', handleHashChange);
    // Check initial hash
    handleHashChange();

    return () => window.removeEventListener('hashchange', handleHashChange);
  }, [activeTab]);

  // Redux state and dispatch
  const dispatch = useAppDispatch()
  const authState = useAppSelector(state => state.auth)
  const { isAuthenticated, systemPublicKey: _systemPublicKey, systemKeyId: _systemKeyId, isLoading: _isLoading, error: _error } = authState

  // Restore session on mount FIRST - this must run before other effects
  useEffect(() => {
    const userId = localStorage.getItem('fold_user_id')
    const userHash = localStorage.getItem('fold_user_hash')
    if (userId && userHash) {
      dispatch(restoreSession({ id: userId, hash: userHash }))
    }
  }, [dispatch])

  // Initialize system key ONLY after authenticated
  useEffect(() => {
    if (isAuthenticated) {
      dispatch(initializeSystemKey())
    }
  }, [dispatch, isAuthenticated])

  // Fetch node private key ONLY after authenticated
  useEffect(() => {
    if (isAuthenticated) {
      dispatch(fetchNodePrivateKey())
    }
  }, [dispatch, isAuthenticated])

  // Use the new useApprovedSchemas hook (TASK-001)
  // Only fetch schemas when authenticated
  const {
    approvedSchemas: _approvedSchemas,
    allSchemas: _allSchemas,
    isLoading: schemasLoading,
    error: schemasError,
    refetch: refetchSchemas
  } = useApprovedSchemas({ enabled: isAuthenticated })

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
      case 'smart-folder':
        return <SmartFolderTab onResult={handleOperationResult} />
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
      <div style={{
        height: '100vh',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        background: '#fafafa'
      }}>
        <div style={{ textAlign: 'center' }}>
          <div style={{
            width: '24px',
            height: '24px',
            border: '2px solid #e5e5e5',
            borderTopColor: '#111',
            borderRadius: '50%',
            animation: 'spin 0.8s linear infinite',
            margin: '0 auto 16px'
          }}></div>
          <p style={{ color: '#666', fontSize: '14px' }}>
            Loading...
          </p>
        </div>
      </div>
    );
  }

  return (
    <div style={{
      height: '100vh',
      display: 'flex',
      flexDirection: 'column',
      background: '#fafafa',
      overflow: 'hidden'
    }}>
      <Header onSettingsClick={() => setIsSettingsOpen(true)} />
      <SettingsModal isOpen={isSettingsOpen} onClose={() => setIsSettingsOpen(false)} />

      <div style={{ display: 'flex', flex: 1, overflow: 'hidden' }}>
        <main style={{ flex: 1, overflowY: 'auto' }}>
          {/* Tab Navigation */}
          <TabNavigation
            activeTab={activeTab}
            onTabChange={handleTabChange}
          />

          <div style={{ maxWidth: '900px', margin: '0 auto', padding: '48px 40px' }}>
            {/* Schema Loading/Error States */}
            {schemasError && (
              <div style={{
                marginBottom: '24px',
                padding: '16px 20px',
                background: '#fff',
                border: '1px solid #fecaca',
                borderLeftWidth: '3px',
                borderLeftColor: '#ef4444'
              }}>
                <p style={{ color: '#ef4444', fontSize: '14px', margin: 0 }}>
                  {schemasError}
                </p>
              </div>
            )}

            {schemasLoading && (
              <div style={{
                marginBottom: '24px',
                padding: '16px 20px',
                background: '#fff',
                border: '1px solid #e5e5e5'
              }}>
                <p style={{ color: '#666', fontSize: '14px', margin: 0 }}>
                  Loading schemas...
                </p>
              </div>
            )}

            {/* Section Title */}
            <div style={{
              fontSize: '11px',
              textTransform: 'uppercase',
              letterSpacing: '2px',
              color: '#999',
              marginBottom: '24px'
            }}>
              {activeTab.replace('-', ' ')}
            </div>

            {/* Tab Content */}
            <div style={{
              background: '#fff',
              border: '1px solid #e5e5e5',
              padding: '32px'
            }}>
              {renderActiveTab()}
            </div>

            {/* Results */}
            {results && (
              <div style={{ marginTop: '48px' }}>
                <div style={{
                  fontSize: '11px',
                  textTransform: 'uppercase',
                  letterSpacing: '2px',
                  color: '#999',
                  marginBottom: '24px'
                }}>
                  Results
                </div>
                <ResultsSection results={results} />
              </div>
            )}
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
