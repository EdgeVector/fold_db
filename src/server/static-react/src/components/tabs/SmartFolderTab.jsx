import { useState } from 'react'
import { ingestionClient } from '../../api/clients'
import { useIngestionStatus } from '../../hooks/useIngestionStatus'
import IngestionStatusBar from '../shared/IngestionStatusBar'

// Check if running in Tauri
const isTauri = typeof window !== 'undefined' && window.__TAURI_INTERNALS__

function SmartFolderTab({ onResult }) {
  const [folderPath, setFolderPath] = useState('')
  const [isScanning, setIsScanning] = useState(false)
  const [isIngesting, setIsIngesting] = useState(false)
  const [scanResult, setScanResult] = useState(null)
  const [ingestionStarted, setIngestionStarted] = useState(false)
  const { ingestionStatus } = useIngestionStatus()

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
      <IngestionStatusBar ingestionStatus={ingestionStatus} />

      {/* Phase 1: Scan Input */}
      {!scanResult && !ingestionStarted && (
        <div className="minimal-card p-4">
          <h3 className="text-success font-medium mb-3">
            <span className="text-secondary">$</span> Smart Folder Scan
          </h3>
          <p className="text-sm text-secondary mb-4">
            Enter a folder path to scan for files. The AI will analyze the contents and recommend files for ingestion.
          </p>
          <div className="flex gap-3">
            <input
              type="text"
              value={folderPath}
              onChange={(e) => setFolderPath(e.target.value)}
              onKeyDown={(e) => { if (e.key === 'Enter') handleScan() }}
              placeholder="/path/to/your/folder"
              className="minimal-input flex-1"
              disabled={isScanning}
            />
            {isTauri && (
              <button
                onClick={openFolderPicker}
                disabled={isScanning}
                className="minimal-btn-secondary px-4 py-2"
                title="Browse folders"
              >
                📁
              </button>
            )}
            <button
              onClick={handleScan}
              disabled={isScanning || !folderPath.trim()}
              className="minimal-btn-secondary minimal-btn px-6 py-2 font-medium"
            >
              {isScanning ? (
                <>
                  <span className="minimal-spinner"></span>
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
          <div className="minimal-card minimal-card-accent-info p-4">
            <h3 className="text-info font-medium mb-2">
              Scan Results
            </h3>
            <div className="flex items-center gap-6 text-sm">
              <span className="text-success">
                {scanResult.recommended_files.length} files to ingest
              </span>
              <span className="text-secondary">
                {scanResult.skipped_files.length} skipped
              </span>
              <span className="text-secondary">
                {scanResult.total_files} total scanned
              </span>
              {estimatedCost && (
                <span className="text-secondary">
                  Est. cost: ~${estimatedCost}
                </span>
              )}
            </div>

            {/* Category summary */}
            {Object.keys(scanResult.summary).length > 0 && (
              <div className="flex gap-3 mt-3 flex-wrap">
                {Object.entries(scanResult.summary).map(([category, count]) => (
                  <span key={category} className="minimal-badge text-xs">
                    {category}: {count}
                  </span>
                ))}
              </div>
            )}
          </div>

          {/* File List */}
          <div className="minimal-card p-4">
            <h4 className="text-success text-sm font-medium mb-3">
              <span className="text-secondary">$</span> Recommended Files
            </h4>
            <div className="space-y-1 max-h-64 overflow-y-auto">
              {scanResult.recommended_files.map((file, i) => (
                <div key={i} className="minimal-list-item text-sm">
                  <span className="text-success text-xs">+</span>
                  <span className="text-primary font-mono text-xs flex-1 truncate">{file.path}</span>
                  <span className="minimal-badge text-xs">{file.category}</span>
                  <span className="text-secondary text-xs">{file.reason}</span>
                </div>
              ))}
            </div>

            {scanResult.skipped_files.length > 0 && (
              <>
                <h4 className="text-secondary text-sm font-medium mt-4 mb-2">
                  Skipped Files
                </h4>
                <div className="space-y-1 max-h-32 overflow-y-auto">
                  {scanResult.skipped_files.map((file, i) => (
                    <div key={i} className="minimal-list-item text-sm">
                      <span className="text-secondary text-xs">-</span>
                      <span className="text-secondary font-mono text-xs flex-1 truncate">{file.path}</span>
                      <span className="text-secondary text-xs">{file.reason}</span>
                    </div>
                  ))}
                </div>
              </>
            )}
          </div>

          {/* Action Buttons */}
          <div className="minimal-card p-4">
            <div className="flex items-center justify-between">
              <button
                onClick={handleBack}
                className="minimal-btn-secondary px-4 py-2 text-secondary"
                disabled={isIngesting}
              >
                ← Back
              </button>
              <button
                onClick={handleIngest}
                disabled={isIngesting || scanResult.recommended_files.length === 0}
                className="minimal-btn-secondary minimal-btn px-6 py-2.5 font-medium"
              >
                {isIngesting ? (
                  <>
                    <span className="minimal-spinner"></span>
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
        <div className="minimal-card minimal-card-accent-success p-4">
          <h3 className="text-success font-medium mb-2">
            Ingestion Started
          </h3>
          <p className="text-sm text-secondary mb-3">
            {scanResult.recommended_files.length} files queued for ingestion. Track progress in the header above.
          </p>
          <button
            onClick={handleBack}
            className="minimal-btn-secondary px-4 py-2 text-secondary"
          >
            ← Scan Another Folder
          </button>
        </div>
      )}

      {/* Info Panel */}
      <div className="minimal-card minimal-card-accent-info p-3">
        <div className="flex items-start gap-2 text-xs text-secondary">
          <span className="text-info font-bold">[i]</span>
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
