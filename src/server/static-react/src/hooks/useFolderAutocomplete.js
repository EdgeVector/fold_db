import { useState, useEffect, useRef, useCallback } from 'react'
import { ingestionClient } from '../api/clients'

/**
 * Returns the longest common prefix of an array of strings.
 */
function longestCommonPrefix(strings) {
  if (strings.length === 0) return ''
  let prefix = strings[0]
  for (let i = 1; i < strings.length; i++) {
    while (strings[i].indexOf(prefix) !== 0) {
      prefix = prefix.slice(0, -1)
      if (prefix === '') return ''
    }
  }
  return prefix
}

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
  const folderPathRef = useRef(folderPath)

  // Keep ref in sync so async handlers always see the latest value
  useEffect(() => { folderPathRef.current = folderPath }, [folderPath])

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

  const handleInputKeyDown = useCallback(async (e) => {
    // Shell-like Tab: intercept before checking if suggestions are visible
    if (e.key === 'Tab' && folderPathRef.current.includes('/') && !isDisabled) {
      e.preventDefault()

      // If suggestions are already loaded, accept from current list
      if (showSuggestions && suggestions.length > 0) {
        const idx = selectedIndex >= 0 ? selectedIndex : 0
        acceptSuggestion(suggestions[idx])
        return
      }

      // No suggestions yet — fetch immediately (bypass debounce)
      if (debounceRef.current) clearTimeout(debounceRef.current)
      try {
        const response = await ingestionClient.completePath(folderPathRef.current)
        if (response.success && response.data?.completions?.length) {
          const completions = response.data.completions
          if (completions.length === 1) {
            acceptSuggestion(completions[0])
          } else {
            const prefix = longestCommonPrefix(completions)
            if (prefix.length > folderPathRef.current.length) {
              onFolderPathChange(prefix)
            }
            setSuggestions(completions)
            setSelectedIndex(-1)
            setShowSuggestions(true)
          }
        }
      } catch { /* tab-complete is best-effort */ }
      return
    }

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
  }, [showSuggestions, suggestions, selectedIndex, acceptSuggestion, onSubmit, isDisabled, onFolderPathChange])

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
