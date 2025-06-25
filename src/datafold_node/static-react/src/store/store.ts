import { configureStore } from '@reduxjs/toolkit';
import authReducer from './authSlice';
import schemaReducer from './schemaSlice';

export const store = configureStore({
  reducer: {
    auth: authReducer,
    schemas: schemaReducer,
  },
  middleware: (getDefaultMiddleware) =>
    getDefaultMiddleware({
      serializableCheck: {
        // Ignore these action types in serializability checks
        ignoredActions: [
          'auth/validatePrivateKey/fulfilled',
          'auth/setPrivateKey',
          // Schema async thunk actions that may contain non-serializable data
          'schemas/fetchSchemas/fulfilled',
          'schemas/approveSchema/fulfilled',
          'schemas/blockSchema/fulfilled',
          'schemas/unloadSchema/fulfilled',
          'schemas/loadSchema/fulfilled'
        ],
        // Ignore these field paths in all actions
        ignoredActionsPaths: ['payload.privateKey', 'payload.schemas.definition'],
        // Ignore these paths in the state
        ignoredPaths: ['auth.privateKey', 'schemas.schemas.*.definition'],
      },
    }),
  devTools: true, // Enable Redux DevTools for debugging
});

export type RootState = ReturnType<typeof store.getState>;
export type AppDispatch = typeof store.dispatch;