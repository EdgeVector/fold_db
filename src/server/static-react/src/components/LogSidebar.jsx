import { useEffect, useRef, useState } from 'react'
import { systemClient } from '../api/clients/systemClient'

function LogSidebar() {
  const [logs, setLogs] = useState([])
  const [isCollapsed, setIsCollapsed] = useState(true)
  const logContainerRef = useRef(null)

  const formatLog = (entry) => {
    if (typeof entry === 'string') return entry
    const meta = entry.metadata ? JSON.stringify(entry.metadata) : ''
    return `[${entry.level}] [${entry.event_type}] - ${entry.message} ${meta}`
  }

  const getLevelColor = (level) => {
    switch (level?.toUpperCase()) {
      case 'ERROR': return 'text-gruvbox-red'
      case 'WARN': case 'WARNING': return 'text-gruvbox-yellow'
      case 'INFO': return 'text-secondary'
      default: return 'text-tertiary'
    }
  }

  const formatLogEntry = (entry) => {
    if (typeof entry === 'string') return <span className="text-tertiary">{entry}</span>
    const time = entry.timestamp ? new Date(entry.timestamp).toLocaleTimeString('en-US', { hour12: false }) : ''
    return (
      <>
        <span className="text-tertiary">{time}</span>
        <span className={`${getLevelColor(entry.level)} ml-2`}>[{entry.level}]</span>
        <span className="text-secondary ml-1">{entry.message}</span>
      </>
    )
  }

  const handleCopy = () => navigator.clipboard.writeText(logs.map(formatLog).join('\n')).catch(() => { /* clipboard not available */ })
  const handleClear = () => setLogs([])

  useEffect(() => {
    let cancelled = false

    systemClient.getLogs().then(r => {
      if (!cancelled && r.success && r.data) setLogs(Array.isArray(r.data.logs) ? r.data.logs : [])
    }).catch(() => { if (!cancelled) setLogs([]) })

    const eventSource = systemClient.createLogStream(
      (message) => {
        if (cancelled) return
        setLogs(prev => {
          let entry
          try { entry = JSON.parse(message) }
          catch { entry = { id: `stream-${Date.now()}`, timestamp: Date.now(), level: 'INFO', message } }
          if (entry.id && prev.some(e => e.id === entry.id)) return prev
          return [...prev, entry]
        })
      },
      () => {}
    )

    const pollInterval = setInterval(() => {
      if (cancelled) return
      setLogs(cur => {
        const last = cur[cur.length - 1]
        systemClient.getLogs(last?.timestamp).then(r => {
          if (!cancelled && r.success && r.data?.logs?.length) {
            setLogs(c => [...c, ...r.data.logs.filter(l => !c.some(e => e.id === l.id))])
          }
        }).catch(() => { /* polling failure - next interval will retry */ })
        return cur
      })
    }, 2000)

    return () => { cancelled = true; eventSource.close(); clearInterval(pollInterval) }
  }, [])

  useEffect(() => {
    if (logContainerRef.current) logContainerRef.current.scrollTop = logContainerRef.current.scrollHeight
  }, [logs])

  if (isCollapsed) {
    return (
      <aside className="w-10 sidebar items-center py-4">
        <button onClick={() => setIsCollapsed(false)} className="btn-secondary btn-sm p-2" title="Expand logs">
          <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 19l-7-7 7-7" />
          </svg>
        </button>
      </aside>
    )
  }

  return (
    <aside className="w-80 sidebar">
      <div className="flex items-center justify-between px-4 py-3 border-b border-border">
        <div className="flex items-center gap-2">
          <span className="text-sm font-medium">Logs</span>
          <span className="badge badge-neutral">{logs.length}</span>
        </div>
        <div className="flex items-center gap-3">
          <button onClick={handleClear} className="btn-secondary btn-sm">clear</button>
          <button onClick={handleCopy} className="btn-secondary btn-sm">copy</button>
          <button onClick={() => setIsCollapsed(true)} className="btn-secondary btn-sm p-1" title="Collapse">
            <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 5l7 7-7 7" />
            </svg>
          </button>
        </div>
      </div>

      <div ref={logContainerRef} className="flex-1 overflow-y-auto p-3 font-mono text-xs">
        {logs.length === 0 ? (
          <div className="text-tertiary text-center py-8">No logs yet</div>
        ) : (
          logs.map((entry, idx) => (
            <div key={entry.id || idx} className="list-item">{formatLogEntry(entry)}</div>
          ))
        )}
      </div>

      <div className="flex items-center justify-between px-4 py-2 border-t border-border bg-surface-secondary text-xs text-tertiary">
        <div className="flex items-center gap-1.5">
          <span className="status-dot status-dot-success animate-pulse" />
          <span>streaming</span>
        </div>
        <span>{logs.length} entries</span>
      </div>
    </aside>
  )
}

export default LogSidebar
