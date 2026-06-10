import { useCallback, useEffect, useRef, useState } from "react";

import * as api from "../lib/api";
import { categoryCacheKeys } from "../lib/categoryCache";

import type { CatalogSort } from "../lib/api";
import type { Series } from "../lib/types";
import { useProfileCategories } from "./useProfileCategories";

const PAGE_SIZE = 100;
const fetchSeriesCategories = (profileId: string) => api.listSeriesCategories(profileId);

export function useSeries(
  profileId: string | null,
  category: string | null,
  search: string,
  sort: CatalogSort = "recent",
) {
  const cacheKey = profileId ? categoryCacheKeys.series(profileId) : null;
  const { categories, categoryCounts, categoriesLoading } = useProfileCategories(
    profileId,
    cacheKey,
    fetchSeriesCategories,
  );

  const [series, setSeries] = useState<Series[]>([]);
  const [total, setTotal] = useState(0);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const requestIdRef = useRef(0);

  const shouldFetchItems =
    category !== null || search.trim().length > 0;
  const effectiveCategory =
    search.trim() && category === null ? "all" : category;

  const refreshItems = useCallback(async () => {
    const requestId = ++requestIdRef.current;

    if (!profileId) {
      setSeries([]);
      setTotal(0);
      return;
    }

    if (!shouldFetchItems) {
      setSeries([]);
      setTotal(0);
      setLoading(false);
      return;
    }

    setLoading(true);
    setError(null);
    try {
      const page = await api.listSeries({
        profileId,
        category:
          effectiveCategory === "all" || effectiveCategory === null
            ? undefined
            : effectiveCategory,
        search: search.trim() || undefined,
        sort,
        offset: 0,
        limit: PAGE_SIZE,
      });

      if (requestId !== requestIdRef.current) return;

      setSeries(page.items);
      setTotal(page.total);
    } catch (err) {
      if (requestId !== requestIdRef.current) return;
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      if (requestId === requestIdRef.current) {
        setLoading(false);
      }
    }
  }, [profileId, effectiveCategory, search, sort, shouldFetchItems]);

  const loadMore = useCallback(async () => {
    if (!profileId || !shouldFetchItems || series.length >= total) return;

    try {
      const page = await api.listSeries({
        profileId,
        category:
          effectiveCategory === "all" || effectiveCategory === null
            ? undefined
            : effectiveCategory,
        search: search.trim() || undefined,
        sort,
        offset: series.length,
        limit: PAGE_SIZE,
      });
      setSeries((prev) => [...prev, ...page.items]);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }, [
    profileId,
    effectiveCategory,
    search,
    series.length,
    total,
    sort,
    shouldFetchItems,
  ]);

  useEffect(() => {
    void refreshItems();
  }, [refreshItems]);

  return {
    series,
    total,
    categories,
    categoryCounts,
    loading,
    categoriesLoading,
    error,
    refresh: refreshItems,
    loadMore,
  };
}
