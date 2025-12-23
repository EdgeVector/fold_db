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
    // Note: SSE returns strings, we convert to pseudo-entries for consistency
    const eventSource = systemClient.createLogStream(
      (message) => {
        setLogs(prev => {
          // Parse string "LEVEL - message" if possible, or wrap
          // This is a rough heuristic matching legacy WebLogger format
          const parts = message.split(' - ')
          const level = parts.length > 1 ? parts[0] : 'INFO'
          const msg = parts.length > 1 ? parts.slice(1).join(' - ') : message
          
          const entry = {
            timestamp: Date.now(),
            level: level,
            event_type: 'stream',
            message: msg
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
                   // Filter duplicates based on timestamp and content
                   // We assume backend returns logs >= since
                   const lastTs = cur.length > 0 ? cur[cur.length - 1].timestamp : 0
                   
                   const newLogs = fetchedLogs.filter(log => {
                     // Keep if newer, or if same time but different message (imperfect dedup)
                     if (log.timestamp > lastTs) return true
                     if (log.timestamp === lastTs) {
                        // Check if this log is already in cur
                        const isDuplicate = cur.some(existing => 
                          existing.timestamp === log.timestamp && 
                          existing.message === log.message &&
                          existing.event_type === log.event_type
                        )
                        return !isDuplicate
                     }
                     return false
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
          <div key={idx}>{formatLog(entry)}</div>
        ))}
        <div ref={endRef}></div>
      </div>
    </aside>
  )
}

export default LogSidebar
