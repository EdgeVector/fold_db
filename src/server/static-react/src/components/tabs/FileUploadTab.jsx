import { useState, useCallback } from 'react'
import FileUploadMode from './upload/FileUploadMode'

function FileUploadTab({ onResult }) {
  const [isDragging, setIsDragging] = useState(false)
  const [selectedFile, setSelectedFile] = useState(null)
  const [autoExecute, setAutoExecute] = useState(true)
  const [trustDistance, setTrustDistance] = useState(0)
  const [pubKey, setPubKey] = useState('default')
  const [isUploading, setIsUploading] = useState(false)

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

    setIsUploading(true)
    onResult(null)

    try {
      const formData = new FormData()
      const progressId = crypto.randomUUID()
      formData.append('progress_id', progressId)
      formData.append('file', selectedFile)
      formData.append('autoExecute', autoExecute.toString())
      formData.append('trustDistance', trustDistance.toString())
      formData.append('pubKey', pubKey)

      const response = await fetch('/api/ingestion/upload', {
        method: 'POST',
        body: formData,
      })

      const result = await response.json()

      if (result.success) {
        onResult({
          success: true,
          data: {
            schema_used: result.schema_name || result.schema_used,
            new_schema_created: result.new_schema_created,
            mutations_generated: result.mutations_generated,
            mutations_executed: result.mutations_executed
          }
        })
      } else {
        onResult({
          success: false,
          error: result.error || 'Failed to process file'
        })
      }
    } catch (error) {
      onResult({
        success: false,
        error: error.message || 'Failed to process file'
      })
    } finally {
      setIsUploading(false)
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
      {isUploading && (
        <div className="flex items-center gap-3 text-gruvbox-blue">
          <span className="spinner" />
          <span>Processing file...</span>
        </div>
      )}

      <FileUploadMode
        isDragging={isDragging}
        selectedFile={selectedFile}
        setSelectedFile={setSelectedFile}
        handleDragEnter={handleDragEnter}
        handleDragOver={handleDragOver}
        handleDragLeave={handleDragLeave}
        handleDrop={handleDrop}
        handleFileSelect={handleFileSelect}
        formatFileSize={formatFileSize}
      />

      <div className="flex items-center justify-between">
        <label className="flex items-center gap-2 text-sm cursor-pointer">
          <input type="checkbox" checked={autoExecute} onChange={(e) => setAutoExecute(e.target.checked)} className="checkbox" />
          <span className="text-secondary">Auto-execute</span>
        </label>

        <button
          onClick={handleUpload}
          disabled={isUploading || !selectedFile}
          className="btn-primary btn-lg flex items-center gap-2"
        >
          {isUploading ? <><span className="spinner" />Processing...</> : '→ Upload'}
        </button>
      </div>
    </div>
  )
}

export default FileUploadTab
