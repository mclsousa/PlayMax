import type { CatalogItem } from "./types";
import { getCachedHomeCatalogEntry } from "./homeCache";

export interface HomeCatalogState {
  recentMovies: CatalogItem[];
  recentSeries: CatalogItem[];
  updatedSeries: CatalogItem[];
  releaseMovies: CatalogItem[];
  releaseSeries: CatalogItem[];
  featuredMovies: CatalogItem[];
  releasesReady: boolean;
  loading: boolean;
}

const EMPTY: HomeCatalogState = {
  recentMovies: [],
  recentSeries: [],
  updatedSeries: [],
  releaseMovies: [],
  releaseSeries: [],
  featuredMovies: [],
  releasesReady: false,
  loading: false,
};

export function readHomeCatalogState(profileId: string | null): HomeCatalogState {
  if (!profileId) return EMPTY;

  // Stale data still renders instantly; useCatalog revalidates in background.
  const cached = getCachedHomeCatalogEntry(profileId)?.data;
  if (!cached) {
    return { ...EMPTY, loading: true };
  }

  return {
    recentMovies: cached.recentMovies ?? [],
    recentSeries: cached.recentSeries ?? [],
    updatedSeries: cached.updatedSeries ?? [],
    releaseMovies: cached.releaseMovies ?? [],
    releaseSeries: cached.releaseSeries ?? [],
    featuredMovies: cached.featuredMovies ?? [],
    releasesReady: true,
    loading: false,
  };
}
