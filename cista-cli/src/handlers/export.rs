//! Plaintext export of a vault to CSV or JSON.
//!
//! Exporting writes decrypted secrets, so it always requires a confirmation
//! (or the `--yes` flag). Output is a flat row per entry. Dates are RFC 3339.

use crate::cli::ExportFormat;
use crate::prompts::InputSource;
use cista_core::Vault;
use secrecy::ExposeSecret;
use std::io::Write;
use std::path::Path;

/// A single decrypted entry row, ready for serialization.
#[derive(Debug, serde::Serialize)]
struct ExportRow {
    name: String,
    username: String,
    url: String,
    notes: String,
    password: String,
    created_at: String,
    updated_at: String,
}

fn rfc3339(t: time::OffsetDateTime) -> String {
    t.format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| String::new())
}

fn rows_from_vault(vault: &Vault) -> Vec<ExportRow> {
    vault
        .entries()
        .iter()
        .map(|e| ExportRow {
            name: e.name().to_string(),
            username: e.username().unwrap_or("").to_string(),
            url: e.url().unwrap_or("").to_string(),
            notes: e
                .notes()
                .map(|n| n.expose_secret().as_str().to_string())
                .unwrap_or_default(),
            password: e.password().expose_secret().as_str().to_string(),
            created_at: rfc3339(e.created_at()),
            updated_at: rfc3339(e.updated_at()),
        })
        .collect()
}

/// Serialize `rows` to the requested format, returning the bytes to write.
fn serialize(format: ExportFormat, rows: &[ExportRow]) -> anyhow::Result<Vec<u8>> {
    match format {
        ExportFormat::Json => {
            let mut out = Vec::new();
            serde_json::to_writer_pretty(&mut out, rows)?;
            Ok(out)
        }
        ExportFormat::Csv => {
            let mut out = Vec::new();
            {
                let mut wtr = csv::Writer::from_writer(&mut out);
                for row in rows {
                    wtr.write_record([
                        &row.name,
                        &row.username,
                        &row.url,
                        &row.notes,
                        &row.password,
                        &row.created_at,
                        &row.updated_at,
                    ])?;
                }
                wtr.flush()?;
            }
            Ok(out)
        }
    }
}

/// Handler for `export`. Prompts for confirmation (unless `--yes`), then
/// writes decrypted entries to `output` or stdout.
pub fn apply_export(
    vault: &Vault,
    format: ExportFormat,
    output: Option<&Path>,
    yes: bool,
    input: &mut dyn InputSource,
) -> anyhow::Result<()> {
    let count = vault.entries().len();

    if !yes {
        let target = output
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "stdout".to_string());
        if !input.prompt_yes_no(&format!(
            "Export {count} entrie(s) in plaintext to {target}? (y/N): "
        ))? {
            return Ok(());
        }
    }

    let rows = rows_from_vault(vault);
    let bytes = serialize(format, &rows)?;

    match output {
        Some(path) => std::fs::write(path, bytes)?,
        None => std::io::stdout().write_all(&bytes)?,
    }

    println!(
        "{}",
        crate::ui::success(format!(
            "Exported {count} entrie(s) {}",
            format.label()
        ))
    );
    Ok(())
}

impl ExportFormat {
    fn label(self) -> &'static str {
        match self {
            ExportFormat::Csv => "as CSV",
            ExportFormat::Json => "as JSON",
        }
    }
}
