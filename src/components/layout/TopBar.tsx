import { SearchIcon } from "./icons";

interface TopBarProps {
  search?: string;
  onSearchChange?: (value: string) => void;
  showSearch?: boolean;
  title?: string;
  subtitle?: string;
}

export function TopBar({
  search = "",
  onSearchChange,
  showSearch = true,
  title,
  subtitle,
}: TopBarProps) {
  const hasPageTitle = Boolean(title || subtitle);

  return (
    <header className="flex shrink-0 items-center justify-between gap-4 border-b border-base-800/50 bg-base-950 px-4 py-2.5 md:px-6">
      {hasPageTitle ? (
        <div className="min-w-0 flex-1">
          {title ? (
            <h1 className="truncate text-sm font-semibold tracking-tight">{title}</h1>
          ) : null}
          {subtitle ? (
            <p className="truncate text-xs text-text-muted">{subtitle}</p>
          ) : null}
        </div>
      ) : (
        <div className="flex-1" aria-hidden />
      )}

      <div className="flex shrink-0 items-center gap-2.5 md:gap-3">
        {showSearch && (
          <div className="relative w-44 max-w-full md:w-52">
            <SearchIcon className="pointer-events-none absolute left-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-text-muted/70" />
            <input
              type="search"
              placeholder="Buscar"
              value={search}
              onChange={(e) => onSearchChange?.(e.target.value)}
              className="h-8 w-full rounded-full border-0 bg-base-800/50 py-0 pl-8 pr-3 text-xs text-text-primary outline-none transition-colors duration-200 placeholder:text-text-muted/60 focus:bg-base-800/80 focus:ring-1 focus:ring-accent/20"
            />
          </div>
        )}
      </div>
    </header>
  );
}
