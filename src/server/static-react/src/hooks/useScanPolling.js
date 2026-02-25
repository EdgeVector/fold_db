import { useState, useEffect, useRef } from 'react'
import { ingestionClient } from '../api/clients'

/**
 * Polls scan progress by ID and fetches the final scan result on completion.
 *
 * @param {Object} opts
 * @param {string|null} opts.scanProgressId - Progress ID to poll (null = idle)
 * @param {Function} opts.onComplete - Called with scan result data on success
 * @param {Function} opts.onFail - Called with error message on failure
 */
export function useScanPolling({ scanProgressId, onComplete, onFail }) {
  const [scanProgress, setScanProgress] = useState(null)
  const onCompleteRef = useRef(onComplete)
  const onFailRef = useRef(onFail)
  useEffect(() => { onCompleteRef.current = onComplete })
  useEffect(() => { onFailRef.current = onFail })

  useEffect(() => {
    if (!scanProgressId) {
      setScanProgress(null)
      return
    }
    let cancelled = false
    let failCount = 0
    let pollTimer = null

    const poll = async () => {
      try {
        const resp = await ingestionClient.getJobProgress(scanProgressId)
        if (cancelled) return
        if (resp.success && resp.data) {
          failCount = 0
          setScanProgress(resp.data)
          if (resp.data.is_complete && !resp.data.is_failed) {
            const result = await ingestionClient.getScanResult(scanProgressId)
            if (!cancelled && result.success && result.data) {
              onCompleteRef.current(result.data)
            }
            setScanProgress(null)
          } else if (resp.data.is_failed) {
            if (!cancelled) onFailRef.current(resp.data.error_message || 'Scan failed')
            setScanProgress(null)
          }
        } else {
          failCount++
          if (failCount >= 5) {
            if (!cancelled) onFailRef.current('Lost connection to scan job')
            setScanProgress(null)
          }
        }
      } catch {
        if (cancelled) return
        failCount++
        if (failCount >= 5) {
          if (!cancelled) onFailRef.current('Lost connection to scan job')
          setScanProgress(null)
        }
      }
    }

    poll()
    pollTimer = setInterval(poll, 1000)
    return () => { cancelled = true; clearInterval(pollTimer) }
  }, [scanProgressId])

  return { scanProgress }
}
