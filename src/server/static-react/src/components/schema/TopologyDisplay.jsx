import { useState } from 'react'
import { ChevronDownIcon, ChevronRightIcon } from '@heroicons/react/24/solid'

/**
 * Recursively renders a topology node with proper styling and indentation
 */
function TopologyNode({ node, depth = 0, name = null }) {
  const [isExpanded, setIsExpanded] = useState(depth === 0)

  if (!node) {
    return <span className="text-tertiary italic">undefined</span>
  }

  // Handle Primitive types
  if (node.type === 'Primitive') {
    const primitiveType = node.value
    const typeColor = {
      String: 'text-success',
      Number: 'text-info',
      Boolean: 'text-purple',
      Null: 'text-secondary'
    }[primitiveType] || 'text-secondary'

    return (
      <span className="inline-flex items-center space-x-2">
        <span className={`font-mono text-sm ${typeColor}`}>
          {primitiveType.toLowerCase()}
        </span>
        {node.classifications && node.classifications.length > 0 && (
          <span className="flex space-x-1">
            {node.classifications.map(cls => (
              <span key={cls} className="px-1.5 py-0.5 text-xs bg-surface-secondary text-primary rounded-full font-sans">
                {cls}
              </span>
            ))}
          </span>
        )}
      </span>
    )
  }

  // Handle Any type
  if (node.type === 'Any') {
    return (
      <span className="font-mono text-sm text-orange-600">
        any
      </span>
    )
  }

  // Handle Array type
  if (node.type === 'Array') {
    return (
      <div className="inline-flex items-start">
        <span className="font-mono text-sm text-primary">Array&lt;</span>
        <TopologyNode node={node.value} depth={depth + 1} />
        <span className="font-mono text-sm text-primary">&gt;</span>
      </div>
    )
  }

  // Handle Object type
  if (node.type === 'Object' && node.value) {
    const fields = Object.entries(node.value)

    if (fields.length === 0) {
      return <span className="font-mono text-sm text-secondary">{'{}'}</span>
    }

    return (
      <div className="inline-block">
        <div className="flex items-center">
          <button
            onClick={() => setIsExpanded(!isExpanded)}
            className="flex items-center hover:bg-surface-secondary rounded px-1 -ml-1"
          >
            {isExpanded ? (
              <ChevronDownIcon className="h-3 w-3 text-secondary" />
            ) : (
              <ChevronRightIcon className="h-3 w-3 text-secondary" />
            )}
            <span className="font-mono text-sm text-primary ml-1">
              {'{'}
              {!isExpanded && `... ${fields.length} fields`}
              {!isExpanded && '}'}
            </span>
          </button>
        </div>

        {isExpanded && (
          <div className="ml-4 border-l-2 border-default pl-3 mt-1">
            {fields.map(([fieldName, fieldNode], index) => (
              <div key={fieldName} className="py-1">
                <span className="font-mono text-sm text-indigo-600">{fieldName}</span>
                <span className="font-mono text-sm text-secondary">: </span>
                <TopologyNode node={fieldNode} depth={depth + 1} name={fieldName} />
                {index < fields.length - 1 && <span className="text-tertiary">,</span>}
              </div>
            ))}
            <div className="font-mono text-sm text-primary">{'}'}</div>
          </div>
        )}
      </div>
    )
  }

  // Fallback for unknown node types
  return (
    <span className="font-mono text-sm text-error">
      unknown ({JSON.stringify(node)})
    </span>
  )
}

/**
 * Main component to display field topology
 */
export default function TopologyDisplay({ topology, compact = false }) {
  if (!topology) {
    return (
      <div className="text-xs text-tertiary italic">
        No topology defined
      </div>
    )
  }

  if (compact) {
    return (
      <div className="inline-flex items-center">
        <TopologyNode node={topology.root} />
      </div>
    )
  }

  return (
    <div className="mt-2 p-2 card">
      <div className="text-xs font-medium text-secondary mb-1">Type Structure:</div>
      <div className="pl-2">
        <TopologyNode node={topology.root} />
      </div>
    </div>
  )
}

