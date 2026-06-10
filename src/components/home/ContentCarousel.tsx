import { Link } from "react-router-dom";
import { memo, useCallback, useState } from "react";
import { useCarouselScroll } from "../../hooks/useDragScroll";
import { CarouselArrows } from "../ui/CarouselArrows";
import { cardPosterUrl } from "../../lib/posterUtils";

export interface CarouselItem {
  id: string;
  title: string;
  subtitle?: string;
  color?: string;
  progress?: number;
  posterUrl?: string | null;
  backdropUrl?: string | null;
  itemType?: string;
  itemId?: string;
  streamUrl?: string;
  startPosition?: number;
  showNovoBadge?: boolean;
}

interface ContentCarouselProps {
  title: string;
  items: CarouselItem[];
  variant?: "landscape" | "poster";
  viewAllHref?: string;
  onItemClick?: (item: CarouselItem) => void;
  loop?: boolean;
}

const POSTER_COLORS = ["#1c1c2e", "#1a2438", "#241a2e", "#1a2e28", "#2e1a24"];
const LANDSCAPE_COLORS = ["#1c1c2e", "#1a2438", "#241a2e", "#1a2e28"];

function formatDisplayTitle(title: string): string {
  return title.trim().toLocaleUpperCase("pt-BR");
}

function PosterImage({
  src,
  alt,
  fallbackColor,
}: {
  src: string;
  alt: string;
  fallbackColor: string;
}) {
  const [failed, setFailed] = useState(false);

  if (failed) {
    return null;
  }

  return (
    <img
      src={src}
      alt={alt}
      loading="lazy"
      decoding="async"
      referrerPolicy="no-referrer"
      draggable={false}
      className="pointer-events-none absolute inset-0 h-full w-full object-cover transition-transform duration-500 group-hover:scale-105"
      onError={() => setFailed(true)}
      style={{ backgroundColor: fallbackColor }}
    />
  );
}

function cardWidthClass(isLandscape: boolean): string {
  if (isLandscape) {
    return "w-[280px] sm:w-[300px] md:w-[320px]";
  }
  return "w-[156px] sm:w-[172px] md:w-[188px] lg:w-[204px]";
}

function cardMediaClass(isLandscape: boolean): string {
  if (isLandscape) {
    return "aspect-video min-h-[158px] sm:min-h-[168px]";
  }
  return "aspect-[2/3] min-h-[234px] sm:min-h-[258px] md:min-h-[282px]";
}

export const ContentCarousel = memo(function ContentCarousel({
  title,
  items,
  variant = "poster",
  viewAllHref,
  onItemClick,
  loop = true,
}: ContentCarouselProps) {
  const isLandscape = variant === "landscape";
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
  } = useCarouselScroll<HTMLDivElement>(
    [items.length, items.map((item) => item.id).join("|")],
    { loop },
  );

  const handleItemClick = useCallback(
    (item: CarouselItem) => {
      if (consumeClickIfDragged()) return;
      onItemClick?.(item);
    },
    [consumeClickIfDragged, onItemClick],
  );

  return (
    <section className="min-w-0 space-y-4">
      <div className="flex items-center justify-between gap-4">
        <h2 className="text-lg font-bold tracking-tight">{title}</h2>
        {viewAllHref ? (
          <Link
            to={viewAllHref}
            className="shrink-0 text-sm font-medium text-text-secondary transition-colors hover:text-text-primary"
          >
            Ver todos &gt;
          </Link>
        ) : null}
      </div>

      <div className="relative min-w-0 overflow-hidden">
        <div
          ref={scrollRef}
          className={`carousel-scroll relative z-0 flex w-full min-w-0 max-w-full snap-x snap-mandatory gap-4 overflow-x-auto pb-2 ${
            isGrabbing ? "is-grabbing" : ""
          }`}
          onPointerDown={onPointerDown}
          onPointerMove={onPointerMove}
          onPointerUp={onPointerUp}
          onPointerCancel={onPointerCancel}
          onWheel={onWheel}
          onDragStart={(event) => event.preventDefault()}
        >
          {items.map((item, index) => {
            const fallbackColor = isLandscape
              ? (item.color ?? LANDSCAPE_COLORS[index % LANDSCAPE_COLORS.length])
              : (item.color ?? POSTER_COLORS[index % POSTER_COLORS.length]);
            const displayTitle = formatDisplayTitle(item.title);
            const imageSrc = cardPosterUrl(item.posterUrl, item.backdropUrl);

            const content = (
              <>
                <div
                  className={`group/media relative overflow-hidden rounded-2xl shadow-lg shadow-black/25 ring-1 ring-base-700/40 transition-all duration-300 group-hover:shadow-xl group-hover:shadow-black/35 group-hover:ring-accent/35 ${cardMediaClass(isLandscape)}`}
                  style={{ backgroundColor: fallbackColor }}
                >
                  {imageSrc ? (
                    <PosterImage
                      src={imageSrc}
                      alt={displayTitle}
                      fallbackColor={fallbackColor}
                    />
                  ) : (
                    <div className="absolute inset-0 bg-gradient-to-br from-base-800 to-base-950" />
                  )}

                  <div className="pointer-events-none absolute inset-0 bg-gradient-to-t from-base-950/70 via-transparent to-transparent opacity-80" />

                  {item.showNovoBadge && (
                    <span className="absolute left-2.5 top-2.5 rounded-full bg-violet-600 px-2.5 py-1 text-[10px] font-bold uppercase tracking-wide text-white shadow-md">
                      Novo
                    </span>
                  )}

                  {item.progress !== undefined && item.progress > 0 && (
                    <div className="absolute bottom-0 left-0 right-0 h-1.5 bg-base-950/60 backdrop-blur-sm">
                      <div
                        className="h-full rounded-r-full bg-accent shadow-[0_0_8px_rgba(123,92,255,0.65)]"
                        style={{ width: `${Math.min(100, Math.max(0, item.progress))}%` }}
                      />
                    </div>
                  )}
                </div>

                <div className="mt-3 min-w-0 px-0.5">
                  <p className="line-clamp-2 text-sm font-semibold leading-snug text-text-primary">
                    {displayTitle}
                  </p>
                  {item.subtitle && (
                    <p className="mt-1 line-clamp-1 text-xs text-text-muted">
                      {item.subtitle}
                    </p>
                  )}
                </div>
              </>
            );

            const cardClass = `group shrink-0 snap-start text-left transition-transform duration-300 hover:scale-[1.04] ${cardWidthClass(isLandscape)}`;

            if (onItemClick) {
              return (
                <button
                  key={item.id}
                  type="button"
                  onPointerDown={(event) => event.stopPropagation()}
                  onClick={() => handleItemClick(item)}
                  className={cardClass}
                >
                  {content}
                </button>
              );
            }

            return (
              <article key={item.id} className={cardClass}>
                {content}
              </article>
            );
          })}
        </div>
        <CarouselArrows
          onScrollLeft={scrollLeft}
          onScrollRight={scrollRight}
          canScrollLeft={canScrollLeft}
          canScrollRight={canScrollRight}
        />
      </div>
    </section>
  );
});
