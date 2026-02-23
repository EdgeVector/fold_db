import { useState, useEffect, useRef, useCallback } from 'react'
import { ingestionClient } from '../../api/clients'
import FolderTreeView from './FolderTreeView'

const isTauri = typeof window !== 'undefined' && window.__TAURI_INTERNALS__

/** Format a dollar amount for display */
const fmtCost = (v) => `$${Number(v).toFixed(2)}`

function SmartFolderTab({ onResult }) {
  const [folderPath, setFolderPath] = useState('~/Documents')
  const [isScanning, setIsScanning] = useState(false)
  const [isLoadingMore, setIsLoadingMore] = useState(false)
  const [isIngesting, setIsIngesting] = useState(false)
  const [scanResult, setScanResult] = useState(null)

  // Batch tracking state
  const [batchId, setBatchId] = useState(null)
  const [batchStatus, setBatchStatus] = useState(null)
  const [spendLimit, setSpendLimit] = useState('')
  const [newLimit, setNewLimit] = useState('')

  // Autocomplete state
  const [suggestions, setSuggestions] = useState([])
  const [selectedIndex, setSelectedIndex] = useState(-1)
  const [showSuggestions, setShowSuggestions] = useState(false)
  const inputRef = useRef(null)
  const suggestionsRef = useRef(null)
  const debounceRef = useRef(null)
  const pollRef = useRef(null)
  const treeRef = useRef(null)

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
    } catch { /* autocomplete is best-effort */
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

  // Poll batch status while running or paused
  useEffect(() => {
    if (!batchId) return
    let cancelled = false
    const poll = async () => {
      try {
        const resp = await ingestionClient.getBatchStatus(batchId)
        if (cancelled) return
        if (resp.success && resp.data) {
          setBatchStatus(resp.data)
          // Store for HeaderProgress
          localStorage.setItem('activeBatchId', batchId)
          localStorage.setItem('activeBatchStatus', JSON.stringify(resp.data))
          const s = resp.data.status
          if (s === 'Completed' || s === 'Cancelled' || s === 'Failed') {
            localStorage.removeItem('activeBatchId')
            localStorage.removeItem('activeBatchStatus')
            // Stop polling once batch reaches terminal state
            if (pollRef.current) { clearInterval(pollRef.current); pollRef.current = null }
          }
        }
      } catch {
        // ignore transient errors
      }
    }
    poll()
    pollRef.current = setInterval(poll, 2000)
    return () => { cancelled = true; clearInterval(pollRef.current); pollRef.current = null }
  }, [batchId])

  const acceptSuggestion = (path) => {
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

  const handleScan = async (maxFiles) => {
    if (!folderPath.trim()) return
    setShowSuggestions(false)
    setIsScanning(true)
    setScanResult(null)
    setBatchId(null)
    setBatchStatus(null)
    onResult(null)
    try {
      const response = await ingestionClient.smartFolderScan(folderPath.trim(), 10, maxFiles)
      if (response.success) {
        setScanResult(response.data)
        // Default spend limit to total estimate
        setSpendLimit(response.data.total_estimated_cost?.toFixed(2) || '')
      } else {
        onResult({ success: false, error: 'Failed to scan folder' })
      }
    } catch (error) {
      onResult({ success: false, error: (error instanceof Error ? error.message : String(error)) || 'Failed to scan folder' })
    } finally {
      setIsScanning(false)
    }
  }

  const handleLoadMore = async () => {
    if (!folderPath.trim() || !scanResult) return
    const nextLimit = (scanResult.max_files_used || 100) + 400
    setIsLoadingMore(true)
    try {
      const response = await ingestionClient.smartFolderScan(folderPath.trim(), 10, nextLimit)
      if (response.success) {
        setScanResult(response.data)
        setSpendLimit(response.data.total_estimated_cost?.toFixed(2) || '')
      }
    } catch (error) {
      onResult({ success: false, error: (error instanceof Error ? error.message : String(error)) || 'Failed to load more files' })
    } finally {
      setIsLoadingMore(false)
    }
  }

  const handleIngest = async () => {
    if (!scanResult) return
    const files = scanResult.recommended_files
    const filePaths = files.map(f => f.path)
    const fileCosts = files.map(f => f.estimated_cost)
    if (filePaths.length === 0) { onResult({ success: false, error: 'No files recommended' }); return }
    setIsIngesting(true)
    onResult(null)
    try {
      const limit = spendLimit ? parseFloat(spendLimit) : undefined
      const response = await ingestionClient.smartFolderIngest(
        folderPath.trim(), filePaths, true, limit, fileCosts
      )
      if (response.success) {
        setBatchId(response.data.batch_id)
        onResult({ success: true, data: { message: response.data.message, batch_id: response.data.batch_id, files_found: response.data.files_found } })
      } else {
        onResult({ success: false, error: 'Failed to start ingestion' })
      }
    } catch (error) {
      onResult({ success: false, error: (error instanceof Error ? error.message : String(error)) || 'Failed to start ingestion' })
    } finally {
      setIsIngesting(false)
    }
  }

  const handleResume = async () => {
    if (!batchId) return
    const limit = parseFloat(newLimit)
    if (isNaN(limit) || limit <= 0) return
    try {
      await ingestionClient.resumeBatch(batchId, limit)
    } catch (error) {
      onResult({ success: false, error: (error instanceof Error ? error.message : String(error)) || 'Failed to resume' })
    }
  }

  const handleCancel = async () => {
    if (!batchId) return
    try {
      await ingestionClient.cancelBatch(batchId)
      localStorage.removeItem('activeBatchId')
      localStorage.removeItem('activeBatchStatus')
    } catch (error) {
      onResult({ success: false, error: (error instanceof Error ? error.message : String(error)) || 'Failed to cancel' })
    }
  }

  const handleBack = async () => {
    // Cancel any active batch before clearing state
    // Also cancel when batchStatus is null (batch started but first poll hasn't returned)
    if (batchId && (!batchStatus || batchStatus.status === 'Running' || batchStatus.status === 'Paused')) {
      try { await ingestionClient.cancelBatch(batchId) } catch { /* best-effort */ }
    }
    setScanResult(null)
    setBatchId(null)
    setBatchStatus(null)
    setSpendLimit('')
    localStorage.removeItem('activeBatchId')
    localStorage.removeItem('activeBatchStatus')
    onResult(null)
  }

  // Pre-fill new limit when paused
  useEffect(() => {
    if (batchStatus?.status === 'Paused') {
      const suggested = batchStatus.accumulated_cost + batchStatus.estimated_remaining_cost
      setNewLimit(suggested.toFixed(2))
    }
  }, [batchStatus?.status, batchStatus?.accumulated_cost, batchStatus?.estimated_remaining_cost])

  const estimatedCost = scanResult?.total_estimated_cost

  // Derive UI state
  const isRunning = batchStatus?.status === 'Running'
  const isPaused = batchStatus?.status === 'Paused'
  const isCompleted = batchStatus?.status === 'Completed'
  const isCancelled = batchStatus?.status === 'Cancelled'
  const isFailed = batchStatus?.status === 'Failed'
  const isDone = isCompleted || isCancelled || isFailed

  // --- RENDER ---
  return (
    <div className="space-y-4">
      {/* State 0: Folder input (no scan yet, no batch) */}
      {!scanResult && !batchId && (
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
          {isTauri && <button onClick={openFolderPicker} disabled={isScanning} className="btn-secondary" title="Browse">Browse</button>}
          <button onClick={() => handleScan()} disabled={isScanning || !folderPath.trim()} className="btn-primary flex items-center gap-2">
            {isScanning ? <><span className="spinner" />Scanning...</> : <>Scan</>}
          </button>
        </div>
      )}

      {/* State 1: Scan results (before Proceed) */}
      {scanResult && !batchId && (
        <>
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-6 text-sm">
              <span className="text-primary font-medium">{scanResult.recommended_files.length} files to ingest</span>
              {scanResult.skipped_files.filter(f => f.already_ingested).length > 0 && (
                <span className="text-gruvbox-blue">{scanResult.skipped_files.filter(f => f.already_ingested).length} already ingested</span>
              )}
              <span className="text-secondary">{scanResult.skipped_files.filter(f => !f.already_ingested).length} skipped</span>
              <span className="text-secondary">{scanResult.total_files} total</span>
            </div>
            {Object.keys(scanResult.summary).length > 0 && (
              <div className="flex gap-2 flex-wrap">
                {Object.entries(scanResult.summary).filter(([, count]) => count > 0).map(([cat, count]) => (
                  <span key={cat} className="badge badge-neutral">{cat.replace(/_/g, ' ')}: {count}</span>
                ))}
              </div>
            )}
          </div>

          {/* Cost estimate & spend limit */}
          <div className="flex items-center gap-4 text-sm">
            <span className="text-secondary">Estimated cost: <span className="text-primary font-medium">~{fmtCost(estimatedCost)}</span></span>
            <label className="flex items-center gap-2 text-secondary">
              Spend limit:
              <input
                type="text"
                value={spendLimit}
                onChange={(e) => setSpendLimit(e.target.value)}
                className="input w-24 text-sm"
                placeholder="no limit"
              />
            </label>
          </div>

          {/* Truncation warning with Load more */}
          {scanResult.scan_truncated && (
            <div className="bg-gruvbox-yellow/10 border border-gruvbox-yellow/30 rounded-lg px-3 py-2 text-sm text-gruvbox-yellow flex items-center justify-between">
              <span>Showing {scanResult.max_files_used} of more files. Some files may not be shown.</span>
              <button onClick={handleLoadMore} disabled={isLoadingMore} className="btn-secondary text-xs ml-3 flex items-center gap-1">
                {isLoadingMore ? <><span className="spinner" />Loading...</> : 'Load more'}
              </button>
            </div>
          )}

          {/* Folder tree controls */}
          <div className="flex items-center gap-2 text-xs text-secondary">
            <button onClick={() => treeRef.current?.expandAll()} className="hover:text-primary underline">Expand all</button>
            <span>·</span>
            <button onClick={() => treeRef.current?.collapseAll()} className="hover:text-primary underline">Collapse all</button>
          </div>

          {/* Folder tree */}
          <FolderTreeView
            ref={treeRef}
            recommendedFiles={scanResult.recommended_files}
            skippedFiles={scanResult.skipped_files}
          />

          <div className="flex items-center justify-between">
            <button onClick={handleBack} className="btn-secondary" disabled={isIngesting}>Back</button>
            <button onClick={handleIngest} disabled={isIngesting || scanResult.recommended_files.length === 0} className="btn-primary btn-lg flex items-center gap-2">
              {isIngesting ? <><span className="spinner" />Starting...</> : <>Proceed ({scanResult.recommended_files.length} files)</>}
            </button>
          </div>
        </>
      )}

      {/* State 2: Ingestion running */}
      {batchId && isRunning && batchStatus && (
        <div className="space-y-3">
          <p className="text-sm font-medium">Ingesting files...</p>
          <div className="w-full bg-border rounded-full h-2 overflow-hidden">
            <div
              className="h-full bg-primary transition-all duration-300"
              style={{ width: `${batchStatus.files_total > 0 ? Math.round((batchStatus.files_completed / batchStatus.files_total) * 100) : 0}%` }}
            />
          </div>
          <div className="flex items-center justify-between text-sm text-secondary">
            <span>{batchStatus.files_completed}/{batchStatus.files_total} files{batchStatus.files_failed > 0 ? ` (${batchStatus.files_failed} failed)` : ''}</span>
            <span>{fmtCost(batchStatus.accumulated_cost)} spent{batchStatus.spend_limit != null ? ` / ${fmtCost(batchStatus.spend_limit)} limit` : ''}</span>
          </div>
          <div className="flex justify-end gap-2">
            <button onClick={handleCancel} className="btn-secondary">Cancel</button>
            <button onClick={handleBack} className="btn-secondary">Scan Another</button>
          </div>
        </div>
      )}

      {/* State 3: Paused (spend limit reached) */}
      {batchId && isPaused && batchStatus && (
        <div className="space-y-3">
          <p className="text-sm font-medium text-gruvbox-yellow">Paused -- spend limit reached</p>
          <div className="w-full bg-border rounded-full h-2 overflow-hidden">
            <div
              className="h-full bg-gruvbox-yellow transition-all duration-300"
              style={{ width: `${batchStatus.files_total > 0 ? Math.round((batchStatus.files_completed / batchStatus.files_total) * 100) : 0}%` }}
            />
          </div>
          <div className="flex items-center justify-between text-sm text-secondary">
            <span>{batchStatus.files_completed}/{batchStatus.files_total} files</span>
            <span>{fmtCost(batchStatus.accumulated_cost)} spent / {fmtCost(batchStatus.spend_limit)} limit</span>
          </div>
          <p className="text-sm text-secondary">
            {batchStatus.files_remaining} files remaining (~{fmtCost(batchStatus.estimated_remaining_cost)} to finish)
          </p>
          <div className="flex items-center gap-3">
            <label className="flex items-center gap-2 text-sm text-secondary">
              New limit:
              <input
                type="text"
                value={newLimit}
                onChange={(e) => setNewLimit(e.target.value)}
                className="input w-24 text-sm"
              />
            </label>
            <button onClick={handleResume} className="btn-primary">Resume</button>
            <button onClick={handleCancel} className="btn-secondary">Stop</button>
          </div>
        </div>
      )}

      {/* State 4: Completed / Cancelled / Failed */}
      {batchId && isDone && batchStatus && (
        <div className="space-y-3">
          <p className="text-sm font-medium">
            {isCompleted && 'Ingestion complete'}
            {isCancelled && 'Ingestion cancelled'}
            {isFailed && 'Ingestion failed'}
          </p>
          <p className="text-sm text-secondary">
            {batchStatus.files_completed} files ingested
            {batchStatus.files_failed > 0 ? ` (${batchStatus.files_failed} failed)` : ''}
            {' · '}{fmtCost(batchStatus.accumulated_cost)} spent
          </p>
          <div className="flex justify-end">
            <button onClick={handleBack} className="btn-secondary">Scan Another</button>
          </div>
        </div>
      )}

      {/* Waiting for first batch status poll */}
      {batchId && !batchStatus && (
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2 text-sm text-secondary">
            <span className="spinner" /> Starting batch...
          </div>
          <button onClick={handleBack} className="btn-secondary text-sm">Cancel</button>
        </div>
      )}
    </div>
  )
}

export default SmartFolderTab
