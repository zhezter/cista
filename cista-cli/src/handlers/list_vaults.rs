use std::fs;

pub fn handle_list_vaults() -> anyhow::Result<()> {
    let dir = cista_core::paths::vaults_dir()?;

    let mut names: Vec<String> = Vec::new();
    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let file_type = entry.file_type()?;
            if file_type.is_file() {
                let name = entry.file_name().to_string_lossy().into_owned();
                if name.ends_with(".cista") {
                    names.push(name);
                }
            }
        }
    }

    if names.is_empty() {
        println!("No vaults in {}", dir.display());
        return Ok(());
    }

    names.sort();
    for name in names {
        println!("{name}");
    }

    Ok(())
}
