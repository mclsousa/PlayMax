import { useNavigate } from "react-router-dom";

import { FavoriteToggle } from "../components/favorites/FavoriteToggle";

import { PosterCard } from "../components/vod/PosterCard";

import { TopBar } from "../components/layout/TopBar";

import { Skeleton } from "../components/ui/Skeleton";

import { useFavorites } from "../hooks/useFavorites";

import { useActiveProfile } from "../hooks/useActiveProfile";
import { useProfiles } from "../hooks/useProfiles";

import type { FavoriteItem } from "../lib/types";



function favoriteHref(item: FavoriteItem): string {

  if (item.itemType === "movie") return `/movies/${item.itemId}`;

  if (item.itemType === "series") return `/series/${item.itemId}`;

  return "/live";

}



function ChannelFavoriteRow({

  item,

  onClick,

}: {

  item: FavoriteItem;

  onClick: () => void;

}) {

  return (

    <div className="flex items-center gap-2 rounded-xl border border-base-800 bg-base-900/40 p-2 transition-colors hover:border-base-700 hover:bg-base-850">

      <button

        type="button"

        onClick={onClick}

        className="flex min-w-0 flex-1 items-center gap-3 rounded-lg p-1 text-left"

      >

        <div className="flex h-11 w-11 shrink-0 items-center justify-center overflow-hidden rounded-lg bg-base-800 ring-1 ring-base-700/50">

          {item.poster ? (

            <img

              src={item.poster}

              alt=""

              loading="lazy"

              className="h-full w-full object-cover"

            />

          ) : (

            <span className="text-[10px] font-bold text-text-muted">TV</span>

          )}

        </div>

        <div className="min-w-0 flex-1">

          <p className="truncate text-sm font-medium text-text-primary">{item.name}</p>

          {item.category ? (

            <p className="mt-0.5 truncate text-[11px] text-text-muted">{item.category}</p>

          ) : null}

        </div>

      </button>

      <FavoriteToggle itemType="channel" itemId={item.itemId} size="sm" stopPropagation />

    </div>

  );

}



export function MyListPage() {

  const navigate = useNavigate();

  const { loading: profilesLoading } = useProfiles();
  const { profileId } = useActiveProfile();

  const { items, loading } = useFavorites(profileId);



  const channelItems = items.filter((item) => item.itemType === "channel");

  const vodItems = items.filter((item) => item.itemType !== "channel");



  const handleClick = (item: FavoriteItem) => {

    if (item.itemType === "channel") {

      navigate("/live", {

        state: {

          channelId: item.itemId,

          group: item.category ?? undefined,

        },

      });

      return;

    }

    navigate(favoriteHref(item), {
      state: {
        preview: {
          id: item.itemId,
          name: item.name,
          poster: item.poster ?? null,
          backdrop: null,
        },
      },
    });

  };



  const pageLoading = profilesLoading || (loading && items.length === 0);



  return (

    <div className="flex h-full flex-col overflow-hidden app-bg">

      <TopBar

        showSearch={false}

        title="Minha Lista"

        subtitle={

          profileId

            ? `${items.length.toLocaleString()} favoritos`

            : "Nenhuma lista configurada"

        }

      />

      <div className="scrollbar-thin flex-1 overflow-y-auto bg-base-950 px-8 py-8">

        {pageLoading ? (

          <div className="grid grid-cols-2 gap-4 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6">

            {Array.from({ length: 8 }).map((_, i) => (

              <Skeleton key={i} className="aspect-[2/3] w-full rounded-xl" />

            ))}

          </div>

        ) : items.length === 0 ? (

          <div className="flex flex-col items-center justify-center gap-3 py-20 text-center">

            <p className="text-sm text-text-secondary">Sua lista está vazia.</p>

            <p className="max-w-sm text-xs text-text-muted">

              Toque no coração em canais, filmes ou séries para adicioná-los aqui.

            </p>

          </div>

        ) : (

          <div className="space-y-8">

            {channelItems.length > 0 ? (

              <section className="space-y-3">

                <h2 className="text-sm font-semibold text-text-secondary">

                  Canais ao vivo ({channelItems.length})

                </h2>

                <div className="grid gap-2 md:grid-cols-2 xl:grid-cols-3">

                  {channelItems.map((item) => (

                    <ChannelFavoriteRow

                      key={item.id}

                      item={item}

                      onClick={() => handleClick(item)}

                    />

                  ))}

                </div>

              </section>

            ) : null}



            {vodItems.length > 0 ? (

              <section className="space-y-3">

                {channelItems.length > 0 ? (

                  <h2 className="text-sm font-semibold text-text-secondary">

                    Filmes e séries ({vodItems.length})

                  </h2>

                ) : null}

                <div className="grid grid-cols-2 gap-4 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6">

                  {vodItems.map((item, index) => (

                    <PosterCard

                      key={item.id}

                      title={item.name}

                      poster={item.poster}

                      colorIndex={index}

                      favoriteItemType={item.itemType}

                      favoriteItemId={item.itemId}

                      onClick={() => handleClick(item)}

                    />

                  ))}

                </div>

              </section>

            ) : null}

          </div>

        )}

      </div>

    </div>

  );

}


