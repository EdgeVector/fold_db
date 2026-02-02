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
        return '#ef4444'
      case 'WARN':
      case 'WARNING':
        return '#f59e0b'
      case 'INFO':
        return '#666'
      case 'DEBUG':
        return '#999'
      default:
        return '#999'
    }
  }

  const formatLogEntry = (entry) => {
    if (typeof entry === 'string') {
      return <span style={{ color: '#999' }}>{entry}</span>
    }

    const levelColor = getLevelColor(entry.level)
    const time = entry.timestamp
      ? new Date(entry.timestamp).toLocaleTimeString('en-US', { hour12: false })
      : ''

    return (
      <>
        <span style={{ color: '#999' }}>{time}</span>
        <span style={{ color: levelColor, marginLeft: '8px' }}>[{entry.level}]</span>
        <span style={{ color: '#666', marginLeft: '4px' }}>{entry.message}</span>
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
      <aside style={{
        width: '40px',
        background: '#fff',
        borderLeft: '1px solid #e5e5e5',
        display: 'flex',
        flexDirection: 'column',
        flexShrink: 0
      }}>
        <button
          onClick={() => setIsCollapsed(false)}
          style={{
            width: '100%',
            height: '100%',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            color: '#999',
            background: 'transparent',
            border: 'none',
            cursor: 'pointer'
          }}
          title="Expand logs"
        >
          <svg style={{ width: '16px', height: '16px' }} fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 19l-7-7 7-7" />
          </svg>
        </button>
      </aside>
    )
  }

  return (
    <aside style={{
      width: '320px',
      background: '#fff',
      borderLeft: '1px solid #e5e5e5',
      display: 'flex',
      flexDirection: 'column',
      flexShrink: 0,
      height: '100%',
      overflow: 'hidden'
    }}>
      {/* Header */}
      <div style={{
        padding: '12px 16px',
        borderBottom: '1px solid #e5e5e5',
        display: 'flex',
        justifyContent: 'space-between',
        alignItems: 'center',
        flexShrink: 0
      }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
          <span style={{ fontSize: '14px', fontWeight: 500, color: '#111' }}>Logs</span>
          <span style={{
            fontSize: '11px',
            color: '#999',
            padding: '2px 8px',
            background: '#f5f5f5',
            border: '1px solid #e5e5e5'
          }}>{logs.length}</span>
        </div>
        <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
          <button
            onClick={handleClear}
            style={{
              fontSize: '12px',
              color: '#999',
              background: 'transparent',
              border: 'none',
              cursor: 'pointer'
            }}
          >
            clear
          </button>
          <button
            onClick={handleCopy}
            style={{
              fontSize: '12px',
              color: '#999',
              background: 'transparent',
              border: 'none',
              cursor: 'pointer'
            }}
          >
            copy
          </button>
          <button
            onClick={() => setIsCollapsed(true)}
            style={{
              color: '#999',
              background: 'transparent',
              border: 'none',
              cursor: 'pointer',
              padding: '4px'
            }}
            title="Collapse"
          >
            <svg style={{ width: '16px', height: '16px' }} fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 5l7 7-7 7" />
            </svg>
          </button>
        </div>
      </div>

      {/* Log content - fixed height with internal scroll */}
      <div
        ref={logContainerRef}
        style={{
          flex: 1,
          overflowY: 'auto',
          padding: '12px',
          fontSize: '12px',
          fontFamily: "'SF Mono', Monaco, monospace",
          lineHeight: 1.6
        }}
      >
        {logs.length === 0 ? (
          <div style={{ color: '#999', textAlign: 'center', padding: '32px 0' }}>
            No logs yet
          </div>
        ) : (
          logs.map((entry, idx) => (
            <div
              key={entry.id || idx}
              style={{
                padding: '4px 0',
                borderBottom: '1px solid #f5f5f5'
              }}
            >
              {formatLogEntry(entry)}
            </div>
          ))
        )}
      </div>

      {/* Status bar */}
      <div style={{
        padding: '8px 16px',
        borderTop: '1px solid #e5e5e5',
        fontSize: '11px',
        color: '#999',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        flexShrink: 0
      }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
          <span style={{
            width: '6px',
            height: '6px',
            background: '#22c55e',
            borderRadius: '50%'
          }}></span>
          <span>streaming</span>
        </div>
        <span>{logs.length} entries</span>
      </div>
    </aside>
  )
}

export default LogSidebar
