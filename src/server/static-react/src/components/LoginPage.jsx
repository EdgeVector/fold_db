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
      await dispatch(loginUser(userId.trim())).unwrap()
    } catch (err) {
      setError(err.message)
    }
  }

  return (
    <div className="min-h-screen flex items-center justify-center bg-surface-secondary">
      <div className="w-full max-w-sm p-10 card">
        <div className="mb-16 text-center">
          <h1 className="text-2xl font-medium">FoldDB</h1>
          <p className="text-secondary mt-2">Your data, your rules</p>
        </div>

        <form onSubmit={handleSubmit}>
          <div className="mb-6">
            <label htmlFor="userId" className="label">User Identifier</label>
            <input
              id="userId"
              type="text"
              autoComplete="username"
              required
              className="input input-lg"
              placeholder="Enter your identifier"
              value={userId}
              onChange={(e) => { setUserId(e.target.value); setError('') }}
              autoFocus
            />
          </div>

          {error && (
            <div className="mb-6 p-3 card card-error text-gruvbox-red text-sm">{error}</div>
          )}

          <button type="submit" disabled={isLoading} className="btn-primary w-full btn-lg">
            {isLoading ? 'Connecting...' : 'Continue'}
          </button>
        </form>

        <p className="text-tertiary text-sm text-center mt-8">
          Use any identifier to create or access your node.
        </p>

        <div className="mt-16 flex items-center justify-center gap-2 text-tertiary text-sm">
          <span className="status-dot status-dot-success" />
          Server online
        </div>
      </div>
    </div>
  )
}
