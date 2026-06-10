use sha2::{Digest, Sha256};

pub fn hash_pin(pin: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"playmax-parental-v1:");
    hasher.update(pin.trim().as_bytes());
    format!("{:x}", hasher.finalize())
}

pub fn verify_pin(pin: &str, stored_hash: &str) -> bool {
    hash_pin(pin) == stored_hash
}

pub fn is_adult_category(name: &str) -> bool {
    let normalized = name.to_lowercase();
    const KEYWORDS: &[&str] = &[
        "xxx",
        "adult",
        "adulto",
        "+18",
        "18+",
        "erotic",
        "erotico",
        "erótico",
        "porn",
        "sexy",
        "hot",
        "playboy",
        "venus",
        " sex",
        "sex ",
        "onlyfans",
        "privé",
        "privado xxx",
    ];
    KEYWORDS.iter().any(|keyword| normalized.contains(keyword))
}

#[cfg(test)]
mod tests {
    use super::{hash_pin, is_adult_category};

    #[test]
    fn detects_adult_categories() {
        assert!(is_adult_category("Canais XXX"));
        assert!(is_adult_category("Filmes +18"));
        assert!(!is_adult_category("Filmes Ação"));
    }

    #[test]
    fn pin_hash_is_stable() {
        assert_eq!(hash_pin("1234"), hash_pin("1234"));
        assert_ne!(hash_pin("1234"), hash_pin("4321"));
    }
}
