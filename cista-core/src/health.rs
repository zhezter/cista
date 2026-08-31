//! Password health assessment used for the vault "health check".
//!
//! This is a lightweight, offline heuristic (no network, no keylogging, no
//! third-party strength trainer). It uses [zxcvbn]'s entropy estimate to score
//! the intrinsic strength of a password, then applies two contextual
//! penalties that zxcvbn cannot know about: how many other entries reuse the
//! same password, and how old the password is.

use zxcvbn::zxcvbn;

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
    let entropy = zxcvbn(password, &[]);

    // zxcvbn reports the order of magnitude of estimated guesses. Anything at
    // or beyond ~10^12 guesses is effectively un-crackable offline, so cap the
    // scale there and map [0, 12] -> [0, 100]. "a" sits well below 10^3, so a
    // single character scores almost zero instead of the old heuristic's 45.
    let base = (entropy.guesses_log10().clamp(0.0, 12.0) / 12.0 * 100.0).round() as i64;

    let mut score = base;
    let mut reason: Option<String> = None;

    // Contextual penalties zxcvbn has no way to know about.
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

    // When the password itself is weak, prefer zxcvbn's targeted feedback over
    // the generic contextual reasons.
    if reason.is_none() && entropy.score() as u8 <= 2 {
        if let Some(feedback) = entropy.feedback() {
            let mut hints: Vec<String> = feedback
                .suggestions()
                .iter()
                .map(|s| s.to_string())
                .collect();
            if let Some(warning) = feedback.warning() {
                hints.insert(0, warning.to_string());
            }
            reason = Some(hints.join("; "));
        }
    }

    let score = score.clamp(0, 100) as u8;
    Health {
        score,
        used_in,
        reason: reason.unwrap_or_else(|| "Strong".to_string()),
    }
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
    fn single_char_password_scores_near_zero() {
        let h = assess("a", 1, 0);
        assert!(h.score <= 10, "single char scored {}", h.score);
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
        assert!(h.score >= 90, "strong password scored {}", h.score);
    }

    #[test]
    fn stale_password_drops_score() {
        let fresh = assess("K9!pqRz2@#mX72wL", 1, 30).score;
        let stale = assess("K9!pqRz2@#mX72wL", 1, 400).score;
        assert!(stale < fresh);
    }

    #[test]
    fn repeated_pattern_scores_low() {
        let h = assess("aaaaaaaaaaaa", 1, 0);
        assert!(h.score < 60, "repeated pattern scored {}", h.score);
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
