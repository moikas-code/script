/// Create a new Script package from a template
use super::{print_info, print_success};
use crate::package::PackageResult;
use std::path::PathBuf;

pub async fn execute(
    path: PathBuf,
    _template: Option<String>,
    list_templates: bool,
) -> PackageResult<()> {
    if list_templates {
        print_info("Available templates:");
        println!("  - default: Basic Script package");
        println!("  - lib: Library package");
        println!("  - cli: Command-line application");
        println!("  - web: Web application (coming soon)");
        println!("  - game: Game development (coming soon)");
    } else {
        print_info(&format!(
            "Creating new package at {:?} is not yet fully implemented",
            path
        ));
        // For now, just create the directory
        std::fs::create_dir_all(&path)?;
        print_success(&format!("Created directory at {:?}", path));
    }
    Ok(())
}
