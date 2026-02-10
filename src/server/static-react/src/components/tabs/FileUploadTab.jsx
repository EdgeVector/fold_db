import { useState, useEffect, useCallback, useRef } from 'react'
import BatchFolderMode from './upload/BatchFolderMode'
import FileUploadMode from './upload/FileUploadMode'
import UploadInfoPanel from './upload/UploadInfoPanel'

function FileUploadTab({ onResult }) {
  const [isDragging, setIsDragging] = useState(false)
  const [selectedFile, setSelectedFile] = useState(null)
  const [autoExecute, setAutoExecute] = useState(true)
  const [trustDistance, setTrustDistance] = useState(0)
  const [pubKey, setPubKey] = useState('default')
  const [isUploading, setIsUploading] = useState(false)

  const [uploadMode, setUploadMode] = useState('upload') // 'upload', 's3-path', 'batch-folder'
  const [s3FilePath, setS3FilePath] = useState('')
  const [folderPath, setFolderPath] = useState('sample_data')
  const [batchProgress, setBatchProgress] = useState(null)
  const [fileProgresses, setFileProgresses] = useState({})
  const pollIntervalRef = useRef(null)

  useEffect(() => {
    return () => {
      if (pollIntervalRef.current) {
        clearInterval(pollIntervalRef.current)
      }
    }
  }, [])
  const pollFileProgress = useCallback(async (progressIds) => {
    const progresses = {}
    let allComplete = true

    for (const info of progressIds) {
      try {
        const response = await fetch(`/api/ingestion/progress/${info.progress_id}`)
        if (response.ok) {
          const progress = await response.json()
          progresses[info.progress_id] = {
            ...progress,
            file_name: info.file_name
          }
          if (!progress.is_complete) {
            allComplete = false
          }
        }
      } catch (error) {
        console.error(`Failed to fetch progress for ${info.file_name}:`, error)
      }
    }

    setFileProgresses(progresses)

    if (allComplete && pollIntervalRef.current) {
      clearInterval(pollIntervalRef.current)
      pollIntervalRef.current = null
      setIsUploading(false)

      const completed = Object.values(progresses).filter(p => p.is_complete && !p.is_failed).length
      const failed = Object.values(progresses).filter(p => p.is_failed).length

      onResult({
        success: failed === 0,
        data: {
          total_files: progressIds.length,
          completed,
          failed,
          message: `Processed ${completed} files successfully${failed > 0 ? `, ${failed} failed` : ''}`
        }
      })
    }
  }, [onResult])

  const handleBatchFolderIngest = async () => {
    if (!folderPath) {
      onResult({
        success: false,
        error: 'Please provide a folder path'
      })
      return
    }

    setIsUploading(true)
    setBatchProgress(null)
    setFileProgresses({})
    onResult(null)

    try {
      const response = await fetch('/api/ingestion/batch-folder', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          folder_path: folderPath,
          auto_execute: autoExecute
        }),
      })

      const result = await response.json()

      if (result.success) {
        setBatchProgress(result)

        // Start polling for individual file progress
        if (result.file_progress_ids && result.file_progress_ids.length > 0) {
          pollIntervalRef.current = setInterval(() => {
            pollFileProgress(result.file_progress_ids)
          }, 1000)
          // Initial poll
          pollFileProgress(result.file_progress_ids)
        }
      } else {
        setIsUploading(false)
        onResult({
          success: false,
          error: result.error || 'Failed to start batch ingestion'
        })
      }
    } catch (error) {
      setIsUploading(false)
      onResult({
        success: false,
        error: error.message || 'Failed to start batch ingestion'
      })
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
    // Handle batch folder mode separately
    if (uploadMode === 'batch-folder') {
      handleBatchFolderIngest()
      return
    }

    // Validate input based on mode
    if (uploadMode === 's3-path') {
      if (!s3FilePath || !s3FilePath.startsWith('s3://')) {
        onResult({
          success: false,
          error: 'Please provide a valid S3 path (e.g., s3://bucket/path/to/file.json)'
        })
        return
      }
    } else {
      if (!selectedFile) {
        onResult({
          success: false,
          error: 'Please select a file to upload'
        })
        return
      }
    }

    setIsUploading(true)
    onResult(null)

    try {
      const formData = new FormData()

      // Generate a progress_id for tracking
      const progressId = crypto.randomUUID()
      formData.append('progress_id', progressId)

      if (uploadMode === 's3-path') {
        formData.append('s3FilePath', s3FilePath)
      } else {
        formData.append('file', selectedFile)
      }
      
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
      {/* Status Bar */}

      {/* Uploading Indicator */}
      {isUploading && uploadMode !== 'batch-folder' && (
        <div className="minimal-card minimal-card-accent-info p-4">
          <div className="flex items-center gap-3">
            <span className="minimal-spinner"></span>
            <span className="text-info font-medium">$ processing file...<span className="cursor"></span></span>
          </div>
        </div>
      )}

      {/* Batch Processing Indicator */}
      {isUploading && uploadMode === 'batch-folder' && (
        <div className="minimal-card minimal-card-accent-info p-4">
          <div className="flex items-center gap-3">
            <span className="minimal-spinner"></span>
            <span className="text-info font-medium">
              $ processing batch...
              {batchProgress && (
                <span className="text-secondary ml-2">
                  ({Object.values(fileProgresses).filter(p => p.is_complete).length}/{batchProgress.files_found} complete)
                </span>
              )}
              <span className="cursor"></span>
            </span>
          </div>
        </div>
      )}

      {/* Mode Toggle */}
      <div className="minimal-card p-4">
        <div className="flex items-center gap-6">
          <span className="text-sm font-medium text-secondary">--mode:</span>
          <label className="flex items-center gap-2 cursor-pointer">
            <input
              type="radio"
              checked={uploadMode === 'upload'}
              onChange={() => setUploadMode('upload')}
              className="accent-black"
            />
            <span className="text-sm text-primary">upload</span>
          </label>
          <label className="flex items-center gap-2 cursor-pointer">
            <input
              type="radio"
              checked={uploadMode === 's3-path'}
              onChange={() => setUploadMode('s3-path')}
              className="accent-black"
            />
            <span className="text-sm text-primary">s3-path</span>
          </label>
          <label className="flex items-center gap-2 cursor-pointer">
            <input
              type="radio"
              checked={uploadMode === 'batch-folder'}
              onChange={() => setUploadMode('batch-folder')}
              className="accent-black"
            />
            <span className="text-sm text-primary">batch-folder</span>
          </label>
        </div>
      </div>

      {/* Mode-specific Input */}
      {uploadMode === 's3-path' && (
        <div className="minimal-card p-6">
          <h3 className="text-success font-medium mb-4">
            <span className="text-secondary">$</span> S3 File Path
          </h3>
          <div className="space-y-3">
            <label className="block text-sm font-medium text-secondary">
              --s3-uri
            </label>
            <input
              type="text"
              value={s3FilePath}
              onChange={(e) => setS3FilePath(e.target.value)}
              placeholder="s3://bucket-name/path/to/file.json"
              className="minimal-input w-full"
            />
            <p className="text-xs text-secondary">
              # File will be downloaded from S3 for processing without re-uploading
            </p>
          </div>
        </div>
      )}

      {uploadMode === 'batch-folder' && (
        <BatchFolderMode
          folderPath={folderPath}
          setFolderPath={setFolderPath}
          batchProgress={batchProgress}
          fileProgresses={fileProgresses}
        />
      )}

      {uploadMode === 'upload' && (
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
      )}

      {/* Options and Upload Button */}
      <div className="minimal-card p-4">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-4">
            <label className="flex items-center gap-2 text-sm cursor-pointer">
              <input
                type="checkbox"
                checked={autoExecute}
                onChange={(e) => setAutoExecute(e.target.checked)}
                className="accent-black"
              />
              <span className="text-primary">--auto-execute</span>
            </label>
            <span className="text-xs text-secondary">
              # File → JSON → AI analysis → Schema mapping
            </span>
          </div>

          <button
            onClick={handleUpload}
            disabled={
              isUploading ||
              (uploadMode === 'upload' && !selectedFile) ||
              (uploadMode === 's3-path' && !s3FilePath) ||
              (uploadMode === 'batch-folder' && !folderPath)
            }
            className="minimal-btn-secondary minimal-btn px-6 py-2.5 font-medium"
          >
            {isUploading ? (
              <>
                <span className="minimal-spinner"></span>
                <span>Processing...</span>
              </>
            ) : (
              <>
                <span>→</span>
                <span>
                  {uploadMode === 's3-path' ? 'Process S3 File' :
                   uploadMode === 'batch-folder' ? 'Ingest Folder' :
                   'Upload & Process'}
                </span>
              </>
            )}
          </button>
        </div>
      </div>

      <UploadInfoPanel uploadMode={uploadMode} />
    </div>
  )
}

export default FileUploadTab
