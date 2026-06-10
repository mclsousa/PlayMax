import {

  createContext,

  useCallback,

  useContext,

  useEffect,

  useMemo,

  useRef,

  useState,

  type ReactNode,

} from "react";

import * as api from "../lib/api";

import type { FavoriteItem } from "../lib/types";

import { useProfilesContext } from "./ProfilesContext";



function favoriteKey(itemType: FavoriteItem["itemType"], itemId: string): string {

  return `${itemType}:${itemId}`;

}



interface FavoritesContextValue {

  items: FavoriteItem[];

  favoriteKeys: Set<string>;

  loading: boolean;

  refresh: (options?: { silent?: boolean }) => Promise<void>;

  toggleFavorite: (

    itemType: "movie" | "series" | "channel",

    itemId: string,

    currentlyFavorite: boolean,

  ) => Promise<void>;

}



const FavoritesContext = createContext<FavoritesContextValue | null>(null);



export function FavoritesProvider({ children }: { children: ReactNode }) {

  const { activeProfileId: profileId } = useProfilesContext();

  const [items, setItems] = useState<FavoriteItem[]>([]);

  const [loading, setLoading] = useState(false);

  const loadedRef = useRef(false);



  const favoriteKeys = useMemo(

    () => new Set(items.map((item) => favoriteKey(item.itemType, item.itemId))),

    [items],

  );



  const refresh = useCallback(async (options?: { silent?: boolean }) => {

    if (!profileId) {

      setItems([]);

      loadedRef.current = false;

      setLoading(false);

      return;

    }

    const showLoading = !options?.silent && !loadedRef.current;

    if (showLoading) {

      setLoading(true);

    }

    try {

      const data = await api.listFavorites(profileId);

      setItems(data);

      loadedRef.current = true;

    } catch {

      if (!loadedRef.current) {

        setItems([]);

      }

    } finally {

      if (showLoading) {

        setLoading(false);

      }

    }

  }, [profileId]);



  useEffect(() => {

    void refresh();

  }, [refresh]);



  const toggleFavorite = useCallback(

    async (

      itemType: "movie" | "series" | "channel",

      itemId: string,

      currentlyFavorite: boolean,

    ) => {

      if (!profileId) return;

      const key = favoriteKey(itemType, itemId);



      if (currentlyFavorite) {

        let previous: FavoriteItem[] = [];

        setItems((current) => {

          previous = current;

          return current.filter(

            (item) => favoriteKey(item.itemType, item.itemId) !== key,

          );

        });

        try {

          await api.removeFavorite(profileId, itemType, itemId);

        } catch {

          setItems(previous);

        }

        return;

      }



      try {

        await api.addFavorite(profileId, itemType, itemId);

        await refresh({ silent: true });

      } catch {

        // Keep list unchanged on failure.

      }

    },

    [profileId, refresh],

  );



  const value = useMemo(

    () => ({ items, favoriteKeys, loading, refresh, toggleFavorite }),

    [items, favoriteKeys, loading, refresh, toggleFavorite],

  );



  return (

    <FavoritesContext.Provider value={value}>

      {children}

    </FavoritesContext.Provider>

  );

}



export function useFavoritesContext(): FavoritesContextValue {

  const ctx = useContext(FavoritesContext);

  if (!ctx) {

    throw new Error("useFavorites must be used within FavoritesProvider");

  }

  return ctx;

}



export { favoriteKey };

