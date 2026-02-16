import { useState, useEffect } from 'react'
import { useAppDispatch, useAppSelector } from '../store/hooks'
import { logoutUser } from '../store/authSlice'
import { BROWSER_CONFIG } from '../constants/config'
import { systemClient } from '../api/clients/systemClient'
import HeaderProgress from './HeaderProgress'
import AnimatedLogo from './AnimatedLogo'

function classifySchemaEnv(url) {
  if (!url) return { label: 'None', color: 'text-gruvbox-yellow' }
  if (url.includes('127.0.0.1') || url.includes('localhost')) return { label: 'Local', color: 'text-gruvbox-yellow' }
  if (url.includes('us-east-1')) return { label: 'Prod', color: 'text-gruvbox-green' }
  if (url.includes('us-west-2')) return { label: 'Dev', color: 'text-gruvbox-blue' }
  return { label: 'Custom', color: 'text-secondary' }
}

function Header({ onSettingsClick, onAiSettingsClick, ingestionStatus }) {
  const dispatch = useAppDispatch()
  const { isAuthenticated, user } = useAppSelector(state => state.auth)
  const [storageMode, setStorageMode] = useState(null)
  const [schemaEnv, setSchemaEnv] = useState(null)

  useEffect(() => {
    systemClient.getDatabaseConfig().then(res => {
      if (res.data) setStorageMode(res.data.type === 'dynamodb' ? 'Cloud' : 'Local')
    }).catch(() => {})
    systemClient.getSystemStatus().then(res => {
      if (res.data) setSchemaEnv(classifySchemaEnv(res.data.schema_service_url))
    }).catch(() => {})
  }, [])

  const handleLogout = () => {
    dispatch(logoutUser())
    localStorage.removeItem(BROWSER_CONFIG.STORAGE_KEYS.USER_ID)
    localStorage.removeItem(BROWSER_CONFIG.STORAGE_KEYS.USER_HASH)
  }

  const aiReady = ingestionStatus?.enabled && ingestionStatus?.configured

  return (
    <header className="bg-surface border-b border-border px-8 py-3 flex-shrink-0">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-6">
          <a href="/" className="flex items-center gap-2 text-lg font-medium tracking-tight text-primary no-underline hover:text-primary">
            <AnimatedLogo size={72} />
            FoldDB
          </a>
          <HeaderProgress />
        </div>
        <div className="flex items-center gap-4">
          <div className="flex items-center gap-2 text-sm text-secondary font-mono">
            <span>{storageMode || '...'}</span>
            {schemaEnv && <><span className="text-tertiary">/</span><span className={schemaEnv.color}>Schema: {schemaEnv.label}</span></>}
            {ingestionStatus && (
              <><span className="text-tertiary">/</span><button
                onClick={onAiSettingsClick}
                className={`bg-transparent border-none cursor-pointer p-0 font-mono text-sm ${aiReady ? 'text-gruvbox-green' : 'text-gruvbox-red'} hover:text-primary`}
                title={aiReady ? `${ingestionStatus.provider} · ${ingestionStatus.model}` : 'AI not configured — click to open Settings'}
              >
                {aiReady ? `AI: ${ingestionStatus.provider}` : 'AI: off'}
              </button></>
            )}
          </div>
          {isAuthenticated && (
            <div className="flex items-center gap-4">
              <span className="text-secondary text-sm">
                {user?.id}
              </span>
              <button
                onClick={handleLogout}
                className="text-tertiary text-sm bg-transparent border-none cursor-pointer hover:text-primary transition-colors"
              >
                logout
              </button>
            </div>
          )}
          <button onClick={onSettingsClick} className="btn-secondary" title="Settings">
            Settings
          </button>
        </div>
      </div>
    </header>
  )
}

export default Header
