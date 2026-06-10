import type { HistoryEntry, HomeCatalog, RecentChannel } from "./types";

const HOME_CACHE_TTL_MS = 60_000;
/** Bump when home catalog query semantics change so clients refetch. */
const HOME_CATALOG_CACHE_VERSION = 2;
const HOME_CATALOG_STORAGE_PREFIX = "playmax:homeCatalog:";

interface CacheEntry<T> {
  data: T;
  fetchedAt: number;
  version?: number;
}

export interface HomeCatalogCacheHit {
  data: HomeCatalog;
  /** Fresh entries can be used as-is; stale ones should revalidate in background. */
  fresh: boolean;
}

const homeCatalogCache = new Map<string, CacheEntry<HomeCatalog>>();
const continueWatchingCache = new Map<string, CacheEntry<HistoryEntry[]>>();
const recentChannelsCache = new Map<string, CacheEntry<RecentChannel[]>>();

function isFresh(entry: CacheEntry<unknown>): boolean {
  return Date.now() - entry.fetchedAt < HOME_CACHE_TTL_MS;
}

function storageKey(profileId: string): string {
  return `${HOME_CATALOG_STORAGE_PREFIX}${profileId}`;
}

function readPersistedCatalog(
  profileId: string,
): CacheEntry<HomeCatalog> | undefined {
  try {
    const raw = window.localStorage.getItem(storageKey(profileId));
    if (!raw) return undefined;
    const entry = JSON.parse(raw) as CacheEntry<HomeCatalog>;
    if (entry?.version !== HOME_CATALOG_CACHE_VERSION || !entry.data) {
      return undefined;
    }
    return entry;
  } catch {
    return undefined;
  }
}

function persistCatalog(profileId: string, entry: CacheEntry<HomeCatalog>): void {
  try {
    window.localStorage.setItem(storageKey(profileId), JSON.stringify(entry));
  } catch {
    // Quota/serialization issues are non-fatal; memory cache still works.
  }
}

/**
 * Returns the cached catalog (memory first, then persisted from a previous
 * session) along with its freshness, enabling stale-while-revalidate: stale
 * data renders instantly while the caller refetches in background.
 */
export function getCachedHomeCatalogEntry(
  profileId: string,
): HomeCatalogCacheHit | undefined {
  const memory = homeCatalogCache.get(profileId);
  if (memory && memory.version === HOME_CATALOG_CACHE_VERSION) {
    return { data: memory.data, fresh: isFresh(memory) };
  }

  const persisted = readPersistedCatalog(profileId);
  if (persisted) {
    homeCatalogCache.set(profileId, persisted);
    return { data: persisted.data, fresh: isFresh(persisted) };
  }

  return undefined;
}

export function getCachedHomeCatalog(profileId: string): HomeCatalog | undefined {
  const hit = getCachedHomeCatalogEntry(profileId);
  return hit?.fresh ? hit.data : undefined;
}

export function setCachedHomeCatalog(profileId: string, data: HomeCatalog): void {
  const entry: CacheEntry<HomeCatalog> = {
    data,
    fetchedAt: Date.now(),
    version: HOME_CATALOG_CACHE_VERSION,
  };
  homeCatalogCache.set(profileId, entry);
  persistCatalog(profileId, entry);
}

export function getCachedContinueWatching(
  profileId: string,
): HistoryEntry[] | undefined {
  const entry = continueWatchingCache.get(profileId);
  return entry && isFresh(entry) ? entry.data : undefined;
}

export function setCachedContinueWatching(
  profileId: string,
  data: HistoryEntry[],
): void {
  continueWatchingCache.set(profileId, { data, fetchedAt: Date.now() });
}

export function getCachedRecentChannels(
  profileId: string,
): RecentChannel[] | undefined {
  const entry = recentChannelsCache.get(profileId);
  return entry && isFresh(entry) ? entry.data : undefined;
}

export function setCachedRecentChannels(
  profileId: string,
  data: RecentChannel[],
): void {
  recentChannelsCache.set(profileId, { data, fetchedAt: Date.now() });
}

export function clearHomeCache(profileId?: string): void {
  if (!profileId) {
    try {
      const stale: string[] = [];
      for (let i = 0; i < window.localStorage.length; i += 1) {
        const key = window.localStorage.key(i);
        if (key?.startsWith(HOME_CATALOG_STORAGE_PREFIX)) {
          stale.push(key);
        }
      }
      stale.forEach((key) => window.localStorage.removeItem(key));
    } catch {
      // Ignore storage failures during cleanup.
    }
    homeCatalogCache.clear();
    continueWatchingCache.clear();
    recentChannelsCache.clear();
    return;
  }

  homeCatalogCache.delete(profileId);
  continueWatchingCache.delete(profileId);
  recentChannelsCache.delete(profileId);
  try {
    window.localStorage.removeItem(storageKey(profileId));
  } catch {
    // Ignore storage failures during cleanup.
  }
}
