use crate::db::catalog_filters::{is_junk_series_name, is_valid_vod_movie};
use crate::services::content_kind::is_channel_like_name;
use crate::db::ids::{stable_channel_id, stable_episode_id, stable_movie_id, stable_series_id};
use crate::db::models::{Channel, Episode, Movie, Profile, Series};
use crate::db::{apply_bulk_write_pragmas, catalog_fts, channels, favorites, movies, restore_write_pragmas, series, series_tracking, SharedDb};
use crate::error::AppResult;
use crate::parsers::m3u::series_parser::parse_series_title;
use crate::services::content_kind::{
    classify_m3u_entry, looks_like_series_entry, ContentKind,
};
use crate::parsers::xtream::XtreamClient;
use crate::parsers::xtream::{
    XtreamEpisode, XtreamLiveStream, XtreamSeries, XtreamSeriesInfo, XtreamVodStream,
};
use std::collections::HashMap;
use uuid::Uuid;

const BATCH_SIZE: usize = 2_000;
const CLASSIFY_PROGRESS_INTERVAL: usize = 8_192;

fn first_nonempty_url(values: &[Option<String>]) -> Option<String> {
    values
        .iter()
        .find_map(|value| value.as_ref().filter(|url| !url.trim().is_empty()).cloned())
}

/// Prefer highest-quality poster fields from Xtream VOD/series payloads.
fn pick_xtream_poster(
    cover_big: &Option<String>,
    cover: &Option<String>,
    movie_image: &Option<String>,
    stream_icon: &Option<String>,
) -> Option<String> {
    first_nonempty_url(&[
        cover_big.clone(),
        cover.clone(),
        movie_image.clone(),
        stream_icon.clone(),
    ])
}

/// Prefer backdrop, then fall back to large cover art.
fn pick_xtream_backdrop(
    backdrop_path: &Option<String>,
    cover_big: &Option<String>,
    cover: &Option<String>,
) -> Option<String> {
    first_nonempty_url(&[
        backdrop_path.clone(),
        cover_big.clone(),
        cover.clone(),
    ])
}

#[derive(Clone)]
pub struct CatalogSyncResult {
    pub channels: Vec<Channel>,
    pub movies: Vec<Movie>,
    pub series: Vec<Series>,
    pub episodes: Vec<Episode>,
}

/// Detects content type from an Xtream-style stream URL path segment.
///
/// Xtream-backed M3U (`type=m3u_plus`) encodes the content type directly in the
/// URL: `.../movie/user/pass/ID.ext`, `.../series/user/pass/ID.ext`, and live
/// channels as `.../user/pass/ID`. This is far more reliable than guessing from
/// group names. Returns `None` for generic M3U URLs so the caller falls back to
/// the name/group heuristic.
fn contains_ignore_ascii_case(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() || haystack.len() < needle.len() {
        return false;
    }
    haystack
        .windows(needle.len())
        .any(|window| window.eq_ignore_ascii_case(needle))
}

fn is_vod_stream_url(url: &str) -> bool {
    let bytes = url.as_bytes();
    contains_ignore_ascii_case(bytes, b"/movie/")
        || contains_ignore_ascii_case(bytes, b"/series/")
}

/// Xtream-backed M3U live URLs (`/live/user/pass/id` or `/user/pass/id`) must stay
/// on the live page even when the title looks like a movie (year in name, etc.).
fn is_xtream_live_path_url(url: &str) -> bool {
    if is_vod_stream_url(url) {
        return false;
    }
    if parse_xtream_credentials_from_live_url(url).is_some() {
        return true;
    }

    let path = url.split('?').next().unwrap_or(url);
    let after_host = path
        .split("//")
        .nth(1)
        .map(|rest| rest.split('/').skip(1).collect::<Vec<_>>())
        .unwrap_or_default();
    if after_host.len() < 3 {
        return false;
    }

    after_host
        .last()
        .and_then(|segment| segment.split('.').next())
        .is_some_and(|id| !id.is_empty() && id.chars().all(|c| c.is_ascii_digit()))
}

fn classify_stream_url(url: &str) -> Option<ContentKind> {
    let bytes = url.as_bytes();
    if contains_ignore_ascii_case(bytes, b"/movie/") {
        Some(ContentKind::Movie)
    } else if contains_ignore_ascii_case(bytes, b"/series/") {
        Some(ContentKind::Series)
    } else {
        None
    }
}

/// Extracts the trailing numeric stream id from an Xtream URL to use as a
/// monotonic recency proxy (higher id = added more recently on the provider).
/// M3U has no real timestamps, so this gives meaningful "recent/releases" order.
pub fn parse_trailing_stream_id(url: &str) -> Option<i64> {
    let last = url.rsplit('/').next()?;
    let digits = last.split('.').next()?;
    if digits.is_empty() {
        return None;
    }
    digits.parse::<i64>().ok()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XtreamCredentials {
    pub base_url: String,
    pub username: String,
    pub password: String,
}

fn decode_url_component(value: &str) -> String {
    urlencoding::decode(value)
        .map(|cow| cow.into_owned())
        .unwrap_or_else(|_| value.to_string())
}

fn parse_query_param(query: &str, key: &str) -> Option<String> {
    for pair in query.split('&') {
        let mut parts = pair.splitn(2, '=');
        let name = parts.next()?;
        if name.eq_ignore_ascii_case(key) {
            let raw = parts.next().unwrap_or("");
            return Some(decode_url_component(raw));
        }
    }
    None
}

fn normalize_xtream_base_url(base: &str) -> String {
    let mut normalized = base.trim_end_matches('/').to_string();
    for suffix in ["/get.php", "/xmltv.php", "/player_api.php"] {
        if normalized.len() >= suffix.len()
            && normalized[normalized.len() - suffix.len()..].eq_ignore_ascii_case(suffix)
        {
            normalized.truncate(normalized.len() - suffix.len());
            normalized = normalized.trim_end_matches('/').to_string();
        }
    }
    normalized
}

/// Parses Xtream credentials embedded in a live stream URL:
/// `http://host:port/live/username/password/12345.ts`
pub fn parse_xtream_credentials_from_live_url(url: &str) -> Option<XtreamCredentials> {
    let lower = url.to_lowercase();
    let live_idx = lower.find("/live/")?;
    let base_url = normalize_xtream_base_url(&url[..live_idx]);
    if base_url.is_empty()
        || (!base_url.starts_with("http://") && !base_url.starts_with("https://"))
    {
        return None;
    }

    let after_live = url[live_idx + 6..].split('?').next()?.trim_matches('/');
    let segments: Vec<&str> = after_live.split('/').collect();
    if segments.len() < 3 {
        return None;
    }

    let username = decode_url_component(segments[0]);
    let password = decode_url_component(segments[1]);
    if username.is_empty() || password.is_empty() {
        return None;
    }

    Some(XtreamCredentials {
        base_url,
        username,
        password,
    })
}

/// Parses Xtream credentials from an M3U playlist URL:
/// `http://host:port/get.php?username=USER&password=PASS&type=m3u_plus`
pub fn parse_xtream_credentials_from_m3u_url(url: &str) -> Option<XtreamCredentials> {
    let trimmed = url.trim();
    let query_start = trimmed.find('?')?;
    let base_url = normalize_xtream_base_url(&trimmed[..query_start]);
    if base_url.is_empty()
        || (!base_url.starts_with("http://") && !base_url.starts_with("https://"))
    {
        return None;
    }

    let query = &trimmed[query_start + 1..];
    let username = parse_query_param(query, "username")?;
    let password = parse_query_param(query, "password")?;
    if username.is_empty() || password.is_empty() {
        return None;
    }

    Some(XtreamCredentials {
        base_url,
        username,
        password,
    })
}

/// Resolves Xtream API credentials and stream id for EPG fetch.
/// Supports native Xtream profiles and Xtream-backed M3U playlists.
pub fn resolve_xtream_epg_access(
    profile: &Profile,
    channel: &Channel,
) -> Option<(XtreamCredentials, i64)> {
    let stream_id = parse_trailing_stream_id(&channel.stream_url)?;

    if profile.profile_type == "xtream" {
        let url = profile.url.as_deref()?.trim();
        let username = profile.username.as_deref()?.trim();
        let password = profile.password.as_deref()?.trim();
        if url.is_empty() || username.is_empty() || password.is_empty() {
            return None;
        }
        return Some((
            XtreamCredentials {
                base_url: url.to_string(),
                username: username.to_string(),
                password: password.to_string(),
            },
            stream_id,
        ));
    }

    if profile.profile_type == "m3u" {
        if let Some(creds) = parse_xtream_credentials_from_live_url(&channel.stream_url) {
            return Some((creds, stream_id));
        }
        if let Some(profile_url) = profile.url.as_deref() {
            if let Some(creds) = parse_xtream_credentials_from_m3u_url(profile_url) {
                return Some((creds, stream_id));
            }
        }
    }

    None
}

/// Best-effort "added at" value for an M3U entry: stream id when available,
/// otherwise the line position in the playlist.
fn m3u_added_at(url: &str, sort_order: i32) -> i64 {
    parse_trailing_stream_id(url).unwrap_or(sort_order as i64)
}

struct SeriesBucket {
    series: Series,
    episodes: Vec<Episode>,
    next_episode_num: i32,
}

pub fn classify_m3u_entries(entries: Vec<Channel>) -> CatalogSyncResult {
    classify_m3u_entries_with_progress(entries, |_, _| {})
}

pub fn classify_m3u_entries_with_progress<F>(
    entries: Vec<Channel>,
    mut on_progress: F,
) -> CatalogSyncResult
where
    F: FnMut(usize, usize),
{
    let total = entries.len();
    let mut channels = Vec::with_capacity(total / 2);
    let mut movies = Vec::new();
    let mut series_map: HashMap<String, SeriesBucket> = HashMap::new();

    for (index, entry) in entries.into_iter().enumerate() {
        if index > 0 && index % CLASSIFY_PROGRESS_INTERVAL == 0 {
            on_progress(index, total);
        }

        let group = entry.group_name.as_deref().unwrap_or("");
        let parsed = parse_series_title(&entry.name);
        let has_episode = parsed.is_some() || looks_like_series_entry(&entry.name);

        if is_xtream_live_path_url(&entry.stream_url) && !has_episode {
            channels.push(entry);
            continue;
        }

        // Prefer the URL path marker (Xtream-backed M3U); only fall back to the
        // name/group heuristic for generic playlists that lack /movie//series/.
        let kind = classify_stream_url(&entry.stream_url)
            .unwrap_or_else(|| classify_m3u_entry(&entry.name, group, has_episode));
        match kind {
            ContentKind::LiveChannel => channels.push(entry),
            ContentKind::Movie => {
                if is_channel_like_name(&entry.name) {
                    channels.push(entry);
                    continue;
                }
                if !is_valid_vod_movie(&entry.name) {
                    continue;
                }
                let added_at = m3u_added_at(&entry.stream_url, entry.sort_order);
                movies.push(Movie {
                    id: stable_movie_id(&entry.profile_id, &entry.stream_url),
                    profile_id: entry.profile_id.clone(),
                    name: entry.name,
                    poster: entry.logo,
                    backdrop: None,
                    plot: None,
                    genres: None,
                    rating: None,
                    stream_url: entry.stream_url,
                    category: entry.group_name,
                    added_at,
                    sort_order: entry.sort_order,
                });
            }
            ContentKind::Series => {
                if is_channel_like_name(&entry.name) {
                    channels.push(entry);
                    continue;
                }
                if let Some(parsed) = parsed {
                    let key = format!("parsed:{}", parsed.series_name);
                    let bucket = series_map.entry(key).or_insert_with(|| SeriesBucket {
                        series: Series {
                            id: stable_series_id(&entry.profile_id, &parsed.series_name),
                            profile_id: entry.profile_id.clone(),
                            name: parsed.series_name.clone(),
                            poster: entry.logo.clone(),
                            backdrop: None,
                            plot: None,
                            genres: None,
                            rating: None,
                            category: entry.group_name.clone(),
                            added_at: entry.sort_order as i64,
                            sort_order: entry.sort_order,
                        },
                        episodes: Vec::new(),
                        next_episode_num: 0,
                    });

                    if bucket.series.poster.is_none() {
                        bucket.series.poster = entry.logo.clone();
                    }

                    let title = if parsed.episode_title.is_empty() {
                        entry.name.clone()
                    } else {
                        parsed.episode_title
                    };

                    let added_at = m3u_added_at(&entry.stream_url, entry.sort_order);
                    bucket.episodes.push(Episode {
                        id: stable_episode_id(&entry.profile_id, &entry.stream_url),
                        series_id: bucket.series.id.clone(),
                        season: parsed.season,
                        episode: parsed.episode,
                        title,
                        plot: None,
                        stream_url: entry.stream_url,
                        duration: None,
                        added_at,
                    });
                } else {
                    let series_name = if group.is_empty() {
                        "Unknown Series".to_string()
                    } else {
                        group.to_string()
                    };
                    let key = format!("group:{series_name}");
                    let series_id = stable_series_id(&entry.profile_id, &key);

                    let bucket = series_map.entry(key).or_insert_with(|| SeriesBucket {
                        series: Series {
                            id: series_id.clone(),
                            profile_id: entry.profile_id.clone(),
                            name: series_name,
                            poster: entry.logo.clone(),
                            backdrop: None,
                            plot: None,
                            genres: None,
                            rating: None,
                            category: entry.group_name.clone(),
                            added_at: entry.sort_order as i64,
                            sort_order: entry.sort_order,
                        },
                        episodes: Vec::new(),
                        next_episode_num: 1,
                    });

                    if bucket.series.poster.is_none() {
                        bucket.series.poster = entry.logo.clone();
                    }

                    let ep_num = bucket.next_episode_num;
                    bucket.next_episode_num += 1;

                    let added_at = m3u_added_at(&entry.stream_url, entry.sort_order);
                    bucket.episodes.push(Episode {
                        id: stable_episode_id(&entry.profile_id, &entry.stream_url),
                        series_id: bucket.series.id.clone(),
                        season: 1,
                        episode: ep_num,
                        title: entry.name.clone(),
                        plot: None,
                        stream_url: entry.stream_url,
                        duration: None,
                        added_at,
                    });
                }
            }
        }
    }

    on_progress(total, total);

    let mut series_list = Vec::new();
    let mut episodes = Vec::new();
    for mut bucket in series_map.into_values() {
        if !bucket.episodes.is_empty() {
            let max_added = bucket
                .episodes
                .iter()
                .map(|episode| episode.added_at)
                .max()
                .unwrap_or(bucket.series.added_at);
            bucket.series.added_at = max_added;
        }
        episodes.extend(bucket.episodes);
        series_list.push(bucket.series);
    }

    CatalogSyncResult {
        channels,
        movies,
        series: series_list,
        episodes,
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PersistOptions {
    /// When false, search FTS is rebuilt in the background after commit.
    pub reindex_fts: bool,
}

impl Default for PersistOptions {
    fn default() -> Self {
        Self { reindex_fts: true }
    }
}

pub fn map_xtream_live_streams(
    profile_id: &str,
    client: &XtreamClient,
    streams: Vec<XtreamLiveStream>,
    category_names: &HashMap<String, String>,
) -> Vec<Channel> {
    streams
        .into_iter()
        .enumerate()
        .map(|(index, stream)| {
            let stream_url = client.build_live_url(stream.stream_id, "ts");
            Channel {
                id: stable_channel_id(profile_id, &stream_url),
                profile_id: profile_id.to_string(),
                name: stream.name,
                logo: stream.stream_icon,
                group_name: resolve_xtream_category(stream.category_id.as_deref(), category_names),
                stream_url,
                tvg_id: stream.epg_channel_id,
                sort_order: index as i32,
            }
        })
        .collect()
}

pub fn map_xtream_vod_streams(
    profile_id: &str,
    client: &XtreamClient,
    streams: Vec<XtreamVodStream>,
    category_names: &HashMap<String, String>,
) -> Vec<Movie> {
    streams
        .into_iter()
        .filter(|stream| {
            !is_channel_like_name(&stream.name) && is_valid_vod_movie(&stream.name)
        })
        .enumerate()
        .map(|(index, stream)| {
            let extension = stream
                .container_extension
                .as_deref()
                .unwrap_or("mp4");
            let poster = pick_xtream_poster(
                &stream.cover_big,
                &stream.cover,
                &stream.movie_image,
                &stream.stream_icon,
            );
            let backdrop = pick_xtream_backdrop(
                &stream.backdrop_path,
                &stream.cover_big,
                &stream.cover,
            );
            Movie {
                id: Uuid::new_v4().to_string(),
                profile_id: profile_id.to_string(),
                name: stream.name,
                poster: poster.clone(),
                backdrop,
                plot: None,
                genres: None,
                rating: stream.rating,
                stream_url: client.build_vod_url(stream.stream_id, extension),
                category: resolve_xtream_category(stream.category_id.as_deref(), category_names),
                added_at: parse_xtream_timestamp(
                    stream
                        .added
                        .as_deref()
                        .or(stream.last_modified.as_deref()),
                ),
                sort_order: index as i32,
            }
        })
        .collect()
}

pub fn map_xtream_series(
    profile_id: &str,
    series_item: &XtreamSeries,
    info: &XtreamSeriesInfo,
    client: &XtreamClient,
    category_names: &HashMap<String, String>,
) -> (Series, Vec<Episode>) {
    let detail = info.info.as_ref();
    let series_id = Uuid::new_v4().to_string();
    let mut episodes = Vec::new();
    for (season_key, season_episodes) in &info.episodes {
        let season_from_key = season_key.parse::<i32>().ok();
        for ep in season_episodes {
            episodes.push(map_xtream_episode(&series_id, season_from_key, ep, client));
        }
    }

    episodes.sort_by(|a, b| (a.season, a.episode).cmp(&(b.season, b.episode)));

    let series_timestamp = parse_xtream_timestamp(
        series_item
            .added
            .as_deref()
            .or(detail.and_then(|d| d.last_modified.as_deref()))
            .or(series_item.last_modified.as_deref()),
    );
    let episode_timestamp = episodes.iter().map(|episode| episode.added_at).max().unwrap_or(0);
    let added_at = series_timestamp.max(episode_timestamp);

    let series_name = detail
        .and_then(|d| d.name.clone())
        .unwrap_or_else(|| series_item.name.clone());
    if is_channel_like_name(&series_name) {
        return (
            Series {
                id: series_id,
                profile_id: profile_id.to_string(),
                name: series_name,
                poster: None,
                backdrop: None,
                plot: None,
                genres: None,
                rating: None,
                category: None,
                added_at: 0,
                sort_order: 0,
            },
            Vec::new(),
        );
    }

    let poster = detail
        .map(|d| {
            pick_xtream_poster(&d.cover_big, &d.cover, &None, &None)
        })
        .flatten()
        .or_else(|| {
            pick_xtream_poster(
                &series_item.cover_big,
                &series_item.cover,
                &None,
                &None,
            )
        });
    let backdrop = detail
        .map(|d| pick_xtream_backdrop(&d.backdrop_path, &d.cover_big, &d.cover))
        .flatten()
        .or_else(|| {
            pick_xtream_backdrop(
                &series_item.backdrop_path,
                &series_item.cover_big,
                &series_item.cover,
            )
        });

    let series_entry = Series {
        id: series_id.clone(),
        profile_id: profile_id.to_string(),
        name: series_name,
        poster,
        backdrop,
        plot: detail
            .and_then(|d| d.plot.clone())
            .or_else(|| series_item.plot.clone()),
        genres: detail
            .and_then(|d| d.genre.clone())
            .or_else(|| series_item.genre.clone()),
        rating: detail
            .and_then(|d| d.rating.clone())
            .or_else(|| series_item.rating.clone()),
        category: resolve_xtream_category(
            detail
                .and_then(|d| d.category_id.as_deref())
                .or(series_item.category_id.as_deref()),
            category_names,
        ),
        added_at,
        sort_order: series_item.series_id as i32,
    };

    (series_entry, episodes)
}

pub fn map_xtream_episodes_for_series(
    series_id: &str,
    info: &XtreamSeriesInfo,
    client: &XtreamClient,
) -> Vec<Episode> {
    let mut episodes = Vec::new();
    for (season_key, season_episodes) in &info.episodes {
        let season_from_key = season_key.parse::<i32>().ok();
        for ep in season_episodes {
            episodes.push(map_xtream_episode(series_id, season_from_key, ep, client));
        }
    }
    episodes.sort_by(|a, b| (a.season, a.episode).cmp(&(b.season, b.episode)));
    episodes
}

pub fn apply_xtream_info_to_series(series: &mut Series, info: &XtreamSeriesInfo) {
    let detail = info.info.as_ref();
    if let Some(d) = detail {
        if let Some(name) = d.name.as_ref().filter(|n| !n.is_empty()) {
            series.name = name.clone();
        }
        if series.poster.is_none() {
            series.poster = pick_xtream_poster(&d.cover_big, &d.cover, &None, &None);
        }
        if series.backdrop.is_none() {
            series.backdrop = pick_xtream_backdrop(&d.backdrop_path, &d.cover_big, &d.cover);
        }
        if series.plot.is_none() {
            series.plot = d.plot.clone();
        }
        if series.genres.is_none() {
            series.genres = d.genre.clone();
        }
        if series.rating.is_none() {
            series.rating = d.rating.clone();
        }
    }
}

pub fn build_xtream_category_map(
    categories: &[crate::parsers::xtream::XtreamCategory],
) -> HashMap<String, String> {
    categories
        .iter()
        .map(|cat| (cat.category_id.clone(), cat.category_name.clone()))
        .collect()
}

fn resolve_xtream_category(
    category_id: Option<&str>,
    category_names: &HashMap<String, String>,
) -> Option<String> {
    category_id.map(|id| {
        category_names
            .get(id)
            .cloned()
            .unwrap_or_else(|| id.to_string())
    })
}

fn map_xtream_episode(
    series_id: &str,
    season_from_key: Option<i32>,
    ep: &XtreamEpisode,
    client: &XtreamClient,
) -> Episode {
    let extension = ep
        .container_extension
        .as_deref()
        .unwrap_or("mp4");
    let season = ep.season.or(season_from_key).unwrap_or(1);
    let episode_num = ep.episode_num.unwrap_or(1);
    let title = ep
        .title
        .clone()
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| format!("Episode {episode_num}"));

    Episode {
        id: Uuid::new_v4().to_string(),
        series_id: series_id.to_string(),
        season,
        episode: episode_num,
        title,
        plot: None,
        stream_url: client.build_series_episode_url(ep.id, extension),
        duration: None,
        added_at: parse_xtream_timestamp(ep.added.as_deref()),
    }
}

pub fn parse_xtream_timestamp(value: Option<&str>) -> i64 {
    value.and_then(|s| s.parse::<i64>().ok()).unwrap_or(0)
}

/// Removes VOD rows that were misclassified into the live channels bucket.
pub fn cleanup_stale_channels(catalog: &mut CatalogSyncResult) {
    catalog.channels.retain(|channel| {
        if is_vod_stream_url(&channel.stream_url) {
            return false;
        }
        let group = channel.group_name.as_deref().unwrap_or("");
        let parsed = parse_series_title(&channel.name);
        let has_episode = parsed.is_some() || looks_like_series_entry(&channel.name);
        matches!(
            classify_m3u_entry(&channel.name, group, has_episode),
            ContentKind::LiveChannel
        )
    });
}

/// Removes channel-brand and other non-VOD rows from movies/series before persistence.
pub fn cleanup_stale_vod(catalog: &mut CatalogSyncResult) {
    catalog
        .movies
        .retain(|movie| !is_channel_like_name(&movie.name) && is_valid_vod_movie(&movie.name));

    catalog.series.retain(|series| {
        !is_junk_series_name(&series.name, series.category.as_deref())
    });

    let valid_series_ids: std::collections::HashSet<String> =
        catalog.series.iter().map(|s| s.id.clone()).collect();
    catalog
        .episodes
        .retain(|episode| valid_series_ids.contains(&episode.series_id));
}

pub fn persist_catalog(db: &SharedDb, profile_id: &str, catalog: &CatalogSyncResult) -> AppResult<()> {
    persist_catalog_with_options(db, profile_id, catalog, PersistOptions::default(), |_, _| {})
}

pub fn persist_catalog_with_options<F>(
    db: &SharedDb,
    profile_id: &str,
    catalog: &CatalogSyncResult,
    options: PersistOptions,
    mut on_progress: F,
) -> AppResult<()>
where
    F: FnMut(f64, &str),
{
    let mut catalog = catalog.clone();
    cleanup_stale_channels(&mut catalog);
    cleanup_stale_vod(&mut catalog);

    for channel in &mut catalog.channels {
        channel.id = stable_channel_id(profile_id, &channel.stream_url);
    }

    db.with_conn(|conn| {
        apply_bulk_write_pragmas(conn)?;
        let previous_episode_snapshots = series_tracking::snapshot_episodes(conn, profile_id)?;
        let previous_series_ids = series_tracking::snapshot_series_ids(conn, profile_id)?;
        let sync_now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let tx = conn.unchecked_transaction()?;

        favorites::remap_legacy_channel_favorites(&tx, profile_id)?;
        crate::db::history::remap_legacy_channel_history(&tx, profile_id)?;
        channels::delete_by_profile(&tx, profile_id)?;
        movies::delete_by_profile(&tx, profile_id)?;
        series::delete_by_profile(&tx, profile_id)?;

        on_progress(72.0, "Salvando canais...");
        insert_batches(&tx, &catalog.channels, channels::insert_batch)?;
        on_progress(78.0, "Salvando filmes...");
        insert_batches(&tx, &catalog.movies, movies::insert_batch)?;
        on_progress(84.0, "Salvando séries...");
        insert_batches(&tx, &catalog.series, series::insert_series_batch)?;
        on_progress(90.0, "Salvando episódios...");
        insert_batches(&tx, &catalog.episodes, series::insert_episodes_batch)?;

        // Finalize the content classification computed at insert time: demote
        // channel rows that leaked into `movies` (Xtream URL rule) and
        // poster-less series without enough episodes.
        movies::refine_xtream_vod_classification(&tx, profile_id)?;
        series::refine_series_listability(&tx, profile_id)?;

        let series_ids: Vec<String> = catalog.series.iter().map(|s| s.id.clone()).collect();
        series_tracking::sync_tracking_for_catalog(
            &tx,
            profile_id,
            &series_ids,
            &catalog.episodes,
            &previous_episode_snapshots,
            &previous_series_ids,
            sync_now,
        )?;
        series_tracking::ensure_profile_tracking(&tx, profile_id)?;

        if options.reindex_fts {
            on_progress(94.0, "Indexando busca...");
            catalog_fts::reindex_profile_from_db(&tx, profile_id)?;
        }

        tx.commit()?;
        restore_write_pragmas(conn)?;
        Ok(())
    })
}

pub fn reindex_catalog_search(db: &SharedDb, profile_id: &str) -> AppResult<()> {
    db.with_conn(|conn| catalog_fts::reindex_profile_from_db(conn, profile_id))
}

fn insert_batches<T, F>(conn: &rusqlite::Connection, items: &[T], insert_fn: F) -> AppResult<()>
where
    F: Fn(&rusqlite::Connection, &[T]) -> AppResult<()>,
{
    for batch in items.chunks(BATCH_SIZE) {
        insert_fn(conn, batch)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::migrations;
    use crate::db::models::Profile;
    use crate::db::profiles;
    use crate::parsers::m3u::M3uParser;
    use rusqlite::Connection;

    fn setup_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        migrations::run(&conn).unwrap();
        conn
    }

    #[test]
    fn parses_xtream_credentials_from_live_stream_url() {
        let creds = parse_xtream_credentials_from_live_url(
            "http://example.com:8080/live/myuser/mypass/3001.ts",
        )
        .unwrap();
        assert_eq!(creds.base_url, "http://example.com:8080");
        assert_eq!(creds.username, "myuser");
        assert_eq!(creds.password, "mypass");
    }

    #[test]
    fn parses_xtream_credentials_from_m3u_url() {
        let creds = parse_xtream_credentials_from_m3u_url(
            "http://example.com:8080/get.php?username=user1&password=secret&type=m3u_plus",
        )
        .unwrap();
        assert_eq!(creds.base_url, "http://example.com:8080");
        assert_eq!(creds.username, "user1");
        assert_eq!(creds.password, "secret");
    }

    #[test]
    fn resolve_xtream_epg_access_uses_m3u_profile_url_fallback() {
        let profile = Profile {
            id: "p1".to_string(),
            name: "Minha Lista".to_string(),
            profile_type: "m3u".to_string(),
            url: Some(
                "http://example.com/get.php?username=u&password=p&type=m3u_plus".to_string(),
            ),
            file_path: None,
            username: None,
            password: None,
            last_sync: None,
        };
        let channel = Channel {
            id: "c1".to_string(),
            profile_id: "p1".to_string(),
            name: "Canal".to_string(),
            logo: None,
            group_name: None,
            stream_url: "http://cdn.example.com/hls/42.m3u8".to_string(),
            tvg_id: None,
            sort_order: 0,
        };

        let access = resolve_xtream_epg_access(&profile, &channel).unwrap();
        assert_eq!(access.0.username, "u");
        assert_eq!(access.1, 42);
    }

    fn fixture_path(name: &str) -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("tests")
            .join("fixtures")
            .join(name)
    }

    #[test]
    fn classifies_m3u_fixture_into_channels_movies_and_series() {
        let fixture = fixture_path("catalog.m3u");
        let mut parser = M3uParser::new("profile-1".to_string());
        let entries = parser.parse_file(fixture.to_str().unwrap());
        let rt = tokio::runtime::Runtime::new().unwrap();
        let entries = rt.block_on(entries).unwrap();

        let catalog = classify_m3u_entries(entries);

        assert_eq!(catalog.channels.len(), 1);
        assert_eq!(catalog.movies.len(), 2);
        assert_eq!(catalog.series.len(), 2);
        assert_eq!(catalog.episodes.len(), 3);

        assert_eq!(catalog.channels[0].name, "News Channel");
        assert_eq!(catalog.movies[0].name, "Movie One");
        assert_eq!(catalog.movies[1].name, "Movie Two");

        let breaking_bad = catalog
            .series
            .iter()
            .find(|s| s.name == "Breaking Bad")
            .expect("Breaking Bad series");
        assert_eq!(breaking_bad.category.as_deref(), Some("Séries"));

        let bb_episodes: Vec<_> = catalog
            .episodes
            .iter()
            .filter(|e| e.series_id == breaking_bad.id)
            .collect();
        assert_eq!(bb_episodes.len(), 2);
        assert_eq!(bb_episodes[0].season, 1);
        assert_eq!(bb_episodes[0].episode, 1);
        assert_eq!(bb_episodes[0].title, "Pilot");

        let group_series = catalog
            .series
            .iter()
            .find(|s| s.name == "Séries")
            .expect("group bucket series");
        let group_eps: Vec<_> = catalog
            .episodes
            .iter()
            .filter(|e| e.series_id == group_series.id)
            .collect();
        assert_eq!(group_eps.len(), 1);
        assert_eq!(group_eps[0].title, "Lost Episode Without Pattern");
    }

    #[test]
    fn persist_catalog_writes_counts_to_db() {
        let conn = setup_db();
        let profile = Profile {
            id: "p1".to_string(),
            name: "Test".to_string(),
            profile_type: "m3u".to_string(),
            url: None,
            file_path: None,
            username: None,
            password: None,
            last_sync: None,
        };
        profiles::insert_profile(&conn, &profile).unwrap();

        let catalog = CatalogSyncResult {
            channels: vec![Channel {
                id: "ch1".to_string(),
                profile_id: "p1".to_string(),
                name: "Live".to_string(),
                logo: None,
                group_name: Some("News".to_string()),
                stream_url: "http://example.com/live".to_string(),
                tvg_id: None,
                sort_order: 0,
            }],
            movies: vec![Movie {
                id: "m1".to_string(),
                profile_id: "p1".to_string(),
                name: "Sample Film (2024)".to_string(),
                poster: Some("http://example.com/film.png".to_string()),
                backdrop: None,
                plot: None,
                genres: None,
                rating: None,
                stream_url: "http://example.com/movie".to_string(),
                category: Some("Filmes".to_string()),
                added_at: 100,
                sort_order: 0,
            }],
            series: vec![Series {
                id: "s1".to_string(),
                profile_id: "p1".to_string(),
                name: "Show".to_string(),
                poster: Some("http://example.com/show.png".to_string()),
                backdrop: None,
                plot: None,
                genres: None,
                rating: None,
                category: Some("Séries".to_string()),
                added_at: 100,
                sort_order: 0,
            }],
            episodes: vec![
                Episode {
                    id: "e1".to_string(),
                    series_id: "s1".to_string(),
                    season: 1,
                    episode: 1,
                    title: "Pilot".to_string(),
                    plot: None,
                    stream_url: "http://example.com/ep1".to_string(),
                    duration: None,
                    added_at: 100,
                },
                Episode {
                    id: "e2".to_string(),
                    series_id: "s1".to_string(),
                    season: 1,
                    episode: 2,
                    title: "Second".to_string(),
                    plot: None,
                    stream_url: "http://example.com/ep2".to_string(),
                    duration: None,
                    added_at: 101,
                },
            ],
        };

        let db = std::sync::Arc::new(crate::db::DbState::single(conn));
        persist_catalog(&db, "p1", &catalog).unwrap();

        db.with_conn(|conn| {
            let channels_page =
                channels::list_channels(conn, "p1", None, None, 0, 100).unwrap();
            let movies_page = movies::list_movies(conn, "p1", None, None, None, 0, 100).unwrap();
            let series_page = series::list_series(conn, "p1", None, None, None, 0, 100).unwrap();
            let episodes = series::list_episodes_for_series(conn, "s1").unwrap();

            assert_eq!(channels_page.total, 1);
            assert_eq!(movies_page.total, 1);
            assert_eq!(series_page.total, 1);
            assert_eq!(episodes.len(), 2);
            Ok(())
        })
        .unwrap();
    }

    fn channel_entry(name: &str, group: &str) -> Channel {
        Channel {
            id: Uuid::new_v4().to_string(),
            profile_id: "p1".to_string(),
            name: name.to_string(),
            logo: Some("http://example.com/logo.png".to_string()),
            group_name: Some(group.to_string()),
            stream_url: format!("http://example.com/{name}"),
            tvg_id: None,
            sort_order: 0,
        }
    }

    #[test]
    fn cleanup_stale_channels_purges_movie_rows_from_live_bucket() {
        let mut catalog = CatalogSyncResult {
            channels: vec![
                channel_entry("MEGAPIX FHD", "Comedia"),
                channel_entry("Inception (2010)", "Comedia"),
                channel_entry("News Channel", "Canais"),
            ],
            movies: vec![],
            series: vec![],
            episodes: vec![],
        };

        cleanup_stale_channels(&mut catalog);

        assert_eq!(catalog.channels.len(), 2);
        assert!(catalog.channels.iter().any(|c| c.name == "MEGAPIX FHD"));
        assert!(catalog.channels.iter().any(|c| c.name == "News Channel"));
        assert!(!catalog
            .channels
            .iter()
            .any(|c| c.name == "Inception (2010)"));
    }

    #[test]
    fn cleanup_stale_vod_purges_channel_rows_from_movies_and_series() {
        let mut catalog = CatalogSyncResult {
            channels: vec![],
            movies: vec![
                Movie {
                    id: "m1".to_string(),
                    profile_id: "p1".to_string(),
                    name: "MEGAPIX FHD".to_string(),
                    poster: Some("http://example.com/megapix.png".to_string()),
                    backdrop: None,
                    plot: None,
                    genres: None,
                    rating: None,
                    stream_url: "http://example.com/megapix".to_string(),
                    category: Some("Filmes".to_string()),
                    added_at: 100,
                    sort_order: 0,
                },
                Movie {
                    id: "m2".to_string(),
                    profile_id: "p1".to_string(),
                    name: "Inception (2010)".to_string(),
                    poster: Some("http://example.com/inception.png".to_string()),
                    backdrop: None,
                    plot: None,
                    genres: None,
                    rating: None,
                    stream_url: "http://example.com/inception".to_string(),
                    category: Some("Filmes".to_string()),
                    added_at: 200,
                    sort_order: 1,
                },
            ],
            series: vec![Series {
                id: "s1".to_string(),
                profile_id: "p1".to_string(),
                name: "UNIVERSAL TV".to_string(),
                poster: Some("http://example.com/universal.png".to_string()),
                backdrop: None,
                plot: None,
                genres: None,
                rating: None,
                category: Some("Séries".to_string()),
                added_at: 100,
                sort_order: 0,
            }],
            episodes: vec![Episode {
                id: "e1".to_string(),
                series_id: "s1".to_string(),
                season: 1,
                episode: 1,
                title: "Pilot".to_string(),
                plot: None,
                stream_url: "http://example.com/ep".to_string(),
                duration: None,
                added_at: 100,
            }],
        };

        cleanup_stale_vod(&mut catalog);

        assert_eq!(catalog.movies.len(), 1);
        assert_eq!(catalog.movies[0].name, "Inception (2010)");
        assert!(catalog.series.is_empty());
        assert!(catalog.episodes.is_empty());
    }

    #[test]
    fn cleanup_stale_vod_purges_decorated_channel_brands() {
        let channel_movie = |name: &str| Movie {
            id: Uuid::new_v4().to_string(),
            profile_id: "p1".to_string(),
            name: name.to_string(),
            poster: Some("http://example.com/logo.png".to_string()),
            backdrop: None,
            plot: None,
            genres: None,
            rating: None,
            stream_url: format!("http://example.com/{name}"),
            category: Some("Filmes".to_string()),
            added_at: 100,
            sort_order: 0,
        };

        let mut catalog = CatalogSyncResult {
            channels: vec![],
            movies: vec![
                channel_movie("MEGAPIX FHD*"),
                channel_movie("MEGAPIX HD*"),
                channel_movie("USA FHD*"),
                channel_movie("USA HDᴮᴿ"),
                channel_movie("UNIVERSAL PREMIER"),
                Movie {
                    id: "m-real".to_string(),
                    profile_id: "p1".to_string(),
                    name: "Inception (2010)".to_string(),
                    poster: Some("http://example.com/inception.png".to_string()),
                    backdrop: None,
                    plot: None,
                    genres: None,
                    rating: None,
                    stream_url: "http://example.com/inception".to_string(),
                    category: Some("Filmes".to_string()),
                    added_at: 200,
                    sort_order: 1,
                },
            ],
            series: vec![],
            episodes: vec![],
        };

        cleanup_stale_vod(&mut catalog);

        assert_eq!(catalog.movies.len(), 1);
        assert_eq!(catalog.movies[0].name, "Inception (2010)");
    }

    #[test]
    fn rejects_channel_brands_from_movie_classification() {
        let entries = vec![
            channel_entry("MEGAPIX FHD", "Comedia"),
            channel_entry("UNIVERSAL TV", "Drama"),
            channel_entry("STUDIO UNIVERSAL", "Filmes"),
            channel_entry("Inception (2010)", "Comedia"),
            channel_entry("20 ANOS + JOVEM (2013)", "Comedia"),
            channel_entry("Breaking Bad S01E05", "Séries"),
        ];

        let catalog = classify_m3u_entries(entries);

        assert_eq!(catalog.movies.len(), 2);
        assert!(catalog.movies.iter().any(|m| m.name == "Inception (2010)"));
        assert!(catalog
            .movies
            .iter()
            .any(|m| m.name == "20 ANOS + JOVEM (2013)"));
        assert_eq!(catalog.series.len(), 1);
        assert_eq!(catalog.series[0].name, "Breaking Bad");
        assert_eq!(catalog.channels.len(), 3);
        assert!(catalog
            .channels
            .iter()
            .any(|c| c.name == "MEGAPIX FHD"));
        assert!(catalog
            .channels
            .iter()
            .any(|c| c.name == "UNIVERSAL TV"));
        assert!(catalog
            .channels
            .iter()
            .any(|c| c.name == "STUDIO UNIVERSAL"));
    }

    #[test]
    fn url_path_overrides_group_for_xtream_m3u() {
        let entries = vec![
            // Movie in a genre group the name/group heuristic does not recognise.
            Channel {
                id: Uuid::new_v4().to_string(),
                profile_id: "p1".to_string(),
                name: "O ESTRANGEIRO (2025)".to_string(),
                logo: Some("http://cdn/x_big.jpg".to_string()),
                group_name: Some("CRIME".to_string()),
                stream_url: "http://host:80/movie/user/pass/10290521.mp4".to_string(),
                tvg_id: None,
                sort_order: 5,
            },
            // Series episode under /series/.
            Channel {
                id: Uuid::new_v4().to_string(),
                profile_id: "p1".to_string(),
                name: "Vikings S01 E01".to_string(),
                logo: None,
                group_name: Some("SERIES V".to_string()),
                stream_url: "http://host:80/series/user/pass/10143246.mp4".to_string(),
                tvg_id: None,
                sort_order: 9,
            },
            // Real live channel (no /movie/ or /series/ segment).
            Channel {
                id: Uuid::new_v4().to_string(),
                profile_id: "p1".to_string(),
                name: "GLOBO SP".to_string(),
                logo: None,
                group_name: Some("GLOBOS | SUDESTE".to_string()),
                stream_url: "http://host:80/user/pass/10283166".to_string(),
                tvg_id: None,
                sort_order: 2,
            },
        ];

        let catalog = classify_m3u_entries(entries);

        assert_eq!(catalog.movies.len(), 1, "movie classified by /movie/ url");
        assert_eq!(catalog.movies[0].name, "O ESTRANGEIRO (2025)");
        // added_at comes from the stream id, not the playlist line index.
        assert_eq!(catalog.movies[0].added_at, 10_290_521);

        assert_eq!(catalog.series.len(), 1, "series classified by /series/ url");
        assert_eq!(catalog.series[0].name, "Vikings");
        assert_eq!(catalog.series[0].added_at, 10_143_246);

        assert_eq!(catalog.channels.len(), 1, "only the live entry stays a channel");
        assert_eq!(catalog.channels[0].name, "GLOBO SP");
    }

    #[test]
    fn xtream_live_url_stays_in_channels_even_with_movie_like_title() {
        let entries = vec![Channel {
            id: Uuid::new_v4().to_string(),
            profile_id: "p1".to_string(),
            name: "Canal Especial (2024)".to_string(),
            logo: None,
            group_name: Some("DOCUMENTÁRIOS".to_string()),
            stream_url: "http://host:80/live/user/pass/5001.ts".to_string(),
            tvg_id: None,
            sort_order: 1,
        }];

        let catalog = classify_m3u_entries(entries);

        assert_eq!(catalog.channels.len(), 1);
        assert_eq!(catalog.channels[0].group_name.as_deref(), Some("DOCUMENTÁRIOS"));
        assert!(catalog.movies.is_empty());
    }

    #[test]
    fn maps_xtream_series_added_at_from_episodes() {
        let series_json = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("src/parsers/xtream/fixtures/series.json"),
        )
        .unwrap();
        let info_json = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("src/parsers/xtream/fixtures/series_info.json"),
        )
        .unwrap();
        let series_list: Vec<XtreamSeries> = serde_json::from_str(&series_json).unwrap();
        let info: XtreamSeriesInfo = serde_json::from_str(&info_json).unwrap();
        let client = XtreamClient::new(
            "http://example.com".to_string(),
            "user".to_string(),
            "pass".to_string(),
        );

        let category_map = std::collections::HashMap::from([(
            "5".to_string(),
            "Séries".to_string(),
        )]);
        let (series_entry, episodes) =
            map_xtream_series("p1", &series_list[0], &info, &client, &category_map);

        assert_eq!(episodes.len(), 3);
        assert_eq!(series_entry.added_at, 1_700_200_000);
        assert_eq!(series_entry.name, "Sample Series");
        assert_eq!(
            series_entry.poster.as_deref(),
            Some("http://example.com/posters/series1-detail-big.jpg")
        );
        assert_eq!(
            series_entry.backdrop.as_deref(),
            Some("http://example.com/backdrops/series1-detail.jpg")
        );
    }

    #[test]
    fn maps_xtream_vod_from_fixture() {
        let json = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("src/parsers/xtream/fixtures/vod_streams.json"),
        )
        .unwrap();
        let streams: Vec<XtreamVodStream> = serde_json::from_str(&json).unwrap();
        let client = XtreamClient::new(
            "http://example.com".to_string(),
            "user".to_string(),
            "pass".to_string(),
        );
        let category_map = std::collections::HashMap::from([(
            "5".to_string(),
            "Filmes".to_string(),
        )]);
        let movies = map_xtream_vod_streams("p1", &client, streams, &category_map);
        assert_eq!(movies.len(), 2);
        assert_eq!(movies[0].category.as_deref(), Some("Filmes"));
        assert!(movies[0].stream_url.contains("/movie/user/pass/1001."));
        assert_eq!(
            movies[0].poster.as_deref(),
            Some("http://example.com/posters/movie1-big.jpg")
        );
        assert_eq!(
            movies[0].backdrop.as_deref(),
            Some("http://example.com/backdrops/movie1.jpg")
        );
    }

    #[test]
    fn maps_xtream_live_with_category_names() {
        let json = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("src/parsers/xtream/fixtures/live_streams.json"),
        )
        .unwrap();
        let streams: Vec<XtreamLiveStream> = serde_json::from_str(&json).unwrap();
        let client = XtreamClient::new(
            "http://example.com".to_string(),
            "user".to_string(),
            "pass".to_string(),
        );
        let category_map = build_xtream_category_map(&[
            crate::parsers::xtream::XtreamCategory {
                category_id: "1".to_string(),
                category_name: "Canais".to_string(),
            },
            crate::parsers::xtream::XtreamCategory {
                category_id: "10".to_string(),
                category_name: "Esportes".to_string(),
            },
        ]);
        let channels = map_xtream_live_streams("p1", &client, streams, &category_map);
        assert_eq!(channels.len(), 2);
        assert_eq!(channels[0].group_name.as_deref(), Some("Canais"));
        assert_eq!(channels[1].group_name.as_deref(), Some("Esportes"));
    }
}
