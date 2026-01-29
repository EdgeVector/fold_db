import { useState, useEffect, useCallback } from 'react'
import { ingestionClient } from '../../api/clients'

function FileUploadTab({ onResult }) {
  const [isDragging, setIsDragging] = useState(false)
  const [selectedFile, setSelectedFile] = useState(null)
  const [autoExecute, setAutoExecute] = useState(true)
  const [trustDistance, setTrustDistance] = useState(0)
  const [pubKey, setPubKey] = useState('default')
  const [isUploading, setIsUploading] = useState(false)
  const [ingestionStatus, setIngestionStatus] = useState(null)
  const [useS3Path, setUseS3Path] = useState(false)
  const [s3FilePath, setS3FilePath] = useState('')

  useEffect(() => {
    fetchIngestionStatus()
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
    // Validate input based on mode
    if (useS3Path) {
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
      
      if (useS3Path) {
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
      {isUploading && (
        <div className="card-terminal p-4 border-l-4 border-terminal-blue">
          <div className="flex items-center gap-3">
            <span className="spinner-terminal"></span>
            <span className="text-terminal-blue font-medium">$ processing file...<span className="cursor"></span></span>
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
              checked={!useS3Path}
              onChange={() => setUseS3Path(false)}
              className="accent-terminal-green"
            />
            <span className="text-sm text-terminal">upload</span>
          </label>
          <label className="flex items-center gap-2 cursor-pointer">
            <input
              type="radio"
              checked={useS3Path}
              onChange={() => setUseS3Path(true)}
              className="accent-terminal-green"
            />
            <span className="text-sm text-terminal">s3-path</span>
          </label>
        </div>
      </div>

      {/* S3 Path Input or File Upload */}
      {useS3Path ? (
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
      ) : (
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
            disabled={isUploading || (!useS3Path && !selectedFile) || (useS3Path && !s3FilePath)}
            className={`btn-terminal px-6 py-2.5 font-medium ${
              isUploading || (!useS3Path && !selectedFile) || (useS3Path && !s3FilePath)
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
                <span>{useS3Path ? 'Process S3 File' : 'Upload & Process'}</span>
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
              <li>{useS3Path ? 'Provide an S3 file path (files already in S3 are not re-uploaded)' : 'Upload any file type (PDFs, documents, spreadsheets, etc.)'}</li>
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
