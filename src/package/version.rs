/// Semantic versioning implementation for Script packages
///
/// This module provides comprehensive version handling including:
/// - Semantic version parsing and validation
/// - Version constraint matching and resolution
/// - Version comparison and ordering
use super::{PackageError, PackageResult};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Semantic version representation
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Version {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
    pub pre: Vec<Prerelease>,
    pub build: Vec<BuildMetadata>,
}

impl Version {
    /// Create a new version
    pub fn new(major: u64, minor: u64, patch: u64) -> Self {
        Self {
            major,
            minor,
            patch,
            pre: Vec::new(),
            build: Vec::new(),
        }
    }

    /// Create a version with prerelease information
    pub fn with_prerelease(major: u64, minor: u64, patch: u64, pre: Vec<Prerelease>) -> Self {
        Self {
            major,
            minor,
            patch,
            pre,
            build: Vec::new(),
        }
    }

    /// Parse version from string
    pub fn parse(input: &str) -> Result<Self, semver::Error> {
        let semver_version = semver::Version::parse(input)?;
        Ok(Self::from_semver(&semver_version))
    }

    /// Convert from semver::Version
    pub fn from_semver(version: &semver::Version) -> Self {
        let pre = if version.pre.is_empty() {
            Vec::new()
        } else {
            vec![Prerelease::from_str(&version.pre.as_str())]
        };

        let build = if version.build.is_empty() {
            Vec::new()
        } else {
            vec![BuildMetadata::from_str(&version.build.as_str())]
        };

        Self {
            major: version.major,
            minor: version.minor,
            patch: version.patch,
            pre,
            build,
        }
    }

    /// Convert to semver::Version
    pub fn to_semver(&self) -> semver::Version {
        let mut version = semver::Version::new(self.major, self.minor, self.patch);

        if !self.pre.is_empty() {
            let pre_str = self
                .pre
                .iter()
                .map(|p| p.to_string())
                .collect::<Vec<_>>()
                .join(".");
            version.pre = semver::Prerelease::new(&pre_str).unwrap_or_default();
        }

        if !self.build.is_empty() {
            let build_str = self
                .build
                .iter()
                .map(|b| b.to_string())
                .collect::<Vec<_>>()
                .join(".");
            version.build = semver::BuildMetadata::new(&build_str).unwrap_or_default();
        }

        version
    }

    /// Check if this is a prerelease version
    pub fn is_prerelease(&self) -> bool {
        !self.pre.is_empty()
    }

    /// Check if this version is compatible with another (same major version)
    pub fn is_compatible_with(&self, other: &Version) -> bool {
        self.major == other.major && self >= other
    }

    /// Get the next major version
    pub fn next_major(&self) -> Version {
        Version::new(self.major + 1, 0, 0)
    }

    /// Get the next minor version
    pub fn next_minor(&self) -> Version {
        Version::new(self.major, self.minor + 1, 0)
    }

    /// Get the next patch version
    pub fn next_patch(&self) -> Version {
        Version::new(self.major, self.minor, self.patch + 1)
    }
}

impl Default for Version {
    fn default() -> Self {
        Self::new(0, 1, 0)
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;

        if !self.pre.is_empty() {
            write!(f, "-")?;
            for (i, pre) in self.pre.iter().enumerate() {
                if i > 0 {
                    write!(f, ".")?;
                }
                write!(f, "{}", pre)?;
            }
        }

        if !self.build.is_empty() {
            write!(f, "+")?;
            for (i, build) in self.build.iter().enumerate() {
                if i > 0 {
                    write!(f, ".")?;
                }
                write!(f, "{}", build)?;
            }
        }

        Ok(())
    }
}

impl FromStr for Version {
    type Err = semver::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

/// Prerelease version identifier
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Prerelease {
    Alpha(u64),
    Beta(u64),
    Rc(u64),
    Custom(String),
}

impl Prerelease {
    fn from_str(s: &str) -> Self {
        if s.starts_with("alpha") {
            if let Some(num_str) = s.strip_prefix("alpha.") {
                if let Ok(num) = num_str.parse() {
                    return Self::Alpha(num);
                }
            } else if s == "alpha" {
                return Self::Alpha(0);
            }
        } else if s.starts_with("beta") {
            if let Some(num_str) = s.strip_prefix("beta.") {
                if let Ok(num) = num_str.parse() {
                    return Self::Beta(num);
                }
            } else if s == "beta" {
                return Self::Beta(0);
            }
        } else if s.starts_with("rc") {
            if let Some(num_str) = s.strip_prefix("rc.") {
                if let Ok(num) = num_str.parse() {
                    return Self::Rc(num);
                }
            } else if s == "rc" {
                return Self::Rc(0);
            }
        }
        Self::Custom(s.to_string())
    }

    fn from_semver_prerelease(pre: &semver::Prerelease) -> Self {
        let s = pre.as_str();
        if s.starts_with("alpha") {
            if let Some(num_str) = s.strip_prefix("alpha.") {
                if let Ok(num) = num_str.parse() {
                    return Self::Alpha(num);
                }
            } else if s == "alpha" {
                return Self::Alpha(0);
            }
        } else if s.starts_with("beta") {
            if let Some(num_str) = s.strip_prefix("beta.") {
                if let Ok(num) = num_str.parse() {
                    return Self::Beta(num);
                }
            } else if s == "beta" {
                return Self::Beta(0);
            }
        } else if s.starts_with("rc") {
            if let Some(num_str) = s.strip_prefix("rc.") {
                if let Ok(num) = num_str.parse() {
                    return Self::Rc(num);
                }
            } else if s == "rc" {
                return Self::Rc(0);
            }
        }
        Self::Custom(s.to_string())
    }

    fn to_semver_prerelease(&self) -> semver::Prerelease {
        let s = match self {
            Self::Alpha(0) => "alpha".to_string(),
            Self::Alpha(n) => format!("alpha.{}", n),
            Self::Beta(0) => "beta".to_string(),
            Self::Beta(n) => format!("beta.{}", n),
            Self::Rc(0) => "rc".to_string(),
            Self::Rc(n) => format!("rc.{}", n),
            Self::Custom(s) => s.clone(),
        };
        semver::Prerelease::new(&s).unwrap()
    }
}

impl fmt::Display for Prerelease {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Alpha(0) => write!(f, "alpha"),
            Self::Alpha(n) => write!(f, "alpha.{}", n),
            Self::Beta(0) => write!(f, "beta"),
            Self::Beta(n) => write!(f, "beta.{}", n),
            Self::Rc(0) => write!(f, "rc"),
            Self::Rc(n) => write!(f, "rc.{}", n),
            Self::Custom(s) => write!(f, "{}", s),
        }
    }
}

/// Build metadata
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct BuildMetadata(String);

impl BuildMetadata {
    fn from_str(s: &str) -> Self {
        Self(s.to_string())
    }

    pub fn new(metadata: impl Into<String>) -> Self {
        Self(metadata.into())
    }

    fn from_semver_build(build: &semver::BuildMetadata) -> Self {
        Self(build.as_str().to_string())
    }

    fn to_semver_build(&self) -> semver::BuildMetadata {
        semver::BuildMetadata::new(&self.0).unwrap()
    }
}

impl fmt::Display for BuildMetadata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Version constraint for dependency specification
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VersionConstraint {
    /// Exact version match (=1.2.3)
    Exact(Version),

    /// Greater than or equal (>=1.2.3)
    GreaterEqual(Version),

    /// Less than (<2.0.0)
    LessThan(Version),

    /// Less than or equal (<=1.9.9)
    LessEqual(Version),

    /// Greater than (>1.2.3)
    GreaterThan(Version),

    /// Compatible version (^1.2.3 means >=1.2.3, <2.0.0)
    Compatible(Version),

    /// Tilde constraint (~1.2.3 means >=1.2.3, <1.3.0)
    Tilde(Version),

    /// Wildcard (1.2.* means >=1.2.0, <1.3.0)
    Wildcard(u64, Option<u64>),

    /// Range (>=1.2.3, <2.0.0)
    Range(Version, Version),

    /// Multiple constraints (AND)
    Multiple(Vec<VersionConstraint>),
}

impl VersionConstraint {
    /// Check if a version satisfies this constraint
    pub fn matches(&self, version: &Version) -> bool {
        match self {
            Self::Exact(v) => version == v,
            Self::GreaterEqual(v) => version >= v,
            Self::LessThan(v) => version < v,
            Self::LessEqual(v) => version <= v,
            Self::GreaterThan(v) => version > v,
            Self::Compatible(v) => version >= v && version.major == v.major,
            Self::Tilde(v) => version >= v && version.major == v.major && version.minor == v.minor,
            Self::Wildcard(major, minor) => {
                if let Some(minor) = minor {
                    version.major == *major && version.minor == *minor
                } else {
                    version.major == *major
                }
            }
            Self::Range(min, max) => version >= min && version < max,
            Self::Multiple(constraints) => constraints.iter().all(|c| c.matches(version)),
        }
    }

    /// Parse constraint from string
    pub fn parse(input: &str) -> PackageResult<Self> {
        let input = input.trim();

        if input.is_empty() {
            return Err(PackageError::ManifestParse(
                "Empty version constraint".to_string(),
            ));
        }

        // Handle multiple constraints separated by commas
        if input.contains(',') {
            let constraints: Result<Vec<_>, _> =
                input.split(',').map(|s| Self::parse(s.trim())).collect();
            return Ok(Self::Multiple(constraints?));
        }

        // Parse single constraint
        if let Some(version_str) = input.strip_prefix(">=") {
            let version = Version::parse(version_str.trim())?;
            Ok(Self::GreaterEqual(version))
        } else if let Some(version_str) = input.strip_prefix("<=") {
            let version = Version::parse(version_str.trim())?;
            Ok(Self::LessEqual(version))
        } else if let Some(version_str) = input.strip_prefix('>') {
            let version = Version::parse(version_str.trim())?;
            Ok(Self::GreaterThan(version))
        } else if let Some(version_str) = input.strip_prefix('<') {
            let version = Version::parse(version_str.trim())?;
            Ok(Self::LessThan(version))
        } else if let Some(version_str) = input.strip_prefix('=') {
            let version = Version::parse(version_str.trim())?;
            Ok(Self::Exact(version))
        } else if let Some(version_str) = input.strip_prefix('^') {
            let version = Version::parse(version_str.trim())?;
            Ok(Self::Compatible(version))
        } else if let Some(version_str) = input.strip_prefix('~') {
            let version = Version::parse(version_str.trim())?;
            Ok(Self::Tilde(version))
        } else if input.contains('*') {
            Self::parse_wildcard(input)
        } else {
            // Default to compatible version
            let version = Version::parse(input)?;
            Ok(Self::Compatible(version))
        }
    }

    fn parse_wildcard(input: &str) -> PackageResult<Self> {
        let parts: Vec<&str> = input.split('.').collect();

        match parts.len() {
            2 if parts[1] == "*" => {
                let major = parts[0].parse().map_err(|_| {
                    PackageError::ManifestParse("Invalid major version in wildcard".to_string())
                })?;
                Ok(Self::Wildcard(major, None))
            }
            3 if parts[2] == "*" => {
                let major = parts[0].parse().map_err(|_| {
                    PackageError::ManifestParse("Invalid major version in wildcard".to_string())
                })?;
                let minor = parts[1].parse().map_err(|_| {
                    PackageError::ManifestParse("Invalid minor version in wildcard".to_string())
                })?;
                Ok(Self::Wildcard(major, Some(minor)))
            }
            _ => Err(PackageError::ManifestParse(
                "Invalid wildcard version format".to_string(),
            )),
        }
    }

    /// Get the highest version that satisfies this constraint from a list
    pub fn highest_matching<'a>(&self, versions: &'a [Version]) -> Option<&'a Version> {
        versions.iter().filter(|v| self.matches(v)).max()
    }

    /// Check if this constraint is compatible with another
    pub fn is_compatible_with(&self, other: &VersionConstraint) -> bool {
        // Simplified compatibility check
        // In a full implementation, this would need more sophisticated logic
        match (self, other) {
            (Self::Compatible(v1), Self::Compatible(v2)) => v1.major == v2.major,
            (Self::Exact(v1), Self::Exact(v2)) => v1 == v2,
            _ => true, // Conservative approach
        }
    }
}

impl fmt::Display for VersionConstraint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Exact(v) => write!(f, "={}", v),
            Self::GreaterEqual(v) => write!(f, ">={}", v),
            Self::LessThan(v) => write!(f, "<{}", v),
            Self::LessEqual(v) => write!(f, "<={}", v),
            Self::GreaterThan(v) => write!(f, ">{}", v),
            Self::Compatible(v) => write!(f, "^{}", v),
            Self::Tilde(v) => write!(f, "~{}", v),
            Self::Wildcard(major, None) => write!(f, "{}.*", major),
            Self::Wildcard(major, Some(minor)) => write!(f, "{}.{}.*", major, minor),
            Self::Range(min, max) => write!(f, ">={}, <{}", min, max),
            Self::Multiple(constraints) => {
                for (i, constraint) in constraints.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", constraint)?;
                }
                Ok(())
            }
        }
    }
}

/// Version specification for dependencies
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum VersionSpec {
    /// Simple version string
    Simple(String),

    /// Complex version specification with additional options
    Complex {
        version: String,

        #[serde(default)]
        features: Vec<String>,

        #[serde(default)]
        optional: bool,

        #[serde(default)]
        default_features: bool,
    },
}

impl VersionSpec {
    /// Get the version constraint from this spec
    pub fn constraint(&self) -> PackageResult<VersionConstraint> {
        let version_str = match self {
            Self::Simple(v) => v,
            Self::Complex { version, .. } => version,
        };
        VersionConstraint::parse(version_str)
    }

    /// Get features specified in this version spec
    pub fn features(&self) -> &[String] {
        match self {
            Self::Simple(_) => &[],
            Self::Complex { features, .. } => features,
        }
    }

    /// Check if this dependency is optional
    pub fn is_optional(&self) -> bool {
        match self {
            Self::Simple(_) => false,
            Self::Complex { optional, .. } => *optional,
        }
    }

    /// Check if default features should be enabled
    pub fn default_features(&self) -> bool {
        match self {
            Self::Simple(_) => true,
            Self::Complex {
                default_features, ..
            } => *default_features,
        }
    }
}

impl From<&str> for VersionSpec {
    fn from(version: &str) -> Self {
        Self::Simple(version.to_string())
    }
}

impl From<String> for VersionSpec {
    fn from(version: String) -> Self {
        Self::Simple(version)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_creation() {
        let version = Version::new(1, 2, 3);
        assert_eq!(version.major, 1);
        assert_eq!(version.minor, 2);
        assert_eq!(version.patch, 3);
        assert!(!version.is_prerelease());
    }

    #[test]
    fn test_version_parsing() {
        let version = Version::parse("1.2.3").unwrap();
        assert_eq!(version, Version::new(1, 2, 3));

        let prerelease = Version::parse("1.2.3-alpha.1").unwrap();
        assert!(prerelease.is_prerelease());
    }

    #[test]
    fn test_version_display() {
        let version = Version::new(1, 2, 3);
        assert_eq!(version.to_string(), "1.2.3");

        let prerelease = Version::with_prerelease(1, 2, 3, vec![Prerelease::Alpha(1)]);
        assert_eq!(prerelease.to_string(), "1.2.3-alpha.1");
    }

    #[test]
    fn test_version_compatibility() {
        let v1 = Version::new(1, 3, 0);
        let v2 = Version::new(1, 2, 3);
        let v3 = Version::new(2, 0, 0);

        assert!(v1.is_compatible_with(&v2)); // 1.3.0 is compatible with 1.2.3 (same major, v1 >= v2)
        assert!(!v1.is_compatible_with(&v3)); // Different major version
    }

    #[test]
    fn test_constraint_parsing() {
        let constraint = VersionConstraint::parse("^1.2.3").unwrap();
        assert!(matches!(constraint, VersionConstraint::Compatible(_)));

        let range = VersionConstraint::parse(">=1.2.3, <2.0.0").unwrap();
        assert!(matches!(range, VersionConstraint::Multiple(_)));
    }

    #[test]
    fn test_constraint_matching() {
        let constraint = VersionConstraint::Compatible(Version::new(1, 2, 3));
        let version1 = Version::new(1, 2, 3);
        let version2 = Version::new(1, 3, 0);
        let version3 = Version::new(2, 0, 0);

        assert!(constraint.matches(&version1));
        assert!(constraint.matches(&version2));
        assert!(!constraint.matches(&version3));
    }

    #[test]
    fn test_wildcard_constraint() {
        let constraint = VersionConstraint::parse("1.2.*").unwrap();
        let version1 = Version::new(1, 2, 0);
        let version2 = Version::new(1, 2, 5);
        let version3 = Version::new(1, 3, 0);

        assert!(constraint.matches(&version1));
        assert!(constraint.matches(&version2));
        assert!(!constraint.matches(&version3));
    }

    #[test]
    fn test_version_spec() {
        let simple = VersionSpec::Simple("1.2.3".to_string());
        assert_eq!(simple.features().len(), 0);
        assert!(!simple.is_optional());
        assert!(simple.default_features());

        let complex = VersionSpec::Complex {
            version: "1.2.3".to_string(),
            features: vec!["feature1".to_string()],
            optional: true,
            default_features: false,
        };
        assert_eq!(complex.features().len(), 1);
        assert!(complex.is_optional());
        assert!(!complex.default_features());
    }
}
