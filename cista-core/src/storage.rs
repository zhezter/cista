use crate::{CoreError, CoreResult, Vault};
use std::{fs, io::Write, path::Path};

pub fn save_new_vault(path: &Path, vault: &Vault, password: &[u8]) -> CoreResult<()> {
    if path.exists() {
        return Err(CoreError::VaultAlreadyExists);
    }
    save_vault_to_path(path, vault, password)
}

pub fn save_vault_to_path(path: &Path, vault: &Vault, password: &[u8]) -> CoreResult<()> {
    let sealed = vault.seal(password)?;

    if path.exists() {
        backup_existing(path)?;
    }

    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(dir)?;
    let mut tmp_file = tempfile::NamedTempFile::new_in(dir)?;

    tmp_file.write_all(&sealed)?;
    tmp_file.as_file().sync_all()?;

    set_owner_only_permissions(tmp_file.path())?;

    tmp_file.persist(path).map_err(|e| CoreError::Io(e.error))?;

    let count = vault.entries().len();
    let _ = crate::config::set_entry_count(path, count);

    Ok(())
}

/// Backs up the current vault file to a timestamped copy under
/// `~/.local/share/cista/backups/<name>/` before overwriting.
fn backup_existing(path: &Path) -> CoreResult<()> {
    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("vault");
    let backup_dir = crate::paths::backups_dir()?.join(name);
    std::fs::create_dir_all(&backup_dir)?;

    let timestamp = time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .map(|s| s.replace(':', "-"))
        .unwrap_or_else(|_| "unknown".to_string());

    let backup_path = backup_dir.join(format!("{timestamp}.cista.bak"));
    std::fs::copy(path, &backup_path)?;
    set_owner_only_permissions(&backup_path)?;
    Ok(())
}

pub fn load_vault_from_path(path: &Path, password: &[u8]) -> CoreResult<Vault> {
    let data = fs::read(path)?;
    let vault = Vault::open(&data, password)?;
    let _ = crate::config::set_entry_count(path, vault.entries().len());
    Ok(vault)
}

#[cfg(unix)]
fn set_owner_only_permissions(path: &Path) -> CoreResult<()> {
    use std::os::unix::fs::PermissionsExt;
    let permissions = fs::Permissions::from_mode(0o600);
    fs::set_permissions(path, permissions)?;
    Ok(())
}

#[cfg(not(unix))]
fn set_owner_only_permissions(_path: &Path) -> CoreResult<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Vault;

    #[test]
    fn save_and_load_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.cista");

        let vault = Vault::new();
        save_new_vault(&path, &vault, b"password123").expect("save should succeed");

        let loaded = load_vault_from_path(&path, b"password123").expect("load should succeed");
        assert_eq!(loaded.entries().len(), 0);
    }

    #[test]
    fn init_fails_if_file_already_exists() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.cista");

        let vault = Vault::new();
        save_new_vault(&path, &vault, b"password123").expect("first save should succeed");

        let result = save_new_vault(&path, &vault, b"password123");
        assert!(result.is_err());
    }

    #[test]
    fn overwriting_creates_timestamped_backup() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.cista");
        let backup_dir = crate::paths::backups_dir().unwrap().join("test");

        // Isolate from any directory left over by previous runs.
        if backup_dir.exists() {
            std::fs::remove_dir_all(&backup_dir).unwrap();
        }

        let mut vault = Vault::new();
        save_new_vault(&path, &vault, b"password123").expect("first save should succeed");
        assert!(!backup_dir.exists());

        let entry = crate::Entry::new(
            "github".to_string(),
            Some("me@example.com".to_string()),
            secrecy::Secret::new(crate::SecretString::from("s3cret".to_string())),
            Some("https://github.com".to_string()),
            None,
        )
        .expect("entry should be valid");
        vault.add_entry(entry);
        save_vault_to_path(&path, &vault, b"password123").expect("overwrite should succeed");

        assert!(backup_dir.exists());
        let backup_files: Vec<_> = std::fs::read_dir(&backup_dir)
            .unwrap()
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().ends_with(".cista.bak"))
            .collect();
        assert_eq!(backup_files.len(), 1);

        let backup_path = backup_files[0].path();
        let backup = load_vault_from_path(&backup_path, b"password123").expect("backup loads");
        assert_eq!(backup.entries().len(), 0);

        std::fs::remove_dir_all(&backup_dir).unwrap();
    }
}
