/**
 * SchemaFieldList Component
 * Displays schema fields with range key highlighting and permission visualization
 * Part of TASK-002: Component Extraction and Modularization
 */

import { useState } from 'react';
import { 
  FIELD_TYPE_CONFIG, 
  PERMISSION_COLORS, 
  RANGE_SCHEMA_CONFIG 
} from '../../constants/ui.js';

/**
 * @typedef {Object} Field
 * @property {string} field_type - Type of the field
 * @property {boolean} [writable] - Whether field is writable
 * @property {Object} [permission_policy] - Permission policies
 * @property {Object} [transform] - Transform information
 * @property {string} [molecule_uuid] - Molecule UUID
 */

/**
 * @typedef {Object} SchemaFieldListProps
 * @property {Object.<string, Field>} fields - Schema fields object
 * @property {string[]} [rangeKeys] - Array of range key field names
 * @property {boolean} [expandable] - Whether fields can be expanded/collapsed
 * @property {boolean} [showPermissions] - Whether to show permission policies
 * @property {string} [className] - Additional CSS classes
 */

/**
 * Reusable schema field list component with range key highlighting
 * 
 * @param {SchemaFieldListProps} props
 * @returns {JSX.Element}
 */
function SchemaFieldList({
  fields = {},
  rangeKeys = [],
  expandable = true,
  showPermissions = true,
  className = ''
}) {
  const [expandedFields, setExpandedFields] = useState({});

  const toggleFieldExpansion = (fieldName) => {
    if (!expandable) return;
    
    setExpandedFields(prev => ({
      ...prev,
      [fieldName]: !prev[fieldName]
    }));
  };

  const formatPermissionPolicy = (policy) => {
    if (!policy) return 'Unknown';
    if (policy.NoRequirement !== undefined) return 'No Requirement';
    if (policy.Distance !== undefined) return `Trust Distance ${policy.Distance}`;
    return 'Unknown';
  };

  const getFieldTypeConfig = (fieldType) => {
    return FIELD_TYPE_CONFIG[fieldType] || FIELD_TYPE_CONFIG.String;
  };

  const renderField = (fieldName, field) => {
    const isRangeKey = rangeKeys.includes(fieldName);
    const isExpanded = expandedFields[fieldName];
    const fieldConfig = getFieldTypeConfig(field.field_type);

    return (
      <div 
        key={fieldName} 
        className={`rounded-md p-4 hover:bg-gray-100 transition-colors duration-200 ${
          isRangeKey 
            ? `${RANGE_SCHEMA_CONFIG.backgroundColor} ${RANGE_SCHEMA_CONFIG.borderColor} border` 
            : 'bg-gray-50'
        }`}
      >
        <div className="flex justify-between items-start">
          <div className="flex-1 space-y-2">
            {/* Field Header */}
            <div className="flex items-center">
              <button
                onClick={() => toggleFieldExpansion(fieldName)}
                className="flex items-center focus:outline-none focus:ring-2 focus:ring-primary rounded"
                disabled={!expandable}
              >
                <span className="font-medium text-gray-900 mr-2">{fieldName}</span>
                
                {/* Field Type Badge */}
                <span className={`px-2 py-0.5 text-xs font-medium rounded-full ${fieldConfig.color} mr-2`}>
                  {fieldConfig.icon && <span className="mr-1">{fieldConfig.icon}</span>}
                  {field.field_type}
                </span>
                
                {/* Range Key Badge */}
                {isRangeKey && (
                  <span className={`px-2 py-0.5 text-xs font-medium rounded-full ${RANGE_SCHEMA_CONFIG.badgeColor} mr-2`}>
                    {RANGE_SCHEMA_CONFIG.label}
                  </span>
                )}

                {/* Expansion Indicator */}
                {expandable && (
                  <span className="text-gray-400">
                    {isExpanded ? '▼' : '▶'}
                  </span>
                )}
              </button>
            </div>
            
            {/* Field Details (expanded view) */}
            {(!expandable || isExpanded) && (
              <div className="space-y-3 ml-4">
                {/* Permission Policies */}
                {showPermissions && field.permission_policy && (
                  <div className="space-y-1">
                    <div className="flex items-center text-xs text-gray-600">
                      <span className="font-medium mr-2">Read:</span>
                      <span className={`px-1.5 py-0.5 rounded ${PERMISSION_COLORS.read}`}>
                        {formatPermissionPolicy(field.permission_policy.read_policy)}
                      </span>
                    </div>
                    <div className="flex items-center text-xs text-gray-600">
                      <span className="font-medium mr-2">Write:</span>
                      <span className={`px-1.5 py-0.5 rounded ${PERMISSION_COLORS.write}`}>
                        {formatPermissionPolicy(field.permission_policy.write_policy)}
                      </span>
                    </div>
                  </div>
                )}
                
                {/* Transform Information */}
                {field.transform && (
                  <div className="flex items-center text-sm text-gray-600">
                    <svg className="h-4 w-4 mr-1" viewBox="0 0 20 20" fill="currentColor">
                      <path fillRule="evenodd" d="M11.3 1.046A1 1 0 0112 2v5h4a1 1 0 01.82 1.573l-7 10A1 1 0 018 18v-5H4a1 1 0 01-.82-1.573l7-10a1 1 0 011.12-.38z" clipRule="evenodd" />
                    </svg>
                    <span className="font-medium">Transform:</span>
                    <span className="ml-1">{field.transform.name}</span>
                  </div>
                )}
                
                {/* Molecule UUID */}
                {field.molecule_uuid && (
                  <div className="text-xs text-gray-500 break-all">
                    <span className="font-medium">Molecule ID:</span> {field.molecule_uuid}
                  </div>
                )}
              </div>
            )}
          </div>
          
          {/* Writable Status Badge */}
          <span className={`
            inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium
            ${field.writable
              ? 'bg-green-100 text-green-800'
              : 'bg-gray-100 text-gray-800'
            }
          `}>
            {field.writable ? 'Writable' : 'Read-only'}
          </span>
        </div>
      </div>
    );
  };

  const fieldEntries = Object.entries(fields);

  if (fieldEntries.length === 0) {
    return (
      <div className={`text-center py-8 text-gray-500 ${className}`}>
        No fields defined for this schema
      </div>
    );
  }

  return (
    <div className={`space-y-3 ${className}`}>
      {/* Field Count Summary */}
      <div className="text-sm text-gray-600 mb-4">
        <span className="font-medium">{fieldEntries.length}</span> field{fieldEntries.length !== 1 ? 's' : ''}
        {rangeKeys.length > 0 && (
          <span className="ml-2">
            • <span className="font-medium">{rangeKeys.length}</span> range key{rangeKeys.length !== 1 ? 's' : ''}
          </span>
        )}
      </div>

      {/* Fields List */}
      {fieldEntries.map(([fieldName, field]) => renderField(fieldName, field))}
    </div>
  );
}

export default SchemaFieldList;