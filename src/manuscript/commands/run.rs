/// Run a Script package or command
use super::print_info;
use crate::package::PackageResult;

pub async fn execute(_script: Option<String>, _args: Vec<String>, list: bool) -> PackageResult<()> {
    if list {
        print_info("Listing scripts is not yet implemented");
    } else {
        print_info("Running scripts is not yet implemented");
    }
    Ok(())
}
