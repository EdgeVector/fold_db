import { useState } from 'react'
import { CheckCircleIcon, TrashIcon } from '@heroicons/react/24/solid'
import { systemClient } from '../api/clients/systemClient'

function StatusSection() {
  const [showConfirmDialog, setShowConfirmDialog] = useState(false)
  const [isResetting, setIsResetting] = useState(false)
  const [resetResult, setResetResult] = useState(null)

  const handleResetDatabase = async () => {
    setIsResetting(true)
    setResetResult(null)
    
    try {
      const response = await systemClient.resetDatabase(true)
      
      if (response.success && response.data) {
        setResetResult({ type: 'success', message: response.data.message })
        // Refresh the page after a short delay to show the new clean state
        setTimeout(() => {
          window.location.reload()
        }, 2000)
      } else {
        setResetResult({ type: 'error', message: response.error || 'Reset failed' })
      }
    } catch (error) {
      setResetResult({ type: 'error', message: `Network error: ${error.message}` })
    } finally {
      setIsResetting(false)
      setShowConfirmDialog(false)
    }
  }

  const ResetConfirmDialog = () => {
    if (!showConfirmDialog) return null

    return (
      <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
        <div className="bg-white rounded-lg p-6 max-w-md w-full mx-4">
          <div className="flex items-center gap-3 mb-4">
            <TrashIcon className="w-6 h-6 text-red-500" />
            <h3 className="text-lg font-semibold text-gray-900">Reset Database</h3>
          </div>
          
          <div className="mb-6">
            <p className="text-gray-700 mb-2">
              This will permanently delete all data and restart the node:
            </p>
            <ul className="list-disc list-inside text-sm text-gray-600 space-y-1">
              <li>All schemas will be removed</li>
              <li>All stored data will be deleted</li>
              <li>Network connections will be reset</li>
              <li>This action cannot be undone</li>
            </ul>
          </div>
          
          <div className="flex gap-3 justify-end">
            <button
              onClick={() => setShowConfirmDialog(false)}
              className="px-4 py-2 text-sm font-medium text-gray-700 bg-gray-100 rounded-md hover:bg-gray-200 transition-colors"
              disabled={isResetting}
            >
              Cancel
            </button>
            <button
              onClick={handleResetDatabase}
              disabled={isResetting}
              className="px-4 py-2 text-sm font-medium text-white bg-red-600 rounded-md hover:bg-red-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
            >
              {isResetting ? 'Resetting...' : 'Reset Database'}
            </button>
          </div>
        </div>
      </div>
    )
  }

  return (
    <>
      <div className="bg-white rounded-lg shadow-sm p-4 mb-6">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <CheckCircleIcon className="icon icon-md text-green-500" />
            <span className="text-gray-700 font-medium">
              Node is running successfully
            </span>
          </div>
          
          <button
            onClick={() => setShowConfirmDialog(true)}
            className="flex items-center gap-2 px-3 py-1.5 text-sm font-medium text-red-600 border border-red-200 rounded-md hover:bg-red-50 hover:border-red-300 transition-colors"
            disabled={isResetting}
          >
            <TrashIcon className="w-4 h-4" />
            Reset Database
          </button>
        </div>
        
        <div className="mt-2 flex items-center gap-2">
          <div className="w-2 h-2 rounded-full bg-green-500 animate-pulse"></div>
          <span className="text-sm text-gray-500">Active and healthy</span>
        </div>
        
        {resetResult && (
          <div className={`mt-3 p-3 rounded-md text-sm ${
            resetResult.type === 'success'
              ? 'bg-green-50 text-green-800 border border-green-200'
              : 'bg-red-50 text-red-800 border border-red-200'
          }`}>
            {resetResult.message}
          </div>
        )}
      </div>
      
      <ResetConfirmDialog />
    </>
  )
}

export default StatusSection