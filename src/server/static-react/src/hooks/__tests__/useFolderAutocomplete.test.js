import { renderHook, act } from '@testing-library/react'
import { describe, it, expect, beforeEach, vi } from 'vitest'

// Mock the ingestionClient before importing the hook
vi.mock('../../api/clients', () => ({
  ingestionClient: {
    completePath: vi.fn(),
  },
}))

import { useFolderAutocomplete } from '../useFolderAutocomplete.js'
import { ingestionClient } from '../../api/clients'

describe('useFolderAutocomplete', () => {
  let onFolderPathChange
  let onSubmit

  beforeEach(() => {
    vi.clearAllMocks()
    vi.useFakeTimers()
    onFolderPathChange = vi.fn()
    onSubmit = vi.fn()
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  function renderAutocomplete(folderPath = '', isDisabled = false) {
    return renderHook(
      ({ folderPath, isDisabled }) =>
        useFolderAutocomplete({ folderPath, isDisabled, onFolderPathChange, onSubmit }),
      { initialProps: { folderPath, isDisabled } }
    )
  }

  describe('debounced completions', () => {
    it('fetches completions after debounce when path includes /', async () => {
      ingestionClient.completePath.mockResolvedValue({
        success: true,
        data: { completions: ['/usr', '/Users'] },
      })

      const { rerender } = renderAutocomplete('/u')

      // Completions should not fire yet
      expect(ingestionClient.completePath).not.toHaveBeenCalled()

      // Advance past debounce
      await act(async () => { vi.advanceTimersByTime(250) })

      expect(ingestionClient.completePath).toHaveBeenCalledWith('/u')
    })

    it('does not fetch when path has no slash', async () => {
      renderAutocomplete('Documents')
      await act(async () => { vi.advanceTimersByTime(250) })
      expect(ingestionClient.completePath).not.toHaveBeenCalled()
    })

    it('does not fetch when disabled', async () => {
      renderAutocomplete('/usr', true)
      await act(async () => { vi.advanceTimersByTime(250) })
      expect(ingestionClient.completePath).not.toHaveBeenCalled()
    })
  })

  describe('Tab key handling', () => {
    it('calls preventDefault on Tab when path has a slash', async () => {
      ingestionClient.completePath.mockResolvedValue({
        success: true,
        data: { completions: ['/usr'] },
      })

      const { result } = renderAutocomplete('/u')

      const event = { key: 'Tab', preventDefault: vi.fn() }
      await act(async () => { result.current.handleInputKeyDown(event) })

      expect(event.preventDefault).toHaveBeenCalled()
    })

    it('auto-completes to single match on Tab', async () => {
      ingestionClient.completePath.mockResolvedValue({
        success: true,
        data: { completions: ['/usr'] },
      })

      const { result } = renderAutocomplete('/u')

      const event = { key: 'Tab', preventDefault: vi.fn() }
      await act(async () => { result.current.handleInputKeyDown(event) })

      // Single match → acceptSuggestion → onFolderPathChange with trailing /
      expect(onFolderPathChange).toHaveBeenCalledWith('/usr/')
    })

    it('completes to longest common prefix with multiple matches', async () => {
      ingestionClient.completePath.mockResolvedValue({
        success: true,
        data: { completions: ['/usr/local', '/usr/lib'] },
      })

      const { result } = renderAutocomplete('/us')

      const event = { key: 'Tab', preventDefault: vi.fn() }
      await act(async () => { result.current.handleInputKeyDown(event) })

      // LCP of /usr/local and /usr/lib is /usr/l — longer than /us so path updates
      expect(onFolderPathChange).toHaveBeenCalledWith('/usr/l')
      // Suggestions should be shown
      expect(result.current.showSuggestions).toBe(true)
      expect(result.current.suggestions).toEqual(['/usr/local', '/usr/lib'])
    })

    it('shows suggestions without changing path when LCP equals input', async () => {
      ingestionClient.completePath.mockResolvedValue({
        success: true,
        data: { completions: ['/Applications', '/Library'] },
      })

      const { result } = renderAutocomplete('/')

      const event = { key: 'Tab', preventDefault: vi.fn() }
      await act(async () => { result.current.handleInputKeyDown(event) })

      // LCP is / which equals current path, so path is NOT changed
      // but onFolderPathChange is not called for the prefix
      // suggestions should still be shown
      expect(result.current.showSuggestions).toBe(true)
      expect(result.current.suggestions).toEqual(['/Applications', '/Library'])
    })

    it('does not intercept Tab when path has no slash', async () => {
      const { result } = renderAutocomplete('Documents')

      const event = { key: 'Tab', preventDefault: vi.fn() }
      await act(async () => { result.current.handleInputKeyDown(event) })

      expect(event.preventDefault).not.toHaveBeenCalled()
      expect(ingestionClient.completePath).not.toHaveBeenCalled()
    })
  })

  describe('keyboard navigation', () => {
    it('calls onSubmit on Enter with no suggestions', async () => {
      const { result } = renderAutocomplete('/tmp')

      const event = { key: 'Enter', preventDefault: vi.fn() }
      await act(async () => { result.current.handleInputKeyDown(event) })

      expect(onSubmit).toHaveBeenCalled()
    })

    it('dismisses suggestions on Escape', async () => {
      ingestionClient.completePath.mockResolvedValue({
        success: true,
        data: { completions: ['/usr', '/Users'] },
      })

      const { result } = renderAutocomplete('/u')
      // Trigger debounce to load suggestions
      await act(async () => { vi.advanceTimersByTime(250) })

      expect(result.current.showSuggestions).toBe(true)

      const event = { key: 'Escape', preventDefault: vi.fn() }
      await act(async () => { result.current.handleInputKeyDown(event) })

      expect(result.current.showSuggestions).toBe(false)
    })
  })

  describe('acceptSuggestion', () => {
    it('appends trailing slash to accepted path', () => {
      const { result } = renderAutocomplete()

      act(() => { result.current.acceptSuggestion('/usr/local') })

      expect(onFolderPathChange).toHaveBeenCalledWith('/usr/local/')
    })

    it('does not double trailing slash', () => {
      const { result } = renderAutocomplete()

      act(() => { result.current.acceptSuggestion('/usr/local/') })

      expect(onFolderPathChange).toHaveBeenCalledWith('/usr/local/')
    })
  })
})
