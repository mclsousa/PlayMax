use crate::db::models::{CategoryCount, Channel, ChannelsPage};
use crate::error::{AppError, AppResult};
use rusqlite::{params, Connection};

/// Xtream-backed M3U encodes VOD as `.../movie/...` and `.../series/...`; live
/// channels never do. Computed once at insert time and stored in `is_live` so
/// queries stay on the (profile_id, is_live, ...) indexes.
fn is_live_stream_url(stream_url: &str) -> bool {
    let url = stream_url.to_lowercase();
    !url.contains("/movie/") && !url.contains("/series/")
}

pub fn delete_by_profile(conn: &Connection, profile_id: &str) -> AppResult<()> {
    conn.execute(
        "DELETE FROM channels WHERE profile_id = ?1",
        params![profile_id],
    )?;
    Ok(())
}

pub fn insert_batch(conn: &Connection, channels: &[Channel]) -> AppResult<()> {
    if channels.is_empty() {
        return Ok(());
    }
    let mut stmt = conn.prepare(
        "INSERT INTO channels (id, profile_id, name, logo, group_name, stream_url, tvg_id, sort_order, is_live)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
    )?;
    for ch in channels {
        // Group names are normalized (trimmed, empty -> NULL) so queries can
        // use plain equality against the indexed column.
        let group = ch
            .group_name
            .as_deref()
            .map(str::trim)
            .filter(|g| !g.is_empty());
        stmt.execute(params![
            ch.id,
            ch.profile_id,
            ch.name,
            ch.logo,
            group,
            ch.stream_url,
            ch.tvg_id,
            ch.sort_order,
            is_live_stream_url(&ch.stream_url) as i64
        ])?;
    }
    Ok(())
}

const GROUP_EXPR: &str = "COALESCE(group_name, 'Sem categoria')";

pub fn list_groups(conn: &Connection, profile_id: &str) -> AppResult<Vec<CategoryCount>> {
    let sql = format!(
        "SELECT {GROUP_EXPR} AS g, COUNT(*) AS c
         FROM channels
         WHERE profile_id = ?1
         GROUP BY g
         ORDER BY g ASC"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params![profile_id], |row| {
        Ok(CategoryCount {
            name: row.get(0)?,
            count: row.get(1)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(AppError::Database)
}

pub fn list_live_groups(conn: &Connection, profile_id: &str) -> AppResult<Vec<CategoryCount>> {
    // Preserve the provider's own ordering (first appearance in the playlist),
    // which keeps related blocks together — e.g. all GLOBOS regions up front —
    // instead of an alphabetical jumble.
    let sql = format!(
        "SELECT {GROUP_EXPR} AS g, COUNT(*) AS c
         FROM channels
         WHERE profile_id = ?1 AND is_live = 1
         GROUP BY g
         ORDER BY MIN(sort_order) ASC"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params![profile_id], |row| {
        Ok(CategoryCount {
            name: row.get(0)?,
            count: row.get(1)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(AppError::Database)
}

pub fn list_channels(
    conn: &Connection,
    profile_id: &str,
    group: Option<&str>,
    search: Option<&str>,
    offset: i64,
    limit: i64,
) -> AppResult<ChannelsPage> {
    let mut conditions = vec!["profile_id = ?1".to_string(), "is_live = 1".to_string()];
    let mut bind_group: Option<String> = None;
    let mut bind_search: Option<String> = None;

    if let Some(g) = group.filter(|v| !v.is_empty() && *v != "all") {
        if g == "Sem categoria" {
            conditions.push("group_name IS NULL".to_string());
        } else {
            conditions.push("group_name = ?2".to_string());
            bind_group = Some(g.trim().to_string());
        }
    }

    if let Some(s) = search.filter(|v| !v.trim().is_empty()) {
        let placeholder = if bind_group.is_some() { "?3" } else { "?2" };
        conditions.push(format!("name LIKE {placeholder} ESCAPE '\\'"));
        bind_search = Some(format!("%{}%", s.replace('%', "\\%").replace('_', "\\_")));
    }

    let where_clause = conditions.join(" AND ");
    let count_sql = format!("SELECT COUNT(*) FROM channels WHERE {where_clause}");

    let total: i64 = match (&bind_group, &bind_search) {
        (Some(g), Some(s)) => conn.query_row(&count_sql, params![profile_id, g, s], |r| r.get(0))?,
        (Some(g), None) => conn.query_row(&count_sql, params![profile_id, g], |r| r.get(0))?,
        (None, Some(s)) => conn.query_row(&count_sql, params![profile_id, s], |r| r.get(0))?,
        (None, None) => conn.query_row(&count_sql, params![profile_id], |r| r.get(0))?,
    };

    let map_row = |row: &rusqlite::Row<'_>| {
        Ok(Channel {
            id: row.get(0)?,
            profile_id: row.get(1)?,
            name: row.get(2)?,
            logo: row.get(3)?,
            group_name: row.get(4)?,
            stream_url: row.get(5)?,
            tvg_id: row.get(6)?,
            sort_order: row.get(7)?,
        })
    };

    let list_sql = format!(
        "SELECT id, profile_id, name, logo, group_name, stream_url, tvg_id, sort_order
         FROM channels WHERE {where_clause}
         ORDER BY sort_order ASC, name ASC
         LIMIT ? OFFSET ?"
    );

    let mut stmt = conn.prepare(&list_sql)?;
    let items: Vec<Channel> = match (&bind_group, &bind_search) {
        (Some(g), Some(s)) => stmt
            .query_map(params![profile_id, g, s, limit, offset], map_row)?
            .collect::<Result<Vec<_>, _>>()?,
        (Some(g), None) => stmt
            .query_map(params![profile_id, g, limit, offset], map_row)?
            .collect::<Result<Vec<_>, _>>()?,
        (None, Some(s)) => stmt
            .query_map(params![profile_id, s, limit, offset], map_row)?
            .collect::<Result<Vec<_>, _>>()?,
        (None, None) => stmt
            .query_map(params![profile_id, limit, offset], map_row)?
            .collect::<Result<Vec<_>, _>>()?,
    };

    Ok(ChannelsPage {
        items,
        total,
        offset,
        limit,
    })
}

pub fn get_channel(conn: &Connection, id: &str) -> AppResult<Option<Channel>> {
    let mut stmt = conn.prepare(
        "SELECT id, profile_id, name, logo, group_name, stream_url, tvg_id, sort_order
         FROM channels WHERE id = ?1",
    )?;
    let mut rows = stmt.query_map(params![id], |row| {
        Ok(Channel {
            id: row.get(0)?,
            profile_id: row.get(1)?,
            name: row.get(2)?,
            logo: row.get(3)?,
            group_name: row.get(4)?,
            stream_url: row.get(5)?,
            tvg_id: row.get(6)?,
            sort_order: row.get(7)?,
        })
    })?;
    match rows.next() {
        Some(row) => Ok(Some(row?)),
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::Profile;
    use crate::db::{migrations, profiles};

    fn setup() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        migrations::run(&conn).unwrap();
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
        conn
    }

    fn ch(id: &str, name: &str, group: &str, url: &str) -> Channel {
        Channel {
            id: id.to_string(),
            profile_id: "p1".to_string(),
            name: name.to_string(),
            logo: None,
            group_name: Some(group.to_string()),
            stream_url: url.to_string(),
            tvg_id: None,
            sort_order: 0,
        }
    }

    #[test]
    fn live_groups_exclude_vod_urls_only() {
        let conn = setup();
        insert_batch(
            &conn,
            &[
                ch("c1", "Globo SP", "GLOBOS | SUDESTE", "http://h/u/p/1001"),
                ch("c2", "O Filme (2020)", "CRIME", "http://h/movie/u/p/2002.mp4"),
                ch("c3", "Serie X S01E01", "SERIES A", "http://h/series/u/p/3003.mp4"),
                ch("c4", "Megapix", "GLOBOSAT FILMES", "http://h/u/p/4004"),
                ch("c5", "SporTV", "BRASILEIRÃO SÉRIE A", "http://h/u/p/5005"),
            ],
        )
        .unwrap();

        let groups = list_live_groups(&conn, "p1").unwrap();
        assert!(groups.iter().any(|g| g.name == "GLOBOS | SUDESTE" && g.count == 1));
        assert!(
            groups.iter().any(|g| g.name == "GLOBOSAT FILMES" && g.count == 1),
            "live channels in vod-labelled groups must still appear"
        );
        assert!(
            groups.iter().any(|g| g.name == "BRASILEIRÃO SÉRIE A" && g.count == 1),
            "categories containing 'série' must not be dropped"
        );
        assert!(
            !groups.iter().any(|g| g.name == "CRIME"),
            "movie-url group must be excluded"
        );
        assert!(
            !groups.iter().any(|g| g.name == "SERIES A"),
            "series-url group must be excluded"
        );
    }

    #[test]
    fn live_groups_match_list_channels_filters() {
        let conn = setup();
        insert_batch(
            &conn,
            &[
                ch("c1", "Globo SP", "GLOBOS | SUDESTE", "http://h/u/p/1001"),
                ch("c2", "Megapix", "GLOBOSAT FILMES", "http://h/u/p/4004"),
                ch("c3", "O Filme", "CRIME", "http://h/movie/u/p/2002.mp4"),
            ],
        )
        .unwrap();

        let groups = list_live_groups(&conn, "p1").unwrap();
        let page = list_channels(&conn, "p1", None, None, 0, 100).unwrap();

        let group_total: i64 = groups.iter().map(|g| g.count).sum();
        assert_eq!(group_total, page.total);
        for group in &groups {
            let group_page =
                list_channels(&conn, "p1", Some(&group.name), None, 0, 100).unwrap();
            assert_eq!(
                group_page.total, group.count,
                "count for {} must match list_channels",
                group.name
            );
        }
    }

    #[test]
    fn live_groups_keep_provider_order() {
        let conn = setup();
        let mut chans = vec![
            Channel { sort_order: 0, ..ch("a", "Casa", "A CASA DO PATRAO", "http://h/u/p/1") },
            Channel { sort_order: 10, ..ch("g1", "Globo SP", "GLOBOS | SUDESTE", "http://h/u/p/2") },
            Channel { sort_order: 20, ..ch("g2", "Globo RS", "GLOBOS | SUL", "http://h/u/p/3") },
            Channel { sort_order: 30, ..ch("r", "Record", "RECORDTV", "http://h/u/p/4") },
            Channel { sort_order: 40, ..ch("e", "SporTV", "ESPORTES", "http://h/u/p/5") },
        ];
        // Insert shuffled to prove ordering comes from sort_order, not insert order.
        chans.reverse();
        insert_batch(&conn, &chans).unwrap();

        let groups = list_live_groups(&conn, "p1").unwrap();
        assert_eq!(
            groups,
            vec![
                CategoryCount { name: "A CASA DO PATRAO".to_string(), count: 1 },
                CategoryCount { name: "GLOBOS | SUDESTE".to_string(), count: 1 },
                CategoryCount { name: "GLOBOS | SUL".to_string(), count: 1 },
                CategoryCount { name: "RECORDTV".to_string(), count: 1 },
                CategoryCount { name: "ESPORTES".to_string(), count: 1 },
            ]
        );
    }

    #[test]
    fn list_channels_excludes_vod_urls() {
        let conn = setup();
        insert_batch(
            &conn,
            &[
                ch("c1", "Globo SP", "GLOBOS", "http://h/u/p/1001"),
                ch("c2", "O Filme", "CRIME", "http://h/movie/u/p/2002.mp4"),
                ch("c3", "Serie", "SERIES A", "http://h/series/u/p/3.mp4"),
            ],
        )
        .unwrap();
        let page = list_channels(&conn, "p1", None, None, 0, 100).unwrap();
        assert_eq!(page.total, 1);
        assert_eq!(page.items[0].name, "Globo SP");
    }
}
