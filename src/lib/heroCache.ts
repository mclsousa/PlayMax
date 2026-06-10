import type { HeroSlide } from "../hooks/useReleaseHero";

const HERO_CACHE_TTL_MS = 30 * 60_000;
const HERO_STORAGE_PREFIX = "playmax:heroSlides:";

interface HeroCacheEntry {
  releaseKey: string;
  slides: HeroSlide[];
  fetchedAt: number;
}

const heroCache = new Map<string, HeroCacheEntry>();

function storageKey(profileId: string): string {
  return `${HERO_STORAGE_PREFIX}${profileId}`;
}

function readPersisted(profileId: string): HeroCacheEntry | undefined {
  try {
    const raw = window.localStorage.getItem(storageKey(profileId));
    if (!raw) return undefined;
    const entry = JSON.parse(raw) as HeroCacheEntry;
    if (!entry?.releaseKey || !Array.isArray(entry.slides)) return undefined;
    return entry;
  } catch {
    return undefined;
  }
}

export function getCachedHeroSlides(
  profileId: string,
  releaseKey: string,
): HeroSlide[] | undefined {
  const entry = heroCache.get(profileId);
  if (entry && entry.releaseKey === releaseKey) {
    if (Date.now() - entry.fetchedAt > HERO_CACHE_TTL_MS) {
      heroCache.delete(profileId);
    } else {
      return entry.slides;
    }
  }

  // Persisted slides from a previous session: the releaseKey match guarantees
  // they were built from the same releases, so age doesn't matter — showing
  // them instantly beats re-enriching the same movies over the network.
  const persisted = readPersisted(profileId);
  if (persisted && persisted.releaseKey === releaseKey && persisted.slides.length > 0) {
    heroCache.set(profileId, persisted);
    return persisted.slides;
  }

  return undefined;
}

export function setCachedHeroSlides(
  profileId: string,
  releaseKey: string,
  slides: HeroSlide[],
): void {
  const entry: HeroCacheEntry = {
    releaseKey,
    slides,
    fetchedAt: Date.now(),
  };
  heroCache.set(profileId, entry);
  try {
    window.localStorage.setItem(storageKey(profileId), JSON.stringify(entry));
  } catch {
    // Quota/serialization issues are non-fatal; memory cache still works.
  }
}

export function clearHeroCache(profileId?: string): void {
  if (profileId) {
    heroCache.delete(profileId);
    try {
      window.localStorage.removeItem(storageKey(profileId));
    } catch {
      // Ignore storage failures during cleanup.
    }
    return;
  }
  heroCache.clear();
  try {
    const stale: string[] = [];
    for (let i = 0; i < window.localStorage.length; i += 1) {
      const key = window.localStorage.key(i);
      if (key?.startsWith(HERO_STORAGE_PREFIX)) {
        stale.push(key);
      }
    }
    stale.forEach((key) => window.localStorage.removeItem(key));
  } catch {
    // Ignore storage failures during cleanup.
  }
}
