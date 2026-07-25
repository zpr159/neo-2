package audit

import (
	"sync"
	"time"
)

type AuditEntry struct {
	Timestamp time.Time
	Level     string
	Source    string
	Action    string
	Principal string
	Result    string
}

type AuditService struct {
	events []AuditEntry
	mu     sync.RWMutex
}

func NewAuditService() *AuditService {
	return &AuditService{
		events: make([]AuditEntry, 0, 256),
	}
}

func (a *AuditService) Log(entry AuditEntry) {
	a.mu.Lock()
	defer a.mu.Unlock()

	if entry.Timestamp.IsZero() {
		entry.Timestamp = time.Now()
	}

	a.events = append(a.events, entry)
}

func (a *AuditService) Query(level string, limit int) []AuditEntry {
	a.mu.RLock()
	defer a.mu.RUnlock()

	var results []AuditEntry
	for i := len(a.events) - 1; i >= 0 && len(results) < limit; i-- {
		if level == "" || a.events[i].Level == level {
			results = append(results, a.events[i])
		}
	}
	return results
}

func (a *AuditService) TotalEvents() int {
	a.mu.RLock()
	defer a.mu.RUnlock()
	return len(a.events)
}
