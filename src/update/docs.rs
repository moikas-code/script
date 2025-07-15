/// Documentation synchronization and validation for the Script language
use crate::update::UpdateError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Schema for tracking document synchronization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentSchema {
    /// Version information across files
    pub version: VersionInfo,
    /// Command examples and CLI usage
    pub commands: CommandInfo,
    /// Feature completion tracking
    pub features: FeatureInfo,
    /// Binary information
    pub binaries: BinaryInfo,
    /// Knowledge base structure
    pub knowledge_base: KnowledgeBaseInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    /// Primary version from Cargo.toml
    pub primary: String,
    /// Version references in documentation files
    pub references: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandInfo {
    /// Development commands (cargo build, test, etc.)
    pub development: HashMap<String, String>,
    /// CLI commands (script, script-mcp, etc.)
    pub cli: HashMap<String, String>,
    /// Usage examples in documentation
    pub examples: HashMap<String, Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureInfo {
    /// Overall completion percentage
    pub completion_percentage: f32,
    /// Individual feature status
    pub features: HashMap<String, FeatureStatus>,
    /// Major systems status
    pub systems: HashMap<String, f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureStatus {
    pub completed: bool,
    pub percentage: f32,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinaryInfo {
    /// Available binaries and their purposes
    pub binaries: HashMap<String, String>,
    /// Feature requirements for binaries
    pub feature_requirements: HashMap<String, Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeBaseInfo {
    /// Active issues
    pub active_issues: Vec<String>,
    /// Completed tasks
    pub completed_tasks: Vec<String>,
    /// Overall status files
    pub status_files: Vec<String>,
}

/// Document synchronization manager
pub struct DocumentSynchronizer {
    project_root: PathBuf,
    schema: DocumentSchema,
}

/// Validation rules for document consistency
#[derive(Debug, Clone)]
pub struct ValidationRules {
    /// Files that must contain version references
    pub version_files: Vec<String>,
    /// Required command examples in documentation
    pub required_commands: Vec<String>,
    /// Files that must be kept in sync
    pub sync_pairs: Vec<(String, String)>,
}

impl DocumentSynchronizer {
    /// Create a new document synchronizer
    pub fn new<P: AsRef<Path>>(project_root: P) -> Result<Self, UpdateError> {
        let project_root = project_root.as_ref().to_path_buf();
        let schema = Self::load_schema(&project_root)?;
        
        Ok(Self {
            project_root,
            schema,
        })
    }

    /// Load the current document schema from the project
    fn load_schema(project_root: &Path) -> Result<DocumentSchema, UpdateError> {
        let cargo_toml = project_root.join("Cargo.toml");
        let cargo_content = fs::read_to_string(&cargo_toml)
            .map_err(|e| UpdateError::IoError(format!("Failed to read Cargo.toml: {}", e)))?;
        
        // Parse version from Cargo.toml
        let version = Self::extract_version_from_cargo(&cargo_content)?;
        
        // Scan documentation files for version references
        let version_references = Self::scan_version_references(project_root)?;
        
        // Extract command information
        let commands = Self::extract_command_info(project_root)?;
        
        // Extract feature information
        let features = Self::extract_feature_info(project_root)?;
        
        // Extract binary information
        let binaries = Self::extract_binary_info(&cargo_content)?;
        
        // Extract knowledge base information
        let knowledge_base = Self::extract_kb_info(project_root)?;
        
        Ok(DocumentSchema {
            version: VersionInfo {
                primary: version,
                references: version_references,
            },
            commands,
            features,
            binaries,
            knowledge_base,
        })
    }

    /// Extract version from Cargo.toml content
    fn extract_version_from_cargo(content: &str) -> Result<String, UpdateError> {
        for line in content.lines() {
            if line.trim().starts_with("version = ") {
                if let Some(version) = line.split('"').nth(1) {
                    return Ok(version.to_string());
                }
            }
        }
        Err(UpdateError::ParseError("Version not found in Cargo.toml".to_string()))
    }

    /// Scan documentation files for version references
    fn scan_version_references(project_root: &Path) -> Result<HashMap<String, String>, UpdateError> {
        let mut references = HashMap::new();
        
        // Check README.md
        let readme_path = project_root.join("README.md");
        if readme_path.exists() {
            if let Ok(content) = fs::read_to_string(&readme_path) {
                if let Some(version) = Self::extract_version_from_text(&content) {
                    references.insert("README.md".to_string(), version);
                }
            }
        }
        
        // Check CLAUDE.md
        let claude_path = project_root.join("CLAUDE.md");
        if claude_path.exists() {
            if let Ok(content) = fs::read_to_string(&claude_path) {
                if let Some(version) = Self::extract_version_from_text(&content) {
                    references.insert("CLAUDE.md".to_string(), version);
                }
            }
        }
        
        // Check kb status files
        let kb_status_path = project_root.join("kb/status");
        if kb_status_path.exists() {
            if let Ok(entries) = fs::read_dir(&kb_status_path) {
                for entry in entries.flatten() {
                    if let Some(filename) = entry.file_name().to_str() {
                        if filename.ends_with(".md") {
                            if let Ok(content) = fs::read_to_string(entry.path()) {
                                if let Some(version) = Self::extract_version_from_text(&content) {
                                    references.insert(format!("kb/status/{}", filename), version);
                                }
                            }
                        }
                    }
                }
            }
        }
        
        Ok(references)
    }

    /// Extract version from text content using regex patterns
    fn extract_version_from_text(content: &str) -> Option<String> {
        // Look for patterns like "v0.5.0-alpha", "0.5.0-alpha", "Script Language v0.5.0-alpha"
        for line in content.lines() {
            if line.contains("0.5.0-alpha") || line.contains("v0.5.0") {
                // Extract the version pattern
                let words: Vec<&str> = line.split_whitespace().collect();
                for word in words {
                    if word.contains("0.5.0") && (word.contains("-alpha") || word.starts_with("v")) {
                        return Some(word.trim_start_matches('v').to_string());
                    }
                }
            }
        }
        None
    }

    /// Extract command information from documentation
    fn extract_command_info(project_root: &Path) -> Result<CommandInfo, UpdateError> {
        let mut development = HashMap::new();
        let mut cli = HashMap::new();
        let mut examples = HashMap::new();
        
        // Extract from CLAUDE.md
        let claude_path = project_root.join("CLAUDE.md");
        if claude_path.exists() {
            if let Ok(content) = fs::read_to_string(&claude_path) {
                Self::parse_commands_from_claude(&content, &mut development, &mut cli, &mut examples);
            }
        }
        
        // Extract from README.md
        let readme_path = project_root.join("README.md");
        if readme_path.exists() {
            if let Ok(content) = fs::read_to_string(&readme_path) {
                Self::parse_commands_from_readme(&content, &mut development, &mut cli, &mut examples);
            }
        }
        
        Ok(CommandInfo {
            development,
            cli,
            examples,
        })
    }

    /// Parse commands from CLAUDE.md content
    fn parse_commands_from_claude(
        content: &str,
        development: &mut HashMap<String, String>,
        cli: &mut HashMap<String, String>,
        examples: &mut HashMap<String, Vec<String>>,
    ) {
        let lines: Vec<&str> = content.lines().collect();
        let mut in_code_block = false;
        let mut current_section = "";
        
        for (i, line) in lines.iter().enumerate() {
            if line.starts_with("```") {
                in_code_block = !in_code_block;
                continue;
            }
            
            // Track sections
            if line.starts_with("##") {
                current_section = line.trim_start_matches("# ").trim();
            }
            
            if in_code_block && (line.starts_with("cargo ") || line.starts_with("script ")) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    let command = format!("{} {}", parts[0], parts[1]);
                    
                    // Look for comment on same line or next line
                    let description = if line.contains(" # ") {
                        line.split(" # ").nth(1).unwrap_or("").to_string()
                    } else if i + 1 < lines.len() && lines[i + 1].trim().starts_with('#') {
                        lines[i + 1].trim_start_matches('#').trim().to_string()
                    } else {
                        "No description".to_string()
                    };
                    
                    if line.starts_with("cargo ") {
                        development.insert(command, description);
                    } else if line.starts_with("script ") {
                        cli.insert(command, description);
                    }
                    
                    // Add to examples for the current section
                    examples.entry(current_section.to_string())
                        .or_insert_with(Vec::new)
                        .push(line.to_string());
                }
            }
        }
    }

    /// Parse commands from README.md content
    fn parse_commands_from_readme(
        content: &str,
        _development: &mut HashMap<String, String>,
        _cli: &mut HashMap<String, String>,
        _examples: &mut HashMap<String, Vec<String>>,
    ) {
        // Similar parsing logic for README.md
        // This can be implemented based on README.md structure
        let _ = content; // Placeholder to avoid unused parameter warning
    }

    /// Extract feature information from documentation and code
    fn extract_feature_info(project_root: &Path) -> Result<FeatureInfo, UpdateError> {
        let mut features = HashMap::new();
        let mut systems = HashMap::new();
        
        // Look for completion percentage in README.md and status files
        let readme_path = project_root.join("README.md");
        if readme_path.exists() {
            if let Ok(content) = fs::read_to_string(&readme_path) {
                if let Some(_percentage) = Self::extract_completion_percentage(&content) {
                    // Default overall completion from README
                }
            }
        }
        
        // Check kb/status files for detailed feature information
        let kb_status_path = project_root.join("kb/status");
        if kb_status_path.exists() {
            if let Ok(entries) = fs::read_dir(&kb_status_path) {
                for entry in entries.flatten() {
                    if let Ok(content) = fs::read_to_string(entry.path()) {
                        Self::parse_feature_status(&content, &mut features, &mut systems);
                    }
                }
            }
        }
        
        Ok(FeatureInfo {
            completion_percentage: 90.0, // Default from project overview
            features,
            systems,
        })
    }

    /// Extract completion percentage from text
    fn extract_completion_percentage(content: &str) -> Option<f32> {
        for line in content.lines() {
            if line.contains('%') && (line.contains("complete") || line.contains("done")) {
                // Extract percentage
                let words: Vec<&str> = line.split_whitespace().collect();
                for word in words {
                    if word.ends_with('%') {
                        if let Ok(percentage) = word.trim_end_matches('%').parse::<f32>() {
                            return Some(percentage);
                        }
                    }
                }
            }
        }
        None
    }

    /// Parse feature status from content
    fn parse_feature_status(
        content: &str,
        features: &mut HashMap<String, FeatureStatus>,
        systems: &mut HashMap<String, f32>,
    ) {
        // Parse markdown content for feature status
        // Look for patterns like "- [x] Feature name (90%)"
        for line in content.lines() {
            if line.contains("- [") {
                let completed = line.contains("- [x]");
                if let Some(feature_part) = line.split("- [").nth(1) {
                    if let Some(feature_name) = feature_part.split(']').nth(1) {
                        let name = feature_name.trim();
                        let percentage = Self::extract_completion_percentage(line).unwrap_or(if completed { 100.0 } else { 0.0 });
                        
                        features.insert(name.to_string(), FeatureStatus {
                            completed,
                            percentage,
                            description: name.to_string(),
                        });
                        
                        // Also track as system if it's a major component
                        if name.contains("System") || name.contains("Core") || name.contains("Engine") {
                            systems.insert(name.to_string(), percentage);
                        }
                    }
                }
            }
        }
    }

    /// Extract binary information from Cargo.toml
    fn extract_binary_info(cargo_content: &str) -> Result<BinaryInfo, UpdateError> {
        let mut binaries = HashMap::new();
        let mut feature_requirements = HashMap::new();
        
        let lines: Vec<&str> = cargo_content.lines().collect();
        let mut i = 0;
        
        while i < lines.len() {
            let line = lines[i].trim();
            
            if line.starts_with("[[bin]]") {
                // Parse binary information
                let mut name = None;
                let mut required_features = Vec::new();
                
                // Look ahead for name and required-features
                for j in (i + 1)..lines.len() {
                    let next_line = lines[j].trim();
                    if next_line.starts_with("[[") {
                        break; // Next section
                    }
                    
                    if next_line.starts_with("name = ") {
                        name = next_line.split('"').nth(1).map(|s| s.to_string());
                    } else if next_line.starts_with("required-features = ") {
                        // Parse required features array
                        if let Some(features_str) = next_line.split('[').nth(1) {
                            if let Some(features_str) = features_str.split(']').next() {
                                required_features = features_str
                                    .split(',')
                                    .map(|s| s.trim().trim_matches('"').to_string())
                                    .collect();
                            }
                        }
                    }
                }
                
                if let Some(bin_name) = name {
                    // Add description based on binary name
                    let description = match bin_name.as_str() {
                        "script" | "script-lang" => "Main Script language interpreter",
                        "script-mcp" => "Model Context Protocol server",
                        "script-lsp" => "Language Server Protocol implementation",
                        "manuscript" => "Script package manager",
                        "script-debug" => "Script debugger",
                        "script-test" => "Script testing framework",
                        _ => "Script binary",
                    };
                    
                    binaries.insert(bin_name.clone(), description.to_string());
                    
                    if !required_features.is_empty() {
                        feature_requirements.insert(bin_name, required_features);
                    }
                }
            }
            
            i += 1;
        }
        
        Ok(BinaryInfo {
            binaries,
            feature_requirements,
        })
    }

    /// Extract knowledge base information
    fn extract_kb_info(project_root: &Path) -> Result<KnowledgeBaseInfo, UpdateError> {
        let mut active_issues = Vec::new();
        let mut completed_tasks = Vec::new();
        let mut status_files = Vec::new();
        
        let kb_path = project_root.join("kb");
        if kb_path.exists() {
            // Scan active issues
            let active_path = kb_path.join("active");
            if active_path.exists() {
                if let Ok(entries) = fs::read_dir(&active_path) {
                    for entry in entries.flatten() {
                        if let Some(filename) = entry.file_name().to_str() {
                            if filename.ends_with(".md") {
                                active_issues.push(filename.to_string());
                            }
                        }
                    }
                }
            }
            
            // Scan completed tasks
            let completed_path = kb_path.join("completed");
            if completed_path.exists() {
                if let Ok(entries) = fs::read_dir(&completed_path) {
                    for entry in entries.flatten() {
                        if let Some(filename) = entry.file_name().to_str() {
                            if filename.ends_with(".md") {
                                completed_tasks.push(filename.to_string());
                            }
                        }
                    }
                }
            }
            
            // Scan status files
            let status_path = kb_path.join("status");
            if status_path.exists() {
                if let Ok(entries) = fs::read_dir(&status_path) {
                    for entry in entries.flatten() {
                        if let Some(filename) = entry.file_name().to_str() {
                            if filename.ends_with(".md") {
                                status_files.push(filename.to_string());
                            }
                        }
                    }
                }
            }
        }
        
        Ok(KnowledgeBaseInfo {
            active_issues,
            completed_tasks,
            status_files,
        })
    }

    /// Validate document consistency
    pub fn validate(&self, rules: &ValidationRules) -> Result<Vec<ValidationIssue>, UpdateError> {
        let mut issues = Vec::new();
        
        // Check version consistency
        for version_file in &rules.version_files {
            if let Some(file_version) = self.schema.version.references.get(version_file) {
                if file_version != &self.schema.version.primary {
                    issues.push(ValidationIssue::VersionMismatch {
                        file: version_file.clone(),
                        expected: self.schema.version.primary.clone(),
                        found: file_version.clone(),
                    });
                }
            } else {
                issues.push(ValidationIssue::MissingVersionReference {
                    file: version_file.clone(),
                });
            }
        }
        
        // Check required commands
        for required_cmd in &rules.required_commands {
            let found_in_dev = self.schema.commands.development.contains_key(required_cmd);
            let found_in_cli = self.schema.commands.cli.contains_key(required_cmd);
            
            if !found_in_dev && !found_in_cli {
                issues.push(ValidationIssue::MissingCommand {
                    command: required_cmd.clone(),
                });
            }
        }
        
        // Check sync pairs
        for (file1, file2) in &rules.sync_pairs {
            if let (Some(examples1), Some(examples2)) = (
                self.schema.commands.examples.get(file1),
                self.schema.commands.examples.get(file2),
            ) {
                if examples1 != examples2 {
                    issues.push(ValidationIssue::SyncMismatch {
                        file1: file1.clone(),
                        file2: file2.clone(),
                    });
                }
            }
        }
        
        Ok(issues)
    }

    /// Synchronize documents based on the schema
    pub fn synchronize(&mut self) -> Result<Vec<String>, UpdateError> {
        let mut updated_files = Vec::new();
        
        // Update version references
        for (file_path, _current_version) in &self.schema.version.references.clone() {
            let full_path = self.project_root.join(file_path);
            if full_path.exists() {
                if let Ok(content) = fs::read_to_string(&full_path) {
                    let updated_content = self.update_version_in_content(&content, &self.schema.version.primary);
                    if updated_content != content {
                        fs::write(&full_path, updated_content)
                            .map_err(|e| UpdateError::IoError(format!("Failed to write {}: {}", file_path, e)))?;
                        updated_files.push(file_path.clone());
                    }
                }
            }
        }
        
        Ok(updated_files)
    }

    /// Update version references in content
    fn update_version_in_content(&self, content: &str, new_version: &str) -> String {
        // Replace version patterns
        content
            .replace("0.5.0-alpha", new_version)
            .replace("v0.5.0-alpha", &format!("v{}", new_version))
    }
}

/// Validation issues found during document checking
#[derive(Debug, Clone)]
pub enum ValidationIssue {
    VersionMismatch {
        file: String,
        expected: String,
        found: String,
    },
    MissingVersionReference {
        file: String,
    },
    MissingCommand {
        command: String,
    },
    SyncMismatch {
        file1: String,
        file2: String,
    },
}

impl Default for ValidationRules {
    fn default() -> Self {
        Self {
            version_files: vec![
                "README.md".to_string(),
                "CLAUDE.md".to_string(),
                "kb/status/OVERALL_STATUS.md".to_string(),
            ],
            required_commands: vec![
                "cargo build".to_string(),
                "cargo test".to_string(),
                "cargo run".to_string(),
            ],
            sync_pairs: vec![
                ("CLAUDE.md".to_string(), "README.md".to_string()),
            ],
        }
    }
}