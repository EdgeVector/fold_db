import React from 'react'
import { screen, fireEvent, waitFor } from '@testing-library/react'
import { describe, it, expect, beforeEach, vi } from 'vitest'
import WordGraphTab from '../../../components/tabs/WordGraphTab'
import { renderWithRedux, createTestSchemaState } from '../../utils/testStore.jsx'

// react-force-graph-2d uses canvas/WebGL — replace with a testable stub
vi.mock('react-force-graph-2d', () => ({
  default: vi.fn(({ graphData, onNodeClick, onNodeHover }) => (
    <div data-testid="force-graph">
      <span data-testid="graph-node-count">{graphData?.nodes?.length ?? 0}</span>
      <span data-testid="graph-link-count">{graphData?.links?.length ?? 0}</span>
      {graphData?.nodes?.map(n => (
        <button
          key={n.id}
          data-testid={`node-${n.id}`}
          onClick={() => onNodeClick?.(n)}
          onMouseEnter={() => onNodeHover?.(n)}
          onMouseLeave={() => onNodeHover?.(null)}
        >
          {n.label}
        </button>
      ))}
    </div>
  ))
}))

vi.mock('../../../api/clients/nativeIndexClient', () => ({
  nativeIndexClient: { search: vi.fn() },
  NativeIndexClient: vi.fn()
}))

vi.mock('../../../api/clients/mutationClient', () => ({
  mutationClient: { executeQuery: vi.fn() }
}))

vi.mock('../../../api/clients/schemaClient', () => ({
  default:      { listSchemaKeys: vi.fn() },
  schemaClient: { listSchemaKeys: vi.fn() }
}))

vi.mock('../../../store/schemaSlice', async () => {
  const actual = await vi.importActual('../../../store/schemaSlice')
  const thunk = vi.fn(() => {
    const fn = () => Promise.resolve({ type: 'schemas/fetchSchemas/fulfilled', payload: undefined })
    fn.fulfilled = { match: () => true }
    return fn
  })
  return { ...actual, fetchSchemas: thunk }
})

import { nativeIndexClient } from '../../../api/clients/nativeIndexClient'
import { mutationClient }    from '../../../api/clients/mutationClient'
import { schemaClient }      from '../../../api/clients/schemaClient'

const APPROVED_SCHEMA_STATE = createTestSchemaState({
  schemas: {
    UserProfile: { name: 'UserProfile', state: 'Approved', fields: ['name', 'bio'] },
    TweetData:   { name: 'TweetData',   state: 'Approved', fields: ['text', 'author'] },
  }
})

function makeSearchResult(word, schema, hash = 'abc123') {
  return { schema_name: schema, field: 'name', key_value: { hash, range: null }, value: word }
}

async function renderTab(schemaState = APPROVED_SCHEMA_STATE) {
  return renderWithRedux(<WordGraphTab />, { preloadedState: schemaState })
}

// Wait for schema nodes to appear AND for the search input to be enabled.
// The input is disabled={isLoading} — it becomes enabled once auto-populate completes.
async function waitForReady() {
  await waitFor(() => {
    expect(screen.getByTestId('graph-node-count').textContent).toBe('2')
    expect(screen.getByPlaceholderText('e.g. alice')).not.toBeDisabled()
  }, { timeout: 5000 })
}

describe('WordGraphTab', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    // Default: no-op responses so prepopulate finishes immediately
    nativeIndexClient.search.mockResolvedValue({ success: true, data: { results: [] } })
    mutationClient.executeQuery.mockResolvedValue({ success: true, data: { results: [] } })
    schemaClient.listSchemaKeys.mockResolvedValue({ success: true, data: { keys: [], total_count: 0 } })
  })

  describe('initial render', () => {
    it('renders search input and Add to Graph button', async () => {
      await renderTab()
      expect(screen.getByPlaceholderText('e.g. alice')).toBeInTheDocument()
      expect(screen.getByText('Add to Graph')).toBeInTheDocument()
    })

    it('renders force graph canvas', async () => {
      await renderTab()
      expect(screen.getByTestId('force-graph')).toBeInTheDocument()
    })

    it('renders legend items', async () => {
      await renderTab()
      expect(screen.getByText(/Schema \(square\)/)).toBeInTheDocument()
      expect(screen.getByText(/Word \(circle\)/)).toBeInTheDocument()
    })

    it('renders Clear & Reload button', async () => {
      await renderTab()
      expect(screen.getByText('Clear & Reload')).toBeInTheDocument()
    })
  })

  describe('schema nodes', () => {
    it('shows schema nodes from approved schemas', async () => {
      await renderTab()
      await waitFor(() => {
        expect(screen.getByTestId('graph-node-count').textContent).toBe('2')
      }, { timeout: 5000 })
    })
  })

  describe('manual search', () => {
    it('calls nativeIndexClient.search with the entered term', async () => {
      await renderTab()
      await waitForReady()

      fireEvent.change(screen.getByPlaceholderText('e.g. alice'), { target: { value: 'alice' } })
      fireEvent.click(screen.getByText('Add to Graph'))

      await waitFor(() => {
        expect(nativeIndexClient.search).toHaveBeenCalledWith('alice')
      }, { timeout: 3000 })
    })

    it('triggers search on Enter key', async () => {
      await renderTab()
      await waitForReady()

      const input = screen.getByPlaceholderText('e.g. alice')
      fireEvent.change(input, { target: { value: 'bob' } })
      fireEvent.keyDown(input, { key: 'Enter' })

      await waitFor(() => {
        expect(nativeIndexClient.search).toHaveBeenCalledWith('bob')
      }, { timeout: 3000 })
    })

    it('adds word node and link from search result', async () => {
      nativeIndexClient.search.mockResolvedValue({
        success: true,
        data: { results: [makeSearchResult('alice', 'UserProfile', 'h1')] }
      })

      await renderTab()
      await waitForReady()

      fireEvent.change(screen.getByPlaceholderText('e.g. alice'), { target: { value: 'alice' } })
      fireEvent.click(screen.getByText('Add to Graph'))

      await waitFor(() => {
        expect(screen.getByTestId('graph-node-count').textContent).toBe('3')
        expect(screen.getByTestId('graph-link-count').textContent).toBe('1')
      }, { timeout: 3000 })
    })

    it('shows error message when search returns no results', async () => {
      await renderTab()
      await waitForReady()

      fireEvent.change(screen.getByPlaceholderText('e.g. alice'), { target: { value: 'xyz' } })
      fireEvent.click(screen.getByText('Add to Graph'))

      await waitFor(() => {
        expect(screen.getByText(/No index entries for "xyz"/)).toBeInTheDocument()
      }, { timeout: 3000 })
    })

    it('shows error message on search API failure', async () => {
      nativeIndexClient.search.mockResolvedValue({ success: false, error: 'Server error' })

      await renderTab()
      await waitForReady()

      fireEvent.change(screen.getByPlaceholderText('e.g. alice'), { target: { value: 'fail' } })
      fireEvent.click(screen.getByText('Add to Graph'))

      await waitFor(() => {
        expect(screen.getByText('Server error')).toBeInTheDocument()
      }, { timeout: 3000 })
    })

    it('disables Add to Graph button when input is empty', async () => {
      await renderTab()
      // Wait for loading to finish — button stays disabled due to empty input
      await waitFor(() => expect(screen.getByPlaceholderText('e.g. alice')).not.toBeDisabled(), { timeout: 5000 })
      expect(screen.getByText('Add to Graph')).toBeDisabled()
    })
  })

  describe('auto-populate on mount', () => {
    it('queries records for each approved schema on mount', async () => {
      mutationClient.executeQuery.mockResolvedValue({
        success: true,
        data: { results: [{ fields: { name: 'Alice Smith' } }] }
      })

      await renderTab()

      await waitFor(() => {
        expect(mutationClient.executeQuery).toHaveBeenCalledWith(
          expect.objectContaining({ schema_name: 'UserProfile' })
        )
      }, { timeout: 5000 })
    })

    it('searches words extracted from record field values', async () => {
      mutationClient.executeQuery.mockResolvedValue({
        success: true,
        data: { results: [{ fields: { name: 'uniquewordxyz' } }] }
      })

      await renderTab()

      await waitFor(() => {
        expect(nativeIndexClient.search).toHaveBeenCalledWith('uniquewordxyz')
      }, { timeout: 5000 })
    })

    it('filters out stopwords and short words', async () => {
      mutationClient.executeQuery.mockResolvedValue({
        success: true,
        data: { results: [{ fields: { bio: 'the and for realword' } }] }
      })

      await renderTab()

      await waitFor(() => {
        expect(nativeIndexClient.search).toHaveBeenCalledWith('realword')
      }, { timeout: 5000 })

      expect(nativeIndexClient.search).not.toHaveBeenCalledWith('the')
      expect(nativeIndexClient.search).not.toHaveBeenCalledWith('and')
      expect(nativeIndexClient.search).not.toHaveBeenCalledWith('for')
    })

    it('adds word nodes from auto-populate results', async () => {
      mutationClient.executeQuery.mockResolvedValue({
        success: true,
        data: { results: [{ fields: { name: 'graphword' } }] }
      })
      nativeIndexClient.search.mockImplementation(async (term) => {
        if (term === 'graphword') {
          return { success: true, data: { results: [makeSearchResult('graphword', 'UserProfile', 'h1')] } }
        }
        return { success: true, data: { results: [] } }
      })

      await renderTab()

      await waitFor(() => {
        expect(screen.getByTestId('graph-node-count').textContent).toBe('3')
      }, { timeout: 5000 })
    })
  })

  describe('graph deduplication', () => {
    it('does not add duplicate word nodes on repeated searches', async () => {
      nativeIndexClient.search.mockResolvedValue({
        success: true,
        data: { results: [makeSearchResult('alice', 'UserProfile', 'h1')] }
      })

      await renderTab()
      await waitForReady()

      fireEvent.change(screen.getByPlaceholderText('e.g. alice'), { target: { value: 'alice' } })
      fireEvent.click(screen.getByText('Add to Graph'))
      await waitFor(() => expect(screen.getByTestId('graph-node-count').textContent).toBe('3'), { timeout: 3000 })

      fireEvent.click(screen.getByText('Add to Graph'))
      await waitFor(() => {
        expect(screen.getByTestId('graph-node-count').textContent).toBe('3')
      }, { timeout: 3000 })
    })
  })

  describe('node selection', () => {
    it('shows selected node detail panel when a word node is clicked', async () => {
      nativeIndexClient.search.mockResolvedValue({
        success: true,
        data: { results: [makeSearchResult('alice', 'UserProfile', 'h1')] }
      })

      await renderTab()
      await waitForReady()

      fireEvent.change(screen.getByPlaceholderText('e.g. alice'), { target: { value: 'alice' } })
      fireEvent.click(screen.getByText('Add to Graph'))
      await waitFor(() => expect(screen.getByTestId('graph-node-count').textContent).toBe('3'), { timeout: 3000 })

      fireEvent.click(screen.getByTestId('node-word:alice'))

      await waitFor(() => {
        expect(screen.getByText('Selected')).toBeInTheDocument()
        expect(screen.getByText('word')).toBeInTheDocument()
      }, { timeout: 3000 })
    })

    it('deselects node when clicked again', async () => {
      nativeIndexClient.search.mockResolvedValue({
        success: true,
        data: { results: [makeSearchResult('alice', 'UserProfile', 'h1')] }
      })

      await renderTab()
      await waitForReady()

      fireEvent.change(screen.getByPlaceholderText('e.g. alice'), { target: { value: 'alice' } })
      fireEvent.click(screen.getByText('Add to Graph'))
      await waitFor(() => expect(screen.getByTestId('graph-node-count').textContent).toBe('3'), { timeout: 3000 })

      fireEvent.click(screen.getByTestId('node-word:alice'))
      await waitFor(() => expect(screen.getByText('Selected')).toBeInTheDocument(), { timeout: 3000 })

      fireEvent.click(screen.getByTestId('node-word:alice'))
      await waitFor(() => expect(screen.queryByText('Selected')).not.toBeInTheDocument(), { timeout: 3000 })
    })
  })

  describe('clear & reload', () => {
    it('removes word nodes and links, keeps schema nodes', async () => {
      nativeIndexClient.search.mockResolvedValue({
        success: true,
        data: { results: [makeSearchResult('alice', 'UserProfile', 'h1')] }
      })

      await renderTab()
      await waitForReady()

      fireEvent.change(screen.getByPlaceholderText('e.g. alice'), { target: { value: 'alice' } })
      fireEvent.click(screen.getByText('Add to Graph'))
      await waitFor(() => expect(screen.getByTestId('graph-node-count').textContent).toBe('3'), { timeout: 3000 })

      fireEvent.click(screen.getByText('Clear & Reload'))

      await waitFor(() => {
        expect(screen.getByTestId('graph-node-count').textContent).toBe('2')
        expect(screen.getByTestId('graph-link-count').textContent).toBe('0')
      }, { timeout: 3000 })
    })
  })

  describe('stats panel', () => {
    it('shows correct link count when one word connects to two schemas', async () => {
      nativeIndexClient.search.mockResolvedValue({
        success: true,
        data: {
          results: [
            makeSearchResult('alice', 'UserProfile', 'h1'),
            makeSearchResult('alice', 'TweetData',   'h2'),
          ]
        }
      })

      await renderTab()
      await waitForReady()

      fireEvent.change(screen.getByPlaceholderText('e.g. alice'), { target: { value: 'alice' } })
      fireEvent.click(screen.getByText('Add to Graph'))

      await waitFor(() => {
        expect(screen.getByTestId('graph-node-count').textContent).toBe('3')
        expect(screen.getByTestId('graph-link-count').textContent).toBe('2')
      }, { timeout: 3000 })
    })
  })
})
