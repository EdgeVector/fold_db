/**
 * Component Style Constants
 * Tailwind utility class patterns for commonly styled form elements.
 */

export const COMPONENT_STYLES = {
  // Input styling
  input: {
    base: "block w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-1 transition-colors duration-200",
    normal: "focus:ring-blue-600 focus:border-blue-600",
    error: "border-red-300 focus:ring-red-500 focus:border-red-500",
    success: "border-green-300 focus:ring-green-500 focus:border-green-500",
    disabled: "bg-gray-100 text-gray-500 cursor-not-allowed",
  },

  // Select styling
  select: {
    base: "block w-full pl-3 pr-10 py-2 text-base border-gray-300 focus:outline-none focus:ring-blue-600 focus:border-blue-600 rounded-md transition-colors duration-200",
    disabled: "bg-gray-100 text-gray-500 cursor-not-allowed",
  },
};
