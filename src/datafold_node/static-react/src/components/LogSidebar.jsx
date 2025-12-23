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
        setLogs(prev => {
          // Avoid duplicates if the message is identical to the last received log
          if (prev.length > 0 && prev[prev.length - 1] === message) {
            return prev
          }
          return [...prev, message]
        })
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
              if (!Array.isArray(fetchedLogs)) return currentLogs

              // If fetched logs are shorter than current logs, checking if it's just a lag
              if (currentLogs.length >= fetchedLogs.length) {
                // Check if fetchedLogs is a prefix of currentLogs
                const isPrefix = fetchedLogs.every((val, index) => val === currentLogs[index])
                if (isPrefix) {
                  // Current logs has more data (from stream), keep it
                  return currentLogs
                }
              }
              
              // If fetched logs has more data or is different (e.g. restart), update it
              // Optimization: check if identical
              if (fetchedLogs.length === currentLogs.length && 
                  fetchedLogs[fetchedLogs.length-1] === currentLogs[currentLogs.length-1]) {
                 return currentLogs
              }

              return fetchedLogs
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
