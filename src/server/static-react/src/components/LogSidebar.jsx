import { useEffect, useRef, useState } from 'react'
import { systemClient } from '../api/clients/systemClient'

function LogSidebar() {
  const [logs, setLogs] = useState([])
  const [isCollapsed, setIsCollapsed] = useState(true) // Start collapsed
  const logContainerRef = useRef(null)

  const formatLog = (entry) => {
    if (typeof entry === 'string') return entry
    const meta = entry.metadata ? JSON.stringify(entry.metadata) : ''
    return `[${entry.level}] [${entry.event_type}] - ${entry.message} ${meta}`
  }

  const getLevelColor = (level) => {
    switch (level?.toUpperCase()) {
      case 'ERROR':
        return 'text-error'
      case 'WARN':
      case 'WARNING':
        return 'text-warning'
      case 'INFO':
        return 'text-secondary'
      case 'DEBUG':
        return 'text-tertiary'
      default:
        return 'text-tertiary'
    }
  }

  const formatLogEntry = (entry) => {
    if (typeof entry === 'string') {
      return <span className="text-tertiary">{entry}</span>
    }

    const levelClass = getLevelColor(entry.level)
    const time = entry.timestamp
      ? new Date(entry.timestamp).toLocaleTimeString('en-US', { hour12: false })
      : ''

    return (
      <>
        <span className="text-tertiary">{time}</span>
        <span className={`${levelClass} ml-2`}>[{entry.level}]</span>
        <span className="text-secondary ml-1">{entry.message}</span>
      </>
    )
  }

  const handleCopy = () => {
    Promise.resolve(
      navigator.clipboard.writeText(logs.map(formatLog).join('\n'))
    ).catch(() => {})
  }

  const handleClear = () => {
    setLogs([])
  }

  useEffect(() => {
    // Load initial logs using systemClient
    systemClient.getLogs()
      .then(response => {
        if (response.success && response.data) {
          const fetchedLogs = response.data.logs || []
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
            entry = JSON.parse(message);
          } catch {
            const parts = message.split(' - ')
            const level = parts.length > 1 ? parts[0] : 'INFO';

            entry = {
              id: `stream-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
              timestamp: Date.now(),
              level: level,
              event_type: 'stream (legacy)',
              message: message
            };
          }

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
                   const newLogs = fetchedLogs.filter(log => {
                     if (log.id) {
                       const exists = cur.some(existing => existing.id === log.id);
                       if (exists) return false;
                     }

                     const isTypesSame = cur.some(existing =>
                         !existing.id &&
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

  // Auto-scroll only within the log container, not the page
  useEffect(() => {
    if (logContainerRef.current) {
      logContainerRef.current.scrollTop = logContainerRef.current.scrollHeight
    }
  }, [logs])

  if (isCollapsed) {
    return (
      <aside className="log-sidebar-collapsed">
        <button
          onClick={() => setIsCollapsed(false)}
          className="log-sidebar-toggle"
          title="Expand logs"
        >
          <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 19l-7-7 7-7" />
          </svg>
        </button>
      </aside>
    )
  }

  return (
    <aside className="log-sidebar">
      {/* Header */}
      <div className="log-sidebar-header">
        <div className="flex items-center gap-2">
          <span className="text-sm font-medium text-primary">Logs</span>
          <span className="log-sidebar-count">{logs.length}</span>
        </div>
        <div className="flex items-center gap-3">
          <button onClick={handleClear} className="log-sidebar-action">
            clear
          </button>
          <button onClick={handleCopy} className="log-sidebar-action">
            copy
          </button>
          <button
            onClick={() => setIsCollapsed(true)}
            className="log-sidebar-action p-1"
            title="Collapse"
          >
            <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 5l7 7-7 7" />
            </svg>
          </button>
        </div>
      </div>

      {/* Log content - fixed height with internal scroll */}
      <div ref={logContainerRef} className="log-sidebar-content">
        {logs.length === 0 ? (
          <div className="text-tertiary text-center py-8">
            No logs yet
          </div>
        ) : (
          logs.map((entry, idx) => (
            <div key={entry.id || idx} className="log-entry">
              {formatLogEntry(entry)}
            </div>
          ))
        )}
      </div>

      {/* Status bar */}
      <div className="log-sidebar-status">
        <div className="flex items-center gap-1.5">
          <span className="log-status-dot"></span>
          <span>streaming</span>
        </div>
        <span>{logs.length} entries</span>
      </div>
    </aside>
  )
}

export default LogSidebar
