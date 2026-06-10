import { useEffect, useMemo, useState } from "react";

import { getChannelEpg } from "../lib/api";
import type { ChannelEpg } from "../lib/types";

const CONCURRENCY = 4;
const CLIENT_CACHE_TTL_MS = 5 * 60_000;

interface CacheEntry {
  epg: ChannelEpg;
  fetchedAt: number;
}

const clientCache = new Map<string, CacheEntry>();

function cacheKey(profileId: string, channelId: string): string {
  return `${profileId}:${channelId}`;
}

function readCachedEpg(profileId: string, channelId: string): ChannelEpg | undefined {
  const entry = clientCache.get(cacheKey(profileId, channelId));
  if (!entry) return undefined;
  if (Date.now() - entry.fetchedAt > CLIENT_CACHE_TTL_MS) {
    clientCache.delete(cacheKey(profileId, channelId));
    return undefined;
  }
  return entry.epg;
}

function writeCachedEpg(profileId: string, channelId: string, epg: ChannelEpg): void {
  clientCache.set(cacheKey(profileId, channelId), {
    epg,
    fetchedAt: Date.now(),
  });
}

const unavailableEpg: ChannelEpg = {
  available: false,
  unavailableReason: "EPG indisponível",
  programs: [],
};

export function useChannelsEpg(
  profileId: string | null,
  channelIds: string[],
  enabled = true,
  limit = 4,
) {
  const [epgByChannel, setEpgByChannel] = useState<Map<string, ChannelEpg>>(
    () => new Map(),
  );
  const [loadingIds, setLoadingIds] = useState<Set<string>>(() => new Set());

  const channelKey = useMemo(
    () => [...new Set(channelIds)].sort().join("|"),
    [channelIds],
  );

  useEffect(() => {
    if (!enabled || !profileId || channelIds.length === 0) {
      setLoadingIds(new Set());
      return;
    }

    let cancelled = false;
    const uniqueIds = [...new Set(channelIds)];

    const load = async () => {
      const pending: string[] = [];

      for (const channelId of uniqueIds) {
        const cached = readCachedEpg(profileId, channelId);
        if (cached) {
          setEpgByChannel((current) => {
            if (current.get(channelId) === cached) return current;
            const next = new Map(current);
            next.set(channelId, cached);
            return next;
          });
        } else {
          pending.push(channelId);
        }
      }

      for (let index = 0; index < pending.length; index += CONCURRENCY) {
        if (cancelled) break;

        const batch = pending.slice(index, index + CONCURRENCY);
        setLoadingIds((current) => new Set([...current, ...batch]));

        await Promise.all(
          batch.map(async (channelId) => {
            try {
              const epg = await getChannelEpg(profileId, channelId, limit);
              writeCachedEpg(profileId, channelId, epg);
              if (!cancelled) {
                setEpgByChannel((current) => {
                  const next = new Map(current);
                  next.set(channelId, epg);
                  return next;
                });
              }
            } catch {
              if (!cancelled) {
                setEpgByChannel((current) => {
                  const next = new Map(current);
                  next.set(channelId, unavailableEpg);
                  return next;
                });
              }
            } finally {
              if (!cancelled) {
                setLoadingIds((current) => {
                  const next = new Set(current);
                  next.delete(channelId);
                  return next;
                });
              }
            }
          }),
        );
      }
    };

    void load();

    return () => {
      cancelled = true;
    };
  }, [profileId, channelKey, enabled, limit, channelIds.length]);

  return { epgByChannel, loadingIds };
}
