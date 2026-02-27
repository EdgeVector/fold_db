import React, { useState } from 'react'
import { systemClient } from '../../api/clients/systemClient'

export default function CloudMigrationSettings({ onClose }) {
  const [migrationMode, setMigrationMode] = useState('encryption_at_rest')
  const [apiUrl, setApiUrl] = useState('')
  const [apiKey, setApiKey] = useState('')
  const [isMigrating, setIsMigrating] = useState(false)
  const [error, setError] = useState(null)
  const [success, setSuccess] = useState(false)
  
  const [switching, setSwitching] = useState(false)
  const [switchError, setSwitchError] = useState(null)

  const handleMigrate = async () => {
    if (!apiUrl || !apiKey) {
      setError('Target Cloud API URL and API Key are required.')
      return
    }
    
    // In current implementation, E2E encryption is not technically complete in the
    // backend so we block it strictly in UI for now as requested or warn if needed.
    // Assuming backend only supports encryption_at_rest right now based on Walkthrough.
    if (migrationMode === 'e2e_encryption') {
      setError('End-to-End Encryption mode is currently in development. Please select Encryption at Rest.')
      return
    }

    setIsMigrating(true)
    setError(null)
    try {
      const response = await systemClient.migrateToCloud(apiUrl, apiKey)
      if (response.success || response.ok || response.data?.success) {
        setSuccess(true)
      } else {
        setError(response.error || response.data?.error || 'Migration failed for an unknown reason.')
      }
    } catch (err) {
      setError(err.message || 'Network error during migration. Check your console logs.')
    } finally {
      setIsMigrating(false)
    }
  }

  const handleSwitchToCloud = async () => {
    setSwitching(true)
    setSwitchError(null)
    try {
      await systemClient.applySetup({
        storage: {
          type: 'exemem',
          api_url: apiUrl.trim(),
          api_key: apiKey.trim()
        }
      })
      // Server invalidates the node immediately, but we should reload the window
      // so all React client stores and headers re-fetch the new config/status.
      window.location.reload()
    } catch (err) {
      console.error('Failed to switch database configuration:', err)
      setSwitchError(err.response?.data?.message || err.message || 'Failed to apply configuration. Please restart the instance manually.')
      setSwitching(false)
    }
  }

  if (success) {
    return (
      <div className="flex flex-col gap-6 w-full max-w-2xl text-gruvbox-bright p-4 border border-gruvbox-dark rounded-md bg-surface-elevated shadow-lg">
        <div className="flex items-start gap-4 p-4 border border-gruvbox-green/50 bg-gruvbox-green/5 rounded-md">
          <div className="text-gruvbox-green mt-1">
            <svg className="w-6 h-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" />
            </svg>
          </div>
          <div className="flex-1">
            <h4 className="text-sm font-bold text-gruvbox-green tracking-wide">Migration Complete!</h4>
            <p className="text-xs text-gruvbox-light mt-2 leading-relaxed">
              Your data has been successfully securely synchronized to your XMEM Cloud instance.
            </p>
            
            <div className="mt-4 p-3 border border-border bg-surface rounded text-xs text-gruvbox-light leading-relaxed">
              <strong className="block text-gruvbox-bright mb-1">Path Forward:</strong>
              You can now update your local FoldDB to act as a thin client connecting directly to your new cloud database. 
              <span className="block mt-2 text-gruvbox-dim italic">
                Note: Your local database file will be preserved safely on your hard drive as a physical backup.
              </span>
            </div>

            {switchError && (
              <div className="mt-3 text-xs text-gruvbox-red bg-gruvbox-red/10 p-2 border border-gruvbox-red/20 rounded">
                {switchError}
              </div>
            )}
          </div>
        </div>
        <div className="flex justify-end gap-3 pt-4 border-t border-border">
          <button 
            onClick={onClose} 
            disabled={switching}
            className="px-4 py-2 text-xs border border-border text-gruvbox-dim hover:text-gruvbox-bright rounded-md transition-colors cursor-pointer"
          >
            Keep Running Locally
          </button>
          <button 
            onClick={handleSwitchToCloud} 
            disabled={switching}
            className="px-4 py-2 text-xs font-bold border border-gruvbox-green text-surface bg-gruvbox-green hover:bg-gruvbox-green/90 rounded-md transition-colors cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed flex items-center gap-2"
          >
            {switching ? (
              <svg className="animate-spin h-3 w-3 text-surface" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4"></circle>
                <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
              </svg>
            ) : null}
            {switching ? 'Switching Configuration...' : 'Switch to Cloud Database'}
          </button>
        </div>
      </div>
    )
  }

  return (
    <div className="flex flex-col gap-6 w-full max-w-2xl text-gruvbox-bright p-4 border border-border rounded-md bg-surface shadow-md">
      
      {/* Explanation Banner */}
      <div className="flex items-start gap-4 p-4 border border-gruvbox-blue bg-gruvbox-blue/5 rounded-md">
        <div className="text-gruvbox-blue mt-1 flex-shrink-0">
          <svg className="w-6 h-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
          </svg>
        </div>
        <div>
          <h4 className="text-sm font-bold text-gruvbox-blue mb-1">Migrate Local Node to FoldDB Cloud</h4>
          <p className="text-xs text-gruvbox-light leading-relaxed">
            Move your local storage securely to a unified cloud environment. You will be able to access your data 
            from anywhere without worrying about local file losses. Data stored locally after a successful sync can be removed.
          </p>
        </div>
      </div>

      {/* Migration Strategy Selection */}
      <div className="flex flex-col gap-2">
        <label className="text-xs uppercase tracking-widest text-gruvbox-light font-bold">Migration Privacy Mode</label>
        
        <div className="flex flex-col gap-3 mt-1">
          <label className={`cursor-pointer p-4 border rounded-md flex items-start gap-3 transition-colors ${migrationMode === 'encryption_at_rest' ? 'border-gruvbox-yellow bg-gruvbox-yellow/5' : 'border-border hover:border-gruvbox-dim'}`}>
            <input 
              type="radio" 
              name="migrationMode" 
              value="encryption_at_rest" 
              className="mt-1"
              checked={migrationMode === 'encryption_at_rest'}
              onChange={() => setMigrationMode('encryption_at_rest')}
            />
            <div>
              <div className="text-sm font-bold text-gruvbox-bright">Encryption at Rest</div>
              <div className="text-xs text-gruvbox-dim mt-1">
                Transfers decoded schemas and data over TLS. Data is encrypted natively by AWS (DynamoDB/S3 KMS) but allows the FoldDB cloud worker to run operations over the data.
                <span className="block mt-1 text-gruvbox-green font-mono">✓ Enables AI Queries & Cloud Native Indexing</span>
              </div>
            </div>
          </label>

          <label className={`cursor-pointer p-4 border rounded-md flex items-start gap-3 transition-colors ${migrationMode === 'e2e_encryption' ? 'border-gruvbox-yellow bg-gruvbox-yellow/5' : 'border-border hover:border-gruvbox-dim'}`}>
            <input 
              type="radio" 
              name="migrationMode" 
              value="e2e_encryption" 
              className="mt-1"
              checked={migrationMode === 'e2e_encryption'}
              onChange={() => setMigrationMode('e2e_encryption')}
            />
            <div>
              <div className="text-sm font-bold text-gruvbox-bright flex items-center gap-2">
                End-to-End Encryption
                <span className="bg-surface-elevated text-[10px] uppercase font-bold px-2 py-0.5 rounded text-gruvbox-red border border-gruvbox-red/20">Coming Soon</span>
              </div>
              <div className="text-xs text-gruvbox-dim mt-1">
                Maximum zero-knowledge privacy. Data is encrypted using your local private key before reaching the cloud. The Cloud Worker stores opaque bytes.
                <span className="block mt-1 text-gruvbox-orange font-mono">⚠ Disables server-side AI Queries</span>
              </div>
            </div>
          </label>
        </div>
      </div>

      {/* Cloud Configuration Keys */}
      <div className="flex flex-col gap-4">
        <div className="flex flex-col gap-2">
          <label className="text-xs uppercase tracking-widest text-gruvbox-light font-bold">
            Cloud API Instance URL
          </label>
          <input
            type="text"
            className="w-full bg-surface border border-border p-2 text-sm text-primary font-mono outline-none focus:border-gruvbox-yellow transition-colors rounded-sm"
            placeholder="https://api.folddb.com"
            value={apiUrl}
            onChange={(e) => setApiUrl(e.target.value)}
          />
        </div>

        <div className="flex flex-col gap-2">
          <label className="text-xs uppercase tracking-widest text-gruvbox-light font-bold">
            Target API Key (or Session Token)
          </label>
          <input
            type="password"
            className="w-full bg-surface border border-border p-2 text-sm text-primary font-mono outline-none focus:border-gruvbox-yellow transition-colors rounded-sm"
            placeholder="sk-**************"
            value={apiKey}
            onChange={(e) => setApiKey(e.target.value)}
          />
        </div>
      </div>

      {error && (
        <div className="flex items-start gap-3 p-3 border border-gruvbox-red bg-gruvbox-red/5 mt-2 rounded-md py-4">
          <span className="text-gruvbox-red text-sm flex-shrink-0 mt-0.5">⚠️</span>
          <p className="text-xs text-gruvbox-red leading-relaxed">{error}</p>
        </div>
      )}

      {/* Footer Buttons */}
      <div className="flex justify-end pt-4 border-t border-border mt-4">
        <button
          onClick={handleMigrate}
          disabled={isMigrating}
          className={`font-bold px-6 py-2 rounded-md transition-colors border ${
            isMigrating || migrationMode === 'e2e_encryption'
              ? 'border-gruvbox-dark text-gruvbox-dim cursor-not-allowed bg-transparent'
              : 'border-gruvbox-yellow text-gruvbox-yellow hover:bg-gruvbox-yellow hover:text-surface cursor-pointer'
          }`}
        >
          {isMigrating ? 'Migrating...' : 'Start Migration'}
        </button>
      </div>

    </div>
  )
}
