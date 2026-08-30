//! Password health assessment used for the vault "health check".
//!
//! This is a lightweight, offline heuristic (no network, no keylogging, no
//! third-party strength trainer). It scores a password 0..=100 based on
//! length, character-class variety, how many other entries reuse the same
//! password, and how old the password is.

/// Result of assessing a single password.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Health {
    /// 0..=100, higher is stronger.
    pub score: u8,
    /// How many entries are known to share this exact password (>= 1).
    pub used_in: u32,
    /// Short human-readable reason explaining the score.
    pub reason: String,
}

/// Categories a password may draw from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Category {
    Lower,
    Upper,
    Digit,
    Symbol,
    Other,
}

fn categorize(c: char) -> Category {
    if c.is_ascii_lowercase() {
        Category::Lower
    } else if c.is_ascii_uppercase() {
        Category::Upper
    } else if c.is_ascii_digit() {
        Category::Digit
    } else if c.is_ascii_alphanumeric() {
        Category::Other
    } else {
        Category::Symbol
    }
}

/// Number of distinct ASCII categories present in `password`.
fn variety(password: &str) -> usize {
    let mut seen = std::collections::HashSet::new();
    for c in password.chars() {
        seen.insert(categorize(c));
    }
    seen.len()
}

/// Bulk helper: counts how many entries share each password, then produces
/// the `Health` for every `(password, age_days)` pair.
///
/// `all_passwords` must contain the plaintext password of every entry in the
/// vault (in any order). Returns one `Health` per entry, in the same order.
pub fn assess_all<'a, I>(all_passwords: I) -> Vec<Health>
where
    I: IntoIterator<Item = (&'a str, i64)>,
{
    let entries: Vec<(&str, i64)> = all_passwords.into_iter().collect();

    // Password -> use count (identity, not cryptographic).
    let mut counts: std::collections::HashMap<&str, u32> = std::collections::HashMap::new();
    for (password, _) in &entries {
        *counts.entry(password).or_insert(0) += 1;
    }

    entries
        .iter()
        .map(|(password, age_days)| assess(password, counts[password], *age_days))
        .collect()
}

/// Scores a single password. `used_in` is how many vault entries share it;
/// `age_days` is how many days the password has gone unchanged.
pub fn assess(password: &str, used_in: u32, age_days: i64) -> Health {
    let mut score: i64 = 100;
    let mut reason: Option<String> = None;
    let len = password.chars().count();
    let var = variety(password);

    // Most salient finding wins; checks run strongest -> weakest so the first
    // one that fires becomes the reason.
    if matching_common(password) {
        score -= 40;
        reason = Some("Common or easily guessed password".to_string());
    } else if len < 8 {
        score -= 35;
        reason = Some("Too short (fewer than 8 characters)".to_string());
    } else if len < 12 {
        score -= 15;
        reason = Some("On the short side (fewer than 12 characters)".to_string());
    }

    if var <= 1 {
        score -= 20;
        reason.get_or_insert("Uses only one character class".to_string());
    } else if var == 2 {
        score -= 8;
        reason.get_or_insert("Uses only two character classes".to_string());
    }

    if used_in > 1 {
        score -= 25 * (used_in as i64 - 1);
        reason = Some(format!("Reused in {used_in} entries"));
    }

    // Age gates: password left unchanged for a long time is a risk factor.
    if age_days >= 730 {
        score -= 25;
        reason = Some(format!("Unchanged for {age_days} days (over 2 years)"));
    } else if age_days >= 365 {
        score -= 20;
        reason = Some(format!("Unchanged for {age_days} days"));
    } else if age_days >= 180 {
        score -= 8;
        reason.get_or_insert("Getting old (unchanged for 6+ months)".to_string());
    }

    let score = score.clamp(0, 100) as u8;
    Health {
        score,
        used_in,
        reason: reason.unwrap_or_else(|| "Strong".to_string()),
    }
}

/// Cheap check against a small set of extremely common passwords.
fn matching_common(password: &str) -> bool {
    matches!(
        password,
        "123456"
            | "password"
            | "123456789"
            | "qwerty"
            | "abc123"
            | "111111"
            | "12345678"
            | "letmein"
            | "iloveyou"
            | "admin"
            | "welcome"
            | "1234567890"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_passwords_score_low() {
        let h = assess("abc", 1, 0);
        assert!(h.score < 60);
        assert!(!h.reason.is_empty());
    }

    #[test]
    fn reused_password_flags_reuse() {
        let h = assess("Str0ng-Passw0rd!", 3, 0);
        assert_ne!(h.reason, "Strong");
        assert!(h.reason.contains("Reused"));
        assert_eq!(h.used_in, 3);
    }

    #[test]
    fn strong_unique_fresh_password_scores_high() {
        let h = assess("K9!pqRz2@#mX72wL", 1, 30);
        assert_eq!(h.reason, "Strong");
        assert!(h.score >= 90);
    }

    #[test]
    fn stale_password_drops_score() {
        let fresh = assess("K9!pqRz2@#mX72wL", 1, 30).score;
        let stale = assess("K9!pqRz2@#mX72wL", 1, 400).score;
        assert!(stale < fresh);
    }

    #[test]
    fn single_class_deduction_applies() {
        let h = assess("aaaaaaaaaaaa", 1, 0);
        assert!(h.reason.contains("one character class"));
    }

    #[test]
    fn assess_all_counts_reuse() {
        let hs = assess_all([("abc123", 0), ("abc123", 0), ("K9!xYz", 0)]);
        assert_eq!(hs.len(), 3);
        assert_eq!(hs[0].used_in, 2);
        assert_eq!(hs[1].used_in, 2);
        assert_eq!(hs[2].used_in, 1);
    }
}
