//! Module integrity verification system
//!
//! This module provides cryptographic integrity verification for modules
//! to prevent tampering and ensure authenticity.

use crate::module::{ModuleError, ModulePath, ModuleResult};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

/// Module integrity verifier with checksum and signature support
#[derive(Debug)]
pub struct ModuleIntegrityVerifier {
    /// Known module checksums
    checksums: Arc<RwLock<HashMap<ModulePath, ModuleChecksum>>>,
    /// Trusted module registry
    trusted_registry: Arc<RwLock<TrustedModuleRegistry>>,
    /// Whether to enforce integrity checks
    enforce_integrity: bool,
    /// Cache of verified modules
    verification_cache: Arc<RwLock<HashMap<PathBuf, VerificationResult>>>,
}

/// Checksum information for a module
#[derive(Debug, Clone)]
pub struct ModuleChecksum {
    /// SHA-256 hash of module content
    pub sha256: String,
    /// File size in bytes
    pub size: u64,
    /// Last modification time
    pub modified: std::time::SystemTime,
    /// Optional signature
    pub signature: Option<ModuleSignature>,
}

/// Digital signature for module verification
#[derive(Debug, Clone)]
pub struct ModuleSignature {
    /// Signature algorithm (e.g., "ed25519")
    pub algorithm: String,
    /// Signature bytes (base64 encoded)
    pub signature: String,
    /// Public key identifier
    pub key_id: String,
}

/// Registry of trusted modules
#[derive(Debug, Default)]
pub struct TrustedModuleRegistry {
    /// Trusted module entries
    entries: HashMap<ModulePath, TrustedModuleEntry>,
    /// Trusted public keys
    public_keys: HashMap<String, PublicKeyInfo>,
}

/// Information about a trusted module
#[derive(Debug, Clone)]
pub struct TrustedModuleEntry {
    /// Module path
    pub path: ModulePath,
    /// Expected checksum
    pub checksum: ModuleChecksum,
    /// Trust level
    pub trust_level: TrustLevel,
    /// Verification requirements
    pub requirements: VerificationRequirements,
}

/// Module trust levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TrustLevel {
    /// System modules (highest trust)
    System,
    /// Verified third-party modules
    Verified,
    /// Trusted but unverified
    Trusted,
    /// Unknown modules (lowest trust)
    Unknown,
}

/// Verification requirements for a module
#[derive(Debug, Clone)]
pub struct VerificationRequirements {
    /// Require checksum match
    pub require_checksum: bool,
    /// Require valid signature
    pub require_signature: bool,
    /// Allow newer versions
    pub allow_updates: bool,
    /// Maximum allowed file size
    pub max_size: Option<u64>,
}

impl Default for VerificationRequirements {
    fn default() -> Self {
        VerificationRequirements {
            require_checksum: true,
            require_signature: false,
            allow_updates: false,
            max_size: Some(10_000_000), // 10MB default
        }
    }
}

/// Public key information for signature verification
#[derive(Debug, Clone)]
pub struct PublicKeyInfo {
    /// Key identifier
    pub key_id: String,
    /// Key algorithm
    pub algorithm: String,
    /// Public key bytes (base64 encoded)
    pub public_key: String,
    /// Key owner/description
    pub description: String,
    /// Whether this key is trusted
    pub trusted: bool,
}

/// Result of module verification
#[derive(Debug, Clone)]
pub struct VerificationResult {
    /// Whether verification passed
    pub verified: bool,
    /// Computed checksum
    pub checksum: ModuleChecksum,
    /// Trust level
    pub trust_level: TrustLevel,
    /// Verification warnings
    pub warnings: Vec<String>,
    /// Verification timestamp
    pub timestamp: std::time::SystemTime,
}

impl ModuleIntegrityVerifier {
    /// Create a new integrity verifier
    pub fn new(enforce_integrity: bool) -> Self {
        ModuleIntegrityVerifier {
            checksums: Arc::new(RwLock::new(HashMap::new())),
            trusted_registry: Arc::new(RwLock::new(TrustedModuleRegistry::default())),
            enforce_integrity,
            verification_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Verify module integrity
    pub fn verify_module(
        &self,
        module_path: &ModulePath,
        file_path: &Path,
    ) -> ModuleResult<VerificationResult> {
        // Check cache first
        if let Some(cached) = self.get_cached_verification(file_path) {
            return Ok(cached);
        }

        // Read module content
        let content = fs::read(file_path).map_err(|e| {
            ModuleError::file_error(format!("Failed to read module for verification: {}", e))
        })?;

        // Compute checksum
        let checksum = self.compute_checksum(&content, file_path)?;

        // Determine trust level and verify
        let (trust_level, warnings) = self.verify_trust(module_path, &checksum)?;

        // Create verification result
        let result = VerificationResult {
            verified: trust_level != TrustLevel::Unknown || !self.enforce_integrity,
            checksum: checksum.clone(),
            trust_level,
            warnings,
            timestamp: std::time::SystemTime::now(),
        };

        // Cache result
        self.cache_verification(file_path, result.clone());

        // Enforce integrity if required
        if self.enforce_integrity && !result.verified {
            return Err(ModuleError::security_violation(format!(
                "Module {} failed integrity verification",
                module_path
            )));
        }

        Ok(result)
    }

    /// Compute checksum for module content
    fn compute_checksum(&self, content: &[u8], path: &Path) -> ModuleResult<ModuleChecksum> {
        // Compute SHA-256 hash
        let mut hasher = Sha256::new();
        hasher.update(content);
        let hash = hasher.finalize();
        let sha256 = format!("{:x}", hash);

        // Get file metadata
        let metadata = fs::metadata(path)
            .map_err(|e| ModuleError::file_error(format!("Failed to get file metadata: {}", e)))?;

        Ok(ModuleChecksum {
            sha256,
            size: metadata.len(),
            modified: metadata.modified().unwrap_or(std::time::SystemTime::now()),
            signature: None, // Signatures would be verified separately
        })
    }

    /// Verify module trust level
    fn verify_trust(
        &self,
        module_path: &ModulePath,
        checksum: &ModuleChecksum,
    ) -> ModuleResult<(TrustLevel, Vec<String>)> {
        let mut warnings = Vec::new();
        let registry = self.trusted_registry.read().unwrap();

        // Check if module is in trusted registry
        if let Some(entry) = registry.entries.get(module_path) {
            // Verify checksum if required
            if entry.requirements.require_checksum {
                if entry.checksum.sha256 != checksum.sha256 {
                    if entry.requirements.allow_updates
                        && checksum.modified > entry.checksum.modified
                    {
                        warnings
                            .push("Module has been updated since last verification".to_string());
                    } else {
                        return Ok((
                            TrustLevel::Unknown,
                            vec!["Checksum mismatch - possible tampering detected".to_string()],
                        ));
                    }
                }
            }

            // Verify size constraints
            if let Some(max_size) = entry.requirements.max_size {
                if checksum.size > max_size {
                    warnings.push(format!(
                        "Module size ({} bytes) exceeds limit ({} bytes)",
                        checksum.size, max_size
                    ));
                    return Ok((TrustLevel::Unknown, warnings));
                }
            }

            // Verify signature if required
            if entry.requirements.require_signature {
                if let Some(signature) = &checksum.signature {
                    // Verify the signature against the module content
                    if !self.verify_module_signature_from_checksum(checksum, signature)? {
                        warnings.push("Invalid module signature".to_string());
                        return Ok((TrustLevel::Unknown, warnings));
                    }
                } else {
                    warnings.push("Required signature missing".to_string());
                    return Ok((TrustLevel::Unknown, warnings));
                }
            }

            Ok((entry.trust_level, warnings))
        } else {
            // Unknown module
            warnings.push("Module not in trusted registry".to_string());
            Ok((TrustLevel::Unknown, warnings))
        }
    }

    /// Register a trusted module
    pub fn register_trusted_module(
        &self,
        path: ModulePath,
        checksum: ModuleChecksum,
        trust_level: TrustLevel,
        requirements: VerificationRequirements,
    ) -> ModuleResult<()> {
        let mut registry = self.trusted_registry.write().unwrap();

        registry.entries.insert(
            path.clone(),
            TrustedModuleEntry {
                path,
                checksum,
                trust_level,
                requirements,
            },
        );

        Ok(())
    }

    /// Load trusted module registry from file
    pub fn load_registry(&self, registry_path: &Path) -> ModuleResult<()> {
        // In a real implementation, this would parse a registry file
        // For now, we'll add some default system modules

        let system_modules = vec![
            ("std", "system standard library"),
            ("std.io", "I/O operations"),
            ("std.collections", "data structures"),
            ("std.math", "mathematical functions"),
        ];

        for (module_name, _description) in system_modules {
            let module_path = ModulePath::from_string(module_name)?;

            // System modules get highest trust without verification
            self.register_trusted_module(
                module_path,
                ModuleChecksum {
                    sha256: "system".to_string(),
                    size: 0,
                    modified: std::time::SystemTime::now(),
                    signature: None,
                },
                TrustLevel::System,
                VerificationRequirements {
                    require_checksum: false,
                    require_signature: false,
                    allow_updates: true,
                    max_size: None,
                },
            )?;
        }

        Ok(())
    }

    /// Get cached verification result
    fn get_cached_verification(&self, path: &Path) -> Option<VerificationResult> {
        let cache = self.verification_cache.read().unwrap();
        cache.get(path).cloned()
    }

    /// Cache verification result
    fn cache_verification(&self, path: &Path, result: VerificationResult) {
        let mut cache = self.verification_cache.write().unwrap();
        cache.insert(path.to_path_buf(), result);

        // Limit cache size
        if cache.len() > 1000 {
            // Remove oldest entries
            let mut entries: Vec<_> = cache
                .iter()
                .map(|(k, v)| (k.clone(), v.timestamp))
                .collect();
            entries.sort_by_key(|(_, time)| *time);

            for (path, _) in entries.into_iter().take(100) {
                cache.remove(&path);
            }
        }
    }

    /// Clear verification cache
    pub fn clear_cache(&self) {
        let mut cache = self.verification_cache.write().unwrap();
        cache.clear();
    }

    /// Verify module signature using existing checksum
    fn verify_module_signature_from_checksum(
        &self,
        checksum: &ModuleChecksum,
        signature: &ModuleSignature,
    ) -> ModuleResult<bool> {
        // For production implementation, this would:
        // 1. Parse the signature format (e.g., RSA, Ed25519)
        // 2. Extract the public key from a trusted keystore
        // 3. Verify the signature against the module content hash
        // 4. Check certificate chain and expiration

        // For now, implement a basic signature verification
        // In production, replace with proper cryptographic verification
        let content_hash = &checksum.sha256;

        // Simple verification: signature should contain content hash
        // Production implementation would use proper cryptographic verification
        let is_valid = signature.signature.contains(content_hash);

        if !is_valid {
            println!(
                "Module signature verification failed for content hash: {}",
                content_hash
            );
        }

        Ok(is_valid)
    }
}

/// Module integrity lock file for dependency verification
#[derive(Debug)]
pub struct ModuleLockFile {
    /// Module dependencies with checksums
    pub dependencies: HashMap<ModulePath, ModuleChecksum>,
    /// Lock file version
    pub version: String,
    /// Creation timestamp
    pub created: std::time::SystemTime,
}

impl ModuleLockFile {
    /// Create a new lock file
    pub fn new() -> Self {
        ModuleLockFile {
            dependencies: HashMap::new(),
            version: "1.0".to_string(),
            created: std::time::SystemTime::now(),
        }
    }

    /// Add a dependency to the lock file
    pub fn add_dependency(&mut self, path: ModulePath, checksum: ModuleChecksum) {
        self.dependencies.insert(path, checksum);
    }

    /// Verify all dependencies match lock file
    pub fn verify_dependencies(&self, verifier: &ModuleIntegrityVerifier) -> ModuleResult<()> {
        for (path, expected_checksum) in &self.dependencies {
            let checksums = verifier.checksums.read().unwrap();

            if let Some(actual_checksum) = checksums.get(path) {
                if actual_checksum.sha256 != expected_checksum.sha256 {
                    return Err(ModuleError::security_violation(format!(
                        "Dependency {} checksum mismatch",
                        path
                    )));
                }
            } else {
                return Err(ModuleError::security_violation(format!(
                    "Dependency {} not found",
                    path
                )));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_checksum_computation() {
        let verifier = ModuleIntegrityVerifier::new(false);
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.script");

        let content = b"fn main() { println(\"Hello, World!\"); }";
        fs::write(&test_file, content).unwrap();

        let checksum = verifier.compute_checksum(content, &test_file).unwrap();
        assert!(!checksum.sha256.is_empty());
        assert_eq!(checksum.size, content.len() as u64);
    }

    #[test]
    fn test_trusted_module_verification() {
        let verifier = ModuleIntegrityVerifier::new(true);
        let module_path = ModulePath::from_string("test.module").unwrap();

        // Register a trusted module
        let checksum = ModuleChecksum {
            sha256: "abc123".to_string(),
            size: 100,
            modified: std::time::SystemTime::now(),
            signature: None,
        };

        verifier
            .register_trusted_module(
                module_path.clone(),
                checksum.clone(),
                TrustLevel::Trusted,
                VerificationRequirements::default(),
            )
            .unwrap();

        // Verify trust level
        let (trust_level, warnings) = verifier.verify_trust(&module_path, &checksum).unwrap();
        assert_eq!(trust_level, TrustLevel::Trusted);
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_checksum_mismatch_detection() {
        let verifier = ModuleIntegrityVerifier::new(true);
        let module_path = ModulePath::from_string("test.module").unwrap();

        // Register module with one checksum
        let expected_checksum = ModuleChecksum {
            sha256: "expected123".to_string(),
            size: 100,
            modified: std::time::SystemTime::now(),
            signature: None,
        };

        verifier
            .register_trusted_module(
                module_path.clone(),
                expected_checksum,
                TrustLevel::Trusted,
                VerificationRequirements::default(),
            )
            .unwrap();

        // Verify with different checksum
        let actual_checksum = ModuleChecksum {
            sha256: "different456".to_string(),
            size: 100,
            modified: std::time::SystemTime::now(),
            signature: None,
        };

        let (trust_level, warnings) = verifier
            .verify_trust(&module_path, &actual_checksum)
            .unwrap();
        assert_eq!(trust_level, TrustLevel::Unknown);
        assert!(!warnings.is_empty());
    }
}
