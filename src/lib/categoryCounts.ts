import type { CategoryCount } from "./types";

export function categoryCountsFromList(items: CategoryCount[]): Record<string, number> {
  const counts: Record<string, number> = {};
  let total = 0;

  for (const item of items) {
    counts[item.name] = item.count;
    total += item.count;
  }

  counts.all = total;
  return counts;
}
