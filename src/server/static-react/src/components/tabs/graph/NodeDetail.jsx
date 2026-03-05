export default function NodeDetail({ node, links, nodes }) {
  if (!node) return null
  const connected = links.filter(l => {
    const src = typeof l.source === 'object' ? l.source?.id : l.source
    const tgt = typeof l.target === 'object' ? l.target?.id : l.target
    return src === node.id || tgt === node.id
  })
  return (
    <div className="space-y-3">
      <div>
        <span className={`text-xs uppercase font-bold tracking-widest ${node.type === 'schema' ? 'text-[#83a598]' : 'text-[#b8bb26]'}`}>
          {node.type}
        </span>
        <div className="text-primary font-mono mt-1 text-sm break-all">{node.label}</div>
      </div>
      {connected.length > 0 && (
        <div>
          <div className="text-xs uppercase tracking-widest text-tertiary mb-2">
            Connections ({connected.length})
          </div>
          <div className="space-y-1 max-h-56 overflow-y-auto pr-1">
            {connected.map((l, i) => {
              const src = typeof l.source === 'object' ? l.source?.id : l.source
              const tgt = typeof l.target === 'object' ? l.target?.id : l.target
              const other = src === node.id ? tgt : src
              const otherNode = nodes.find(n => n.id === other)
              const otherLabel = otherNode?.label ?? String(other ?? '').replace(/^(schema:|word:)/, '')
              return (
                <div key={i} className="text-xs bg-surface-secondary border border-border p-2 space-y-0.5">
                  <div className="text-primary font-mono truncate">{otherLabel}</div>
                  <div className="text-tertiary">field: <span className="text-secondary">{l.field}</span></div>
                  <div className="text-tertiary">key: <span className="text-secondary font-mono">{l.keyLabel}</span></div>
                </div>
              )
            })}
          </div>
        </div>
      )}
    </div>
  )
}
