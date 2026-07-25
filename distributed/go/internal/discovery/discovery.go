package discovery

import (
	"fmt"
	"sync"
)

type DiscoveryService struct {
	method string
	nodes  sync.Map
}

func NewDiscovery(method string) *DiscoveryService {
	return &DiscoveryService{
		method: method,
	}
}

func (d *DiscoveryService) Register(id string, info map[string]string) error {
	d.nodes.Store(id, info)
	return nil
}

func (d *DiscoveryService) Deregister(id string) error {
	_, loaded := d.nodes.LoadAndDelete(id)
	if !loaded {
		return fmt.Errorf("node not found: %s", id)
	}
	return nil
}

func (d *DiscoveryService) FindNodes() []map[string]string {
	var result []map[string]string
	d.nodes.Range(func(key, value interface{}) bool {
		if info, ok := value.(map[string]string); ok {
			result = append(result, info)
		}
		return true
	})
	return result
}

func (d *DiscoveryService) NodeCount() int {
	count := 0
	d.nodes.Range(func(key, value interface{}) bool {
		count++
		return true
	})
	return count
}
