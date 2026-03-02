import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import { renderHook, act } from '@testing-library/react'
import { useFolderAutocomplete } from '../../hooks/useFolderAutocomplete'

vi.mock('../../api/clients', () => ({
  ingestionClient: { completePath: vi.fn() }
}))

import { ingestionClient } from '../../api/clients'

describe('useFolderAutocomplete', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    vi.useFakeTimers()
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  it('calls completePath and shows suggestions when path contains /', async () => {
    ingestionClient.completePath.mockResolvedValue({
      success: true,
      data: { completions: ['/home/user/docs', '/home/user/downloads'] }
    })

    const { result } = renderHook(() => useFolderAutocomplete({
      folderPath: '/home/user',
      isDisabled: false,
      onFolderPathChange: vi.fn(),
      onSubmit: vi.fn(),
    }))

    await act(async () => {
      await vi.runAllTimersAsync()
    })

    expect(ingestionClient.completePath).toHaveBeenCalledWith('/home/user')
    expect(result.current.suggestions).toEqual(['/home/user/docs', '/home/user/downloads'])
    expect(result.current.showSuggestions).toBe(true)
  })

  it('does NOT call completePath when path has no /', async () => {
    const { result } = renderHook(() => useFolderAutocomplete({
      folderPath: 'nodash',
      isDisabled: false,
      onFolderPathChange: vi.fn(),
      onSubmit: vi.fn(),
    }))

    await act(async () => {
      await vi.runAllTimersAsync()
    })

    expect(ingestionClient.completePath).not.toHaveBeenCalled()
    expect(result.current.suggestions).toEqual([])
    expect(result.current.showSuggestions).toBe(false)
  })

  it('shows no suggestions when API returns empty completions array', async () => {
    ingestionClient.completePath.mockResolvedValue({
      success: true,
      data: { completions: [] }
    })

    const { result } = renderHook(() => useFolderAutocomplete({
      folderPath: '/home/user',
      isDisabled: false,
      onFolderPathChange: vi.fn(),
      onSubmit: vi.fn(),
    }))

    await act(async () => {
      await vi.runAllTimersAsync()
    })

    expect(ingestionClient.completePath).toHaveBeenCalledWith('/home/user')
    expect(result.current.suggestions).toEqual([])
    expect(result.current.showSuggestions).toBe(false)
  })
})
