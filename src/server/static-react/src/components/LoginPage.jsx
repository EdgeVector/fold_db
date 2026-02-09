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
      setError('User identifier required')
      return
    }

    try {
      // loginUser thunk handles localStorage persistence internally
      await dispatch(loginUser(userId.trim())).unwrap()
    } catch (err) {
      setError(err.message)
    }
  }

  return (
    <div className="minimal-login-page">
      <div className="minimal-login-card">
        {/* Logo */}
        <div className="mb-16">
          <h1 className="minimal-login-logo">
            FoldDB
          </h1>
          <p className="minimal-login-subtitle">
            Your data, your rules
          </p>
        </div>

        {/* Login form */}
        <form onSubmit={handleSubmit}>
          <div className="mb-6">
            <label htmlFor="userId" className="minimal-label">
              User Identifier
            </label>
            <input
              id="userId"
              name="userId"
              type="text"
              autoComplete="username"
              required
              className="minimal-input minimal-input-lg"
              placeholder="Enter your identifier"
              value={userId}
              onChange={(e) => {
                setUserId(e.target.value)
                setError('')
              }}
              autoFocus
            />
          </div>

          {error && (
            <div className="minimal-error-banner mb-6">
              <p>{error}</p>
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

        {/* Tip */}
        <p className="minimal-hint mt-8">
          Use any identifier (email, username) to create or access your node.
        </p>

        {/* Status */}
        <div className="minimal-status mt-16 text-[13px]">
          <span className="minimal-status-dot"></span>
          Server online
        </div>
      </div>
    </div>
  )
}
