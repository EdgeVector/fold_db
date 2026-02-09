import { useState, useEffect } from 'react'
import { ingestionClient } from '../../api/clients'
import { generateBlogPosts } from '../../data/sampleBlogPosts'
import { twitterSamples, instagramSamples, linkedinSamples, tiktokSamples } from '../../data/sampleSocialPosts'

function IngestionTab({ onResult }) {
  const [jsonData, setJsonData] = useState('')
  const [autoExecute, setAutoExecute] = useState(true)
  const [isLoading, setIsLoading] = useState(false)
  const [ingestionStatus, setIngestionStatus] = useState(null)

  useEffect(() => {
    fetchIngestionStatus()
  }, [])

  const fetchIngestionStatus = async () => {
    try {
      const response = await ingestionClient.getStatus()
      if (response.success) {
        setIngestionStatus(response.data)
      }
    } catch (error) {
      console.error('Failed to fetch ingestion status:', error)
    }
  }

  const processIngestion = async () => {
    setIsLoading(true)
    
    // Clear any previous results
    onResult(null)
    
    try {
      const parsedData = JSON.parse(jsonData)

      const options = {
        autoExecute,
        trustDistance: 0,
        pubKey: 'default'
      }

      const response = await ingestionClient.processIngestion(parsedData, options)
      
      if (response.success) {
        onResult({
          success: true,
          data: response.data
        })
        setJsonData('') // Clear the form on success
      } else {
        onResult({
          success: false,
          error: 'Failed to process ingestion'
        })
      }
    } catch (error) {
      onResult({
        success: false,
        error: error.message || 'Failed to process ingestion'
      })
    } finally {
      setIsLoading(false)
    }
  }

  const loadSampleData = (sampleType) => {
    const samples = {
      blogposts: generateBlogPosts(),
      twitter: twitterSamples,
      instagram: instagramSamples,
      linkedin: linkedinSamples,
      tiktok: tiktokSamples
    }
    setJsonData(JSON.stringify(samples[sampleType], null, 2))
  }

  return (
    <div className="space-y-4">
      {/* Status Bar */}
      {ingestionStatus && (
        <div className="minimal-card p-3 border-l-4" style={{ borderLeftColor: ingestionStatus.enabled && ingestionStatus.configured ? 'var(--color-success)' : 'var(--color-error)' }}>
          <div className="flex items-center gap-4 text-sm">
            <span className={`minimal-badge ${
              ingestionStatus.enabled && ingestionStatus.configured 
                ? 'minimal-badge-success' 
                : 'minimal-badge-error'
            }`}>
              {ingestionStatus.enabled && ingestionStatus.configured ? 'Ready' : 'Not Configured'}
            </span>
            <span className="text-secondary">{ingestionStatus.provider} · {ingestionStatus.model}</span>
            <span className="text-xs text-secondary">Configure AI settings using the Settings button in the header</span>
          </div>
        </div>
      )}


      <div className="minimal-card p-4">
        <div className="flex items-center justify-between mb-3">
          <h3 className="text-primary font-medium">
            JSON Data
          </h3>
          <div className="flex gap-2">
            <button
              onClick={() => loadSampleData('blogposts')}
              className="minimal-btn-secondary minimal-btn-sm"
            >
              Blog Posts (100)
            </button>
            <button
              onClick={() => loadSampleData('twitter')}
              className="minimal-btn-secondary minimal-btn-sm"
            >
              Twitter
            </button>
            <button
              onClick={() => loadSampleData('instagram')}
              className="minimal-btn-secondary minimal-btn-sm"
            >
              Instagram
            </button>
            <button
              onClick={() => loadSampleData('linkedin')}
              className="minimal-btn-secondary minimal-btn-sm"
            >
              LinkedIn
            </button>
            <button
              onClick={() => loadSampleData('tiktok')}
              className="minimal-btn-secondary minimal-btn-sm"
            >
              TikTok
            </button>
          </div>
        </div>
        
        <textarea
          id="jsonData"
          value={jsonData}
          onChange={(e) => setJsonData(e.target.value)}
          placeholder="Enter your JSON data here or load a sample..."
          className="minimal-textarea w-full h-64"
        />
      </div>

      {/* Process Button */}
      <div className="minimal-card p-4">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-4">
            <label className="flex items-center gap-2 text-sm cursor-pointer">
              <input
                type="checkbox"
                checked={autoExecute}
                onChange={(e) => setAutoExecute(e.target.checked)}
              />
              <span className="text-primary">Auto-execute mutations</span>
            </label>
            <span className="text-xs text-secondary">AI will analyze and automatically map data to schemas</span>
          </div>
          
          <button
            onClick={processIngestion}
            disabled={isLoading || !jsonData.trim()}
            className={`minimal-btn px-6 py-2.5 font-medium ${
              isLoading || !jsonData.trim()
                ? 'opacity-50 cursor-not-allowed'
                : ''
            }`}
          >
            {isLoading ? (
              <>
                <span className="minimal-spinner inline-block w-4 h-4 border-width-1"></span>
                <span>Processing...</span>
              </>
            ) : (
              <>
                <span>→</span>
                <span>Process Data</span>
              </>
            )}
          </button>
        </div>
      </div>
    </div>
  )
}

export default IngestionTab
