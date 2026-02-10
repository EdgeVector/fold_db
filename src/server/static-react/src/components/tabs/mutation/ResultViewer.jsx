
function ResultViewer({ result }) {
  if (!result) return null

  return (
    <div className="bg-surface-secondary p-4 mt-4">
      <pre className="font-mono text-sm whitespace-pre-wrap">
        {typeof result === 'string' ? result : JSON.stringify(result, null, 2)}
      </pre>
    </div>
  )
}

export default ResultViewer
