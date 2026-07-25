// Neo Executive — Go Tests
package executor_test

import (
    "context"
    "testing"

    "github.com/neo-agi/neo/executive/go/internal/executor"
)

func TestNewExecutor(t *testing.T) {
    exec := executor.NewExecutor()
    if exec == nil {
        t.Fatal("expected non-nil executor")
    }
    if exec.IsRunning() {
        t.Fatal("executor should not be running initially")
    }
}

func TestExecutorStartStop(t *testing.T) {
    exec := executor.NewExecutor()
    ctx := context.Background()

    if err := exec.Start(ctx); err != nil {
        t.Fatalf("failed to start: %v", err)
    }
    if !exec.IsRunning() {
        t.Fatal("executor should be running")
    }

    if err := exec.Stop(); err != nil {
        t.Fatalf("failed to stop: %v", err)
    }
    if exec.IsRunning() {
        t.Fatal("executor should not be running after stop")
    }
}
