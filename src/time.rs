use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

/// A UTC timestamp used throughout Neo for time tracking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Timestamp(#[serde(with = "chrono::serde::ts_seconds")] pub DateTime<Utc>);

impl Timestamp {
    /// Create a timestamp representing the current moment.
    pub fn now() -> Self {
        Self(Utc::now())
    }

    /// Seconds elapsed since this timestamp.
    pub fn elapsed_secs(&self) -> f64 {
        let now = Utc::now();
        let duration = now.signed_duration_since(self.0);
        duration.num_milliseconds() as f64 / 1000.0
    }

    /// Returns true if more than `timeout_secs` have passed since this timestamp.
    pub fn is_expired(&self, timeout_secs: u64) -> bool {
        self.elapsed_secs() >= timeout_secs as f64
    }
}

impl Default for Timestamp {
    fn default() -> Self {
        Self::now()
    }
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.to_rfc3339())
    }
}

impl From<DateTime<Utc>> for Timestamp {
    fn from(dt: DateTime<Utc>) -> Self {
        Self(dt)
    }
}

impl From<Timestamp> for DateTime<Utc> {
    fn from(ts: Timestamp) -> DateTime<Utc> {
        ts.0
    }
}
