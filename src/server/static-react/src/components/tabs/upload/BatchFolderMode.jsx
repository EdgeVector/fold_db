/**
 * Batch Folder Mode panel for FileUploadTab
 * Displays folder path input and per-file batch progress
 */
function BatchFolderMode({ folderPath, setFolderPath, batchProgress, fileProgresses }) {
  return (
    <div className="minimal-card p-6">
      <h3 className="text-success font-medium mb-4">
        <span className="text-secondary">$</span> Batch Folder Ingestion
      </h3>
      <div className="space-y-3">
        <label className="block text-sm font-medium text-secondary">
          --folder-path
        </label>
        <input
          type="text"
          value={folderPath}
          onChange={(e) => setFolderPath(e.target.value)}
          placeholder="sample_data"
          className="minimal-input w-full"
        />
        <p className="text-xs text-secondary">
          # Path relative to project root or absolute path. Supported files: .json, .csv, .txt, .md
        </p>
      </div>

      {/* Batch Progress Display */}
      {batchProgress && (
        <div className="mt-4 space-y-3">
          <div className="flex items-center gap-2 text-sm">
            <span className="text-secondary">batch_id:</span>
            <span className="text-info font-mono text-xs">{batchProgress.batch_id.slice(0, 8)}...</span>
            <span className="text-secondary">|</span>
            <span className="text-secondary">files:</span>
            <span className="text-success">{batchProgress.files_found}</span>
          </div>

          {/* Individual File Progress */}
          <div className="space-y-2 max-h-48 overflow-y-auto">
            {Object.entries(fileProgresses).map(([id, progress]) => (
              <div key={id} className="flex items-center gap-3 text-sm bg-surface-secondary p-2 rounded">
                <span className={`w-2 h-2 rounded-full ${
                  progress.is_failed ? 'bg-error' :
                  progress.is_complete ? 'bg-success' :
                  'bg-warning animate-pulse'
                }`}></span>
                <span className="text-primary font-mono text-xs truncate flex-1">
                  {progress.file_name}
                </span>
                <span className="text-secondary text-xs">
                  {progress.progress_percentage}%
                </span>
                <span className={`text-xs ${
                  progress.is_failed ? 'text-error' :
                  progress.is_complete ? 'text-success' :
                  'text-warning'
                }`}>
                  {progress.is_failed ? 'failed' :
                   progress.is_complete ? 'done' :
                   progress.current_step || 'processing'}
                </span>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  )
}

export default BatchFolderMode
