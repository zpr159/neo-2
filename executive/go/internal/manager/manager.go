package manager

import (
	"context"
	"fmt"
	"sync"

	"go.uber.org/zap"
)

type Component interface {
	Name() string
	Start(ctx context.Context) error
	Stop() error
}

type Manager struct {
	components map[string]Component
	logger     *zap.Logger
	mu         sync.RWMutex
}

func NewManager(logger *zap.Logger) *Manager {
	return &Manager{
		components: make(map[string]Component),
		logger:     logger,
	}
}

func (m *Manager) Register(name string, comp Component) {
	m.mu.Lock()
	defer m.mu.Unlock()
	m.components[name] = comp
	m.logger.Info("component registered", zap.String("name", name))
}

func (m *Manager) StartAll(ctx context.Context) error {
	m.mu.RLock()
	defer m.mu.RUnlock()

	for name, comp := range m.components {
		m.logger.Info("starting component", zap.String("name", name))
		if err := comp.Start(ctx); err != nil {
			return fmt.Errorf("failed to start component %s: %w", name, err)
		}
		m.logger.Info("component started", zap.String("name", name))
	}
	return nil
}

func (m *Manager) StopAll() error {
	m.mu.RLock()
	defer m.mu.RUnlock()

	var lastErr error
	for name, comp := range m.components {
		m.logger.Info("stopping component", zap.String("name", name))
		if err := comp.Stop(); err != nil {
			m.logger.Error("failed to stop component", zap.String("name", name), zap.Error(err))
			lastErr = err
		}
		m.logger.Info("component stopped", zap.String("name", name))
	}
	return lastErr
}

func (m *Manager) GetComponent(name string) (Component, bool) {
	m.mu.RLock()
	defer m.mu.RUnlock()
	comp, ok := m.components[name]
	return comp, ok
}
