use crate::db::models::{ChannelEpg, EpgProgram};
use crate::db::{channels, epg, profiles, SharedDb};
use crate::error::{AppError, AppResult};
use crate::parsers::xtream::{decode_xtream_epg_text, XtreamClient, XtreamEpgListing};
use crate::services::catalog_sync::{
    parse_trailing_stream_id, resolve_xtream_epg_access,
};

const DEFAULT_EPG_LIMIT: i64 = 4;
const MAX_EPG_LIMIT: i64 = 20;

fn now_unix_ts() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

pub fn unavailable_epg(reason: &str) -> ChannelEpg {
    ChannelEpg {
        available: false,
        unavailable_reason: Some(reason.to_string()),
        programs: Vec::new(),
    }
}

fn epg_unavailable_reason(profile_type: &str, has_stream_id: bool) -> &'static str {
    match (profile_type, has_stream_id) {
        ("m3u", false) => "EPG indisponível (ID do canal não encontrado)",
        ("m3u", true) => "EPG indisponível (lista M3U sem credenciais Xtream)",
        (_, false) => "EPG indisponível (ID do canal não encontrado)",
        _ => "EPG indisponível",
    }
}

pub fn map_xtream_listing(listing: &XtreamEpgListing, now: i64) -> Option<EpgProgram> {
    let start_ts = listing.start_timestamp?;
    let end_ts = listing.stop_timestamp?;
    let title = listing
        .title
        .as_deref()
        .map(decode_xtream_epg_text)
        .filter(|t| !t.trim().is_empty())?;
    let description = listing
        .description
        .as_deref()
        .map(decode_xtream_epg_text)
        .filter(|d| !d.trim().is_empty());
    Some(EpgProgram {
        title,
        description,
        start_ts,
        end_ts,
        epg_id: listing.epg_id.clone(),
        is_now: now >= start_ts && now < end_ts,
    })
}

pub fn map_xtream_epg_listings(listings: &[XtreamEpgListing], now: i64) -> Vec<EpgProgram> {
    let mut programs: Vec<EpgProgram> = listings
        .iter()
        .filter_map(|listing| map_xtream_listing(listing, now))
        .collect();
    programs.sort_by_key(|p| p.start_ts);
    programs
}

pub async fn get_channel_epg(
    db: &SharedDb,
    profile_id: &str,
    channel_id: &str,
    limit: Option<i64>,
) -> AppResult<ChannelEpg> {
    let limit = limit.unwrap_or(DEFAULT_EPG_LIMIT).clamp(1, MAX_EPG_LIMIT);
    let now = now_unix_ts();

    let (channel, profile) = db.with_conn(|conn| {
        let channel = channels::get_channel(conn, channel_id)?
            .ok_or_else(|| AppError::msg("Canal não encontrado."))?;
        if channel.profile_id != profile_id {
            return Err(AppError::msg("Canal não pertence ao perfil."));
        }
        let profile = profiles::get_profile(conn, profile_id)?
            .ok_or_else(|| AppError::msg("Perfil não encontrado."))?;
        Ok((channel, profile))
    })?;

    let has_stream_id = parse_trailing_stream_id(&channel.stream_url).is_some();
    let Some((creds, stream_id)) = resolve_xtream_epg_access(&profile, &channel) else {
        return Ok(unavailable_epg(epg_unavailable_reason(
            &profile.profile_type,
            has_stream_id,
        )));
    };

    let cache_hit = db.with_conn(|conn| epg::cache_fresh(conn, profile_id, channel_id, now))?;
    if cache_hit {
        let programs = db.with_conn(|conn| {
            epg::list_cached_programs(conn, profile_id, channel_id, limit, now)
        })?;
        return Ok(ChannelEpg {
            available: true,
            unavailable_reason: None,
            programs,
        });
    }

    let client = XtreamClient::new(creds.base_url, creds.username, creds.password);

    let response = client.get_short_epg(stream_id, limit).await?;
    let programs = map_xtream_epg_listings(&response.epg_listings, now);

    db.with_conn(|conn| {
        epg::replace_channel_programs(conn, profile_id, channel_id, &programs, now)
    })?;

    Ok(ChannelEpg {
        available: true,
        unavailable_reason: None,
        programs: programs.into_iter().take(limit as usize).collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::migrations;
    use crate::db::models::{Channel, Profile};
    use crate::db::profiles;
    use crate::parsers::xtream::XtreamShortEpgResponse;
    use rusqlite::Connection;

    fn fixture_path(name: &str) -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("parsers")
            .join("xtream")
            .join("fixtures")
            .join(name)
    }

    #[test]
    fn deserializes_short_epg_fixture() {
        let json = std::fs::read_to_string(fixture_path("short_epg.json")).unwrap();
        let response: XtreamShortEpgResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(response.epg_listings.len(), 2);
    }

    #[test]
    fn maps_base64_titles_from_fixture() {
        let json = std::fs::read_to_string(fixture_path("short_epg.json")).unwrap();
        let response: XtreamShortEpgResponse = serde_json::from_str(&json).unwrap();
        let now = 1_749_328_900_i64;
        let programs = map_xtream_epg_listings(&response.epg_listings, now);
        assert_eq!(programs.len(), 2);
        assert_eq!(programs[0].title, "Jornal Nacional");
        assert_eq!(programs[1].title, "Jornal da Noite");
        assert!(programs[0].is_now);
    }

    #[test]
    fn m3u_profile_without_xtream_credentials_returns_unavailable() {
        let conn = Connection::open_in_memory().unwrap();
        migrations::run(&conn).unwrap();
        let profile = Profile {
            id: "p1".to_string(),
            name: "M3U".to_string(),
            profile_type: "m3u".to_string(),
            url: None,
            file_path: None,
            username: None,
            password: None,
            last_sync: None,
        };
        profiles::insert_profile(&conn, &profile).unwrap();
        channels::insert_batch(
            &conn,
            &[Channel {
                id: "c1".to_string(),
                profile_id: "p1".to_string(),
                name: "Globo".to_string(),
                logo: None,
                group_name: Some("TV".to_string()),
                stream_url: "http://h/live/1.ts".to_string(),
                tvg_id: None,
                sort_order: 0,
            }],
        )
        .unwrap();
        let db = std::sync::Arc::new(crate::db::DbState::single(conn));
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt
            .block_on(get_channel_epg(&db, "p1", "c1", None))
            .unwrap();
        assert!(!result.available);
        assert_eq!(
            result.unavailable_reason.as_deref(),
            Some("EPG indisponível (lista M3U sem credenciais Xtream)")
        );
    }

    #[test]
    fn m3u_profile_resolves_xtream_credentials_from_live_url() {
        use crate::services::catalog_sync::resolve_xtream_epg_access;

        let profile = Profile {
            id: "p1".to_string(),
            name: "Minha Lista".to_string(),
            profile_type: "m3u".to_string(),
            url: None,
            file_path: None,
            username: None,
            password: None,
            last_sync: None,
        };
        let channel = Channel {
            id: "c1".to_string(),
            profile_id: "p1".to_string(),
            name: "GB SP FHD".to_string(),
            logo: None,
            group_name: Some("GLOBOS | CAPITAIS".to_string()),
            stream_url: "http://example.com:8080/live/myuser/mypass/3001.ts".to_string(),
            tvg_id: Some("globo.sp".to_string()),
            sort_order: 0,
        };

        let access = resolve_xtream_epg_access(&profile, &channel).unwrap();
        assert_eq!(access.0.base_url, "http://example.com:8080");
        assert_eq!(access.0.username, "myuser");
        assert_eq!(access.0.password, "mypass");
        assert_eq!(access.1, 3001);
    }

    #[test]
    fn builds_short_epg_url_with_stream_id() {
        let client = XtreamClient::new(
            "http://example.com".to_string(),
            "user".to_string(),
            "pass".to_string(),
        );
        let url = client.short_epg_url(3001, 4);
        assert!(url.contains("action=get_short_epg"));
        assert!(url.contains("stream_id=3001"));
        assert!(url.contains("limit=4"));
    }
}
