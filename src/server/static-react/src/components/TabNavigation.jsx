/**
 * @fileoverview TabNavigation Component - Minimal tab navigation
 * @module TabNavigation
 */

import {
  DEFAULT_TABS,
  TAB_TRANSITION_DURATION_MS
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

    const baseStyles = {
      padding: '16px 24px',
      fontSize: '14px',
      color: isActive ? '#111' : '#666',
      background: 'transparent',
      border: 'none',
      borderBottom: isActive ? '2px solid #111' : '2px solid transparent',
      cursor: isDisabled ? 'not-allowed' : 'pointer',
      opacity: isDisabled ? 0.4 : 1,
      transition: 'all 0.2s ease',
      fontFamily: 'inherit',
      fontWeight: isActive ? 500 : 400,
    };

    return (
      <button
        key={tab.id}
        style={baseStyles}
        onClick={() => handleTabClick(tab.id)}
        disabled={isDisabled}
        aria-current={isActive ? 'page' : undefined}
        aria-label={`${tab.label} tab`}
        onMouseOver={(e) => {
          if (!isDisabled && !isActive) {
            e.target.style.color = '#111';
          }
        }}
        onMouseOut={(e) => {
          if (!isDisabled && !isActive) {
            e.target.style.color = '#666';
          }
        }}
      >
        {tab.label}
      </button>
    );
  };

  return (
    <nav
      className={className}
      style={{
        background: '#fff',
        borderBottom: '1px solid #e5e5e5',
        padding: '0 40px',
        display: 'flex',
        alignItems: 'center',
      }}
    >
      {/* Main tabs */}
      <div style={{ display: 'flex' }}>
        {mainTabs.map((tab) => renderTab(tab))}
      </div>

      {/* Separator and Advanced tabs */}
      {advancedTabs.length > 0 && (
        <>
          <div style={{
            width: '1px',
            height: '24px',
            background: '#e5e5e5',
            margin: '0 16px'
          }} />
          <div style={{ display: 'flex' }}>
            {advancedTabs.map((tab) => renderTab(tab))}
          </div>
        </>
      )}
    </nav>
  );
}

export default TabNavigation;
