import { useCallback, useEffect, useState } from "react";
import * as api from "../lib/api";
import type { SearchResult } from "../lib/types";

export function useSearch(profileId: string | null, query: string) {
  const [results, setResults] = useState<SearchResult[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const search = useCallback(async () => {
    const trimmed = query.trim();
    if (!profileId || !trimmed) {
      setResults([]);
      setError(null);
      return;
    }
    setLoading(true);
    setError(null);
    try {
      const data = await api.searchCatalog(profileId, trimmed);
      setResults(data);
    } catch (err) {
      setResults([]);
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }, [profileId, query]);

  useEffect(() => {
    void search();
  }, [search]);

  return { results, loading, error };
}
