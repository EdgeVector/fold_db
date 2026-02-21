import React from 'react'
import { screen, fireEvent } from '@testing-library/react'
import { describe, it, expect, beforeEach, vi } from 'vitest'
import IngestionTab from '../../../components/tabs/IngestionTab'
import { renderWithRedux, createTestSchemaState, createMockAuthState } from '../../utils/testStore.jsx'

// Mock the ingestion client
vi.mock('../../../api/clients/ingestionClient', () => ({
  ingestionClient: {
    client: {
      post: vi.fn(() => Promise.resolve({
        success: true,
        data: { 
          ingestionId: 'test-ingestion-123',
          suggestedSchema: {
            name: 'auto_generated_schema',
            fields: {
              name: { field_type: 'String' },
              age: { field_type: 'Integer' }
            }
          }
        }
      })),
      get: vi.fn(() => Promise.resolve({
        success: true,
        data: { status: 'completed' }
      }))
    }
  }
}))

// Mock OpenRouter configuration
vi.mock('../../../config/openRouter', () => ({
  openRouterConfig: {
    apiKey: 'test-api-key',
    baseUrl: 'https://openrouter.ai/api/v1',
    models: {
      'gpt-4': { name: 'GPT-4', contextWindow: 8192 },
      'claude-3-sonnet': { name: 'Claude 3 Sonnet', contextWindow: 200000 }
    }
  },
  validateOpenRouterConfig: vi.fn(() => ({ isValid: true })),
  updateOpenRouterConfig: vi.fn()
}))

// Mock Redux hooks
const mockDispatch = vi.fn()
vi.mock('react-redux', async (importOriginal) => {
  const actual = await importOriginal()
  return {
    ...actual,
    useDispatch: () => mockDispatch
  }
})



// Mock localStorage for OpenRouter config
const mockLocalStorage = {
  getItem: vi.fn((key) => {
    if (key === 'openrouter_config') {
      return JSON.stringify({
        apiKey: 'stored-api-key',
        selectedModel: 'gpt-4'
      })
    }
    return null
  }),
  setItem: vi.fn(),
  removeItem: vi.fn()
}
Object.defineProperty(window, 'localStorage', { value: mockLocalStorage })

describe('IngestionTab Component', () => {
  const mockOnResult = vi.fn()

  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('renders ingestion interface', async () => {
    const authState = createMockAuthState({ isAuthenticated: true })
    const initialState = {
      auth: authState,
      ...createTestSchemaState()
    }

    await renderWithRedux(<IngestionTab onResult={mockOnResult} />, {
      preloadedState: initialState
    })

    expect(screen.getByPlaceholderText('Enter JSON data or load a sample...')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /process data/i })).toBeInTheDocument()
  })

  it('allows JSON input interaction', async () => {
    const authState = createMockAuthState({ isAuthenticated: true })
    const initialState = {
      auth: authState,
      ...createTestSchemaState()
    }

    await renderWithRedux(<IngestionTab onResult={mockOnResult} />, {
      preloadedState: initialState
    })

    const jsonInput = screen.getByPlaceholderText('Enter JSON data or load a sample...')
    
    // Test input interaction
    fireEvent.change(jsonInput, { target: { value: '{"name": "John", "age": 30}' } })
    expect(jsonInput.value).toBe('{"name": "John", "age": 30}')
  })

  it('allows data processing button interaction', async () => {
    const authState = createMockAuthState({ isAuthenticated: true })
    const initialState = {
      auth: authState,
      ...createTestSchemaState()
    }

    await renderWithRedux(<IngestionTab onResult={mockOnResult} />, {
      preloadedState: initialState
    })

    const jsonInput = screen.getByPlaceholderText('Enter JSON data or load a sample...')
    fireEvent.change(jsonInput, { target: { value: '{"name": "John", "age": 30}' } })

    const processButton = screen.getByRole('button', { name: /process data/i })
    expect(processButton).toBeInTheDocument()
    expect(processButton).not.toBeDisabled()
  })

  it('provides sample data loading functionality', async () => {
    const authState = createMockAuthState({ isAuthenticated: true })
    const initialState = {
      auth: authState,
      ...createTestSchemaState()
    }

    await renderWithRedux(<IngestionTab onResult={mockOnResult} />, {
      preloadedState: initialState
    })

    const twitterButton = screen.getByRole('button', { name: 'Twitter' })
    fireEvent.click(twitterButton)

    const jsonInput = screen.getByPlaceholderText('Enter JSON data or load a sample...')
    expect(jsonInput.value).toContain('"post_id":') // Should contain sample data (formatted JSON)
  })

  it('loads different social media samples', async () => {
    const authState = createMockAuthState({ isAuthenticated: true })
    const initialState = {
      auth: authState,
      ...createTestSchemaState()
    }

    await renderWithRedux(<IngestionTab onResult={mockOnResult} />, {
      preloadedState: initialState
    })

    const jsonInput = screen.getByPlaceholderText('Enter JSON data or load a sample...')

    // Test Twitter sample
    const twitterButton = screen.getByRole('button', { name: 'Twitter' })
    fireEvent.click(twitterButton)
    expect(jsonInput.value).toContain('"post_id": "tweet_')

    // Test Instagram sample
    const instagramButton = screen.getByRole('button', { name: 'Instagram' })
    fireEvent.click(instagramButton)
    expect(jsonInput.value).toContain('"username":')

    // Test LinkedIn sample
    const linkedinButton = screen.getByRole('button', { name: 'Linkedin' })
    fireEvent.click(linkedinButton)
    expect(jsonInput.value).toContain('"author":')

    // Test TikTok sample
    const tiktokButton = screen.getByRole('button', { name: 'Tiktok' })
    fireEvent.click(tiktokButton)
    expect(jsonInput.value).toContain('"video_id":')
  })
})
