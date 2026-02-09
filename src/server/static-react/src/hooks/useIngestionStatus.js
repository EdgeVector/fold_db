import { useState, useEffect, useCallback } from "react";
import { ingestionClient } from "../api/clients";

/**
 * Shared hook for fetching and managing ingestion status.
 * Replaces duplicated fetchIngestionStatus logic across
 * SmartFolderTab, FileUploadTab, and IngestionTab.
 */
export function useIngestionStatus() {
  const [ingestionStatus, setIngestionStatus] = useState(null);

  const fetchIngestionStatus = useCallback(async () => {
    try {
      const response = await ingestionClient.getStatus();
      if (response.success) {
        setIngestionStatus(response.data);
      }
    } catch (error) {
      console.error("Failed to fetch ingestion status:", error);
    }
  }, []);

  useEffect(() => {
    fetchIngestionStatus();
  }, [fetchIngestionStatus]);

  return { ingestionStatus, refetchIngestionStatus: fetchIngestionStatus };
}
