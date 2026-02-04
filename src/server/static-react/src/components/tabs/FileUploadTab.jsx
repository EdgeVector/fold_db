import { useState, useEffect, useCallback, useRef } from 'react'
import { ingestionClient } from '../../api/clients'

function FileUploadTab({ onResult }) {
  const [isDragging, setIsDragging] = useState(false)
  const [selectedFile, setSelectedFile] = useState(null)
  const [autoExecute, setAutoExecute] = useState(true)
  const [trustDistance, setTrustDistance] = useState(0)
  const [pubKey, setPubKey] = useState('default')
  const [isUploading, setIsUploading] = useState(false)
  const [ingestionStatus, setIngestionStatus] = useState(null)
  const [uploadMode, setUploadMode] = useState('upload') // 'upload', 's3-path', 'batch-folder'
  const [s3FilePath, setS3FilePath] = useState('')
  const [folderPath, setFolderPath] = useState('sample_data')
  const [batchProgress, setBatchProgress] = useState(null)
  const [fileProgresses, setFileProgresses] = useState({})
  const pollIntervalRef = useRef(null)

  useEffect(() => {
    fetchIngestionStatus()
    return () => {
      if (pollIntervalRef.current) {
        clearInterval(pollIntervalRef.current)
      }
    }
  }, [])

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
      {ingestionStatus && (
        <div className="card-terminal p-3 border-l-4 border-terminal-green">
          <div className="flex items-center gap-4 text-sm">
            <span className={`badge-terminal ${
              ingestionStatus.enabled && ingestionStatus.configured 
                ? 'badge-terminal-success' 
                : 'badge-terminal-error'
            }`}>
              {ingestionStatus.enabled && ingestionStatus.configured ? 'Ready' : 'Not Configured'}
            </span>
            <span className="text-terminal-dim">{ingestionStatus.provider} · {ingestionStatus.model}</span>
            <span className="text-xs text-terminal-dim">Configure AI settings using the Settings button in the header</span>
          </div>
        </div>
      )}

      {/* Uploading Indicator */}
      {isUploading && uploadMode !== 'batch-folder' && (
        <div className="card-terminal p-4 border-l-4 border-terminal-blue">
          <div className="flex items-center gap-3">
            <span className="spinner-terminal"></span>
            <span className="text-terminal-blue font-medium">$ processing file...<span className="cursor"></span></span>
          </div>
        </div>
      )}

      {/* Batch Processing Indicator */}
      {isUploading && uploadMode === 'batch-folder' && (
        <div className="card-terminal p-4 border-l-4 border-terminal-blue">
          <div className="flex items-center gap-3">
            <span className="spinner-terminal"></span>
            <span className="text-terminal-blue font-medium">
              $ processing batch...
              {batchProgress && (
                <span className="text-terminal-dim ml-2">
                  ({Object.values(fileProgresses).filter(p => p.is_complete).length}/{batchProgress.files_found} complete)
                </span>
              )}
              <span className="cursor"></span>
            </span>
          </div>
        </div>
      )}

      {/* Mode Toggle */}
      <div className="card-terminal p-4">
        <div className="flex items-center gap-6">
          <span className="text-sm font-medium text-terminal-dim">--mode:</span>
          <label className="flex items-center gap-2 cursor-pointer">
            <input
              type="radio"
              checked={uploadMode === 'upload'}
              onChange={() => setUploadMode('upload')}
              className="accent-terminal-green"
            />
            <span className="text-sm text-terminal">upload</span>
          </label>
          <label className="flex items-center gap-2 cursor-pointer">
            <input
              type="radio"
              checked={uploadMode === 's3-path'}
              onChange={() => setUploadMode('s3-path')}
              className="accent-terminal-green"
            />
            <span className="text-sm text-terminal">s3-path</span>
          </label>
          <label className="flex items-center gap-2 cursor-pointer">
            <input
              type="radio"
              checked={uploadMode === 'batch-folder'}
              onChange={() => setUploadMode('batch-folder')}
              className="accent-terminal-green"
            />
            <span className="text-sm text-terminal">batch-folder</span>
          </label>
        </div>
      </div>

      {/* Mode-specific Input */}
      {uploadMode === 's3-path' && (
        <div className="card-terminal p-6">
          <h3 className="text-terminal-green font-medium mb-4">
            <span className="text-terminal-dim">$</span> S3 File Path
          </h3>
          <div className="space-y-3">
            <label className="block text-sm font-medium text-terminal-dim">
              --s3-uri
            </label>
            <input
              type="text"
              value={s3FilePath}
              onChange={(e) => setS3FilePath(e.target.value)}
              placeholder="s3://bucket-name/path/to/file.json"
              className="input-terminal w-full"
            />
            <p className="text-xs text-terminal-dim">
              # File will be downloaded from S3 for processing without re-uploading
            </p>
          </div>
        </div>
      )}

      {uploadMode === 'batch-folder' && (
        <div className="card-terminal p-6">
          <h3 className="text-terminal-green font-medium mb-4">
            <span className="text-terminal-dim">$</span> Batch Folder Ingestion
          </h3>
          <div className="space-y-3">
            <label className="block text-sm font-medium text-terminal-dim">
              --folder-path
            </label>
            <input
              type="text"
              value={folderPath}
              onChange={(e) => setFolderPath(e.target.value)}
              placeholder="sample_data"
              className="input-terminal w-full"
            />
            <p className="text-xs text-terminal-dim">
              # Path relative to project root or absolute path. Supported files: .json, .csv, .txt, .md
            </p>
          </div>

          {/* Batch Progress Display */}
          {batchProgress && (
            <div className="mt-4 space-y-3">
              <div className="flex items-center gap-2 text-sm">
                <span className="text-terminal-dim">batch_id:</span>
                <span className="text-terminal-cyan font-mono text-xs">{batchProgress.batch_id.slice(0, 8)}...</span>
                <span className="text-terminal-dim">|</span>
                <span className="text-terminal-dim">files:</span>
                <span className="text-terminal-green">{batchProgress.files_found}</span>
              </div>

              {/* Individual File Progress */}
              <div className="space-y-2 max-h-48 overflow-y-auto">
                {Object.entries(fileProgresses).map(([id, progress]) => (
                  <div key={id} className="flex items-center gap-3 text-sm bg-terminal-darker p-2 rounded">
                    <span className={`w-2 h-2 rounded-full ${
                      progress.is_failed ? 'bg-terminal-red' :
                      progress.is_complete ? 'bg-terminal-green' :
                      'bg-terminal-yellow animate-pulse'
                    }`}></span>
                    <span className="text-terminal font-mono text-xs truncate flex-1">
                      {progress.file_name}
                    </span>
                    <span className="text-terminal-dim text-xs">
                      {progress.progress_percentage}%
                    </span>
                    <span className={`text-xs ${
                      progress.is_failed ? 'text-terminal-red' :
                      progress.is_complete ? 'text-terminal-green' :
                      'text-terminal-yellow'
                    }`}>
                      {progress.is_failed ? 'failed' :
                       progress.is_complete ? 'done' :
                       progress.current_step || 'processing'}
                    </span>
                  </div>
                ))}
              </div>
            </div>
          )}
        </div>
      )}

      {uploadMode === 'upload' && (
        <div className="card-terminal p-6">
          <h3 className="text-terminal-green font-medium mb-4">
            <span className="text-terminal-dim">$</span> Upload File
          </h3>

          <div
            className={`border-2 border-dashed p-12 text-center transition-colors ${
              isDragging
                ? 'border-terminal-green bg-terminal-light'
                : 'border-terminal bg-terminal hover:border-terminal-green'
            }`}
            onDragEnter={handleDragEnter}
            onDragOver={handleDragOver}
            onDragLeave={handleDragLeave}
            onDrop={handleDrop}
          >
            <div className="space-y-4">
              {/* Upload Icon - ASCII style */}
              <div className="text-terminal-dim font-mono text-3xl">
                ↑
              </div>

              {/* Text */}
              {selectedFile ? (
                <div className="space-y-2">
                  <p className="text-terminal-green font-medium">{selectedFile.name}</p>
                  <p className="text-sm text-terminal-dim">{formatFileSize(selectedFile.size)}</p>
                  <button
                    onClick={() => setSelectedFile(null)}
                    className="text-sm text-terminal-red hover:glow-red"
                  >
                    [x] remove
                  </button>
                </div>
              ) : (
                <div>
                  <p className="text-terminal mb-2">
                    Drag and drop a file here, or click to select
                  </p>
                  <p className="text-sm text-terminal-dim">
                    # Supported: PDF, DOCX, TXT, CSV, JSON, XML
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
                  className="btn-terminal btn-terminal-primary inline-block cursor-pointer"
                >
                  → Browse Files
                </label>
              )}
            </div>
          </div>
        </div>
      )}

      {/* Options and Upload Button */}
      <div className="card-terminal p-4">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-4">
            <label className="flex items-center gap-2 text-sm cursor-pointer">
              <input
                type="checkbox"
                checked={autoExecute}
                onChange={(e) => setAutoExecute(e.target.checked)}
                className="accent-terminal-green"
              />
              <span className="text-terminal">--auto-execute</span>
            </label>
            <span className="text-xs text-terminal-dim">
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
            className={`btn-terminal px-6 py-2.5 font-medium ${
              isUploading ||
              (uploadMode === 'upload' && !selectedFile) ||
              (uploadMode === 's3-path' && !s3FilePath) ||
              (uploadMode === 'batch-folder' && !folderPath)
                ? 'opacity-50 cursor-not-allowed'
                : 'btn-terminal-primary'
            }`}
          >
            {isUploading ? (
              <>
                <span className="spinner-terminal"></span>
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

      {/* Info Panel */}
      <div className="card-terminal p-4 border-l-4 border-terminal-cyan">
        <div className="flex items-start gap-3">
          <span className="text-terminal-cyan">[i]</span>
          <div className="text-sm text-terminal-dim">
            <p className="font-medium mb-1 text-terminal-cyan"># How it works:</p>
            <ol className="list-decimal list-inside space-y-1">
              {uploadMode === 'batch-folder' ? (
                <>
                  <li>Specify a folder path containing files to ingest</li>
                  <li>All supported files (.json, .csv, .txt, .md) are processed in parallel</li>
                  <li>Each file is converted to JSON and analyzed by AI</li>
                  <li>Data is mapped to schemas and stored in the database</li>
                </>
              ) : uploadMode === 's3-path' ? (
                <>
                  <li>Provide an S3 file path (files already in S3 are not re-uploaded)</li>
                  <li>File is automatically converted to JSON using AI</li>
                  <li>AI analyzes the JSON and maps it to appropriate schemas</li>
                  <li>Data is stored in the database with the file location tracked</li>
                </>
              ) : (
                <>
                  <li>Upload any file type (PDFs, documents, spreadsheets, etc.)</li>
                  <li>File is automatically converted to JSON using AI</li>
                  <li>AI analyzes the JSON and maps it to appropriate schemas</li>
                  <li>Data is stored in the database with the file location tracked</li>
                </>
              )}
            </ol>
          </div>
        </div>
      </div>
    </div>
  )
}

export default FileUploadTab
