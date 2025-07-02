/// Display package information
use super::print_info;
use crate::package::PackageResult;

pub async fn execute(
    package: String,
    _version: Option<String>,
    _deps: bool,
    _versions: bool,
) -> PackageResult<()> {
    print_info(&format!(
        "Package info for '{}' is not yet implemented",
        package
    ));
    Ok(())
}
