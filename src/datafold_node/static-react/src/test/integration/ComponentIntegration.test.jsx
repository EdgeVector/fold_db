/**
 * Integration tests for TASK-002 component extraction
 * Tests how new components work together in realistic scenarios
 * Part of TASK-002: Component Extraction and Modularization
 */

import { render, screen, fireEvent, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { Provider } from 'react-redux'
import { configureStore } from '@reduxjs/toolkit'
import TabNavigation from '../../components/TabNavigation'
import SelectField from '../../components/form/SelectField'
import TextField from '../../components/form/TextField'
import SchemaStatusBadge from '../../components/schema/SchemaStatusBadge'
import SchemaActions from '../../components/schema/SchemaActions'

// Mock store for testing
const createMockStore = () => configureStore({
  reducer: {
    auth: (state = { isAuthenticated: true }) => state
  }
})

describe('Component Integration Tests', () => {
  describe('TabNavigation with Authentication', () => {
    it('integrates properly with authentication state changes', () => {
      const onTabChange = jest.fn()
      const { rerender } = render(
        <TabNavigation
          activeTab="keys"
          isAuthenticated={false}
          onTabChange={onTabChange}
        />
      )

      // Should show locked tabs when not authenticated
      expect(screen.getByRole('button', { name: /schemas tab.*authentication required/i })).toBeDisabled()

      // Re-render with authentication
      rerender(
        <TabNavigation
          activeTab="keys"
          isAuthenticated={true}
          onTabChange={onTabChange}
        />
      )

      // Should now enable previously locked tabs
      expect(screen.getByRole('button', { name: /schemas tab/i })).toBeEnabled()
    })
  })

  describe('Form Components Integration', () => {
    it('handles schema selection workflow', async () => {
      const user = userEvent.setup()
      const onSchemaChange = jest.fn()
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

  describe('Schema Components Integration', () => {
    it('displays schema information with status and actions', () => {
      const mockSchema = {
        name: 'UserProfile',
        state: 'approved',
        fields: { id: { field_type: 'String' }, name: { field_type: 'String' } }
      }

      const onApprove = jest.fn()
      const onBlock = jest.fn()
      const onUnload = jest.fn()

      render(
        <div>
          <SchemaStatusBadge 
            state="approved" 
            isRangeSchema={true}
            showTooltip={true} 
          />
          <SchemaActions
            schema={mockSchema}
            onApprove={onApprove}
            onBlock={onBlock}
            onUnload={onUnload}
          />
        </div>
      )

      // Should show schema status
      expect(screen.getByText('Approved')).toBeInTheDocument()
      expect(screen.getByText('Range Key')).toBeInTheDocument()

      // Should show appropriate actions for approved schema
      expect(screen.getByRole('button', { name: /block schema/i })).toBeInTheDocument()
      expect(screen.getByRole('button', { name: /unload schema/i })).toBeInTheDocument()
      expect(screen.queryByRole('button', { name: /approve schema/i })).not.toBeInTheDocument()
    })

    it('handles schema action workflow with confirmation', async () => {
      const user = userEvent.setup()
      const mockSchema = {
        name: 'TestSchema',
        state: 'approved'
      }
      const onBlock = jest.fn()

      render(
        <SchemaActions
          schema={mockSchema}
          onApprove={jest.fn()}
          onBlock={onBlock}
          onUnload={jest.fn()}
          showConfirmation={true}
        />
      )

      // Click block button
      const blockButton = screen.getByRole('button', { name: /block schema/i })
      await user.click(blockButton)

      // Should show confirmation dialog
      expect(screen.getByText('Confirm Action')).toBeInTheDocument()
      expect(screen.getByText(/are you sure you want to block schema/i)).toBeInTheDocument()

      // Confirm the action
      const confirmButton = screen.getByRole('button', { name: /confirm/i })
      await user.click(confirmButton)

      expect(onBlock).toHaveBeenCalledWith('TestSchema')
    })
  })

  describe('Complete Workflow Integration', () => {
    it('simulates a complete schema selection and mutation workflow', async () => {
      const user = userEvent.setup()
      const onTabChange = jest.fn()
      const onSchemaChange = jest.fn()
      const onRangeKeyChange = jest.fn()

      const mockSchemas = [
        { value: 'users', label: 'User Profiles' },
        { value: 'products', label: 'Product Catalog' }
      ]

      render(
        <div>
          {/* Tab Navigation */}
          <TabNavigation
            activeTab="mutation"
            isAuthenticated={true}
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
        </div>
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
            onChange={jest.fn()}
            required={true}
            error="This field is required"
          />
          
          <SelectField
            name="field2"
            label="Schema Selection"
            value=""
            onChange={jest.fn()}
            options={[]}
            emptyMessage="No schemas available"
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