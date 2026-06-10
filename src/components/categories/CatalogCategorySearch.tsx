import { SearchIcon } from "../layout/icons";

interface CatalogCategorySearchProps {
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
  disabled?: boolean;
}

export function CatalogCategorySearch({
  value,
  onChange,
  placeholder = "Buscar nesta categoria",
  disabled = false,
}: CatalogCategorySearchProps) {
  return (
    <div className="relative w-full">
      <SearchIcon className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-text-muted/70" />
      <input
        type="search"
        value={value}
        disabled={disabled}
        placeholder={placeholder}
        onChange={(event) => onChange(event.target.value)}
        className="h-9 w-full rounded-full border border-base-800/80 bg-base-900/60 py-0 pl-9 pr-4 text-sm text-text-primary outline-none transition-colors duration-200 placeholder:text-text-muted/60 focus:border-accent/30 focus:bg-base-900 focus:ring-1 focus:ring-accent/20 disabled:cursor-not-allowed disabled:opacity-50"
      />
    </div>
  );
}
