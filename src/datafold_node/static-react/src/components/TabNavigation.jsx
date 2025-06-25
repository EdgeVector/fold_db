/**
 * @fileoverview TabNavigation Component - Authentication-aware tab navigation
 *
 * This component provides a reusable tab navigation interface with built-in
 * authentication awareness, accessibility features, and smooth transitions.
 * It automatically handles disabled states for authentication-required tabs
 * and provides visual indicators for auth status.
 *
 * **Key Features:**
 * - Authentication-aware tab management
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
  AUTH_INDICATORS,
  TAB_TRANSITION_DURATION_MS
} from '../constants/ui.js';

import { COMPONENT_STYLES } from '../constants/styling.js';

/**
 * @typedef {Object} TabConfig
 * @property {string} id - Unique tab identifier used for navigation and state management
 * @property {string} label - Display label shown to users
 * @property {boolean} requiresAuth - Whether tab requires user authentication to access
 * @property {string} [icon] - Optional emoji or icon character for visual enhancement
 * @property {boolean} [disabled] - Whether tab is disabled regardless of auth status
 */

/**
 * @typedef {Object} TabNavigationProps
 * @property {TabConfig[]} [tabs=DEFAULT_TABS] - Array of tab configurations to display
 * @property {string} activeTab - Currently active tab ID for highlighting
 * @property {boolean} isAuthenticated - Current user authentication status
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
 * - Tabs with `requiresAuth: true` are disabled when user is not authenticated
 * - Visual indicators (🔒/✓) show authentication status
 * - Clicking disabled auth-required tabs is prevented
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
 *   const [isAuthenticated, setIsAuthenticated] = useState(false);
 *
 *   return (
 *     <TabNavigation
 *       activeTab={activeTab}
 *       isAuthenticated={isAuthenticated}
 *       onTabChange={setActiveTab}
 *     />
 *   );
 * }
 *
 * // Custom tabs configuration
 * const customTabs = [
 *   { id: 'dashboard', label: 'Dashboard', requiresAuth: true, icon: '📊' },
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
 *       isAuthenticated={isAuthenticated}
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
  isAuthenticated,
  onTabChange,
  className = ''
}) {
  const handleTabClick = (tabId, requiresAuth) => {
    // Prevent navigation to auth-required tabs when not authenticated
    if (requiresAuth && !isAuthenticated) {
      return;
    }
    onTabChange(tabId);
  };

  const getTabStyles = (tab) => {
    const isActive = activeTab === tab.id;
    const isDisabled = tab.disabled || (tab.requiresAuth && !isAuthenticated);
    
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

  const getAuthIndicator = (tab) => {
    if (!tab.requiresAuth) {
      return isAuthenticated ? AUTH_INDICATORS.unlocked : null;
    }
    
    return isAuthenticated ? null : AUTH_INDICATORS.locked;
  };

  return (
    <div className={`border-b border-gray-200 ${className}`}>
      <div className="flex space-x-8">
        {tabs.map((tab) => {
          const isDisabled = tab.disabled || (tab.requiresAuth && !isAuthenticated);
          const authIndicator = getAuthIndicator(tab);
          
          return (
            <button
              key={tab.id}
              className={getTabStyles(tab)}
              onClick={() => handleTabClick(tab.id, tab.requiresAuth)}
              disabled={isDisabled}
              aria-current={activeTab === tab.id ? 'page' : undefined}
              aria-label={`${tab.label} tab${tab.requiresAuth && !isAuthenticated ? ' (requires authentication)' : ''}`}
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
              
              {/* Auth Indicator */}
              {authIndicator && (
                <span 
                  className="ml-1 text-xs" 
                  aria-label={
                    authIndicator === AUTH_INDICATORS.locked 
                      ? 'authentication required' 
                      : 'authenticated'
                  }
                >
                  {authIndicator}
                </span>
              )}
            </button>
          );
        })}
      </div>
    </div>
  );
}

export default TabNavigation;