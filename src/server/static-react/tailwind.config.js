/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        surface: {
          DEFAULT: '#282828',
          secondary: '#1d2021',
        },
        border: {
          DEFAULT: '#504945',
          dark: '#665c54',
        },
        primary: '#ebdbb2',
        secondary: '#928374',
        tertiary: '#665c54',
        accent: {
          DEFAULT: '#fe8019',
          hover: '#fabd2f',
        },
        gruvbox: {
          orange: '#fe8019',
          yellow: '#fabd2f',
          green: '#b8bb26',
          blue: '#83a598',
          purple: '#d3869b',
          red: '#fb4934',
          link: '#8ec07c',
          bright: '#fbf1c7',
          elevated: '#3c3836',
          hover: '#504945',
        },
      },
      fontFamily: {
        sans: ['IBM Plex Mono', 'monospace'],
        mono: ['IBM Plex Mono', 'monospace'],
      },
      keyframes: {
        indeterminate: {
          '0%': { transform: 'translateX(-100%)', width: '40%' },
          '50%': { transform: 'translateX(60%)', width: '60%' },
          '100%': { transform: 'translateX(200%)', width: '40%' },
        },
      },
      animation: {
        indeterminate: 'indeterminate 1.5s ease-in-out infinite',
      },
    },
  },
  plugins: [
    require('@tailwindcss/forms'),
  ],
}
