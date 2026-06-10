import { useCallback, useEffect, useState } from "react";
import * as api from "../lib/api";
import {
  getCachedContinueWatching,
  setCachedContinueWatching,
} from "../lib/homeCache";
import type { HistoryEntry } from "../lib/types";

const CONTINUE_LIMIT = 20;

export function useContinueWatching(
  profileId: string | null,
  profilesLoading = false,
) {
  const [items, setItems] = useState<HistoryEntry[]>(() =>
    profileId ? getCachedContinueWatching(profileId) ?? [] : [],
  );
  const [loading, setLoading] = useState(
    () => Boolean(profileId && !getCachedContinueWatching(profileId)),
  );
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    setItems(profileId ? getCachedContinueWatching(profileId) ?? [] : []);
    setLoading(Boolean(profileId && !getCachedContinueWatching(profileId)));
  }, [profileId]);

  const refresh = useCallback(async (options?: { force?: boolean }) => {
    if (profilesLoading) {
      return;
    }

    if (!profileId) {
      setItems([]);
      setLoading(false);
      return;
    }

    const cached = options?.force
      ? undefined
      : getCachedContinueWatching(profileId);
    if (cached) {
      setItems(cached);
      setLoading(false);
      return;
    }

    setLoading(true);
    setError(null);
    try {
      const data = await api.listContinueWatching(profileId, CONTINUE_LIMIT);
      const next = Array.isArray(data) ? data : [];
      setCachedContinueWatching(profileId, next);
      setItems(next);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }, [profileId, profilesLoading]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  return { items, loading, error, refresh };
}
