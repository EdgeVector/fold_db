import { useState, useEffect, useRef, useCallback } from 'react'
import { ingestionClient } from '../../../api/clients'

/**
 * Modal for browsing and selecting a directory via the web UI.
 *
 * @param {Object} props
 * @param {string} props.initialPath - Starting directory path
 * @param {Function} props.onSelect - Called with chosen directory path
 * @param {Function} props.onClose - Called to dismiss the modal
 */
export default function DirectoryBrowserModal({ initialPath, onSelect, onClose }) {
  const [currentPath, setCurrentPath] = useState(initialPath || '/')
  const [directories, setDirectories] = useState([])
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState(null)
  const modalRef = useRef(null)

  const fetchDirectories = useCallback(async (path) => {
    setLoading(true)
    setError(null)
    try {
      const response = await ingestionClient.listDirectory(path)
      if (response.data?.directories) {
        setDirectories(response.data.directories)
        setCurrentPath(path)
        if (response.data.error) {
          setError(response.data.error)
        }
      } else {
        setDirectories([])
        setError(response.data?.error || 'Failed to list directory')
      }
    } catch (e) {
      setDirectories([])
      setError('Failed to list directory')
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    fetchDirectories(currentPath)
  }, []) // eslint-disable-line react-hooks/exhaustive-deps

  const navigateUp = () => {
    const parent = currentPath.replace(/\/[^/]+\/?$/, '') || '/'
    fetchDirectories(parent)
  }

  const navigateInto = (dirName) => {
    const next = currentPath.endsWith('/')
      ? currentPath + dirName
      : currentPath + '/' + dirName
    fetchDirectories(next)
  }

  const handleKeyDown = (e) => {
    if (e.key === 'Escape') onClose()
  }

  return (
    <div className="modal-overlay" onClick={onClose} onKeyDown={handleKeyDown}>
      <div
        className="modal"
        ref={modalRef}
        role="dialog"
        aria-modal="true"
        aria-label="Browse Directories"
        onClick={(e) => e.stopPropagation()}
        style={{ maxWidth: '36rem' }}
      >
        <div className="modal-header">
          <h3 className="text-lg font-medium">Browse Directories</h3>
          <button onClick={onClose} className="btn-secondary btn-sm p-1" aria-label="Close">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <line x1="18" y1="6" x2="6" y2="18" />
              <line x1="6" y1="6" x2="18" y2="18" />
            </svg>
          </button>
        </div>

        <div className="px-6 py-3 border-b border-border">
          <div className="flex items-center gap-2">
            <button
              onClick={navigateUp}
              disabled={currentPath === '/'}
              className="btn-secondary btn-sm px-2"
              title="Up one level"
            >
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                <polyline points="15 18 9 12 15 6" />
              </svg>
            </button>
            <code className="text-sm text-secondary truncate flex-1">{currentPath}</code>
          </div>
        </div>

        <div className="modal-body" style={{ minHeight: '16rem', maxHeight: '24rem' }}>
          {loading && <p className="text-sm text-secondary">Loading...</p>}
          {error && <p className="text-sm text-red-400">{error}</p>}
          {!loading && !error && directories.length === 0 && (
            <p className="text-sm text-secondary">No subdirectories found.</p>
          )}
          {!loading && directories.length > 0 && (
            <ul className="space-y-0.5">
              {directories.map((dir) => (
                <li key={dir}>
                  <button
                    className="w-full text-left px-3 py-1.5 text-sm font-mono rounded hover:bg-surface-hover truncate"
                    onClick={() => navigateInto(dir)}
                  >
                    {dir}/
                  </button>
                </li>
              ))}
            </ul>
          )}
        </div>

        <div className="modal-footer">
          <button onClick={onClose} className="btn-secondary">Cancel</button>
          <button onClick={() => onSelect(currentPath)} className="btn-primary">Select This Folder</button>
        </div>
      </div>
    </div>
  )
}
