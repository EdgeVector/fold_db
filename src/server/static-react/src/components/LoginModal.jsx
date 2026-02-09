import { useState } from 'react'
import { useAppDispatch, useAppSelector } from '../store/hooks'
import { loginUser } from '../store/authSlice'

export default function LoginModal() {
  const [userId, setUserId] = useState('')
  const [error, setError] = useState('')
  const dispatch = useAppDispatch()
  const { isAuthenticated, isLoading } = useAppSelector(state => state.auth)

  if (isAuthenticated) return null

  const handleSubmit = async (e) => {
    e.preventDefault()
    if (!userId.trim()) {
      setError('Please enter a user identifier')
      return
    }

    try {
      const result = await dispatch(loginUser(userId.trim())).unwrap()
      // Persist to local storage
      localStorage.setItem('fold_user_id', result.id)
      localStorage.setItem('fold_user_hash', result.hash)
    } catch (err) {
      setError('Login failed: ' + err.message)
    }
  }

  return (
    <div className="minimal-modal-overlay">
      <div className="minimal-modal sm:max-w-lg">
        <div className="p-6">
          <h3 className="text-xl font-medium text-primary mb-2">
            Welcome to Fold DB
          </h3>
          <p className="text-sm text-secondary mb-4">
            Enter your user identifier to continue. This will generate a unique session hash for your environment.
          </p>
          
          <form onSubmit={handleSubmit}>
            <div className="mb-4">
              <label htmlFor="userId" className="minimal-label">
                User Identifier
              </label>
              <input
                type="text"
                id="userId"
                className="minimal-input"
                placeholder="e.g. alice-dev"
                value={userId}
                onChange={(e) => {
                  setUserId(e.target.value)
                  setError('')
                }}
                autoFocus
              />
            </div>
            
            {error && (
              <div className="mb-4 text-sm text-error">
                {error}
              </div>
            )}

            <button
              type="submit"
              disabled={isLoading}
              className="minimal-btn w-full"
            >
              {isLoading ? 'Connecting...' : 'Continue'}
            </button>
          </form>
        </div>
      </div>
    </div>
  )
}
