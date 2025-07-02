/// Package caching system for Script language packages
///
/// This module provides:
/// - Local package caching to avoid repeated downloads
/// - Cache management and cleanup
/// - Package integrity verification
/// - Cache statistics and monitoring
use super::{PackageError, PackageResult, Version};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Package cache manager
pub struct PackageCache {
    cache_dir: PathBuf,
    config: CacheConfig,
    index: CacheIndex,
}

impl PackageCache {
    /// Create a new package cache with default configuration
    pub fn new() -> PackageResult<Self> {
        Self::with_config(CacheConfig::default())
    }

    /// Create a package cache with custom configuration
    pub fn with_config(config: CacheConfig) -> PackageResult<Self> {
        let cache_dir = config.cache_dir.clone();

        // Create cache directory if it doesn't exist
        if !cache_dir.exists() {
            fs::create_dir_all(&cache_dir)?;
        }

        // Load or create cache index
        let index_path = cache_dir.join("index.json");
        let index = if index_path.exists() {
            CacheIndex::load(&index_path)?
        } else {
            CacheIndex::new()
        };

        let mut cache = Self {
            cache_dir,
            config,
            index,
        };

        // Perform initial cleanup if needed
        cache.cleanup_if_needed()?;

        Ok(cache)
    }

    /// Check if a package is cached
    pub fn has_package(&self, name: &str, version: &Version) -> PackageResult<bool> {
        let key = package_key(name, version);
        Ok(self.index.entries.contains_key(&key))
    }

    /// Get a cached package
    pub fn get_package(&self, name: &str, version: &Version) -> PackageResult<Vec<u8>> {
        let key = package_key(name, version);

        if let Some(entry) = self.index.entries.get(&key) {
            let package_path = self.cache_dir.join(&entry.path);

            if package_path.exists() {
                // Verify integrity if enabled
                if self.config.verify_integrity {
                    let computed_hash = compute_file_hash(&package_path)?;
                    if computed_hash != entry.checksum {
                        return Err(PackageError::Cache(format!(
                            "Package integrity check failed for {}",
                            key
                        )));
                    }
                }

                // Update access time
                self.update_access_time(&key)?;

                return Ok(fs::read(&package_path)?);
            } else {
                // Package file is missing, remove from index
                self.remove_from_index(&key)?;
            }
        }

        Err(PackageError::PackageNotFound {
            name: name.to_string(),
        })
    }

    /// Store a package in the cache
    pub fn store_package(&self, name: &str, version: &Version, data: Vec<u8>) -> PackageResult<()> {
        let key = package_key(name, version);
        let filename = format!("{}.tar.gz", key);
        let package_path = self.cache_dir.join(&filename);

        // Write package data
        fs::write(&package_path, &data)?;

        // Compute checksum
        let checksum = compute_file_hash(&package_path)?;

        // Create cache entry
        let entry = CacheEntry {
            name: name.to_string(),
            version: version.clone(),
            path: filename,
            checksum,
            size: data.len() as u64,
            created_at: current_timestamp(),
            accessed_at: current_timestamp(),
            download_url: None,
        };

        // Add to index
        self.add_to_index(key, entry)?;

        // Check if cleanup is needed after adding
        self.cleanup_if_needed()?;

        Ok(())
    }

    /// Remove a package from the cache
    pub fn remove_package(&self, name: &str, version: &Version) -> PackageResult<()> {
        let key = package_key(name, version);

        if let Some(entry) = self.index.entries.get(&key) {
            let package_path = self.cache_dir.join(&entry.path);

            // Remove file if it exists
            if package_path.exists() {
                fs::remove_file(&package_path)?;
            }

            // Remove from index
            self.remove_from_index(&key)?;
        }

        Ok(())
    }

    /// Clear all cached packages
    pub fn clear(&self) -> PackageResult<()> {
        // Remove all package files
        for entry in self.index.entries.values() {
            let package_path = self.cache_dir.join(&entry.path);
            if package_path.exists() {
                fs::remove_file(&package_path)?;
            }
        }

        // Clear index
        let empty_index = CacheIndex::new();
        self.save_index(&empty_index)?;

        Ok(())
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        let total_size = self.index.entries.values().map(|e| e.size).sum();
        let total_packages = self.index.entries.len();

        let oldest_access = self
            .index
            .entries
            .values()
            .map(|e| e.accessed_at)
            .min()
            .unwrap_or(0);

        let newest_access = self
            .index
            .entries
            .values()
            .map(|e| e.accessed_at)
            .max()
            .unwrap_or(0);

        CacheStats {
            total_packages,
            total_size,
            cache_dir: self.cache_dir.clone(),
            oldest_access,
            newest_access,
        }
    }

    /// Perform cache cleanup if needed
    fn cleanup_if_needed(&self) -> PackageResult<()> {
        let stats = self.stats();
        let max_size = self.config.max_size_bytes;

        if stats.total_size > max_size {
            let target_size = (max_size as f64 * self.config.cleanup_threshold) as u64;
            self.cleanup_to_size(target_size)?;
        }

        Ok(())
    }

    /// Clean up cache to reach target size
    fn cleanup_to_size(&self, target_size: u64) -> PackageResult<()> {
        let mut entries: Vec<_> = self.index.entries.iter().collect();

        // Sort by access time (least recently accessed first)
        entries.sort_by_key(|(_, entry)| entry.accessed_at);

        let mut current_size = entries.iter().map(|(_, e)| e.size).sum::<u64>();
        let mut to_remove = Vec::new();

        for (key, entry) in entries {
            if current_size <= target_size {
                break;
            }

            current_size -= entry.size;
            to_remove.push(key.clone());
        }

        // Remove selected packages
        for key in to_remove {
            if let Some(entry) = self.index.entries.get(&key) {
                let package_path = self.cache_dir.join(&entry.path);
                if package_path.exists() {
                    fs::remove_file(&package_path)?;
                }
            }
            self.remove_from_index(&key)?;
        }

        Ok(())
    }

    /// Update access time for a package
    fn update_access_time(&self, key: &str) -> PackageResult<()> {
        let mut index = self.load_index()?;

        if let Some(entry) = index.entries.get_mut(key) {
            entry.accessed_at = current_timestamp();
            self.save_index(&index)?;
        }

        Ok(())
    }

    /// Add entry to index
    fn add_to_index(&self, key: String, entry: CacheEntry) -> PackageResult<()> {
        let mut index = self.load_index()?;
        index.entries.insert(key, entry);
        self.save_index(&index)?;
        Ok(())
    }

    /// Remove entry from index
    fn remove_from_index(&self, key: &str) -> PackageResult<()> {
        let mut index = self.load_index()?;
        index.entries.remove(key);
        self.save_index(&index)?;
        Ok(())
    }

    /// Load index from disk
    fn load_index(&self) -> PackageResult<CacheIndex> {
        let index_path = self.cache_dir.join("index.json");
        CacheIndex::load(&index_path)
    }

    /// Save index to disk
    fn save_index(&self, index: &CacheIndex) -> PackageResult<()> {
        let index_path = self.cache_dir.join("index.json");
        index.save(&index_path)
    }
}

/// Configuration for package cache
#[derive(Debug, Clone)]
pub struct CacheConfig {
    pub cache_dir: PathBuf,
    pub max_size_bytes: u64,
    pub cleanup_threshold: f64,
    pub verify_integrity: bool,
    pub compression_level: u32,
}

impl Default for CacheConfig {
    fn default() -> Self {
        let cache_dir = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("script")
            .join("packages");

        Self {
            cache_dir,
            max_size_bytes: 1024 * 1024 * 1024, // 1GB
            cleanup_threshold: 0.8,             // Clean up when 80% full
            verify_integrity: true,
            compression_level: 6,
        }
    }
}

/// Cache index for tracking cached packages
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheIndex {
    version: String,
    entries: HashMap<String, CacheEntry>,
    created_at: u64,
    last_cleanup: u64,
}

impl CacheIndex {
    fn new() -> Self {
        let now = current_timestamp();
        Self {
            version: "1".to_string(),
            entries: HashMap::new(),
            created_at: now,
            last_cleanup: now,
        }
    }

    fn load(path: &Path) -> PackageResult<Self> {
        if !path.exists() {
            return Ok(Self::new());
        }

        let content = fs::read_to_string(path)?;
        let index: CacheIndex = serde_json::from_str(&content)
            .map_err(|e| PackageError::Cache(format!("Failed to parse cache index: {}", e)))?;

        Ok(index)
    }

    fn save(&self, path: &Path) -> PackageResult<()> {
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| PackageError::Cache(format!("Failed to serialize cache index: {}", e)))?;

        fs::write(path, content)?;
        Ok(())
    }
}

/// Individual cache entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    pub name: String,
    pub version: Version,
    pub path: String,
    pub checksum: String,
    pub size: u64,
    pub created_at: u64,
    pub accessed_at: u64,
    pub download_url: Option<String>,
}

impl CacheEntry {
    pub fn new(
        name: impl Into<String>,
        version: Version,
        path: impl Into<String>,
        checksum: impl Into<String>,
        size: u64,
    ) -> Self {
        let now = current_timestamp();
        Self {
            name: name.into(),
            version,
            path: path.into(),
            checksum: checksum.into(),
            size,
            created_at: now,
            accessed_at: now,
            download_url: None,
        }
    }

    pub fn age_seconds(&self) -> u64 {
        current_timestamp().saturating_sub(self.created_at)
    }

    pub fn last_accessed_seconds(&self) -> u64 {
        current_timestamp().saturating_sub(self.accessed_at)
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub total_packages: usize,
    pub total_size: u64,
    pub cache_dir: PathBuf,
    pub oldest_access: u64,
    pub newest_access: u64,
}

impl CacheStats {
    pub fn size_mb(&self) -> f64 {
        self.total_size as f64 / (1024.0 * 1024.0)
    }

    pub fn size_gb(&self) -> f64 {
        self.total_size as f64 / (1024.0 * 1024.0 * 1024.0)
    }
}

/// Cache manager for advanced cache operations
pub struct CacheManager {
    cache: PackageCache,
}

impl CacheManager {
    pub fn new(cache: PackageCache) -> Self {
        Self { cache }
    }

    /// Perform maintenance on the cache
    pub fn maintain(&self) -> PackageResult<MaintenanceReport> {
        let stats_before = self.cache.stats();

        // Remove orphaned files
        let orphaned_count = self.remove_orphaned_files()?;

        // Verify package integrity
        let corrupted_count = self.verify_all_packages()?;

        // Update index if needed
        self.rebuild_index_if_needed()?;

        let stats_after = self.cache.stats();

        Ok(MaintenanceReport {
            packages_before: stats_before.total_packages,
            packages_after: stats_after.total_packages,
            size_before: stats_before.total_size,
            size_after: stats_after.total_size,
            orphaned_files_removed: orphaned_count,
            corrupted_packages_removed: corrupted_count,
        })
    }

    fn remove_orphaned_files(&self) -> PackageResult<usize> {
        let mut removed_count = 0;

        // Get all files in cache directory
        for entry in fs::read_dir(&self.cache.cache_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("gz") {
                let filename = path.file_name().unwrap().to_string_lossy();

                // Check if this file is referenced in the index
                let is_referenced = self
                    .cache
                    .index
                    .entries
                    .values()
                    .any(|e| e.path == filename);

                if !is_referenced && filename != "index.json" {
                    fs::remove_file(&path)?;
                    removed_count += 1;
                }
            }
        }

        Ok(removed_count)
    }

    fn verify_all_packages(&self) -> PackageResult<usize> {
        let mut corrupted_count = 0;
        let entries: Vec<_> = self.cache.index.entries.clone().into_iter().collect();

        for (key, entry) in entries {
            let package_path = self.cache.cache_dir.join(&entry.path);

            if package_path.exists() {
                match compute_file_hash(&package_path) {
                    Ok(computed_hash) => {
                        if computed_hash != entry.checksum {
                            // Package is corrupted, remove it
                            fs::remove_file(&package_path)?;
                            self.cache.remove_from_index(&key)?;
                            corrupted_count += 1;
                        }
                    }
                    Err(_) => {
                        // Can't compute hash, assume corrupted
                        fs::remove_file(&package_path)?;
                        self.cache.remove_from_index(&key)?;
                        corrupted_count += 1;
                    }
                }
            } else {
                // File is missing, remove from index
                self.cache.remove_from_index(&key)?;
                corrupted_count += 1;
            }
        }

        Ok(corrupted_count)
    }

    fn rebuild_index_if_needed(&self) -> PackageResult<()> {
        // This would check if the index needs rebuilding and do so if necessary
        // For now, we'll just ensure it's saved
        self.cache.save_index(&self.cache.index)?;
        Ok(())
    }
}

/// Maintenance operation report
#[derive(Debug, Clone)]
pub struct MaintenanceReport {
    pub packages_before: usize,
    pub packages_after: usize,
    pub size_before: u64,
    pub size_after: u64,
    pub orphaned_files_removed: usize,
    pub corrupted_packages_removed: usize,
}

impl MaintenanceReport {
    pub fn size_saved_mb(&self) -> f64 {
        (self.size_before.saturating_sub(self.size_after)) as f64 / (1024.0 * 1024.0)
    }
}

// Helper functions

fn package_key(name: &str, version: &Version) -> String {
    format!("{}-{}", name, version)
}

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

fn compute_file_hash(path: &Path) -> PackageResult<String> {
    let content = fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&content);
    Ok(format!("{:x}", hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_cache_creation() {
        let temp_dir = TempDir::new().unwrap();
        let config = CacheConfig {
            cache_dir: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        let mut cache = PackageCache::with_config(config).unwrap();
        assert!(temp_dir.path().exists());
        // Index file is created lazily, not in constructor
    }

    #[test]
    fn test_package_storage_and_retrieval() {
        let temp_dir = TempDir::new().unwrap();
        let config = CacheConfig {
            cache_dir: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        let mut cache = PackageCache::with_config(config).unwrap();
        let version = Version::new(1, 0, 0);
        let data = b"test package data".to_vec();

        // Store package
        cache
            .store_package("test-pkg", &version, data.clone())
            .unwrap();

        // Check if package exists
        assert!(cache.has_package("test-pkg", &version).unwrap());

        // Retrieve package
        let retrieved_data = cache.get_package("test-pkg", &version).unwrap();
        assert_eq!(retrieved_data, data);
    }

    #[test]
    fn test_package_removal() {
        let temp_dir = TempDir::new().unwrap();
        let config = CacheConfig {
            cache_dir: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        let mut cache = PackageCache::with_config(config).unwrap();
        let version = Version::new(1, 0, 0);
        let data = b"test package data".to_vec();

        // Store and then remove package
        cache.store_package("test-pkg", &version, data).unwrap();
        assert!(cache.has_package("test-pkg", &version).unwrap());

        cache.remove_package("test-pkg", &version).unwrap();
        assert!(!cache.has_package("test-pkg", &version).unwrap());
    }

    #[test]
    fn test_cache_stats() {
        let temp_dir = TempDir::new().unwrap();
        let config = CacheConfig {
            cache_dir: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        let mut cache = PackageCache::with_config(config).unwrap();
        let version = Version::new(1, 0, 0);
        let data = b"test package data".to_vec();

        let stats_before = cache.stats();
        assert_eq!(stats_before.total_packages, 0);
        assert_eq!(stats_before.total_size, 0);

        cache
            .store_package("test-pkg", &version, data.clone())
            .unwrap();

        let stats_after = cache.stats();
        assert_eq!(stats_after.total_packages, 1);
        assert_eq!(stats_after.total_size, data.len() as u64);
    }

    #[test]
    fn test_cache_cleanup() {
        let temp_dir = TempDir::new().unwrap();
        let config = CacheConfig {
            cache_dir: temp_dir.path().to_path_buf(),
            max_size_bytes: 50, // Very small cache
            ..Default::default()
        };

        let mut cache = PackageCache::with_config(config).unwrap();

        // Store packages that exceed cache size
        let data1 = b"package data 1 with some content".to_vec();
        let data2 = b"package data 2 with some content".to_vec();

        cache
            .store_package("pkg1", &Version::new(1, 0, 0), data1)
            .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10)); // Ensure different timestamps
        cache
            .store_package("pkg2", &Version::new(1, 0, 0), data2)
            .unwrap();

        // First package should be evicted due to size limit
        assert!(!cache.has_package("pkg1", &Version::new(1, 0, 0)).unwrap());
        assert!(cache.has_package("pkg2", &Version::new(1, 0, 0)).unwrap());
    }

    #[test]
    fn test_cache_manager_maintenance() {
        let temp_dir = TempDir::new().unwrap();
        let config = CacheConfig {
            cache_dir: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        let mut cache = PackageCache::with_config(config).unwrap();
        let manager = CacheManager::new(cache);

        let report = manager.maintain().unwrap();
        assert_eq!(report.packages_before, report.packages_after);
        assert_eq!(report.orphaned_files_removed, 0);
        assert_eq!(report.corrupted_packages_removed, 0);
    }

    #[test]
    fn test_cache_entry() {
        let version = Version::new(1, 0, 0);
        let entry = CacheEntry::new(
            "test-pkg",
            version.clone(),
            "test-pkg-1.0.0.tar.gz",
            "abc123",
            1024,
        );

        assert_eq!(entry.name, "test-pkg");
        assert_eq!(entry.version, version);
        assert_eq!(entry.size, 1024);
        assert!(entry.age_seconds() < 2); // Should be very recent
    }
}
