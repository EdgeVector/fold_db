import React from 'react'
import { Provider } from 'react-redux'
import { store } from '../store/store'
import { SchemaServiceConfigProvider } from '../contexts/SchemaServiceConfigContext'

export const FoldDbProvider = ({ children, store: customStore }) => {
  return (
    <Provider store={customStore || store}>
      <SchemaServiceConfigProvider>
        {children}
      </SchemaServiceConfigProvider>
    </Provider>
  )
}
