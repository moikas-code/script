/// Self-update functionality for the Script language
use colored::*;
use self_update::cargo_crate_version;
use std::io::{self, Write};

mod updater;
mod docs;

pub use updater::UpdateError;
pub use docs::{DocumentSynchronizer, ValidationRules, ValidationIssue};

/// Check if an update is available
pub fn check_update() -> Result<Option<String>, UpdateError> {
    println!("{} {}", "⏳", "Checking for updates...".bright_blue());

    let current_version = cargo_crate_version!();
    let updater = updater::ScriptUpdater::new()?;

    match updater.get_latest_version()? {
        Some(latest) if latest != current_version => {
            println!(
                "\n{} {} {} {} {}",
                "Update available:".green().bold(),
                current_version.yellow(),
                "→".bright_white(),
                latest.green(),
                "✨"
            );
            Ok(Some(latest))
        }
        _ => {
            println!(
                "{} {} {}",
                "✓".green(),
                "You're already on the latest version".bright_white(),
                current_version.cyan()
            );
            Ok(None)
        }
    }
}

/// Update to the latest version
pub fn update(force: bool) -> Result<(), UpdateError> {
    let current_version = cargo_crate_version!();

    if !force {
        // Check if update is available
        match check_update()? {
            Some(new_version) => {
                // Prompt user for confirmation
                print!(
                    "\n{} {} {} {}? [Y/n] ",
                    "Update Script from".bright_white(),
                    current_version.yellow(),
                    "to".bright_white(),
                    new_version.green()
                );
                io::stdout().flush()?;

                let mut input = String::new();
                io::stdin().read_line(&mut input)?;

                if !input.trim().is_empty() && !input.trim().eq_ignore_ascii_case("y") {
                    println!("{}", "Update cancelled.".yellow());
                    return Ok(());
                }
            }
            None => return Ok(()),
        }
    }

    // Perform update
    println!("\n{} {}", "📦", "Downloading update...".bright_blue());

    let updater = updater::ScriptUpdater::new()?;
    let status = updater.update()?;

    if status.updated() {
        println!(
            "\n{} {} {} {}",
            "✓".green().bold(),
            "Successfully updated to version".bright_white(),
            status.version().green().bold(),
            "🎉"
        );
        println!(
            "\n{}",
            "Please restart Script to use the new version."
                .italic()
                .bright_white()
        );
    } else {
        println!("{} {}", "✓".green(), "Already up to date!".bright_white());
    }

    Ok(())
}

/// Update to a specific version
pub fn update_to_version(version: &str) -> Result<(), UpdateError> {
    println!(
        "{} {} {}",
        "Updating to version".bright_blue(),
        version.cyan(),
        "📦"
    );

    let updater = updater::ScriptUpdater::new()?;
    let status = updater.update_to_version(version)?;

    if status.updated() {
        println!(
            "\n{} {} {} {}",
            "✓".green().bold(),
            "Successfully updated to version".bright_white(),
            status.version().green().bold(),
            "🎉"
        );
        println!(
            "\n{}",
            "Please restart Script to use the new version."
                .italic()
                .bright_white()
        );
    } else {
        println!(
            "{} {}",
            "✓".green(),
            "Already on the requested version!".bright_white()
        );
    }

    Ok(())
}

/// Show available versions
pub fn list_versions() -> Result<(), UpdateError> {
    println!("{}", "Available versions:".bright_blue().bold());

    let updater = updater::ScriptUpdater::new()?;
    let versions = updater.get_available_versions()?;

    let current = cargo_crate_version!();

    for (_i, version) in versions.iter().enumerate().take(10) {
        if version == current {
            println!(
                "  {} {} {}",
                version.green().bold(),
                "(current)".bright_green(),
                "✓"
            );
        } else {
            println!("  {}", version.bright_white());
        }
    }

    if versions.len() > 10 {
        println!(
            "\n  {} {}",
            "...and".italic(),
            format!("{} more versions", versions.len() - 10).italic()
        );
    }

    Ok(())
}

/// Rollback to the previous version
pub fn rollback() -> Result<(), UpdateError> {
    println!("{}", "Rolling back to previous version...".bright_blue());

    let updater = updater::ScriptUpdater::new()?;
    match updater.rollback()? {
        Some(version) => {
            println!(
                "\n{} {} {}",
                "✓".green().bold(),
                "Successfully rolled back to version".bright_white(),
                version.green().bold()
            );
            println!(
                "\n{}",
                "Please restart Script to use the previous version."
                    .italic()
                    .bright_white()
            );
            Ok(())
        }
        None => {
            println!(
                "{} {}",
                "⚠".yellow(),
                "No previous version found to rollback to.".yellow()
            );
            Ok(())
        }
    }
}

/// Update documentation to sync with current project state
pub fn update_docs() -> Result<(), UpdateError> {
    println!("{} {}", "📚", "Updating documentation...".bright_blue());

    let current_dir = std::env::current_dir()
        .map_err(|e| UpdateError::IoError(format!("Failed to get current directory: {}", e)))?;
    
    let mut synchronizer = DocumentSynchronizer::new(&current_dir)?;
    let updated_files = synchronizer.synchronize()?;

    if updated_files.is_empty() {
        println!("{} {}", "✓".green(), "Documentation is already up to date!".bright_white());
    } else {
        println!(
            "\n{} {} {}:",
            "✓".green().bold(),
            "Updated".bright_white(),
            format!("{} files", updated_files.len()).cyan()
        );
        for file in &updated_files {
            println!("  {} {}", "•".bright_blue(), file.bright_white());
        }
    }

    Ok(())
}

/// Check documentation consistency
pub fn check_docs_consistency() -> Result<(), UpdateError> {
    println!("{} {}", "🔍", "Checking documentation consistency...".bright_blue());

    let current_dir = std::env::current_dir()
        .map_err(|e| UpdateError::IoError(format!("Failed to get current directory: {}", e)))?;
    
    let synchronizer = DocumentSynchronizer::new(&current_dir)?;
    let rules = ValidationRules::default();
    let issues = synchronizer.validate(&rules)?;

    if issues.is_empty() {
        println!("{} {}", "✓".green(), "Documentation is consistent!".bright_white());
    } else {
        println!(
            "\n{} {} {}:",
            "⚠".yellow().bold(),
            "Found".bright_white(),
            format!("{} issues", issues.len()).yellow()
        );
        
        for issue in &issues {
            match issue {
                ValidationIssue::VersionMismatch { file, expected, found } => {
                    println!(
                        "  {} Version mismatch in {}: expected {}, found {}",
                        "•".red(),
                        file.bright_white(),
                        expected.green(),
                        found.red()
                    );
                }
                ValidationIssue::MissingVersionReference { file } => {
                    println!(
                        "  {} Missing version reference in {}",
                        "•".red(),
                        file.bright_white()
                    );
                }
                ValidationIssue::MissingCommand { command } => {
                    println!(
                        "  {} Missing command documentation: {}",
                        "•".red(),
                        command.bright_white()
                    );
                }
                ValidationIssue::SyncMismatch { file1, file2 } => {
                    println!(
                        "  {} Sync mismatch between {} and {}",
                        "•".red(),
                        file1.bright_white(),
                        file2.bright_white()
                    );
                }
            }
        }
        
        println!(
            "\n{} {}",
            "💡".bright_yellow(),
            "Run 'script update --docs' to fix these issues.".italic()
        );
    }

    Ok(())
}
