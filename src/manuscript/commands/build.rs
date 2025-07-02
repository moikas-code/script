/// Build a Script package
use super::{print_error, print_info, print_progress, print_success};
use crate::compilation::CompilationContext;
use crate::manuscript;
use crate::package::{Package, PackageError, PackageResult};
use crate::{parser::Parser, AstLowerer, CodeGenerator, Lexer};
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use std::fs;
use std::path::{Path, PathBuf};

pub async fn execute(
    release: bool,
    targets: Vec<String>,
    all: bool,
    clean: bool,
) -> PackageResult<()> {
    // Find package root
    let package_root = manuscript::find_package_root(None).ok_or_else(|| {
        PackageError::ManifestParse(
            "Not in a Script package directory. Run 'manuscript init' to create a package."
                .to_string(),
        )
    })?;

    let manifest_path = package_root.join("script.toml");
    let package = Package::from_manifest_file(&manifest_path)?;

    // Clean build directory if requested
    if clean {
        let build_dir = package_root.join("target");
        if build_dir.exists() {
            print_progress("Cleaning", "target directory");
            fs::remove_dir_all(&build_dir)?;
        }
    }

    // Determine build mode
    let mode = if release { "release" } else { "debug" };
    let build_dir = package_root.join("target").join(mode);
    fs::create_dir_all(&build_dir)?;

    print_info(&format!(
        "Building {} package in {} mode",
        package.manifest.package.name.cyan(),
        mode.yellow()
    ));

    // Collect build targets
    let mut build_targets = Vec::new();

    if all || (!targets.is_empty() && targets.contains(&"lib".to_string())) {
        if let Some(lib_path) = package.lib_entry_point() {
            build_targets.push(BuildTarget {
                name: "lib".to_string(),
                path: lib_path,
                kind: TargetKind::Library,
            });
        }
    }

    if all || targets.is_empty() || targets.iter().any(|t| t != "lib") {
        for bin_path in package.bin_entry_points() {
            let bin_name = bin_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("main")
                .to_string();

            if targets.is_empty() || all || targets.contains(&bin_name) {
                build_targets.push(BuildTarget {
                    name: bin_name,
                    path: bin_path,
                    kind: TargetKind::Binary,
                });
            }
        }
    }

    if build_targets.is_empty() {
        return Err(PackageError::ManifestParse(
            "No build targets found. Add [lib] or [[bin]] sections to script.toml".to_string(),
        ));
    }

    // Create progress bar
    let pb = ProgressBar::new(build_targets.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("#>-"),
    );

    // Build each target
    let mut built_count = 0;
    let mut errors = Vec::new();

    for target in &build_targets {
        pb.set_message(format!("Building {} ({})", target.name.cyan(), target.kind));

        match build_target(&package, &target, &build_dir, release).await {
            Ok(output_path) => {
                built_count += 1;
                pb.inc(1);
                print_success(&format!(
                    "Built {} -> {}",
                    target.name.cyan(),
                    output_path.display()
                ));
            }
            Err(e) => {
                errors.push((target.name.clone(), e));
                pb.inc(1);
            }
        }
    }

    pb.finish_and_clear();

    // Report results
    if !errors.is_empty() {
        println!();
        for (name, error) in &errors {
            print_error(&format!("Failed to build {}: {}", name.red(), error));
        }
        return Err(PackageError::ManifestParse(format!(
            "Build failed with {} errors",
            errors.len()
        )));
    }

    print_success(&format!("Successfully built {} targets", built_count));

    Ok(())
}

#[derive(Debug, Clone)]
struct BuildTarget {
    name: String,
    path: PathBuf,
    kind: TargetKind,
}

#[derive(Debug, Clone, Copy)]
enum TargetKind {
    Library,
    Binary,
}

impl std::fmt::Display for TargetKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Library => write!(f, "lib"),
            Self::Binary => write!(f, "bin"),
        }
    }
}

async fn build_target(
    package: &Package,
    target: &BuildTarget,
    build_dir: &Path,
    release: bool,
) -> PackageResult<PathBuf> {
    // Read source file
    let source = fs::read_to_string(&target.path)?;

    // Create compilation context
    let mut context = CompilationContext::new();
    context.set_package_root(package.root_path.clone());
    context.set_release_mode(release);

    // Tokenize
    let lexer = Lexer::new(&source);
    let (tokens, lex_errors) = lexer.scan_tokens();

    if !lex_errors.is_empty() {
        return Err(PackageError::ManifestParse(format!(
            "Lexical errors in {}: {} errors",
            target.path.display(),
            lex_errors.len()
        )));
    }

    // Parse
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().map_err(|e| {
        PackageError::ManifestParse(format!("Parse error in {}: {}", target.path.display(), e))
    })?;

    // Lower to IR
    let symbol_table = crate::semantic::SymbolTable::new();
    let type_info = std::collections::HashMap::new();
    let mut lowerer = AstLowerer::new(symbol_table, type_info);
    let ir_module = lowerer.lower_program(&ast).map_err(|e| {
        PackageError::ManifestParse(format!(
            "Lowering error in {}: {}",
            target.path.display(),
            e
        ))
    })?;

    // Generate code
    let mut generator = CodeGenerator::new();
    let _executable_module = generator.generate(&ir_module).map_err(|e| {
        PackageError::ManifestParse(format!(
            "Code generation error in {}: {}",
            target.path.display(),
            e
        ))
    })?;

    // Determine output path
    let output_name = match target.kind {
        TargetKind::Library => format!("lib{}.script", target.name),
        TargetKind::Binary => format!("{}.script", target.name),
    };
    let output_path = build_dir.join(output_name);

    // For now, just copy the source file as we don't have actual compilation yet
    fs::copy(&target.path, &output_path)?;

    // Make binary executable
    if matches!(target.kind, TargetKind::Binary) {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&output_path)?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&output_path, perms)?;
        }
    }

    Ok(output_path)
}
