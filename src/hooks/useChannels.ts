import { useCallback, useEffect, useRef, useState } from "react";
import * as api from "../lib/api";
import { categoryCacheKeys } from "../lib/categoryCache";
import type { Channel } from "../lib/types";
import { useProfileCategories } from "./useProfileCategories";

const PAGE_SIZE = 100;
const fetchLiveGroups = (profileId: string) => api.listGroups(profileId, "live");

export function useChannels(
  profileId: string | null,
  group: string,
  search: string,
  enabled = true,
) {
  const cacheKey = profileId ? categoryCacheKeys.live(profileId) : null;
  const {
    categories: groups,
    categoryCounts,
    categoriesLoading: groupsLoading,
  } = useProfileCategories(profileId, cacheKey, fetchLiveGroups);

  const [channels, setChannels] = useState<Channel[]>([]);
  const [total, setTotal] = useState(0);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const requestIdRef = useRef(0);

  const refreshChannels = useCallback(async () => {
    const requestId = ++requestIdRef.current;

    if (!profileId) {
      setChannels([]);
      setTotal(0);
      return;
    }

    if (!enabled) {
      setChannels([]);
      setTotal(0);
      setLoading(false);
      return;
    }

    setLoading(true);
    setError(null);
    try {
      const page = await api.listChannels({
        profileId,
        group: group === "all" ? undefined : group,
        search: search.trim() || undefined,
        offset: 0,
        limit: PAGE_SIZE,
      });

      if (requestId !== requestIdRef.current) return;

      setChannels(page.items);
      setTotal(page.total);
    } catch (err) {
      if (requestId !== requestIdRef.current) return;
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      if (requestId === requestIdRef.current) {
        setLoading(false);
      }
    }
  }, [profileId, group, search, enabled]);

  const loadMore = useCallback(async () => {
    if (!profileId || !enabled || channels.length >= total) return;
    try {
      const page = await api.listChannels({
        profileId,
        group: group === "all" ? undefined : group,
        search: search.trim() || undefined,
        offset: channels.length,
        limit: PAGE_SIZE,
      });
      setChannels((prev) => [...prev, ...page.items]);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }, [profileId, group, search, channels.length, total, enabled]);

  useEffect(() => {
    void refreshChannels();
  }, [refreshChannels]);

  return {
    channels,
    total,
    groups,
    categoryCounts,
    loading,
    groupsLoading,
    error,
    refresh: refreshChannels,
    loadMore,
  };
}
