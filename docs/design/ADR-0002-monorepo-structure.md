# ADR-0002: Monorepo Structure

## Status

Accepted

## Context

Neo AGI OS consists of 15+ subsystems across 7 languages. We must decide whether to host all code in a single repository (monorepo) or split across multiple repositories (polyrepo).

Key considerations:
- Cross-subsystem dependencies are frequent
- Protobuf schemas are shared across all languages
- Build and test coordination is required
- Team size and deployment cadence

## Decision

We adopt a monorepo structure with a single repository containing all subsystems, build scripts, documentation, and deployment configurations.

### Directory Layout

```
Neo_2.0/
  core/                    # Rust core crates
  neural-network-framework/ # ML framework (Rust/C++/Python)
  api-gateway/             # Go API server
  web-dashboard/           # TypeScript frontend
  sdk/                     # Client SDKs (Kotlin, Swift, TS, Python)
  ui/                      # Native UIs (Kotlin, Swift)
  robotics/                # Robotics control (Rust/C++/Kotlin)
  device-manager/          # Go device management
  observability/           # Monitoring (Rust/Python)
  proto/                   # Shared protobuf schemas
  scripts/                 # Build and utility scripts
  docs/                    # Documentation
  tests/                   # Integration tests
  deploy/                  # Deployment configs
```

## Consequences

### Positive

- Atomic commits across subsystems
- Single CI/CD pipeline
- Shared protobuf schemas with guaranteed consistency
- Simplified dependency management
- Easy cross-referencing and refactoring
- Single source of truth for documentation

### Negative

- Larger repository size
- Slower git operations for full clones
- Build times may increase
- Access control is coarser

### Mitigations

- Git sparse checkout for developers working on specific subsystems
- Build caching (sccache, ccache) reduces rebuild times
- CI runs only affected component tests on PRs
- Shallow clones for CI

## Alternatives Considered

### Option 1: Polyrepo

Each subsystem in its own repository with cross-repo dependencies.

Pros: Smaller repos, fine-grained access control, independent versioning.
Cons: Protobuf schema drift, complex cross-repo testing, harder refactoring, duplicated CI configuration, no atomic commits.
Rejected because: The tight coupling between subsystems makes polyrepo impractical.

### Option 2: Multi-Repo with Meta-Repo

Separate repos with a "meta" repo that orchestrates builds.

Pros: Combines benefits of mono and polyrepo.
Cons: Adds complexity of maintaining the meta repo, still has schema drift issues.
Rejected because: Adds complexity without sufficient benefit over monorepo.
