# Neo AGI OS — Coding Standards

## Table of Contents

- [1. General Principles](#1-general-principles)
- [2. Rust](#2-rust)
- [3. C++](#3-c)
- [4. Python](#4-python)
- [5. TypeScript](#5-typescript)
- [6. Go](#6-go)
- [7. Kotlin](#7-kotlin)
- [8. Swift](#8-swift)
- [9. Protobuf](#9-protobuf)
- [10. Documentation](#10-documentation)
- [11. Git Conventions](#11-git-conventions)

---

## 1. General Principles

### 1.1 Code Quality

- Write code that is clear and self-documenting
- Prefer explicit over implicit
- Keep functions small and focused (single responsibility)
- Avoid premature optimization
- Handle all error paths

### 1.2 Naming Conventions

All languages follow a consistent philosophy:

- **Variables and functions**: descriptive, camelCase or snake_case per language convention
- **Types and classes**: PascalCase
- **Constants**: SCREAMING_SNAKE_CASE
- **Private members**: prefixed with underscore or `private` keyword per language

### 1.3 Error Handling

- Never silently ignore errors
- Use language-native error handling (Result, exceptions, error returns)
- Provide meaningful error messages
- Log errors with context
- Include source location in error messages where possible

### 1.4 Comments

- Do not add comments unless requested by the user
- When writing code, let the code speak for itself
- Use doc comments for public API surfaces only

---

## 2. Rust

### 2.1 Formatting

```bash
# Format all code
cargo fmt --all

# Check formatting
cargo fmt --all -- --check
```

### 2.2 Style Rules

```rust
// GOOD: Clear function signature with ownership
pub fn process_batch(items: &[Item]) -> Result<Vec<Output>, Error> {
    items.iter()
        .map(|item| process_single(item))
        .collect::<Result<Vec<_>, _>>()
}

// GOOD: Enum with clear variants
pub enum TaskState {
    Pending,
    Running { started_at: Instant },
    Completed { result: Value },
    Failed { error: Error },
}

// GOOD: Struct with builder pattern for complex construction
pub struct NeuralConfig {
    pub model_path: PathBuf,
    pub batch_size: usize,
    pub max_sequence_length: usize,
}

impl NeuralConfig {
    pub fn builder() -> NeuralConfigBuilder {
        NeuralConfigBuilder::default()
    }
}

// BAD: Unnecessary clone
let data = items.clone(); // Avoid if reference suffices

// GOOD: Use reference
let data = &items;
```

### 2.3 Lint Rules

```bash
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Key clippy lints enforced:

- `clippy::unwrap_used`: Use `?` or `expect()` instead
- `clippy::panic!`: Use `return Err()` instead
- `clippy::clone_on_copy`: Use copy semantics
- `clippy::needless_pass_by_value`: Use references
- `clippy::module_inception`: Module names should not match type names

### 2.4 Error Handling

```rust
// GOOD: Custom error type with thiserror
#[derive(Debug, thiserror::Error)]
pub enum NeuralError {
    #[error("model not found: {0}")]
    ModelNotFound(String),
    
    #[error("inference failed: {0}")]
    InferenceFailed(String),
    
    #[error("gpu out of memory")]
    GpuOutOfMemory,
}

// GOOD: Using ? operator
pub fn infer(model: &Model, input: &Tensor) -> Result<Tensor, NeuralError> {
    let preprocessed = preprocess(input)?;
    let output = model.forward(&preprocessed)?;
    postprocess(&output)
}

// BAD: Using unwrap()
let output = model.forward(&input).unwrap(); // Panics on error
```

### 2.5 Async Code

```rust
// GOOD: Explicit async with proper bounds
pub async fn process_task(
    task: Task,
    scheduler: &Scheduler,
) -> Result<TaskResult, Error> {
    let agent = scheduler.find_agent(&task).await?;
    agent.execute(task).await
}

// GOOD: Use tokio::spawn for concurrent work
let handles: Vec<_> = tasks.into_iter()
    .map(|task| tokio::spawn(process_task(task, scheduler.clone())))
    .collect();

let results = futures::future::join_all(handles).await;
```

### 2.6 Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_functionality() {
        let config = NeuralConfig::builder()
            .model_path("test_model.bin")
            .batch_size(32)
            .build();
        assert_eq!(config.batch_size, 32);
    }

    #[tokio::test]
    async fn test_async_operation() {
        let result = async_process("input").await.unwrap();
        assert!(!result.is_empty());
    }
}
```

---

## 3. C++

### 3.1 Formatting

Use clang-format with the following configuration:

```yaml
BasedOnStyle: Google
IndentWidth: 2
ColumnLimit: 100
AllowShortFunctionsOnASingleLine: Empty
AllowShortIfStatementsOnASingleLine: false
BreakBeforeBraces: Attach
PointerAlignment: Left
```

### 3.2 Style Rules

```cpp
// GOOD: RAII for resource management
class GpuBuffer {
public:
    explicit GpuBuffer(size_t size) : size_(size) {
        cudaMalloc(&data_, size);
    }
    
    ~GpuBuffer() {
        cudaFree(data_);
    }
    
    // Non-copyable, movable
    GpuBuffer(const GpuBuffer&) = delete;
    GpuBuffer& operator=(const GpuBuffer&) = delete;
    GpuBuffer(GpuBuffer&& other) noexcept;
    GpuBuffer& operator=(GpuBuffer&& other) noexcept;
    
    [[nodiscard]] void* data() const { return data_; }
    [[nodiscard]] size_t size() const { return size_; }

private:
    void* data_ = nullptr;
    size_t size_ = 0;
};

// GOOD: Smart pointers
auto buffer = std::make_unique<GpuBuffer>(1024);

// BAD: Raw new/delete
GpuBuffer* buffer = new GpuBuffer(1024); // Memory leak risk
delete buffer; // Manual management
```

### 3.3 CUDA Code

```cpp
// GOOD: Kernel with proper error checking
__global__ void vector_add(
    const float* a, const float* b, float* c, int n
) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx < n) {
        c[idx] = a[idx] + b[idx];
    }
}

// GOOD: Error checking macro
#define CUDA_CHECK(call) do { \
    cudaError_t err = call; \
    if (err != cudaSuccess) { \
        throw std::runtime_error( \
            cudaGetErrorString(err)); \
    } \
} while(0)

// Usage
CUDA_CHECK(cudaMalloc(&d_ptr, size));
```

### 3.4 Memory Safety

- Use RAII for all resource management
- Use smart pointers (`std::unique_ptr`, `std::shared_ptr`)
- Avoid raw pointers except for FFI and CUDA kernels
- Use `std::array` or `std::vector` instead of C arrays
- Use `std::string_view` for read-only string parameters

---

## 4. Python

### 4.1 Formatting

```bash
# Format
ruff format .

# Lint
ruff check .

# Type check
mypy .
```

### 4.2 Style Rules

```python
# GOOD: Type hints everywhere
def process_batch(
    items: list[Item],
    batch_size: int = 32,
) -> list[Output]:
    results: list[Output] = []
    for i in range(0, len(items), batch_size):
        batch = items[i:i + batch_size]
        results.extend(process_single(item) for item in batch)
    return results

# GOOD: Dataclass for structured data
from dataclasses import dataclass, field

@dataclass
class TrainingConfig:
    model_name: str
    learning_rate: float = 1e-3
    batch_size: int = 32
    epochs: int = 10
    device: str = "cuda"

# GOOD: Exception handling
def load_model(path: Path) -> Model:
    if not path.exists():
        raise FileNotFoundError(f"Model not found: {path}")
    try:
        return Model.load(path)
    except Exception as e:
        raise RuntimeError(f"Failed to load model: {e}") from e

# BAD: No type hints
def process(items, batch_size=32):
    pass

# BAD: Bare except
try:
    do_something()
except:
    pass  # Silently swallows all errors
```

### 4.3 Ruff Configuration

```toml
[tool.ruff]
line-length = 100
target-version = "py311"

[tool.ruff.lint]
select = ["E", "F", "I", "N", "W", "UP", "B", "A", "C4", "SIM"]
ignore = ["E501"]

[tool.mypy]
python_version = "3.11"
strict = true
warn_return_any = true
warn_unused_configs = true
```

### 4.4 PyTorch Code

```python
# GOOD: Model with proper device management
import torch
import torch.nn as nn

class NeoTransformer(nn.Module):
    def __init__(self, config: ModelConfig) -> None:
        super().__init__()
        self.config = config
        self.embedding = nn.Embedding(config.vocab_size, config.d_model)
        self.layers = nn.ModuleList([
            TransformerLayer(config) for _ in range(config.num_layers)
        ])
        self.output = nn.Linear(config.d_model, config.vocab_size)
    
    def forward(
        self, input_ids: torch.Tensor, attention_mask: torch.Tensor | None = None
    ) -> torch.Tensor:
        x = self.embedding(input_ids)
        for layer in self.layers:
            x = layer(x, attention_mask)
        return self.output(x)

# GOOD: Training loop with mixed precision
def train_epoch(
    model: nn.Module,
    dataloader: DataLoader,
    optimizer: Optimizer,
    scaler: torch.cuda.amp.GradScaler,
) -> float:
    model.train()
    total_loss = 0.0
    for batch in dataloader:
        optimizer.zero_grad()
        with torch.cuda.amp.autocast():
            loss = compute_loss(model, batch)
        scaler.scale(loss).backward()
        scaler.step(optimizer)
        scaler.update()
        total_loss += loss.item()
    return total_loss / len(dataloader)
```

---

## 5. TypeScript

### 5.1 Formatting

```bash
# Lint
pnpm lint

# Format check
pnpm format:check

# Type check
pnpm typecheck
```

### 5.2 Style Rules

```typescript
// GOOD: Strict typing
interface AgentConfig {
  readonly id: string;
  name: string;
  maxRetries: number;
  timeout: number;
}

// GOOD: Discriminated unions
type TaskResult =
  | { status: "success"; data: unknown }
  | { status: "error"; error: string }
  | { status: "pending"; taskId: string };

// GOOD: Error handling with Result pattern
function parseConfig(raw: unknown): Result<Config, Error> {
  if (!isValidConfig(raw)) {
    return { ok: false, error: new Error("Invalid config") };
  }
  return { ok: true, data: raw as Config };
}

// GOOD: Generic constraints
async function fetchAndParse<T>(
  url: string,
  parser: (data: unknown) => T,
): Promise<T> {
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(`HTTP ${response.status}`);
  }
  const data = await response.json();
  return parser(data);
}

// BAD: any type
function process(data: any): any {
  return data.whatever;
}

// BAD: Non-null assertion
const element = document.getElementById("app")!;
```

### 5.3 ESLint Configuration

```json
{
  "extends": [
    "eslint:recommended",
    "plugin:@typescript-eslint/recommended",
    "plugin:@typescript-eslint/recommended-requiring-type-checking"
  ],
  "rules": {
    "@typescript-eslint/no-explicit-any": "error",
    "@typescript-eslint/no-unused-vars": "error",
    "@typescript-eslint/explicit-function-return-type": "warn",
    "no-console": "warn"
  }
}
```

---

## 6. Go

### 6.1 Formatting

```bash
# Format
gofmt -w .

# Lint
golangci-lint run

# Vet
go vet ./...
```

### 6.2 Style Rules

```go
// GOOD: Clear error handling
func ProcessTask(ctx context.Context, task *Task) (*Result, error) {
    if task == nil {
        return nil, errors.New("task is nil")
    }
    
    result, err := executeTask(ctx, task)
    if err != nil {
        return nil, fmt.Errorf("executing task %s: %w", task.ID, err)
    }
    
    return result, nil
}

// GOOD: Interface design
type Storer interface {
    Get(ctx context.Context, key string) ([]byte, error)
    Put(ctx context.Context, key string, value []byte) error
    Delete(ctx context.Context, key string) error
}

// GOOD: Context propagation
func (s *Server) HandleRequest(w http.ResponseWriter, r *http.Request) {
    ctx := r.Context()
    result, err := s.service.Process(ctx, r.Body)
    if err != nil {
        http.Error(w, err.Error(), http.StatusInternalServerError)
        return
    }
    json.NewEncoder(w).Encode(result)
}

// BAD: Ignoring errors
result, _ := doSomething()

// BAD: Named return values with naked returns
func bad() (result string, err error) {
    result = "hello"
    return // Naked return
}
```

### 6.3 Error Handling

```go
// GOOD: Custom error types
type ValidationError struct {
    Field   string
    Message string
}

func (e *ValidationError) Error() string {
    return fmt.Sprintf("validation error on field %s: %s", e.Field, e.Message)
}

// GOOD: Error wrapping
if err != nil {
    return fmt.Errorf("processing agent %s: %w", agent.ID, err)
}

// GOOD: Error checking
var validationErr *ValidationError
if errors.As(err, &validationErr) {
    log.Printf("Validation failed: %s", validationErr.Field)
}
```

---

## 7. Kotlin

### 7.1 Formatting

Use ktlint with default rules plus:

```editorconfig
[*.{kt,kts}]
indent_size = 4
max_line_length = 120
```

### 7.2 Style Rules

```kotlin
// GOOD: Null safety
fun processAgent(agent: Agent?): String {
    return agent?.name ?: "unknown"
}

// GOOD: Data class for structured data
@Serializable
data class TaskConfig(
    val name: String,
    val priority: Int = 0,
    val timeout: Duration = 30.seconds,
)

// GOOD: Sealed class for state
sealed class AgentState {
    data object Idle : AgentState()
    data class Running(val taskId: String) : AgentState()
    data class Failed(val error: String) : AgentState()
}

// GOOD: Coroutine usage
suspend fun processTask(task: Task): Result<TaskResult> = coroutineScope {
    val deferred = async { executeTask(task) }
    withTimeout(30.seconds) {
        deferred.await()
    }
}

// GOOD: Extension function
fun Agent.Companion.create(name: String): Agent {
    return Agent(
        id = UUID.randomUUID().toString(),
        name = name,
        state = AgentState.Idle,
    )
}

// BAD: Nullable without safe call
val length = name!!.length // Crashes on null

// BAD: Blocking in coroutine
fun blocking(): String {
    Thread.sleep(1000) // Blocks the thread
    return "done"
}
```

---

## 8. Swift

### 8.1 Formatting

Use SwiftFormat with default settings.

### 8.2 Style Rules

```swift
// GOOD: Protocol-oriented design
protocol AgentProtocol {
    var id: String { get }
    var state: AgentState { get }
    func start() async throws
    func stop() async
}

// GOOD: Struct over class for value types
struct TaskConfig {
    let name: String
    let priority: Int
    let timeout: TimeInterval
}

// GOOD: Error handling
enum AgentError: Error, LocalizedError {
    case notConnected
    case taskFailed(String)
    case timeout(TimeInterval)
    
    var errorDescription: String? {
        switch self {
        case .notConnected:
            return "Agent is not connected"
        case .taskFailed(let reason):
            return "Task failed: \(reason)"
        case .timeout(let interval):
            return "Operation timed out after \(interval)s"
        }
    }
}

// GOOD: Async/await
func processTask(_ task: Task) async throws -> TaskResult {
    guard isConnected else {
        throw AgentError.notConnected
    }
    
    return try await withThrowingTaskGroup(of: TaskResult.self) { group in
        group.addTask { try await self.execute(task) }
        return try await group.next()!
    }
}

// GOOD: Property wrapper for validation
@propertyWrapper
struct Positive {
    var wrappedValue: Double {
        didSet {
            if wrappedValue < 0 { wrappedValue = 0 }
        }
    }
    
    init(wrappedValue: Double) {
        self.wrappedValue = max(0, wrappedValue)
    }
}

// BAD: Force unwrap
let value = dictionary["key"]! // Crashes if nil

// BAD: Mutable global state
var globalConfig: Config? = nil // Avoid globals
```

---

## 9. Protobuf

### 9.1 Naming Conventions

```protobuf
// Service: PascalCase
service AgentScheduler {
    // Method: PascalCase
    rpc SubmitTask(SubmitTaskRequest) returns (TaskHandle);
}

// Message: PascalCase
message SubmitTaskRequest {
    // Field: snake_case
    string agent_id = 1;
    map<string, string> payload = 2;
    int32 priority = 3;
}

// Enum: PascalCase for type, SCREAMING_SNAKE for values
enum TaskStatus {
    TASK_STATUS_UNSPECIFIED = 0;
    TASK_STATUS_PENDING = 1;
    TASK_STATUS_RUNNING = 2;
    TASK_STATUS_COMPLETED = 3;
}
```

### 9.2 Best Practices

- Always start enums at 0 (UNSPECIFIED)
- Use `map<string, string>` for flexible key-value data
- Reserve field numbers for deleted fields
- Use `oneof` for mutually exclusive fields
- Package proto files logically (`neo.agent`, `neo.neural`)

---

## 10. Documentation

### 10.1 Public API Documentation

Every public function, struct, enum, and trait/class must have doc comments:

```rust
/// Processes a batch of inference requests.
///
/// # Arguments
/// * `requests` - Slice of inference requests to process
/// * `config` - Inference configuration
///
/// # Returns
/// Vector of inference results in the same order as input requests.
///
/// # Errors
/// Returns `NeuralError` if any request fails.
///
/// # Examples
/// ```
/// let results = process_batch(&requests, &config)?;
/// ```
pub fn process_batch(
    requests: &[InferenceRequest],
    config: &InferenceConfig,
) -> Result<Vec<InferenceResult>, NeuralError> {
    // ...
}
```

### 10.2 README Files

Each major component must have a README.md containing:

1. Purpose and overview
2. Quick start guide
3. API reference (brief)
4. Configuration options
5. Troubleshooting

---

## 11. Git Conventions

### 11.1 Commit Messages

```
<type>(<scope>): <description>

[optional body]

[optional footer]
```

Types:

- `feat`: New feature
- `fix`: Bug fix
- `refactor`: Code refactoring
- `docs`: Documentation
- `test`: Tests
- `chore`: Build, CI, tooling
- `perf`: Performance improvement

Examples:

```
feat(neural): add flash attention kernel
fix(storage): prevent WAL corruption on crash
docs(architecture): update deployment topology
```

### 11.2 Branch Naming

```
feat/<ticket-id>-<description>
fix/<ticket-id>-<description>
refactor/<component>-<description>
```

### 11.3 Pull Requests

- Title matches commit message format
- Description explains what and why
- Tests pass
- Lint passes
- At least one review required
- No merge conflicts
