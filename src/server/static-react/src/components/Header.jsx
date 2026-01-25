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
    <header className="bg-white border-b border-gray-200 shadow-sm flex-shrink-0">
      <div className="flex items-center justify-between px-6 py-3">
        <a href="/" className="flex items-center gap-3 text-blue-600 hover:text-blue-700 transition-colors">
          <svg className="w-8 h-8 flex-shrink-0" viewBox="0 0 24 24" fill="currentColor">
            <path d="M12 4C7.58172 4 4 5.79086 4 8C4 10.2091 7.58172 12 12 12C16.4183 12 20 10.2091 20 8C20 5.79086 16.4183 4 12 4Z" />
            <path d="M4 12V16C4 18.2091 7.58172 20 12 20C16.4183 20 20 18.2091 20 16V12" strokeWidth="2" strokeLinecap="round" />
            <path d="M4 8V12C4 14.2091 7.58172 16 12 16C16.4183 16 20 14.2091 20 12V8" strokeWidth="2" strokeLinecap="round" />
          </svg>
          <span className="text-xl font-semibold text-gray-900">DataFold Node</span>
        </a>
        <div className="flex items-center gap-3">
          {isAuthenticated && (
            <div className="flex items-center gap-3 mr-2">
              <span className="text-sm text-gray-600">
                {user?.id}
              </span>
              <button
                onClick={handleLogout}
                className="text-sm text-red-600 hover:text-red-700 font-medium"
              >
                Logout
              </button>
            </div>
          )}
          <div className="h-6 w-px bg-gray-300 mx-1"></div>
          <button
          onClick={onSettingsClick}
          className="inline-flex items-center gap-2 px-3 py-2 text-sm text-gray-700 hover:bg-gray-100 rounded-md border border-gray-300 transition-colors"
          title="Settings"
        >
          <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
          </svg>
          Settings
        </button>
        </div>
      </div>
    </header>
  )
}

export default Header