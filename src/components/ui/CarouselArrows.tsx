import type { MouseEvent, ReactNode } from "react";

interface CarouselArrowsProps {
  onScrollLeft: () => void;
  onScrollRight: () => void;
  canScrollLeft: boolean;
  canScrollRight: boolean;
  size?: "sm" | "md";
  variant?: "overlay" | "inline";
  className?: string;
}

function ChevronIcon({ direction }: { direction: "left" | "right" }) {
  return (
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      className="h-4 w-4"
      aria-hidden
    >
      {direction === "left" ? (
        <path d="M15 18l-6-6 6-6" />
      ) : (
        <path d="M9 18l6-6-6-6" />
      )}
    </svg>
  );
}

function handleNavClick(event: MouseEvent, onClick: () => void) {
  event.preventDefault();
  event.stopPropagation();
  onClick();
}

function NavButton({
  direction,
  onClick,
  disabled,
  size,
  variant,
}: {
  direction: "left" | "right";
  onClick: () => void;
  disabled: boolean;
  size: "sm" | "md";
  variant: "overlay" | "inline";
}) {
  const iconSizeClass = size === "sm" ? "h-8 w-8" : "h-9 w-9";
  const label = direction === "left" ? "Anterior" : "Próximo";

  if (variant === "overlay") {
    const sideClass =
      direction === "left"
        ? "left-0 justify-start pl-1.5 bg-gradient-to-r from-base-950/90 to-transparent"
        : "right-0 justify-end pr-1.5 bg-gradient-to-l from-base-950/90 to-transparent";

    return (
      <button
        type="button"
        aria-label={label}
        disabled={disabled}
        onClick={(event) => handleNavClick(event, onClick)}
        onPointerDown={(event) => event.stopPropagation()}
        className={`absolute inset-y-0 z-20 flex w-14 items-center ${sideClass} transition-opacity duration-300 disabled:pointer-events-none disabled:opacity-0`}
      >
        <span className={`carousel-nav-btn inline-flex ${iconSizeClass} items-center justify-center`}>
          <ChevronIcon direction={direction} />
        </span>
      </button>
    );
  }

  return (
    <button
      type="button"
      aria-label={label}
      disabled={disabled}
      onClick={(event) => handleNavClick(event, onClick)}
      onPointerDown={(event) => event.stopPropagation()}
      className={`carousel-nav-btn relative z-20 shrink-0 ${iconSizeClass}`}
    >
      <ChevronIcon direction={direction} />
    </button>
  );
}

export function CarouselArrows({
  onScrollLeft,
  onScrollRight,
  canScrollLeft,
  canScrollRight,
  size = "md",
  variant = "overlay",
  className = "",
}: CarouselArrowsProps) {
  if (variant === "inline") {
    return (
      <>
        <NavButton
          direction="left"
          onClick={onScrollLeft}
          disabled={!canScrollLeft}
          size={size}
          variant="inline"
        />
        <NavButton
          direction="right"
          onClick={onScrollRight}
          disabled={!canScrollRight}
          size={size}
          variant="inline"
        />
      </>
    );
  }

  return (
    <div className={`pointer-events-none absolute inset-0 z-30 ${className}`.trim()}>
      <div className={canScrollLeft ? "pointer-events-auto" : "pointer-events-none"}>
        <NavButton
          direction="left"
          onClick={onScrollLeft}
          disabled={!canScrollLeft}
          size={size}
          variant="overlay"
        />
      </div>
      <div className={canScrollRight ? "pointer-events-auto" : "pointer-events-none"}>
        <NavButton
          direction="right"
          onClick={onScrollRight}
          disabled={!canScrollRight}
          size={size}
          variant="overlay"
        />
      </div>
    </div>
  );
}

interface CarouselNavRowProps {
  onScrollLeft: () => void;
  onScrollRight: () => void;
  canScrollLeft: boolean;
  canScrollRight: boolean;
  size?: "sm" | "md";
  children: ReactNode;
  className?: string;
}

export function CarouselNavRow({
  onScrollLeft,
  onScrollRight,
  canScrollLeft,
  canScrollRight,
  size = "sm",
  children,
  className = "",
}: CarouselNavRowProps) {
  return (
    <div className={`flex min-w-0 items-center gap-1.5 ${className}`.trim()}>
      <NavButton
        direction="left"
        onClick={onScrollLeft}
        disabled={!canScrollLeft}
        size={size}
        variant="inline"
      />
      <div className="relative min-w-0 flex-1 overflow-hidden">{children}</div>
      <NavButton
        direction="right"
        onClick={onScrollRight}
        disabled={!canScrollRight}
        size={size}
        variant="inline"
      />
    </div>
  );
}
