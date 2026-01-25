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
    <div className="fixed inset-0 z-50 overflow-y-auto">
      <div className="flex items-center justify-center min-h-screen px-4 pt-4 pb-20 text-center sm:block sm:p-0">
        <div className="fixed inset-0 transition-opacity bg-gray-900 bg-opacity-75" />

        <span className="hidden sm:inline-block sm:align-middle sm:h-screen">&#8203;</span>

        <div className="inline-block align-bottom bg-white rounded-lg text-left overflow-hidden shadow-xl transform transition-all sm:my-8 sm:align-middle sm:max-w-lg sm:w-full">
          <div className="bg-white px-4 pt-5 pb-4 sm:p-6 sm:pb-4">
            <div className="sm:flex sm:items-start">
              <div className="mt-3 text-center sm:mt-0 sm:text-left w-full">
                <h3 className="text-xl leading-6 font-medium text-gray-900 mb-2">
                  Welcome to DataFold
                </h3>
                <div className="mt-2">
                  <p className="text-sm text-gray-500 mb-4">
                    Enter your user identifier to continue. This will generate a unique session hash for your environment.
                  </p>
                  
                  <form onSubmit={handleSubmit}>
                    <div className="mb-4">
                      <label htmlFor="userId" className="block text-sm font-medium text-gray-700 mb-1">
                        User Identifier
                      </label>
                      <input
                        type="text"
                        id="userId"
                        className="shadow-sm focus:ring-blue-500 focus:border-blue-500 block w-full sm:text-sm border-gray-300 rounded-md p-2 border"
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
                      <div className="mb-4 text-sm text-red-600">
                        {error}
                      </div>
                    )}

                    <button
                      type="submit"
                      disabled={isLoading}
                      className="w-full inline-flex justify-center rounded-md border border-transparent shadow-sm px-4 py-2 bg-blue-600 text-base font-medium text-white hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 sm:text-sm disabled:opacity-50"
                    >
                      {isLoading ? 'Connecting...' : 'Continue'}
                    </button>
                  </form>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}
