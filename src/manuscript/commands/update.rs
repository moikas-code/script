/// Update dependencies to latest versions
use super::print_info;
use crate::package::PackageResult;

pub async fn execute(
    _packages: Vec<String>,
    _all: bool,
    _patch: bool,
    _minor: bool,
    _major: bool,
    _dry_run: bool,
) -> PackageResult<()> {
    print_info("Updating dependencies is not yet implemented");
    Ok(())
}
