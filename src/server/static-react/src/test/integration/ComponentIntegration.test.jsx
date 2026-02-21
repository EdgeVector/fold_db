/**
 * Integration tests for TASK-002 component extraction
 * Tests how new components work together in realistic scenarios
 * Part of TASK-002: Component Extraction and Modularization
 */

import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { Provider } from 'react-redux'
import TabNavigation from '../../components/TabNavigation'
import SelectField from '../../components/form/SelectField'
import TextField from '../../components/form/TextField'
import SchemaStatusBadge from '../../components/schema/SchemaStatusBadge'

import { renderWithRedux } from '../utils/testHelpers'
import { createAuthenticatedState, createUnauthenticatedState } from '../utils/testHelpers'

describe('Component Integration Tests', () => {
  describe('TabNavigation with Authentication', () => {
    it('integrates properly with different tabs', async () => {
      const onTabChange = vi.fn()
      const { unmount } = await renderWithRedux(
        <TabNavigation
          activeTab="ingestion"
          onTabChange={onTabChange}
        />, { initialState: createUnauthenticatedState() }
      )

      // All tabs should be enabled
      expect(screen.getByRole('button', { name: /ingestion tab/i })).toBeEnabled()
      expect(screen.getByRole('button', { name: /ai query tab/i })).toBeEnabled()

      // Unmount and re-mount with different active tab
      unmount()
      await renderWithRedux(
        <TabNavigation
          activeTab="llm-query"
          onTabChange={onTabChange}
        />, { initialState: createAuthenticatedState() }
      )

      // Tabs remain enabled
      expect(screen.getByRole('button', { name: /ingestion tab/i })).toBeEnabled()
      expect(screen.getByRole('button', { name: /ai query tab/i })).toBeEnabled()
    })
  })

  describe('Form Components Integration', () => {
    it('handles schema selection workflow', async () => {
      const user = userEvent.setup()
      const onSchemaChange = vi.fn()
      const mockSchemas = [
        { value: 'schema1', label: 'User Profile Schema' },
        { value: 'schema2', label: 'Product Catalog Schema' }
      ]

      render(
        <SelectField
          name="schema"
          label="Select Schema"
          value=""
          onChange={onSchemaChange}
          options={mockSchemas}
          placeholder="Choose a schema..."
          helpText="Only approved schemas are shown"
        />
      )

      const select = screen.getByRole('combobox')
      await user.selectOptions(select, 'schema1')
      
      expect(onSchemaChange).toHaveBeenCalledWith('schema1')
    })

    it('handles text input with validation', async () => {
      const user = userEvent.setup()
      const onChange = jest.fn()

      render(
        <TextField
          name="rangeKey"
          label="Range Key"
          value=""
          onChange={onChange}
          required={true}
          placeholder="Enter range key value"
          debounced={true}
          debounceMs={100}
        />
      )

      const input = screen.getByRole('textbox')
      await user.type(input, 'user:123')

      // Should show debouncing indicator
      expect(screen.getByRole('status')).toBeInTheDocument()

      // Should call onChange after debounce
      await waitFor(() => {
        expect(onChange).toHaveBeenCalledWith('user:123')
      }, { timeout: 200 })
    })
  })



  describe('Complete Workflow Integration', () => {
    it('simulates a complete schema selection and form workflow', async () => {
      const user = userEvent.setup()
      const onTabChange = jest.fn()
      const onSchemaChange = jest.fn()
      const onRangeKeyChange = vi.fn()

      const mockSchemas = [
        { value: 'users', label: 'User Profiles' },
        { value: 'products', label: 'Product Catalog' }
      ]

      await renderWithRedux(
        <div>
          {/* Tab Navigation */}
          <TabNavigation
            activeTab="ingestion"
            onTabChange={onTabChange}
          />

          {/* Form Components */}
          <SelectField
            name="schema"
            label="Select Schema"
            value=""
            onChange={onSchemaChange}
            options={mockSchemas}
          />

          <TextField
            name="rangeKey"
            label="Range Key"
            value=""
            onChange={onRangeKeyChange}
            required={true}
          />
        </div>,
        { initialState: createAuthenticatedState() }
      )

      // Navigate to AI Query tab
      const aiQueryTab = screen.getByRole('button', { name: /ai query tab/i })
      await user.click(aiQueryTab)
      expect(onTabChange).toHaveBeenCalledWith('llm-query')

      // Select a schema
      const schemaSelect = screen.getByRole('combobox')
      await user.selectOptions(schemaSelect, 'users')
      expect(onSchemaChange).toHaveBeenCalledWith('users')

      // Enter range key
      const rangeKeyInput = screen.getByRole('textbox')
      await user.type(rangeKeyInput, 'user:john')
      expect(onRangeKeyChange).toHaveBeenCalledWith('user:john')
    })
  })

  describe('Error Handling Integration', () => {
    it('displays validation errors across components', () => {
      render(
        <div>
          <TextField
            name="field1"
            label="Required Field"
            value=""
            onChange={vi.fn()}
            required={true}
            error="This field is required"
          />
          
          <SelectField
            name="field2"
            label="Schema Selection"
            value=""
            onChange={jest.fn()}
            options={[]}
            config={{ emptyMessage: "No schemas available" }}
          />
        </div>
      )

      // Should show text field error
      expect(screen.getByRole('alert')).toHaveTextContent('This field is required')
      
      // Should show empty state for select
      expect(screen.getByText('No schemas available')).toBeInTheDocument()
    })
  })
})