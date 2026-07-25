# Contributing to Neo

Thank you for your interest in contributing to Neo. This document establishes the standards and processes for all contributions.

---

## Table of Contents

- [Development Setup](#development-setup)
- [Monorepo Structure](#monorepo-structure)
- [Branching Model](#branching-model)
- [Commit Conventions](#commit-conventions)
- [Pull Request Process](#pull-request-process)
- [Coding Standards](#coding-standards)
- [Testing Requirements](#testing-requirements)
- [Documentation Requirements](#documentation-requirements)
- [Security Requirements](#security-requirements)

---

## Development Setup

### Prerequisites

| Tool | Version | Purpose |
|---|---|---|
| Rust | >= 1.75 | Core, neural engine, memory, reasoning |
| Go | >= 1.22 | Distributed systems, services |
| Python | >= 3.11 | ML integration, SDK, plugins |
| Node.js | >= 20 LTS | UI, workflows, TypeScript SDK |
| CMake | >= 3.28 | C/C++ build system |
| CUDA Toolkit | >= 12.3 | GPU acceleration (optional) |
| Docker | >= 24.0 | Containerized builds |
| pnpm | >= 8.0 | Node.js package management |

### Bootstrap

```bash
# One-command setup (installs all toolchains and dependencies)
./scripts/bootstrap.sh

# Verify installation
./scripts/verify-toolchain.sh
```

### IDE Configuration

- **VS Code**: Open `neo.code-workspace` for multi-root workspace configuration
- **Rust Analyzer**: Configured via `rust-analyzer.toml` at workspace root
- **Clangd**: Configured via `.clangd` at workspace root

---

## Commit Conventions

Neo follows [Conventional Commits](https://www.conventionalcommits.org/) with scope prefixes matching the monorepo subsystems.

### Format

```
<type>(<scope>): <description>

[optional body]

[optional footer(s)]
```

### Types

| Type | Description |
|---|---|
| `feat` | New feature |
| `fix` | Bug fix |
| `docs` | Documentation only changes |
| `style` | Code style changes (formatting, no logic change) |
| `refactor` | Code change that neither fixes a bug nor adds a feature |
| `perf` | Performance improvement |
| `test` | Adding or correcting tests |
| `build` | Build system or external dependency changes |
| `ci` | CI configuration changes |
| `chore` | Other changes that don't modify src or test |
| `revert` | Reverts a previous commit |

### Scopes

| Scope | Subsystem |
|---|---|
| `core` | Core primitives and type system |
| `runtime` | Execution runtime |
| `neural` | Neural engine and NN framework |
| `inference` | Model inference |
| `memory` | Memory subsystem |
| `kg` | Knowledge graph |
| `reasoning` | Reasoning engine |
| `executive` | Executive/planning layer |
| `capabilities` | Capability management |
| `agents` | Agent framework |
| `workflows` | Workflow orchestration |
| `tools` | Tool framework |
| `distributed` | Distributed computing |
| `robotics` | Robotics subsystem |
| `business` | Business automation |
| `security` | Security layer |
| `ui` | User interface |
| `sdk` | Client SDKs |
| `plugins` | Plugin system |
| `infra` | Infrastructure, CI/CD, Docker |
| `docs` | Documentation |

### Examples

```
feat(reasoning): implement causal inference chain
fix(memory): resolve memory leak in episodic store
perf(neural): optimize CUDA kernel for matrix multiplication
docs(api): add inference API reference
ci(infra): add CUDA build pipeline
```

---

## Branching Model

Neo uses a simplified trunk-based development model:

```
main                    ← Production-ready, always deployable
├── develop             ← Integration branch (optional, for large features)
├── feat/<scope>/<name> ← Feature branches
├── fix/<scope>/<name>  ← Bug fix branches
├── release/<version>   ← Release preparation branches
└── hotfix/<scope>/<name> ← Emergency production fixes
```

### Rules

- `main` is always green (CI must pass)
- Feature branches are created from `main` and merged via squash-merge
- All changes require at least one review
- Force-push is prohibited on `main` and `release/*` branches
- Branches are deleted after merge

---

## Pull Request Process

1. **Create a branch** following the naming convention
2. **Make changes** following coding standards
3. **Write tests** for all new functionality
4. **Update documentation** if adding/changing public APIs
5. **Run the full test suite** locally before submitting
6. **Submit PR** with a Conventional Commit title
7. **Address review feedback** — all conversations must be resolved
8. **Squash-merge** once approved and CI passes

### PR Template

```markdown
## Description

<What does this PR do?>

## Type of Change

- [ ] Bug fix
- [ ] New feature
- [ ] Breaking change
- [ ] Documentation update
- [ ] Performance improvement
- [ ] Refactoring

## Testing

- [ ] Unit tests pass
- [ ] Integration tests pass
- [ ] New tests added (if applicable)
- [ ] Benchmarks show no regression (if applicable)

## Checklist

- [ ] Code follows project coding standards
- [ ] Self-review completed
- [ ] Documentation updated
- [ ] No secrets or keys committed
- [ ] Error handling is comprehensive
- [ ] Logging follows conventions
```

---

## Coding Standards

### Universal Rules

1. **No placeholder comments** — every file must contain real, compilable/runnable code
2. **No `TODO`s without a linked issue** — use `// TODO(#123): description`
3. **No commented-out code** — delete it; version control preserves history
4. **Maximum function length**: 50 lines (prefer smaller, composable functions)
5. **Maximum file length**: 500 lines (split into modules if larger)
6. **Maximum nesting depth**: 4 levels
7. **Every public API must have documentation**
8. **Every error path must be handled**

### Language-Specific Standards

See [docs/developer/coding-standards.md](docs/developer/coding-standards.md) for detailed per-language standards.

---

## Testing Requirements

| Category | Requirement |
|---|---|
| Unit tests | 100% of public API functions |
| Integration tests | All cross-subsystem interfaces |
| Property tests | Data structures, serialization, protocol handlers |
| Benchmarks | All performance-critical paths |
| Fuzzing | All parsing, deserialization, and network interfaces |

Run tests before submitting:

```bash
# Rust
cargo test --workspace

# Go
go test ./...

# Python
python -m pytest

# TypeScript
pnpm test

# Full suite
./scripts/test.sh
```

---

## Documentation Requirements

- All public APIs must have doc comments
- All architectural decisions must have ADR documents in `docs/design/`
- All subsystems must have a README explaining purpose, architecture, and usage
- All cross-system interactions must be documented in architecture docs

---

## Security Requirements

- **Never commit secrets, keys, or credentials**
- **Never use `unsafe` code without a security review** (Rust)
- **Never disable security features** without explicit approval
- **All input must be validated and sanitized**
- **All network communication must be authenticated and encrypted**
- **Report vulnerabilities** via [SECURITY.md](SECURITY.md) process

---

## Questions?

Open a discussion at the repository's GitHub Discussions page or reach out to the maintainers.
