import { useState, useEffect, useRef, useCallback } from 'react'
import { ingestionClient } from '../api/clients'
import { usePolling } from './usePolling'

/**
 * Polls batch status and aggregates process results when a batch reaches
 * a terminal state (Completed / Cancelled / Failed).
 */
export function useBatchMonitor({ batchId, fileProgressIds, onBatchLost, onTerminal }) {
  const [batchStatus, setBatchStatus] = useState(null)
  const [batchReport, setBatchReport] = useState(null)
  const onTerminalRef = useRef(onTerminal)
  useEffect(() => { onTerminalRef.current = onTerminal })

  const pollFn = useCallback(async () => {
    const resp = await ingestionClient.getBatchStatus(batchId)
    if (!resp.success || !resp.data) throw new Error('poll failed')
    setBatchStatus(resp.data)
    localStorage.setItem('activeBatchId', batchId)
    localStorage.setItem('activeBatchStatus', JSON.stringify(resp.data))
    const s = resp.data.status
    if (s === 'Completed' || s === 'Cancelled' || s === 'Failed') {
      localStorage.removeItem('activeBatchId')
      localStorage.removeItem('activeBatchStatus')
      onTerminalRef.current()
      return { stop: true }
    }
  }, [batchId])

  const onMaxFailures = useCallback(() => {
    setBatchStatus(null)
    localStorage.removeItem('activeBatchId')
    localStorage.removeItem('activeBatchStatus')
    // onBatchLost is stable via usePolling's ref pattern
  }, [])

  const onBatchLostRef = useRef(onBatchLost)
  useEffect(() => { onBatchLostRef.current = onBatchLost })

  const handleMaxFailures = useCallback(() => {
    onMaxFailures()
    onBatchLostRef.current()
  }, [onMaxFailures])

  usePolling({
    key: batchId,
    pollFn,
    intervalMs: 2000,
    maxFailures: 2,
    onMaxFailures: handleMaxFailures,
  })

  // Reset when batchId is cleared
  useEffect(() => {
    if (!batchId) { setBatchStatus(null); setBatchReport(null) }
  }, [batchId])

  // Aggregate process results when batch completes
  useEffect(() => {
    if (!batchId || !batchStatus) return
    const s = batchStatus.status
    if (s !== 'Completed' && s !== 'Cancelled') return
    if (batchReport || fileProgressIds.length === 0) return

    let cancelled = false
    let retryCount = 0
    const MAX_RETRIES = 5
    const RETRY_DELAY_MS = 2000

    const attempt = async () => {
      if (cancelled) return
      try {
        const merged = {}
        let totalGen = 0
        let totalExec = 0
        let anyNew = false

        const progressResp = await ingestionClient.getAllProgress()
        const progressList = Array.isArray(progressResp.data?.progress) ? progressResp.data.progress
          : Array.isArray(progressResp.data) ? progressResp.data : []
        const idSet = new Set(fileProgressIds.map(f => f.progress_id))
        for (const job of progressList) {
          if (!idSet.has(job.id)) continue
          const r = job.results
          if (!r) continue
          totalGen += r.mutations_generated || 0
          totalExec += r.mutations_executed || 0
          if (r.new_schema_created) anyNew = true
        }

        for (const file of fileProgressIds) {
          if (cancelled) return
          try {
            const resp = await ingestionClient.getProcessResults(file.progress_id)
            if (!resp.success) continue
            const results = resp.data?.results || []
            for (const r of results) {
              if (!merged[r.schema_name]) merged[r.schema_name] = []
              merged[r.schema_name].push(r.key_value)
            }
          } catch { /* skip failed fetches */ }
        }

        const schemasWritten = Object.entries(merged).map(([name, keys]) => ({
          schema_name: name,
          keys_written: keys,
        }))
        if (cancelled) return
        if (schemasWritten.length > 0) {
          setBatchReport({
            success: true,
            data: {
              schemas_written: schemasWritten,
              mutations_generated: totalGen,
              mutations_executed: totalExec,
              new_schema_created: anyNew,
            },
          })
        } else if (retryCount < MAX_RETRIES) {
          retryCount++
          setTimeout(attempt, RETRY_DELAY_MS)
        }
      } catch {
        if (!cancelled && retryCount < MAX_RETRIES) {
          retryCount++
          setTimeout(attempt, RETRY_DELAY_MS)
        }
      }
    }

    const timer = setTimeout(attempt, 1000)
    return () => { cancelled = true; clearTimeout(timer) }
  }, [batchId, batchStatus, batchReport, fileProgressIds])

  return { batchStatus, batchReport, setBatchReport }
}
