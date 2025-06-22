import js from '@eslint/js'
import reactHooks from 'eslint-plugin-react-hooks'
import reactRefresh from 'eslint-plugin-react-refresh'

export default [
  { ignores: ['dist'] },
  {
    files: ['**/*.{js,jsx}'],
    languageOptions: {
      ecmaVersion: 2020,
      globals: {
        window: 'readonly',
        document: 'readonly',
        console: 'readonly',
        fetch: 'readonly'
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
      'no-unused-vars': ['error', { varsIgnorePattern: '^[A-Z_]' }],
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
]
