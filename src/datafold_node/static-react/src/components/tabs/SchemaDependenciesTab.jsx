import { useMemo } from 'react'
import { getDependencyGraph } from '../../utils/dependencyUtils'
import { useAppSelector } from '../../store/hooks'
import { selectAllSchemas } from '../../store/schemaSlice'

function SchemaDependenciesTab() {
  // Redux state - TASK-003: Use Redux instead of props
  const schemas = useAppSelector(selectAllSchemas)
  const { nodes, edges } = useMemo(() => getDependencyGraph(schemas), [schemas])

  const nodeWidth = 120
  const nodeHeight = 40
  const vSpacing = 60
  const positions = {}
  nodes.forEach((name, idx) => {
    positions[name] = { x: 160, y: idx * (nodeHeight + vSpacing) + 20 }
  })
  const svgHeight = nodes.length * (nodeHeight + vSpacing) + 40

  return (
    <div className="p-6">
      <svg className="w-full" height={svgHeight}>
        <defs>
          <marker id="arrow" markerWidth="10" markerHeight="10" refX="10" refY="5" orient="auto" markerUnits="strokeWidth">
            <path d="M0 0 L10 5 L0 10 Z" fill="currentColor" />
          </marker>
        </defs>
        {edges.map((edge, idx) => {
          const src = positions[edge.source]
          const tgt = positions[edge.target]
          const x1 = src.x + nodeWidth / 2
          const y1 = src.y + nodeHeight / 2
          const x2 = tgt.x - nodeWidth / 2
          const y2 = tgt.y + nodeHeight / 2
          const midX = (x1 + x2) / 2
          const midY = (y1 + y2) / 2
          const color = edge.type === 'transform' ? '#2563eb' : '#16a34a'
          return (
            <g key={idx} className="text-xs">
              <line x1={x1} y1={y1} x2={x2} y2={y2} stroke={color} strokeWidth="2" markerEnd="url(#arrow)" />
              <text x={midX} y={midY - 4} textAnchor="middle" fill={color}>{edge.type}</text>
            </g>
          )
        })}

        {nodes.map(name => {
          const pos = positions[name]
          return (
            <g key={name} transform={`translate(${pos.x - nodeWidth / 2}, ${pos.y})`}>
              <rect width={nodeWidth} height={nodeHeight} rx="4" fill="#f9fafb" stroke="#4b5563" />
              <text x={nodeWidth / 2} y={nodeHeight / 2 + 4} textAnchor="middle" className="text-sm" fill="#111827">
                {name}
              </text>
            </g>
          )
        })}
      </svg>
    </div>
  )
}

export default SchemaDependenciesTab
