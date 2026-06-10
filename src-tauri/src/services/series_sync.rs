use crate::db::{profiles, series, SharedDb};
use crate::error::{AppError, AppResult};
use crate::parsers::xtream::XtreamClient;
use crate::services::catalog_sync::{
    apply_xtream_info_to_series, map_xtream_episodes_for_series, parse_xtream_timestamp,
};

pub async fn sync_episodes_on_demand(db: &SharedDb, series_id: &str) -> AppResult<bool> {
    let (mut series_entry, profile) = db.with_conn(|conn| {
        let series_entry = series::get_series(conn, series_id)?
            .ok_or_else(|| AppError::msg("Série não encontrada."))?;
        let profile = profiles::get_profile(conn, &series_entry.profile_id)?
            .ok_or_else(|| AppError::msg("Perfil não encontrado."))?;
        Ok((series_entry, profile))
    })?;

    if profile.profile_type != "xtream" {
        return Ok(false);
    }

    let xtream_series_id = i64::from(series_entry.sort_order);
    if xtream_series_id <= 0 {
        return Ok(false);
    }

    let url = profile
        .url
        .as_deref()
        .filter(|u| !u.trim().is_empty())
        .ok_or_else(|| AppError::msg("Perfil Xtream sem URL configurada."))?;
    let username = profile
        .username
        .as_deref()
        .filter(|u| !u.trim().is_empty())
        .ok_or_else(|| AppError::msg("Perfil Xtream sem usuário configurado."))?;
    let password = profile
        .password
        .as_deref()
        .filter(|p| !p.trim().is_empty())
        .ok_or_else(|| AppError::msg("Perfil Xtream sem senha configurada."))?;

    let client = XtreamClient::new(
        url.trim().to_string(),
        username.trim().to_string(),
        password.trim().to_string(),
    );

    let info = client.get_series_info(xtream_series_id).await?;
    let episodes = map_xtream_episodes_for_series(&series_entry.id, &info, &client);

    apply_xtream_info_to_series(&mut series_entry, &info);

    let episode_timestamp = episodes.iter().map(|ep| ep.added_at).max().unwrap_or(0);
    let detail_timestamp = parse_xtream_timestamp(
        info.info
            .as_ref()
            .and_then(|d| d.last_modified.as_deref()),
    );
    series_entry.added_at = series_entry
        .added_at
        .max(episode_timestamp)
        .max(detail_timestamp);

    db.with_conn(|conn| {
        series::replace_episodes_for_series(conn, &series_entry.id, &episodes)?;
        series::update_series_metadata(conn, &series_entry)?;
        // Metadata/episodes changed, so the stored listability flag may flip
        // (e.g. a poster-less series that just gained episodes).
        series::reclassify_series(conn, &series_entry.id)?;
        Ok(())
    })?;

    Ok(!episodes.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::migrations;
    use crate::db::models::{Profile, Series};
    use crate::db::profiles;
    use rusqlite::Connection;

    fn setup_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        migrations::run(&conn).unwrap();
        conn
    }

    #[test]
    fn skips_non_xtream_profiles() {
        let conn = setup_db();
        profiles::insert_profile(
            &conn,
            &Profile {
                id: "p1".to_string(),
                name: "M3U".to_string(),
                profile_type: "m3u".to_string(),
                url: None,
                file_path: None,
                username: None,
                password: None,
                last_sync: None,
            },
        )
        .unwrap();
        series::insert_series_batch(
            &conn,
            &[Series {
                id: "s1".to_string(),
                profile_id: "p1".to_string(),
                name: "Show".to_string(),
                poster: None,
                backdrop: None,
                plot: None,
                genres: None,
                rating: None,
                category: None,
                added_at: 1,
                sort_order: 100,
            }],
        )
        .unwrap();

        let db = std::sync::Arc::new(crate::db::DbState::single(conn));

        let rt = tokio::runtime::Runtime::new().unwrap();
        let synced = rt.block_on(sync_episodes_on_demand(&db, "s1")).unwrap();
        assert!(!synced);
    }
}
