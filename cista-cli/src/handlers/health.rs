//! Vault-wide password health, mirroring the TUI's health dashboard.

use comfy_table::{Cell, Color, ContentArrangement, Table};
use cista_core::health::{assess_all, Health};
use cista_core::{Entry, Vault};
use secrecy::ExposeSecret;

/// Builds the health report for `vault`: one row per entry, weakest first,
/// followed by a summary line. Returns the plain-text report ready to print.
pub fn health_report(vault: &Vault) -> String {
    let now = time::OffsetDateTime::now_utc();
    let pairs: Vec<(&str, i64)> = vault
        .entries()
        .iter()
        .map(|e| {
            let age_days = (now - e.updated_at()).whole_days().max(0);
            (e.password().expose_secret().as_str(), age_days)
        })
        .collect();
    let healths = assess_all(pairs);

    if healths.is_empty() {
        return "No entries stored.".to_string();
    }

    // Weakest first, mirroring the TUI dashboard so the worst offenders get
    // the most attention.
    let mut rows: Vec<(&Entry, &Health)> = vault.entries().iter().zip(healths.iter()).collect();
    rows.sort_by_key(|(_, h)| h.score);

    let mut table = Table::new();
    table
        .set_header(vec!["Name", "Score", "Used in", "Reason"])
        .load_preset(comfy_table::presets::UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic);

    for (entry, health) in rows {
        let score = format!("{}/100", health.score);
        table.add_row(vec![
            Cell::new(entry.name()),
            Cell::new(score).fg(score_color(health.score)),
            Cell::new(health.used_in),
            Cell::new(&health.reason),
        ]);
    }

    let mut report = table.to_string();

    let weak = healths.iter().filter(|h| h.score < 60).count();
    let reused = healths.iter().filter(|h| h.used_in > 1).count();
    let strong = healths.iter().filter(|h| h.score >= 60).count();

    let summary = if weak == 0 && reused == 0 {
        crate::ui::success(format!("{strong} strong — healthy vault"))
    } else {
        let mut parts = Vec::new();
        if weak > 0 {
            parts.push(format!("{weak} weak"));
        }
        if reused > 0 {
            parts.push(format!("{reused} reused"));
        }
        crate::ui::warn(parts.join(" · "))
    };
    report.push('\n');
    report.push_str(&summary);
    report
}

/// Red for weak, yellow for middling, green for strong, matching the TUI.
fn score_color(score: u8) -> Color {
    if score < 60 {
        Color::Red
    } else if score < 90 {
        Color::Yellow
    } else {
        Color::Green
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cista_core::SecretString;
    use secrecy::Secret;

    fn entry(name: &str, password: &str) -> Entry {
        Entry::new(
            name.to_string(),
            None,
            Secret::new(SecretString::from(password.to_string())),
            None,
            None,
        )
        .expect("entry")
    }

    fn vault_with(entries: Vec<Entry>) -> Vault {
        let mut v = Vault::new();
        for e in entries {
            v.add_entry(e);
        }
        v
    }

    #[test]
    fn health_report_is_empty_without_entries() {
        let report = health_report(&Vault::new());
        assert!(report.contains("No entries stored."));
        assert!(!report.contains("/100"));
    }

    #[test]
    fn health_report_orders_weakest_first() {
        let v = vault_with(vec![
            entry("strong", "K9!pqRz2@#mX72wL"),
            entry("weak", "password"),
            entry("medium", "Str0ng-Passw0rd!"),
        ]);
        let report = health_report(&v);
        let weak_pos = report.find("weak").expect("weak row");
        let strong_pos = report.find("strong").expect("strong row");
        assert!(weak_pos < strong_pos, "weak entry should sort first");
    }

    #[test]
    fn health_report_summarizes_weak_and_reused() {
        let v = vault_with(vec![
            entry("a", "password"),
            entry("b", "password"),
            entry("c", "K9!pqRz2@#mX72wL"),
        ]);
        let report = health_report(&v);
        assert!(report.contains("2 weak"));
        assert!(report.contains("2 reused"));
    }

    #[test]
    fn healthy_vault_gets_success_summary() {
        let v = vault_with(vec![entry("a", "K9!pqRz2@#mX72wL")]);
        assert!(health_report(&v).contains("healthy vault"));
    }
}