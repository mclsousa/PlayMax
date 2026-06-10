import { useEffect, useRef, useState } from "react";
import { ChannelEpgDisplay } from "../live/ChannelEpgDisplay";
import type { ChannelEpg } from "../../lib/types";

interface EpgPopoverProps {
  channelKey: string | null;
  epg: ChannelEpg | null;
  loading?: boolean;
  disabled?: boolean;
  onToggle?: (open: boolean) => void;
}

export function EpgPopover({
  channelKey,
  epg,
  loading = false,
  disabled = false,
  onToggle,
}: EpgPopoverProps) {
  const [open, setOpen] = useState(false);
  const rootRef = useRef<HTMLDivElement>(null);
  const onToggleRef = useRef(onToggle);

  useEffect(() => {
    onToggleRef.current = onToggle;
  }, [onToggle]);

  useEffect(() => {
    setOpen(false);
    onToggleRef.current?.(false);
  }, [channelKey]);

  useEffect(() => {
    if (!open) return;
    const onPointerDown = (event: PointerEvent) => {
      if (!rootRef.current?.contains(event.target as Node)) {
        setOpen(false);
        onToggle?.(false);
      }
    };
    window.addEventListener("pointerdown", onPointerDown);
    return () => window.removeEventListener("pointerdown", onPointerDown);
  }, [open, onToggle]);

  const toggle = () => {
    setOpen((value) => {
      const next = !value;
      onToggle?.(next);
      return next;
    });
  };

  return (
    <div ref={rootRef} className="relative shrink-0">
      <button
        type="button"
        onClick={toggle}
        disabled={disabled}
        aria-expanded={open}
        aria-label={open ? "Ocultar programação" : "Ver programação (EPG)"}
        title={open ? "Ocultar programação" : "Ver programação (EPG)"}
        className={`rounded-full px-3 py-1.5 text-xs font-semibold tracking-wide transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent/60 disabled:cursor-not-allowed disabled:opacity-40 ${
          open
            ? "bg-accent/20 text-accent"
            : "text-text-secondary hover:bg-white/10 hover:text-text-primary"
        }`}
      >
        EPG
      </button>

      {open && (
        <div
          role="dialog"
          aria-label="Programação do canal"
          onPointerDown={(event) => event.stopPropagation()}
          onWheel={(event) => event.stopPropagation()}
          className="absolute bottom-full right-0 z-30 mb-2 w-80 max-w-[min(20rem,calc(100vw-2rem))] overflow-hidden rounded-xl border border-base-700/60 bg-base-950/95 shadow-xl backdrop-blur-xl"
        >
          <div className="border-b border-base-800/80 px-3 py-2">
            <p className="text-xs font-semibold uppercase tracking-wide text-text-muted">
              Programação
            </p>
          </div>
          <div className="p-2">
            <ChannelEpgDisplay epg={epg} loading={loading} fullList />
          </div>
        </div>
      )}
    </div>
  );
}
