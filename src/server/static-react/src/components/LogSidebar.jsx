import { useEffect, useRef, useState } from 'react'
import { systemClient } from '../api/clients/systemClient'

function LogSidebar() {
  const [logs, setLogs] = useState([])
  const [isCollapsed, setIsCollapsed] = useState(false)
  const endRef = useRef(null)

  const formatLog = (entry) => {
    if (typeof entry === 'string') return entry
    const meta = entry.metadata ? JSON.stringify(entry.metadata) : ''
    return `[${entry.level}] [${entry.event_type}] - ${entry.message} ${meta}`
  }

  const getLevelColor = (level) => {
    switch (level?.toUpperCase()) {
      case 'ERROR':
        return 'text-terminal-red'
      case 'WARN':
      case 'WARNING':
        return 'text-terminal-yellow'
      case 'INFO':
        return 'text-terminal-blue'
      case 'DEBUG':
        return 'text-terminal-purple'
      default:
        return 'text-terminal-dim'
    }
  }

  const formatLogEntry = (entry) => {
    if (typeof entry === 'string') {
      return <span className="text-terminal-dim">{entry}</span>
    }
    
    const levelColor = getLevelColor(entry.level)
    const time = entry.timestamp 
      ? new Date(entry.timestamp).toLocaleTimeString('en-US', { hour12: false })
      : ''
    
    return (
      <>
        <span className="text-terminal-dim">{time}</span>
        <span className={`ml-2 ${levelColor}`}>[{entry.level}]</span>
        <span className="text-terminal-cyan ml-1">[{entry.event_type}]</span>
        <span className="text-terminal ml-1">{entry.message}</span>
        {entry.metadata && (
          <span className="text-terminal-dim ml-1">
            {JSON.stringify(entry.metadata)}
          </span>
        )}
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

  useEffect(() => {
    endRef.current?.scrollIntoView({ behavior: 'smooth' })
  }, [logs])

  if (isCollapsed) {
    return (
      <aside className="w-10 sidebar-terminal">
        <button
          onClick={() => setIsCollapsed(false)}
          className="w-full h-full flex items-center justify-center text-terminal-dim hover:text-terminal-green transition-colors"
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
    <aside className="w-80 sidebar-terminal">
      <div className="sidebar-terminal-header">
        <div className="flex items-center gap-2">
          <span className="text-terminal-green">$</span>
          <h2 className="text-sm font-medium text-terminal">logs</h2>
          <span className="badge-terminal text-xs">{logs.length}</span>
        </div>
        <div className="flex items-center gap-2">
          <button
            onClick={handleClear}
            className="text-xs text-terminal-dim hover:text-terminal-yellow transition-colors"
            title="Clear logs"
          >
            clear
          </button>
          <button
            onClick={handleCopy}
            className="text-xs text-terminal-dim hover:text-terminal-blue transition-colors"
            title="Copy logs"
          >
            copy
          </button>
          <button
            onClick={() => setIsCollapsed(true)}
            className="text-terminal-dim hover:text-terminal transition-colors"
            title="Collapse"
          >
            <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 5l7 7-7 7" />
            </svg>
          </button>
        </div>
      </div>
      
      <div className="flex-1 overflow-y-auto p-3 space-y-1 text-xs">
        {logs.length === 0 ? (
          <div className="text-terminal-dim text-center py-8">
            <p>No logs yet...</p>
            <p className="mt-1 text-terminal-green">
              <span className="cursor"></span>
            </p>
          </div>
        ) : (
          logs.map((entry, idx) => (
            <div 
              key={entry.id || idx} 
              className="py-1 px-2 hover:bg-terminal-lighter rounded transition-colors leading-relaxed"
            >
              {formatLogEntry(entry)}
            </div>
          ))
        )}
        <div ref={endRef}></div>
      </div>
      
      {/* Status bar */}
      <div className="px-3 py-2 border-t border-terminal text-xs text-terminal-dim flex items-center justify-between">
        <div className="flex items-center gap-2">
          <span className="status-dot status-online"></span>
          <span>streaming</span>
        </div>
        <span>{logs.length} entries</span>
      </div>
    </aside>
  )
}

export default LogSidebar
