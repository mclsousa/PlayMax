use crate::db::models::Profile;
use crate::db::settings::{self, KEY_ACTIVE_PROFILE};
use crate::db::{profiles, SharedDb};
use crate::error::{AppError, AppResult};
use crate::parsers::m3u::validate_source;
use uuid::Uuid;

pub fn list_profiles(db: &SharedDb) -> AppResult<Vec<Profile>> {
    db.with_conn(profiles::list_profiles)
}

pub fn resolve_active_profile_id(db: &SharedDb) -> AppResult<Option<String>> {
    db.with_conn(|conn| {
        let all = profiles::list_profiles(conn)?;
        if all.is_empty() {
            settings::set_string(conn, KEY_ACTIVE_PROFILE, None)?;
            return Ok(None);
        }

        let stored = settings::get_string(conn, KEY_ACTIVE_PROFILE)?;
        if let Some(id) = stored {
            if all.iter().any(|profile| profile.id == id) {
                return Ok(Some(id));
            }
        }

        let fallback = all[0].id.clone();
        settings::set_string(conn, KEY_ACTIVE_PROFILE, Some(&fallback))?;
        Ok(Some(fallback))
    })
}

pub fn set_active_profile_id(db: &SharedDb, profile_id: &str) -> AppResult<()> {
    db.with_conn(|conn| {
        profiles::get_profile(conn, profile_id)?
            .ok_or_else(|| AppError::msg("Perfil não encontrado."))?;
        settings::set_string(conn, KEY_ACTIVE_PROFILE, Some(profile_id))
    })
}

pub fn add_m3u_profile(
    db: &SharedDb,
    name: String,
    url: Option<String>,
    file_path: Option<String>,
) -> AppResult<Profile> {
    validate_source(url.as_deref(), file_path.as_deref())?;
    let trimmed_name = name.trim();
    if trimmed_name.is_empty() {
        return Err(AppError::msg("Nome da lista é obrigatório."));
    }

    let profile = Profile {
        id: Uuid::new_v4().to_string(),
        name: trimmed_name.to_string(),
        profile_type: "m3u".to_string(),
        url: url.map(|v| v.trim().to_string()).filter(|v| !v.is_empty()),
        file_path: file_path
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty()),
        username: None,
        password: None,
        last_sync: None,
    };

    db.with_conn(|conn| profiles::insert_profile(conn, &profile))?;
    set_active_profile_id(db, &profile.id)?;
    Ok(profile)
}

pub fn add_xtream_profile(
    db: &SharedDb,
    name: String,
    url: String,
    username: String,
    password: String,
) -> AppResult<Profile> {
    let trimmed_name = name.trim();
    if trimmed_name.is_empty() {
        return Err(AppError::msg("Nome da lista é obrigatório."));
    }

    let trimmed_url = url.trim();
    if trimmed_url.is_empty() {
        return Err(AppError::msg("URL do servidor Xtream é obrigatória."));
    }
    if !trimmed_url.starts_with("http://") && !trimmed_url.starts_with("https://") {
        return Err(AppError::msg("URL deve começar com http:// ou https://"));
    }

    let trimmed_username = username.trim();
    if trimmed_username.is_empty() {
        return Err(AppError::msg("Usuário Xtream é obrigatório."));
    }

    let trimmed_password = password.trim();
    if trimmed_password.is_empty() {
        return Err(AppError::msg("Senha Xtream é obrigatória."));
    }

    let profile = Profile {
        id: Uuid::new_v4().to_string(),
        name: trimmed_name.to_string(),
        profile_type: "xtream".to_string(),
        url: Some(trimmed_url.to_string()),
        file_path: None,
        username: Some(trimmed_username.to_string()),
        password: Some(trimmed_password.to_string()),
        last_sync: None,
    };

    db.with_conn(|conn| profiles::insert_profile(conn, &profile))?;
    set_active_profile_id(db, &profile.id)?;
    Ok(profile)
}

pub fn remove_profile(db: &SharedDb, profile_id: &str) -> AppResult<()> {
    db.with_conn(|conn| {
        crate::db::apply_bulk_write_pragmas(conn)?;
        crate::db::catalog_fts::delete_by_profile(conn, profile_id)?;
        // Catalog rows cascade from profiles; avoid redundant per-table deletes.
        profiles::delete_profile(conn, profile_id)?;
        crate::db::restore_write_pragmas(conn)?;
        Ok(())
    })?;
    let _ = resolve_active_profile_id(db);
    Ok(())
}

pub fn get_profile(db: &SharedDb, profile_id: &str) -> AppResult<Profile> {
    db.with_conn(|conn| {
        profiles::get_profile(conn, profile_id)?
            .ok_or_else(|| AppError::msg("Perfil não encontrado."))
    })
}

pub fn update_m3u_profile(
    db: &SharedDb,
    profile_id: &str,
    name: String,
    url: Option<String>,
    file_path: Option<String>,
) -> AppResult<Profile> {
    validate_source(url.as_deref(), file_path.as_deref())?;
    let trimmed_name = name.trim();
    if trimmed_name.is_empty() {
        return Err(AppError::msg("Nome da lista é obrigatório."));
    }

    db.with_conn(|conn| {
        let mut profile = profiles::get_profile(conn, profile_id)?
            .ok_or_else(|| AppError::msg("Perfil não encontrado."))?;

        if profile.profile_type != "m3u" {
            return Err(AppError::msg("Este perfil não é M3U."));
        }

        profile.name = trimmed_name.to_string();
        profile.url = url.map(|v| v.trim().to_string()).filter(|v| !v.is_empty());
        profile.file_path = file_path
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty());
        profile.username = None;
        profile.password = None;

        profiles::update_profile(conn, &profile)?;
        Ok(profile)
    })
}

pub fn update_xtream_profile(
    db: &SharedDb,
    profile_id: &str,
    name: String,
    url: String,
    username: String,
    password: String,
) -> AppResult<Profile> {
    let trimmed_name = name.trim();
    if trimmed_name.is_empty() {
        return Err(AppError::msg("Nome da lista é obrigatório."));
    }

    let trimmed_url = url.trim();
    if trimmed_url.is_empty() {
        return Err(AppError::msg("URL do servidor Xtream é obrigatória."));
    }
    if !trimmed_url.starts_with("http://") && !trimmed_url.starts_with("https://") {
        return Err(AppError::msg("URL deve começar com http:// ou https://"));
    }

    let trimmed_username = username.trim();
    if trimmed_username.is_empty() {
        return Err(AppError::msg("Usuário Xtream é obrigatório."));
    }

    let trimmed_password = password.trim();

    db.with_conn(|conn| {
        let mut profile = profiles::get_profile(conn, profile_id)?
            .ok_or_else(|| AppError::msg("Perfil não encontrado."))?;

        if profile.profile_type != "xtream" {
            return Err(AppError::msg("Este perfil não é Xtream."));
        }

        profile.name = trimmed_name.to_string();
        profile.url = Some(trimmed_url.to_string());
        profile.file_path = None;
        profile.username = Some(trimmed_username.to_string());
        if !trimmed_password.is_empty() {
            profile.password = Some(trimmed_password.to_string());
        }

        profiles::update_profile(conn, &profile)?;
        Ok(profile)
    })
}
