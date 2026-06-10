import { useEffect, useRef, useState, type ReactNode } from "react";
import type { MpvTrack } from "../../lib/mpvTracks";
import { PlayerIconButton } from "./PlayerIconButton";

interface TrackPopoverProps {
  label: string;
  icon: ReactNode;
  tracks: MpvTrack[];
  activeId: number | "no" | null;
  disabled?: boolean;
  showOffOption?: boolean;
  offLabel?: string;
  emptyLabel?: string;
  onSelect: (id: number | "no") => void;
}

export function TrackPopover({
  label,
  icon,
  tracks,
  activeId,
  disabled = false,
  showOffOption = false,
  offLabel = "Desativadas",
  emptyLabel,
  onSelect,
}: TrackPopoverProps) {
  const [open, setOpen] = useState(false);
  const rootRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    const onPointerDown = (event: MouseEvent) => {
      if (!rootRef.current?.contains(event.target as Node)) {
        setOpen(false);
      }
    };
    window.addEventListener("mousedown", onPointerDown);
    return () => window.removeEventListener("mousedown", onPointerDown);
  }, [open]);

  const hasItems = tracks.length > 0 || showOffOption || Boolean(emptyLabel);
  if (!hasItems) return null;

  return (
    <div ref={rootRef} className="relative">
      <PlayerIconButton
        label={label}
        onClick={() => setOpen((value) => !value)}
        disabled={disabled}
        active={open}
      >
        {icon}
      </PlayerIconButton>

      {open && (
        <div
          role="menu"
          className="absolute bottom-full right-0 z-30 mb-2 min-w-[180px] max-w-[240px] overflow-hidden rounded-xl border border-base-700/60 bg-base-950/95 py-1 shadow-xl backdrop-blur-xl"
        >
          {showOffOption && (
            <button
              type="button"
              role="menuitemradio"
              aria-checked={activeId === "no"}
              onClick={() => {
                onSelect("no");
                setOpen(false);
              }}
              className={`flex w-full px-3 py-2 text-left text-sm transition-colors hover:bg-white/10 ${
                activeId === "no" ? "text-accent" : "text-text-secondary"
              }`}
            >
              {offLabel}
            </button>
          )}
          {tracks.length === 0 && emptyLabel ? (
            <p className="px-3 py-2 text-sm text-text-secondary">{emptyLabel}</p>
          ) : null}
          {tracks.map((track) => (
            <button
              key={track.id}
              type="button"
              role="menuitemradio"
              aria-checked={activeId === track.id}
              onClick={() => {
                onSelect(track.id);
                setOpen(false);
              }}
              className={`flex w-full px-3 py-2 text-left text-sm transition-colors hover:bg-white/10 ${
                activeId === track.id || track.selected
                  ? "text-accent"
                  : "text-text-secondary"
              }`}
            >
              <span className="truncate">{track.title}</span>
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
