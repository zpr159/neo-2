package executor

import (
	"context"
	"sync"

	"go.uber.org/zap"
)

type Executor struct {
	logger  *zap.Logger
	running bool
	mu      sync.RWMutex
}

func NewExecutor(logger *zap.Logger) *Executor {
	return &Executor{
		logger:  logger,
		running: false,
	}
}

func (e *Executor) Start(ctx context.Context) error {
	e.mu.Lock()
	defer e.mu.Unlock()

	if e.running {
		return nil
	}

	e.logger.Info("executor starting")
	e.running = true

	go func() {
		<-ctx.Done()
		e.Stop()
	}()

	e.logger.Info("executor started")
	return nil
}

func (e *Executor) Stop() error {
	e.mu.Lock()
	defer e.mu.Unlock()

	if !e.running {
		return nil
	}

	e.logger.Info("executor stopping")
	e.running = false
	e.logger.Info("executor stopped")
	return nil
}

func (e *Executor) IsRunning() bool {
	e.mu.RLock()
	defer e.mu.RUnlock()
	return e.running
}
