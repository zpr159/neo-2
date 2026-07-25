# ADR-0004: Error Handling

## Status

Accepted

## Context

Neo AGI OS uses 7 languages, each with different error handling paradigms. We need a unified approach that provides consistency across language boundaries while respecting each language's idioms.

Key requirements:
- Errors must be machine-parseable at API boundaries
- Error information must be preserved across language boundaries
- Error handling must not significantly impact performance
- Developers should be able to handle errors idiomatically in each language
- Errors should include sufficient context for debugging

## Decision

We adopt a layered error handling strategy:

### Layer 1: Internal Language-Idiomatic Handling

Each language uses its native error handling:

| Language | Mechanism |
|----------|-----------|
| Rust | `Result<T, E>` with custom error types via `thiserror` |
| C++ | Exception hierarchy + RAII |
| Python | Exception classes + context managers |
| TypeScript | Discriminated union `Result<T, E>` pattern |
| Go | Error return values + `fmt.Errorf` wrapping |
| Kotlin | `Result<T>` + sealed class errors |
| Swift | `throws` + `Error` protocol |

### Layer 2: Cross-Language Error Protocol

At language boundaries (gRPC, API), errors are serialized using a standard error schema:

```protobuf
message NeoError {
    string code = 1;
    string message = 2;
    map<string, string> details = 3;
    string trace_id = 4;
    repeated NeoError causes = 5;
}
```

### Layer 3: Error Classification

All errors are classified into categories:

| Category | Code Range | Description | Retryable |
|----------|-----------|-------------|-----------|
| Validation | 1000-1999 | Invalid input | No |
| Authentication | 2000-2999 | Auth failures | No |
| Authorization | 3000-3999 | Permission denied | No |
| Not Found | 4000-4999 | Resource not found | No |
| Conflict | 5000-5999 | State conflicts | Sometimes |
| Resource | 6000-6999 | Resource exhaustion | Yes |
| Internal | 7000-7999 | System errors | Once |
| External | 8000-8999 | External service errors | Yes |

## Consequences

### Positive

- Each language handles errors idiomatically
- Cross-language errors are consistent and parseable
- Error classification enables automated retry logic
- Trace IDs enable distributed debugging
- Error causes provide full context chains

### Negative

- Error mapping between languages requires maintenance
- Custom error types in each language add boilerplate
- Error classification must be consistent across services

### Mitigations

- Protobuf schema enforces error structure at API boundaries
- Shared error code registry prevents conflicts
- CI validates error code uniqueness
- Code generation produces error handling boilerplate

## Alternatives Considered

### Option 1: Exceptions Everywhere

Use exceptions in all languages (where supported).

Pros: Familiar to most developers, automatic stack unwinding.
Cons: Performance overhead, Rust does not support exceptions, Go does not support exceptions, error paths are invisible in function signatures.
Rejected because: Not all target languages support exceptions.

### Option 2: Result Types Everywhere

Use Result/Option types in all languages.

Pros: Explicit error paths, no hidden control flow.
Cons: Python and Go have different idioms, TypeScript Result types are verbose, adds friction for developers.
Rejected because: Forcing non-native patterns reduces productivity.

### Option 3: Error Codes Only

Return integer error codes everywhere.

Pros: Simple, language-agnostic.
Cons: Lose error context, hard to debug, no error chains.
Rejected because: Insufficient debugging information.
