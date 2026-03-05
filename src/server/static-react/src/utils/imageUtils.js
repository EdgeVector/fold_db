const IMAGE_EXTENSIONS = /\.(jpe?g|png|gif|webp|svg)$/i

/** Try to extract fileHash+sourceFile from a metadata-like object */
export function extractImageFromMeta(meta, seen, images) {
  if (!meta || typeof meta !== 'object') return
  const sourceFile = meta.source_file_name
  const fileHash = meta.metadata?.file_hash || meta.file_hash
  if (sourceFile && fileHash && IMAGE_EXTENSIONS.test(sourceFile) && !seen.has(fileHash)) {
    seen.add(fileHash)
    images.push({ fileHash, sourceFile })
  }
}

/** Extract unique image references from tool call results */
export function extractImagesFromToolCalls(toolCalls) {
  const images = []
  const seen = new Set()
  if (!Array.isArray(toolCalls)) return images
  for (const tc of toolCalls) {
    const results = Array.isArray(tc.result) ? tc.result : []
    for (const record of results) {
      const metadata = record?.metadata
      if (metadata && typeof metadata === 'object') {
        for (const val of Object.values(metadata)) {
          extractImageFromMeta(val, seen, images)
        }
        extractImageFromMeta(metadata, seen, images)
      }
    }
  }
  return images
}
