/// Publish a Script package to the registry
use super::print_info;
use crate::package::PackageResult;

pub async fn execute(
    _registry: Option<String>,
    _token: Option<String>,
    _dry_run: bool,
    _allow_dirty: bool,
) -> PackageResult<()> {
    print_info("Publishing to registry is not yet implemented");
    Ok(())
}
