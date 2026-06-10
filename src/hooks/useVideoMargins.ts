import { useEffect, useRef } from "react";
import type { RefObject } from "react";
import {
  setVideoMarginRatio,
  type VideoMarginRatio,
} from "tauri-plugin-libmpv-api";

function clamp01(value: number): number {
  return Math.max(0, Math.min(1, value));
}

const ZERO_MARGINS: VideoMarginRatio = {
  left: 0,
  right: 0,
  top: 0,
  bottom: 0,
};

const MARGIN_DEBOUNCE_MS = 400;

/** Serialize native margin updates — concurrent calls during D3D11 present crash mpv. */
let marginUpdateChain = Promise.resolve<void>(undefined);

function queueMarginUpdate(ratio: VideoMarginRatio): void {
  marginUpdateChain = marginUpdateChain
    .then(async () => {
      await setVideoMarginRatio(ratio);
    })
    .catch(() => undefined);
}

function ratioKey(ratio: VideoMarginRatio): string {
  return `${ratio.left},${ratio.right},${ratio.top},${ratio.bottom}`;
}

export function useVideoMargins(
  elementRef: RefObject<HTMLElement | null>,
  enabled: boolean,
  fullscreen: boolean,
  loading: boolean,
  revision: string | number = 0,
) {
  const lastAppliedRef = useRef<string>("");
  const debounceRef = useRef<number | null>(null);

  useEffect(() => {
    if (!enabled) return;

    if (fullscreen) {
      lastAppliedRef.current = "";
      queueMarginUpdate(ZERO_MARGINS);
      return;
    }

    // Never touch video margins while a stream is opening — the log shows
    // margin thrashing during loadfile causes D3D11 resize crashes.
    if (loading) return;

    const element = elementRef.current;
    if (!element) return;

    let cancelled = false;

    const applyMargins = () => {
      if (cancelled) return;

      const rect = element.getBoundingClientRect();
      const width = window.innerWidth;
      const height = window.innerHeight;
      if (
        width <= 0 ||
        height <= 0 ||
        rect.width <= 0 ||
        rect.height <= 0
      ) {
        return;
      }

      const ratio: VideoMarginRatio = {
        left: clamp01(Math.round(rect.left) / width),
        right: clamp01(1 - Math.round(rect.right) / width),
        top: clamp01(Math.round(rect.top) / height),
        bottom: clamp01(1 - Math.round(rect.bottom) / height),
      };

      const key = ratioKey(ratio);
      if (key === lastAppliedRef.current) return;

      lastAppliedRef.current = key;
      queueMarginUpdate(ratio);
    };

    const schedule = () => {
      if (debounceRef.current != null) {
        window.clearTimeout(debounceRef.current);
      }
      debounceRef.current = window.setTimeout(() => {
        debounceRef.current = null;
        window.requestAnimationFrame(applyMargins);
      }, MARGIN_DEBOUNCE_MS);
    };

    schedule();

    const observer = new ResizeObserver(schedule);
    observer.observe(element);
    window.addEventListener("resize", schedule);

    return () => {
      cancelled = true;
      if (debounceRef.current != null) {
        window.clearTimeout(debounceRef.current);
        debounceRef.current = null;
      }
      observer.disconnect();
      window.removeEventListener("resize", schedule);
      // Do NOT reset margins to zero here — that races with loadfile and
      // triggers vo/gpu Resize during stream open (STATUS_ACCESS_VIOLATION).
    };
  }, [elementRef, enabled, fullscreen, loading, revision]);
}
