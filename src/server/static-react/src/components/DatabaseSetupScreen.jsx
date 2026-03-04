import { useState } from 'react'
import { applySetup } from '../api/clients/systemClient'

// Gruvbox-warm palette matching the rest of the app
const colors = {
  bg: '#282828',
  bgElevated: '#3c3836',
  border: '#504945',
  text: '#ebdbb2',
  textBright: '#fbf1c7',
  dim: '#928374',
  orange: '#fe8019',
  green: '#b8bb26',
  blue: '#83a598',
  red: '#fb4934',
}

export default function DatabaseSetupScreen({ onComplete }) {
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState(null)

  const handleLocalSetup = async () => {
    setLoading(true)
    setError(null)

    // Default local path
    const homedir = typeof window !== 'undefined'
      ? (window.__TAURI__ ? null : null) // path comes from backend default
      : null
    const defaultPath = '~/.folddb/data'

    try {
      const response = await applySetup({
        storage: { type: 'local', path: defaultPath },
      })
      if (response.success) {
        onComplete()
      } else {
        setError(response.data?.message || 'Setup failed')
      }
    } catch (e) {
      const msg = e?.message || String(e)
      if (msg.includes('could not acquire lock') || msg.includes('WouldBlock')) {
        setError(
          'Another FoldDB instance is already running and holds the database lock. ' +
          'Please close the other instance and try again.'
        )
      } else {
        setError(msg)
      }
    } finally {
      setLoading(false)
    }
  }

  const handleExememSetup = async () => {
    setLoading(true)
    setError(null)
    try {
      const response = await applySetup({
        storage: {
          type: 'exemem',
          api_url: 'https://api.exemem.com',
          api_key: '',
        },
      })
      if (response.success) {
        onComplete()
      } else {
        setError(response.data?.message || 'Setup failed')
      }
    } catch (e) {
      setError(e?.message || String(e))
    } finally {
      setLoading(false)
    }
  }

  return (
    <div style={{
      position: 'fixed', top: 0, left: 0, width: '100%', height: '100%',
      background: colors.bg, zIndex: 1000,
      display: 'flex', alignItems: 'center', justifyContent: 'center',
      fontFamily: "'IBM Plex Mono', monospace", color: colors.text,
    }}>
      <div style={{ maxWidth: '600px', width: '90%', textAlign: 'center' }}>
        <h1 style={{
          fontSize: '24px', fontWeight: 700, color: colors.textBright,
          marginBottom: '8px',
        }}>
          FoldDB
        </h1>
        <p style={{ fontSize: '14px', color: colors.dim, marginBottom: '32px' }}>
          Choose where to store your data
        </p>

        <div style={{ display: 'flex', gap: '16px', justifyContent: 'center', flexWrap: 'wrap' }}>
          {/* Local Storage Card */}
          <button
            onClick={handleLocalSetup}
            disabled={loading}
            style={{
              background: colors.bgElevated, border: `1px solid ${colors.border}`,
              padding: '24px', width: '250px', cursor: loading ? 'wait' : 'pointer',
              color: colors.text, textAlign: 'left',
              fontFamily: "'IBM Plex Mono', monospace",
              opacity: loading ? 0.6 : 1,
              transition: 'border-color 0.15s',
            }}
            onMouseEnter={e => { if (!loading) e.currentTarget.style.borderColor = colors.green }}
            onMouseLeave={e => { e.currentTarget.style.borderColor = colors.border }}
          >
            <div style={{
              fontSize: '20px', marginBottom: '8px',
            }}>
              Local Storage
            </div>
            <div style={{
              display: 'inline-block', padding: '1px 6px', fontSize: '11px',
              fontWeight: 700, background: colors.green, color: colors.bg,
              marginBottom: '12px',
            }}>
              RECOMMENDED
            </div>
            <p style={{ fontSize: '13px', color: colors.dim, margin: 0, lineHeight: '1.5' }}>
              Store data on your device. Fast, private, works offline. Uses ~/.folddb/data.
            </p>
          </button>

          {/* Exemem Cloud Card */}
          <button
            onClick={handleExememSetup}
            disabled={loading}
            style={{
              background: colors.bgElevated, border: `1px solid ${colors.border}`,
              padding: '24px', width: '250px', cursor: loading ? 'wait' : 'pointer',
              color: colors.text, textAlign: 'left',
              fontFamily: "'IBM Plex Mono', monospace",
              opacity: loading ? 0.6 : 1,
              transition: 'border-color 0.15s',
            }}
            onMouseEnter={e => { if (!loading) e.currentTarget.style.borderColor = colors.blue }}
            onMouseLeave={e => { e.currentTarget.style.borderColor = colors.border }}
          >
            <div style={{
              fontSize: '20px', marginBottom: '8px',
            }}>
              Exemem Cloud
            </div>
            <div style={{
              display: 'inline-block', padding: '1px 6px', fontSize: '11px',
              fontWeight: 700, background: colors.blue, color: colors.bg,
              marginBottom: '12px',
            }}>
              CLOUD
            </div>
            <p style={{ fontSize: '13px', color: colors.dim, margin: 0, lineHeight: '1.5' }}>
              Store data in the cloud. Syncs across devices. Requires an Exemem account.
            </p>
          </button>
        </div>

        {loading && (
          <div style={{ marginTop: '24px', color: colors.dim, fontSize: '13px' }}>
            <div style={{
              width: '20px', height: '20px',
              border: `2px solid ${colors.border}`,
              borderTopColor: colors.green,
              borderRadius: '50%',
              animation: 'spin 0.8s linear infinite',
              margin: '0 auto 8px',
            }} />
            Initializing database...
            <style>{`@keyframes spin { to { transform: rotate(360deg) } }`}</style>
          </div>
        )}

        {error && (
          <div style={{
            marginTop: '24px', padding: '12px',
            background: `${colors.red}15`, border: `1px solid ${colors.red}`,
            fontSize: '13px', color: colors.red, textAlign: 'left',
          }}>
            {error}
          </div>
        )}
      </div>
    </div>
  )
}
