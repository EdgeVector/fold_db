/**
 * File Upload Mode panel for FileUploadTab
 * Displays drag-and-drop zone with file selection
 */
function FileUploadMode({
  isDragging,
  selectedFile,
  setSelectedFile,
  handleDragEnter,
  handleDragOver,
  handleDragLeave,
  handleDrop,
  handleFileSelect,
  formatFileSize
}) {
  return (
    <div className="minimal-card p-6">
      <h3 className="text-success font-medium mb-4">
        <span className="text-secondary">$</span> Upload File
      </h3>

      <div
        className={`border-2 border-dashed p-12 text-center transition-colors ${
          isDragging
            ? 'border-current bg-surface-secondary'
            : 'border-default bg-surface hover:border-current'
        }`}
        onDragEnter={handleDragEnter}
        onDragOver={handleDragOver}
        onDragLeave={handleDragLeave}
        onDrop={handleDrop}
      >
        <div className="space-y-4">
          {/* Upload Icon - ASCII style */}
          <div className="text-secondary font-mono text-3xl">
            ↑
          </div>

          {/* Text */}
          {selectedFile ? (
            <div className="space-y-2">
              <p className="text-success font-medium">{selectedFile.name}</p>
              <p className="text-sm text-secondary">{formatFileSize(selectedFile.size)}</p>
              <button
                onClick={() => setSelectedFile(null)}
                className="text-sm text-error hover:glow-red"
              >
                [x] remove
              </button>
            </div>
          ) : (
            <div>
              <p className="text-primary mb-2">
                Drag and drop a file here, or click to select
              </p>
              <p className="text-sm text-secondary">
                # Supported: PDF, DOCX, TXT, CSV, JSON, XML
              </p>
            </div>
          )}

          {/* Hidden File Input */}
          <input
            type="file"
            id="file-upload"
            className="hidden"
            onChange={handleFileSelect}
          />

          {/* Browse Button */}
          {!selectedFile && (
            <label
              htmlFor="file-upload"
              className="minimal-btn-secondary minimal-btn inline-block cursor-pointer"
            >
              → Browse Files
            </label>
          )}
        </div>
      </div>
    </div>
  )
}

export default FileUploadMode
