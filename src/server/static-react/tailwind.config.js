/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        // Grayscale
        surface: {
          DEFAULT: '#ffffff',
          secondary: '#fafafa',
        },
        border: {
          DEFAULT: '#e5e5e5',
          dark: '#d4d4d4',
        },
        // Text colors
        primary: '#111111',
        secondary: '#666666',
        tertiary: '#999999',
        // Accent colors
        accent: {
          DEFAULT: '#111111',
          hover: '#333333',
        },
      },
      fontFamily: {
        sans: ['Inter', 'Helvetica Neue', 'Helvetica', 'Arial', 'sans-serif'],
        mono: ['SF Mono', 'Monaco', 'monospace'],
      },
    },
  },
  plugins: [
    require('@tailwindcss/forms'),
  ],
}
