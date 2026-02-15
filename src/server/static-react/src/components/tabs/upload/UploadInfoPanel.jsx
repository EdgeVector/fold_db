/**
 * Info panel for FileUploadTab
 * Shows how-it-works instructions per upload mode
 */
function UploadInfoPanel({ uploadMode }) {
  return (
    <div className="card card-info p-4">
      <div className="flex items-start gap-3">
        <span className="text-gruvbox-blue">[i]</span>
        <div className="text-sm text-secondary">
          <p className="font-medium mb-1 text-gruvbox-blue">How it works</p>
          <ol className="list-decimal list-inside space-y-1">
            {uploadMode === 'batch-folder' ? (
              <>
                <li>Specify a folder path containing files to ingest</li>
                <li>All supported files (.json, .csv, .txt, .md) are processed in parallel</li>
                <li>Each file is converted to JSON and analyzed by AI</li>
                <li>Data is mapped to schemas and stored in the database</li>
              </>
            ) : uploadMode === 's3-path' ? (
              <>
                <li>Provide an S3 file path (files already in S3 are not re-uploaded)</li>
                <li>File is automatically converted to JSON using AI</li>
                <li>AI analyzes the JSON and maps it to appropriate schemas</li>
                <li>Data is stored in the database with the file location tracked</li>
              </>
            ) : (
              <>
                <li>Upload any file type (PDFs, documents, spreadsheets, etc.)</li>
                <li>File is automatically converted to JSON using AI</li>
                <li>AI analyzes the JSON and maps it to appropriate schemas</li>
                <li>Data is stored in the database with the file location tracked</li>
              </>
            )}
          </ol>
        </div>
      </div>
    </div>
  )
}

export default UploadInfoPanel
