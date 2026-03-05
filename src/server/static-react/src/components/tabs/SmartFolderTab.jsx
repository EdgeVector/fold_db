import { useState, useEffect, useRef, useCallback } from 'react'
import { toErrorMessage } from '../../utils/schemaUtils'
import { ingestionClient } from '../../api/clients'
import { useFolderAutocomplete } from '../../hooks/useFolderAutocomplete'
import { useScanPolling } from '../../hooks/useScanPolling'
import { useBatchMonitor } from '../../hooks/useBatchMonitor'
import FolderInput from './smart-folder/FolderInput'
import ScanResultsView from './smart-folder/ScanResultsView'
import BatchProgressView from './smart-folder/BatchProgressView'

const STORAGE_KEY = 'smartFolderTabState'

/** Load persisted SmartFolderTab state from localStorage.
 *  Only restores state when there's an active batch or scan to reconnect to.
 *  Completed/stale sessions start fresh. */
function loadPersistedState() {
  try {
    const raw = localStorage.getItem(STORAGE_KEY)
    if (!raw) return null
    const state = JSON.parse(raw)
    const hasActiveBatch = state.batchId && localStorage.getItem('activeBatchId')
    const hasActiveScan = !!state.scanProgressId
    if (!hasActiveBatch && !hasActiveScan) {
      localStorage.removeItem(STORAGE_KEY)
      return null
    }
    return state
  } catch { return null }
}

function persistState(state) {
  try { localStorage.setItem(STORAGE_KEY, JSON.stringify(state)) } catch { /* best-effort */ }
}

function clearPersistedState() {
  localStorage.removeItem(STORAGE_KEY)
}

function SmartFolderTab({ onResult: onResultProp }) {
  // Stabilize the onResult callback so polling effects don't restart on every parent render
  const onResultRef = useRef(onResultProp)
  useEffect(() => { onResultRef.current = onResultProp })
  const onResult = useCallback((...args) => onResultRef.current(...args), [])

  // Restore persisted state on mount
  const [restored] = useState(() => loadPersistedState())

  const [folderPath, setFolderPath] = useState(() => restored?.folderPath || '~/Documents')
  const [isScanning, setIsScanning] = useState(() => !!restored?.scanProgressId)
  const [isLoadingMore, setIsLoadingMore] = useState(false)
  const [isIngesting, setIsIngesting] = useState(false)
  const [scanResult, setScanResult] = useState(() => restored?.scanResult || null)
  const [batchId, setBatchId] = useState(() => restored?.batchId || null)
  const [spendLimit, setSpendLimit] = useState(() => restored?.spendLimit || '')
  const [fileProgressIds, setFileProgressIds] = useState(() => restored?.fileProgressIds || [])
  const [scanProgressId, setScanProgressId] = useState(() => restored?.scanProgressId || null)
  const [includeAlreadyIngested, setIncludeAlreadyIngested] = useState(() => !!restored?.includeAlreadyIngested)

  // Ref for batchStatus so handleBack can read it without a stale closure
  const batchStatusRef = useRef(null)

  // Persist key state whenever it changes
  useEffect(() => {
    if (!scanProgressId && !scanResult && !batchId) {
      clearPersistedState()
      return
    }
    persistState({ folderPath, scanProgressId, scanResult, batchId, spendLimit, includeAlreadyIngested, fileProgressIds })
  }, [folderPath, scanProgressId, scanResult, batchId, spendLimit, includeAlreadyIngested, fileProgressIds])

  // --- Handlers ---

  const startScan = useCallback(async (maxFiles) => {
    if (!folderPath.trim()) return
    setIsScanning(true)
    setScanResult(null)
    onResult(null)
    try {
      const response = await ingestionClient.smartFolderScan(folderPath.trim(), 10, maxFiles)
      if (response.success && response.data?.progress_id) {
        setScanProgressId(response.data.progress_id)
      } else {
        onResult({ success: false, error: 'Failed to start scan' })
        setIsScanning(false)
      }
    } catch (error) {
      onResult({ success: false, error: toErrorMessage(error) || 'Failed to scan folder' })
      setIsScanning(false)
    }
  }, [folderPath, onResult])

  const handleScan = useCallback(async (maxFiles) => {
    setBatchId(null)
    setIncludeAlreadyIngested(false)
    await startScan(maxFiles)
  }, [startScan])

  const handleScanComplete = useCallback((result) => {
    setScanResult(result)
    setSpendLimit(result.total_estimated_cost?.toFixed(2) || '')
    setScanProgressId(null)
    setIsScanning(false)
  }, [])

  const handleScanFail = useCallback((msg) => {
    onResult({ success: false, error: msg })
    setScanProgressId(null)
    setIsScanning(false)
  }, [onResult])

  const handleBatchLost = useCallback(() => {
    setBatchId(null)
    clearPersistedState()
  }, [])

  const handleBatchTerminal = useCallback(() => {
    clearPersistedState()
  }, [])

  const handleCancelScan = useCallback(() => {
    setScanProgressId(null)
    setIsScanning(false)
    clearPersistedState()
  }, [])

  const handleLoadMore = useCallback(async () => {
    if (!scanResult) return
    const nextLimit = (scanResult.max_files_used || 100) + 400
    setIsLoadingMore(true)
    await startScan(nextLimit)
    setIsLoadingMore(false)
  }, [scanResult, startScan])

  const handleIngest = useCallback(async () => {
    if (!scanResult) return
    const files = includeAlreadyIngested
      ? [...scanResult.recommended_files, ...scanResult.skipped_files.filter(f => f.already_ingested)]
      : scanResult.recommended_files
    const filePaths = files.map(f => f.path)
    const fileCosts = files.map(f => f.estimated_cost)
    if (filePaths.length === 0) { onResult({ success: false, error: 'No files recommended' }); return }
    setIsIngesting(true)
    onResult(null)
    try {
      const limit = spendLimit ? parseFloat(spendLimit) : undefined
      const response = await ingestionClient.smartFolderIngest(
        folderPath.trim(), filePaths, true, limit, fileCosts, includeAlreadyIngested
      )
      if (response.success) {
        setBatchId(response.data.batch_id)
        setFileProgressIds(response.data.file_progress_ids || [])
        onResult({ success: true, data: { message: response.data.message, batch_id: response.data.batch_id, files_found: response.data.files_found } })
      } else {
        onResult({ success: false, error: 'Failed to start ingestion' })
      }
    } catch (error) {
      onResult({ success: false, error: toErrorMessage(error) || 'Failed to start ingestion' })
    } finally {
      setIsIngesting(false)
    }
  }, [scanResult, includeAlreadyIngested, spendLimit, folderPath, onResult])

  const handleResume = useCallback(async (limit) => {
    if (!batchId) return
    try {
      await ingestionClient.resumeBatch(batchId, limit)
    } catch (error) {
      onResult({ success: false, error: toErrorMessage(error) || 'Failed to resume' })
    }
  }, [batchId, onResult])

  const handleCancel = useCallback(async () => {
    if (!batchId) return
    try {
      await ingestionClient.cancelBatch(batchId)
      localStorage.removeItem('activeBatchId')
      localStorage.removeItem('activeBatchStatus')
    } catch (error) {
      onResult({ success: false, error: toErrorMessage(error) || 'Failed to cancel' })
    }
  }, [batchId, onResult])

  const handleBack = useCallback(async () => {
    const status = batchStatusRef.current
    if (batchId && (!status || status.status === 'Running' || status.status === 'Paused')) {
      try { await ingestionClient.cancelBatch(batchId) } catch { /* best-effort */ }
    }
    setScanResult(null)
    setScanProgressId(null)
    setBatchId(null)
    setSpendLimit('')
    setFileProgressIds([])
    clearPersistedState()
    localStorage.removeItem('activeBatchId')
    localStorage.removeItem('activeBatchStatus')
    onResult(null)
  }, [batchId, onResult])

  // --- Hooks ---

  const autocomplete = useFolderAutocomplete({
    folderPath,
    isDisabled: isScanning,
    onFolderPathChange: setFolderPath,
    onSubmit: () => handleScan(),
  })

  const { scanProgress } = useScanPolling({
    scanProgressId,
    onComplete: handleScanComplete,
    onFail: handleScanFail,
  })

  const { batchStatus, batchReport, setBatchReport } = useBatchMonitor({
    batchId,
    fileProgressIds,
    onBatchLost: handleBatchLost,
    onTerminal: handleBatchTerminal,
  })
  batchStatusRef.current = batchStatus

  // --- Render ---

  return (
    <div className="space-y-4">
      {!scanResult && !batchId && (
        <FolderInput
          folderPath={folderPath}
          onFolderPathChange={setFolderPath}
          onScan={handleScan}
          onCancelScan={handleCancelScan}
          isScanning={isScanning}
          scanProgress={scanProgress}
          autocomplete={autocomplete}
        />
      )}

      {scanResult && !batchId && (
        <ScanResultsView
          scanResult={scanResult}
          spendLimit={spendLimit}
          onSpendLimitChange={setSpendLimit}
          includeAlreadyIngested={includeAlreadyIngested}
          onIncludeChange={setIncludeAlreadyIngested}
          onIngest={handleIngest}
          onLoadMore={handleLoadMore}
          onBack={handleBack}
          isIngesting={isIngesting}
          isLoadingMore={isLoadingMore}
        />
      )}

      {batchId && (
        <BatchProgressView
          batchStatus={batchStatus}
          batchReport={batchReport}
          onResume={handleResume}
          onCancel={handleCancel}
          onBack={handleBack}
          onDismissReport={() => setBatchReport(null)}
          isIngesting={isIngesting}
        />
      )}
    </div>
  )
}

export default SmartFolderTab
