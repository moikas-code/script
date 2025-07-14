/// Install dependencies for a Script package
use super::{print_info, print_progress, print_success, print_warning};
use crate::manuscript;
use crate::package::{
    DependencySpec, Package, PackageError, PackageManager, PackageManifest, PackageResult,
};
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use std::fs;
use std::path::PathBuf;

pub async fn execute(
    packages: Vec<String>,
    dev: bool,
    save: bool,
    force: bool,
    global: bool,
) -> PackageResult<()> {
    if global {
        return install_global(packages, force).await;
    }

    // Find package root
    let package_root = manuscript::find_package_root(None).ok_or_else(|| {
        PackageError::ManifestParse(
            "Not in a Script package directory. Run 'manuscript init' to create a package."
                .to_string(),
        )
    })?;

    let manifest_path = package_root.join("script.toml");

    if packages.is_empty() {
        // Install from manifest
        install_from_manifest(&manifest_path, force).await
    } else {
        // Install specific packages
        install_packages(&manifest_path, packages, dev, save, force).await
    }
}

async fn install_from_manifest(manifest_path: &PathBuf, force: bool) -> PackageResult<()> {
    print_info("Installing dependencies from script.toml");

    let mut package = Package::from_manifest_file(manifest_path)?;
    let mut manager = PackageManager::new()?;

    // Check if lock file exists
    let lock_path = manifest_path.parent().unwrap().join("script.lock");
    if lock_path.exists() && !force {
        print_info("Using script.lock for consistent dependencies");
        package.lock_file = Some(crate::package::LockFile::from_file(&lock_path)?);
    }

    // Count total dependencies
    let total_deps = package.manifest.dependencies.len() + package.manifest.dev_dependencies.len();
    if total_deps == 0 {
        print_info("No dependencies to install");
        return Ok(());
    }

    // Create progress bar
    let pb = ProgressBar::new(total_deps as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("#>-"),
    );

    // Resolve and install dependencies
    print_progress("Resolving", "dependency graph");
    let graph = manager.resolve_dependencies(&package)?;

    let build_order = graph.build_order();
    pb.set_length(build_order.len() as u64);

    for package_name in build_order {
        pb.set_message(format!("Installing {package_name}"));

        // Install package
        // In a real implementation, this would download and extract the package
        let package_dir = manuscript::cache_dir()?.join("packages").join(package_name);
        if !package_dir.exists() || force {
            fs::create_dir_all(&package_dir)?;
            // Simulate package installation
            pb.inc(1);
        } else {
            pb.set_message(format!("Using cached {package_name}"));
            pb.inc(1);
        }
    }

    pb.finish_with_message("Installation complete");

    // Update lock file
    if package.lock_file.is_none() {
        print_progress("Creating", "script.lock");
        let lock_file = create_lock_file(&graph)?;
        lock_file.save_to_file(&lock_path)?;
    }

    print_success(&format!("Installed {} dependencies", build_order.len()));

    Ok(())
}

async fn install_packages(
    manifest_path: &PathBuf,
    packages: Vec<String>,
    dev: bool,
    save: bool,
    force: bool,
) -> PackageResult<()> {
    if !save {
        print_warning("Installing packages without --save. They won't be added to script.toml");
    }

    let mut manifest = PackageManifest::from_file(manifest_path)?;
    let mut added = Vec::new();

    for package_spec in packages {
        let (name, version) = parse_package_spec(&package_spec)?;

        print_progress("Installing", &format!("{} {name, version}"));

        // Add to manifest if --save
        if save {
            let dep_spec = DependencySpec::Simple(version.clone());
            if dev {
                manifest.dev_dependencies.insert(name.clone(), dep_spec);
            } else {
                manifest.dependencies.insert(name.clone(), dep_spec);
            }
            added.push(name.clone());
        }

        // Install the package
        // In a real implementation, this would download and install
        let package_dir = manuscript::cache_dir()?.join("packages").join(&name);
        if !package_dir.exists() || force {
            fs::create_dir_all(&package_dir)?;
        }

        print_success(&format!("Installed {} {name.cyan(}"), version));
    }

    // Save updated manifest
    if save && !added.is_empty() {
        print_progress("Updating", "script.toml");
        let content = toml::to_string_pretty(&manifest)
            .map_err(|e| PackageError::ManifestParse(e.to_string()))?;
        fs::write(manifest_path, content)?;

        let dep_type = if dev {
            "dev dependencies"
        } else {
            "dependencies"
        };
        print_success(&format!("Added {} packages to {added.len(}"), dep_type));
    }

    Ok(())
}

async fn install_global(packages: Vec<String>, force: bool) -> PackageResult<()> {
    if packages.is_empty() {
        return Err(PackageError::ManifestParse(
            "No packages specified for global installation".to_string(),
        ));
    }

    manuscript::ensure_manuscript_dirs()?;
    let global_dir = manuscript::global_packages_dir()?;

    for package_spec in packages {
        let (name, version) = parse_package_spec(&package_spec)?;

        print_progress("Installing", &format!("{} {} (global)", name, version));

        let package_dir = global_dir.join(&name);
        if package_dir.exists() && !force {
            print_warning(&format!(
                "{} is already installed globally. Use --force to reinstall.",
                name
            ));
            continue;
        }

        // Install globally
        fs::create_dir_all(&package_dir)?;

        // Create symlinks for binaries
        // In a real implementation, this would link executables to PATH

        print_success(&format!("Installed {} {} globally", name.cyan(), version));
    }

    Ok(())
}

fn parse_package_spec(spec: &str) -> PackageResult<(String, String)> {
    if let Some(at_pos) = spec.find('@') {
        let name = spec[..at_pos].to_string();
        let version = spec[at_pos + 1..].to_string();
        Ok((name, version))
    } else {
        // Default to latest version
        Ok((spec.to_string(), "*".to_string()))
    }
}

fn create_lock_file(
    graph: &crate::package::DependencyGraph,
) -> PackageResult<crate::package::LockFile> {
    let mut lock_file = crate::package::LockFile::new();

    for package_name in graph.build_order() {
        if let Some(version) = graph.get_resolved_version(package_name) {
            let entry = crate::package::LockEntry {
                name: package_name.to_string(),
                version: version.to_string(),
                source: "registry".to_string(),
                checksum: None,
                dependencies: graph
                    .get_dependencies(package_name)
                    .map(|deps| deps.iter().map(|d| d.name.clone()).collect())
                    .unwrap_or_default(),
            };
            lock_file.packages.push(entry);
        }
    }

    Ok(lock_file)
}
