import type { ButtonHTMLAttributes, ReactNode } from "react";

interface PlayerIconButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  label: string;
  active?: boolean;
  children: ReactNode;
}

export function PlayerIconButton({
  label,
  active = false,
  className = "",
  children,
  disabled,
  ...props
}: PlayerIconButtonProps) {
  return (
    <button
      type="button"
      title={label}
      aria-label={label}
      disabled={disabled}
      className={`inline-flex size-9 shrink-0 items-center justify-center rounded-full text-text-secondary transition-all duration-200 hover:bg-white/10 hover:text-text-primary focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent/60 disabled:cursor-not-allowed disabled:opacity-40 disabled:hover:bg-transparent disabled:hover:text-text-secondary ${
        active ? "bg-accent/20 text-accent hover:bg-accent/25 hover:text-accent" : ""
      } ${className}`}
      {...props}
    >
      {children}
    </button>
  );
}
