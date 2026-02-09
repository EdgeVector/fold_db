import { useAppDispatch, useAppSelector } from '../store/hooks'
import { logoutUser } from '../store/authSlice'
import HeaderProgress from './HeaderProgress'

function Header({ onSettingsClick }) {
  const dispatch = useAppDispatch()
  const { isAuthenticated, user } = useAppSelector(state => state.auth)

  const handleLogout = () => {
    dispatch(logoutUser())
    localStorage.removeItem('fold_user_id')
    localStorage.removeItem('fold_user_hash')
  }

  return (
    <header className="minimal-header flex-shrink-0">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-6">
          <a href="/" className="minimal-logo">
            FoldDB
          </a>
          <HeaderProgress />
        </div>
        <div className="flex items-center gap-4">
          <div className="minimal-status">
            <span className="minimal-status-dot"></span>
            Connected
          </div>
          {isAuthenticated && (
            <div className="flex items-center gap-4">
              <span className="text-sm" style={{ color: '#666' }}>
                {user?.id?.length > 12 ? `${user.id.slice(0, 8)}…` : user?.id}
              </span>
              <button
                onClick={handleLogout}
                className="text-sm transition-colors"
                style={{ color: '#999' }}
                onMouseOver={(e) => e.target.style.color = '#111'}
                onMouseOut={(e) => e.target.style.color = '#999'}
              >
                logout
              </button>
            </div>
          )}
          <button
            onClick={onSettingsClick}
            className="btn-minimal-secondary text-sm"
            style={{ padding: '8px 16px' }}
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
