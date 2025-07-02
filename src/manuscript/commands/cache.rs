/// Manage the package cache
use super::{print_info, print_success};
use crate::manuscript;
use crate::package::PackageResult;

pub async fn clean(all: bool, _older_than: Option<u32>) -> PackageResult<()> {
    if all {
        let cache_dir = manuscript::cache_dir()?;
        if cache_dir.exists() {
            std::fs::remove_dir_all(&cache_dir)?;
            std::fs::create_dir_all(&cache_dir)?;
            print_success("Cleaned all cache");
        }
    } else {
        print_info("Selective cache cleaning is not yet implemented");
    }
    Ok(())
}

pub async fn list(_size: bool) -> PackageResult<()> {
    print_info("Cache listing is not yet implemented");
    Ok(())
}

pub async fn verify() -> PackageResult<()> {
    print_info("Cache verification is not yet implemented");
    Ok(())
}
