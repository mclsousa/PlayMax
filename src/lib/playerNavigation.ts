import type { Channel, Movie, VodNavigationItem, VodPlayInfo } from "./types";
import type { Episode } from "./types";

export function buildChannelNavigation(channels: Channel[], playingId: string | null) {
  const currentIndex = playingId
    ? channels.findIndex((channel) => channel.id === playingId)
    : -1;
  return {
    canPrev: currentIndex > 0,
    canNext: currentIndex >= 0 && currentIndex < channels.length - 1,
    goPrev: (onPlay: (channel: Channel) => void) => {
      if (currentIndex > 0) onPlay(channels[currentIndex - 1]);
    },
    goNext: (onPlay: (channel: Channel) => void) => {
      if (currentIndex >= 0 && currentIndex < channels.length - 1) {
        onPlay(channels[currentIndex + 1]);
      }
    },
  };
}

export function buildMovieNavItems(movies: Movie[]): VodNavigationItem[] {
  return movies.map((movie) => ({
    itemId: movie.id,
    name: movie.name,
    streamUrl: movie.streamUrl,
  }));
}

export function moviePlayInfo(movie: Movie, movies: Movie[]): Omit<VodPlayInfo, "itemType"> & {
  itemType: "movie";
} {
  const navItems = buildMovieNavItems(movies);
  const navIndex = navItems.findIndex((item) => item.itemId === movie.id);
  return {
    name: movie.name,
    streamUrl: movie.streamUrl,
    profileId: movie.profileId,
    itemType: "movie",
    itemId: movie.id,
    poster: movie.poster,
    navItems,
    navIndex: navIndex >= 0 ? navIndex : undefined,
  };
}

export function buildEpisodeNavItems(
  episodes: Episode[],
  _seriesName: string,
  season?: number,
): VodNavigationItem[] {
  const filtered =
    season != null
      ? episodes.filter((episode) => episode.season === season)
      : episodes;
  const sorted = [...filtered].sort((a, b) => {
    if (a.season !== b.season) return a.season - b.season;
    return a.episode - b.episode;
  });
  return sorted.map((ep) => ({
    itemId: ep.id,
    name: `T${ep.season} E${ep.episode}`,
    streamUrl: ep.streamUrl,
  }));
}
