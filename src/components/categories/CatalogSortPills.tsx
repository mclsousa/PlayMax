import type { CatalogSort } from "../../lib/api";

const SORT_OPTIONS: { value: CatalogSort; label: string }[] = [
  { value: "recent", label: "Recentes" },
  { value: "az", label: "A-Z" },
];

interface CatalogSortPillsProps {
  value: CatalogSort;
  onChange: (value: CatalogSort) => void;
}

export function CatalogSortPills({ value, onChange }: CatalogSortPillsProps) {
  return (
    <div
      className="flex shrink-0 items-center gap-0.5 rounded-full bg-base-800/50 p-0.5 ring-1 ring-base-700/40"
      role="group"
      aria-label="Ordenar conteúdo"
    >
      {SORT_OPTIONS.map((option) => {
        const selected = value === option.value;
        return (
          <button
            key={option.value}
            type="button"
            onClick={() => onChange(option.value)}
            aria-pressed={selected}
            className={`rounded-full px-2.5 py-1 text-[11px] font-medium transition-colors duration-200 ${
              selected
                ? "bg-base-700/90 text-text-primary shadow-sm"
                : "text-text-muted hover:text-text-secondary"
            }`}
          >
            {option.label}
          </button>
        );
      })}
    </div>
  );
}
