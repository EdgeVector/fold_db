import js from '@eslint/js'
import reactHooks from 'eslint-plugin-react-hooks'
import reactRefresh from 'eslint-plugin-react-refresh'
import tseslint from '@typescript-eslint/eslint-plugin'
import tsparser from '@typescript-eslint/parser'

export default [
  { ignores: ['dist', 'node_modules'] },
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
        fetch: 'readonly',
        navigator: 'readonly',
        EventSource: 'readonly',
        setTimeout: 'readonly',
        setInterval: 'readonly',
        clearInterval: 'readonly',
        Element: 'readonly',
        // Node.js globals
        global: 'readonly',
        require: 'readonly',
        __dirname: 'readonly',
        module: 'readonly',
        exports: 'readonly',
        process: 'readonly'
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
          message: "🚫 REGRESSION PREVENTION: Use API_ENDPOINTS.SCHEMA instead of hardcoded '/api/schema'"
        },
        {
          selector: "Literal[value='/api/data/mutate']",
          message: "🚫 REGRESSION PREVENTION: Invalid endpoint! Use API_ENDPOINTS.MUTATION instead"
        }
      ],
      // Encourage using API clients instead of direct fetch
      'no-restricted-globals': [
        'warn',
        {
          name: 'fetch',
          message: '⚠️ Consider using MutationClient, SchemaClient, or SecurityClient instead of direct fetch() calls'
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
        fetch: 'readonly',
        navigator: 'readonly',
        EventSource: 'readonly',
        setTimeout: 'readonly',
        setInterval: 'readonly',
        clearInterval: 'readonly',
        Element: 'readonly',
        // Node.js globals
        global: 'readonly',
        require: 'readonly',
        __dirname: 'readonly',
        module: 'readonly',
        exports: 'readonly',
        process: 'readonly'
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
          message: "🚫 REGRESSION PREVENTION: Use API_ENDPOINTS.SCHEMA instead of hardcoded '/api/schema'"
        },
        {
          selector: "Literal[value='/api/data/mutate']",
          message: "🚫 REGRESSION PREVENTION: Invalid endpoint! Use API_ENDPOINTS.MUTATION instead"
        }
      ],
      // Encourage using API clients instead of direct fetch
      'no-restricted-globals': [
        'warn',
        {
          name: 'fetch',
          message: '⚠️ Consider using MutationClient, SchemaClient, or SecurityClient instead of direct fetch() calls'
        }
      ]
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
