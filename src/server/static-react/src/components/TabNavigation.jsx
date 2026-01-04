/**
 * @fileoverview TabNavigation Component - Public UI tab navigation
 *
 * This component provides a reusable tab navigation interface with built-in
 * authentication awareness, accessibility features, and smooth transitions.
 * It provides accessible navigation, disabled state handling, and icons.
 *
 * **Key Features:**
 * - Simple, public tab management (no authentication)
 * - Accessibility compliant (ARIA attributes, keyboard navigation)
 * - Smooth transitions with configurable duration
 * - Icon support for visual enhancement
 * - Disabled state handling
 * - Customizable styling via CSS classes
 *
 * TASK-002: Extracted from App.jsx for reusability and modularization
 * TASK-006: Enhanced with comprehensive JSDoc documentation
 *
 * @module TabNavigation
 * @since 2.0.0
 */

import {
  DEFAULT_TABS,
  TAB_TRANSITION_DURATION_MS
} from '../constants/ui.js';

import { COMPONENT_STYLES } from '../constants/styling.js';

/**
 * @typedef {Object} TabConfig
 * @property {string} id - Unique tab identifier used for navigation and state management
 * @property {string} label - Display label shown to users
 * @property {boolean} [requiresAuth] - Deprecated, ignored in public UI
 * @property {string} [icon] - Optional emoji or icon character for visual enhancement
 * @property {boolean} [disabled] - Whether tab is disabled regardless of auth status
 */

/**
 * @typedef {Object} TabNavigationProps
 * @property {TabConfig[]} [tabs=DEFAULT_TABS] - Array of tab configurations to display
 * @property {string} activeTab - Currently active tab ID for highlighting
 * @property {Function} onTabChange - Callback fired when user selects a tab (tabId: string) => void
 * @property {string} [className] - Additional CSS classes for customization
 */

/**
 * Authentication-aware tab navigation component with accessibility support
 *
 * This component renders a horizontal tab navigation bar that automatically
 * handles authentication requirements, disabled states, and visual indicators.
 * It provides smooth transitions and follows accessibility best practices.
 *
 * **Authentication Behavior:**
 * - Not applicable. UI does not use authentication.
 *
 * **Accessibility Features:**
 * - Proper ARIA attributes (`aria-current`, `aria-label`)
 * - Keyboard navigation support
 * - Screen reader friendly with descriptive labels
 * - Focus management and visual indicators
 *
 * **Styling:**
 * - Uses constants from COMPONENT_STYLES for consistent theming
 * - Configurable transition duration
 * - Responsive design considerations
 * - Support for custom CSS classes
 *
 * @component
 * @param {TabNavigationProps} props - Component props
 * @returns {JSX.Element} Rendered tab navigation component
 *
 * @example
 * ```jsx
 * // Basic usage with default tabs
 * function App() {
 *   const [activeTab, setActiveTab] = useState('schemas');
 *
 *   return (
 *     <TabNavigation
 *       activeTab={activeTab}
 *       onTabChange={setActiveTab}
 *     />
 *   );
 * }
 *
 * // Custom tabs configuration
 * const customTabs = [
 *   { id: 'dashboard', label: 'Dashboard', icon: '📊' },
 *   { id: 'settings', label: 'Settings', requiresAuth: false, icon: '⚙️' },
 *   { id: 'admin', label: 'Admin', requiresAuth: true, disabled: !isAdmin }
 * ];
 *
 * <TabNavigation
 *   tabs={customTabs}
 *   activeTab={activeTab}
 *   isAuthenticated={isAuthenticated}
 *   onTabChange={handleTabChange}
 *   className="border-t-2"
 * />
 * ```
 *
 * @example
 * ```jsx
 * // Integration with routing
 * function NavigationContainer() {
 *   const location = useLocation();
 *   const navigate = useNavigate();
 *   const { isAuthenticated } = useAuth();
 *
 *   const handleTabChange = (tabId) => {
 *     navigate(`/${tabId}`);
 *   };
 *
 *   return (
 *     <TabNavigation
 *       activeTab={location.pathname.slice(1)}
 *       onTabChange={handleTabChange}
 *     />
 *   );
 * }
 * ```
 *
 * @since 2.0.0
 */
function TabNavigation({
  tabs = DEFAULT_TABS,
  activeTab,
  onTabChange,
  className = ''
}) {
  const handleTabClick = (tabId, _requiresAuth) => {
    onTabChange(tabId);
  };

  const getTabStyles = (tab) => {
    const isActive = activeTab === tab.id;
    const isDisabled = tab.disabled || false;
    
    let styles = COMPONENT_STYLES.tab.base;
    
    if (isActive) {
      styles += ` ${COMPONENT_STYLES.tab.active}`;
    } else if (isDisabled) {
      styles += ` ${COMPONENT_STYLES.tab.disabled}`;
    } else {
      styles += ` ${COMPONENT_STYLES.tab.inactive}`;
    }
    
    return styles;
  };

  // Group tabs
  const mainTabs = tabs.filter(tab => tab.group === 'main');
  const advancedTabs = tabs.filter(tab => tab.group === 'advanced');

  const renderTab = (tab) => {
    const isDisabled = tab.disabled || false;
    
    return (
      <button
        key={tab.id}
        className={getTabStyles(tab)}
        onClick={() => handleTabClick(tab.id, tab.requiresAuth)}
        disabled={isDisabled}
        aria-current={activeTab === tab.id ? 'page' : undefined}
        aria-label={`${tab.label} tab`}
        style={{
          transitionDuration: `${TAB_TRANSITION_DURATION_MS}ms`
        }}
      >
        {/* Tab Icon */}
        {tab.icon && (
          <span className="mr-2" aria-hidden="true">
            {tab.icon}
          </span>
        )}
        
        {/* Tab Label */}
        <span>{tab.label}</span>
      </button>
    );
  };

  return (
    <div className={`border-b border-gray-200 ${className}`}>
      <div className="flex items-center">
        {/* Main tabs */}
        <div className="flex space-x-8">
          {mainTabs.map(renderTab)}
        </div>

        {/* Separator */}
        {advancedTabs.length > 0 && (
          <div className="mx-6 h-6 w-px bg-gray-300" aria-hidden="true"></div>
        )}

        {/* Advanced tabs with label */}
        {advancedTabs.length > 0 && (
          <div className="flex items-center space-x-6">
            <span className="text-xs text-gray-500 font-medium uppercase tracking-wider">Advanced</span>
            <div className="flex space-x-6">
              {advancedTabs.map(renderTab)}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

export default TabNavigation;