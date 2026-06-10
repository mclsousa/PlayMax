import { useEffect, useMemo, useState } from "react";
import { useNavigate, useSearchParams } from "react-router-dom";
import { TopBar } from "../components/layout/TopBar";
import { Skeleton } from "../components/ui/Skeleton";
import { useActiveProfile } from "../hooks/useActiveProfile";
import { useDebouncedValue } from "../hooks/useDebouncedValue";
import { useSearch } from "../hooks/useSearch";
import { thumbPosterUrl } from "../lib/posterUtils";
import type { SearchResult } from "../lib/types";
const TYPE_LABELS: Record<SearchResult["itemType"], string> = {
  channel: "Canais ao vivo",
  movie: "Filmes",
  series: "Séries",
};

function resultHref(item: SearchResult): string {
  if (item.itemType === "movie") return `/movies/${item.id}`;
  if (item.itemType === "series") return `/series/${item.id}`;
  return "/live";
}

function SearchSection({
  title,
  items,
  onSelect,
}: {
  title: string;
  items: SearchResult[];
  onSelect: (item: SearchResult) => void;
}) {
  if (items.length === 0) return null;

  return (
    <section className="mb-8">
      <h2 className="mb-3 text-sm font-semibold uppercase tracking-wider text-text-muted">
        {title}
      </h2>
      <ul className="space-y-2">
        {items.map((item) => {
          const poster = thumbPosterUrl(item.poster, null);
          return (
            <li key={`${item.itemType}-${item.id}`}>
              <button
                type="button"
                onClick={() => onSelect(item)}
                className="flex w-full max-w-2xl items-center gap-3 rounded-xl border border-base-700/60 bg-base-850/40 px-4 py-3 text-left transition-colors hover:border-accent/40 hover:bg-base-800"
              >
                <div className="flex h-11 w-11 shrink-0 items-center justify-center overflow-hidden rounded-lg bg-base-800">
                  {poster ? (
                    <img
                      src={poster}
                      alt=""
                      loading="lazy"
                      decoding="async"
                      className="h-full w-full object-cover"
                      referrerPolicy="no-referrer"
                    />
                  ) : (
                    <span className="text-[10px] font-bold text-text-muted">
                      {item.itemType === "channel" ? "TV" : "VOD"}
                    </span>
                  )}
                </div>
                <div className="min-w-0 flex-1">
                  <p className="truncate text-sm font-medium">{item.name}</p>
                  {item.category && (
                    <p className="truncate text-xs text-text-muted">{item.category}</p>
                  )}
                </div>
              </button>
            </li>
          );
        })}
      </ul>
    </section>
  );
}

export function SearchPage() {
  const navigate = useNavigate();
  const [searchParams, setParams] = useSearchParams();
  const initialQuery = searchParams.get("q") ?? "";
  const [query, setQuery] = useState(initialQuery);
  const debouncedQuery = useDebouncedValue(query, 250);
  const { profileId } = useActiveProfile();
  const { results, loading, error } = useSearch(profileId, debouncedQuery);

  useEffect(() => {
    const trimmed = debouncedQuery.trim();
    setParams(
      (prev) => {
        const current = prev.get("q") ?? "";
        if (trimmed === current) return prev;
        return trimmed ? new URLSearchParams({ q: trimmed }) : new URLSearchParams();
      },
      { replace: true },
    );
  }, [debouncedQuery, setParams]);

  const activeQuery = debouncedQuery.trim();
  const isTyping = query.trim() !== activeQuery;

  const grouped = useMemo(() => {
    const channels = results.filter((r) => r.itemType === "channel");
    const movies = results.filter((r) => r.itemType === "movie");
    const series = results.filter((r) => r.itemType === "series");
    return { channels, movies, series };
  }, [results]);

  const handleSelect = (item: SearchResult) => {
    if (item.itemType === "channel") {
      navigate("/live", {
        state: { channelId: item.id, group: item.category ?? undefined },
      });
      return;
    }
    const path = resultHref(item);
    navigate(path, {
      state: {
        preview: {
          id: item.id,
          name: item.name,
          poster: item.poster ?? null,
          backdrop: null,
        },
      },
    });
  };

  return (
    <div className="flex h-full flex-col overflow-hidden app-bg">
      <TopBar search={query} onSearchChange={setQuery} title="Buscar" />
      <div className="scrollbar-thin flex-1 overflow-y-auto bg-base-950 px-8 py-8">
        {!query.trim() ? (
          <p className="text-sm text-text-secondary">
            Digite um termo para buscar canais, filmes e séries.
          </p>
        ) : loading || isTyping ? (
          <div className="space-y-3">
            {Array.from({ length: 6 }).map((_, i) => (
              <Skeleton key={i} className="h-14 w-full max-w-2xl rounded-xl" />
            ))}
          </div>
        ) : error ? (
          <p className="text-sm text-red-400">{error}</p>
        ) : results.length === 0 ? (
          <p className="text-sm text-text-secondary">
            Nenhum resultado para &quot;{activeQuery}&quot;.
          </p>
        ) : (
          <>
            <SearchSection
              title={TYPE_LABELS.channel}
              items={grouped.channels}
              onSelect={handleSelect}
            />
            <SearchSection
              title={TYPE_LABELS.movie}
              items={grouped.movies}
              onSelect={handleSelect}
            />
            <SearchSection
              title={TYPE_LABELS.series}
              items={grouped.series}
              onSelect={handleSelect}
            />
          </>
        )}
      </div>
    </div>
  );
}
