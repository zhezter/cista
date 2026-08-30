//! Random password generation with category guarantees.

use rand::seq::SliceRandom;
use rand::thread_rng;

use crate::{CoreError, CoreResult};
const LOWERCASE: &str = "abcdefghijklmnopqrstuvwxyz";
const UPPERCASE: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const DIGITS: &str = "0123456789";
const SYMBOLS: &str = "!@#$%^&*()-_=+";
const AMBIGUOUS: &str = "0O1lI";

pub struct PasswordPolicy {
    pub length: usize,
    pub include_lowercase: bool,
    pub include_uppercase: bool,
    pub include_digits: bool,
    pub include_symbols: bool,
    pub exclude_ambiguous: bool,
}

impl Default for PasswordPolicy {
    fn default() -> Self {
        Self {
            length: 20,
            include_lowercase: true,
            include_uppercase: true,
            include_digits: true,
            include_symbols: true,
            exclude_ambiguous: false,
        }
    }
}

/// Returns the list of active category alphabets (one string per enabled category),
/// with ambiguous characters already filtered out if requested.
fn active_categories(policy: &PasswordPolicy) -> Vec<String> {
    let mut categories = Vec::new();

    let push_filtered = |raw: &str, categories: &mut Vec<String>| {
        let filtered: String = if policy.exclude_ambiguous {
            raw.chars().filter(|c| !AMBIGUOUS.contains(*c)).collect()
        } else {
            raw.to_string()
        };
        if !filtered.is_empty() {
            categories.push(filtered);
        }
    };

    if policy.include_lowercase {
        push_filtered(LOWERCASE, &mut categories);
    }
    if policy.include_uppercase {
        push_filtered(UPPERCASE, &mut categories);
    }
    if policy.include_digits {
        push_filtered(DIGITS, &mut categories);
    }
    if policy.include_symbols {
        push_filtered(SYMBOLS, &mut categories);
    }

    categories
}

/// Generates a random password per `policy`, guaranteeing one char from each
/// enabled category (when `length` allows), then fills and shuffles.
pub fn generate_password(policy: &PasswordPolicy) -> CoreResult<String> {
    if policy.length == 0 {
        return Err(CoreError::ZeroLength);
    }

    let categories = active_categories(policy);
    if categories.is_empty() {
        return Err(CoreError::EmptyCharset);
    }

    let combined: Vec<char> = categories.iter().flat_map(|c| c.chars()).collect();
    let mut rng = thread_rng();

    let mut password_chars: Vec<char> = Vec::with_capacity(policy.length);

    // One char per enabled category first.
    for category in categories.iter().take(policy.length) {
        let chars: Vec<char> = category.chars().collect();
        let chosen = *chars.choose(&mut rng).expect("category is non-empty");
        password_chars.push(chosen);
    }

    // Then fill the rest from the combined alphabet.
    while password_chars.len() < policy.length {
        let chosen = *combined
            .choose(&mut rng)
            .expect("combined alphabet is non-empty");
        password_chars.push(chosen);
    }

    // Shuffle so the guaranteed chars aren't always at the front.
    password_chars.shuffle(&mut rng);

    Ok(password_chars.into_iter().collect())
}

/// A small heuristic that flags weak passwords without any network access.
/// Returns a list of human-readable warnings (empty if the password looks ok).
pub fn password_feedback(password: &str) -> Vec<&'static str> {
    const COMMON: &[&str] = &[
        "123456", "password", "123456789", "qwerty", "abc123", "111111", "12345678", "letmein",
        "iloveyou", "admin", "welcome",
    ];

    let mut warnings: Vec<&'static str> = Vec::new();

    if COMMON.contains(&password) {
        warnings.push("this is an extremely common password");
    }

    let has_lower = password.chars().any(|c| c.is_ascii_lowercase());
    let has_upper = password.chars().any(|c| c.is_ascii_uppercase());
    let has_digit = password.chars().any(|c| c.is_ascii_digit());
    let has_symbol = password.chars().any(|c| !c.is_ascii_alphanumeric());

    let variety = [has_lower, has_upper, has_digit, has_symbol]
        .iter()
        .filter(|&&v| v)
        .count();

    if password.len() < 10 {
        warnings.push("password is shorter than 10 characters");
    } else if password.len() < 16 {
        warnings.push("password is shorter than 16 characters");
    }

    if variety < 2 {
        warnings.push("password uses only one character class (mix cases, digits and symbols)");
    }

    warnings
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_password_of_requested_length() {
        let policy = PasswordPolicy {
            length: 16,
            ..Default::default()
        };
        let password = generate_password(&policy).unwrap();
        assert_eq!(password.chars().count(), 16);
    }

    #[test]
    fn fails_with_zero_length() {
        let policy = PasswordPolicy {
            length: 0,
            ..Default::default()
        };
        assert!(matches!(
            generate_password(&policy),
            Err(CoreError::ZeroLength)
        ));
    }

    #[test]
    fn fails_with_empty_charset() {
        let policy = PasswordPolicy {
            include_lowercase: false,
            include_uppercase: false,
            include_digits: false,
            include_symbols: false,
            ..Default::default()
        };
        assert!(matches!(
            generate_password(&policy),
            Err(CoreError::EmptyCharset)
        ));
    }

    #[test]
    fn excludes_ambiguous_characters_when_requested() {
        let policy = PasswordPolicy {
            length: 200,
            exclude_ambiguous: true,
            ..Default::default()
        };
        let password = generate_password(&policy).unwrap();
        assert!(!password.chars().any(|c| AMBIGUOUS.contains(c)));
    }

    #[test]
    fn guarantees_all_active_categories_present() {
        let policy = PasswordPolicy {
            length: 20,
            ..Default::default()
        };
        let password = generate_password(&policy).unwrap();

        assert!(password.chars().any(|c| LOWERCASE.contains(c)));
        assert!(password.chars().any(|c| UPPERCASE.contains(c)));
        assert!(password.chars().any(|c| DIGITS.contains(c)));
        assert!(password.chars().any(|c| SYMBOLS.contains(c)));
    }

    #[test]
    fn flags_weak_and_common_passwords() {
        assert!(!password_feedback("123456").is_empty());
        assert!(!password_feedback("abc").is_empty());
        assert!(!password_feedback("aaaaaaaaaaaa").is_empty()); // single class
        assert!(password_feedback("Str0ng-Passw0rd!").is_empty());
    }
}
