/**
 * @fileoverview TabNavigation Component - Terminal-styled tab navigation
 *
 * This component provides a CLI-themed tab navigation interface.
 *
 * @module TabNavigation
 * @since 2.0.0
 */

import {
  DEFAULT_TABS,
  TAB_TRANSITION_DURATION_MS
} from '../constants/ui.js';

/**
 * Terminal-styled tab navigation component
 *
 * @component
 * @param {Object} props - Component props
 * @param {Array} props.tabs - Array of tab configurations
 * @param {string} props.activeTab - Currently active tab ID
 * @param {Function} props.onTabChange - Callback when tab changes
 * @param {string} props.className - Additional CSS classes
 * @returns {JSX.Element} Rendered tab navigation component
 */
function TabNavigation({
  tabs = DEFAULT_TABS,
  activeTab,
  onTabChange,
  className = ''
}) {
  const handleTabClick = (tabId) => {
    onTabChange(tabId);
  };

  const getTabStyles = (tab) => {
    const isActive = activeTab === tab.id;
    const isDisabled = tab.disabled || false;
    
    let baseStyles = 'terminal-tab relative px-4 py-2.5 text-sm font-medium transition-all duration-150';
    
    if (isActive) {
      return `${baseStyles} active text-terminal-green`;
    } else if (isDisabled) {
      return `${baseStyles} disabled text-terminal-dim opacity-50 cursor-not-allowed`;
    } else {
      return `${baseStyles} text-terminal-dim hover:text-terminal`;
    }
  };

  // Group tabs
  const mainTabs = tabs.filter(tab => tab.group === 'main');
  const advancedTabs = tabs.filter(tab => tab.group === 'advanced');

  const renderTab = (tab, index) => {
    const isDisabled = tab.disabled || false;
    const isActive = activeTab === tab.id;
    
    return (
      <button
        key={tab.id}
        className={getTabStyles(tab)}
        onClick={() => handleTabClick(tab.id)}
        disabled={isDisabled}
        aria-current={isActive ? 'page' : undefined}
        aria-label={`${tab.label} tab`}
        style={{
          transitionDuration: `${TAB_TRANSITION_DURATION_MS}ms`
        }}
      >
        {/* Command number prefix */}
        <span className="text-terminal-dim mr-1.5 text-xs">[{index + 1}]</span>
        
        {/* Tab Icon */}
        {tab.icon && (
          <span className="mr-1.5 opacity-75" aria-hidden="true">
            {tab.icon}
          </span>
        )}
        
        {/* Tab Label - CLI style */}
        <span className="lowercase">{tab.label.toLowerCase().replace(/\s+/g, '-')}</span>
        
        {/* Active indicator line */}
        {isActive && (
          <span className="absolute bottom-0 left-0 right-0 h-0.5 bg-terminal-green" 
                style={{ boxShadow: '0 0 8px rgba(63, 185, 80, 0.5)' }} />
        )}
      </button>
    );
  };

  return (
    <div className={`terminal-tabs ${className}`}>
      {/* Prompt prefix */}
      <div className="flex items-center px-3 py-2.5 text-sm">
        <span className="text-terminal-green font-medium">$</span>
        <span className="text-terminal-dim ml-2">navigate</span>
        <span className="text-terminal-dim ml-1">→</span>
      </div>
      
      {/* Main tabs */}
      <div className="flex">
        {mainTabs.map((tab, index) => renderTab(tab, index))}
      </div>

      {/* Separator */}
      {advancedTabs.length > 0 && (
        <div className="flex items-center px-3">
          <span className="text-terminal-dim">|</span>
        </div>
      )}

      {/* Advanced tabs with label */}
      {advancedTabs.length > 0 && (
        <div className="flex items-center">
          <span className="text-xs text-terminal-yellow font-medium uppercase tracking-wider px-2">
            --advanced
          </span>
          <div className="flex">
            {advancedTabs.map((tab, index) => renderTab(tab, mainTabs.length + index))}
          </div>
        </div>
      )}
    </div>
  );
}

export default TabNavigation;