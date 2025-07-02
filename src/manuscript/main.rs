/// Manuscript - Package Manager for Script Language
///
/// This is the main entry point for the manuscript CLI tool.
/// It provides commands for managing Script packages including:
/// - init: Initialize a new Script package
/// - install: Install dependencies
/// - build: Build the package
/// - publish: Publish to registry
/// - search: Search for packages
/// - run: Run a script command
use clap::{Parser, Subcommand};
use colored::*;
use std::path::PathBuf;
use std::process;

#[derive(Parser)]
#[command(
    name = "manuscript",
    version = env!("CARGO_PKG_VERSION"),
    about = "Package manager for the Script programming language",
    long_about = None
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Set the verbosity level
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Disable colored output
    #[arg(long)]
    no_color: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new Script package in the current directory
    Init {
        /// Package name (defaults to directory name)
        #[arg(long)]
        name: Option<String>,

        /// Create a library package
        #[arg(long, conflicts_with = "bin")]
        lib: bool,

        /// Create a binary package
        #[arg(long)]
        bin: bool,

        /// Skip interactive prompts
        #[arg(short, long)]
        yes: bool,
    },

    /// Install dependencies from script.toml
    Install {
        /// Install specific packages
        packages: Vec<String>,

        /// Add packages as dev dependencies
        #[arg(long)]
        dev: bool,

        /// Save packages to script.toml
        #[arg(long)]
        save: bool,

        /// Force reinstall packages
        #[arg(short, long)]
        force: bool,

        /// Install packages globally
        #[arg(short, long)]
        global: bool,
    },

    /// Build the current package
    Build {
        /// Build in release mode
        #[arg(long)]
        release: bool,

        /// Build specific targets
        #[arg(long)]
        target: Vec<String>,

        /// Build all targets
        #[arg(long)]
        all: bool,

        /// Clean before building
        #[arg(long)]
        clean: bool,
    },

    /// Publish package to registry
    Publish {
        /// Registry URL to publish to
        #[arg(long)]
        registry: Option<String>,

        /// Authentication token
        #[arg(long, env = "MANUSCRIPT_TOKEN")]
        token: Option<String>,

        /// Perform dry run without publishing
        #[arg(long)]
        dry_run: bool,

        /// Allow dirty working directory
        #[arg(long)]
        allow_dirty: bool,
    },

    /// Search for packages in the registry
    Search {
        /// Search query
        query: String,

        /// Maximum number of results
        #[arg(long, default_value = "10")]
        limit: usize,

        /// Show full descriptions
        #[arg(long)]
        full: bool,
    },

    /// Run a script or command
    Run {
        /// Script or command to run
        script: Option<String>,

        /// Arguments to pass to the script
        args: Vec<String>,

        /// List available scripts
        #[arg(long)]
        list: bool,
    },

    /// Update dependencies to their latest versions
    Update {
        /// Update specific packages
        packages: Vec<String>,

        /// Update all dependencies
        #[arg(long)]
        all: bool,

        /// Only update patch versions
        #[arg(long)]
        patch: bool,

        /// Only update minor versions
        #[arg(long)]
        minor: bool,

        /// Update to latest major versions
        #[arg(long)]
        major: bool,

        /// Perform dry run
        #[arg(long)]
        dry_run: bool,
    },

    /// Manage package cache
    Cache {
        #[command(subcommand)]
        command: CacheCommands,
    },

    /// Display package information
    Info {
        /// Package name
        package: String,

        /// Show specific version
        #[arg(long)]
        version: Option<String>,

        /// Show dependencies tree
        #[arg(long)]
        deps: bool,

        /// Show all versions
        #[arg(long)]
        versions: bool,
    },

    /// Create a new Script package from a template
    New {
        /// Package path
        path: PathBuf,

        /// Use specific template
        #[arg(long)]
        template: Option<String>,

        /// List available templates
        #[arg(long)]
        list_templates: bool,
    },
}

#[derive(Subcommand)]
enum CacheCommands {
    /// Clean the package cache
    Clean {
        /// Remove all cached packages
        #[arg(long)]
        all: bool,

        /// Remove packages older than days
        #[arg(long)]
        older_than: Option<u32>,
    },

    /// List cached packages
    List {
        /// Show package sizes
        #[arg(long)]
        size: bool,
    },

    /// Verify cache integrity
    Verify,
}

use script::manuscript::commands::{
    build, cache, info, init, install, new, publish, run, search, update,
};

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    // Set up colored output
    if cli.no_color {
        colored::control::set_override(false);
    }

    // Set up logging based on verbosity
    let log_level = match cli.verbose {
        0 => "manuscript=warn",
        1 => "manuscript=info",
        2 => "manuscript=debug",
        _ => "manuscript=trace",
    };
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(log_level)).init();

    let result = match cli.command {
        Some(Commands::Init {
            name,
            lib,
            bin,
            yes,
        }) => init::execute(name, lib, bin, yes).await,
        Some(Commands::Install {
            packages,
            dev,
            save,
            force,
            global,
        }) => install::execute(packages, dev, save, force, global).await,
        Some(Commands::Build {
            release,
            target,
            all,
            clean,
        }) => build::execute(release, target, all, clean).await,
        Some(Commands::Publish {
            registry,
            token,
            dry_run,
            allow_dirty,
        }) => publish::execute(registry, token, dry_run, allow_dirty).await,
        Some(Commands::Search { query, limit, full }) => search::execute(query, limit, full).await,
        Some(Commands::Run { script, args, list }) => run::execute(script, args, list).await,
        Some(Commands::Update {
            packages,
            all,
            patch,
            minor,
            major,
            dry_run,
        }) => update::execute(packages, all, patch, minor, major, dry_run).await,
        Some(Commands::Cache { command }) => match command {
            CacheCommands::Clean { all, older_than } => cache::clean(all, older_than).await,
            CacheCommands::List { size } => cache::list(size).await,
            CacheCommands::Verify => cache::verify().await,
        },
        Some(Commands::Info {
            package,
            version,
            deps,
            versions,
        }) => info::execute(package, version, deps, versions).await,
        Some(Commands::New {
            path,
            template,
            list_templates,
        }) => new::execute(path, template, list_templates).await,
        None => {
            // Show help if no command provided
            use clap::CommandFactory;
            let _ = Cli::command().print_help();
            println!();
            Ok(())
        }
    };

    if let Err(e) = result {
        eprintln!("{} {}", "error:".red().bold(), e);
        process::exit(1);
    }
}
