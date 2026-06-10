import { useCallback, useMemo } from "react";

import { useCarouselScroll } from "../../hooks/useDragScroll";
import { CarouselNavRow } from "../ui/CarouselArrows";
import { Skeleton } from "../ui/Skeleton";

type CategoryOrientation = "vertical" | "horizontal";

const HORIZONTAL_SKELETON_COUNT = 8;
const VERTICAL_SKELETON_COUNT = 6;

interface CategoryCardsProps {
  groups: string[];
  counts?: Record<string, number>;
  onSelect: (value: string) => void;
  className?: string;
  orientation?: CategoryOrientation;
  selectedId?: string | null;
  loading?: boolean;
  lockedIds?: ReadonlySet<string>;
}

export function formatGroupLabel(value: string): string {
  if (value === "all") return "Todos";
  return value
    .toLowerCase()
    .split(/\s+/)
    .map((word) => word.charAt(0).toUpperCase() + word.slice(1))
    .join(" ");
}

const GRADIENTS = [
  "from-violet-600/30 to-indigo-900/40",
  "from-purple-600/30 to-fuchsia-900/40",
  "from-blue-600/30 to-slate-900/40",
  "from-emerald-600/25 to-teal-900/40",
  "from-rose-600/25 to-red-900/40",
  "from-amber-600/25 to-orange-900/40",
  "from-cyan-600/25 to-blue-900/40",
  "from-pink-600/25 to-purple-900/40",
];

function gradientForId(id: string): string {
  let hash = 0;
  for (let i = 0; i < id.length; i++) {
    hash = (hash * 31 + id.charCodeAt(i)) >>> 0;
  }
  return GRADIENTS[hash % GRADIENTS.length];
}

function CategoryIcon({ id, compact = false }: { id: string; compact?: boolean }) {
  const iconClass = compact ? "h-4 w-4 opacity-80" : "h-5 w-5 opacity-80";
  if (id === "all") {
    return (
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.75" className={iconClass}>
        <rect x="3" y="3" width="7" height="7" rx="1.5" />
        <rect x="14" y="3" width="7" height="7" rx="1.5" />
        <rect x="3" y="14" width="7" height="7" rx="1.5" />
        <rect x="14" y="14" width="7" height="7" rx="1.5" />
      </svg>
    );
  }
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.75" className={iconClass}>
      <path d="M4 6h16M4 12h10M4 18h14" strokeLinecap="round" />
    </svg>
  );
}

function formatContentCount(count: number | undefined): string | null {
  if (count == null) return null;
  return count.toLocaleString("pt-BR");
}

function CategoryCardButton({
  id,
  label,
  count,
  onSelect,
  orientation,
  selected,
  locked = false,
}: {
  id: string;
  label: string;
  count?: number;
  onSelect: (value: string) => void;
  orientation: CategoryOrientation;
  selected: boolean;
  locked?: boolean;
}) {
  const gradient = gradientForId(id);
  const isHorizontal = orientation === "horizontal";
  const countLabel = formatContentCount(count);
  const sizeClass = isHorizontal
    ? "h-[84px] min-w-[116px] w-[116px] shrink-0 snap-start"
    : "h-[88px] w-full";
  const layoutClass = isHorizontal
    ? "gap-1 justify-between p-2.5 ring-inset"
    : "justify-between p-2.5";

  return (
    <button
      type="button"
      role="option"
      aria-selected={selected}
      onPointerDown={(event) => event.stopPropagation()}
      onClick={() => onSelect(id)}
      className={`group relative flex ${sizeClass} ${layoutClass} flex-col overflow-hidden rounded-xl text-left ring-1 transition-all duration-250 ease-out hover:-translate-y-0.5 hover:scale-[1.03] hover:shadow-lg active:scale-[0.99] ${
        selected
          ? "ring-accent/80 shadow-md shadow-accent/15 hover:shadow-accent/25"
          : "ring-base-700/60 hover:ring-accent/45 hover:shadow-accent/10"
      }`}
    >
      <div
        className={`absolute inset-0 bg-gradient-to-br ${gradient} opacity-70 transition-all duration-250 group-hover:opacity-100 group-hover:brightness-110`}
      />
      <div className="absolute inset-0 bg-base-900/50 transition-colors duration-250 group-hover:bg-base-900/30" />
      <div className="pointer-events-none absolute inset-0 opacity-0 transition-opacity duration-250 group-hover:opacity-100 bg-gradient-to-t from-accent/10 via-transparent to-white/5" />

      <div className="relative shrink-0 text-text-secondary transition-colors duration-250 group-hover:text-accent">
        <CategoryIcon id={id} compact={isHorizontal} />
      </div>
      <p
        className={`relative w-full min-w-0 font-semibold text-text-secondary transition-colors duration-250 group-hover:text-text-primary ${
          isHorizontal
            ? "line-clamp-2 whitespace-normal break-words text-[11px] leading-[1.35]"
            : "line-clamp-2 text-xs leading-snug"
        }`}
      >
        {label}
      </p>
      {countLabel ? (
        <p
          className={`relative w-full min-w-0 font-medium text-text-muted transition-colors duration-250 group-hover:text-text-secondary ${
            isHorizontal ? "text-[10px] leading-none" : "text-[11px] leading-none"
          }`}
        >
          {countLabel}
        </p>
      ) : null}
      {locked ? (
        <span className="absolute right-2 top-2 rounded-md bg-base-950/80 px-1.5 py-0.5 text-[10px] font-semibold text-amber-300">
          🔒
        </span>
      ) : null}
    </button>
  );
}

function CategoryCardsSkeleton({
  orientation,
  className = "",
}: {
  orientation: CategoryOrientation;
  className?: string;
}) {
  if (orientation === "horizontal") {
    return (
      <div
        className={`category-panel shrink-0 border-b border-base-800/80 bg-base-950 px-4 md:px-6 ${className}`.trim()}
        aria-busy="true"
        aria-label="Carregando categorias"
      >
        <div className="flex gap-2.5 overflow-hidden py-2.5">
          {Array.from({ length: HORIZONTAL_SKELETON_COUNT }).map((_, index) => (
            <Skeleton key={index} className="h-[84px] min-w-[116px] w-[116px] shrink-0 rounded-xl" />
          ))}
        </div>
      </div>
    );
  }

  return (
    <div
      className={`category-panel flex h-full min-h-0 flex-1 flex-col overflow-hidden ${className}`.trim()}
      aria-busy="true"
      aria-label="Carregando categorias"
    >
      <div className="panel-scrollbar min-h-0 flex-1 overflow-hidden px-3 py-3">
        <div className="grid grid-cols-2 gap-2">
          {Array.from({ length: VERTICAL_SKELETON_COUNT }).map((_, index) => (
            <Skeleton key={index} className="h-[88px] w-full rounded-xl" />
          ))}
        </div>
      </div>
    </div>
  );
}

export function CategoryCards({
  groups,
  counts,
  onSelect,
  className = "",
  orientation = "horizontal",
  selectedId = null,
  loading = false,
  lockedIds,
}: CategoryCardsProps) {
  const categoryOptions = useMemo(
    () => [
      { id: "all", label: formatGroupLabel("all") },
      ...groups.map((g) => ({ id: g, label: formatGroupLabel(g) })),
    ],
    [groups],
  );

  const {
    ref: scrollRef,
    isGrabbing,
    canScrollLeft,
    canScrollRight,
    scrollLeft,
    scrollRight,
    onPointerDown,
    onPointerMove,
    onPointerUp,
    onPointerCancel,
    onWheel,
    consumeClickIfDragged,
  } = useCarouselScroll<HTMLDivElement>([categoryOptions.length]);

  const handleSelect = useCallback(
    (value: string) => {
      if (consumeClickIfDragged()) return;
      onSelect(value);
    },
    [consumeClickIfDragged, onSelect],
  );

  if (loading && groups.length === 0) {
    return <CategoryCardsSkeleton orientation={orientation} className={className} />;
  }

  if (orientation === "horizontal") {
    return (
      <div
        className={`category-panel min-w-0 shrink-0 border-b border-base-800/80 bg-base-950 px-4 md:px-6 ${className}`.trim()}
      >
        <CarouselNavRow
          onScrollLeft={scrollLeft}
          onScrollRight={scrollRight}
          canScrollLeft={canScrollLeft}
          canScrollRight={canScrollRight}
          size="sm"
        >
          <div
            ref={scrollRef}
            role="listbox"
            aria-label="Categorias"
            aria-busy={loading}
            className={`carousel-scroll panel-scrollbar flex w-full min-w-0 gap-2.5 overflow-x-auto py-2.5 snap-x snap-mandatory ${
              isGrabbing ? "is-grabbing" : ""
            }`}
            onPointerDown={onPointerDown}
            onPointerMove={onPointerMove}
            onPointerUp={onPointerUp}
            onPointerCancel={onPointerCancel}
            onWheel={onWheel}
            onDragStart={(event) => event.preventDefault()}
          >
            {categoryOptions.map((option) => (
              <CategoryCardButton
                key={option.id}
                id={option.id}
                label={option.label}
                count={counts?.[option.id]}
                onSelect={handleSelect}
                orientation={orientation}
                selected={selectedId === option.id}
                locked={lockedIds?.has(option.id) ?? false}
              />
            ))}
          </div>
        </CarouselNavRow>
      </div>
    );
  }

  return (
    <div className={`category-panel flex h-full min-h-0 flex-1 flex-col overflow-hidden ${className}`.trim()}>
      <div className="panel-scrollbar min-h-0 flex-1 overflow-y-auto px-3 py-3">
        <div role="listbox" aria-label="Categorias" className="grid grid-cols-2 gap-2">
          {categoryOptions.map((option) => (
            <CategoryCardButton
              key={option.id}
              id={option.id}
              label={option.label}
              count={counts?.[option.id]}
              onSelect={onSelect}
              orientation={orientation}
              selected={selectedId === option.id}
              locked={lockedIds?.has(option.id) ?? false}
            />
          ))}
        </div>
      </div>
    </div>
  );
}
