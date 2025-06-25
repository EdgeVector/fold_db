/**
 * SchemaStatusBadge Component
 * Displays schema status with consistent styling and range indicators
 * Part of TASK-002: Component Extraction and Modularization
 */

import { 
  SCHEMA_BADGE_COLORS, 
  RANGE_SCHEMA_CONFIG,
  HELP_TEXT 
} from '../../constants/ui.js';

/**
 * @typedef {Object} SchemaStatusBadgeProps
 * @property {'approved'|'available'|'blocked'|'pending'} state - Schema state
 * @property {boolean} [isRangeSchema] - Whether this is a range schema
 * @property {'sm'|'md'|'lg'} [size] - Badge size
 * @property {string} [className] - Additional CSS classes
 * @property {boolean} [showTooltip] - Whether to show state explanation on hover
 */

/**
 * Reusable schema status badge component with range schema indicators
 * 
 * @param {SchemaStatusBadgeProps} props
 * @returns {JSX.Element}
 */
function SchemaStatusBadge({
  state,
  isRangeSchema = false,
  size = 'md',
  className = '',
  showTooltip = true
}) {
  // Size configurations
  const sizeClasses = {
    sm: 'px-1.5 py-0.5 text-xs',
    md: 'px-2.5 py-0.5 text-xs',
    lg: 'px-3 py-1 text-sm'
  };

  // Get badge color based on state
  const getBadgeColor = () => {
    return SCHEMA_BADGE_COLORS[state] || SCHEMA_BADGE_COLORS.available;
  };

  // Get state display text
  const getStateText = () => {
    const stateTexts = {
      approved: 'Approved',
      available: 'Available',
      blocked: 'Blocked',
      pending: 'Pending'
    };
    return stateTexts[state] || 'Unknown';
  };

  // Get tooltip text
  const getTooltipText = () => {
    if (!showTooltip) return '';
    return HELP_TEXT.schemaStates[state] || 'Unknown schema state';
  };

  const badgeClasses = `
    inline-flex items-center rounded-full font-medium
    ${sizeClasses[size]}
    ${getBadgeColor()}
    ${className}
  `.trim();

  return (
    <div className="inline-flex items-center space-x-2">
      {/* Main Status Badge */}
      <span
        className={badgeClasses}
        title={getTooltipText()}
        aria-label={`Schema status: ${getStateText()}${isRangeSchema ? ', Range Schema' : ''}`}
      >
        {getStateText()}
      </span>
      
      {/* Range Schema Indicator */}
      {isRangeSchema && (
        <span
          className={`
            inline-flex items-center rounded-full font-medium
            ${sizeClasses[size]}
            ${RANGE_SCHEMA_CONFIG.badgeColor}
          `}
          title="This schema uses range-based keys for efficient querying"
          aria-label="Range Schema"
        >
          {RANGE_SCHEMA_CONFIG.label}
        </span>
      )}
    </div>
  );
}

export default SchemaStatusBadge;