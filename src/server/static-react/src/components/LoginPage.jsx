import { useState } from 'react'
import { useAppDispatch, useAppSelector } from '../store/hooks'
import { loginUser } from '../store/authSlice'

export default function LoginPage() {
  const [userId, setUserId] = useState('')
  const [error, setError] = useState('')
  const dispatch = useAppDispatch()
  const { isLoading } = useAppSelector(state => state.auth)

  const handleSubmit = async (e) => {
    e.preventDefault()
    if (!userId.trim()) {
      setError('ERROR: User identifier required')
      return
    }

    try {
      const result = await dispatch(loginUser(userId.trim())).unwrap()
      // Persist to local storage
      localStorage.setItem('fold_user_id', result.id)
      localStorage.setItem('fold_user_hash', result.hash)
    } catch (err) {
      setError('ERROR: ' + err.message)
    }
  }

  return (
    <div className="min-h-screen bg-terminal flex flex-col justify-center py-12 px-4">
      <div className="w-full max-w-md mx-auto">
        {/* ASCII-style logo */}
        <div className="text-center mb-8">
          <pre className="ascii-art inline-block text-terminal-green text-left" style={{ fontSize: '0.5rem', lineHeight: 1.1 }}>
{`
 ██████╗  █████╗ ████████╗ █████╗ ███████╗ ██████╗ ██╗     ██████╗ 
 ██╔══██╗██╔══██╗╚══██╔══╝██╔══██╗██╔════╝██╔═══██╗██║     ██╔══██╗
 ██║  ██║███████║   ██║   ███████║█████╗  ██║   ██║██║     ██║  ██║
 ██║  ██║██╔══██║   ██║   ██╔══██║██╔══╝  ██║   ██║██║     ██║  ██║
 ██████╔╝██║  ██║   ██║   ██║  ██║██║     ╚██████╔╝███████╗██████╔╝
 ╚═════╝ ╚═╝  ╚═╝   ╚═╝   ╚═╝  ╚═╝╚═╝      ╚═════╝ ╚══════╝╚═════╝ 
`}
          </pre>
          <p className="text-terminal-dim text-sm mt-4">
            <span className="text-terminal-green">v1.0.0</span> | Personal Data Node
          </p>
        </div>

        {/* Terminal window */}
        <div className="terminal-window">
          <div className="terminal-header">
            <div className="terminal-dot terminal-dot-red"></div>
            <div className="terminal-dot terminal-dot-yellow"></div>
            <div className="terminal-dot terminal-dot-green"></div>
            <span className="terminal-title">fold_db --login</span>
          </div>
          
          <div className="terminal-body p-6">
            <div className="mb-4 text-terminal-dim text-sm">
              <p className="mb-1">Welcome to Fold DB.</p>
              <p>Enter your user identifier to continue.</p>
            </div>

            <form className="space-y-4" onSubmit={handleSubmit}>
              <div>
                <div className="flex items-center">
                  <span className="text-terminal-green mr-2">$</span>
                  <span className="text-terminal-cyan mr-2">login</span>
                  <span className="text-terminal-dim mr-2">--user</span>
                  <input
                    id="userId"
                    name="userId"
                    type="text"
                    autoComplete="username"
                    required
                    className="flex-1 bg-transparent border-none outline-none text-terminal-green focus:ring-0 p-0"
                    placeholder="<user-id>"
                    value={userId}
                    onChange={(e) => {
                      setUserId(e.target.value)
                      setError('')
                    }}
                    autoFocus
                    style={{ caretColor: 'var(--terminal-green)' }}
                  />
                  {!userId && <span className="cursor"></span>}
                </div>
                <div className="border-b border-terminal-lighter mt-2"></div>
              </div>

              {error && (
                <div className="text-sm text-terminal-red flex items-center gap-2">
                  <span>✖</span>
                  <span>{error}</span>
                </div>
              )}

              <div className="pt-2">
                <button
                  type="submit"
                  disabled={isLoading}
                  className="btn-terminal btn-terminal-primary w-full justify-center"
                >
                  {isLoading ? (
                    <>
                      <span className="spinner-terminal"></span>
                      <span>Connecting...</span>
                    </>
                  ) : (
                    <>
                      <span>→</span>
                      <span>Connect</span>
                    </>
                  )}
                </button>
              </div>
            </form>

            <div className="mt-6 pt-4 border-t border-terminal-lighter">
              <p className="text-xs text-terminal-dim">
                <span className="text-terminal-yellow">TIP:</span> Use any identifier (e.g., email, username) to create or access your node.
              </p>
            </div>
          </div>
        </div>

        {/* Status bar */}
        <div className="mt-4 flex items-center justify-center text-xs text-terminal-dim gap-4">
          <div className="flex items-center gap-2">
            <span className="status-dot status-online"></span>
            <span>Server Online</span>
          </div>
          <span>|</span>
          <span>Secure Connection</span>
        </div>
      </div>
    </div>
  )
}
