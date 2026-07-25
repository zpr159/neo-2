package consensus

import (
	"sync"
)

type ConsensusManager struct {
	currentTerm uint64
	votedFor    string
	logIndex    uint64
	leader      string
	mu          sync.RWMutex
}

func NewConsensus() *ConsensusManager {
	return &ConsensusManager{
		currentTerm: 0,
		logIndex:    0,
	}
}

func (c *ConsensusManager) RequestVote(term uint64, candidateID string) (bool, uint64) {
	c.mu.Lock()
	defer c.mu.Unlock()

	if term < c.currentTerm {
		return false, c.currentTerm
	}

	if term > c.currentTerm {
		c.currentTerm = term
		c.votedFor = ""
	}

	if c.votedFor == "" || c.votedFor == candidateID {
		c.votedFor = candidateID
		return true, c.currentTerm
	}

	return false, c.currentTerm
}

func (c *ConsensusManager) AppendEntries(term uint64, leaderID string, entries [][]byte) (bool, uint64) {
	c.mu.Lock()
	defer c.mu.Unlock()

	if term < c.currentTerm {
		return false, c.currentTerm
	}

	c.currentTerm = term
	c.leader = leaderID
	c.logIndex += uint64(len(entries))

	return true, c.currentTerm
}

func (c *ConsensusManager) CurrentTerm() uint64 {
	c.mu.RLock()
	defer c.mu.RUnlock()
	return c.currentTerm
}

func (c *ConsensusManager) VotedFor() string {
	c.mu.RLock()
	defer c.mu.RUnlock()
	return c.votedFor
}

func (c *ConsensusManager) LogIndex() uint64 {
	c.mu.RLock()
	defer c.mu.RUnlock()
	return c.logIndex
}

func (c *ConsensusManager) Leader() string {
	c.mu.RLock()
	defer c.mu.RUnlock()
	return c.leader
}
