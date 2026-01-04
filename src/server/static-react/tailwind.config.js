/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        primary: '#4f46e5',
        secondary: '#6366f1',
        success: '#10b981',
        error: '#ef4444',
      },
    },
  },
  plugins: [
    require('@tailwindcss/forms'),
  ],
}