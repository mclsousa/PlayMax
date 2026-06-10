import type { CategoryCount } from "./types";

const cache = new Map<string, CategoryCount[]>();
const invalidationListeners = new Set<(profileId: string) => void>();

export function getCachedCategories(key: string): CategoryCount[] | undefined {
  return cache.get(key);
}

export function setCachedCategories(key: string, categories: CategoryCount[]): void {
  cache.set(key, categories);
}

export function subscribeCategoryCacheInvalidation(
  listener: (profileId: string) => void,
): () => void {
  invalidationListeners.add(listener);
  return () => invalidationListeners.delete(listener);
}

export function clearCategoryCache(profileId?: string): void {
  if (!profileId) {
    cache.clear();
    return;
  }

  for (const key of cache.keys()) {
    if (key.endsWith(`:${profileId}`)) {
      cache.delete(key);
    }
  }

  for (const listener of invalidationListeners) {
    listener(profileId);
  }
}

export const categoryCacheKeys = {
  live: (profileId: string) => `live:${profileId}`,
  movies: (profileId: string) => `movies:${profileId}`,
  series: (profileId: string) => `series:${profileId}`,
};
