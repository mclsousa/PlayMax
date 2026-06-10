import { useCallback, useEffect, useState } from "react";
import * as api from "../lib/api";
import {
  getCachedRecentChannels,
  setCachedRecentChannels,
} from "../lib/homeCache";
import type { RecentChannel } from "../lib/types";

const RECENT_CHANNELS_LIMIT = 10;

export function useRecentChannels(
  profileId: string | null,
  profilesLoading = false,
) {
  const [channels, setChannels] = useState<RecentChannel[]>(() =>
    profileId ? getCachedRecentChannels(profileId) ?? [] : [],
  );
  const [loading, setLoading] = useState(
    () => Boolean(profileId && !getCachedRecentChannels(profileId)),
  );
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    setChannels(profileId ? getCachedRecentChannels(profileId) ?? [] : []);
    setLoading(Boolean(profileId && !getCachedRecentChannels(profileId)));
  }, [profileId]);

  const refresh = useCallback(async (options?: { force?: boolean }) => {
    if (profilesLoading) {
      return;
    }

    if (!profileId) {
      setChannels([]);
      setLoading(false);
      return;
    }

    const cached = options?.force
      ? undefined
      : getCachedRecentChannels(profileId);
    if (cached) {
      setChannels(cached);
      setLoading(false);
      return;
    }

    setLoading(true);
    setError(null);
    try {
      const data = await api.listRecentChannels(
        profileId,
        RECENT_CHANNELS_LIMIT,
      );
      const next = Array.isArray(data) ? data : [];
      setCachedRecentChannels(profileId, next);
      setChannels(next);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }, [profileId, profilesLoading]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  return { channels, loading, error, refresh };
}
