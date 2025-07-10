/// Initialize a new Script package
use super::{print_info, print_progress, print_success};
use crate::manuscript::templates;
use crate::package::{PackageError, PackageManifest, PackageResult};
use colored::*;
use dialoguer::{Confirm, Input, Select};
use std::env;
use std::fs;
use std::path::Path;

pub async fn execute(name: Option<String>, lib: bool, bin: bool, yes: bool) -> PackageResult<()> {
    let current_dir = env::current_dir()?;

    // Check if script.toml already exists
    if current_dir.join("script.toml").exists() {
        return Err(PackageError::ManifestParse(
            "A script.toml file already exists in this directory".to_string(),
        ));
    }

    // Determine package name
    let package_name = if let Some(name) = name {
        validate_package_name(&name)?;
        name
    } else {
        let default_name = current_dir
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("my-package")
            .to_string();

        if yes {
            default_name
        } else {
            let name: String = Input::new()
                .with_prompt("Package name")
                .default(default_name)
                .interact_text()
                .map_err(|e| {
                    PackageError::Io(std::io::Error::new(std::io::ErrorKind::InvalidInput, e))
                })?;
            validate_package_name(&name)?;
            name
        }
    };

    // Determine package type
    let package_type = if lib && bin {
        PackageType::Both
    } else if lib {
        PackageType::Library
    } else if bin {
        PackageType::Binary
    } else if yes {
        PackageType::Library
    } else {
        let options = vec!["Library", "Binary", "Both"];
        let selection = Select::new()
            .with_prompt("Package type")
            .items(&options)
            .default(0)
            .interact()
            .map_err(|e| {
                PackageError::Io(std::io::Error::new(std::io::ErrorKind::InvalidInput, e))
            })?;

        match selection {
            0 => PackageType::Library,
            1 => PackageType::Binary,
            2 => PackageType::Both,
            _ => unreachable!(),
        }
    };

    // Get author information
    let author = if yes {
        get_git_author().unwrap_or_else(|| "Anonymous".to_string())
    } else {
        let default_author = get_git_author().unwrap_or_else(|| "Anonymous".to_string());
        Input::new()
            .with_prompt("Author")
            .default(default_author)
            .interact_text()
            .map_err(|e| {
                PackageError::Io(std::io::Error::new(std::io::ErrorKind::InvalidInput, e))
            })?
    };

    // Get description
    let description = if yes {
        None
    } else {
        let desc: String = Input::new()
            .with_prompt("Description (optional)")
            .allow_empty(true)
            .interact_text()
            .map_err(|e| {
                PackageError::Io(std::io::Error::new(std::io::ErrorKind::InvalidInput, e))
            })?;
        if desc.is_empty() {
            None
        } else {
            Some(desc)
        }
    };

    // Create package structure
    print_progress("Creating", format!("package '{}'", package_name));

    // Create directories
    fs::create_dir_all("src")?;

    // Create manifest
    let manifest = create_manifest(
        &package_name,
        &author,
        description.as_deref(),
        &package_type,
    );
    let manifest_content = toml::to_string_pretty(&manifest)
        .map_err(|e| PackageError::ManifestParse(e.to_string()))?;
    fs::write("script.toml", manifest_content)?;
    print_success("Created script.toml");

    // Create source files based on package type
    match package_type {
        PackageType::Library => {
            create_library_structure()?;
        }
        PackageType::Binary => {
            create_binary_structure(&package_name)?;
        }
        PackageType::Both => {
            create_library_structure()?;
            create_binary_structure(&package_name)?;
        }
    }

    // Create .gitignore
    fs::write(".gitignore", templates::GITIGNORE_TEMPLATE)?;
    print_success("Created .gitignore");

    // Create README.md
    let readme_content = templates::generate_readme(&package_name, description.as_deref());
    fs::write("README.md", readme_content)?;
    print_success("Created README.md");

    // Initialize git repository if not already in one
    if !is_in_git_repo()? && !yes {
        if Confirm::new()
            .with_prompt("Initialize git repository?")
            .default(true)
            .interact()
            .map_err(|e| {
                PackageError::Io(std::io::Error::new(std::io::ErrorKind::InvalidInput, e))
            })?
        {
            init_git_repo()?;
            print_success("Initialized git repository");
        }
    }

    println!();
    print_success(format!(
        "Created package '{}' at {}",
        package_name.cyan().bold(),
        current_dir.display()
    ));

    print_info("Next steps:");
    println!(
        "  • Run {} to install dependencies",
        "manuscript install".cyan()
    );
    println!("  • Run {} to build the package", "manuscript build".cyan());
    println!("  • Edit {} to add dependencies", "script.toml".cyan());

    Ok(())
}

#[derive(Debug, Clone, Copy)]
enum PackageType {
    Library,
    Binary,
    Both,
}

fn create_manifest(
    name: &str,
    author: &str,
    description: Option<&str>,
    package_type: &PackageType,
) -> PackageManifest {
    let mut manifest = PackageManifest::new(name);
    manifest.package.authors.push(author.to_string());
    manifest.package.description = description.map(|s| s.to_string());
    manifest.package.license = Some("MIT".to_string());
    manifest.package.edition = "2024".to_string();

    match package_type {
        PackageType::Library => {
            manifest.lib = Some(Default::default());
        }
        PackageType::Binary => {
            let mut bin_config = crate::package::BinaryConfig::default();
            bin_config.name = name.to_string();
            manifest.bin.push(bin_config);
        }
        PackageType::Both => {
            manifest.lib = Some(Default::default());
            let mut bin_config = crate::package::BinaryConfig::default();
            bin_config.name = name.to_string();
            manifest.bin.push(bin_config);
        }
    }

    manifest
}

fn create_library_structure() -> PackageResult<()> {
    let lib_content = templates::LIBRARY_TEMPLATE;
    fs::write("src/lib.script", lib_content)?;
    print_success("Created src/lib.script");

    // Create tests directory
    fs::create_dir_all("tests")?;
    fs::write("tests/lib_test.script", templates::LIBRARY_TEST_TEMPLATE)?;
    print_success("Created tests/lib_test.script");

    Ok(())
}

fn create_binary_structure(name: &str) -> PackageResult<()> {
    let main_content = templates::generate_main_file(name);
    fs::write("src/main.script", main_content)?;
    print_success("Created src/main.script");

    Ok(())
}

fn validate_package_name(name: &str) -> PackageResult<()> {
    if name.is_empty() {
        return Err(PackageError::ManifestParse(
            "Package name cannot be empty".to_string(),
        ));
    }

    // Check for valid characters (alphanumeric, dash, underscore)
    if !name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        return Err(PackageError::ManifestParse(
            "Package name can only contain letters, numbers, dashes, and underscores".to_string(),
        ));
    }

    // Check that it doesn't start with a number
    if name.chars().next().unwrap().is_numeric() {
        return Err(PackageError::ManifestParse(
            "Package name cannot start with a number".to_string(),
        ));
    }

    Ok(())
}

fn get_git_author() -> Option<String> {
    // Try to get author from git config
    let output = std::process::Command::new("git")
        .args(&["config", "user.name"])
        .output()
        .ok()?;

    if output.status.success() {
        String::from_utf8(output.stdout)
            .ok()
            .map(|s| s.trim().to_string())
    } else {
        None
    }
}

fn is_in_git_repo() -> PackageResult<bool> {
    Ok(Path::new(".git").exists())
}

fn init_git_repo() -> PackageResult<()> {
    let output = std::process::Command::new("git").arg("init").output()?;

    if !output.status.success() {
        return Err(PackageError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Failed to initialize git repository",
        )));
    }

    Ok(())
}
