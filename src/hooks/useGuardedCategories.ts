import { useCallback, useMemo } from "react";
import { useParentalControl } from "../contexts/ParentalControlContext";

export function useGuardedCategories(groups: string[]) {
  const { isCategoryBlocked, requestAccess } = useParentalControl();

  const lockedIds = useMemo(() => {
    const ids = new Set<string>();
    for (const group of groups) {
      if (isCategoryBlocked(group)) {
        ids.add(group);
      }
    }
    return ids;
  }, [groups, isCategoryBlocked]);

  const guardSelect = useCallback(
    (categoryId: string, onSelect: (categoryId: string) => void) => {
      if (isCategoryBlocked(categoryId)) {
        requestAccess(() => onSelect(categoryId));
        return;
      }
      onSelect(categoryId);
    },
    [isCategoryBlocked, requestAccess],
  );

  return { lockedIds, guardSelect };
}
