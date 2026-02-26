import { useState } from 'react'
import { ingestionClient } from '../../api/clients'
import { generateBlogPosts } from '../../data/sampleBlogPosts'
import { twitterSamples, instagramSamples, linkedinSamples, tiktokSamples } from '../../data/sampleSocialPosts'

const SAMPLES_ENABLED = import.meta.env.VITE_ENABLE_SAMPLES === 'true'

function IngestionTab({ onResult }) {
  const [jsonData, setJsonData] = useState('')
  const [autoExecute, setAutoExecute] = useState(true)
  const [isLoading, setIsLoading] = useState(false)

  const processIngestion = async () => {
    setIsLoading(true)
    onResult(null)
    try {
      const parsedData = JSON.parse(jsonData)
      const response = await ingestionClient.processIngestion(parsedData, { autoExecute, pubKey: 'default' })
      if (response.success) {
        onResult({ success: true, data: response.data })
        setJsonData('')
      } else {
        onResult({ success: false, error: 'Failed to process ingestion' })
      }
    } catch (error) {
      onResult({ success: false, error: (error instanceof Error ? error.message : String(error)) || 'Failed to process ingestion' })
    } finally {
      setIsLoading(false)
    }
  }

  const loadSampleData = (type) => {
    const samples = { blogposts: generateBlogPosts(), twitter: twitterSamples, instagram: instagramSamples, linkedin: linkedinSamples, tiktok: tiktokSamples }
    setJsonData(JSON.stringify(samples[type], null, 2))
  }

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        {SAMPLES_ENABLED && (
          <div className="flex gap-2">
            {['blogposts', 'twitter', 'instagram', 'linkedin', 'tiktok'].map(t => (
              <button key={t} onClick={() => loadSampleData(t)} className="btn-secondary btn-sm">
                {t === 'blogposts' ? 'Blog Posts (100)' : t.charAt(0).toUpperCase() + t.slice(1)}
              </button>
            ))}
          </div>
        )}
        <label className="flex items-center gap-2 text-sm cursor-pointer">
          <input type="checkbox" checked={autoExecute} onChange={(e) => setAutoExecute(e.target.checked)} className="checkbox" />
          <span className="text-secondary">Auto-execute</span>
        </label>
      </div>

      <textarea
        value={jsonData}
        onChange={(e) => setJsonData(e.target.value)}
        placeholder="Enter JSON data or load a sample..."
        className="textarea h-72 font-mono"
      />

      <div className="flex justify-end">
        <button onClick={processIngestion} disabled={isLoading || !jsonData.trim()} className="btn-primary btn-lg flex items-center gap-2">
          {isLoading ? <><span className="spinner" />Processing...</> : <>→ Process Data</>}
        </button>
      </div>
    </div>
  )
}

export default IngestionTab
