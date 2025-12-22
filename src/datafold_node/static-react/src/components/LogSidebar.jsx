import { useEffect, useRef, useState } from 'react'
import { systemClient } from '../api/clients/systemClient'

function LogSidebar() {
  const [logs, setLogs] = useState([])
  const endRef = useRef(null)

  const handleCopy = () => {
    Promise.resolve(
      navigator.clipboard.writeText(logs.join('\n'))
    ).catch(() => {})
  }

  useEffect(() => {
    // Load initial logs using systemClient
    systemClient.getLogs()
      .then(response => {
        if (response.success && response.data) {
          const logs = response.data.logs || []
          setLogs(Array.isArray(logs) ? logs : [])
        } else {
          setLogs([])
        }
      })
      .catch(() => setLogs([]))

    // Set up log streaming using systemClient
    const eventSource = systemClient.createLogStream(
      (message) => {
        setLogs(prev => [...prev, message])
      },
      (error) => {
        console.warn('Log stream error:', error)
      }
    )

    // Fallback polling for serverless/stateless environments where SSE might miss cross-instance logs
    // or if the stream disconnects. Since we updated /api/logs to query DynamoDB, this ensures consistency.
    const pollInterval = setInterval(() => {
      systemClient.getLogs()
        .then(response => {
          if (response.success && response.data) {
            const fetchedLogs = response.data.logs || []
            setLogs(currentLogs => {
              // Simple optimization: only update if length changed or last log is different
              // to avoid unnecessary re-renders/scrolls
              if (fetchedLogs.length !== currentLogs.length || 
                  (fetchedLogs.length > 0 && fetchedLogs[fetchedLogs.length - 1] !== currentLogs[currentLogs.length - 1])) {
                return Array.isArray(fetchedLogs) ? fetchedLogs : []
              }
              return currentLogs
            })
          }
        })
        .catch(err => console.warn('Log polling error:', err))
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
        {logs.map((line, idx) => (
          <div key={idx}>{line}</div>
        ))}
        <div ref={endRef}></div>
      </div>
    </aside>
  )
}

export default LogSidebar
