import { useEffect, useRef } from 'react'

/**
 * Generic interval-based polling hook with consecutive failure tracking.
 *
 * @param {Object} opts
 * @param {*} opts.key - Truthy value to enable polling (null/undefined = idle)
 * @param {Function} opts.pollFn - Async function called each interval. Receives no args.
 *   Should return { stop: true } to halt polling, or undefined/anything else to continue.
 * @param {number} opts.intervalMs - Polling interval in milliseconds
 * @param {number} opts.maxFailures - Stop polling after this many consecutive failures
 * @param {Function} opts.onMaxFailures - Called when maxFailures is reached
 */
export function usePolling({ key, pollFn, intervalMs, maxFailures, onMaxFailures }) {
  const pollFnRef = useRef(pollFn)
  const onMaxFailuresRef = useRef(onMaxFailures)
  useEffect(() => { pollFnRef.current = pollFn })
  useEffect(() => { onMaxFailuresRef.current = onMaxFailures })

  useEffect(() => {
    if (!key) return
    let cancelled = false
    let failCount = 0
    let timer = null

    const tick = async () => {
      try {
        const result = await pollFnRef.current()
        if (cancelled) return
        failCount = 0
        if (result?.stop) {
          if (timer) { clearInterval(timer); timer = null }
        }
      } catch {
        if (cancelled) return
        failCount++
        if (failCount >= maxFailures) {
          onMaxFailuresRef.current?.()
          if (timer) { clearInterval(timer); timer = null }
        }
      }
    }

    tick()
    timer = setInterval(tick, intervalMs)
    return () => { cancelled = true; if (timer) clearInterval(timer) }
  }, [key, intervalMs, maxFailures])
}
