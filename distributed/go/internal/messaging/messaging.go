package messaging

import (
	"sync"
)

type MessageBroker struct {
	subscribers map[string][]chan []byte
	mu          sync.RWMutex
}

func NewBroker() *MessageBroker {
	return &MessageBroker{
		subscribers: make(map[string][]chan []byte),
	}
}

func (b *MessageBroker) Publish(topic string, data []byte) error {
	b.mu.RLock()
	channels := b.subscribers[topic]
	b.mu.RUnlock()

	for _, ch := range channels {
		select {
		case ch <- data:
		default:
		}
	}
	return nil
}

func (b *MessageBroker) Subscribe(topic string) (<-chan []byte, func()) {
	ch := make(chan []byte, 64)

	b.mu.Lock()
	b.subscribers[topic] = append(b.subscribers[topic], ch)
	b.mu.Unlock()

	unsubscribe := func() {
		b.mu.Lock()
		defer b.mu.Unlock()
		subs := b.subscribers[topic]
		for i, sub := range subs {
			if sub == ch {
				b.subscribers[topic] = append(subs[:i], subs[i+1:]...)
				close(ch)
				return
			}
		}
	}

	return ch, unsubscribe
}

func (b *MessageBroker) TopicCount() int {
	b.mu.RLock()
	defer b.mu.RUnlock()
	return len(b.subscribers)
}
