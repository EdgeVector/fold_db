import { useAppDispatch, useAppSelector } from '../store/hooks'
import { logoutUser } from '../store/authSlice'

function Header({ onSettingsClick }) {
  const dispatch = useAppDispatch()
  const { isAuthenticated, user } = useAppSelector(state => state.auth)
  
  const handleLogout = () => {
    dispatch(logoutUser())
    localStorage.removeItem('fold_user_id')
    localStorage.removeItem('fold_user_hash')
  }

  return (
    <header className="bg-terminal-lighter border-b border-terminal flex-shrink-0">
      <div className="flex items-center justify-between px-6 py-3">
        <a href="/" className="flex items-center gap-3 text-terminal-green hover:glow-green transition-all duration-150">
          <span className="text-xl font-bold tracking-tight">
            <span className="text-terminal-dim">$</span> fold_db
          </span>
          <span className="badge-terminal-success text-xs">v1.0</span>
        </a>
        <div className="flex items-center gap-3">
          {isAuthenticated && (
            <div className="flex items-center gap-3 mr-2">
              <span className="text-sm text-terminal-cyan">
                <span className="text-terminal-dim">user:</span> {user?.id}
              </span>
              <button
                onClick={handleLogout}
                className="text-sm text-terminal-red hover:glow-red font-medium transition-all duration-150"
              >
                exit
              </button>
            </div>
          )}
          <div className="h-6 w-px bg-terminal-lighter mx-1"></div>
          <button
            onClick={onSettingsClick}
            className="btn-terminal text-sm"
            title="Settings"
          >
            <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
            </svg>
            config
          </button>
        </div>
      </div>
    </header>
  )
}

export default Header