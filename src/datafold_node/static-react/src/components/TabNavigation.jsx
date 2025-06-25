/**
 * TabNavigation Component
 * Extracted from App.jsx to provide reusable tab navigation
 * Part of TASK-002: Component Extraction and Modularization
 */

import { 
  DEFAULT_TABS, 
  COMPONENT_STYLES, 
  AUTH_INDICATORS,
  TAB_TRANSITION_DURATION_MS 
} from '../constants/ui.js';

/**
 * @typedef {Object} TabConfig
 * @property {string} id - Unique tab identifier
 * @property {string} label - Tab display label
 * @property {boolean} requiresAuth - Whether tab requires authentication
 * @property {string} [icon] - Optional icon for tab
 * @property {boolean} [disabled] - Whether tab is disabled
 */

/**
 * @typedef {Object} TabNavigationProps
 * @property {TabConfig[]} [tabs] - Array of tab configurations
 * @property {string} activeTab - Currently active tab ID
 * @property {boolean} isAuthenticated - Whether user is authenticated
 * @property {function} onTabChange - Callback when tab changes (tabId) => void
 * @property {string} [className] - Additional CSS classes
 */

/**
 * Reusable tab navigation component with authentication awareness
 * 
 * @param {TabNavigationProps} props
 * @returns {JSX.Element}
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