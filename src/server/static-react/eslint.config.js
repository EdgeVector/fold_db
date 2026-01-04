import js from '@eslint/js'
import reactHooks from 'eslint-plugin-react-hooks'
import reactRefresh from 'eslint-plugin-react-refresh'
import tseslint from '@typescript-eslint/eslint-plugin'
import tsparser from '@typescript-eslint/parser'

export default [
  { ignores: ['dist', 'node_modules', 'src/types/openapi.ts'] },
  // JavaScript and JSX files
  {
    files: ['**/*.{js,jsx}'],
    languageOptions: {
      ecmaVersion: 2020,
      globals: {
        // Browser globals
        window: 'readonly',
        document: 'readonly',
        console: 'readonly',
        localStorage: 'readonly',
        fetch: 'readonly',
        navigator: 'readonly',
        EventSource: 'readonly',
        setTimeout: 'readonly',
        setInterval: 'readonly',
        clearInterval: 'readonly',
        clearTimeout: 'readonly',
        Element: 'readonly',
        TextEncoder: 'readonly',
        crypto: 'readonly',
        // Node.js globals
        global: 'readonly',
        require: 'readonly',
        __dirname: 'readonly',
        module: 'readonly',
        exports: 'readonly',
        process: 'readonly',
        URL: 'readonly'
      },
      parserOptions: {
        ecmaVersion: 'latest',
        ecmaFeatures: { jsx: true },
        sourceType: 'module',
      },
    },
    plugins: {
      'react-hooks': reactHooks,
      'react-refresh': reactRefresh,
    },
    rules: {
      ...js.configs.recommended.rules,
      ...reactHooks.configs.recommended.rules,
      'no-unused-vars': ['error', {
        varsIgnorePattern: '^[A-Z_]|^_',
        argsIgnorePattern: '^_'
      }],
      'react-refresh/only-export-components': [
        'warn',
        { allowConstantExport: true },
      ],
      // PREVENT UI REGRESSIONS: No hardcoded API URLs (except in endpoints definition file)
      'no-restricted-syntax': [
        'error',
        {
          selector: "Literal[value='/api/mutation']",
          message: "🚫 REGRESSION PREVENTION: Use API_ENDPOINTS.MUTATION instead of hardcoded '/api/mutation'"
        },
        {
          selector: "Literal[value='/api/query']",
          message: "🚫 REGRESSION PREVENTION: Use API_ENDPOINTS.QUERY instead of hardcoded '/api/query'"
        },
        {
          selector: "Literal[value='/api/schema']",
          message: "🚫 REGRESSION PREVENTION: Use API_ENDPOINTS.SCHEMAS_BASE or SCHEMA_BY_NAME(name)"
        },
        {
          selector: "Literal[value='/api/data/mutate']",
          message: "🚫 REGRESSION PREVENTION: Invalid endpoint! Use API_ENDPOINTS.MUTATION instead"
        }
      ],
      // Block axios usage in app code
      'no-restricted-imports': [
        'error',
        {
          paths: [
            {
              name: 'axios',
              message: '🚫 Use unified ApiClient and domain clients instead of axios.'
            }
          ]
        }
      ],
      // Encourage using API clients instead of direct fetch
      'no-restricted-globals': [
        'warn',
        {
          name: 'fetch',
          message: '⚠️ Consider using domain clients (schemaClient, mutationClient, etc.) instead of direct fetch() calls'
        }
      ]
    },
  },
  // TypeScript and TSX files
  {
    files: ['**/*.{ts,tsx}'],
    languageOptions: {
      parser: tsparser,
      parserOptions: {
        ecmaVersion: 'latest',
        sourceType: 'module',
        ecmaFeatures: { jsx: true },
      },
      globals: {
        // Browser globals
        window: 'readonly',
        document: 'readonly',
        console: 'readonly',
        localStorage: 'readonly',
        fetch: 'readonly',
        navigator: 'readonly',
        EventSource: 'readonly',
        setTimeout: 'readonly',
        setInterval: 'readonly',
        clearInterval: 'readonly',
        clearTimeout: 'readonly',
        Element: 'readonly',
        TextEncoder: 'readonly',
        crypto: 'readonly',
        // Node.js globals
        global: 'readonly',
        require: 'readonly',
        __dirname: 'readonly',
        module: 'readonly',
        exports: 'readonly',
        process: 'readonly',
        URL: 'readonly'
      },
    },
    plugins: {
      '@typescript-eslint': tseslint,
      'react-hooks': reactHooks,
      'react-refresh': reactRefresh,
    },
    rules: {
      // TypeScript recommended rules
      ...tseslint.configs.recommended.rules,
      ...reactHooks.configs.recommended.rules,
      // Override no-unused-vars for TypeScript
      'no-unused-vars': 'off',
      '@typescript-eslint/no-unused-vars': ['error', {
        varsIgnorePattern: '^[A-Z_]|^_',
        argsIgnorePattern: '^_'
      }],
      'react-refresh/only-export-components': [
        'warn',
        { allowConstantExport: true },
      ],
      // Allow explicit any in API boundary types in TS for now
      '@typescript-eslint/no-explicit-any': ['warn'],
      // PREVENT UI REGRESSIONS: No hardcoded API URLs
      'no-restricted-syntax': [
        'error',
        {
          selector: "Literal[value='/api/mutation']",
          message: "🚫 REGRESSION PREVENTION: Use API_ENDPOINTS.MUTATION instead of hardcoded '/api/mutation'"
        },
        {
          selector: "Literal[value='/api/query']",
          message: "🚫 REGRESSION PREVENTION: Use API_ENDPOINTS.QUERY instead of hardcoded '/api/query'"
        },
        {
          selector: "Literal[value='/api/schema']",
          message: "🚫 REGRESSION PREVENTION: Use API_ENDPOINTS.SCHEMAS_BASE or SCHEMA_BY_NAME(name)"
        },
        {
          selector: "Literal[value='/api/data/mutate']",
          message: "🚫 REGRESSION PREVENTION: Invalid endpoint! Use API_ENDPOINTS.MUTATION instead"
        }
      ],
      // Block axios usage in app code
      'no-restricted-imports': [
        'error',
        {
          paths: [
            {
              name: 'axios',
              message: '🚫 Use unified ApiClient and domain clients instead of axios.'
            }
          ]
        }
      ],
      // Encourage using API clients instead of direct fetch
      'no-restricted-globals': [
        'warn',
        {
          name: 'fetch',
          message: '⚠️ Consider using domain clients (schemaClient, mutationClient, etc.) instead of direct fetch() calls'
        }
      ]
    },
  },
  // Test files configuration
  {
    files: ['**/*.{test,spec}.{js,jsx,ts,tsx}', '**/test/**/*.{js,jsx,ts,tsx}', '**/tests/**/*.{js,jsx,ts,tsx}'],
    plugins: {
      '@typescript-eslint': tseslint,
      'react-hooks': reactHooks,
      'react-refresh': reactRefresh,
    },
    languageOptions: {
      globals: {
        // Vitest globals
        describe: 'readonly',
        it: 'readonly',
        test: 'readonly',
        expect: 'readonly',
        beforeAll: 'readonly',
        beforeEach: 'readonly',
        afterAll: 'readonly',
        afterEach: 'readonly',
        vi: 'readonly',
        vitest: 'readonly',
        // Jest compatibility globals
        jest: 'readonly',
        // Browser globals
        window: 'readonly',
        document: 'readonly',
        console: 'readonly',
        localStorage: 'readonly',
        fetch: 'readonly',
        navigator: 'readonly',
        EventSource: 'readonly',
        setTimeout: 'readonly',
        setInterval: 'readonly',
        clearInterval: 'readonly',
        clearTimeout: 'readonly',
        Element: 'readonly',
        performance: 'readonly',
        TextEncoder: 'readonly',
        crypto: 'readonly',
        // Web API globals for MSW tests
        Response: 'readonly',
        Request: 'readonly',
        Headers: 'readonly',
        // Test constants
        TEST_TIMEOUT_MS: 'readonly',
        // React testing globals
        React: 'readonly',
        useState: 'readonly',
        useEffect: 'readonly',
        useContext: 'readonly',
        useCallback: 'readonly',
        useMemo: 'readonly',
        useRef: 'readonly',
        useReducer: 'readonly',
        // Custom hooks for testing
        useApprovedSchemas: 'readonly',
        useRangeSchema: 'readonly',
        useFormValidation: 'readonly',
        // Test utilities
        approveSchema: 'readonly',
      },
    },
    rules: {
      'no-restricted-globals': 'off', // Allow fetch in test files
      'no-restricted-syntax': 'off', // Allow hardcoded endpoints in test files
      'no-unused-vars': ['warn', { varsIgnorePattern: '^[A-Z_]|^_', argsIgnorePattern: '^_' }],
      '@typescript-eslint/no-unused-vars': ['warn', { varsIgnorePattern: '^[A-Z_]|^_', argsIgnorePattern: '^_' }],
      '@typescript-eslint/no-explicit-any': 'off'
    },
  },
  // Node-based scripts configuration
  {
    files: ['scripts/**/*.{js,ts}'],
    plugins: {
      '@typescript-eslint': tseslint,
    },
    languageOptions: {
      globals: {
        console: 'readonly',
        process: 'readonly',
        __dirname: 'readonly',
        module: 'readonly',
        require: 'readonly',
        URL: 'readonly'
      },
    },
    rules: {
      'no-unused-vars': ['warn', { varsIgnorePattern: '^[A-Z_]|^_', argsIgnorePattern: '^_' }],
      'no-useless-escape': 'off'
    }
  },
  // Main application files - add missing globals
  {
    files: ['**/*.{js,jsx}', '!**/*.{test,spec}.{js,jsx}', '!**/test/**/*.{js,jsx}', '!**/tests/**/*.{js,jsx}'],
    languageOptions: {
      globals: {
        // Browser globals
        window: 'readonly',
        document: 'readonly',
        console: 'readonly',
        localStorage: 'readonly',
        fetch: 'readonly',
        navigator: 'readonly',
        EventSource: 'readonly',
        setTimeout: 'readonly',
        setInterval: 'readonly',
        clearInterval: 'readonly',
        clearTimeout: 'readonly',
        Element: 'readonly',
        performance: 'readonly',
        TextEncoder: 'readonly',
        crypto: 'readonly',
        // Node.js globals
        global: 'readonly',
        require: 'readonly',
        __dirname: 'readonly',
        module: 'readonly',
        exports: 'readonly',
        process: 'readonly'
      },
    },
  },
  // Main application files - TypeScript - add missing globals
  {
    files: ['**/*.{ts,tsx}', '!**/*.{test,spec}.{ts,tsx}', '!**/test/**/*.{ts,tsx}', '!**/tests/**/*.{ts,tsx}'],
    languageOptions: {
      globals: {
        // Browser globals
        window: 'readonly',
        document: 'readonly',
        console: 'readonly',
        localStorage: 'readonly',
        fetch: 'readonly',
        navigator: 'readonly',
        EventSource: 'readonly',
        setTimeout: 'readonly',
        setInterval: 'readonly',
        clearInterval: 'readonly',
        clearTimeout: 'readonly',
        Element: 'readonly',
        performance: 'readonly',
        TextEncoder: 'readonly',
        crypto: 'readonly',
        // Node.js globals
        global: 'readonly',
        require: 'readonly',
        __dirname: 'readonly',
        module: 'readonly',
        exports: 'readonly',
        process: 'readonly'
      },
    },
  },
  // Override for API endpoints definition file and validation tests
  {
    files: ['**/api/endpoints.ts', '**/api/endpoints.js', '**/validation/**/*.test.js', '**/validation/**/*.test.ts'],
    rules: {
      'no-restricted-syntax': 'off', // Allow hardcoded endpoints in source of truth and validation files
    },
  },
]
