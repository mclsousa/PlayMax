use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub profile_type: String,
    pub url: Option<String>,
    pub file_path: Option<String>,
    pub username: Option<String>,
    #[serde(skip_serializing)]
    pub password: Option<String>,
    pub last_sync: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Channel {
    pub id: String,
    pub profile_id: String,
    pub name: String,
    pub logo: Option<String>,
    pub group_name: Option<String>,
    pub stream_url: String,
    pub tvg_id: Option<String>,
    pub sort_order: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelsPage {
    pub items: Vec<Channel>,
    pub total: i64,
    pub offset: i64,
    pub limit: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryCount {
    pub name: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Movie {
    pub id: String,
    pub profile_id: String,
    pub name: String,
    pub poster: Option<String>,
    pub backdrop: Option<String>,
    pub plot: Option<String>,
    pub genres: Option<String>,
    pub rating: Option<String>,
    pub stream_url: String,
    pub category: Option<String>,
    pub added_at: i64,
    pub sort_order: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Series {
    pub id: String,
    pub profile_id: String,
    pub name: String,
    pub poster: Option<String>,
    pub backdrop: Option<String>,
    pub plot: Option<String>,
    pub genres: Option<String>,
    pub rating: Option<String>,
    pub category: Option<String>,
    pub added_at: i64,
    pub sort_order: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CastMember {
    pub name: String,
    pub photo: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecommendedItem {
    pub id: String,
    pub name: String,
    pub poster: Option<String>,
    pub backdrop: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnrichedMovie {
    #[serde(flatten)]
    pub movie: Movie,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cast: Option<Vec<CastMember>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub similar: Option<Vec<RecommendedItem>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Episode {
    pub id: String,
    pub series_id: String,
    pub season: i32,
    pub episode: i32,
    pub title: String,
    pub plot: Option<String>,
    pub stream_url: String,
    pub duration: Option<i32>,
    pub added_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MoviesPage {
    pub items: Vec<Movie>,
    pub total: i64,
    pub offset: i64,
    pub limit: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeriesPage {
    pub items: Vec<Series>,
    pub total: i64,
    pub offset: i64,
    pub limit: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeriesDetail {
    pub series: Series,
    pub episodes: Vec<Episode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cast: Option<Vec<CastMember>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub similar: Option<Vec<RecommendedItem>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogItem {
    pub id: String,
    pub item_type: String,
    pub name: String,
    pub poster: Option<String>,
    pub backdrop: Option<String>,
    pub added_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncProgress {
    pub percent: f64,
    pub message: String,
    pub profile_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Favorite {
    pub id: String,
    pub profile_id: String,
    pub item_type: String,
    pub item_id: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FavoriteItem {
    pub id: String,
    pub profile_id: String,
    pub item_type: String,
    pub item_id: String,
    pub name: String,
    pub poster: Option<String>,
    pub category: Option<String>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub id: String,
    pub item_type: String,
    pub name: String,
    pub poster: Option<String>,
    pub category: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub id: String,
    pub profile_id: String,
    pub item_type: String,
    pub item_id: String,
    pub name: String,
    pub poster: Option<String>,
    pub stream_url: String,
    pub position: f64,
    pub duration: f64,
    pub watched_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EpgProgram {
    pub title: String,
    pub description: Option<String>,
    pub start_ts: i64,
    pub end_ts: i64,
    pub epg_id: Option<String>,
    pub is_now: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelEpg {
    pub available: bool,
    pub unavailable_reason: Option<String>,
    pub programs: Vec<EpgProgram>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub auto_sync_enabled: bool,
    pub last_background_sync: Option<i64>,
    pub parental_control_enabled: bool,
    pub parental_pin_set: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HomeCatalog {
    pub release_movies: Vec<CatalogItem>,
    pub recent_movies: Vec<CatalogItem>,
    pub recent_series: Vec<CatalogItem>,
    pub updated_series: Vec<CatalogItem>,
    pub release_series: Vec<CatalogItem>,
    pub featured_movies: Vec<CatalogItem>,
}
