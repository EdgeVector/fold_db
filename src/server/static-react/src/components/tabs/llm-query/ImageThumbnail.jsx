import { useEffect, useState } from 'react'

/** Fetches and displays an image from the file API */
export default function ImageThumbnail({ fileHash, sourceFile }) {
  const [blobUrl, setBlobUrl] = useState(null)

  useEffect(() => {
    const url = `/api/file/${fileHash}?name=${encodeURIComponent(sourceFile || '')}`
    let revoked = false
    const userHash = localStorage.getItem('fold_user_hash')
    const headers = {}
    if (userHash) {
      headers['x-user-hash'] = userHash
      headers['x-user-id'] = userHash
    }
    fetch(url, { headers })
      .then((res) => { if (!res.ok) throw new Error(res.statusText); return res.blob() })
      .then((blob) => { if (!revoked) setBlobUrl(URL.createObjectURL(blob)) })
      .catch(() => {})
    return () => {
      revoked = true
      setBlobUrl((prev) => { if (prev) URL.revokeObjectURL(prev); return null })
    }
  }, [fileHash, sourceFile])

  if (!blobUrl) return null
  return (
    <img
      src={blobUrl}
      alt={sourceFile}
      className="max-w-xs max-h-64 rounded border border-border object-contain bg-surface-secondary"
    />
  )
}
