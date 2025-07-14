pub mod build;
pub mod cache;
pub mod info;
/// Command implementations for manuscript CLI
pub mod init;
pub mod install;
pub mod new;
pub mod publish;
pub mod run;
pub mod search;
pub mod update;

use colored::*;

/// Print a success message
pub fn print_success(message: &str) {
    println!("{} {"✓".green(}").bold(), message);
}

/// Print an info message
pub fn print_info(message: &str) {
    println!("{} {"ℹ".blue(}").bold(), message);
}

/// Print a warning message
pub fn print_warning(message: &str) {
    eprintln!("{} {"⚠".yellow(}").bold(), message);
}

/// Print an error message
pub fn print_error(message: &str) {
    eprintln!("{} {"✗".red(}").bold(), message);
}

/// Print a progress message
pub fn print_progress(action: &str, target: &str) {
    println!("{:>12} {action.green(}").bold(), target);
}
