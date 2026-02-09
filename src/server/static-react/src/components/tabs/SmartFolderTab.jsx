import { useState, useEffect } from 'react'
import { ingestionClient } from '../../api/clients'

// Check if running in Tauri
const isTauri = typeof window !== 'undefined' && window.__TAURI_INTERNALS__

function SmartFolderTab({ onResult }) {
  const [folderPath, setFolderPath] = useState('')
  const [isScanning, setIsScanning] = useState(false)
  const [isIngesting, setIsIngesting] = useState(false)
  const [scanResult, setScanResult] = useState(null)
  const [ingestionStarted, setIngestionStarted] = useState(false)
  const [ingestionStatus, setIngestionStatus] = useState(null)

  // Open native folder picker (Tauri only)
  const openFolderPicker = async () => {
    if (!isTauri) return

    try {
      // Dynamic import to avoid bundling Tauri in web builds
      const { open } = await import('@tauri-apps/plugin-dialog')
      const selected = await open({
        directory: true,
        multiple: false,
        title: 'Select folder to scan'
      })

      if (selected) {
        setFolderPath(selected)
      }
    } catch (error) {
      console.error('Failed to open folder picker:', error)
    }
  }

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

  const handleScan = async () => {
    if (!folderPath.trim()) return

    setIsScanning(true)
    setScanResult(null)
    setIngestionStarted(false)
    onResult(null)

    try {
      const response = await ingestionClient.smartFolderScan(folderPath.trim())
      if (response.success) {
        setScanResult(response.data)
      } else {
        onResult({ success: false, error: 'Failed to scan folder' })
      }
    } catch (error) {
      onResult({ success: false, error: error.message || 'Failed to scan folder' })
    } finally {
      setIsScanning(false)
    }
  }

  const handleIngest = async () => {
    if (!scanResult) return

    const filePaths = scanResult.recommended_files.map(f => f.path)
    if (filePaths.length === 0) {
      onResult({ success: false, error: 'No files recommended for ingestion' })
      return
    }

    setIsIngesting(true)
    onResult(null)

    try {
      const response = await ingestionClient.smartFolderIngest(folderPath.trim(), filePaths)
      if (response.success) {
        setIngestionStarted(true)
        onResult({
          success: true,
          data: {
            message: response.data.message,
            batch_id: response.data.batch_id,
            files_found: response.data.files_found,
          }
        })
      } else {
        onResult({ success: false, error: 'Failed to start ingestion' })
      }
    } catch (error) {
      onResult({ success: false, error: error.message || 'Failed to start ingestion' })
    } finally {
      setIsIngesting(false)
    }
  }

  const handleBack = () => {
    setScanResult(null)
    setIngestionStarted(false)
    onResult(null)
  }

  const estimatedCost = scanResult
    ? (scanResult.recommended_files.length * 0.02).toFixed(2)
    : null

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
          </div>
        </div>
      )}

      {/* Phase 1: Scan Input */}
      {!scanResult && !ingestionStarted && (
        <div className="card-terminal p-4">
          <h3 className="text-terminal-green font-medium mb-3">
            <span className="text-terminal-dim">$</span> Smart Folder Scan
          </h3>
          <p className="text-sm text-terminal-dim mb-4">
            Enter a folder path to scan for files. The AI will analyze the contents and recommend files for ingestion.
          </p>
          <div className="flex gap-3">
            <input
              type="text"
              value={folderPath}
              onChange={(e) => setFolderPath(e.target.value)}
              onKeyDown={(e) => { if (e.key === 'Enter') handleScan() }}
              placeholder="/path/to/your/folder"
              className="input-terminal flex-1"
              disabled={isScanning}
            />
            {isTauri && (
              <button
                onClick={openFolderPicker}
                disabled={isScanning}
                className="btn-terminal px-4 py-2"
                title="Browse folders"
              >
                📁
              </button>
            )}
            <button
              onClick={handleScan}
              disabled={isScanning || !folderPath.trim()}
              className={`btn-terminal px-6 py-2 font-medium ${
                isScanning || !folderPath.trim()
                  ? 'opacity-50 cursor-not-allowed'
                  : 'btn-terminal-primary'
              }`}
            >
              {isScanning ? (
                <>
                  <span className="spinner-terminal"></span>
                  <span>Scanning...</span>
                </>
              ) : (
                <>
                  <span>→</span>
                  <span>Scan Folder</span>
                </>
              )}
            </button>
          </div>
        </div>
      )}

      {/* Phase 2: Scan Results */}
      {scanResult && !ingestionStarted && (
        <>
          {/* Summary */}
          <div className="card-terminal p-4 border-l-4 border-terminal-cyan">
            <h3 className="text-terminal-cyan font-medium mb-2">
              Scan Results
            </h3>
            <div className="flex items-center gap-6 text-sm">
              <span className="text-terminal-green">
                {scanResult.recommended_files.length} files to ingest
              </span>
              <span className="text-terminal-dim">
                {scanResult.skipped_files.length} skipped
              </span>
              <span className="text-terminal-dim">
                {scanResult.total_files} total scanned
              </span>
              {estimatedCost && (
                <span className="text-terminal-dim">
                  Est. cost: ~${estimatedCost}
                </span>
              )}
            </div>

            {/* Category summary */}
            {Object.keys(scanResult.summary).length > 0 && (
              <div className="flex gap-3 mt-3 flex-wrap">
                {Object.entries(scanResult.summary).map(([category, count]) => (
                  <span key={category} className="badge-terminal text-xs">
                    {category}: {count}
                  </span>
                ))}
              </div>
            )}
          </div>

          {/* File List */}
          <div className="card-terminal p-4">
            <h4 className="text-terminal-green text-sm font-medium mb-3">
              <span className="text-terminal-dim">$</span> Recommended Files
            </h4>
            <div className="space-y-1 max-h-64 overflow-y-auto">
              {scanResult.recommended_files.map((file, i) => (
                <div key={i} className="flex items-center gap-3 text-sm py-1 border-b border-terminal/10 last:border-0">
                  <span className="text-terminal-green text-xs">+</span>
                  <span className="text-terminal font-mono text-xs flex-1 truncate">{file.path}</span>
                  <span className="badge-terminal text-xs">{file.category}</span>
                  <span className="text-terminal-dim text-xs">{file.reason}</span>
                </div>
              ))}
            </div>

            {scanResult.skipped_files.length > 0 && (
              <>
                <h4 className="text-terminal-dim text-sm font-medium mt-4 mb-2">
                  Skipped Files
                </h4>
                <div className="space-y-1 max-h-32 overflow-y-auto">
                  {scanResult.skipped_files.map((file, i) => (
                    <div key={i} className="flex items-center gap-3 text-sm py-1 border-b border-terminal/10 last:border-0">
                      <span className="text-terminal-dim text-xs">-</span>
                      <span className="text-terminal-dim font-mono text-xs flex-1 truncate">{file.path}</span>
                      <span className="text-terminal-dim text-xs">{file.reason}</span>
                    </div>
                  ))}
                </div>
              </>
            )}
          </div>

          {/* Action Buttons */}
          <div className="card-terminal p-4">
            <div className="flex items-center justify-between">
              <button
                onClick={handleBack}
                className="btn-terminal px-4 py-2 text-terminal-dim"
                disabled={isIngesting}
              >
                ← Back
              </button>
              <button
                onClick={handleIngest}
                disabled={isIngesting || scanResult.recommended_files.length === 0}
                className={`btn-terminal px-6 py-2.5 font-medium ${
                  isIngesting || scanResult.recommended_files.length === 0
                    ? 'opacity-50 cursor-not-allowed'
                    : 'btn-terminal-primary'
                }`}
              >
                {isIngesting ? (
                  <>
                    <span className="spinner-terminal"></span>
                    <span>Starting...</span>
                  </>
                ) : (
                  <>
                    <span>→</span>
                    <span>Proceed with Ingestion ({scanResult.recommended_files.length} files)</span>
                  </>
                )}
              </button>
            </div>
          </div>
        </>
      )}

      {/* Phase 3: Ingestion Started */}
      {ingestionStarted && (
        <div className="card-terminal p-4 border-l-4 border-terminal-green">
          <h3 className="text-terminal-green font-medium mb-2">
            Ingestion Started
          </h3>
          <p className="text-sm text-terminal-dim mb-3">
            {scanResult.recommended_files.length} files queued for ingestion. Track progress in the header above.
          </p>
          <button
            onClick={handleBack}
            className="btn-terminal px-4 py-2 text-terminal-dim"
          >
            ← Scan Another Folder
          </button>
        </div>
      )}

      {/* Info Panel */}
      <div className="card-terminal p-3 border-l-4 border-terminal-cyan">
        <div className="flex items-start gap-2 text-xs text-terminal-dim">
          <span className="text-terminal-cyan font-bold">[i]</span>
          <span>
            Smart Folder scans a directory, identifies supported file types, and recommends files for AI-powered ingestion.
            Each file is processed individually with schema generation and data mapping. Progress is tracked in the header.
          </span>
        </div>
      </div>
    </div>
  )
}

export default SmartFolderTab
