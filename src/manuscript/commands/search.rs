/// Search for packages in the registry
use super::print_info;
use crate::package::PackageResult;

pub async fn execute(query: String, _limit: usize, _full: bool) -> PackageResult<()> {
    print_info(format!("Searching for '{}' is not yet implemented", query));
    Ok(())
}
