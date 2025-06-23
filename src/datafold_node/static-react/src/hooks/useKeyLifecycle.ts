import { useEffect } from 'react';

const LOGOUT_EVENT = 'logout';
const SESSION_EXPIRED_EVENT = 'session-expired';

/**
 * Enhanced key lifecycle hook that supports multiple cleanup functions
 * for better separation of concerns and comprehensive cleanup coordination.
 */
export function useKeyLifecycle(cleanupFunctions: (() => void) | (() => void)[]) {
  useEffect(() => {
    const handleCleanup = () => {
      // Support both single function and array of functions
      const functions = Array.isArray(cleanupFunctions) ? cleanupFunctions : [cleanupFunctions];
      functions.forEach(fn => fn());
    };

    window.addEventListener('beforeunload', handleCleanup);
    window.addEventListener(LOGOUT_EVENT, handleCleanup);
    window.addEventListener(SESSION_EXPIRED_EVENT, handleCleanup);

    return () => {
      window.removeEventListener('beforeunload', handleCleanup);
      window.removeEventListener(LOGOUT_EVENT, handleCleanup);
      window.removeEventListener(SESSION_EXPIRED_EVENT, handleCleanup);
    };
  }, [cleanupFunctions]);
}
