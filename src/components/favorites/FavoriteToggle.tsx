import { useState } from "react";
import { useIsFavorite, useFavorites } from "../../hooks/useFavorites";
import { useActiveProfile } from "../../hooks/useActiveProfile";

interface FavoriteToggleProps {
  itemType: "movie" | "series" | "channel";
  itemId: string;
  className?: string;
  size?: "md" | "sm";
  stopPropagation?: boolean;
}

export function FavoriteToggle({
  itemType,
  itemId,
  className = "",
  size = "md",
  stopPropagation = false,
}: FavoriteToggleProps) {
  const { profileId } = useActiveProfile();
  const { favorite, loading } = useIsFavorite(profileId, itemType, itemId);
  const { toggleFavorite } = useFavorites(profileId);
  const [busy, setBusy] = useState(false);

  const handleClick = async (event: React.MouseEvent) => {
    if (stopPropagation) event.stopPropagation();
    if (!profileId || busy || loading) return;
    setBusy(true);
    try {
      await toggleFavorite(itemType, itemId, favorite);
    } finally {
      setBusy(false);
    }
  };

  const sizeClass =
    size === "sm"
      ? "h-7 w-7 border-transparent bg-transparent hover:bg-base-800/80"
      : "h-10 w-10 border border-base-700/60 bg-base-900/60 hover:border-accent/40";

  const iconClass = size === "sm" ? "h-4 w-4" : "h-5 w-5";

  return (
    <button
      type="button"
      onClick={(event) => void handleClick(event)}
      disabled={!profileId || loading || busy}
      aria-label={favorite ? "Remover dos favoritos" : "Adicionar aos favoritos"}
      aria-pressed={favorite}
      className={`inline-flex shrink-0 items-center justify-center rounded-full text-text-secondary transition-colors hover:text-accent disabled:opacity-50 ${sizeClass} ${className}`}
    >
      <svg
        viewBox="0 0 24 24"
        fill={favorite ? "currentColor" : "none"}
        stroke="currentColor"
        strokeWidth="1.75"
        className={`${iconClass} ${favorite ? "text-accent" : ""}`}
      >
        <path d="M12 21.35l-1.45-1.32C5.4 15.36 2 12.28 2 8.5 2 5.42 4.42 3 7.5 3c1.74 0 3.41.81 4.5 2.09C13.09 3.81 14.76 3 16.5 3 19.58 3 22 5.42 22 8.5c0 3.78-3.4 6.86-8.55 11.54L12 21.35z" />
      </svg>
    </button>
  );
}
