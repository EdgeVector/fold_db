import { useState } from 'react'
import { ingestionClient } from '../../api/clients'

const isTauri = typeof window !== 'undefined' && window.__TAURI_INTERNALS__

function SmartFolderTab({ onResult }) {
  const [folderPath, setFolderPath] = useState('')
  const [isScanning, setIsScanning] = useState(false)
  const [isIngesting, setIsIngesting] = useState(false)
  const [scanResult, setScanResult] = useState(null)
  const [ingestionStarted, setIngestionStarted] = useState(false)

  const openFolderPicker = async () => {
    if (!isTauri) return
    try {
      const { open } = await import('@tauri-apps/plugin-dialog')
      const selected = await open({ directory: true, multiple: false, title: 'Select folder to scan' })
      if (selected) setFolderPath(selected)
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
      if (response.success) setScanResult(response.data)
      else onResult({ success: false, error: 'Failed to scan folder' })
    } catch (error) {
      onResult({ success: false, error: error.message || 'Failed to scan folder' })
    } finally {
      setIsScanning(false)
    }
  }

  const handleIngest = async () => {
    if (!scanResult) return
    const filePaths = scanResult.recommended_files.map(f => f.path)
    if (filePaths.length === 0) { onResult({ success: false, error: 'No files recommended' }); return }
    setIsIngesting(true)
    onResult(null)
    try {
      const response = await ingestionClient.smartFolderIngest(folderPath.trim(), filePaths)
      if (response.success) {
        setIngestionStarted(true)
        onResult({ success: true, data: { message: response.data.message, batch_id: response.data.batch_id, files_found: response.data.files_found } })
      } else onResult({ success: false, error: 'Failed to start ingestion' })
    } catch (error) {
      onResult({ success: false, error: error.message || 'Failed to start ingestion' })
    } finally {
      setIsIngesting(false)
    }
  }

  const handleBack = () => { setScanResult(null); setIngestionStarted(false); onResult(null) }
  const estimatedCost = scanResult ? (scanResult.recommended_files.length * 0.02).toFixed(2) : null

  return (
    <div className="space-y-4">
      {!scanResult && !ingestionStarted && (
        <div className="card p-4">
          <p className="text-sm text-secondary mb-4">Enter a folder path to scan. AI will recommend files for ingestion.</p>
          <div className="flex gap-3">
            <input type="text" value={folderPath} onChange={(e) => setFolderPath(e.target.value)} onKeyDown={(e) => e.key === 'Enter' && handleScan()} placeholder="/path/to/your/folder" className="input flex-1" disabled={isScanning} />
            {isTauri && <button onClick={openFolderPicker} disabled={isScanning} className="btn-secondary" title="Browse">📁</button>}
            <button onClick={handleScan} disabled={isScanning || !folderPath.trim()} className="btn-primary flex items-center gap-2">
              {isScanning ? <><span className="spinner" />Scanning...</> : <>→ Scan Folder</>}
            </button>
          </div>
        </div>
      )}

      {scanResult && !ingestionStarted && (
        <>
          <div className="card card-info p-4">
            <div className="flex items-center gap-6 text-sm">
              <span className="text-success">{scanResult.recommended_files.length} files to ingest</span>
              <span className="text-secondary">{scanResult.skipped_files.length} skipped</span>
              <span className="text-secondary">{scanResult.total_files} total</span>
              {estimatedCost && <span className="text-secondary">Est. ~${estimatedCost}</span>}
            </div>
            {Object.keys(scanResult.summary).length > 0 && (
              <div className="flex gap-2 mt-3 flex-wrap">
                {Object.entries(scanResult.summary).map(([cat, count]) => (
                  <span key={cat} className="badge badge-neutral">{cat}: {count}</span>
                ))}
              </div>
            )}
          </div>

          <div className="card p-4">
            <div className="space-y-1 max-h-64 overflow-y-auto">
              {scanResult.recommended_files.map((file, i) => (
                <div key={i} className="list-item text-sm">
                  <span className="text-success text-xs">+</span>
                  <span className="font-mono text-xs flex-1 truncate">{file.path}</span>
                  <span className="badge badge-neutral">{file.category}</span>
                  <span className="text-secondary text-xs">{file.reason}</span>
                </div>
              ))}
            </div>
            {scanResult.skipped_files.length > 0 && (
              <>
                <h4 className="text-secondary text-sm font-medium mt-4 mb-2">Skipped Files</h4>
                <div className="space-y-1 max-h-32 overflow-y-auto">
                  {scanResult.skipped_files.map((file, i) => (
                    <div key={i} className="list-item text-sm">
                      <span className="text-secondary text-xs">-</span>
                      <span className="text-secondary font-mono text-xs flex-1 truncate">{file.path}</span>
                      <span className="text-secondary text-xs">{file.reason}</span>
                    </div>
                  ))}
                </div>
              </>
            )}
          </div>

          <div className="card p-4 flex items-center justify-between">
            <button onClick={handleBack} className="btn-secondary" disabled={isIngesting}>← Back</button>
            <button onClick={handleIngest} disabled={isIngesting || scanResult.recommended_files.length === 0} className="btn-primary btn-lg flex items-center gap-2">
              {isIngesting ? <><span className="spinner" />Starting...</> : <>→ Proceed ({scanResult.recommended_files.length} files)</>}
            </button>
          </div>
        </>
      )}

      {ingestionStarted && (
        <div className="card card-success p-4">
          <p className="text-sm text-secondary mb-3">{scanResult.recommended_files.length} files queued. Track progress in header.</p>
          <button onClick={handleBack} className="btn-secondary">← Scan Another Folder</button>
        </div>
      )}

    </div>
  )
}

export default SmartFolderTab
