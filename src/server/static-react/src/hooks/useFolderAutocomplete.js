import { useState, useEffect, useRef, useCallback } from 'react'
import { ingestionClient } from '../api/clients'

/**
 * Manages folder path autocomplete: debounced completions, keyboard nav,
 * click-outside dismiss, and suggestion acceptance.
 *
 * @param {Object} opts
 * @param {string} opts.folderPath - Current input value
 * @param {boolean} opts.isDisabled - Suppress completions (e.g. while scanning)
 * @param {Function} opts.onFolderPathChange - Setter for folder path
 * @param {Function} opts.onSubmit - Called on bare Enter (no suggestion selected)
 */
export function useFolderAutocomplete({ folderPath, isDisabled, onFolderPathChange, onSubmit }) {
  const [suggestions, setSuggestions] = useState([])
  const [selectedIndex, setSelectedIndex] = useState(-1)
  const [showSuggestions, setShowSuggestions] = useState(false)
  const inputRef = useRef(null)
  const suggestionsRef = useRef(null)
  const debounceRef = useRef(null)

  const fetchCompletions = useCallback(async (path) => {
    if (!path.includes('/')) {
      setSuggestions([])
      setShowSuggestions(false)
      return
    }
    try {
      const response = await ingestionClient.completePath(path)
      if (response.success && response.data?.completions) {
        setSuggestions(response.data.completions)
        setSelectedIndex(-1)
        setShowSuggestions(response.data.completions.length > 0)
      } else {
        setSuggestions([])
        setShowSuggestions(false)
      }
    } catch { /* autocomplete is best-effort */
      setSuggestions([])
      setShowSuggestions(false)
    }
  }, [])

  // Debounced fetch on path change
  useEffect(() => {
    if (debounceRef.current) clearTimeout(debounceRef.current)
    if (!folderPath.includes('/') || isDisabled) {
      setSuggestions([])
      setShowSuggestions(false)
      return
    }
    debounceRef.current = setTimeout(() => fetchCompletions(folderPath), 200)
    return () => { if (debounceRef.current) clearTimeout(debounceRef.current) }
  }, [folderPath, isDisabled, fetchCompletions])

  // Close suggestions when clicking outside
  useEffect(() => {
    const handleClickOutside = (e) => {
      if (
        inputRef.current && !inputRef.current.contains(e.target) &&
        suggestionsRef.current && !suggestionsRef.current.contains(e.target)
      ) {
        setShowSuggestions(false)
      }
    }
    document.addEventListener('mousedown', handleClickOutside)
    return () => document.removeEventListener('mousedown', handleClickOutside)
  }, [])

  const acceptSuggestion = useCallback((path) => {
    const newPath = path.endsWith('/') ? path : path + '/'
    onFolderPathChange(newPath)
    setShowSuggestions(false)
    setSelectedIndex(-1)
    inputRef.current?.focus()
  }, [onFolderPathChange])

  const handleInputKeyDown = useCallback((e) => {
    if (showSuggestions && suggestions.length > 0) {
      if (e.key === 'ArrowDown') {
        e.preventDefault()
        setSelectedIndex((prev) => (prev < suggestions.length - 1 ? prev + 1 : 0))
        return
      }
      if (e.key === 'ArrowUp') {
        e.preventDefault()
        setSelectedIndex((prev) => (prev > 0 ? prev - 1 : suggestions.length - 1))
        return
      }
      if (e.key === 'Tab') {
        e.preventDefault()
        const idx = selectedIndex >= 0 ? selectedIndex : 0
        acceptSuggestion(suggestions[idx])
        return
      }
      if (e.key === 'Enter') {
        if (selectedIndex >= 0) {
          e.preventDefault()
          acceptSuggestion(suggestions[selectedIndex])
          return
        }
      }
      if (e.key === 'Escape') {
        setShowSuggestions(false)
        setSelectedIndex(-1)
        return
      }
    }
    if (e.key === 'Enter') onSubmit()
  }, [showSuggestions, suggestions, selectedIndex, acceptSuggestion, onSubmit])

  return {
    suggestions,
    selectedIndex,
    showSuggestions,
    setShowSuggestions,
    setSelectedIndex,
    acceptSuggestion,
    handleInputKeyDown,
    inputRef,
    suggestionsRef,
  }
}
