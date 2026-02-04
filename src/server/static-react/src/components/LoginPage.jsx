import { useState } from 'react'
import { useAppDispatch, useAppSelector } from '../store/hooks'
import { loginUser } from '../store/authSlice'

export default function LoginPage() {
  const [userId, setUserId] = useState('')
  const [error, setError] = useState('')
  const dispatch = useAppDispatch()
  const { isLoading } = useAppSelector(state => state.auth)

  const handleSubmit = async (e) => {
    e.preventDefault()
    if (!userId.trim()) {
      setError('User identifier required')
      return
    }

    try {
      // loginUser thunk handles localStorage persistence internally
      await dispatch(loginUser(userId.trim())).unwrap()
    } catch (err) {
      setError(err.message)
    }
  }

  return (
    <div style={{
      minHeight: '100vh',
      background: '#fafafa',
      display: 'flex',
      flexDirection: 'column',
      justifyContent: 'center',
      padding: '48px 24px'
    }}>
      <div style={{ maxWidth: '400px', margin: '0 auto', width: '100%' }}>
        {/* Logo */}
        <div style={{ marginBottom: '64px' }}>
          <h1 style={{
            fontSize: '32px',
            fontWeight: 300,
            letterSpacing: '-1px',
            color: '#111',
            marginBottom: '8px'
          }}>
            SoverignDB
          </h1>
          <p style={{ color: '#999', fontSize: '14px' }}>
            Personal data sovereignty
          </p>
        </div>

        {/* Login form */}
        <form onSubmit={handleSubmit}>
          <div style={{ marginBottom: '24px' }}>
            <label style={{
              display: 'block',
              fontSize: '11px',
              textTransform: 'uppercase',
              letterSpacing: '2px',
              color: '#999',
              marginBottom: '12px'
            }}>
              User Identifier
            </label>
            <input
              id="userId"
              name="userId"
              type="text"
              autoComplete="username"
              required
              style={{
                width: '100%',
                padding: '16px 20px',
                border: '1px solid #e5e5e5',
                background: '#fff',
                fontSize: '15px',
                color: '#111',
                outline: 'none',
                transition: 'border-color 0.2s',
                boxSizing: 'border-box'
              }}
              placeholder="Enter your identifier"
              value={userId}
              onChange={(e) => {
                setUserId(e.target.value)
                setError('')
              }}
              onFocus={(e) => e.target.style.borderColor = '#111'}
              onBlur={(e) => e.target.style.borderColor = '#e5e5e5'}
              autoFocus
            />
          </div>

          {error && (
            <div style={{
              marginBottom: '24px',
              padding: '12px 16px',
              background: '#fff',
              border: '1px solid #fecaca',
              color: '#ef4444',
              fontSize: '14px'
            }}>
              {error}
            </div>
          )}

          <button
            type="submit"
            disabled={isLoading}
            style={{
              width: '100%',
              padding: '16px 32px',
              background: '#111',
              color: '#fff',
              border: 'none',
              fontSize: '14px',
              fontWeight: 500,
              cursor: isLoading ? 'wait' : 'pointer',
              transition: 'background 0.2s',
              opacity: isLoading ? 0.7 : 1
            }}
            onMouseOver={(e) => !isLoading && (e.target.style.background = '#333')}
            onMouseOut={(e) => !isLoading && (e.target.style.background = '#111')}
          >
            {isLoading ? 'Connecting...' : 'Continue'}
          </button>
        </form>

        {/* Tip */}
        <p style={{
          marginTop: '32px',
          color: '#999',
          fontSize: '13px',
          lineHeight: 1.6
        }}>
          Use any identifier (email, username) to create or access your node.
        </p>

        {/* Status */}
        <div style={{
          marginTop: '64px',
          display: 'flex',
          alignItems: 'center',
          gap: '8px',
          fontSize: '13px',
          color: '#999'
        }}>
          <span style={{
            width: '8px',
            height: '8px',
            background: '#22c55e',
            borderRadius: '50%'
          }}></span>
          Server online
        </div>
      </div>
    </div>
  )
}
