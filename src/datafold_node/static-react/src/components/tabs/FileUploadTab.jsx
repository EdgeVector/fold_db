import { useState, useEffect, useCallback } from 'react'
import { ingestionClient } from '../../api/clients'
import ProgressBar from '../ProgressBar'

function FileUploadTab({ onResult }) {
  const [isDragging, setIsDragging] = useState(false)
  const [selectedFile, setSelectedFile] = useState(null)
  const [autoExecute, setAutoExecute] = useState(true)
  const [trustDistance, setTrustDistance] = useState(0)
  const [pubKey, setPubKey] = useState('default')
  const [isUploading, setIsUploading] = useState(false)
  const [ingestionStatus, setIngestionStatus] = useState(null)
  const [currentProgress, setCurrentProgress] = useState(null)
  const [progressId, setProgressId] = useState(null)

  useEffect(() => {
    fetchIngestionStatus()
  }, [])

  // Poll for progress updates when we have a progress ID
  useEffect(() => {
    if (!progressId) return

    const pollProgress = async () => {
      try {
        // Poll for specific progress ID
        const response = await ingestionClient.getProgress(progressId)
        if (response.success && response.data) {
          setCurrentProgress(response.data)
          
          // Stop polling if complete or failed
          if (response.data.is_complete) {
            setIsUploading(false)
            setProgressId(null)
            
            // Show results
            if (response.data.results) {
              onResult({
                success: true,
                data: {
                  schema_used: response.data.results.schema_name,
                  new_schema_created: response.data.results.new_schema_created,
                  mutations_generated: response.data.results.mutations_generated,
                  mutations_executed: response.data.results.mutations_executed
                }
              })
            } else if (response.data.error_message) {
              onResult({
                success: false,
                error: response.data.error_message
              })
            }
          }
        }
      } catch (error) {
        console.error('Failed to fetch progress:', error)
      }
    }

    // Poll immediately, then every 200ms for faster updates
    pollProgress()
    const interval = setInterval(pollProgress, 200)

    return () => clearInterval(interval)
  }, [progressId, onResult])

  const fetchIngestionStatus = async () => {
    try {
      const response = await ingestionClient.getStatus()
      if (response.success) {
        setIngestionStatus(response.data)
      }
    } catch (error) {
      console.error('Failed to fetch ingestion status:', error)
    }
  }

  const handleDragEnter = useCallback((e) => {
    e.preventDefault()
    e.stopPropagation()
    setIsDragging(true)
  }, [])

  const handleDragLeave = useCallback((e) => {
    e.preventDefault()
    e.stopPropagation()
    setIsDragging(false)
  }, [])

  const handleDragOver = useCallback((e) => {
    e.preventDefault()
    e.stopPropagation()
  }, [])

  const handleDrop = useCallback((e) => {
    e.preventDefault()
    e.stopPropagation()
    setIsDragging(false)

    const files = e.dataTransfer.files
    if (files && files.length > 0) {
      setSelectedFile(files[0])
    }
  }, [])

  const handleFileSelect = useCallback((e) => {
    const files = e.target.files
    if (files && files.length > 0) {
      setSelectedFile(files[0])
    }
  }, [])

  const handleUpload = async () => {
    if (!selectedFile) {
      onResult({
        success: false,
        error: 'Please select a file to upload'
      })
      return
    }

    // Reset all progress-related state immediately
    setIsUploading(true)
    setProgressId(null)
    
    // Clear any previous results
    onResult(null)
    
    // Show initial progress state immediately
    setCurrentProgress({
      progress_percentage: 0,
      status_message: 'Uploading file...',
      current_step: 'ValidatingConfig',
      is_complete: false,
      started_at: new Date().toISOString()
    })
    
    // Small delay to ensure UI updates before starting upload
    await new Promise(resolve => setTimeout(resolve, 100))

    try {
      const formData = new FormData()
      formData.append('file', selectedFile)
      formData.append('autoExecute', autoExecute.toString())
      formData.append('trustDistance', trustDistance.toString())
      formData.append('pubKey', pubKey)

      const response = await fetch('/api/ingestion/upload', {
        method: 'POST',
        body: formData,
      })

      const result = await response.json()

      if (result.success && result.progress_id) {
        // Start polling for the specific progress ID
        setProgressId(result.progress_id)
        
        // Emit event for header status tracker
        console.log('🟢 FileUploadTab: Dispatching ingestion-started event', result.progress_id)
        window.dispatchEvent(new CustomEvent('ingestion-started', {
          detail: { progressId: result.progress_id }
        }))
        console.log('🟢 FileUploadTab: Event dispatched')
      } else {
        onResult({
          success: false,
          error: result.error || 'Failed to upload file'
        })
        setIsUploading(false)
        setCurrentProgress(null)
      }
    } catch (error) {
      onResult({
        success: false,
        error: error.message || 'Failed to upload file'
      })
      setIsUploading(false)
      setCurrentProgress(null)
    }
  }

  const formatFileSize = (bytes) => {
    if (bytes === 0) return '0 Bytes'
    const k = 1024
    const sizes = ['Bytes', 'KB', 'MB', 'GB']
    const i = Math.floor(Math.log(bytes) / Math.log(k))
    return Math.round(bytes / Math.pow(k, i) * 100) / 100 + ' ' + sizes[i]
  }

  return (
    <div className="space-y-4">
      {/* Status Bar */}
      {ingestionStatus && (
        <div className="bg-white p-3 rounded-lg shadow-sm border border-gray-200">
          <div className="flex items-center gap-4 text-sm">
            <span className={`px-2 py-1 rounded text-xs font-medium ${
              ingestionStatus.enabled && ingestionStatus.configured 
                ? 'bg-green-100 text-green-800' 
                : 'bg-red-100 text-red-800'
            }`}>
              {ingestionStatus.enabled && ingestionStatus.configured ? 'Ready' : 'Not Configured'}
            </span>
            <span className="text-gray-600">{ingestionStatus.provider} · {ingestionStatus.model}</span>
            <span className="text-xs text-gray-500">Configure AI settings using the Settings button in the header</span>
          </div>
        </div>
      )}

      {/* Progress Bar - Shows Ingestion Progress */}
      {currentProgress && (
        <ProgressBar progress={currentProgress} />
      )}

      {/* Drag and Drop Area */}
      <div className="bg-white p-6 rounded-lg shadow">
        <h3 className="text-lg font-medium text-gray-900 mb-4">Upload File</h3>
        
        <div
          className={`border-2 border-dashed rounded-lg p-12 text-center transition-colors ${
            isDragging
              ? 'border-blue-500 bg-blue-50'
              : 'border-gray-300 bg-gray-50 hover:bg-gray-100'
          }`}
          onDragEnter={handleDragEnter}
          onDragOver={handleDragOver}
          onDragLeave={handleDragLeave}
          onDrop={handleDrop}
        >
          <div className="space-y-4">
            {/* Upload Icon */}
            <div className="flex justify-center">
              <svg
                className="w-16 h-16 text-gray-400"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
                xmlns="http://www.w3.org/2000/svg"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12"
                />
              </svg>
            </div>

            {/* Text */}
            {selectedFile ? (
              <div className="space-y-2">
                <p className="text-lg font-medium text-gray-900">{selectedFile.name}</p>
                <p className="text-sm text-gray-500">{formatFileSize(selectedFile.size)}</p>
                <button
                  onClick={() => setSelectedFile(null)}
                  className="text-sm text-blue-600 hover:text-blue-700 underline"
                >
                  Remove file
                </button>
              </div>
            ) : (
              <div>
                <p className="text-lg text-gray-700 mb-2">
                  Drag and drop a file here, or click to select
                </p>
                <p className="text-sm text-gray-500">
                  Supported formats: PDF, DOCX, TXT, CSV, JSON, XML, and more
                </p>
              </div>
            )}

            {/* Hidden File Input */}
            <input
              type="file"
              id="file-upload"
              className="hidden"
              onChange={handleFileSelect}
            />
            
            {/* Browse Button */}
            {!selectedFile && (
              <label
                htmlFor="file-upload"
                className="inline-block px-6 py-3 bg-blue-600 text-white rounded-lg cursor-pointer hover:bg-blue-700 transition-colors"
              >
                Browse Files
              </label>
            )}
          </div>
        </div>
      </div>

      {/* Options and Upload Button */}
      <div className="bg-white p-4 rounded-lg shadow">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-4">
            <label className="flex items-center gap-2 text-sm">
              <input
                type="checkbox"
                checked={autoExecute}
                onChange={(e) => setAutoExecute(e.target.checked)}
                className="rounded"
              />
              <span className="text-gray-700">Auto-execute mutations</span>
            </label>
            <span className="text-xs text-gray-500">
              File will be converted to JSON and processed by AI
            </span>
          </div>
          
          <button
            onClick={handleUpload}
            disabled={isUploading || !selectedFile}
            className={`px-6 py-2.5 rounded font-medium transition-colors ${
              isUploading || !selectedFile
                ? 'bg-gray-300 text-gray-500 cursor-not-allowed'
                : 'bg-blue-600 text-white hover:bg-blue-700'
            }`}
          >
            {isUploading ? 'Uploading...' : 'Upload & Process'}
          </button>
        </div>
      </div>

      {/* Info Panel */}
      <div className="bg-blue-50 border border-blue-200 rounded-lg p-4">
        <div className="flex items-start gap-3">
          <svg
            className="w-6 h-6 text-blue-600 flex-shrink-0 mt-0.5"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
            />
          </svg>
          <div className="text-sm text-blue-800">
            <p className="font-medium mb-1">How it works:</p>
            <ol className="list-decimal list-inside space-y-1">
              <li>Upload any file type (PDFs, documents, spreadsheets, etc.)</li>
              <li>File is automatically converted to JSON using AI</li>
              <li>AI analyzes the JSON and maps it to appropriate schemas</li>
              <li>Data is stored in the database with the file location tracked</li>
            </ol>
          </div>
        </div>
      </div>
    </div>
  )
}

export default FileUploadTab

