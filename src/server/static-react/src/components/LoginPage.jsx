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
    <div className="min-h-screen bg-gray-50 flex flex-col justify-center py-12 sm:px-6 lg:px-8">
      <div className="sm:mx-auto sm:w-full sm:max-w-md">
        <h2 className="mt-6 text-center text-3xl font-extrabold text-gray-900">
          Sign in to Exemem
        </h2>
        <p className="mt-2 text-center text-sm text-gray-600">
          Enter your user identifier to access your Exemem node
        </p>
      </div>

      <div className="mt-8 sm:mx-auto sm:w-full sm:max-w-md">
        <div className="bg-white py-8 px-4 shadow sm:rounded-lg sm:px-10">
          <form className="space-y-6" onSubmit={handleSubmit}>
            <div>
              <label htmlFor="userId" className="block text-sm font-medium text-gray-700">
                User Identifier
              </label>
              <div className="mt-1">
                <input
                  id="userId"
                  name="userId"
                  type="text"
                  autoComplete="username"
                  required
                  className="appearance-none block w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm placeholder-gray-400 focus:outline-none focus:ring-blue-500 focus:border-blue-500 sm:text-sm"
                  placeholder="e.g. alice-dev"
                  value={userId}
                  onChange={(e) => {
                    setUserId(e.target.value)
                    setError('')
                  }}
                  autoFocus
                />
              </div>
            </div>

            {error && (
              <div className="text-sm text-red-600">
                {error}
              </div>
            )}

            <div>
              <button
                type="submit"
                disabled={isLoading}
                className="w-full flex justify-center py-2 px-4 border border-transparent rounded-md shadow-sm text-sm font-medium text-white bg-blue-600 hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 disabled:opacity-50"
              >
                {isLoading ? 'Connecting...' : 'Continue'}
              </button>
            </div>
          </form>
        </div>
      </div>
    </div>
  )
}
