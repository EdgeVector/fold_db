import { useEffect, useRef, useState } from 'react'
import { systemClient } from '../api/clients/systemClient'

function LogSidebar() {
  const [logs, setLogs] = useState([])
  const endRef = useRef(null)

  const formatLog = (entry) => {
    if (typeof entry === 'string') return entry
    const meta = entry.metadata ? JSON.stringify(entry.metadata) : ''
    // Format: [LEVEL] [TYPE] - message (metadata)
    // Matches StdoutLogger roughly
    return `[${entry.level}] [${entry.event_type}] - ${entry.message} ${meta}`
  }

  const handleCopy = () => {
    Promise.resolve(
      navigator.clipboard.writeText(logs.map(formatLog).join('\n'))
    ).catch(() => {})
  }

  useEffect(() => {
    // Load initial logs using systemClient
    systemClient.getLogs()
      .then(response => {
        if (response.success && response.data) {
          const fetchedLogs = response.data.logs || []
          // Ensure logs are in chronological order (backend logic dependent, but assuming we fixed it)
          setLogs(Array.isArray(fetchedLogs) ? fetchedLogs : [])
        } else {
          setLogs([])
        }
      })
      .catch(() => setLogs([]))

    // Set up log streaming using systemClient
    const eventSource = systemClient.createLogStream(
      (message) => {
        setLogs(prev => {
          let entry;
          try {
            // New format: message is a JSON string of LogEntry
            entry = JSON.parse(message);
          } catch {
            // Fallback for legacy format: "LEVEL - message" or raw string
            const parts = message.split(' - ')
            const level = parts.length > 1 ? parts[0] : 'INFO';
            // Simple heuristic to strip [timestamp] if present in legacy format
            // But usually raw message doesn't have it if it came from legacy web logger
            // unless formatted.
            
            entry = {
              id: `stream-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
              timestamp: Date.now(),
              level: level,
              event_type: 'stream (legacy)',
              message: message
            };
          }

          // Deduplication:
          // Check if we already have this ID
          if (entry.id && prev.some(existing => existing.id === entry.id)) {
             return prev;
          }

          return [...prev, entry]
        })
      },
      (error) => {
        console.warn('Log stream error:', error)
      }
    )

    // Poll for new logs
    const pollInterval = setInterval(() => {
      setLogs(currentLogs => {
        const lastLog = currentLogs.length > 0 ? currentLogs[currentLogs.length - 1] : null
        const since = lastLog ? lastLog.timestamp : undefined

        systemClient.getLogs(since)
          .then(response => {
            if (response.success && response.data) {
              const fetchedLogs = response.data.logs || []
              if (fetchedLogs.length > 0) {
                 setLogs(cur => {
                   // Filter duplicates based on ID
                   
                   const newLogs = fetchedLogs.filter(log => {
                     // Strict deduplication by ID
                     if (log.id) {
                       const exists = cur.some(existing => existing.id === log.id);
                       if (exists) return false;
                     } 
                     
                     // Fallback for missing IDs (shouldn't happen with new backend)
                     // or if we somehow have a log without ID in cur
                     const isTypesSame = cur.some(existing => 
                         !existing.id && // Only check content if existing has no ID
                         existing.timestamp === log.timestamp && 
                         existing.message === log.message
                     );
                     
                     return !isTypesSame;
                   })
                   
                   if (newLogs.length === 0) return cur
                   return [...cur, ...newLogs]
                 })
              }
            }
          })
          .catch(err => console.warn('Log polling error:', err))
        
        return currentLogs
      })
    }, 2000)

    return () => {
      eventSource.close()
      clearInterval(pollInterval)
    }
  }, [])

  useEffect(() => {
    endRef.current?.scrollIntoView({ behavior: 'smooth' })
  }, [logs])

  return (
    <aside className="w-80 bg-gray-900 text-white flex flex-col overflow-hidden">
      <div className="flex items-center justify-between p-4 border-b border-gray-700">
        <h2 className="text-lg font-semibold">Logs</h2>
        <button
          onClick={handleCopy}
          className="text-xs text-blue-300 hover:underline"
        >
          Copy
        </button>
      </div>
      <div className="flex-1 overflow-y-auto p-4 space-y-1 text-xs font-mono">
        {logs.map((entry, idx) => (
          <div key={entry.id || idx}>{formatLog(entry)}</div>
        ))}
        <div ref={endRef}></div>
      </div>
    </aside>
  )
}

export default LogSidebar
