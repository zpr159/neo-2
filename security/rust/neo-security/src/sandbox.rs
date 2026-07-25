use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub type NeoResult<T> = Result<T, SandboxError>;

#[derive(Debug, Clone, thiserror::Error)]
pub enum SandboxError {
    #[error("sandbox violation: {0}")]
    Violation(String),
    #[error("sandbox not active")]
    NotActive,
    #[error("sandbox already active")]
    AlreadyActive,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxViolation {
    pub violation_type: String,
    pub message: String,
    pub severity: String,
    pub timestamp: DateTime<Utc>,
}

impl SandboxViolation {
    pub fn new(violation_type: &str, message: &str, severity: &str) -> Self {
        tracing::warn!(
            violation_type = violation_type,
            severity = severity,
            "sandbox violation detected"
        );
        Self {
            violation_type: violation_type.to_string(),
            message: message.to_string(),
            severity: severity.to_string(),
            timestamp: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxPolicy {
    pub max_memory_bytes: u64,
    pub max_cpu_ms: u64,
    pub allowed_syscalls: Vec<String>,
    pub blocked_paths: Vec<String>,
    pub network_allowed: bool,
    pub filesystem_allowed: bool,
}

impl Default for SandboxPolicy {
    fn default() -> Self {
        Self {
            max_memory_bytes: 512 * 1024 * 1024,
            max_cpu_ms: 30_000,
            allowed_syscalls: vec![
                "read".to_string(),
                "write".to_string(),
                "open".to_string(),
                "close".to_string(),
                "mmap".to_string(),
            ],
            blocked_paths: vec![
                "/etc".to_string(),
                "/proc".to_string(),
                "/sys".to_string(),
            ],
            network_allowed: false,
            filesystem_allowed: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SecuritySandbox {
    pub policy: SandboxPolicy,
    pub violations: Vec<SandboxViolation>,
    pub is_active: bool,
}

impl SecuritySandbox {
    pub fn new(policy: SandboxPolicy) -> Self {
        tracing::info!("security sandbox created with policy");
        Self {
            policy,
            violations: Vec::new(),
            is_active: false,
        }
    }

    pub fn enter(&mut self) -> NeoResult<()> {
        if self.is_active {
            return Err(SandboxError::AlreadyActive);
        }
        self.is_active = true;
        tracing::info!("sandbox entered");
        Ok(())
    }

    pub fn exit(&mut self) -> NeoResult<()> {
        if !self.is_active {
            return Err(SandboxError::NotActive);
        }
        self.is_active = false;
        tracing::info!(
            violations = self.violations.len(),
            "sandbox exited"
        );
        Ok(())
    }

    pub fn check_violation(&mut self, action: &str) -> bool {
        if !self.is_active {
            return false;
        }

        if action.starts_with("network:") && !self.policy.network_allowed {
            self.report_violation(SandboxViolation::new(
                "network_access",
                &format!("blocked network action: {}", action),
                "high",
            ));
            return false;
        }

        for path in &self.policy.blocked_paths {
            if action.contains(path) {
                self.report_violation(SandboxViolation::new(
                    "blocked_path",
                    &format!("access to blocked path: {}", action),
                    "medium",
                ));
                return false;
            }
        }

        if action.starts_with("syscall:") {
            let syscall = action.strip_prefix("syscall:").unwrap_or(action);
            if !self.policy.allowed_syscalls.iter().any(|s| s == syscall) {
                self.report_violation(SandboxViolation::new(
                    "syscall_violation",
                    &format!("blocked syscall: {}", syscall),
                    "high",
                ));
                return false;
            }
        }

        true
    }

    pub fn report_violation(&mut self, violation: SandboxViolation) {
        self.violations.push(violation);
    }

    pub fn violation_count(&self) -> usize {
        self.violations.len()
    }

    pub fn violations(&self) -> &[SandboxViolation] {
        &self.violations
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sandbox_lifecycle() {
        let policy = SandboxPolicy::default();
        let mut sandbox = SecuritySandbox::new(policy);

        assert!(!sandbox.is_active);
        sandbox.enter().unwrap();
        assert!(sandbox.is_active);
        sandbox.exit().unwrap();
        assert!(!sandbox.is_active);
    }

    #[test]
    fn test_violation_detection() {
        let policy = SandboxPolicy::default();
        let mut sandbox = SecuritySandbox::new(policy);
        sandbox.enter().unwrap();

        assert!(sandbox.check_violation("read"));
        assert!(sandbox.check_violation("write"));
        assert!(!sandbox.check_violation("network:connect"));
        assert!(!sandbox.check_violation("access /etc/passwd"));
        assert!(!sandbox.check_violation("syscall:ptrace"));

        assert_eq!(sandbox.violation_count(), 3);
        sandbox.exit().unwrap();
    }

    #[test]
    fn test_already_active_error() {
        let policy = SandboxPolicy::default();
        let mut sandbox = SecuritySandbox::new(policy);
        sandbox.enter().unwrap();
        assert!(sandbox.enter().is_err());
        sandbox.exit().unwrap();
    }
}
