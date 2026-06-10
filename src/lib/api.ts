import { tauriInvoke } from "./tauri";
import type {
  AppSettings,
  CatalogItem,
  ChannelEpg,
  Channel,
  ChannelsPage,
  FavoriteItem,
  HistoryEntry,
  Movie,
  MoviesPage,
  Profile,
  RecentChannel,
  SearchResult,
  SeriesDetail,
  SeriesPage,
  CategoryCount,
} from "./types";

export function listProfiles(): Promise<Profile[]> {
  return tauriInvoke<Profile[]>("list_profiles");
}

export function getActiveProfileId(): Promise<string | null> {
  return tauriInvoke<string | null>("get_active_profile_id");
}

export function setActiveProfileId(profileId: string): Promise<void> {
  return tauriInvoke("set_active_profile_id", { profileId });
}

export function addM3uProfile(
  name: string,
  url?: string,
  filePath?: string,
): Promise<Profile> {
  return tauriInvoke<Profile>("add_m3u_profile", { name, url, filePath });
}

export function removeProfile(profileId: string): Promise<void> {
  return tauriInvoke("remove_profile", { profileId });
}

export function syncProfile(profileId: string): Promise<void> {
  return tauriInvoke("sync_profile", { profileId });
}

export function syncProfileBlocking(profileId: string): Promise<void> {
  return tauriInvoke("sync_profile_blocking", { profileId });
}

export function listChannels(params: {
  profileId: string;
  group?: string;
  search?: string;
  offset?: number;
  limit?: number;
}): Promise<ChannelsPage> {
  return tauriInvoke<ChannelsPage>("list_channels", params);
}

export function getChannel(channelId: string): Promise<Channel | null> {
  return tauriInvoke<Channel | null>("get_channel", { channelId });
}

export function listGroups(
  profileId: string,
  contentKind?: "live" | "all",
): Promise<CategoryCount[]> {
  return tauriInvoke<CategoryCount[]>("list_groups", {
    profileId,
    contentKind: contentKind === "live" ? "live" : undefined,
  });
}

export function pickM3uFile(): Promise<string | null> {
  return tauriInvoke<string | null>("pick_m3u_file");
}

export function addXtreamProfile(
  name: string,
  url: string,
  username: string,
  password: string,
): Promise<Profile> {
  return tauriInvoke<Profile>("add_xtream_profile", {
    name,
    url,
    username,
    password,
  });
}

export type CatalogSort = "recent" | "az";

export function listMovies(params: {
  profileId: string;
  category?: string;
  search?: string;
  sort?: CatalogSort;
  offset?: number;
  limit?: number;
}): Promise<MoviesPage> {
  return tauriInvoke<MoviesPage>("list_movies", params);
}

export function listMovieCategories(profileId: string): Promise<CategoryCount[]> {
  return tauriInvoke<CategoryCount[]>("list_movie_categories", { profileId });
}

export function getMovie(
  id: string,
  options?: { includeSimilar?: boolean; enrich?: boolean },
): Promise<Movie> {
  return tauriInvoke<Movie>("get_movie", {
    id,
    includeSimilar: options?.includeSimilar,
    enrich: options?.enrich,
  });
}

export function listSeries(params: {
  profileId: string;
  category?: string;
  search?: string;
  sort?: CatalogSort;
  offset?: number;
  limit?: number;
}): Promise<SeriesPage> {
  return tauriInvoke<SeriesPage>("list_series", params);
}

export function listSeriesCategories(profileId: string): Promise<CategoryCount[]> {
  return tauriInvoke<CategoryCount[]>("list_series_categories", { profileId });
}

export function getSeriesDetail(
  id: string,
  options?: { enrich?: boolean; includeSimilar?: boolean; syncEpisodes?: boolean },
): Promise<SeriesDetail> {
  return tauriInvoke<SeriesDetail>("get_series_detail", {
    id,
    enrich: options?.enrich,
    includeSimilar: options?.includeSimilar,
    syncEpisodes: options?.syncEpisodes,
  });
}

export function listRecent(
  profileId: string,
  limit?: number,
): Promise<CatalogItem[]> {
  return tauriInvoke<CatalogItem[]>("list_recent", { profileId, limit });
}

export function listRecentMovies(
  profileId: string,
  limit?: number,
): Promise<CatalogItem[]> {
  return tauriInvoke<CatalogItem[]>("list_recent_movies", { profileId, limit });
}

export function listRecentSeries(
  profileId: string,
  limit?: number,
): Promise<CatalogItem[]> {
  return tauriInvoke<CatalogItem[]>("list_recent_series", { profileId, limit });
}

export function listReleases(
  profileId: string,
  limit?: number,
): Promise<CatalogItem[]> {
  return tauriInvoke<CatalogItem[]>("list_releases", { profileId, limit });
}

export function listReleasesMovies(
  profileId: string,
  limit?: number,
): Promise<CatalogItem[]> {
  return tauriInvoke<CatalogItem[]>("list_releases_movies", { profileId, limit });
}

export function listReleasesSeries(
  profileId: string,
  limit?: number,
): Promise<CatalogItem[]> {
  return tauriInvoke<CatalogItem[]>("list_releases_series", { profileId, limit });
}

export function listFeaturedMovies(
  profileId: string,
  limit?: number,
): Promise<CatalogItem[]> {
  return tauriInvoke<CatalogItem[]>("list_featured_movies", { profileId, limit });
}

export function saveHistory(params: {
  profileId: string;
  itemType: "movie" | "episode" | "live";
  itemId: string;
  name: string;
  poster?: string | null;
  streamUrl: string;
  position: number;
  duration: number;
}): Promise<void> {
  return tauriInvoke("save_history", params);
}

export function listContinueWatching(
  profileId: string,
  limit?: number,
): Promise<HistoryEntry[]> {
  return tauriInvoke<HistoryEntry[]>("list_continue_watching", {
    profileId,
    limit,
  });
}

export function listRecentChannels(
  profileId: string,
  limit?: number,
): Promise<RecentChannel[]> {
  return tauriInvoke<HistoryEntry[]>("list_recent_channels", {
    profileId,
    limit,
  }).then((entries) =>
    entries.map((entry) => ({
      id: entry.id,
      profileId: entry.profileId,
      channelId: entry.itemId,
      name: entry.name,
      logo: entry.poster,
      streamUrl: entry.streamUrl,
      watchedAt: entry.watchedAt,
    })),
  );
}

export function addFavorite(
  profileId: string,
  itemType: "movie" | "series" | "channel",
  itemId: string,
): Promise<void> {
  return tauriInvoke("add_favorite", { profileId, itemType, itemId });
}

export function removeFavorite(
  profileId: string,
  itemType: "movie" | "series" | "channel",
  itemId: string,
): Promise<void> {
  return tauriInvoke("remove_favorite", { profileId, itemType, itemId });
}

export function listFavorites(profileId: string): Promise<FavoriteItem[]> {
  return tauriInvoke<FavoriteItem[]>("list_favorites", { profileId });
}

export function isFavorite(
  profileId: string,
  itemType: "movie" | "series" | "channel",
  itemId: string,
): Promise<boolean> {
  return tauriInvoke<boolean>("is_favorite", { profileId, itemType, itemId });
}

export function searchCatalog(
  profileId: string,
  query: string,
  limit?: number,
): Promise<SearchResult[]> {
  return tauriInvoke<SearchResult[]>("search_catalog", { profileId, query, limit });
}

export function getChannelEpg(
  profileId: string,
  channelId: string,
  limit?: number,
): Promise<ChannelEpg> {
  return tauriInvoke<ChannelEpg>("get_channel_epg", { profileId, channelId, limit });
}

export function getAppSettings(): Promise<AppSettings> {
  return tauriInvoke<AppSettings>("get_app_settings");
}

export function getHomeCatalog(
  profileId: string,
  releaseMoviesLimit = 40,
  catalogLimit = 20,
): Promise<import("./types").HomeCatalog> {
  return tauriInvoke("get_home_catalog", {
    profileId,
    releaseMoviesLimit,
    catalogLimit,
  });
}

export function verifyParentalPin(pin: string): Promise<boolean> {
  return tauriInvoke<boolean>("verify_parental_pin", { pin });
}

export function updateAppSettings(params: {
  autoSyncEnabled?: boolean;
  parentalControlEnabled?: boolean;
  parentalPin?: string;
}): Promise<AppSettings> {
  return tauriInvoke<AppSettings>("update_app_settings", params);
}

export function getLicenseState(): Promise<import("./types").LicenseState> {
  return tauriInvoke("get_license_state");
}

export function activateLicense(
  apiBaseUrl: string,
  licenseKey: string,
): Promise<import("./types").LicenseState> {
  return tauriInvoke("activate_license", { apiBaseUrl, licenseKey });
}

export function validateLicense(
  apiBaseUrl: string,
): Promise<import("./types").LicenseState> {
  return tauriInvoke("validate_license", { apiBaseUrl });
}

export function clearLicense(): Promise<void> {
  return tauriInvoke("clear_license");
}

export async function profileHasCatalog(profileId: string): Promise<boolean> {
  const [movies, channelsPage] = await Promise.all([
    listRecentMovies(profileId, 1),
    listChannels({ profileId, limit: 1 }),
  ]);
  return movies.length > 0 || channelsPage.total > 0;
}

export function updateM3uProfile(
  profileId: string,
  name: string,
  url?: string,
  filePath?: string,
): Promise<Profile> {
  return tauriInvoke("update_m3u_profile", { profileId, name, url, filePath });
}

export function updateXtreamProfile(
  profileId: string,
  name: string,
  url: string,
  username: string,
  password: string,
): Promise<Profile> {
  return tauriInvoke("update_xtream_profile", {
    profileId,
    name,
    url,
    username,
    password,
  });
}
