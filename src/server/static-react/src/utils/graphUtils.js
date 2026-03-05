const STOPWORDS = new Set([
  'the','and','for','are','but','not','you','all','can','her','was','one',
  'our','out','get','has','him','his','how','new','now','old','see','two',
  'way','who','did','its','let','put','say','she','too','use','that','this',
  'with','from','they','been','have','will','more','also','than','then',
  'when','just','over','into','some','what','your','would','could','which',
])

const SEARCH_BATCH = 8

export function makeSchemaId(name) { return `schema:${name}` }
export function makeWordId(term)   { return `word:${term}` }

export function formatKey(kv) {
  const parts = []
  if (kv?.hash)  parts.push(kv.hash.slice(0, 12))
  if (kv?.range) parts.push(kv.range.slice(0, 12))
  return parts.join(' / ') || '—'
}

export function mergeGraphData(prev, newNodes, newLinks) {
  const nodeMap = new Map(prev.nodes.map(n => [n.id, n]))
  const linkMap = new Map(prev.links.map(l => [l.id, l]))
  for (const n of newNodes) if (!nodeMap.has(n.id)) nodeMap.set(n.id, n)
  for (const l of newLinks) if (!linkMap.has(l.id)) linkMap.set(l.id, l)
  return { nodes: Array.from(nodeMap.values()), links: Array.from(linkMap.values()) }
}

export function extractWordsFromRecord(record) {
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

export function buildFromResults(results) {
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

export async function searchBatch(words, nativeIndexClient, onBatchResult, onWordComplete) {
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
