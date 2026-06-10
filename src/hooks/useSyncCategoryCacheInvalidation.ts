import { listen } from "@tauri-apps/api/event";
import { useEffect } from "react";

import { clearCategoryCache } from "../lib/categoryCache";
import { clearHeroCache } from "../lib/heroCache";
import { clearHomeCache } from "../lib/homeCache";
import type { SyncProgress } from "../lib/types";

export function useSyncCategoryCacheInvalidation() {
  useEffect(() => {
    const unlisten = listen<SyncProgress>("sync-progress", (event) => {
      const { percent, profileId } = event.payload;
      if (percent >= 100 && profileId) {
        clearCategoryCache(profileId);
        clearHomeCache(profileId);
        clearHeroCache(profileId);
      }
    });

    return () => {
      void unlisten.then((fn) => fn());
    };
  }, []);
}
