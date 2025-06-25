/**
 * Styling Constants and Theme Values
 * TASK-005: Constants Extraction and Configuration Centralization
 * Section 2.1.12 - Use of Constants for Repeated or Special Values
 */

// ============================================================================
// COLOR PALETTE
// ============================================================================

/**
 * Application color palette following design system
 */
export const COLORS = {
  // Primary brand colors
  PRIMARY: {
    50: '#eff6ff',
    100: '#dbeafe',
    200: '#bfdbfe',
    300: '#93c5fd',
    400: '#60a5fa',
    500: '#3b82f6',
    600: '#2563eb',
    700: '#1d4ed8',
    800: '#1e40af',
    900: '#1e3a8a'
  },
  
  // Secondary colors
  SECONDARY: {
    50: '#f8fafc',
    100: '#f1f5f9',
    200: '#e2e8f0',
    300: '#cbd5e1',
    400: '#94a3b8',
    500: '#64748b',
    600: '#475569',
    700: '#334155',
    800: '#1e293b',
    900: '#0f172a'
  },
  
  // Status colors
  STATUS: {
    SUCCESS: '#10b981',
    SUCCESS_LIGHT: '#d1fae5',
    SUCCESS_DARK: '#047857',
    WARNING: '#f59e0b',
    WARNING_LIGHT: '#fef3c7',
    WARNING_DARK: '#d97706',
    ERROR: '#ef4444',
    ERROR_LIGHT: '#fee2e2',
    ERROR_DARK: '#dc2626',
    INFO: '#3b82f6',
    INFO_LIGHT: '#dbeafe',
    INFO_DARK: '#1d4ed8'
  },
  
  // Schema state colors (SCHEMA-002 compliance)
  SCHEMA_STATES: {
    APPROVED: {
      BACKGROUND: 'bg-green-100',
      TEXT: 'text-green-800',
      BORDER: 'border-green-200',
      FULL: 'bg-green-100 text-green-800 border border-green-200'
    },
    AVAILABLE: {
      BACKGROUND: 'bg-blue-100',
      TEXT: 'text-blue-800',
      BORDER: 'border-blue-200',
      FULL: 'bg-blue-100 text-blue-800 border border-blue-200'
    },
    BLOCKED: {
      BACKGROUND: 'bg-red-100',
      TEXT: 'text-red-800',
      BORDER: 'border-red-200',
      FULL: 'bg-red-100 text-red-800 border border-red-200'
    },
    PENDING: {
      BACKGROUND: 'bg-yellow-100',
      TEXT: 'text-yellow-800',
      BORDER: 'border-yellow-200',
      FULL: 'bg-yellow-100 text-yellow-800 border border-yellow-200'
    }
  },
  
  // Field type colors
  FIELD_TYPES: {
    STRING: {
      BACKGROUND: 'bg-blue-100',
      TEXT: 'text-blue-800',
      ICON: '📝'
    },
    NUMBER: {
      BACKGROUND: 'bg-green-100',
      TEXT: 'text-green-800',
      ICON: '🔢'
    },
    BOOLEAN: {
      BACKGROUND: 'bg-purple-100',
      TEXT: 'text-purple-800',
      ICON: '✓'
    },
    RANGE: {
      BACKGROUND: 'bg-orange-100',
      TEXT: 'text-orange-800',
      ICON: '📊'
    },
    OBJECT: {
      BACKGROUND: 'bg-gray-100',
      TEXT: 'text-gray-800',
      ICON: '📦'
    },
    ARRAY: {
      BACKGROUND: 'bg-pink-100',
      TEXT: 'text-pink-800',
      ICON: '📋'
    }
  },
  
  // Permission policy colors
  PERMISSIONS: {
    READ: {
      BACKGROUND: 'bg-blue-100',
      TEXT: 'text-blue-800'
    },
    WRITE: {
      BACKGROUND: 'bg-orange-100',
      TEXT: 'text-orange-800'
    },
    NO_REQUIREMENT: {
      BACKGROUND: 'bg-gray-100',
      TEXT: 'text-gray-800'
    },
    DISTANCE: {
      BACKGROUND: 'bg-purple-100',
      TEXT: 'text-purple-800'
    }
  },
  
  // Range schema specific colors
  RANGE_SCHEMA: {
    BACKGROUND: 'bg-purple-50',
    BORDER: 'border-purple-200',
    BADGE_BACKGROUND: 'bg-purple-200',
    BADGE_TEXT: 'text-purple-800',
    ACCENT: '#8b5cf6'
  }
};

// ============================================================================
// LAYOUT AND SPACING
// ============================================================================

/**
 * Layout dimensions and spacing values
 */
export const LAYOUT = {
  // Component dimensions
  SIDEBAR_WIDTH: 320,
  HEADER_HEIGHT: 64,
  FOOTER_HEIGHT: 48,
  TAB_HEIGHT: 44,
  
  // Content spacing
  CONTENT_PADDING: 24,
  SECTION_PADDING: 16,
  CARD_PADDING: 20,
  
  // Border radius values
  BORDER_RADIUS: {
    NONE: 0,
    SMALL: 4,
    MEDIUM: 8,
    LARGE: 12,
    FULL: 9999
  },
  
  // Shadow values
  SHADOWS: {
    NONE: 'shadow-none',
    SMALL: 'shadow-sm',
    MEDIUM: 'shadow-md',
    LARGE: 'shadow-lg',
    EXTRA_LARGE: 'shadow-xl'
  },
  
  // Grid and flexbox
  GRID: {
    COLUMNS_2: 'grid-cols-2',
    COLUMNS_3: 'grid-cols-3',
    COLUMNS_4: 'grid-cols-4',
    GAP_SMALL: 'gap-2',
    GAP_MEDIUM: 'gap-4',
    GAP_LARGE: 'gap-6'
  }
};

// ============================================================================
// TYPOGRAPHY
// ============================================================================

/**
 * Typography scale and text styling
 */
export const TYPOGRAPHY = {
  // Font sizes
  FONT_SIZE: {
    EXTRA_SMALL: 'text-xs',      // 12px
    SMALL: 'text-sm',            // 14px
    BASE: 'text-base',           // 16px
    LARGE: 'text-lg',            // 18px
    EXTRA_LARGE: 'text-xl',      // 20px
    DOUBLE_EXTRA_LARGE: 'text-2xl', // 24px
    TRIPLE_EXTRA_LARGE: 'text-3xl'  // 30px
  },
  
  // Font weights
  FONT_WEIGHT: {
    THIN: 'font-thin',
    LIGHT: 'font-light',
    NORMAL: 'font-normal',
    MEDIUM: 'font-medium',
    SEMIBOLD: 'font-semibold',
    BOLD: 'font-bold',
    EXTRA_BOLD: 'font-extrabold'
  },
  
  // Line heights
  LINE_HEIGHT: {
    TIGHT: 'leading-tight',
    NORMAL: 'leading-normal',
    RELAXED: 'leading-relaxed',
    LOOSE: 'leading-loose'
  },
  
  // Text alignment
  TEXT_ALIGN: {
    LEFT: 'text-left',
    CENTER: 'text-center',
    RIGHT: 'text-right',
    JUSTIFY: 'text-justify'
  }
};

// ============================================================================
// COMPONENT STYLES
// ============================================================================

/**
 * Reusable component styling patterns
 */
export const COMPONENT_STYLES = {
  // Tab styling
  tab: {
    base: 'px-4 py-2 text-sm font-medium transition-all duration-200',
    active: 'text-primary border-b-2 border-primary',
    inactive: 'text-gray-500 hover:text-gray-700 hover:border-gray-300',
    disabled: 'text-gray-300 cursor-not-allowed'
  },
  
  // Button styling
  button: {
    base: 'inline-flex items-center px-4 py-2 border border-transparent rounded-md shadow-sm text-sm font-medium transition-colors duration-200 focus:outline-none focus:ring-2 focus:ring-offset-2',
    primary: 'bg-primary hover:bg-primary/90 text-white focus:ring-primary',
    secondary: 'bg-gray-100 hover:bg-gray-200 text-gray-700 focus:ring-gray-500',
    success: 'bg-green-600 hover:bg-green-700 text-white focus:ring-green-500',
    danger: 'bg-red-600 hover:bg-red-700 text-white focus:ring-red-500',
    warning: 'bg-yellow-600 hover:bg-yellow-700 text-white focus:ring-yellow-500',
    disabled: 'bg-gray-300 cursor-not-allowed text-gray-500'
  },
  
  // Input styling
  input: {
    base: 'block w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-1 transition-colors duration-200',
    normal: 'focus:ring-primary focus:border-primary',
    error: 'border-red-300 focus:ring-red-500 focus:border-red-500',
    success: 'border-green-300 focus:ring-green-500 focus:border-green-500',
    disabled: 'bg-gray-100 text-gray-500 cursor-not-allowed'
  },
  
  // Select styling
  select: {
    base: 'block w-full pl-3 pr-10 py-2 text-base border-gray-300 focus:outline-none focus:ring-primary focus:border-primary rounded-md transition-colors duration-200',
    disabled: 'bg-gray-100 text-gray-500 cursor-not-allowed'
  },
  
  // Card styling
  card: {
    base: 'bg-white rounded-lg border border-gray-200 shadow-sm overflow-hidden transition-all duration-200',
    hover: 'hover:shadow-md',
    selected: 'ring-2 ring-primary border-primary'
  },
  
  // Modal styling
  modal: {
    overlay: 'fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50',
    content: 'bg-white rounded-lg shadow-xl max-w-md w-full mx-4 p-6',
    header: 'flex items-center justify-between pb-4 border-b',
    body: 'py-4',
    footer: 'pt-4 border-t flex justify-end space-x-2'
  },
  
  // Alert styling
  alert: {
    base: 'p-4 rounded-lg border',
    success: 'bg-green-50 border-green-200 text-green-800',
    error: 'bg-red-50 border-red-200 text-red-800',
    warning: 'bg-yellow-50 border-yellow-200 text-yellow-800',
    info: 'bg-blue-50 border-blue-200 text-blue-800'
  }
};

// ============================================================================
// ANIMATION AND TRANSITIONS
// ============================================================================

/**
 * Animation timing and transition values
 */
export const ANIMATIONS = {
  // Duration values
  DURATION: {
    FAST: 100,
    NORMAL: 200,
    SLOW: 300,
    EXTRA_SLOW: 500
  },
  
  // Easing functions
  EASING: {
    LINEAR: 'linear',
    EASE: 'ease',
    EASE_IN: 'ease-in',
    EASE_OUT: 'ease-out',
    EASE_IN_OUT: 'ease-in-out'
  },
  
  // Common transitions
  TRANSITIONS: {
    ALL: 'transition-all duration-200',
    COLORS: 'transition-colors duration-200',
    TRANSFORM: 'transition-transform duration-200',
    OPACITY: 'transition-opacity duration-200'
  }
};

// ============================================================================
// BREAKPOINTS AND RESPONSIVE
// ============================================================================

/**
 * Responsive breakpoints and media queries
 */
export const BREAKPOINTS = {
  // Pixel values
  MOBILE: 320,
  TABLET: 768,
  DESKTOP: 1024,
  LARGE_DESKTOP: 1280,
  EXTRA_LARGE: 1440,
  
  // CSS values
  CSS: {
    MOBILE: '320px',
    TABLET: '768px',
    DESKTOP: '1024px',
    LARGE_DESKTOP: '1280px',
    EXTRA_LARGE: '1440px'
  },
  
  // Tailwind responsive prefixes
  TAILWIND: {
    SMALL: 'sm:',
    MEDIUM: 'md:',
    LARGE: 'lg:',
    EXTRA_LARGE: 'xl:',
    DOUBLE_EXTRA_LARGE: '2xl:'
  }
};

// ============================================================================
// Z-INDEX STACK
// ============================================================================

/**
 * Z-index layering system
 */
export const Z_INDEX = {
  BASE: 0,
  DROPDOWN: 10,
  STICKY: 20,
  FIXED: 30,
  MODAL_BACKDROP: 40,
  MODAL: 50,
  POPOVER: 60,
  TOOLTIP: 100,
  NOTIFICATION: 500,
  OVERLAY: 1000
};

// ============================================================================
// DEFAULT EXPORT
// ============================================================================

export default {
  COLORS,
  LAYOUT,
  TYPOGRAPHY,
  COMPONENT_STYLES,
  ANIMATIONS,
  BREAKPOINTS,
  Z_INDEX
};