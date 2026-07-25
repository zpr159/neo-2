// Neo AGI OS — Go mock objects for testing.

package mock

import (
	"crypto/rand"
	"encoding/hex"
	"sync"
)

// MockAgent is a lightweight mock agent for testing.
type MockAgent struct {
	id       string
	name     string
	state    string
	messages []map[string]interface{}
	mu       sync.Mutex
}

// NewMockAgent creates a new MockAgent with the given name.
func NewMockAgent(name string) *MockAgent {
	return &MockAgent{
		id:       generateID(),
		name:     name,
		state:    "stopped",
		messages: make([]map[string]interface{}, 0),
	}
}

// ID returns the agent's unique identifier.
func (a *MockAgent) ID() string {
	return a.id
}

// Name returns the agent's name.
func (a *MockAgent) Name() string {
	return a.name
}

// State returns the agent's current state.
func (a *MockAgent) State() string {
	a.mu.Lock()
	defer a.mu.Unlock()
	return a.state
}

// Start transitions the agent to the running state.
func (a *MockAgent) Start() {
	a.mu.Lock()
	defer a.mu.Unlock()
	a.state = "running"
}

// Stop transitions the agent to the stopped state.
func (a *MockAgent) Stop() {
	a.mu.Lock()
	defer a.mu.Unlock()
	a.state = "stopped"
}

// SendMessage adds a message to the agent's outbox.
func (a *MockAgent) SendMessage(msg map[string]interface{}) {
	a.mu.Lock()
	defer a.mu.Unlock()
	a.messages = append(a.messages, msg)
}

// ReceiveMessage returns the next message or nil.
func (a *MockAgent) ReceiveMessage() map[string]interface{} {
	a.mu.Lock()
	defer a.mu.Unlock()
	if len(a.messages) == 0 {
		return nil
	}
	msg := a.messages[0]
	a.messages = a.messages[1:]
	return msg
}

// MockTool is a lightweight mock tool for testing.
type MockTool struct {
	id   string
	name string
}

// NewMockTool creates a new MockTool.
func NewMockTool(name string) *MockTool {
	return &MockTool{
		id:   generateID(),
		name: name,
	}
}

// ID returns the tool's unique identifier.
func (t *MockTool) ID() string {
	return t.id
}

// Name returns the tool's name.
func (t *MockTool) Name() string {
	return t.name
}

// Execute runs the tool with given parameters and returns a result.
func (t *MockTool) Execute(params map[string]interface{}) map[string]interface{} {
	return map[string]interface{}{
		"status": "ok",
		"params": params,
	}
}

func generateID() string {
	b := make([]byte, 16)
	_, _ = rand.Read(b)
	return hex.EncodeToString(b)
}
