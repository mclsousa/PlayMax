import { useCallback, useEffect, useState } from "react";

import * as api from "../lib/api";
import {
  getCachedHomeCatalogEntry,
  setCachedHomeCatalog,
} from "../lib/homeCache";
import { readHomeCatalogState } from "../lib/homeCatalogState";

import type { CatalogItem } from "../lib/types";

const CATALOG_LIMIT = 20;
const RELEASE_MOVIES_LIMIT = 40;

function applyCatalog(
  catalog: Awaited<ReturnType<typeof api.getHomeCatalog>>,
  setters: {
    setReleaseMovies: (items: CatalogItem[]) => void;
    setRecentMovies: (items: CatalogItem[]) => void;
    setRecentSeries: (items: CatalogItem[]) => void;
    setUpdatedSeries: (items: CatalogItem[]) => void;
    setReleaseSeries: (items: CatalogItem[]) => void;
    setFeaturedMovies: (items: CatalogItem[]) => void;
    setReleasesReady: (ready: boolean) => void;
  },
) {
  setters.setReleaseMovies(
    Array.isArray(catalog.releaseMovies) ? catalog.releaseMovies : [],
  );
  setters.setReleasesReady(true);
  setters.setRecentMovies(
    Array.isArray(catalog.recentMovies) ? catalog.recentMovies : [],
  );
  setters.setRecentSeries(
    Array.isArray(catalog.recentSeries) ? catalog.recentSeries : [],
  );
  setters.setUpdatedSeries(
    Array.isArray(catalog.updatedSeries) ? catalog.updatedSeries : [],
  );
  setters.setReleaseSeries(
    Array.isArray(catalog.releaseSeries) ? catalog.releaseSeries : [],
  );
  setters.setFeaturedMovies(
    Array.isArray(catalog.featuredMovies) ? catalog.featuredMovies : [],
  );
}

export function useCatalog(
  profileId: string | null,
  profilesLoading = false,
) {
  const initial = readHomeCatalogState(profileId);
  const [recentMovies, setRecentMovies] = useState(initial.recentMovies);
  const [recentSeries, setRecentSeries] = useState(initial.recentSeries);
  const [updatedSeries, setUpdatedSeries] = useState(initial.updatedSeries);
  const [releaseMovies, setReleaseMovies] = useState(initial.releaseMovies);
  const [releaseSeries, setReleaseSeries] = useState(initial.releaseSeries);
  const [featuredMovies, setFeaturedMovies] = useState(initial.featuredMovies);
  const [loading, setLoading] = useState(initial.loading);
  const [releasesReady, setReleasesReady] = useState(initial.releasesReady);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const next = readHomeCatalogState(profileId);
    setRecentMovies(next.recentMovies);
    setRecentSeries(next.recentSeries);
    setUpdatedSeries(next.updatedSeries);
    setReleaseMovies(next.releaseMovies);
    setReleaseSeries(next.releaseSeries);
    setFeaturedMovies(next.featuredMovies);
    setReleasesReady(next.releasesReady);
    setLoading(next.loading);
  }, [profileId]);

  const refresh = useCallback(async (options?: { force?: boolean }) => {
    if (profilesLoading) {
      return;
    }

    if (!profileId) {
      setRecentMovies([]);
      setRecentSeries([]);
      setUpdatedSeries([]);
      setReleaseMovies([]);
      setReleaseSeries([]);
      setFeaturedMovies([]);
      setReleasesReady(false);
      setLoading(false);
      return;
    }

    const setters = {
      setReleaseMovies,
      setRecentMovies,
      setRecentSeries,
      setUpdatedSeries,
      setReleaseSeries,
      setFeaturedMovies,
      setReleasesReady,
    };

    // Stale-while-revalidate: any cached catalog (memory or persisted from a
    // previous session) renders instantly; stale entries refetch in background
    // without flashing loading states.
    const cached = options?.force ? undefined : getCachedHomeCatalogEntry(profileId);
    if (cached) {
      applyCatalog(cached.data, setters);
      setLoading(false);
      if (cached.fresh) {
        return;
      }
    } else {
      setLoading(true);
      setReleasesReady(false);
    }
    setError(null);

    try {
      const catalog = await api.getHomeCatalog(
        profileId,
        RELEASE_MOVIES_LIMIT,
        CATALOG_LIMIT,
      );
      setCachedHomeCatalog(profileId, catalog);
      applyCatalog(catalog, setters);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }, [profileId, profilesLoading]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  return {
    recentMovies,
    recentSeries,
    updatedSeries,
    releaseMovies,
    releaseSeries,
    featuredMovies,
    loading,
    releasesReady,
    error,
    refresh,
  };
}
