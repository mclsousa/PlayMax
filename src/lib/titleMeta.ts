const YEAR_IN_PARENS = /\((\d{4})\)/;
const YEAR_TRAILING = /\b(19\d{2}|20\d{2})\b/;

export function extractYearFromTitle(title: string): string | null {
  const parenMatch = title.match(YEAR_IN_PARENS);
  if (parenMatch) return parenMatch[1];

  const trailingMatch = title.match(YEAR_TRAILING);
  if (trailingMatch) return trailingMatch[1];

  return null;
}

export function formatRating(rating: string | null | undefined): string | null {
  if (!rating?.trim()) return null;
  const value = rating.trim();
  if (value.startsWith("★")) return value;
  const numeric = Number.parseFloat(value);
  if (!Number.isNaN(numeric) && numeric > 0) {
    return `★ ${numeric.toFixed(1)}`;
  }
  return `★ ${value}`;
}
