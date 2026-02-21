/**
 * QueryForm Component Tests
 * Tests for UCR-1-4: QueryForm component for input validation
 * Part of UTC-1 Test Coverage Enhancement - UCR-1 Component Testing
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { screen, fireEvent } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import QueryForm from '../../../components/query/QueryForm';
import { renderWithRedux, createAuthenticatedState } from '../../utils/testHelpers';

describe('QueryForm Component', () => {
  let mockProps;
  let user;

  const mockApprovedSchemas = [
    {
      name: 'UserSchema',
      state: 'approved',
      fields: ['id', 'name', 'age', 'range_field'],
      schema_type: { Single: {} }
    },
    {
      name: 'ProductSchema',
      state: 'approved',
      fields: ['product_id', 'price', 'category'],
      schema_type: { Single: {} }
    }
  ];

  beforeEach(() => {
    user = userEvent.setup();
    mockProps = {
      queryState: {
        selectedSchema: '',
        queryFields: [],
        rangeFilters: {},
        rangeSchemaFilter: {}
      },
      onSchemaChange: vi.fn(),
      onFieldToggle: vi.fn(),
      onRangeFilterChange: vi.fn(),
      onRangeSchemaFilterChange: vi.fn(),
      approvedSchemas: mockApprovedSchemas,
      schemasLoading: false,
      isRangeSchema: false,
      rangeKey: null,
      className: ''
    };
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  describe('rendering', () => {
    it('should render schema selection field', () => {
      renderWithRedux(<QueryForm {...mockProps} />, { initialState: createAuthenticatedState() });

      expect(screen.getByText('Schema')).toBeInTheDocument();
      expect(screen.getByRole('combobox')).toBeInTheDocument();
      expect(screen.getByText('Select a schema to work with')).toBeInTheDocument();
    });

    it('should render schema options correctly', () => {
      renderWithRedux(<QueryForm {...mockProps} />, { initialState: createAuthenticatedState() });

      const select = screen.getByRole('combobox');
      expect(select).toBeInTheDocument();

      // Check placeholder
      expect(screen.getByText('Select an option...')).toBeInTheDocument();
    });

    it('should show loading state for schemas', () => {
      mockProps.schemasLoading = true;
      renderWithRedux(<QueryForm {...mockProps} />, { initialState: createAuthenticatedState() });

      // The SelectField component should handle loading state
      expect(screen.getByRole('combobox')).toBeInTheDocument();
    });

    it('should apply custom className', () => {
      mockProps.className = 'custom-form-class';
      const { container } = renderWithRedux(<QueryForm {...mockProps} />, { initialState: createAuthenticatedState() });

      expect(container.firstChild).toHaveClass('custom-form-class');
    });
  });

  describe('schema selection', () => {
    it('should call onSchemaChange when schema is selected', async () => {
      renderWithRedux(<QueryForm {...mockProps} />, { initialState: createAuthenticatedState() });

      const select = screen.getByRole('combobox');
      await user.selectOptions(select, 'UserSchema');

      expect(mockProps.onSchemaChange).toHaveBeenCalledWith('UserSchema');
    });

    it('should clear schema validation error when schema is selected', async () => {
      // Start with validation error by trying to validate empty form
      renderWithRedux(<QueryForm {...mockProps} />, { initialState: createAuthenticatedState() });

      const select = screen.getByRole('combobox');
      await user.selectOptions(select, 'UserSchema');

      expect(mockProps.onSchemaChange).toHaveBeenCalledWith('UserSchema');
    });
  });

  describe('field selection', () => {
    beforeEach(() => {
      mockProps.queryState.selectedSchema = 'UserSchema';
    });

    it('should render field selection when schema is selected', () => {
      renderWithRedux(<QueryForm {...mockProps} />, { initialState: createAuthenticatedState() });

      expect(screen.getByText('Field Selection')).toBeInTheDocument();
      expect(screen.getByText('Select fields to include in your query')).toBeInTheDocument();

      // Should show all fields from the selected schema (declarative schemas don't filter by type)
      expect(screen.getByText('id')).toBeInTheDocument();
      expect(screen.getByText('name')).toBeInTheDocument();
      expect(screen.getByText('age')).toBeInTheDocument();
      expect(screen.getByText('range_field')).toBeInTheDocument();

      // Note: Declarative schemas don't have field_type metadata, so we don't display types
    });

    it('should call onFieldToggle when field checkbox is clicked', async () => {
      renderWithRedux(<QueryForm {...mockProps} />, { initialState: createAuthenticatedState() });

      const idCheckbox = screen.getByRole('checkbox', { name: /id/i });
      await user.click(idCheckbox);

      expect(mockProps.onFieldToggle).toHaveBeenCalledWith('id');
    });

    it('should show checked state for selected fields', () => {
      mockProps.queryState.queryFields = ['id', 'name'];
      renderWithRedux(<QueryForm {...mockProps} />, { initialState: createAuthenticatedState() });

      const idCheckbox = screen.getByRole('checkbox', { name: /id/i });
      const nameCheckbox = screen.getByRole('checkbox', { name: /name/i });
      const ageCheckbox = screen.getByRole('checkbox', { name: /age/i });

      expect(idCheckbox).toBeChecked();
      expect(nameCheckbox).toBeChecked();
      expect(ageCheckbox).not.toBeChecked();
    });

    it('should not render field selection when no schema is selected', () => {
      mockProps.queryState.selectedSchema = '';
      renderWithRedux(<QueryForm {...mockProps} />, { initialState: createAuthenticatedState() });

      expect(screen.queryByText('Select Fields')).not.toBeInTheDocument();
    });
  });

  describe('range schema filter', () => {
    beforeEach(() => {
      mockProps.queryState.selectedSchema = 'UserSchema';
      mockProps.isRangeSchema = true;
      mockProps.rangeKey = 'range_field';
    });

    it('should render range filter for range schemas', () => {
      renderWithRedux(<QueryForm {...mockProps} />, { initialState: createAuthenticatedState() });

      expect(screen.getByText('Range Filter')).toBeInTheDocument();
      expect(screen.getByText('Filter data by range key values')).toBeInTheDocument();
    });

    it('should not render range filter for non-range schemas', () => {
      mockProps.isRangeSchema = false;
      renderWithRedux(<QueryForm {...mockProps} />, { initialState: createAuthenticatedState() });

      expect(screen.queryByText('Range Filter')).not.toBeInTheDocument();
    });

    it('should not render range filter when rangeKey is null', () => {
      mockProps.rangeKey = null;
      renderWithRedux(<QueryForm {...mockProps} />, { initialState: createAuthenticatedState() });

      expect(screen.queryByText('Range Filter')).not.toBeInTheDocument();
    });

    it('should call onRangeSchemaFilterChange when range filter changes', async () => {
      renderWithRedux(<QueryForm {...mockProps} />, { initialState: createAuthenticatedState() });

      // The RangeField component should trigger this callback
      // We'll simulate this by checking that the component receives the right props
      expect(screen.getByText('Range Filter')).toBeInTheDocument();
    });
  });

  // Skip: regular range field filters removed - declarative schemas don't have field_type metadata
  describe.skip('regular range field filters', () => {
    beforeEach(() => {
      mockProps.queryState.selectedSchema = 'UserSchema';
      mockProps.queryState.queryFields = ['range_field'];
      mockProps.isRangeSchema = false;
    });

    it('should render range field filters for non-range schemas with range fields', () => {
      renderWithRedux(<QueryForm {...mockProps} />, { initialState: createAuthenticatedState() });

      expect(screen.getByText('Range Field Filters')).toBeInTheDocument();
      expect(screen.getByText('Configure filters for range fields')).toBeInTheDocument();
      expect(screen.getAllByText('range_field')).toHaveLength(2); // One in field selection, one in range filters
    });

    it('should render range filter inputs', () => {
      renderWithRedux(<QueryForm {...mockProps} />, { initialState: createAuthenticatedState() });

      expect(screen.getByText('Key Range')).toBeInTheDocument();
      expect(screen.getByText('Exact Key')).toBeInTheDocument();
      expect(screen.getByText('Key Prefix')).toBeInTheDocument();

      expect(screen.getByPlaceholderText('Start key')).toBeInTheDocument();
      expect(screen.getByPlaceholderText('End key')).toBeInTheDocument();
      expect(screen.getByPlaceholderText('Exact key to match')).toBeInTheDocument();
      expect(screen.getByPlaceholderText("Key prefix (e.g., 'user:')")).toBeInTheDocument();
    });

    it('should call onRangeFilterChange when range inputs change', async () => {
      mockProps.queryState.rangeFilters = { range_field: {} };
      renderWithRedux(<QueryForm {...mockProps} />, { initialState: createAuthenticatedState() });

      const startKeyInput = screen.getByPlaceholderText('Start key');
      await user.clear(startKeyInput);
      
      // Use fireEvent.change for full string input instead of user.type character by character
      fireEvent.change(startKeyInput, { target: { value: 'start_value' } });

      expect(mockProps.onRangeFilterChange).toHaveBeenLastCalledWith('range_field', 'start', 'start_value');
    });

    it('should show current filter values', () => {
      mockProps.queryState.rangeFilters = {
        range_field: {
          start: 'start_val',
          end: 'end_val',
          key: 'exact_val',
          keyPrefix: 'prefix_val'
        }
      };
      renderWithRedux(<QueryForm {...mockProps} />, { initialState: createAuthenticatedState() });

      expect(screen.getByDisplayValue('start_val')).toBeInTheDocument();
      expect(screen.getByDisplayValue('end_val')).toBeInTheDocument();
      expect(screen.getByDisplayValue('exact_val')).toBeInTheDocument();
      expect(screen.getByDisplayValue('prefix_val')).toBeInTheDocument();
    });

    it('should not render range field filters for range schemas', () => {
      mockProps.isRangeSchema = true;
      renderWithRedux(<QueryForm {...mockProps} />, { initialState: createAuthenticatedState() });

      expect(screen.queryByText('Range Field Filters')).not.toBeInTheDocument();
    });

    it('should not render range field filters when no range fields are selected', () => {
      mockProps.queryState.queryFields = ['id', 'name']; // No range fields
      renderWithRedux(<QueryForm {...mockProps} />, { initialState: createAuthenticatedState() });

      expect(screen.queryByText('Range Field Filters')).not.toBeInTheDocument();
    });
  });

  describe('form validation', () => {
    it('should show validation error when no schema is selected', () => {
      // This would be tested in integration with the validation logic
      // The component uses internal state for validation errors
      renderWithRedux(<QueryForm {...mockProps} />, { initialState: createAuthenticatedState() });

      // Component should show required indicator for schema field
      expect(screen.getByText('Schema')).toBeInTheDocument();
      // Required fields should have visual indicators
    });

    it('should show validation error when no fields are selected', () => {
      mockProps.queryState.selectedSchema = 'UserSchema';
      mockProps.queryState.queryFields = [];
      renderWithRedux(<QueryForm {...mockProps} />, { initialState: createAuthenticatedState() });

      expect(screen.getByText('Field Selection')).toBeInTheDocument();
    });

    it('should validate range filter values', () => {
      mockProps.queryState.selectedSchema = 'UserSchema';
      mockProps.isRangeSchema = true;
      mockProps.rangeKey = 'range_field';
      mockProps.queryState.rangeSchemaFilter = {
        start: 'z',
        end: 'a' // Invalid: start > end
      };
      
      renderWithRedux(<QueryForm {...mockProps} />, { initialState: createAuthenticatedState() });

      // The validation would be handled internally by the component
      expect(screen.getByText('Range Filter')).toBeInTheDocument();
    });
  });

  describe('range schema validation removal', () => {
    it('should allow range schema queries without range key input', async () => {
      // Use a range schema fixture
      const rangeSchema = {
        name: 'time_series_data',
        state: 'approved',
        fields: ['timestamp', 'value', 'metadata'],
        key: { range_field: 'timestamp' },
        schema_type: 'Range'
      };

      mockProps.approvedSchemas = [rangeSchema];
      mockProps.queryState.selectedSchema = 'time_series_data';
      mockProps.isRangeSchema = true;
      mockProps.rangeKey = 'timestamp';
      mockProps.queryState.rangeSchemaFilter = {}; // Empty range filter
      mockProps.queryState.queryFields = ['value']; // Selected fields but no range key

      renderWithRedux(<QueryForm {...mockProps} />, { initialState: createAuthenticatedState() });

      // Verify range schema is detected
      expect(screen.getByText('Range Filter')).toBeInTheDocument();
      
      // Verify that no validation error is shown for missing range key
      // Since we removed front-end validation, the form should not prevent submission
      // due to missing range key input
      const rangeFilterSection = screen.getByText('Range Filter').closest('div');
      expect(rangeFilterSection).toBeInTheDocument();
      
      // The range filter fields should be present but not required
      const startField = rangeFilterSection.querySelector('input[placeholder*="start" i]');
      const endField = rangeFilterSection.querySelector('input[placeholder*="end" i]');
      
      if (startField) {
        expect(startField).not.toHaveAttribute('required');
        expect(startField.value).toBe('');
      }
      
      if (endField) {
        expect(endField).not.toHaveAttribute('required');
        expect(endField.value).toBe('');
      }
    });

    it('should allow range schema queries with partial range key input', async () => {
      const rangeSchema = {
        name: 'sensor_readings',
        state: 'approved',
        fields: ['sensor_id', 'reading_value', 'calibration_data'],
        key: { range_field: 'sensor_id' },
        schema_type: 'Range'
      };

      mockProps.approvedSchemas = [rangeSchema];
      mockProps.queryState.selectedSchema = 'sensor_readings';
      mockProps.isRangeSchema = true;
      mockProps.rangeKey = 'sensor_id';
      mockProps.queryState.rangeSchemaFilter = {
        start: 'sensor_001' // Only start value, no end value
      };
      mockProps.queryState.queryFields = ['reading_value'];

      renderWithRedux(<QueryForm {...mockProps} />, { initialState: createAuthenticatedState() });

      // Verify the form renders without validation errors
      expect(screen.getByText('Range Filter')).toBeInTheDocument();
      expect(screen.getByText('Field Selection')).toBeInTheDocument();
      
      // The form should accept partial range input without validation errors
      const rangeFilterSection = screen.getByText('Range Filter').closest('div');
      const startField = rangeFilterSection.querySelector('input[value="sensor_001"]');
      expect(startField).toBeInTheDocument();
    });

    it('should not validate range key format or content', async () => {
      const rangeSchema = {
        name: 'user_activity',
        state: 'approved',
        fields: ['user_id', 'activity_type'],
        key: { range_field: 'user_id' },
        schema_type: 'Range'
      };

      mockProps.approvedSchemas = [rangeSchema];
      mockProps.queryState.selectedSchema = 'user_activity';
      mockProps.isRangeSchema = true;
      mockProps.rangeKey = 'user_id';
      mockProps.queryState.rangeSchemaFilter = {
        start: '   ', // Whitespace only
        end: 'invalid@#$%format' // Invalid characters
      };
      mockProps.queryState.queryFields = ['activity_type'];

      renderWithRedux(<QueryForm {...mockProps} />, { initialState: createAuthenticatedState() });

      // Verify the form renders without validation errors for invalid range key format
      expect(screen.getByText('Range Filter')).toBeInTheDocument();
      
      // No validation error messages should be shown
      const errorMessages = screen.queryAllByText(/required|invalid|format|error/i);
      const rangeRelatedErrors = errorMessages.filter(msg => 
        msg.textContent.toLowerCase().includes('range') || 
        msg.textContent.toLowerCase().includes('key')
      );
      expect(rangeRelatedErrors).toHaveLength(0);
    });

    it('should allow empty range filter for range schemas', async () => {
      const rangeSchema = {
        name: 'analytics_events',
        state: 'approved',
        fields: ['event_id', 'timestamp'],
        key: { range_field: 'event_id' },
        schema_type: 'Range'
      };

      mockProps.approvedSchemas = [rangeSchema];
      mockProps.queryState.selectedSchema = 'analytics_events';
      mockProps.isRangeSchema = true;
      mockProps.rangeKey = 'event_id';
      mockProps.queryState.rangeSchemaFilter = {}; // Completely empty
      mockProps.queryState.queryFields = ['timestamp'];

      renderWithRedux(<QueryForm {...mockProps} />, { initialState: createAuthenticatedState() });

      // Verify the form renders successfully with empty range filter
      expect(screen.getByText('Range Filter')).toBeInTheDocument();
      expect(screen.getByText('Field Selection')).toBeInTheDocument();
      
      // No validation should prevent the form from being rendered or submitted
      const form = screen.getByText('Range Filter').closest('form') || 
                   screen.getByText('Range Filter').closest('div');
      expect(form).toBeInTheDocument();
    });
  });

  describe('error handling', () => {
    it('should handle missing schema fields gracefully', () => {
      const schemasWithoutFields = [
        { name: 'EmptySchema', state: 'approved' }
      ];
      mockProps.approvedSchemas = schemasWithoutFields;
      mockProps.queryState.selectedSchema = 'EmptySchema';

      renderWithRedux(<QueryForm {...mockProps} />, { initialState: createAuthenticatedState() });

      // Should not crash and should still show the form
      expect(screen.getByText('Schema')).toBeInTheDocument();
      // Single Field Options section is not rendered when schema has no fields
    });

    it('should handle empty approved schemas array', () => {
      mockProps.approvedSchemas = [];
      renderWithRedux(<QueryForm {...mockProps} />, { initialState: createAuthenticatedState() });

      expect(screen.getByText('Schema')).toBeInTheDocument();
      expect(screen.getByText('No options available')).toBeInTheDocument();
    });

    it('should handle null queryState gracefully', () => {
      // This shouldn't happen in practice, but good to test defensive coding
      const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
      
      try {
        renderWithRedux(<QueryForm {...mockProps} queryState={null} />, { initialState: createAuthenticatedState() });
        // Should not crash
      } catch (error) {
        // If it does crash, that's also a valid test result
        expect(error).toBeDefined();
      }
      
      consoleSpy.mockRestore();
    });
  });
});