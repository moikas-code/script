use super::http_client::HttpClient;
/// Package registry client and API for Script language packages
///
/// This module provides:
/// - Registry API client for package operations
/// - Package publishing and downloading
/// - Registry metadata and search functionality
/// - Authentication and authorization handling
use super::{PackageError, PackageMetadata, PackageResult, Version};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Package registry trait for different registry implementations
pub trait PackageRegistry: Send + Sync {
    /// Search for packages matching a query
    fn search(&self, query: &str, limit: Option<usize>) -> PackageResult<Vec<PackageSearchResult>>;

    /// Get package information
    fn get_package_info(&self, name: &str) -> PackageResult<PackageInfo>;

    /// Get package metadata for a specific version
    fn get_package_metadata(&self, name: &str, version: &str) -> PackageResult<PackageMetadata>;

    /// Get all available versions for a package
    fn get_package_versions(&self, name: &str) -> PackageResult<Vec<Version>>;

    /// Download a package archive
    fn download_package(&self, name: &str, version: &str) -> PackageResult<Vec<u8>>;

    /// Publish a package to the registry
    fn publish_package(
        &self,
        package: &PublishablePackage,
        auth_token: &str,
    ) -> PackageResult<PublishResult>;

    /// Check if a package version exists
    fn package_exists(&self, name: &str, version: &str) -> PackageResult<bool>;

    /// Get download statistics for a package
    fn get_download_stats(&self, name: &str) -> PackageResult<DownloadStats>;
}

/// Registry client implementation
pub struct RegistryClient {
    base_url: String,
    client: HttpClient,
    auth_token: Option<String>,
}

impl RegistryClient {
    /// Create a new registry client
    pub fn new(base_url: impl Into<String>) -> PackageResult<Self> {
        Ok(Self {
            base_url: base_url.into(),
            client: HttpClient::new()?,
            auth_token: None,
        })
    }

    /// Create a registry client with authentication
    pub fn with_auth(
        base_url: impl Into<String>,
        auth_token: impl Into<String>,
    ) -> PackageResult<Self> {
        Ok(Self {
            base_url: base_url.into(),
            client: HttpClient::new()?,
            auth_token: Some(auth_token.into()),
        })
    }

    /// Set authentication token
    pub fn set_auth_token(&mut self, token: impl Into<String>) {
        self.auth_token = Some(token.into());
    }

    /// Remove authentication token
    pub fn clear_auth_token(&mut self) {
        self.auth_token = None;
    }

    fn build_url(&self, path: &str) -> String {
        if self.base_url.ends_with('/') {
            format!("{}{}", self.base_url, path.trim_start_matches('/'))
        } else {
            format!("{}/{}", self.base_url, path.trim_start_matches('/'))
        }
    }

    fn make_request(&self, method: &str, url: &str, body: Option<&[u8]>) -> PackageResult<Vec<u8>> {
        match method {
            "GET" => {
                if let Some(ref token) = self.auth_token {
                    let mut headers = HashMap::new();
                    headers.insert("Authorization".to_string(), format!("Bearer {}", token));
                    self.client.get_with_headers(url, headers)
                } else {
                    self.client.get(url)
                }
            }
            "POST" => {
                let body_vec = body.map(|b| b.to_vec()).unwrap_or_default();
                if let Some(ref token) = self.auth_token {
                    self.client.post_with_auth(url, body_vec, token)
                } else {
                    self.client.post(url, body_vec)
                }
            }
            "HEAD" => self
                .client
                .head(url)
                .map(|exists| if exists { Vec::new() } else { vec![] }),
            _ => Err(PackageError::Registry(format!(
                "Unsupported HTTP method: {}",
                method
            ))),
        }
    }
}

impl PackageRegistry for RegistryClient {
    fn search(&self, query: &str, limit: Option<usize>) -> PackageResult<Vec<PackageSearchResult>> {
        let mut url = self.build_url(&format!(
            "/api/v1/packages/search?q={}",
            urlencoding::encode(query)
        ));

        if let Some(limit) = limit {
            url.push_str(&format!("&limit={}", limit));
        }

        let response_data = self.make_request("GET", &url, None)?;
        let response: SearchResponse = serde_json::from_slice(&response_data).map_err(|e| {
            PackageError::Registry(format!("Failed to parse search response: {}", e))
        })?;

        Ok(response.packages)
    }

    fn get_package_info(&self, name: &str) -> PackageResult<PackageInfo> {
        let url = self.build_url(&format!("/api/v1/packages/{}", name));
        let response_data = self.make_request("GET", &url, None)?;

        let package_info: PackageInfo = serde_json::from_slice(&response_data)
            .map_err(|e| PackageError::Registry(format!("Failed to parse package info: {}", e)))?;

        Ok(package_info)
    }

    fn get_package_metadata(&self, name: &str, version: &str) -> PackageResult<PackageMetadata> {
        let url = self.build_url(&format!("/api/v1/packages/{}/{}/metadata", name, version));
        let response_data = self.make_request("GET", &url, None)?;

        let metadata: PackageMetadata = serde_json::from_slice(&response_data)
            .map_err(|e| PackageError::Registry(format!("Failed to parse metadata: {}", e)))?;

        Ok(metadata)
    }

    fn get_package_versions(&self, name: &str) -> PackageResult<Vec<Version>> {
        let url = self.build_url(&format!("/api/v1/packages/{}/versions", name));
        let response_data = self.make_request("GET", &url, None)?;

        let response: VersionsResponse = serde_json::from_slice(&response_data)
            .map_err(|e| PackageError::Registry(format!("Failed to parse versions: {}", e)))?;

        let versions: Result<Vec<_>, _> = response
            .versions
            .iter()
            .map(|v| Version::parse(v))
            .collect();

        versions.map_err(|e| PackageError::Registry(format!("Invalid version format: {}", e)))
    }

    fn download_package(&self, name: &str, version: &str) -> PackageResult<Vec<u8>> {
        let url = self.build_url(&format!("/api/v1/packages/{}/{}/download", name, version));
        self.make_request("GET", &url, None)
    }

    fn publish_package(
        &self,
        package: &PublishablePackage,
        auth_token: &str,
    ) -> PackageResult<PublishResult> {
        let url = self.build_url("/api/v1/packages/publish");

        let publish_request = PublishRequest {
            metadata: package.metadata.clone(),
            package_data: package.archive_data.clone(),
            signature: package.signature.clone(),
        };

        let request_body = serde_json::to_vec(&publish_request).map_err(|e| {
            PackageError::Registry(format!("Failed to serialize publish request: {}", e))
        })?;

        // Temporarily set auth token for this request
        let old_token = self.auth_token.clone();
        let mut client = RegistryClient {
            base_url: self.base_url.clone(),
            client: HttpClient::new()?,
            auth_token: Some(auth_token.to_string()),
        };

        let response_data = client.make_request("POST", &url, Some(&request_body))?;

        let result: PublishResult = serde_json::from_slice(&response_data).map_err(|e| {
            PackageError::Registry(format!("Failed to parse publish result: {}", e))
        })?;

        Ok(result)
    }

    fn package_exists(&self, name: &str, version: &str) -> PackageResult<bool> {
        let url = self.build_url(&format!("/api/v1/packages/{}/{}/exists", name, version));

        match self.make_request("HEAD", &url, None) {
            Ok(_) => Ok(true),
            Err(PackageError::Registry(_)) => Ok(false),
            Err(e) => Err(e),
        }
    }

    fn get_download_stats(&self, name: &str) -> PackageResult<DownloadStats> {
        let url = self.build_url(&format!("/api/v1/packages/{}/stats", name));
        let response_data = self.make_request("GET", &url, None)?;

        let stats: DownloadStats = serde_json::from_slice(&response_data)
            .map_err(|e| PackageError::Registry(format!("Failed to parse stats: {}", e)))?;

        Ok(stats)
    }
}

/// Package information from registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageInfo {
    pub name: String,
    pub description: Option<String>,
    pub latest_version: String,
    pub versions: Vec<String>,
    pub authors: Vec<String>,
    pub license: Option<String>,
    pub repository: Option<String>,
    pub homepage: Option<String>,
    pub documentation: Option<String>,
    pub keywords: Vec<String>,
    pub categories: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
    pub download_count: u64,
}

/// Search result for package queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageSearchResult {
    pub name: String,
    pub description: Option<String>,
    pub latest_version: String,
    pub authors: Vec<String>,
    pub keywords: Vec<String>,
    pub download_count: u64,
    pub updated_at: String,
    pub relevance_score: f64,
}

/// Package download statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadStats {
    pub total_downloads: u64,
    pub downloads_last_day: u64,
    pub downloads_last_week: u64,
    pub downloads_last_month: u64,
    pub version_downloads: HashMap<String, u64>,
}

/// Package ready for publishing
#[derive(Debug, Clone)]
pub struct PublishablePackage {
    pub metadata: PackageMetadata,
    pub archive_data: Vec<u8>,
    pub signature: Option<String>,
}

impl PublishablePackage {
    pub fn new(metadata: PackageMetadata, archive_data: Vec<u8>) -> Self {
        Self {
            metadata,
            archive_data,
            signature: None,
        }
    }

    pub fn with_signature(mut self, signature: impl Into<String>) -> Self {
        self.signature = Some(signature.into());
        self
    }
}

/// Result of a publish operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishResult {
    pub success: bool,
    pub message: String,
    pub package_url: Option<String>,
    pub version: String,
    pub warnings: Vec<String>,
}

// Internal API types

#[derive(Serialize, Deserialize)]
struct SearchResponse {
    packages: Vec<PackageSearchResult>,
    total_count: usize,
}

#[derive(Serialize, Deserialize)]
struct VersionsResponse {
    versions: Vec<String>,
}

#[derive(Serialize, Deserialize)]
struct PublishRequest {
    metadata: PackageMetadata,
    package_data: Vec<u8>,
    signature: Option<String>,
}

// Helper to avoid adding urlencoding dependency for this example
mod urlencoding {
    pub fn encode(input: &str) -> String {
        // Simple URL encoding - in a real implementation, use a proper library
        input
            .replace(' ', "%20")
            .replace('&', "%26")
            .replace('#', "%23")
    }
}

/// Registry configuration for managing multiple registries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryConfig {
    pub name: String,
    pub url: String,
    pub auth_token: Option<String>,
    pub trusted: bool,
    pub timeout_seconds: u64,
}

impl RegistryConfig {
    pub fn new(name: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            url: url.into(),
            auth_token: None,
            trusted: false,
            timeout_seconds: 30,
        }
    }

    pub fn with_auth(mut self, token: impl Into<String>) -> Self {
        self.auth_token = Some(token.into());
        self
    }

    pub fn trusted(mut self) -> Self {
        self.trusted = true;
        self
    }
}

/// Registry manager for handling multiple registries
pub struct RegistryManager {
    registries: HashMap<String, Box<dyn PackageRegistry>>,
    default_registry: String,
}

impl RegistryManager {
    pub fn new() -> Self {
        Self {
            registries: HashMap::new(),
            default_registry: "default".to_string(),
        }
    }

    /// Add a registry
    pub fn add_registry(&mut self, name: impl Into<String>, registry: Box<dyn PackageRegistry>) {
        let name = name.into();
        if self.registries.is_empty() {
            self.default_registry = name.clone();
        }
        self.registries.insert(name, registry);
    }

    /// Set the default registry
    pub fn set_default_registry(&mut self, name: impl Into<String>) -> PackageResult<()> {
        let name = name.into();
        if !self.registries.contains_key(&name) {
            return Err(PackageError::Registry(format!(
                "Registry '{}' not found",
                name
            )));
        }
        self.default_registry = name;
        Ok(())
    }

    /// Get a registry by name
    pub fn get_registry(&self, name: &str) -> Option<&dyn PackageRegistry> {
        self.registries.get(name).map(|r| r.as_ref())
    }

    /// Get the default registry
    pub fn default_registry(&self) -> Option<&dyn PackageRegistry> {
        self.get_registry(&self.default_registry)
    }

    /// Search across all registries
    pub fn search_all(
        &self,
        query: &str,
        limit: Option<usize>,
    ) -> PackageResult<Vec<PackageSearchResult>> {
        let mut all_results = Vec::new();

        for (_, registry) in &self.registries {
            match registry.search(query, limit) {
                Ok(mut results) => all_results.append(&mut results),
                Err(_) => continue, // Skip registries that fail
            }
        }

        // Sort by relevance score
        all_results.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap());

        // Apply limit if specified
        if let Some(limit) = limit {
            all_results.truncate(limit);
        }

        Ok(all_results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_client_creation() {
        let client = RegistryClient::new("https://packages.script.org").unwrap();
        assert!(client.auth_token.is_none());

        let auth_client =
            RegistryClient::with_auth("https://packages.script.org", "test-token").unwrap();
        assert!(auth_client.auth_token.is_some());
    }

    #[test]
    fn test_url_building() {
        let client = RegistryClient::new("https://registry.example.com").unwrap();
        assert_eq!(
            client.build_url("/api/v1/packages"),
            "https://registry.example.com/api/v1/packages"
        );

        let client_with_slash = RegistryClient::new("https://registry.example.com/").unwrap();
        assert_eq!(
            client_with_slash.build_url("/api/v1/packages"),
            "https://registry.example.com/api/v1/packages"
        );
    }

    // Integration tests would go here - commented out since they require a real registry
    // #[test]
    // fn test_package_search() {
    //     let client = RegistryClient::new("https://packages.script.org").unwrap();
    //     let results = client.search("test", Some(10)).unwrap();
    //     assert!(!results.is_empty());
    // }

    #[test]
    fn test_publishable_package() {
        let metadata = PackageMetadata::new("test-package", Version::new(1, 0, 0));
        let data = b"package data".to_vec();

        let package = PublishablePackage::new(metadata.clone(), data.clone());
        assert_eq!(package.metadata.name, "test-package");
        assert_eq!(package.archive_data, data);
        assert!(package.signature.is_none());

        let signed_package = package.with_signature("signature123");
        assert!(signed_package.signature.is_some());
    }

    #[test]
    fn test_registry_manager() {
        let mut manager = RegistryManager::new();

        let registry1 = Box::new(RegistryClient::new("https://registry1.com").unwrap());
        let registry2 = Box::new(RegistryClient::new("https://registry2.com").unwrap());

        manager.add_registry("registry1", registry1);
        manager.add_registry("registry2", registry2);

        assert!(manager.get_registry("registry1").is_some());
        assert!(manager.get_registry("registry2").is_some());
        assert!(manager.default_registry().is_some());

        manager.set_default_registry("registry2").unwrap();
        assert_eq!(manager.default_registry, "registry2");
    }
}
