import React from 'react'
import { Provider } from 'react-redux'
import { store } from '../store/store'

export const FoldDbProvider = ({ children, store: customStore }) => {
  return (
    <Provider store={customStore || store}>
      {children}
    </Provider>
  )
}
