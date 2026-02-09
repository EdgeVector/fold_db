/**
 * @fileoverview TabNavigation Component - Minimal tab navigation
 * @module TabNavigation
 */

import {
  DEFAULT_TABS,
} from '../constants/ui.js';

/**
 * Minimal tab navigation component
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

  // Group tabs
  const mainTabs = tabs.filter(tab => tab.group === 'main');
  const advancedTabs = tabs.filter(tab => tab.group === 'advanced');

  const renderTab = (tab) => {
    const isDisabled = tab.disabled || false;
    const isActive = activeTab === tab.id;

    return (
      <button
        key={tab.id}
        className={`minimal-nav-item${isActive ? ' active' : ''}`}
        onClick={() => handleTabClick(tab.id)}
        disabled={isDisabled}
        aria-current={isActive ? 'page' : undefined}
        aria-label={`${tab.label} tab`}
      >
        {tab.label}
      </button>
    );
  };

  return (
    <nav className={`minimal-nav ${className}`}>
      {/* Main tabs */}
      <div className="minimal-nav-group">
        {mainTabs.map((tab) => renderTab(tab))}
      </div>

      {/* Separator and Advanced tabs */}
      {advancedTabs.length > 0 && (
        <>
          <div className="minimal-nav-separator" />
          <div className="minimal-nav-group">
            {advancedTabs.map((tab) => renderTab(tab))}
          </div>
        </>
      )}
    </nav>
  );
}

export default TabNavigation;
