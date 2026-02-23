import { useState, forwardRef, useImperativeHandle, useMemo } from 'react'

const fmtCostShort = (v) => `$${Number(v).toFixed(3)}`

/** Build a nested tree structure from flat file paths */
function buildTree(files) {
  const root = { name: '', children: {}, files: [] }
  for (const file of files) {
    const parts = file.path.split('/')
    let node = root
    for (let i = 0; i < parts.length - 1; i++) {
      const dir = parts[i]
      if (!node.children[dir]) {
        node.children[dir] = { name: dir, children: {}, files: [] }
      }
      node = node.children[dir]
    }
    node.files.push(file)
  }
  return root
}

/** Recursively compute recommended/skipped/alreadyIngested counts for a folder */
function computeStats(node) {
  let recommended = 0
  let skipped = 0
  let alreadyIngested = 0
  for (const f of node.files) {
    if (f.already_ingested) alreadyIngested++
    else if (f.should_ingest) recommended++
    else skipped++
  }
  for (const child of Object.values(node.children)) {
    const childStats = computeStats(child)
    recommended += childStats.recommended
    skipped += childStats.skipped
    alreadyIngested += childStats.alreadyIngested
  }
  return { recommended, skipped, alreadyIngested }
}

/** Collect all folder paths in the tree */
function collectPaths(node, prefix = '') {
  const paths = []
  for (const [name, child] of Object.entries(node.children)) {
    const path = prefix ? `${prefix}/${name}` : name
    paths.push(path)
    paths.push(...collectPaths(child, path))
  }
  return paths
}

function FileNode({ file }) {
  const isIngested = file.already_ingested
  const iconClass = isIngested
    ? 'text-gruvbox-blue text-xs'
    : file.should_ingest
      ? 'text-gruvbox-green text-xs'
      : 'text-secondary text-xs'
  const icon = isIngested ? '\u2713' : file.should_ingest ? '+' : '-'
  const nameClass = isIngested || !file.should_ingest ? 'text-secondary' : ''
  const badgeClass = isIngested
    ? 'bg-gruvbox-blue/20 text-gruvbox-blue border-gruvbox-blue/30'
    : file.should_ingest
      ? 'bg-gruvbox-green/20 text-gruvbox-green border-gruvbox-green/30'
      : 'badge-neutral'

  return (
    <div className={`flex items-center gap-2 py-0.5 px-2 hover:bg-surface-secondary rounded text-sm group ${isIngested ? 'opacity-60' : ''}`}>
      <span className={iconClass}>{icon}</span>
      <span className={`font-mono text-xs flex-1 truncate ${nameClass}`}>
        {file.path.split('/').pop()}
      </span>
      <span className={`badge text-xs ${badgeClass}`}>{file.category}</span>
      {file.should_ingest && !isIngested && (
        <span className="text-secondary text-xs">~{fmtCostShort(file.estimated_cost)}</span>
      )}
      <span className="text-secondary text-xs hidden group-hover:inline max-w-48 truncate" title={file.reason}>
        {file.reason}
      </span>
    </div>
  )
}

function TreeNode({ node, name, depth, expanded, onToggle, pathPrefix }) {
  const path = pathPrefix ? `${pathPrefix}/${name}` : name
  const stats = useMemo(() => computeStats(node), [node])
  const isExpanded = expanded.has(path)
  const childNames = Object.keys(node.children).sort()
  const hasContent = childNames.length > 0 || node.files.length > 0

  if (!hasContent) return null

  return (
    <div>
      <button
        type="button"
        className="w-full flex items-center gap-1 py-0.5 cursor-pointer hover:bg-surface-secondary rounded select-none text-left bg-transparent border-none"
        style={{ paddingLeft: depth * 16 + 8 }}
        onClick={() => onToggle(path)}
        aria-expanded={isExpanded}
        aria-label={`${isExpanded ? 'Collapse' : 'Expand'} folder ${name}`}
      >
        <span className="text-xs w-3 text-secondary">{isExpanded ? '\u25BE' : '\u25B8'}</span>
        <span className="text-sm">{name}/</span>
        <span className="text-xs text-secondary ml-auto mr-2">
          {stats.alreadyIngested > 0 && <span className="text-gruvbox-blue">{stats.alreadyIngested} done</span>}
          {stats.alreadyIngested > 0 && (stats.recommended > 0 || stats.skipped > 0) && <span> · </span>}
          {stats.recommended > 0 && <span className="text-gruvbox-green">{stats.recommended} ingest</span>}
          {stats.recommended > 0 && stats.skipped > 0 && <span> · </span>}
          {stats.skipped > 0 && <span>{stats.skipped} skip</span>}
        </span>
      </button>
      {isExpanded && (
        <div>
          {childNames.map((childName) => (
            <TreeNode
              key={childName}
              node={node.children[childName]}
              name={childName}
              depth={depth + 1}
              expanded={expanded}
              onToggle={onToggle}
              pathPrefix={path}
            />
          ))}
          <div style={{ paddingLeft: (depth + 1) * 16 }}>
            {node.files.map((file) => (
              <FileNode key={file.path} file={file} />
            ))}
          </div>
        </div>
      )}
    </div>
  )
}

const FolderTreeView = forwardRef(function FolderTreeView({ recommendedFiles, skippedFiles }, ref) {
  const allFiles = useMemo(() => [...recommendedFiles, ...skippedFiles], [recommendedFiles, skippedFiles])
  const tree = useMemo(() => buildTree(allFiles), [allFiles])
  const allPaths = useMemo(() => collectPaths(tree), [tree])

  // Auto-expand root-level directories
  const [expanded, setExpanded] = useState(() => {
    const initial = new Set()
    for (const name of Object.keys(tree.children)) {
      initial.add(name)
    }
    return initial
  })

  const toggle = (path) => {
    setExpanded((prev) => {
      const next = new Set(prev)
      if (next.has(path)) next.delete(path)
      else next.add(path)
      return next
    })
  }

  useImperativeHandle(ref, () => ({
    expandAll() {
      setExpanded(new Set(allPaths))
    },
    collapseAll() {
      setExpanded(new Set())
    },
  }))

  const childNames = Object.keys(tree.children).sort()

  return (
    <div className="border border-border rounded-lg overflow-hidden">
      <div className="max-h-96 overflow-y-auto p-2">
        {/* Root-level files (if any) */}
        {tree.files.map((file) => (
          <FileNode key={file.path} file={file} />
        ))}
        {/* Child directories */}
        {childNames.map((name) => (
          <TreeNode
            key={name}
            node={tree.children[name]}
            name={name}
            depth={0}
            expanded={expanded}
            onToggle={toggle}
            pathPrefix=""
          />
        ))}
      </div>
    </div>
  )
})

export default FolderTreeView
