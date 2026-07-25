# Neo AGI OS — Testing Guide

## Overview

Testing is mandatory for all code in Neo AGI OS. Every PR must include tests for new functionality and maintain existing test coverage.

### Test Types

| Type | Scope | Speed | Confidence |
|------|-------|-------|------------|
| Unit | Single function | < 1ms | High |
| Integration | Multiple components | 1-10s | Medium |
| End-to-end | Full system | 10-60s | High |
| Performance | Benchmarks | varies | Baseline |

### Coverage Targets

| Component | Minimum Coverage |
|-----------|-----------------|
| Neural Core | 90% |
| Agent Scheduler | 85% |
| Storage Engine | 90% |
| Knowledge Graph | 85% |
| API Gateway | 80% |
| SDKs | 80% |

## Rust Testing

### Unit Tests

Rust unit tests live alongside the code in `#[cfg(test)]` modules:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_joint_creation() {
        let joint = Joint::new("joint_0");
        assert_eq!(joint.angle(), 0.0);
    }

    #[test]
    fn test_joint_angle_limits() {
        let mut joint = Joint::with_limits("test", -90.0, 90.0);
        joint.set_angle(100.0);
        assert_eq!(joint.angle(), 90.0);
        joint.set_angle(-100.0);
        assert_eq!(joint.angle(), -90.0);
    }

    #[test]
    fn test_concurrent_access() {
        use std::sync::Arc;
        use std::thread;
        
        let counter = Arc::new(AtomicU64::new(0));
        let handles: Vec<_> = (0..10)
            .map(|_| {
                let c = Arc::clone(&counter);
                thread::spawn(move || {
                    for _ in 0..1000 {
                        c.fetch_add(1, Ordering::Relaxed);
                    }
                })
            })
            .collect();
        
        for h in handles {
            h.join().unwrap();
        }
        assert_eq!(counter.load(Ordering::Relaxed), 10000);
    }
}
```

### Integration Tests

Integration tests live in `tests/` directory:

```rust
// tests/storage_integration.rs
use neo_storage::StorageEngine;

#[tokio::test]
async fn test_put_get_delete() {
    let engine = StorageEngine::open_temp().await.unwrap();
    
    engine.put("key1", b"value1").await.unwrap();
    let value = engine.get("key1").await.unwrap();
    assert_eq!(value, Some(b"value1".to_vec()));
    
    engine.delete("key1").await.unwrap();
    let value = engine.get("key1").await.unwrap();
    assert_eq!(value, None);
}

#[tokio::test]
async fn test_concurrent_writes() {
    let engine = StorageEngine::open_temp().await.unwrap();
    let engine = Arc::new(engine);
    
    let handles: Vec<_> = (0..100)
        .map(|i| {
            let engine = Arc::clone(&engine);
            tokio::spawn(async move {
                engine.put(&format!("key_{}", i), b"data").await.unwrap();
            })
        })
        .collect();
    
    for h in handles {
        h.await.unwrap();
    }
}
```

### Running Rust Tests

```bash
# All tests
cargo test --workspace

# Specific crate
cargo test -p neo-storage

# With output
cargo test -- --nocapture

# Specific test
cargo test test_joint_creation

# With backtrace
RUST_BACKTRACE=1 cargo test
```

## C++ Testing

### Using Google Test

```cpp
// tests/test_neural_layer.cpp
#include <gtest/gtest.h>
#include "neural/layer.h"

TEST(NeuralLayerTest, ForwardPass) {
    Layer layer(128, 64);
    Tensor input = Tensor::ones({1, 128});
    Tensor output = layer.forward(input);
    
    EXPECT_EQ(output.shape(), (std::vector<size_t>{1, 64}));
}

TEST(NeuralLayerTest, GradientCheck) {
    Layer layer(32, 16);
    auto [value, grad] = layer.forward_with_grad(Tensor::ones({1, 32}));
    
    double eps = 1e-5;
    for (size_t i = 0; i < 16; ++i) {
        layer.set_weight(i, layer.get_weight(i) + eps);
        auto plus = layer.forward(Tensor::ones({1, 32}));
        layer.set_weight(i, layer.get_weight(i) - 2 * eps);
        auto minus = layer.forward(Tensor::ones({1, 32}));
        layer.set_weight(i, layer.get_weight(i) + eps);
        
        double numerical = (plus[i] - minus[i]) / (2 * eps);
        EXPECT_NEAR(grad[i], numerical, 1e-4);
    }
}
```

### Running C++ Tests

```bash
cmake -B build -DCMAKE_BUILD_TYPE=Debug -DNEO_ENABLE_TESTS=ON
cmake --build build --parallel $(nproc)
ctest --test-dir build --output-on-failure
```

## Python Testing

### Using pytest

```python
# tests/test_training.py
import pytest
import torch
from neo_neural import Model, TrainingConfig

@pytest.fixture
def model():
    config = TrainingConfig(
        model_name="test",
        d_model=64,
        num_layers=2,
        device="cpu",
    )
    return Model(config)

@pytest.fixture
def sample_batch():
    return {
        "input_ids": torch.randint(0, 1000, (4, 128)),
        "labels": torch.randint(0, 1000, (4, 128)),
    }

def test_forward_pass(model, sample_batch):
    output = model(sample_batch["input_ids"])
    assert output.shape == (4, 128, 1000)

def test_training_step(model, sample_batch):
    optimizer = torch.optim.Adam(model.parameters())
    loss = model.training_step(sample_batch)
    loss.backward()
    optimizer.step()
    assert not torch.isnan(loss)

@pytest.mark.parametrize("batch_size", [1, 4, 16, 32])
def test_variable_batch_sizes(model, batch_size):
    input_ids = torch.randint(0, 1000, (batch_size, 128))
    output = model(input_ids)
    assert output.shape[0] == batch_size

class TestKnowledgeGraph:
    def test_entity_creation(self):
        graph = KnowledgeGraph()
        entity = graph.create_entity("Person", {"name": "Neo"})
        assert entity.type == "Person"
    
    def test_relation_creation(self):
        graph = KnowledgeGraph()
        e1 = graph.create_entity("Person", {"name": "Neo"})
        e2 = graph.create_entity("City", {"name": "Zion"})
        rel = graph.create_relation(e1.id, e2.id, "LIVES_IN")
        assert rel.type == "LIVES_IN"
```

### Running Python Tests

```bash
# All tests
python -m pytest

# With coverage
python -m pytest --cov=neo_neural --cov-report=html

# Specific file
python -m pytest tests/test_training.py

# Verbose output
python -m pytest -v

# Stop on first failure
python -m pytest -x
```

## TypeScript Testing

### Using Vitest

```typescript
// tests/neo-app.test.ts
import { describe, it, expect, beforeEach } from 'vitest';
import { NeoApp } from '../src/NeoApp';

describe('NeoApp', () => {
    let app: NeoApp;

    beforeEach(() => {
        app = new NeoApp();
    });

    it('should initialize', () => {
        expect(app.state.initialized).toBe(false);
        app.initialize();
        expect(app.state.initialized).toBe(true);
    });

    it('should navigate between screens', () => {
        app.initialize();
        expect(app.state.currentScreen).toBe('home');
        app.navigate('settings');
        expect(app.state.currentScreen).toBe('settings');
    });

    it('should manage notifications', () => {
        app.notify('Hello');
        expect(app.state.notifications).toHaveLength(1);
        app.shutdown();
        expect(app.state.notifications).toHaveLength(0);
    });
});
```

### Running TypeScript Tests

```bash
# All tests
pnpm test

# Watch mode
pnpm test --watch

# Coverage
pnpm test --coverage

# Specific file
pnpm test -- src/NeoApp.test.ts
```

## Go Testing

### Unit Tests

```go
// handlers/agent_test.go
package handlers

import (
    "context"
    "net/http"
    "net/http/httptest"
    "testing"
)

func TestCreateAgent(t *testing.T) {
    handler := NewAgentHandler(mockService{})
    
    req := httptest.NewRequest("POST", "/agents", strings.NewReader(`{"name":"test"}`))
    req.Header.Set("Content-Type", "application/json")
    w := httptest.NewRecorder()
    
    handler.Create(w, req)
    
    if w.Code != http.StatusCreated {
        t.Errorf("expected 201, got %d", w.Code)
    }
}

func TestCreateAgentInvalidInput(t *testing.T) {
    handler := NewAgentHandler(mockService{})
    
    req := httptest.NewRequest("POST", "/agents", strings.NewReader(`{}`))
    w := httptest.NewRecorder()
    
    handler.Create(w, req)
    
    if w.Code != http.StatusBadRequest {
        t.Errorf("expected 400, got %d", w.Code)
    }
}
```

### Running Go Tests

```bash
# All tests
go test ./...

# With coverage
go test -cover ./...

# Verbose
go test -v ./...

# Race detector
go test -race ./...

# Benchmarks
go test -bench=. ./...
```

## Kotlin Testing

### Using kotlin.test

```kotlin
// NeoClientTest.kt
package com.neo.sdk

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class NeoClientTest {
    @Test
    fun testConnectionLifecycle() {
        val client = NeoClient()
        assertFalse(client.isConnected)
        client.connect()
        assertTrue(client.isConnected)
        client.disconnect()
        assertFalse(client.isConnected)
    }

    @Test
    fun testCreateAgent() {
        val client = NeoClient()
        client.connect()
        val agent = client.createAgent(name = "test-agent")
        assertEquals("test-agent", agent.name)
        assertEquals("idle", agent.state)
    }

    @Test(expected = IllegalStateException::class)
    fun testCreateAgentWithoutConnection() {
        val client = NeoClient()
        client.createAgent(name = "test")
    }
}
```

### Running Kotlin Tests

```bash
cd sdk/kotlin
./gradlew test
```

## Swift Testing

### Using XCTest

```swift
// NeoUITests.swift
import XCTest
@testable import NeoUI

final class NeoUITests: XCTestCase {
    func testInitialization() {
        let app = NeoApp()
        XCTAssertFalse(app.state.initialized)
        app.initialize()
        XCTAssertTrue(app.state.initialized)
    }

    func testNavigation() {
        let app = NeoApp()
        app.initialize()
        app.navigate(to: "settings")
        XCTAssertEqual(app.state.currentScreen, "settings")
    }

    func testShutdown() {
        let app = NeoApp()
        app.initialize()
        app.shutdown()
        XCTAssertFalse(app.state.initialized)
        XCTAssertTrue(app.state.notifications.isEmpty)
    }
}
```

### Running Swift Tests

```bash
swift test
```

## Integration Testing

### Docker-Based Integration Tests

```yaml
# docker-compose.test.yml
version: '3.8'
services:
  neo-core:
    build: .
    ports:
      - "8080:8080"
  
  test-runner:
    image: neo-test-runner
    depends_on:
      - neo-core
    environment:
      NEO_API_URL: http://neo-core:8080
    command: pytest integration_tests/
```

### Running Integration Tests

```bash
# Start services
docker-compose -f docker-compose.test.yml up -d

# Run tests
docker-compose -f docker-compose.test.yml run test-runner

# Cleanup
docker-compose -f docker-compose.test.yml down
```

## Test Organization

### Directory Structure

```
tests/
  unit/
    rust/
    python/
    typescript/
    go/
  integration/
    api/
    neural/
    storage/
  e2e/
    workflows/
  fixtures/
    models/
    data/
    configs/
  helpers/
    common.py
    test_utils.rs
```

### Naming Conventions

| Language | Pattern | Example |
|----------|---------|---------|
| Rust | `test_<function_name>` | `test_joint_creation` |
| Python | `test_<function_name>` or `TestClassName` | `test_forward_pass` |
| TypeScript | `describe/it` blocks | `describe('NeoApp')` |
| Go | `Test<TypeName>` | `TestCreateAgent` |
| Kotlin | `test<FunctionName>` | `testConnectionLifecycle` |
| Swift | `test<FunctionName>` | `testInitialization` |

## Fixtures and Mocking

### Rust Fixtures

```rust
use tempfile::TempDir;

fn test_storage() -> (StorageEngine, TempDir) {
    let dir = TempDir::new().unwrap();
    let engine = StorageEngine::open(dir.path()).unwrap();
    (engine, dir)
}

#[test]
fn test_with_fixture() {
    let (engine, _dir) = test_storage();
    engine.put("key", b"value").unwrap();
}
```

### Python Fixtures

```python
@pytest.fixture(scope="session")
def model_config():
    return TrainingConfig(
        model_name="test",
        device="cpu",
    )

@pytest.fixture
def model(model_config):
    return Model(model_config)

@pytest.fixture
def temp_dir():
    with tempfile.TemporaryDirectory() as tmpdir:
        yield Path(tmpdir)
```

### TypeScript Mocking

```typescript
import { describe, it, expect, vi } from 'vitest';

const mockFetch = vi.fn();
vi.stubGlobal('fetch', mockFetch);

describe('API Client', () => {
    it('should handle errors', async () => {
        mockFetch.mockResolvedValue({
            ok: false,
            status: 500,
            json: () => Promise.resolve({ error: 'Internal error' }),
        });
        
        const client = new ApiClient();
        await expect(client.getAgents()).rejects.toThrow('HTTP 500');
    });
});
```
