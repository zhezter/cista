use crate::prompts::InputSource;
use cista_core::Vault;
use uuid::Uuid;

/// Resolves an entry by name, prompting the user to pick by index when several
/// entries share the name. Returns the chosen entry id.
pub fn resolve_entry_id(
    vault: &Vault,
    name: &str,
    input: &mut dyn InputSource,
) -> anyhow::Result<Uuid> {
    let matches = vault.find_by_name(name);
    match matches.as_slice() {
        [] => anyhow::bail!("No entry found for '{name}'"),
        [single] => Ok(single.id()),
        multiple => {
            println!("Multiple entries found for '{name}':");
            for (i, e) in multiple.iter().enumerate() {
                println!(
                    "  [{}] {} ({})",
                    i,
                    e.name(),
                    e.username().unwrap_or("no username")
                );
            }
            let choice = input.read_line("Choose one by index: ")?;
            let index: usize = choice
                .parse()
                .map_err(|_| anyhow::anyhow!("Invalid selection"))?;
            Ok(multiple
                .get(index)
                .ok_or_else(|| anyhow::anyhow!("Invalid selection"))?
                .id())
        }
    }
}
