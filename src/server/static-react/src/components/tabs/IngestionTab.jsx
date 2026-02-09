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
        <div className="card-terminal p-3 border-l-4 border-terminal-green">
          <div className="flex items-center gap-4 text-sm">
            <span className={`badge-terminal ${
              ingestionStatus.enabled && ingestionStatus.configured 
                ? 'badge-terminal-success' 
                : 'badge-terminal-error'
            }`}>
              {ingestionStatus.enabled && ingestionStatus.configured ? 'Ready' : 'Not Configured'}
            </span>
            <span className="text-terminal-dim">{ingestionStatus.provider} · {ingestionStatus.model}</span>
            <span className="text-xs text-terminal-dim">Configure AI settings using the Settings button in the header</span>
          </div>
        </div>
      )}


      <div className="card-terminal p-4">
        <div className="flex items-center justify-between mb-3">
          <h3 className="text-terminal-green font-medium">
            <span className="text-terminal-dim">$</span> JSON Data
          </h3>
          <div className="flex gap-2">
            <button
              onClick={() => loadSampleData('blogposts')}
              className="btn-terminal text-xs py-1 px-3"
            >
              Blog Posts (100)
            </button>
            <button
              onClick={() => loadSampleData('twitter')}
              className="btn-terminal text-xs py-1 px-3"
            >
              Twitter
            </button>
            <button
              onClick={() => loadSampleData('instagram')}
              className="btn-terminal text-xs py-1 px-3"
            >
              Instagram
            </button>
            <button
              onClick={() => loadSampleData('linkedin')}
              className="btn-terminal text-xs py-1 px-3"
            >
              LinkedIn
            </button>
            <button
              onClick={() => loadSampleData('tiktok')}
              className="btn-terminal text-xs py-1 px-3"
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
          className="textarea-terminal w-full h-64"
        />
      </div>

      {/* Process Button */}
      <div className="card-terminal p-4">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-4">
            <label className="flex items-center gap-2 text-sm cursor-pointer">
              <input
                type="checkbox"
                checked={autoExecute}
                onChange={(e) => setAutoExecute(e.target.checked)}
                className="w-4 h-4 accent-terminal-green bg-terminal border-terminal"
              />
              <span className="text-terminal">Auto-execute mutations</span>
            </label>
            <span className="text-xs text-terminal-dim">AI will analyze and automatically map data to schemas</span>
          </div>
          
          <button
            onClick={processIngestion}
            disabled={isLoading || !jsonData.trim()}
            className={`btn-terminal px-6 py-2.5 font-medium ${
              isLoading || !jsonData.trim()
                ? 'opacity-50 cursor-not-allowed'
                : 'btn-terminal-primary'
            }`}
          >
            {isLoading ? (
              <>
                <span className="spinner-terminal"></span>
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
