/**
 * File Upload Mode panel for FileUploadTab
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
    <div
      className={`border-2 border-dashed p-8 text-center transition-colors ${
        isDragging ? 'border-primary bg-surface-secondary' : 'border-border bg-surface hover:border-primary'
      }`}
      onDragEnter={handleDragEnter}
      onDragOver={handleDragOver}
      onDragLeave={handleDragLeave}
      onDrop={handleDrop}
    >
      {selectedFile ? (
        <div className="space-y-2">
          <p className="font-medium">{selectedFile.name}</p>
          <p className="text-sm text-secondary">{formatFileSize(selectedFile.size)}</p>
          <button onClick={() => setSelectedFile(null)} className="text-sm text-red-600">
            Remove
          </button>
        </div>
      ) : (
        <div className="space-y-3">
          <p className="text-secondary">Drop file here or click to browse</p>
          <p className="text-xs text-tertiary">PDF, DOCX, TXT, CSV, JSON, XML</p>
          <input type="file" id="file-upload" className="hidden" onChange={handleFileSelect} />
          <label htmlFor="file-upload" className="btn-secondary inline-block cursor-pointer">
            Browse
          </label>
        </div>
      )}
    </div>
  )
}

export default FileUploadMode
