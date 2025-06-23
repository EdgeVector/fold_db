import { configureStore } from '@reduxjs/toolkit';
import authReducer from './authSlice';

export const store = configureStore({
  reducer: {
    auth: authReducer,
  },
  middleware: (getDefaultMiddleware) =>
    getDefaultMiddleware({
      serializableCheck: {
        // Ignore these action types in serializability checks
        ignoredActions: ['auth/validatePrivateKey/fulfilled', 'auth/setPrivateKey'],
        // Ignore these field paths in all actions
        ignoredActionsPaths: ['payload.privateKey'],
        // Ignore these paths in the state
        ignoredPaths: ['auth.privateKey'],
      },
    }),
  devTools: true, // Enable Redux DevTools for debugging
});

export type RootState = ReturnType<typeof store.getState>;
export type AppDispatch = typeof store.dispatch;