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

    return () => eventSource.close()
  }, [])

  useEffect(() => {
    endRef.current?.scrollIntoView({ behavior: 'smooth' })
  }, [logs])

  return (
    <aside className="w-80 h-screen bg-gray-900 text-white p-4 overflow-y-auto">
      <div className="flex items-center justify-between mb-2">
        <h2 className="text-lg font-semibold">Logs</h2>
        <button
          onClick={handleCopy}
          className="text-xs text-blue-300 hover:underline"
        >
          Copy
        </button>
      </div>
      <div className="space-y-1 text-xs font-mono">
        {logs.map((line, idx) => (
          <div key={idx}>{line}</div>
        ))}
        <div ref={endRef}></div>
      </div>
    </aside>
  )
}

export default LogSidebar
