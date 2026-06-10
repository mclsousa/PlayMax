import { useEffect, useRef } from "react";
import { Skeleton } from "../ui/Skeleton";
import { PosterCard } from "./PosterCard";

export interface VodGridItem {
  id: string;
  title: string;
  poster?: string | null;
  backdrop?: string | null;
}

interface VodGridProps {
  items: VodGridItem[];
  total: number;
  loading: boolean;
  emptyMessage?: string;
  favoriteItemType?: "movie" | "series";
  onItemClick: (item: VodGridItem) => void;
  onLoadMore: () => void;
}

const MIN_CARD_WIDTH = 130;

function displayTitle(item: VodGridItem): string {
  const name = item.title?.trim();
  return name || item.id;
}

export function VodGrid({
  items,
  total,
  loading,
  emptyMessage = "Nenhum item encontrado",
  favoriteItemType,
  onItemClick,
  onLoadMore,
}: VodGridProps) {
  const scrollRef = useRef<HTMLDivElement>(null);
  const sentinelRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const root = scrollRef.current;
    const sentinel = sentinelRef.current;
    if (!root || !sentinel || items.length === 0 || items.length >= total) {
      return;
    }

    const observer = new IntersectionObserver(
      (entries) => {
        if (entries.some((entry) => entry.isIntersecting)) {
          onLoadMore();
        }
      },
      { root, rootMargin: "200px" },
    );

    observer.observe(sentinel);
    return () => observer.disconnect();
  }, [items.length, total, onLoadMore]);

  if (loading && items.length === 0) {
    return (
      <div
        className="grid gap-3 px-8 pb-8"
        style={{
          gridTemplateColumns: `repeat(auto-fill, minmax(${MIN_CARD_WIDTH}px, 1fr))`,
        }}
      >
        {Array.from({ length: 18 }).map((_, i) => (
          <div key={i} className="min-w-0">
            <Skeleton className="aspect-[2/3] w-full rounded-xl" />
            <Skeleton className="mt-2 h-3 w-3/4 rounded" />
          </div>
        ))}
      </div>
    );
  }

  if (!loading && items.length === 0) {
    return (
      <div className="flex flex-1 flex-col items-center justify-center gap-2 px-8 py-16 text-center">
        <p className="text-sm text-text-secondary">{emptyMessage}</p>
        <p className="text-xs text-text-muted">Tente outra categoria ou busca</p>
      </div>
    );
  }

  return (
    <div
      ref={scrollRef}
      className="scrollbar-thin min-h-0 flex-1 overflow-y-auto px-8 pb-8"
    >
      <div
        className="grid gap-3"
        style={{
          gridTemplateColumns: `repeat(auto-fill, minmax(${MIN_CARD_WIDTH}px, 1fr))`,
        }}
      >
        {items.map((item, index) => (
          <PosterCard
            key={item.id}
            title={displayTitle(item)}
                    poster={item.poster}
                    backdrop={item.backdrop}
            colorIndex={index}
            favoriteItemType={favoriteItemType}
            favoriteItemId={item.id}
            onClick={() => onItemClick(item)}
          />
        ))}
      </div>
      {items.length < total && (
        <>
          <div ref={sentinelRef} className="h-1" aria-hidden />
          <p className="py-4 text-center text-[11px] text-text-muted">
            Carregando mais...
          </p>
        </>
      )}
    </div>
  );
}
