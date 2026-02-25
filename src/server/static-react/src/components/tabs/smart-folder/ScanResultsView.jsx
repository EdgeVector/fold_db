import { useRef } from 'react'
import FolderTreeView from '../FolderTreeView'
import { fmtCost } from '../../../utils/formatCost'

/**
 * Displays scan results: file counts, cost estimate, folder tree, and action buttons.
 *
 * @param {Object} props
 * @param {Object} props.scanResult
 * @param {string} props.spendLimit
 * @param {Function} props.onSpendLimitChange
 * @param {boolean} props.includeAlreadyIngested
 * @param {Function} props.onIncludeChange
 * @param {Function} props.onIngest
 * @param {Function} props.onLoadMore
 * @param {Function} props.onBack
 * @param {boolean} props.isIngesting
 * @param {boolean} props.isLoadingMore
 */
export default function ScanResultsView({
  scanResult,
  spendLimit,
  onSpendLimitChange,
  includeAlreadyIngested,
  onIncludeChange,
  onIngest,
  onLoadMore,
  onBack,
  isIngesting,
  isLoadingMore,
}) {
  const treeRef = useRef(null)
  const estimatedCost = scanResult?.total_estimated_cost

  const totalFiles = scanResult.recommended_files.length +
    (includeAlreadyIngested ? scanResult.skipped_files.filter(f => f.already_ingested).length : 0)

  return (
    <>
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-6 text-sm">
          <span className="text-primary font-medium">{scanResult.recommended_files.length} files to ingest</span>
          {scanResult.skipped_files.filter(f => f.already_ingested).length > 0 && (
            <label className="flex items-center gap-1.5 cursor-pointer">
              <input
                type="checkbox"
                checked={includeAlreadyIngested}
                onChange={(e) => onIncludeChange(e.target.checked)}
                className="accent-gruvbox-blue"
              />
              <span className={includeAlreadyIngested ? 'text-primary font-medium' : 'text-gruvbox-blue'}>
                {scanResult.skipped_files.filter(f => f.already_ingested).length} already ingested
              </span>
            </label>
          )}
          <span className="text-secondary">{scanResult.skipped_files.filter(f => !f.already_ingested).length} skipped</span>
          <span className="text-secondary">{scanResult.total_files} total</span>
        </div>
        {Object.keys(scanResult.summary).length > 0 && (
          <div className="flex gap-2 flex-wrap">
            {Object.entries(scanResult.summary).filter(([, count]) => count > 0).map(([cat, count]) => (
              <span key={cat} className="badge badge-neutral">{cat.replace(/_/g, ' ')}: {count}</span>
            ))}
          </div>
        )}
      </div>

      {/* Cost estimate & spend limit */}
      <div className="flex items-center gap-4 text-sm">
        <span className="text-secondary">Estimated cost: <span className="text-primary font-medium">~{fmtCost(estimatedCost)}</span></span>
        {Number(estimatedCost) > 0 && (
          <label className="flex items-center gap-2 text-secondary">
            Spend limit:
            <input
              type="text"
              value={spendLimit}
              onChange={(e) => onSpendLimitChange(e.target.value)}
              className="input w-24 text-sm"
              placeholder="no limit"
            />
          </label>
        )}
      </div>

      {/* Truncation warning with Load more */}
      {scanResult.scan_truncated && (
        <div className="bg-gruvbox-yellow/10 border border-gruvbox-yellow/30 rounded-lg px-3 py-2 text-sm text-gruvbox-yellow flex items-center justify-between">
          <span>Showing {scanResult.max_files_used} of more files. Some files may not be shown.</span>
          <button onClick={onLoadMore} disabled={isLoadingMore} className="btn-secondary text-xs ml-3 flex items-center gap-1">
            {isLoadingMore ? <><span className="spinner" />Loading...</> : 'Load more'}
          </button>
        </div>
      )}

      {/* Folder tree controls */}
      <div className="flex items-center gap-2 text-xs text-secondary">
        <button onClick={() => treeRef.current?.expandAll()} className="hover:text-primary underline">Expand all</button>
        <span>·</span>
        <button onClick={() => treeRef.current?.collapseAll()} className="hover:text-primary underline">Collapse all</button>
      </div>

      {/* Folder tree */}
      <FolderTreeView
        ref={treeRef}
        recommendedFiles={scanResult.recommended_files}
        skippedFiles={scanResult.skipped_files}
      />

      <div className="flex items-center justify-between">
        <button onClick={onBack} className="btn-secondary" disabled={isIngesting}>Back</button>
        <button onClick={onIngest} disabled={isIngesting || totalFiles === 0} className="btn-primary btn-lg flex items-center gap-2">
          {isIngesting ? <><span className="spinner" />Starting...</> : <>Proceed ({totalFiles} files)</>}
        </button>
      </div>
    </>
  )
}
