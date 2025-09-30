/**
 * Integration tests for TASK-002 component extraction
 * Tests how new components work together in realistic scenarios
 * Part of TASK-002: Component Extraction and Modularization
 */

import { render, screen, fireEvent, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { Provider } from 'react-redux'
import TabNavigation from '../../components/TabNavigation'
import SelectField from '../../components/form/SelectField'
import TextField from '../../components/form/TextField'
import SchemaStatusBadge from '../../components/schema/SchemaStatusBadge'

import { renderWithRedux } from '../utils/testHelpers'
import { createAuthenticatedState, createUnauthenticatedState } from '../utils/testHelpers'
import { createTestStore } from '../utils/testUtilities.jsx'

describe('Component Integration Tests', () => {
  describe('TabNavigation with Authentication', () => {
    it('integrates properly with authentication state changes', async () => {
      const onTabChange = vi.fn()
      const { unmount } = await renderWithRedux(
        <TabNavigation
          activeTab="keys"
          onTabChange={onTabChange}
        />, { initialState: createUnauthenticatedState() }
      )

      // Tabs are not locked when unauthenticated (public UI)
      const authRequiredTabs = [
        { id: 'admin', label: 'Admin', requiresAuth: true, icon: '👑' }
      ]
      
      await renderWithRedux(
        <TabNavigation
          tabs={authRequiredTabs}
          activeTab="admin"
          onTabChange={onTabChange}
        />, { initialState: createUnauthenticatedState() }
      )
      
      expect(screen.getByRole('button', { name: /admin tab/i })).toBeEnabled()

      // Unmount and re-mount with authenticated state
      unmount()
      await renderWithRedux(
        <TabNavigation
          activeTab="keys"
          onTabChange={onTabChange}
        />, { initialState: createAuthenticatedState() }
      )

      // Tabs remain enabled when authenticated
      expect(screen.getByRole('button', { name: /schemas tab/i })).toBeEnabled()
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
    it('simulates a complete schema selection and mutation workflow', async () => {
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
            activeTab="mutation"
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

      // Navigate to mutation tab
      const mutationTab = screen.getByRole('button', { name: /mutation tab/i })
      await user.click(mutationTab)
      expect(onTabChange).toHaveBeenCalledWith('mutation')

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