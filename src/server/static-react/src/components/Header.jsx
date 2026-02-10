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
    <header className="minimal-header flex-shrink-0">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-6">
          <a href="/" className="minimal-logo">
            FoldDB
          </a>
          <HeaderProgress />
          {ingestionStatus && (
            <div className="minimal-progress-pill" title={aiReady ? `${ingestionStatus.provider} · ${ingestionStatus.model}` : 'AI not configured — open Settings'}>
              <div className={`minimal-progress-dot ${aiReady ? 'minimal-progress-dot-success' : 'minimal-progress-dot-error'}`} />
              <span className={`text-xs font-mono ${aiReady ? 'text-success' : 'text-error'}`}>
                {aiReady ? `AI · ${ingestionStatus.provider}` : 'AI off'}
              </span>
            </div>
          )}
        </div>
        <div className="flex items-center gap-4">
          <div className="minimal-status">
            <span className="minimal-status-dot"></span>
            Connected
          </div>
          {isAuthenticated && (
            <div className="flex items-center gap-4">
              <span className="minimal-user-id text-sm">
                {user?.id?.length > 12 ? `${user.id.slice(0, 8)}…` : user?.id}
              </span>
              <button
                onClick={handleLogout}
                className="minimal-header-link text-sm"
              >
                logout
              </button>
            </div>
          )}
          <button
            onClick={onSettingsClick}
            className="minimal-btn-secondary text-sm"
            title="Settings"
          >
            Settings
          </button>
        </div>
      </div>
    </header>
  )
}

export default Header
