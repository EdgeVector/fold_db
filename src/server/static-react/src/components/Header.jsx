import { useAppDispatch, useAppSelector } from '../store/hooks'
import { logoutUser } from '../store/authSlice'
import { useIngestionStatus } from '../hooks/useIngestionStatus'
import HeaderProgress from './HeaderProgress'

function Header({ onSettingsClick }) {
  const dispatch = useAppDispatch()
  const { isAuthenticated, user } = useAppSelector(state => state.auth)
  const { ingestionStatus } = useIngestionStatus()

  const handleLogout = () => {
    dispatch(logoutUser())
    localStorage.removeItem('fold_user_id')
    localStorage.removeItem('fold_user_hash')
  }

  const aiReady = ingestionStatus?.enabled && ingestionStatus?.configured

  return (
    <header className="bg-surface border-b border-border px-8 py-3 flex-shrink-0">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-6">
          <a href="/" className="text-lg font-medium tracking-tight text-primary no-underline hover:text-primary">
            FoldDB
          </a>
          <HeaderProgress />
          {ingestionStatus && (
            <div
              className="flex items-center gap-2 px-3 py-1.5 bg-surface-secondary border border-border"
              title={aiReady ? `${ingestionStatus.provider} · ${ingestionStatus.model}` : 'AI not configured — open Settings'}
            >
              <div className={`w-2 h-2 rounded-full animate-pulse ${aiReady ? 'bg-green-600' : 'bg-red-600'}`} />
              <span className={`text-xs font-mono ${aiReady ? 'text-green-600' : 'text-red-600'}`}>
                {aiReady ? `AI · ${ingestionStatus.provider}` : 'AI off'}
              </span>
            </div>
          )}
        </div>
        <div className="flex items-center gap-4">
          <div className="flex items-center gap-2 text-sm text-secondary">
            <span className="w-2 h-2 bg-green-600 rounded-full" />
            Connected
          </div>
          {isAuthenticated && (
            <div className="flex items-center gap-4">
              <span className="text-secondary text-sm">
                {user?.id?.length > 12 ? `${user.id.slice(0, 8)}…` : user?.id}
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
