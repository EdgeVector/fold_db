import { useCallback, useEffect, useRef, useState } from 'react'
import ForceGraph2D from 'react-force-graph-2d'
import { useApprovedSchemas } from '../../hooks/useApprovedSchemas.js'
import { nativeIndexClient, mutationClient, schemaClient } from '../../api/clients'
import { getFieldNames, getSchemaDisplayName, toErrorMessage } from '../../utils/schemaUtils'

// Gruvbox-inspired palette
const COLORS = {
  schema:    '#83a598',
  word:      '#b8bb26',
  key:       '#fe8019',
  link:      '#504945',
  linkHover: '#928374',
  bg:        '#282828',
  text:      '#ebdbb2',
}

const STOPWORDS = new Set([
  'the','and','for','are','but','not','you','all','can','her','was','one',
  'our','out','get','has','him','his','how','new','now','old','see','two',
  'way','who','did','its','let','put','say','she','too','use','that','this',
  'with','from','they','been','have','will','more','also','than','then',
  'when','just','over','into','some','what','your','would','could','which',
])

const MAX_WORDS     = 300  // cap on unique words to search
const MAX_RECORDS   = 20   // records to query per schema
const SEARCH_BATCH  = 8    // concurrent searches at a time

function makeSchemaId(name) { return `schema:${name}` }
function makeWordId(term)   { return `word:${term}` }

function formatKey(kv) {
  const parts = []
  if (kv?.hash)  parts.push(kv.hash.slice(0, 12))
  if (kv?.range) parts.push(kv.range.slice(0, 12))
  return parts.join(' / ') || '—'
}

function mergeGraphData(prev, newNodes, newLinks) {
  const nodeMap = new Map(prev.nodes.map(n => [n.id, n]))
  const linkMap = new Map(prev.links.map(l => [l.id, l]))
  for (const n of newNodes) if (!nodeMap.has(n.id)) nodeMap.set(n.id, n)
  for (const l of newLinks) if (!linkMap.has(l.id)) linkMap.set(l.id, l)
  return { nodes: Array.from(nodeMap.values()), links: Array.from(linkMap.values()) }
}

function extractWordsFromRecord(record) {
  const words = new Set()
  const fields = record?.fields ?? (typeof record === 'object' ? record : {})
  for (const value of Object.values(fields ?? {})) {
    if (typeof value !== 'string') continue
    for (const w of value.toLowerCase().split(/[^a-z0-9]+/)) {
      if (w.length >= 3 && !STOPWORDS.has(w)) words.add(w)
    }
  }
  return words
}

function buildFromResults(results) {
  const nodes = []
  const links = []
  const seenWords = new Set()
  for (const r of results) {
    const schemaId = makeSchemaId(r.schema_name)
    const wordLabel = String(r.value ?? r.field ?? '')
    if (!wordLabel) continue
    const wordId = makeWordId(wordLabel)
    const linkId = `${wordId}-->${schemaId}:${r.key_value?.hash}:${r.key_value?.range}:${r.field}`
    if (!seenWords.has(wordId)) {
      seenWords.add(wordId)
      nodes.push({ id: wordId, label: wordLabel, type: 'word', field: r.field })
    }
    links.push({
      id: linkId,
      source: wordId,
      target: schemaId,
      keyLabel: formatKey(r.key_value),
      field: r.field,
      hash: r.key_value?.hash ?? '',
      range: r.key_value?.range ?? '',
    })
  }
  return { nodes, links }
}

async function searchBatch(words, onBatchResult, onWordComplete) {
  const pending = [...words]
  while (pending.length > 0) {
    const batch = pending.splice(0, SEARCH_BATCH)
    await Promise.all(batch.map(async (word) => {
      const res = await nativeIndexClient.search(word)
      if (res.success) {
        const results = res.data?.results ?? []
        if (results.length) onBatchResult(results)
      }
      onWordComplete()
    }))
  }
}

function NodeDetail({ node, links, nodes }) {
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

export default function WordGraphTab() {
  const { approvedSchemas } = useApprovedSchemas()
  const [graphData, setGraphData] = useState({ nodes: [], links: [] })
  const [searchTerm, setSearchTerm] = useState('')
  const [isSearching, setIsSearching] = useState(false)
  const [loadStatus, setLoadStatus] = useState(null) // { phase, progress, total } | null
  const [error, setError] = useState(null)
  const [selectedNode, setSelectedNode] = useState(null)
  const [highlightNodes, setHighlightNodes] = useState(new Set())
  const [highlightLinks, setHighlightLinks] = useState(new Set())
  const graphRef = useRef(null)
  const containerRef = useRef(null)
  const [dimensions, setDimensions] = useState({ width: 800, height: 550 })
  const prepopulatedRef = useRef(false)

  useEffect(() => {
    if (!containerRef.current) return
    const ro = new ResizeObserver(entries => {
      for (const entry of entries) {
        setDimensions({ width: entry.contentRect.width, height: entry.contentRect.height })
      }
    })
    ro.observe(containerRef.current)
    return () => ro.disconnect()
  }, [])

  // Seed schema nodes whenever approved schemas change
  useEffect(() => {
    if (!approvedSchemas?.length) return
    const schemaNodes = approvedSchemas.map(s => ({ id: makeSchemaId(s.name), label: getSchemaDisplayName(s), type: 'schema' }))
    setGraphData(prev => mergeGraphData(prev, schemaNodes, []))
  }, [approvedSchemas])

  const addResults = useCallback((results) => {
    const { nodes, links } = buildFromResults(results)
    setGraphData(prev => mergeGraphData(prev, nodes, links))
  }, [])

  // Auto-populate on first schema load
  const prepopulate = useCallback(async (schemas) => {
    if (prepopulatedRef.current || !schemas?.length) return
    prepopulatedRef.current = true

    setError(null)
    const allWords = new Set()

    try {
      // Phase 1: query records from each schema to extract real words
      setLoadStatus({ phase: 'Reading records…', progress: 0, total: schemas.length })
      for (let i = 0; i < schemas.length; i++) {
        const schema = schemas[i]
        setLoadStatus({ phase: `Reading ${getSchemaDisplayName(schema)}…`, progress: i, total: schemas.length })
        try {
          const fields = getFieldNames(schema)
          const res = await mutationClient.executeQuery({ schema_name: schema.name, fields })
          const records = Array.isArray(res.data?.results) ? res.data.results : []
          for (const record of records.slice(0, MAX_RECORDS)) {
            for (const w of extractWordsFromRecord(record)) {
              if (allWords.size < MAX_WORDS) allWords.add(w)
            }
          }
        } catch {
          // schema query failure is non-fatal
        }
      }

      if (allWords.size === 0) {
        // Fallback: list keys and use their hashes as seed terms
        for (const schema of schemas) {
          if (allWords.size >= MAX_WORDS) break
          try {
            const res = await schemaClient.listSchemaKeys(schema.name, 0, 50)
            for (const kv of (res.data?.keys ?? [])) {
              if (kv.hash && allWords.size < MAX_WORDS) allWords.add(kv.hash)
              if (kv.range && allWords.size < MAX_WORDS) allWords.add(kv.range)
            }
          } catch { /* non-fatal */ }
        }
      }

      if (allWords.size === 0) return

      // Phase 2: search each word in the native index
      const wordList = Array.from(allWords)
      let done = 0
      setLoadStatus({ phase: 'Indexing words…', progress: 0, total: wordList.length })
      await searchBatch(
        wordList,
        (results) => { addResults(results) },
        () => {
          done += 1
          setLoadStatus({ phase: 'Indexing words…', progress: done, total: wordList.length })
        }
      )
    } finally {
      setLoadStatus(null)
    }
  }, [addResults])

  useEffect(() => {
    if (approvedSchemas?.length) {
      prepopulate(approvedSchemas)
    }
  }, [approvedSchemas, prepopulate])

  const handleSearch = useCallback(async () => {
    const q = searchTerm.trim()
    if (!q) return
    setIsSearching(true)
    setError(null)
    try {
      const res = await nativeIndexClient.search(q)
      if (res.success) {
        const results = res.data?.results ?? []
        addResults(results)
        if (results.length === 0) setError(`No index entries for "${q}"`)
      } else {
        setError(res.error || 'Search failed')
      }
    } catch (e) {
      setError(toErrorMessage(e) || 'Network error')
    } finally {
      setIsSearching(false)
    }
  }, [searchTerm, addResults])

  const handleNodeHover = useCallback((node) => {
    if (!node) { setHighlightNodes(new Set()); setHighlightLinks(new Set()); return }
    const hl = new Set([node.id])
    const hlLinks = new Set()
    for (const l of graphData.links) {
      const src = typeof l.source === 'object' ? l.source?.id : l.source
      const tgt = typeof l.target === 'object' ? l.target?.id : l.target
      if (src === node.id || tgt === node.id) {
        hlLinks.add(l.id); hl.add(src); hl.add(tgt)
      }
    }
    setHighlightNodes(hl)
    setHighlightLinks(hlLinks)
  }, [graphData.links])

  const handleNodeClick = useCallback((node) => {
    setSelectedNode(prev => prev?.id === node.id ? null : node)
  }, [])

  const nodeCanvasObject = useCallback((node, ctx, globalScale) => {
    const isHighlighted = highlightNodes.has(node.id)
    const isSelected = selectedNode?.id === node.id
    const isSchema = node.type === 'schema'
    const baseColor = isSchema ? COLORS.schema : COLORS.word
    const r = isSchema ? 8 : 5

    ctx.beginPath()
    if (isSchema) {
      ctx.rect(node.x - r, node.y - r, r * 2, r * 2)
    } else {
      ctx.arc(node.x, node.y, r, 0, 2 * Math.PI)
    }
    ctx.fillStyle = isHighlighted || isSelected ? baseColor : `${baseColor}99`
    ctx.fill()
    if (isSelected) { ctx.strokeStyle = COLORS.key; ctx.lineWidth = 2; ctx.stroke() }

    const fontSize = Math.max(10 / globalScale, isSchema ? 11 : 9)
    ctx.font = `${isSchema ? 'bold ' : ''}${fontSize}px monospace`
    ctx.textAlign = 'center'
    ctx.textBaseline = 'middle'
    ctx.fillStyle = isHighlighted || isSelected ? COLORS.text : `${COLORS.text}99`
    const lbl = node.label.length > 20 ? node.label.slice(0, 18) + '…' : node.label
    ctx.fillText(lbl, node.x, node.y + r + fontSize)
  }, [highlightNodes, selectedNode])

  const linkCanvasObject = useCallback((link, ctx) => {
    const src = link.source
    const tgt = link.target
    if (!src?.x || !tgt?.x) return
    const isHighlighted = highlightLinks.has(link.id)
    ctx.beginPath()
    ctx.moveTo(src.x, src.y)
    ctx.lineTo(tgt.x, tgt.y)
    ctx.strokeStyle = isHighlighted ? COLORS.linkHover : COLORS.link
    ctx.lineWidth = isHighlighted ? 1.5 : 0.8
    ctx.stroke()
    if (isHighlighted) {
      const mx = (src.x + tgt.x) / 2
      const my = (src.y + tgt.y) / 2
      ctx.font = '8px monospace'
      ctx.textAlign = 'center'
      ctx.textBaseline = 'middle'
      ctx.fillStyle = COLORS.key
      ctx.fillText(link.keyLabel, mx, my - 5)
    }
  }, [highlightLinks])

  const handleClear = () => {
    const schemaNodes = (approvedSchemas ?? []).map(s => ({ id: makeSchemaId(s.name), label: getSchemaDisplayName(s), type: 'schema' }))
    setGraphData({ nodes: schemaNodes, links: [] })
    setSelectedNode(null)
    setHighlightNodes(new Set())
    setHighlightLinks(new Set())
    prepopulatedRef.current = false
    prepopulate(approvedSchemas)
  }

  const wordNodeCount   = graphData.nodes.filter(n => n.type === 'word').length
  const schemaNodeCount = graphData.nodes.filter(n => n.type === 'schema').length
  const isLoading = !!loadStatus

  return (
    <div className="flex gap-4" style={{ height: '600px' }}>
      {/* Sidebar */}
      <div className="w-56 flex-shrink-0 flex flex-col gap-3 overflow-y-auto">
        <div>
          <div className="text-xs uppercase tracking-widest text-tertiary mb-2">Search Word</div>
          <div className="flex flex-col gap-2">
            <input
              type="text"
              value={searchTerm}
              onChange={e => setSearchTerm(e.target.value)}
              onKeyDown={e => e.key === 'Enter' && handleSearch()}
              placeholder="e.g. alice"
              className="input text-sm"
              disabled={isLoading}
            />
            <button
              onClick={handleSearch}
              disabled={isSearching || isLoading || !searchTerm.trim()}
              className="btn-primary text-sm"
            >
              {isSearching ? 'Searching…' : 'Add to Graph'}
            </button>
          </div>
        </div>

        {/* Load status */}
        {loadStatus && (
          <div className="border border-border p-2 text-xs space-y-1">
            <div className="text-secondary">{loadStatus.phase}</div>
            <div className="w-full bg-surface-secondary h-1.5 rounded-full overflow-hidden">
              <div
                className="h-full bg-[#83a598] transition-all duration-300"
                style={{ width: `${loadStatus.total ? (loadStatus.progress / loadStatus.total) * 100 : 0}%` }}
              />
            </div>
            <div className="text-tertiary">{loadStatus.progress} / {loadStatus.total}</div>
          </div>
        )}

        <div className="flex flex-col gap-1 text-xs text-secondary border border-border p-2">
          <div>Schemas: <span className="text-primary font-mono">{schemaNodeCount}</span></div>
          <div>Words: <span className="text-primary font-mono">{wordNodeCount}</span></div>
          <div>Links: <span className="text-primary font-mono">{graphData.links.length}</span></div>
        </div>

        <div className="flex flex-col gap-1">
          <div className="flex items-center gap-2 text-xs text-secondary">
            <span className="inline-block w-3 h-3" style={{ background: COLORS.schema }} />
            Schema (square)
          </div>
          <div className="flex items-center gap-2 text-xs text-secondary">
            <span className="inline-block w-3 h-3 rounded-full" style={{ background: COLORS.word }} />
            Word (circle)
          </div>
          <div className="flex items-center gap-2 text-xs text-secondary">
            <span className="inline-block w-8 h-px" style={{ background: COLORS.key }} />
            Key (hover)
          </div>
        </div>

        <button
          onClick={handleClear}
          disabled={isLoading}
          className="btn-secondary text-xs"
        >
          Clear & Reload
        </button>

        {error && (
          <div className="text-xs text-gruvbox-red border border-gruvbox-red/30 p-2">
            {error}
          </div>
        )}

        {selectedNode && (
          <div className="border border-border p-2">
            <div className="text-xs uppercase tracking-widest text-tertiary mb-2">Selected</div>
            <NodeDetail node={selectedNode} links={graphData.links} nodes={graphData.nodes} />
          </div>
        )}
      </div>

      {/* Graph Canvas */}
      <div
        ref={containerRef}
        className="flex-1 border border-border overflow-hidden relative"
        style={{ background: COLORS.bg }}
      >
        {isLoading && (
          <div className="absolute inset-0 flex items-center justify-center z-10 pointer-events-none">
            <div className="text-xs text-[#928374] bg-[#282828]/80 px-3 py-1.5 border border-[#504945]">
              {loadStatus.phase}
            </div>
          </div>
        )}
        <ForceGraph2D
          ref={graphRef}
          width={dimensions.width}
          height={dimensions.height}
          graphData={graphData}
          nodeCanvasObject={nodeCanvasObject}
          nodeCanvasObjectMode={() => 'replace'}
          linkCanvasObject={linkCanvasObject}
          linkCanvasObjectMode={() => 'replace'}
          onNodeHover={handleNodeHover}
          onNodeClick={handleNodeClick}
          cooldownTicks={100}
          nodePointerAreaPaint={(node, color, ctx) => {
            const r = node.type === 'schema' ? 10 : 7
            ctx.fillStyle = color
            if (node.type === 'schema') {
              ctx.fillRect(node.x - r, node.y - r, r * 2, r * 2)
            } else {
              ctx.beginPath(); ctx.arc(node.x, node.y, r, 0, 2 * Math.PI); ctx.fill()
            }
          }}
          d3AlphaDecay={0.02}
          d3VelocityDecay={0.3}
        />
      </div>
    </div>
  )
}
