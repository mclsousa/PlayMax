use uuid::Uuid;

/// Deterministic catalog id so favorites/history survive profile re-sync.
pub fn stable_channel_id(profile_id: &str, stream_url: &str) -> String {
    stable_catalog_id(profile_id, "channel", stream_url)
}

pub fn stable_movie_id(profile_id: &str, stream_url: &str) -> String {
    stable_catalog_id(profile_id, "movie", stream_url)
}

pub fn stable_series_id(profile_id: &str, series_key: &str) -> String {
    stable_catalog_id(profile_id, "series", series_key)
}

pub fn stable_episode_id(profile_id: &str, stream_url: &str) -> String {
    stable_catalog_id(profile_id, "episode", stream_url)
}

fn stable_catalog_id(profile_id: &str, kind: &str, key: &str) -> String {
    let seed = format!("{profile_id}:{kind}:{}", key.trim());
    Uuid::new_v5(&Uuid::NAMESPACE_URL, seed.as_bytes()).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_channel_id_is_deterministic() {
        let a = stable_channel_id("p1", "http://example.com/live/u/p/1.ts");
        let b = stable_channel_id("p1", "http://example.com/live/u/p/1.ts");
        assert_eq!(a, b);
    }

    #[test]
    fn stable_channel_id_differs_by_stream_url() {
        let a = stable_channel_id("p1", "http://example.com/live/u/p/1.ts");
        let b = stable_channel_id("p1", "http://example.com/live/u/p/2.ts");
        assert_ne!(a, b);
    }
}
