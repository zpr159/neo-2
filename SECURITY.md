# Security Policy

## Reporting a Vulnerability

If you discover a security vulnerability within Neo, please report it responsibly.

**Do NOT open a public GitHub issue for security vulnerabilities.**

### Report Process

1. **Email**: Send a report to `security@neo-agi.org`
2. **Subject**: `[SECURITY] Brief description of vulnerability`
3. **Include**:
   - Description of the vulnerability
   - Steps to reproduce
   - Potential impact assessment
   - Suggested fix (if any)

### What to Expect

- **Acknowledgment** within 48 hours
- **Initial assessment** within 1 week
- **Resolution timeline** communicated based on severity
- **Credit** in release notes (unless anonymity is requested)

## Security Measures

### Authentication & Authorization

- All inter-node communication uses mutual TLS (mTLS)
- Agent execution is sandboxed with capability-based permissions
- Role-based access control (RBAC) for all administrative operations
- API keys and tokens are rotated automatically

### Memory Protection

- Memory subsystems use Rust's ownership model for memory safety
- Process isolation for untrusted plugin execution
- Encrypted at-rest storage for sensitive memory entries
- Automatic memory entry expiration and garbage collection

### Network Security

- All external communication encrypted with TLS 1.3
- Internal cluster communication encrypted with mTLS
- Network policies enforced via Kubernetes NetworkPolicy
- DDoS protection at the ingress layer

### Supply Chain Security

- All dependencies audited via `cargo audit`, `npm audit`, `pip audit`
- Reproducible builds with pinned dependency versions
- Signed container images via Sigstore/Cosign
- SBOM (Software Bill of Materials) generated for every release

### Code Security

- Static analysis: `clippy` (Rust), `golangci-lint` (Go), `semgrep` (all)
- Fuzzing: `cargo-fuzz` for Rust parsing/deserialization
- `unsafe` code requires explicit approval and security review
- No hardcoded secrets — all secrets loaded from environment or secret stores

## Supported Versions

| Version | Supported |
|---|---|
| 0.1.x | Yes |
| < 0.1 | No |

## Security Updates

Security patches are released as soon as possible and are clearly marked in release notes and commit messages with the `[SECURITY]` prefix.
