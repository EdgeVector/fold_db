import { defineConfig } from 'vitest/config'
import react from '@vitejs/plugin-react'

export default defineConfig({
  plugins: [react()],
  define: {
    'import.meta.env.VITE_ENABLE_SAMPLES': JSON.stringify('true'),
  },
  test: {
    globals: true,
    environment: 'jsdom',
    setupFiles: ['./src/test/setup.js'],
    css: true,
    testTimeout: 1000, // 1 second timeout - fast but allows for legitimate delays
    define: {
      global: 'globalThis',
    },
    coverage: {
      provider: 'v8',
      reporter: ['text', 'text-summary', 'json', 'json-summary', 'html', 'lcov'],
      reportOnFailure: true,
      all: true,
      include: [
        'src/**/*.{js,jsx,ts,tsx}',
      ],
      exclude: [
        'node_modules/',
        'src/test/**',
        'src/**/*.test.{js,jsx,ts,tsx}',
        'src/**/*.spec.{js,jsx,ts,tsx}',
        'src/**/__tests__/**',
        'src/**/__mocks__/**',
        'src/main.jsx', // Entry point, typically minimal logic
        'src/assets/**',
        'src/styles/**',
        '**/coverage/**',
        '**/dist/**',
        '**/build/**',
        '**/*.config.{js,ts}',
        '**/*.d.ts'
      ],
      // Set 80% minimum coverage thresholds
      thresholds: {
        global: {
          branches: 80,
          functions: 80,
          lines: 80,
          statements: 80
        },
        // Specific thresholds for different file types
        'src/components/**/*.{js,jsx,ts,tsx}': {
          branches: 80,
          functions: 80,
          lines: 80,
          statements: 80
        },
        'src/hooks/**/*.{js,jsx,ts,tsx}': {
          branches: 85,
          functions: 85,
          lines: 85,
          statements: 85
        },
        'src/utils/**/*.{js,jsx,ts,tsx}': {
          branches: 90,
          functions: 90,
          lines: 90,
          statements: 90
        },
        'src/api/**/*.{js,jsx,ts,tsx}': {
          branches: 80,
          functions: 80,
          lines: 80,
          statements: 80
        },
        'src/store/**/*.{js,jsx,ts,tsx}': {
          branches: 85,
          functions: 85,
          lines: 85,
          statements: 85
        }
      },
      // Additional coverage options
      cleanOnRerun: true,
      skipFull: false,
      watermarks: {
        statements: [80, 90],
        functions: [80, 90],
        branches: [80, 90],
        lines: [80, 90]
      }
    }
  }
})