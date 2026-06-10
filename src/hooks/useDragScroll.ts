import { useCallback, useLayoutEffect, useRef, useState } from "react";

function readScrollEdges(el: HTMLElement) {
  const maxScroll = el.scrollWidth - el.clientWidth;
  return {
    canScrollLeft: el.scrollLeft > 1,
    canScrollRight: el.scrollLeft < maxScroll - 1,
  };
}

function isInteractiveTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  return Boolean(
    target.closest(
      'button, a, input, textarea, select, [role="button"], [role="option"]',
    ),
  );
}

export interface CarouselScrollOptions {
  loop?: boolean;
}

/**
 * Horizontal carousel scroll: drag-to-scroll, vertical wheel → horizontal,
 * arrow navigation, and click suppression after a drag gesture. Touch/trackpad
 * scroll natively via overflow-x-auto.
 */
export function useCarouselScroll<T extends HTMLElement>(
  deps: readonly unknown[] = [],
  options: CarouselScrollOptions = {},
) {
  const { loop = false } = options;
  const ref = useRef<T>(null);
  const dragRef = useRef({ active: false, startX: 0, scrollLeft: 0, moved: false });
  const [isGrabbing, setIsGrabbing] = useState(false);
  const [canScrollLeft, setCanScrollLeft] = useState(false);
  const [canScrollRight, setCanScrollRight] = useState(false);

  const endDrag = useCallback((pointerId: number) => {
    const el = ref.current;
    dragRef.current.active = false;
    setIsGrabbing(false);
    if (el?.hasPointerCapture(pointerId)) {
      el.releasePointerCapture(pointerId);
    }
  }, []);

  const onPointerDown = useCallback((event: React.PointerEvent<T>) => {
    const el = ref.current;
    if (!el || event.button !== 0) return;
    if (isInteractiveTarget(event.target)) return;

    dragRef.current = {
      active: true,
      startX: event.clientX,
      scrollLeft: el.scrollLeft,
      moved: false,
    };
    setIsGrabbing(true);
    el.setPointerCapture(event.pointerId);
  }, []);

  const onPointerMove = useCallback((event: React.PointerEvent<T>) => {
    const el = ref.current;
    if (!el || !dragRef.current.active) return;

    const deltaX = event.clientX - dragRef.current.startX;
    if (Math.abs(deltaX) > 4) {
      dragRef.current.moved = true;
    }
    el.scrollLeft = dragRef.current.scrollLeft - deltaX;
  }, []);

  const onPointerUp = useCallback(
    (event: React.PointerEvent<T>) => {
      endDrag(event.pointerId);
    },
    [endDrag],
  );

  const onPointerCancel = useCallback(
    (event: React.PointerEvent<T>) => {
      endDrag(event.pointerId);
    },
    [endDrag],
  );

  const onWheel = useCallback((event: React.WheelEvent<T>) => {
    const el = ref.current;
    if (!el) return;
    if (Math.abs(event.deltaY) <= Math.abs(event.deltaX)) return;
    if (el.scrollWidth <= el.clientWidth) return;

    el.scrollLeft += event.deltaY;
    event.preventDefault();
  }, []);

  const consumeClickIfDragged = useCallback(() => {
    if (dragRef.current.moved) {
      dragRef.current.moved = false;
      return true;
    }
    return false;
  }, []);

  const updateScrollState = useCallback(() => {
    const el = ref.current;
    if (!el) return;
    const maxScroll = el.scrollWidth - el.clientWidth;
    const scrollable = maxScroll > 1;
    if (loop && scrollable) {
      setCanScrollLeft(true);
      setCanScrollRight(true);
      return;
    }
    const { canScrollLeft: left, canScrollRight: right } = readScrollEdges(el);
    setCanScrollLeft(left);
    setCanScrollRight(right);
  }, [loop]);

  const scrollByDirection = useCallback(
    (direction: "left" | "right") => {
      const el = ref.current;
      if (!el) return;

      const maxScroll = el.scrollWidth - el.clientWidth;
      if (maxScroll <= 0) return;

      const amount = Math.max(el.clientWidth * 0.8, 240);
      const atLeft = el.scrollLeft <= 1;
      const atRight = el.scrollLeft >= maxScroll - 1;

      if (loop) {
        if (direction === "right" && atRight) {
          el.scrollTo({ left: 0, behavior: "smooth" });
          return;
        }
        if (direction === "left" && atLeft) {
          el.scrollTo({ left: maxScroll, behavior: "smooth" });
          return;
        }
      }

      el.scrollBy({
        left: direction === "left" ? -amount : amount,
        behavior: "smooth",
      });
    },
    [loop],
  );

  const scrollLeft = useCallback(() => scrollByDirection("left"), [scrollByDirection]);
  const scrollRight = useCallback(() => scrollByDirection("right"), [scrollByDirection]);

  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) return;

    const scheduleUpdate = () => {
      requestAnimationFrame(() => {
        updateScrollState();
      });
    };

    scheduleUpdate();
    el.addEventListener("scroll", scheduleUpdate, { passive: true });
    window.addEventListener("resize", scheduleUpdate);

    const resizeObserver = new ResizeObserver(scheduleUpdate);
    resizeObserver.observe(el);

    const mutationObserver = new MutationObserver(scheduleUpdate);
    mutationObserver.observe(el, { childList: true, subtree: true });

    return () => {
      el.removeEventListener("scroll", scheduleUpdate);
      window.removeEventListener("resize", scheduleUpdate);
      resizeObserver.disconnect();
      mutationObserver.disconnect();
    };
  }, [updateScrollState, ...deps]);

  return {
    ref,
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
  };
}

/** @deprecated Use useCarouselScroll */
export function useHorizontalWheel<T extends HTMLElement>() {
  const { ref, onWheel } = useCarouselScroll<T>();
  return { ref, onWheel };
}
