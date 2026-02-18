import { useState, useEffect, useRef, useCallback } from 'react'
import { ingestionClient } from '../../api/clients'

const isTauri = typeof window !== 'undefined' && window.__TAURI_INTERNALS__

function SmartFolderTab({ onResult }) {
  const [folderPath, setFolderPath] = useState('')
  const [isScanning, setIsScanning] = useState(false)
  const [isIngesting, setIsIngesting] = useState(false)
  const [scanResult, setScanResult] = useState(null)
  const [ingestionStarted, setIngestionStarted] = useState(false)

  // Autocomplete state
  const [suggestions, setSuggestions] = useState([])
  const [selectedIndex, setSelectedIndex] = useState(-1)
  const [showSuggestions, setShowSuggestions] = useState(false)
  const inputRef = useRef(null)
  const suggestionsRef = useRef(null)
  const debounceRef = useRef(null)

  const fetchCompletions = useCallback(async (path) => {
    if (!path.includes('/')) {
      setSuggestions([])
      setShowSuggestions(false)
      return
    }
    try {
      const response = await ingestionClient.completePath(path)
      if (response.success && response.data?.completions) {
        setSuggestions(response.data.completions)
        setSelectedIndex(-1)
        setShowSuggestions(response.data.completions.length > 0)
      } else {
        setSuggestions([])
        setShowSuggestions(false)
      }
    } catch {
      setSuggestions([])
      setShowSuggestions(false)
    }
  }, [])

  useEffect(() => {
    if (debounceRef.current) clearTimeout(debounceRef.current)
    if (!folderPath.includes('/') || isScanning) {
      setSuggestions([])
      setShowSuggestions(false)
      return
    }
    debounceRef.current = setTimeout(() => fetchCompletions(folderPath), 200)
    return () => { if (debounceRef.current) clearTimeout(debounceRef.current) }
  }, [folderPath, isScanning, fetchCompletions])

  // Close suggestions when clicking outside
  useEffect(() => {
    const handleClickOutside = (e) => {
      if (
        inputRef.current && !inputRef.current.contains(e.target) &&
        suggestionsRef.current && !suggestionsRef.current.contains(e.target)
      ) {
        setShowSuggestions(false)
      }
    }
    document.addEventListener('mousedown', handleClickOutside)
    return () => document.removeEventListener('mousedown', handleClickOutside)
  }, [])

  const acceptSuggestion = (path) => {
    // Append trailing slash so user can keep drilling down
    const newPath = path.endsWith('/') ? path : path + '/'
    setFolderPath(newPath)
    setShowSuggestions(false)
    setSelectedIndex(-1)
    inputRef.current?.focus()
  }

  const handleInputKeyDown = (e) => {
    if (showSuggestions && suggestions.length > 0) {
      if (e.key === 'ArrowDown') {
        e.preventDefault()
        setSelectedIndex((prev) => (prev < suggestions.length - 1 ? prev + 1 : 0))
        return
      }
      if (e.key === 'ArrowUp') {
        e.preventDefault()
        setSelectedIndex((prev) => (prev > 0 ? prev - 1 : suggestions.length - 1))
        return
      }
      if (e.key === 'Tab') {
        e.preventDefault()
        const idx = selectedIndex >= 0 ? selectedIndex : 0
        acceptSuggestion(suggestions[idx])
        return
      }
      if (e.key === 'Enter') {
        if (selectedIndex >= 0) {
          e.preventDefault()
          acceptSuggestion(suggestions[selectedIndex])
          return
        }
        // No suggestion selected — fall through to scan
      }
      if (e.key === 'Escape') {
        setShowSuggestions(false)
        setSelectedIndex(-1)
        return
      }
    }
    if (e.key === 'Enter') handleScan()
  }

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
    setShowSuggestions(false)
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
        <div className="flex gap-3">
          <div className="relative flex-1">
            <input
              ref={inputRef}
              type="text"
              value={folderPath}
              onChange={(e) => setFolderPath(e.target.value)}
              onKeyDown={handleInputKeyDown}
              onFocus={() => { if (suggestions.length > 0) setShowSuggestions(true) }}
              placeholder="/path/to/your/folder"
              className="input w-full"
              disabled={isScanning}
              autoComplete="off"
            />
            {showSuggestions && suggestions.length > 0 && (
              <ul
                ref={suggestionsRef}
                className="absolute z-50 left-0 right-0 top-full mt-1 border border-border rounded-lg bg-surface shadow-lg max-h-48 overflow-y-auto"
              >
                {suggestions.map((path, i) => (
                  <li
                    key={path}
                    className={`px-3 py-1.5 cursor-pointer text-sm font-mono truncate ${
                      i === selectedIndex ? 'bg-accent text-on-accent' : 'hover:bg-surface-hover'
                    }`}
                    onMouseDown={(e) => { e.preventDefault(); acceptSuggestion(path) }}
                    onMouseEnter={() => setSelectedIndex(i)}
                  >
                    {path}
                  </li>
                ))}
              </ul>
            )}
          </div>
          {isTauri && <button onClick={openFolderPicker} disabled={isScanning} className="btn-secondary" title="Browse">📁</button>}
          <button onClick={handleScan} disabled={isScanning || !folderPath.trim()} className="btn-primary flex items-center gap-2">
            {isScanning ? <><span className="spinner" />Scanning...</> : <>→ Scan</>}
          </button>
        </div>
      )}

      {scanResult && !ingestionStarted && (
        <>
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-6 text-sm">
              <span className="text-primary font-medium">{scanResult.recommended_files.length} files to ingest</span>
              <span className="text-secondary">{scanResult.skipped_files.length} skipped</span>
              <span className="text-secondary">{scanResult.total_files} total</span>
              {estimatedCost && <span className="text-secondary">Est. ~${estimatedCost}</span>}
            </div>
            {Object.keys(scanResult.summary).length > 0 && (
              <div className="flex gap-2 flex-wrap">
                {Object.entries(scanResult.summary).map(([cat, count]) => (
                  <span key={cat} className="badge badge-neutral">{cat}: {count}</span>
                ))}
              </div>
            )}
          </div>

          <div className="border border-border rounded-lg overflow-hidden">
            <div className="space-y-1 max-h-64 overflow-y-auto p-3">
              {scanResult.recommended_files.map((file, i) => (
                <div key={i} className="list-item text-sm">
                  <span className="text-gruvbox-green text-xs">+</span>
                  <span className="font-mono text-xs flex-1 truncate">{file.path}</span>
                  <span className="badge badge-neutral">{file.category}</span>
                  <span className="text-secondary text-xs">{file.reason}</span>
                </div>
              ))}
            </div>
            {scanResult.skipped_files.length > 0 && (
              <div className="border-t border-border p-3">
                <p className="text-secondary text-xs mb-2">Skipped ({scanResult.skipped_files.length})</p>
                <div className="space-y-1 max-h-32 overflow-y-auto">
                  {scanResult.skipped_files.map((file, i) => (
                    <div key={i} className="list-item text-sm">
                      <span className="text-secondary text-xs">-</span>
                      <span className="text-secondary font-mono text-xs flex-1 truncate">{file.path}</span>
                      <span className="text-secondary text-xs">{file.reason}</span>
                    </div>
                  ))}
                </div>
              </div>
            )}
          </div>

          <div className="flex items-center justify-between">
            <button onClick={handleBack} className="btn-secondary" disabled={isIngesting}>← Back</button>
            <button onClick={handleIngest} disabled={isIngesting || scanResult.recommended_files.length === 0} className="btn-primary btn-lg flex items-center gap-2">
              {isIngesting ? <><span className="spinner" />Starting...</> : <>→ Proceed ({scanResult.recommended_files.length} files)</>}
            </button>
          </div>
        </>
      )}

      {ingestionStarted && (
        <div className="flex items-center justify-between">
          <p className="text-sm text-secondary">{scanResult.recommended_files.length} files queued. Track progress in header.</p>
          <button onClick={handleBack} className="btn-secondary">← Scan Another Folder</button>
        </div>
      )}
    </div>
  )
}

export default SmartFolderTab
