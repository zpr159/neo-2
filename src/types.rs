use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;
use strum::{Display, EnumString};

/// Severity levels for logging and diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Display, EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum Severity {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    Fatal,
}

/// Deployment environment identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Display, EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum Environment {
    Development,
    Testing,
    Staging,
    Production,
}

/// Semantic version with pre-release and build metadata.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub pre_release: Option<String>,
    pub build: Option<String>,
}

impl Version {
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
            pre_release: None,
            build: None,
        }
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
        if let Some(ref pre) = self.pre_release {
            write!(f, "-{}", pre)?;
        }
        if let Some(ref build) = self.build {
            write!(f, "+{}", build)?;
        }
        Ok(())
    }
}

impl FromStr for Version {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let build_str;
        let version_and_pre = if let Some(idx) = s.find('+') {
            build_str = Some(s[idx + 1..].to_string());
            &s[..idx]
        } else {
            build_str = None;
            s
        };

        let pre_str;
        let version_part = if let Some(idx) = version_and_pre.find('-') {
            pre_str = Some(version_and_pre[idx + 1..].to_string());
            &version_and_pre[..idx]
        } else {
            pre_str = None;
            version_and_pre
        };

        let parts: Vec<&str> = version_part.split('.').collect();
        if parts.len() != 3 {
            return Err(format!(
                "invalid version format: expected MAJOR.MINOR.PATCH, got '{}'",
                s
            ));
        }

        let major = parts[0]
            .parse::<u32>()
            .map_err(|e| format!("invalid major version: {}", e))?;
        let minor = parts[1]
            .parse::<u32>()
            .map_err(|e| format!("invalid minor version: {}", e))?;
        let patch = parts[2]
            .parse::<u32>()
            .map_err(|e| format!("invalid patch version: {}", e))?;

        Ok(Self {
            major,
            minor,
            patch,
            pre_release: pre_str,
            build: build_str,
        })
    }
}

/// Generic metadata as key-value string pairs.
pub type GenericMetadata = HashMap<String, String>;
