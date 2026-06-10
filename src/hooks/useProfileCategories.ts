import { useEffect, useState } from "react";

import {
  getCachedCategories,
  setCachedCategories,
  subscribeCategoryCacheInvalidation,
} from "../lib/categoryCache";
import { categoryCountsFromList } from "../lib/categoryCounts";
import type { CategoryCount } from "../lib/types";

export function useProfileCategories(
  profileId: string | null,
  cacheKey: string | null,
  fetchCategories: (id: string) => Promise<CategoryCount[]>,
) {
  const cached = cacheKey ? getCachedCategories(cacheKey) : undefined;
  const [categories, setCategories] = useState<string[]>(
    cached?.map((item) => item.name) ?? [],
  );
  const [categoryCounts, setCategoryCounts] = useState<Record<string, number>>(() =>
    cached ? categoryCountsFromList(cached) : {},
  );
  const [categoriesLoading, setCategoriesLoading] = useState(
    Boolean(profileId) && (cached?.length ?? 0) === 0,
  );

  useEffect(() => {
    if (!profileId || !cacheKey) {
      setCategories([]);
      setCategoryCounts({});
      setCategoriesLoading(false);
      return;
    }

    let cancelled = false;

    const applyCategories = (items: CategoryCount[]) => {
      setCategories(items.map((item) => item.name));
      setCategoryCounts(categoryCountsFromList(items));
      setCategoriesLoading(false);
    };

    const load = (force = false) => {
      if (cancelled) return;

      const cachedItems = force ? undefined : getCachedCategories(cacheKey);
      if (cachedItems?.length) {
        applyCategories(cachedItems);
        return;
      }

      setCategoriesLoading(true);
      void fetchCategories(profileId)
        .then((items) => {
          if (cancelled) return;
          setCachedCategories(cacheKey, items);
          applyCategories(items);
        })
        .catch(() => {
          if (!cancelled) setCategoriesLoading(false);
        });
    };

    load();

    const unsubscribe = subscribeCategoryCacheInvalidation((id) => {
      if (id === profileId) load(true);
    });

    return () => {
      cancelled = true;
      unsubscribe();
    };
  }, [profileId, cacheKey, fetchCategories]);

  return { categories, categoryCounts, categoriesLoading };
}
