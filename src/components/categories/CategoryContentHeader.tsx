import type { ReactNode } from "react";

interface CategoryContentHeaderProps {
  title?: string;
  categoryLabel: string;
  subtitle: string;
  icon?: ReactNode;
  actions?: ReactNode;
}

export function CategoryContentHeader({
  title,
  categoryLabel,
  subtitle,
  icon,
  actions,
}: CategoryContentHeaderProps) {
  return (
    <header className="shrink-0 border-b border-base-800/80 px-4 py-2.5">
      {title ? (
        <p className="mb-2 text-xs font-medium uppercase tracking-wide text-text-muted">{title}</p>
      ) : null}
      <div className="flex items-center gap-3">
        {icon ? (
          <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-xl bg-base-800/80 ring-1 ring-base-700/50">
            {icon}
          </div>
        ) : null}
        <div className="min-w-0 flex-1">
          <h2 className="truncate text-sm font-semibold tracking-tight">{categoryLabel}</h2>
          <p className="truncate text-xs text-text-muted">{subtitle}</p>
        </div>
        {actions}
      </div>
    </header>
  );
}
