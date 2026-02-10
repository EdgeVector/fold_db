/**
 * Batch Folder Mode panel for FileUploadTab
 */
function BatchFolderMode({ folderPath, setFolderPath, batchProgress, fileProgresses }) {
  return (
    <div className="space-y-3">
      <input
        type="text"
        value={folderPath}
        onChange={(e) => setFolderPath(e.target.value)}
        placeholder="Folder path (e.g. sample_data)"
        className="input w-full"
      />

      {batchProgress && (
        <div className="space-y-2 max-h-48 overflow-y-auto">
          {Object.entries(fileProgresses).map(([id, progress]) => (
            <div key={id} className="flex items-center gap-3 text-sm">
              <span className={`w-2 h-2 rounded-full ${
                progress.is_failed ? 'bg-error' : progress.is_complete ? 'bg-success' : 'bg-warning animate-pulse'
              }`} />
              <span className="font-mono text-xs truncate flex-1">{progress.file_name}</span>
              <span className="text-secondary text-xs">{progress.progress_percentage}%</span>
            </div>
          ))}
        </div>
      )}
    </div>
  )
}

export default BatchFolderMode
