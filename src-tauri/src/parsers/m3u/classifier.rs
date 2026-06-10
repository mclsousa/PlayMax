pub use crate::services::content_kind::{
    classify_group, classify_m3u_entry, classify_m3u_entry as classify_entry, ContentKind,
    looks_like_series_entry,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reexports_unified_classification() {
        assert_eq!(
            classify_entry("Breaking Bad S01E05", "Séries", true),
            ContentKind::Series
        );
        assert_eq!(
            classify_entry("MEGAPIX FHD", "Comedia", false),
            ContentKind::LiveChannel
        );
        assert_eq!(
            classify_entry("UNIVERSAL TV", "Drama", false),
            ContentKind::LiveChannel
        );
        assert_eq!(
            classify_entry("STUDIO UNIVERSAL", "Filmes", false),
            ContentKind::LiveChannel
        );
        assert_eq!(classify_group("Filmes"), ContentKind::Movie);
    }
}
