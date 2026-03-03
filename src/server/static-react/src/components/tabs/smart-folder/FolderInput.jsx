import { useState } from 'react'
import DirectoryBrowserModal from './DirectoryBrowserModal'

/**
 * Tauri capabilities required in src-tauri/capabilities/default.json:
 *   - dialog:allow-open   (Browse button: open({ directory: true }))
 */

const isTauri = typeof window !== 'undefined' && window.__TAURI_INTERNALS__

/**
 * Folder path input with autocomplete, scan/cancel button, and progress display.
 *
 * @param {Object} props
 * @param {string} props.folderPath
 * @param {Function} props.onFolderPathChange
 * @param {Function} props.onScan - Called with no args to start a scan
 * @param {Function} props.onCancelScan
 * @param {boolean} props.isScanning
 * @param {Object|null} props.scanProgress
 * @param {Object} props.autocomplete - Return value from useFolderAutocomplete
 */
export default function FolderInput({
  folderPath,
  onFolderPathChange,
  onScan,
  onCancelScan,
  isScanning,
  scanProgress,
  autocomplete,
}) {
  const {
    suggestions,
    selectedIndex,
    showSuggestions,
    setShowSuggestions,
    setSelectedIndex,
    acceptSuggestion,
    handleInputKeyDown,
    inputRef,
    suggestionsRef,
  } = autocomplete

  const [, setPickerError] = useState(null)
  const [showBrowser, setShowBrowser] = useState(false)

  const openFolderPicker = async () => {
    if (isTauri) {
      try {
        const { open } = await import('@tauri-apps/plugin-dialog')
        const selected = await open({ directory: true, multiple: false, title: 'Select folder to scan' })
        if (selected) onFolderPathChange(selected)
      } catch (error) {
        setPickerError(error)
        console.error('Failed to open folder picker:', error)
      }
    } else {
      setShowBrowser(true)
    }
  }

  const handleBrowserSelect = (path) => {
    onFolderPathChange(path)
    setShowBrowser(false)
  }

  return (
    <>
      <div className="flex gap-3">
        <div className="relative flex-1">
          <input
            ref={inputRef}
            type="text"
            value={folderPath}
            onChange={(e) => onFolderPathChange(e.target.value)}
            onKeyDown={handleInputKeyDown}
            onFocus={() => { if (suggestions.length > 0) setShowSuggestions(true) }}
            placeholder="/path/to/your/folder"
            className="input w-full"
            disabled={isScanning}
            autoComplete="off"
          />
          {showSuggestions && suggestions.length > 0 && (
            <ul
              ref={suggestionsRef}
              className="absolute z-50 left-0 right-0 top-full mt-1 border border-border rounded-lg bg-surface shadow-lg max-h-48 overflow-y-auto"
            >
              {suggestions.map((path, i) => (
                <li
                  key={path}
                  className={`px-3 py-1.5 cursor-pointer text-sm font-mono truncate ${
                    i === selectedIndex ? 'bg-accent text-on-accent' : 'hover:bg-surface-hover'
                  }`}
                  onMouseDown={(e) => { e.preventDefault(); acceptSuggestion(path) }}
                  onMouseEnter={() => setSelectedIndex(i)}
                >
                  {path}
                </li>
              ))}
            </ul>
          )}
        </div>
        <button onClick={openFolderPicker} disabled={isScanning} className="btn-secondary" title="Browse">Browse</button>
        {isScanning
          ? <button onClick={onCancelScan} className="btn-secondary">Cancel</button>
          : <button onClick={() => onScan()} disabled={!folderPath.trim()} className="btn-primary">Scan</button>
        }
      </div>
      {isScanning && scanProgress && (
        <div className="space-y-1.5">
          <div className="w-full bg-border rounded-full h-1.5 overflow-hidden">
            <div
              className="h-full bg-primary rounded-full transition-all duration-300"
              style={{ width: `${scanProgress.progress_percentage || 0}%` }}
            />
          </div>
          <p className="text-xs text-secondary">{scanProgress.status_message || 'Starting scan...'}</p>
        </div>
      )}
      {isScanning && !scanProgress && (
        <p className="text-xs text-secondary">Starting scan...</p>
      )}
      {import.meta.env.DEV && (
        <button
          onClick={() => onFolderPathChange('sample_data')}
          className="text-xs text-secondary hover:text-primary underline"
          disabled={isScanning}
        >
          Try sample data
        </button>
      )}
      {showBrowser && (
        <DirectoryBrowserModal
          initialPath={folderPath.includes('/') ? folderPath : '/'}
          onSelect={handleBrowserSelect}
          onClose={() => setShowBrowser(false)}
        />
      )}
    </>
  )
}
