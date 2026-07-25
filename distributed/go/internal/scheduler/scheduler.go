package scheduler

import (
	"fmt"
	"sync"
)

type Task struct {
	ID           string
	Payload      []byte
	Priority     int
	AssignedNode string
}

type DistributedScheduler struct {
	taskQueue        chan Task
	nodeCapabilities map[string][]string
	taskNodes        map[string]string
	mu               sync.RWMutex
}

func NewScheduler(queueSize int) *DistributedScheduler {
	if queueSize <= 0 {
		queueSize = 256
	}
	return &DistributedScheduler{
		taskQueue:        make(chan Task, queueSize),
		nodeCapabilities: make(map[string][]string),
		taskNodes:        make(map[string]string),
	}
}

func (s *DistributedScheduler) Enqueue(task Task) error {
	select {
	case s.taskQueue <- task:
		return nil
	default:
		return fmt.Errorf("task queue is full")
	}
}

func (s *DistributedScheduler) AssignToNode(taskID string, nodeID string) error {
	s.mu.Lock()
	defer s.mu.Unlock()
	s.taskNodes[taskID] = nodeID
	return nil
}

func (s *DistributedScheduler) CompleteTask(taskID string) error {
	s.mu.Lock()
	defer s.mu.Unlock()
	delete(s.taskNodes, taskID)
	return nil
}

func (s *DistributedScheduler) QueueLength() int {
	return len(s.taskQueue)
}

func (s *DistributedScheduler) RegisteredNodes() int {
	s.mu.RLock()
	defer s.mu.RUnlock()
	return len(s.nodeCapabilities)
}

func (s *DistributedScheduler) RegisterNode(nodeID string, capabilities []string) {
	s.mu.Lock()
	defer s.mu.Unlock()
	s.nodeCapabilities[nodeID] = capabilities
}
