package scheduler

import (
	"context"
	"sync"
	"sync/atomic"
)

type Scheduler struct {
	tasks         chan func()
	workers       int
	activeWorkers int32
	stopCh        chan struct{}
	stopped       bool
	mu            sync.Mutex
}

func NewScheduler(workers int) *Scheduler {
	if workers <= 0 {
		workers = 4
	}
	return &Scheduler{
		tasks:   make(chan func(), 256),
		workers: workers,
		stopCh:  make(chan struct{}),
	}
}

func (s *Scheduler) Submit(task func()) error {
	s.mu.Lock()
	if s.stopped {
		s.mu.Unlock()
		return ErrSchedulerStopped
	}
	s.mu.Unlock()

	s.tasks <- task
	return nil
}

func (s *Scheduler) Start(ctx context.Context) {
	for i := 0; i < s.workers; i++ {
		go s.worker(ctx)
	}
}

func (s *Scheduler) worker(ctx context.Context) {
	for {
		select {
		case <-ctx.Done():
			return
		case <-s.stopCh:
			return
		case task, ok := <-s.tasks:
			if !ok {
				return
			}
			atomic.AddInt32(&s.activeWorkers, 1)
			task()
			atomic.AddInt32(&s.activeWorkers, -1)
		}
	}
}

func (s *Scheduler) Stop() {
	s.mu.Lock()
	defer s.mu.Unlock()
	if s.stopped {
		return
	}
	s.stopped = true
	close(s.stopCh)
	close(s.tasks)
}

func (s *Scheduler) PendingCount() int {
	return len(s.tasks)
}

func (s *Scheduler) ActiveWorkers() int {
	return int(atomic.LoadInt32(&s.activeWorkers))
}

var ErrSchedulerStopped = &schedulerError{"scheduler is stopped"}

type schedulerError struct {
	msg string
}

func (e *schedulerError) Error() string {
	return e.msg
}
