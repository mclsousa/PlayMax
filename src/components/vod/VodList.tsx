import { useVirtualizer } from "@tanstack/react-virtual";
import { useEffect, useRef } from "react";
import { Skeleton } from "../ui/Skeleton";
import { VodListCard } from "./VodListCard";

export interface VodListItem {
  id: string;
  title: string;
  poster?: string | null;
  backdrop?: string | null;
}

interface VodListProps {
  items: VodListItem[];
  total: number;
  loading: boolean;
  activeItemId: string | null;
  emptyMessage?: string;
  onItemClick: (item: VodListItem) => void;
  onLoadMore: () => void;
}

export function VodList({
  items,
  total,
  loading,
  activeItemId,
  emptyMessage = "Nenhum item encontrado",
  onItemClick,
  onLoadMore,
}: VodListProps) {
  const parentRef = useRef<HTMLDivElement>(null);

  const virtualizer = useVirtualizer({
    count: items.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 72,
    overscan: 8,
  });

  const virtualItems = virtualizer.getVirtualItems();

  useEffect(() => {
    const last = virtualItems[virtualItems.length - 1];
    if (!last) return;
    if (last.index >= items.length - 10 && items.length < total) {
      onLoadMore();
    }
  }, [virtualItems, items.length, total, onLoadMore]);

  if (loading && items.length === 0) {
    return (
      <div className="flex flex-col gap-2 px-3 py-2">
        {Array.from({ length: 8 }).map((_, i) => (
          <Skeleton key={i} className="h-[52px] w-full rounded-xl" />
        ))}
      </div>
    );
  }

  if (!loading && items.length === 0) {
    return (
      <div className="flex flex-1 flex-col items-center justify-center gap-2 px-6 py-12 text-center">
        <div className="flex h-12 w-12 items-center justify-center rounded-full bg-base-800 text-text-muted">
          <svg
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.5"
            className="h-6 w-6"
          >
            <rect x="2" y="4" width="20" height="16" rx="2" />
            <path d="M8 10h8M8 14h5" strokeLinecap="round" />
          </svg>
        </div>
        <p className="text-sm text-text-secondary">{emptyMessage}</p>
        <p className="text-xs text-text-muted">Tente outra categoria ou busca</p>
      </div>
    );
  }

  return (
    <div ref={parentRef} className="panel-scrollbar min-h-0 flex-1 overflow-y-auto px-3 py-2">
      <div
        style={{
          height: `${virtualizer.getTotalSize()}px`,
          width: "100%",
          position: "relative",
        }}
      >
        {virtualItems.map((virtualItem) => {
          const item = items[virtualItem.index];
          return (
            <div
              key={item.id}
              style={{
                position: "absolute",
                top: 0,
                left: 0,
                width: "100%",
                height: `${virtualItem.size}px`,
                transform: `translateY(${virtualItem.start}px)`,
              }}
              className="pb-1.5"
            >
              <VodListCard
                title={item.title}
                poster={item.poster}
                backdrop={item.backdrop}
                active={item.id === activeItemId}
                onClick={() => onItemClick(item)}
              />
            </div>
          );
        })}
      </div>
      {items.length < total && (
        <p className="py-3 text-center text-[11px] text-text-muted">
          Carregando mais...
        </p>
      )}
    </div>
  );
}
