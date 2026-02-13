import { createContext, useContext, useState } from 'react'
import { resetSchemaClient } from '../api/clients/configuredSchemaClient'
import { BROWSER_CONFIG } from '../constants/config'

// Schema service environment configurations
export const SCHEMA_SERVICE_ENVIRONMENTS = {
  LOCAL: {
    id: 'local',
    name: 'Local',
    description: 'Local development server',
    baseUrl: 'http://127.0.0.1:9002/api' // Local schema service with /api prefix
  },
  DEV: {
    id: 'dev',
    name: 'Development (AWS)',
    description: 'DEV Environment (us-west-2)',
    baseUrl: 'https://cemkk2xzxd.execute-api.us-west-2.amazonaws.com'
  },
  PROD: {
    id: 'prod',
    name: 'Production (AWS)',
    description: 'PROD Environment (us-east-1)',
    baseUrl: 'https://owwjygkso3.execute-api.us-east-1.amazonaws.com'
  }
}

const STORAGE_KEY = BROWSER_CONFIG.STORAGE_KEYS.SCHEMA_SERVICE_ENV

const SchemaServiceConfigContext = createContext({
  environment: SCHEMA_SERVICE_ENVIRONMENTS.LOCAL,
  setEnvironment: () => {},
  getSchemaServiceBaseUrl: () => ''
})

export function SchemaServiceConfigProvider({ children }) {
  const [environment, setEnvironmentState] = useState(() => {
    // Load from localStorage on initialization
    const stored = localStorage.getItem(STORAGE_KEY)
    if (stored) {
      const envConfig = Object.values(SCHEMA_SERVICE_ENVIRONMENTS).find(env => env.id === stored)
      if (envConfig) return envConfig
    }
    return SCHEMA_SERVICE_ENVIRONMENTS.LOCAL
  })

  const setEnvironment = (envId) => {
    const envConfig = Object.values(SCHEMA_SERVICE_ENVIRONMENTS).find(env => env.id === envId)
    if (envConfig) {
      setEnvironmentState(envConfig)
      localStorage.setItem(STORAGE_KEY, envId)
      // Reset the schema client to pick up new configuration
      resetSchemaClient()
    }
  }

  const getSchemaServiceBaseUrl = () => {
    return environment.baseUrl || ''
  }

  return (
    <SchemaServiceConfigContext.Provider value={{ environment, setEnvironment, getSchemaServiceBaseUrl }}>
      {children}
    </SchemaServiceConfigContext.Provider>
  )
}

export function useSchemaServiceConfig() {
  const context = useContext(SchemaServiceConfigContext)
  if (!context) {
    throw new Error('useSchemaServiceConfig must be used within SchemaServiceConfigProvider')
  }
  return context
}

