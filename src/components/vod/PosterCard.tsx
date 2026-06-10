import { useEffect, useState } from "react";
import { FavoriteToggle } from "../favorites/FavoriteToggle";
import { cardPosterUrl } from "../../lib/posterUtils";

const POSTER_COLORS = ["#1c1c2e", "#1a2438", "#241a2e", "#1a2e28", "#2e1a24"];

interface PosterCardProps {
  title: string;
  poster?: string | null;
  backdrop?: string | null;
  colorIndex?: number;
  onClick?: () => void;
  favoriteItemType?: "movie" | "series" | "channel";
  favoriteItemId?: string;
}

function formatTitle(title: string): string {
  const trimmed = title.trim();
  return trimmed ? trimmed.toLocaleUpperCase("pt-BR") : trimmed;
}

export function PosterCard({
  title,
  poster,
  backdrop,
  colorIndex = 0,
  onClick,
  favoriteItemType,
  favoriteItemId,
}: PosterCardProps) {
  const [imgError, setImgError] = useState(false);
  const fallbackColor = POSTER_COLORS[colorIndex % POSTER_COLORS.length];
  const posterSrc = cardPosterUrl(poster, backdrop);
  const showPoster = Boolean(posterSrc) && !imgError;
  const displayTitle = formatTitle(title);

  useEffect(() => {
    setImgError(false);
  }, [posterSrc]);

  return (
    <div
      role="button"
      tabIndex={0}
      onClick={() => onClick?.()}
      onKeyDown={(event) => {
        if (event.key === "Enter" || event.key === " ") {
          event.preventDefault();
          onClick?.();
        }
      }}
      className="group w-full min-w-0 cursor-pointer text-left transition-transform duration-250 hover:scale-[1.03]"
    >
      <div
        className="relative aspect-[2/3] min-h-[195px] w-full overflow-hidden rounded-xl"
        style={{ backgroundColor: fallbackColor }}
      >
        {showPoster ? (
          <img
            src={posterSrc!}
            alt={displayTitle || title}
            loading="lazy"
            decoding="async"
            referrerPolicy="no-referrer"
            className="pointer-events-none absolute inset-0 h-full w-full object-cover"
            onError={() => setImgError(true)}
          />
        ) : null}
        {favoriteItemType && favoriteItemId ? (
          <div
            className="absolute right-2 top-2 z-10 opacity-0 transition-opacity group-hover:opacity-100"
            onClick={(event) => event.stopPropagation()}
            onKeyDown={(event) => event.stopPropagation()}
          >
            <FavoriteToggle
              itemType={favoriteItemType}
              itemId={favoriteItemId}
              size="sm"
              stopPropagation
              className="bg-base-950/70 opacity-100 backdrop-blur-sm"
            />
          </div>
        ) : null}
      </div>
      <p className="mt-2 truncate text-xs font-semibold text-text-primary">
        {displayTitle || title}
      </p>
    </div>
  );
}
