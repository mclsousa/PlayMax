import { favoriteKey, useFavoritesContext } from "../contexts/FavoritesContext";



export function useFavorites(_profileId: string | null) {

  const { items, loading, refresh, toggleFavorite } = useFavoritesContext();

  return { items, loading, refresh, toggleFavorite };

}



export function useIsFavorite(

  profileId: string | null,

  itemType: "movie" | "series" | "channel",

  itemId: string | undefined,

) {

  const { favoriteKeys, loading, items } = useFavoritesContext();



  const favorite =

    profileId && itemId

      ? favoriteKeys.has(favoriteKey(itemType, itemId))

      : false;



  return {

    favorite,

    loading: Boolean(profileId && itemId && loading && items.length === 0),

  };

}

