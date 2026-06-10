export interface Profile {
  id: string;
  name: string;
  type: string;
  url?: string | null;
  filePath?: string | null;
  username?: string | null;
  lastSync?: number | null;
}

export interface AppSettings {
  autoSyncEnabled: boolean;
  lastBackgroundSync?: number | null;
  parentalControlEnabled: boolean;
  parentalPinSet: boolean;
}

export interface HomeCatalog {
  releaseMovies: CatalogItem[];
  recentMovies: CatalogItem[];
  recentSeries: CatalogItem[];
  updatedSeries: CatalogItem[];
  releaseSeries: CatalogItem[];
  featuredMovies: CatalogItem[];
}

export interface LicenseState {
  licenseKey?: string | null;
  status?: string | null;
  expiresAt?: number | null;
  lastValidatedAt?: number | null;
  valid: boolean;
  message?: string | null;
}

export interface Channel {
  id: string;
  profileId: string;
  name: string;
  logo?: string | null;
  groupName?: string | null;
  streamUrl: string;
  tvgId?: string | null;
  sortOrder: number;
}

export interface ChannelsPage {
  items: Channel[];
  total: number;
  offset: number;
  limit: number;
}

export interface CategoryCount {
  name: string;
  count: number;
}

export interface CastMember {
  name: string;
  photo?: string | null;
}

export interface RecommendedItem {
  id: string;
  name: string;
  poster?: string | null;
  backdrop?: string | null;
}

export interface Movie {
  id: string;
  profileId: string;
  name: string;
  poster?: string | null;
  backdrop?: string | null;
  plot?: string | null;
  genres?: string | null;
  rating?: string | null;
  streamUrl: string;
  category?: string | null;
  addedAt: number;
  sortOrder: number;
  /** Elenco enriquecido via TMDB (somente em getMovie). */
  cast?: CastMember[] | null;
  /** Títulos semelhantes do catálogo IPTV (somente em getMovie). */
  similar?: RecommendedItem[] | null;
}

export interface Series {
  id: string;
  profileId: string;
  name: string;
  poster?: string | null;
  backdrop?: string | null;
  plot?: string | null;
  genres?: string | null;
  rating?: string | null;
  category?: string | null;
  addedAt: number;
  sortOrder: number;
}

export interface Episode {
  id: string;
  seriesId: string;
  season: number;
  episode: number;
  title: string;
  plot?: string | null;
  streamUrl: string;
  duration?: number | null;
  addedAt: number;
}

export interface MoviesPage {
  items: Movie[];
  total: number;
  offset: number;
  limit: number;
}

export interface SeriesPage {
  items: Series[];
  total: number;
  offset: number;
  limit: number;
}

export interface SeriesDetail {
  series: Series;
  episodes: Episode[];
  cast?: CastMember[] | null;
  similar?: RecommendedItem[] | null;
}

export interface CatalogItem {
  id: string;
  itemType: string;
  name: string;
  poster?: string | null;
  backdrop?: string | null;
  addedAt: number;
}

export interface SyncProgress {
  percent: number;
  message: string;
  profileId: string;
}

export interface HistoryEntry {
  id: string;
  profileId: string;
  itemType: "movie" | "episode" | "live";
  itemId: string;
  name: string;
  poster?: string | null;
  streamUrl: string;
  position: number;
  duration: number;
  watchedAt: number;
}

export interface RecentChannel {
  id: string;
  profileId: string;
  channelId: string;
  name: string;
  logo?: string | null;
  streamUrl: string;
  watchedAt: number;
}

export interface VodNavigationItem {
  itemId: string;
  name: string;
  streamUrl: string;
}

export interface VodPlayInfo {
  name: string;
  streamUrl: string;
  profileId: string;
  itemType: "movie" | "episode";
  itemId: string;
  poster?: string | null;
  startPosition?: number;
  seriesId?: string;
  season?: number;
  episodeNumber?: number;
  autoplayQueue?: AutoplayEpisode[];
  navItems?: VodNavigationItem[];
  navIndex?: number;
  /** Abre a janela em tela cheia ao iniciar a reprodução. */
  startFullscreen?: boolean;
}

export interface AutoplayEpisode {
  itemId: string;
  name: string;
  streamUrl: string;
}

export interface FavoriteItem {
  id: string;
  profileId: string;
  itemType: "movie" | "series" | "channel";
  itemId: string;
  name: string;
  poster?: string | null;
  category?: string | null;
  createdAt: number;
}

export interface SearchResult {
  id: string;
  itemType: "movie" | "series" | "channel";
  name: string;
  poster?: string | null;
  category?: string | null;
}

export interface EpgProgram {
  title: string;
  description?: string | null;
  startTs: number;
  endTs: number;
  epgId?: string | null;
  isNow: boolean;
}

export interface ChannelEpg {
  available: boolean;
  unavailableReason?: string | null;
  programs: EpgProgram[];
}
